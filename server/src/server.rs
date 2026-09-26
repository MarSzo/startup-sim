//! Authoritative game server: fixed 20 Hz tick, handshake, input processing,
//! room-based interest management and per-client snapshots.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use crate::building::Building;
use crate::net::{canonical, LinkConditions, Net};
use crate::protocol::{self as proto, EntityState, Packet, PlayerInfoEntry, SelfState};
use crate::sim::{self, Body, Pos};

pub const TICK_HZ: u32 = 20;
pub const TICK: Duration = Duration::from_millis(1000 / TICK_HZ as u64);
pub const DEFAULT_CLIENT_TIMEOUT: Duration = Duration::from_secs(5);
/// Max input steps applied per client per tick (3 expected at 60/20 Hz; the
/// slack absorbs jitter, the cap stops speed hacks).
pub const MAX_INPUTS_PER_TICK: usize = 6;
/// Inputs buffered beyond this are dropped (client running ahead / flooding).
pub const MAX_INPUT_QUEUE: usize = 30;
/// PlayerInfo entries per packet (worst case 2 + 1 + 16 B each -> ~1100 B).
const INFO_PER_PACKET: usize = 55;

pub struct Config {
    pub bind: SocketAddr,
    pub link: LinkConditions,
    pub max_players: usize,
    pub stats_every: Duration,
    pub client_timeout: Duration,
}

struct Player {
    id: u16,
    token: u32,
    nonce: u32,
    addr: SocketAddr,
    nick: String,
    body: Body,
    room: u16,
    flags: u8,
    last_heard: Instant,
    inputs: VecDeque<(u32, u8)>,
    last_received_seq: u32,
    last_processed_seq: u32,
    /// Ids whose PlayerInfo this client has been sent.
    known: HashSet<u16>,
    bytes_out: u64,
}

#[derive(Default)]
struct Stats {
    ticks: u64,
    tick_total: Duration,
    tick_max: Duration,
    missed: u64,
    snapshot_bytes: u64,
    max_visible: usize,
}

pub struct Server {
    building: Building,
    net: Net,
    cfg: Config,
    players: BTreeMap<u16, Player>,
    /// Session lookup: the token identifies the player, not the address, so a
    /// client survives a network change (Wi-Fi <-> LTE, new NAT port).
    by_token: HashMap<u32, u16>,
    /// Only used to dedupe `Connect` retries from the same address.
    by_addr: HashMap<SocketAddr, u16>,
    tick: u32,
    next_id: u16,
    next_spawn: usize,
    rng: fastrand::Rng,
    stats: Stats,
    started: Instant,
}

impl Server {
    pub fn new(building: Building, cfg: Config) -> std::io::Result<Server> {
        let net = Net::bind(cfg.bind, cfg.link)?;
        Ok(Server {
            building,
            net,
            cfg,
            players: BTreeMap::new(),
            by_token: HashMap::new(),
            by_addr: HashMap::new(),
            tick: 0,
            next_id: 1,
            next_spawn: 0,
            rng: fastrand::Rng::new(),
            stats: Stats::default(),
            started: Instant::now(),
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.net.local_addr().expect("bound socket")
    }

    fn log(&self, msg: impl AsRef<str>) {
        println!("[{:>8.2}s] {}", self.started.elapsed().as_secs_f64(), msg.as_ref());
    }

    /// Run forever.
    pub fn run(&mut self) {
        let mut next_tick = Instant::now() + TICK;
        let mut next_stats = Instant::now() + self.cfg.stats_every;
        loop {
            let now = Instant::now();
            if now >= next_tick {
                let t0 = Instant::now();
                self.tick();
                let dt = t0.elapsed();
                self.stats.ticks += 1;
                self.stats.tick_total += dt;
                self.stats.tick_max = self.stats.tick_max.max(dt);
                next_tick += TICK;
                // Fell behind by more than a whole tick: skip, don't spiral.
                let late = Instant::now().saturating_duration_since(next_tick);
                if late >= TICK {
                    let skipped = (late.as_nanos() / TICK.as_nanos()) as u32;
                    self.stats.missed += skipped as u64;
                    self.tick = self.tick.wrapping_add(skipped);
                    next_tick += TICK * skipped;
                }
            }
            if now >= next_stats {
                self.print_stats();
                next_stats += self.cfg.stats_every;
            }
            let now = Instant::now();
            self.net.flush(now);
            while let Some((addr, data)) = self.net.pop_inbound(now) {
                self.handle_datagram(addr, &data, now);
            }
            let mut deadline = next_tick.min(next_stats);
            if let Some(r) = self.net.next_release() {
                deadline = deadline.min(r);
            }
            self.net.recv(deadline.saturating_duration_since(Instant::now()));
        }
    }

    fn send(&mut self, addr: SocketAddr, p: &Packet) -> usize {
        let b = p.encode();
        debug_assert!(b.len() <= proto::MAX_PACKET);
        let n = b.len();
        self.net.send(addr, b);
        n
    }

    fn handle_datagram(&mut self, addr: SocketAddr, data: &[u8], now: Instant) {
        let Ok(packet) = Packet::decode(data) else {
            if let Some(proto::DecodeError::BadVersion(_)) = Packet::decode(data).err() {
                self.send(addr, &Packet::Reject { reason: proto::reject::BAD_VERSION });
            }
            return;
        };
        if let Packet::Connect { nonce, nick } = packet {
            self.handle_connect(addr, nonce, nick, now);
            return;
        }
        // Every other packet is identified by its session token.
        let token = match &packet {
            Packet::Input { token, .. }
            | Packet::InfoRequest { token, .. }
            | Packet::Ping { token, .. }
            | Packet::Disconnect { token, .. } => *token,
            _ => return,
        };
        let Some(&id) = self.by_token.get(&token) else {
            // Unknown/expired session: tell the client so it can reconnect.
            if matches!(packet, Packet::Input { .. } | Packet::Ping { .. }) {
                self.send(addr, &Packet::Disconnect { token, reason: proto::disconnect::SESSION_UNKNOWN });
            }
            return;
        };
        // Address migration: only packets that prove liveness *now* (a ping or
        // new inputs) may move the session, so a late reordered packet from the
        // old address cannot pull it back.
        let fresh = match &packet {
            Packet::Ping { .. } => true,
            Packet::Input { last_seq, .. } => *last_seq > self.players[&id].last_received_seq,
            _ => false,
        };
        if fresh && self.players[&id].addr != addr {
            self.migrate(id, addr);
        }
        self.players.get_mut(&id).unwrap().last_heard = now;
        let addr = self.players[&id].addr;
        match packet {
            Packet::Input { last_seq, inputs, .. } => {
                let p = self.players.get_mut(&id).unwrap();
                let n = inputs.len() as u32;
                for (i, &bits) in inputs.iter().enumerate() {
                    let seq = last_seq.wrapping_sub(n - 1 - i as u32);
                    if seq > p.last_received_seq {
                        p.inputs.push_back((seq, bits));
                        p.last_received_seq = seq;
                    }
                }
                while p.inputs.len() > MAX_INPUT_QUEUE {
                    let (seq, _) = p.inputs.pop_front().unwrap();
                    p.last_processed_seq = seq;
                }
            }
            Packet::InfoRequest { ids, .. } => {
                let entries: Vec<PlayerInfoEntry> = ids
                    .iter()
                    .filter_map(|i| self.players.get(i))
                    .map(|p| PlayerInfoEntry { id: p.id, nick: p.nick.clone() })
                    .collect();
                let p = self.players.get_mut(&id).unwrap();
                p.known.extend(entries.iter().map(|e| e.id));
                for chunk in entries.chunks(INFO_PER_PACKET) {
                    self.send(addr, &Packet::PlayerInfo { players: chunk.to_vec() });
                }
            }
            Packet::Ping { client_time, .. } => {
                let tick = self.tick;
                self.send(addr, &Packet::Pong { client_time, server_tick: tick });
            }
            Packet::Disconnect { .. } => {
                self.remove_player(id, "left");
            }
            _ => {}
        }
    }

    fn handle_connect(&mut self, addr: SocketAddr, nonce: u32, nick: String, now: Instant) {
        if let Some(&id) = self.by_addr.get(&addr) {
            if self.players[&id].nonce == nonce {
                // Our Welcome was lost; resend it.
                let welcome = self.welcome(id);
                self.send(addr, &welcome);
                return;
            }
            self.remove_player(id, "reconnected");
        }
        let nick: String = nick.chars().filter(|c| !c.is_control()).collect::<String>().trim().to_string();
        if nick.is_empty() {
            self.send(addr, &Packet::Reject { reason: proto::reject::BAD_NICK });
            return;
        }
        if self.players.len() >= self.cfg.max_players {
            self.send(addr, &Packet::Reject { reason: proto::reject::SERVER_FULL });
            return;
        }
        let id = self.alloc_id();
        let token = loop {
            let t = self.rng.u32(1..);
            if !self.by_token.contains_key(&t) {
                break t;
            }
        };
        let spawns = self.building.spawns();
        let (spawn_floor, spawn) = spawns[self.next_spawn % spawns.len()];
        self.next_spawn += 1;
        let pos = Pos::tile_center(spawn.x, spawn.y);
        let player = Player {
            id,
            token,
            nonce,
            addr,
            nick,
            body: Body::at(spawn_floor, pos),
            room: self.room_of(spawn_floor, pos),
            flags: 0,
            last_heard: now,
            inputs: VecDeque::new(),
            last_received_seq: 0,
            last_processed_seq: 0,
            known: HashSet::new(),
            bytes_out: 0,
        };
        self.log(format!("+ player {} '{}' from {} ({} online)", id, player.nick, canonical(addr), self.players.len() + 1));
        self.players.insert(id, player);
        self.by_addr.insert(addr, id);
        self.by_token.insert(token, id);
        let welcome = self.welcome(id);
        self.send(addr, &welcome);
    }

    fn room_of(&self, floor: u8, pos: Pos) -> u16 {
        self.building.floor(floor).map_or(0, |m| m.room_at(pos.x, pos.y))
    }

    fn welcome(&self, id: u16) -> Packet {
        let p = &self.players[&id];
        Packet::Welcome {
            nonce: p.nonce,
            player_id: id,
            token: p.token,
            tick_hz: TICK_HZ as u8,
            input_hz: sim::INPUT_HZ as u8,
            map_crc: self.building.crc,
            server_tick: self.tick,
        }
    }

    fn alloc_id(&mut self) -> u16 {
        loop {
            let id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1).max(1);
            if !self.players.contains_key(&id) {
                return id;
            }
        }
    }

    fn migrate(&mut self, id: u16, addr: SocketAddr) {
        let p = self.players.get_mut(&id).unwrap();
        let old = std::mem::replace(&mut p.addr, addr);
        if self.by_addr.get(&old) == Some(&id) {
            self.by_addr.remove(&old);
        }
        self.by_addr.insert(addr, id);
        self.log(format!("~ player {id} moved {} -> {}", canonical(old), canonical(addr)));
    }

    fn remove_player(&mut self, id: u16, why: &str) {
        if let Some(p) = self.players.remove(&id) {
            self.by_token.remove(&p.token);
            if self.by_addr.get(&p.addr) == Some(&id) {
                self.by_addr.remove(&p.addr);
            }
            for other in self.players.values_mut() {
                other.known.remove(&id);
            }
            self.log(format!("- player {} '{}' {} ({} online)", id, p.nick, why, self.players.len()));
        }
    }

    fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        let now = Instant::now();

        // 1. Timeouts.
        let stale: Vec<u16> = self
            .players
            .values()
            .filter(|p| now.duration_since(p.last_heard) > self.cfg.client_timeout)
            .map(|p| p.id)
            .collect();
        for id in stale {
            let (addr, token) = (self.players[&id].addr, self.players[&id].token);
            self.send(addr, &Packet::Disconnect { token, reason: proto::disconnect::TIMEOUT });
            self.remove_player(id, "timed out");
        }

        // 2. Simulation: apply queued inputs in sequence order.
        for p in self.players.values_mut() {
            let mut moved = false;
            for _ in 0..MAX_INPUTS_PER_TICK {
                let Some((seq, bits)) = p.inputs.pop_front() else { break };
                let before = p.body;
                p.body = sim::step(&self.building, p.body, bits);
                p.last_processed_seq = seq;
                moved |= p.body.pos != before.pos || p.body.floor != before.floor;
                let (dx, dy) = sim::input_dir(bits);
                let facing = if dy > 0 { 0 } else if dy < 0 { 1 } else if dx < 0 { 2 } else if dx > 0 { 3 } else { p.flags & 3 };
                p.flags = facing;
            }
            if moved {
                p.flags |= 0b100;
            }
            p.room = self.building.floor(p.body.floor).map_or(0, |m| m.room_at(p.body.pos.x, p.body.pos.y));
        }

        // 3. Interest management: group entities by (floor, room).
        let mut groups: HashMap<(u8, u16), Vec<EntityState>> = HashMap::new();
        for p in self.players.values() {
            groups.entry((p.body.floor, p.room)).or_default().push(EntityState {
                id: p.id,
                kind: proto::kind::PLAYER,
                x: p.body.pos.x,
                y: p.body.pos.y,
                flags: p.flags,
            });
        }

        // 4. Snapshots + PlayerInfo for newly visible entities.
        let tick = self.tick;
        let ids: Vec<u16> = self.players.keys().copied().collect();
        let mut outgoing: Vec<(SocketAddr, u16, Packet)> = Vec::new();
        for id in ids {
            let p = &self.players[&id];
            let visible: Vec<EntityState> =
                groups[&(p.body.floor, p.room)].iter().filter(|e| e.id != id).copied().collect();
            self.stats.max_visible = self.stats.max_visible.max(visible.len());
            let new_infos: Vec<PlayerInfoEntry> = visible
                .iter()
                .filter(|e| !p.known.contains(&e.id))
                .map(|e| PlayerInfoEntry { id: e.id, nick: self.players[&e.id].nick.clone() })
                .collect();
            let me = SelfState {
                x: p.body.pos.x,
                y: p.body.pos.y,
                floor: p.body.floor,
                room: p.room,
                lock: p.body.lock,
                prev_input: p.body.prev_input,
            };
            for f in proto::snapshot_fragments(tick, p.last_processed_seq, me, &visible) {
                outgoing.push((p.addr, id, f));
            }
            for chunk in new_infos.chunks(INFO_PER_PACKET) {
                outgoing.push((p.addr, id, Packet::PlayerInfo { players: chunk.to_vec() }));
            }
            let p = self.players.get_mut(&id).unwrap();
            p.known.extend(new_infos.iter().map(|e| e.id));
        }
        for (addr, id, packet) in outgoing {
            let n = self.send(addr, &packet);
            if matches!(packet, Packet::Snapshot { .. }) {
                self.stats.snapshot_bytes += n as u64;
            }
            if let Some(p) = self.players.get_mut(&id) {
                p.bytes_out += n as u64;
            }
        }
    }

    fn print_stats(&mut self) {
        let secs = self.cfg.stats_every.as_secs_f64();
        let s = std::mem::take(&mut self.stats);
        let avg_us = if s.ticks > 0 { s.tick_total.as_micros() as f64 / s.ticks as f64 } else { 0.0 };
        let n = self.players.len();
        let (mut min_bps, mut max_bps, mut sum) = (u64::MAX, 0u64, 0u64);
        for p in self.players.values_mut() {
            let bps = (p.bytes_out as f64 / secs) as u64;
            min_bps = min_bps.min(bps);
            max_bps = max_bps.max(bps);
            sum += bps;
            p.bytes_out = 0;
        }
        let avg_bps = if n > 0 { sum / n as u64 } else { 0 };
        if n == 0 {
            min_bps = 0;
        }
        let msg = format!(
            "tick {} | ticks {} (missed {}) | tick avg {:.0} us max {} us | players {} max_visible {} | out/client avg {:.1} KB/s (min {:.1}, max {:.1}) | in {:.0} pkt/s {:.1} KB/s | out {:.0} pkt/s | sim-dropped {}",
            self.tick,
            s.ticks,
            s.missed,
            avg_us,
            s.tick_max.as_micros(),
            n,
            s.max_visible,
            avg_bps as f64 / 1024.0,
            min_bps as f64 / 1024.0,
            max_bps as f64 / 1024.0,
            self.net.packets_in as f64 / secs,
            self.net.bytes_in as f64 / secs / 1024.0,
            self.net.packets_out as f64 / secs,
            self.net.dropped,
        );
        self.net.packets_in = 0;
        self.net.bytes_in = 0;
        self.net.packets_out = 0;
        self.net.bytes_out = 0;
        self.net.dropped = 0;
        self.log(msg);
    }
}
