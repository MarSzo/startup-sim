//! Authoritative game server: fixed 20 Hz tick, handshake, input processing,
//! room-based interest management and per-client snapshots.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use crate::building::Building;
use crate::coffee::{self, Cup, Machine};
use crate::computer::{self, Account, Computer, Messenger, Workstation};
use crate::inventory::{self, kind as item_kind, Inventory, Item};
use crate::net::{canonical, LinkConditions, Net};
use crate::npc::{self, Npc};
use crate::protocol::{self as proto, EntityState, Packet, PlayerInfoEntry, Profile, SelfState};
use crate::recruitment::{Attempt, Recruitment};
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
    /// Rights every new player starts with (`map::access::*`); 0 in normal
    /// play, CARD for load tests (`--start-with-card`) so bots pass the gates.
    pub start_access: u8,
    /// Job portal offers and quizzes.
    pub recruitment: Recruitment,
    /// Spawn straight into the world (dev / tests), no job portal.
    pub skip_recruitment: bool,
    /// Also already hired (contract, card, laptop) and spawned at a desk of
    /// the department (odd player ids IT, even Biznes). Dev / tests.
    pub start_employed: bool,
}

/// Where a connected player is in the game.
enum Stage {
    /// At home, on the computer desktop: job portal, mail, online interview.
    /// Not in the world yet.
    Portal(Box<Desk>),
    /// Hired and in the building.
    Working,
}

/// Desktop state of a candidate (GDD 9a, step 2).
#[derive(Default)]
struct Desk {
    /// Offers applied for (shown as "applied" on the portal).
    applied: Vec<u8>,
    /// Replies to send: (offer, due tick).
    pending: Vec<(u8, u32)>,
    /// Offers with an interview invitation.
    invited: Vec<u8>,
    /// Online interview in progress.
    attempt: Option<Attempt>,
    /// Passed an interview: department, waiting for "go to the office".
    hired: Option<u8>,
    inbox: Vec<MailMsg>,
    next_mail: u8,
}

struct MailMsg {
    id: u8,
    from: String,
    subject: String,
    body: String,
    action: u8,
    arg: u8,
}

impl Desk {
    fn mail(&mut self, from: &str, subject: String, body: String, action: u8, arg: u8) {
        self.next_mail = self.next_mail.wrapping_add(1).max(1);
        self.inbox.push(MailMsg { id: self.next_mail, from: from.into(), subject, body, action, arg });
        if self.inbox.len() > 12 {
            self.inbox.remove(0);
        }
    }
}

const RECRUITER: &str = "Startup Sim — Rekrutacja";

/// An item lying on the floor; `handle` is its entity id in snapshots.
struct Dropped {
    handle: u16,
    item: Item,
    floor: u8,
    pos: Pos,
}

/// Entity ids of items on the floor (players below, NPCs from 0xF000).
const DROP_HANDLE_BASE: u16 = 0xE000;
/// Reach for picking up / handing over items.
const PICKUP_RADIUS: i32 = sim::TILE_UNITS * 5 / 4;
const GIVE_RADIUS: i32 = sim::TILE_UNITS * 2;
/// Resend the inventory this often (ticks).
const INVENTORY_RESEND_TICKS: u32 = 40;

/// Resend the computer screen state this often (ticks).
const COMPUTER_RESEND_TICKS: u32 = 20;

/// Resend the current portal screen this often (ticks) - UDP may drop it.
const PORTAL_RESEND_TICKS: u32 = 20;

struct Player {
    id: u16,
    /// Character from the creation screen (age, city, e-mail stay here).
    profile: Profile,
    stage: Stage,
    /// Department of the position the player was recruited for (0 = none).
    department: u8,
    /// Contract signed at HR: the department is official (shown to others).
    contract: bool,
    /// Recruitment attempts so far (numbers the attempts).
    attempts: u8,
    /// Coffee being brewed.
    cup: Cup,
    /// Pockets and hands.
    inventory: Inventory,
    /// Inventory changed: send it to the owner this tick.
    inv_dirty: bool,
    /// Handle of the computer whose screen the player is looking at.
    at_computer: Option<u16>,
    /// Messenger spam guard / retry dedupe.
    last_chat_tick: Option<u32>,
    last_chat_nonce: u32,
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
    npcs: Vec<Npc>,
    machines: Vec<Machine>,
    /// Items lying on the floor.
    dropped: Vec<Dropped>,
    /// Desks where a laptop can stand, and the laptops standing on them.
    workstations: Vec<Workstation>,
    computers: Vec<Computer>,
    messenger: Messenger,
    next_item_id: u32,
    next_drop_handle: u16,
    /// Lines players "say" to themselves outside the tick (item actions).
    pending_says: Vec<(u16, String, Option<u16>)>,
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
            npcs: Npc::spawn_all(&building),
            machines: coffee::find_machines(&building),
            dropped: Vec::new(),
            workstations: computer::find_workstations(&building),
            computers: Vec::new(),
            messenger: Messenger::default(),
            next_item_id: 1,
            next_drop_handle: DROP_HANDLE_BASE,
            pending_says: Vec::new(),
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
        if let Packet::Connect { nonce, nick, profile } = packet {
            self.handle_connect(addr, nonce, nick, profile, now);
            return;
        }
        // Every other packet is identified by its session token.
        let token = match &packet {
            Packet::Input { token, .. }
            | Packet::InfoRequest { token, .. }
            | Packet::Ping { token, .. }
            | Packet::Disconnect { token, .. }
            | Packet::Apply { token, .. }
            | Packet::Answer { token, .. }
            | Packet::PortalAction { token, .. }
            | Packet::ItemAction { token, .. }
            | Packet::ComputerAction { token, .. } => *token,
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
                let entries: Vec<PlayerInfoEntry> = ids.iter().filter_map(|&i| self.info_of(i)).collect();
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
            Packet::Apply { offer, .. } => self.handle_apply(id, offer),
            Packet::PortalAction { action, arg, .. } => self.handle_portal_action(id, action, arg),
            Packet::ItemAction { action, slot, .. } => self.handle_item_action(id, action, slot),
            Packet::ComputerAction { action, conv, arg, text, .. } => self.handle_computer_action(id, action, conv, arg, &text),
            Packet::Answer { attempt, index, choice, .. } => self.handle_answer(id, attempt, index, choice),
            _ => {}
        }
    }

    fn handle_connect(&mut self, addr: SocketAddr, nonce: u32, nick: String, profile: Profile, now: Instant) {
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
        let Some(profile) = validate_profile(profile) else {
            self.send(addr, &Packet::Reject { reason: proto::reject::BAD_PROFILE });
            return;
        };
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
        let skip = self.cfg.skip_recruitment;
        let player = Player {
            id,
            profile,
            stage: if skip { Stage::Working } else { Stage::Portal(Box::default()) },
            department: 0,
            contract: false,
            attempts: 0,
            cup: Cup::None,
            inventory: Inventory::default(),
            inv_dirty: true,
            at_computer: None,
            last_chat_tick: None,
            last_chat_nonce: 0,
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
        if self.cfg.start_employed {
            self.employ(id);
        } else if skip && self.cfg.start_access & crate::map::access::CARD != 0 {
            self.give_new(id, item_kind::EMPLOYEE_CARD); // load tests: straight in with a card
        }
        self.by_token.insert(token, id);
        let welcome = self.welcome(id);
        self.send(addr, &welcome);
        if !skip {
            self.send_portal(id, true);
        }
    }

    /// (Re)send the desktop state: portal offers, current interview
    /// question and (with `mails`) the whole inbox. Clients dedupe.
    fn send_portal(&mut self, id: u16, mails: bool) {
        let Some(p) = self.players.get(&id) else { return };
        let Stage::Portal(desk) = &p.stage else { return };
        let r = &self.cfg.recruitment;
        // Offers may not fit one datagram: split them (the client merges by id).
        let mut packets = Vec::new();
        let mut chunk = Vec::new();
        let mut size = proto::HEADER_LEN + 1;
        for offer in r.portal(|o| desk.applied.contains(&o)) {
            let len = 3 + 6 + offer.company.len() + offer.title.len() + offer.description.len();
            if size + len > proto::MAX_PACKET && !chunk.is_empty() {
                packets.push(Packet::JobOffers { offers: std::mem::take(&mut chunk) });
                size = proto::HEADER_LEN + 1;
            }
            size += len;
            chunk.push(offer);
        }
        packets.push(Packet::JobOffers { offers: chunk });
        if let Some(q) = desk.attempt.as_ref().and_then(|a| a.current(r).map(|q| (a.number, q))) {
            let (attempt, q) = q;
            packets.push(Packet::Question { attempt, index: q.index, total: q.total, text: q.text, options: q.options });
        }
        if mails {
            for m in &desk.inbox {
                packets.push(Packet::Mail {
                    id: m.id,
                    from: m.from.clone(),
                    subject: m.subject.clone(),
                    body: m.body.clone(),
                    action: m.action,
                    arg: m.arg,
                });
            }
        }
        let addr = p.addr;
        for packet in packets {
            self.send(addr, &packet);
        }
    }

    fn desk(&mut self, id: u16) -> Option<&mut Desk> {
        match &mut self.players.get_mut(&id)?.stage {
            Stage::Portal(d) => Some(d),
            Stage::Working => None,
        }
    }

    fn handle_apply(&mut self, id: u16, offer: u8) {
        let due = self.tick + self.cfg.recruitment.invite_delay_secs * TICK_HZ;
        let Some(o) = self.cfg.recruitment.offer(offer).map(|o| (o.hiring, o.reply.is_some())) else { return };
        let Some(desk) = self.desk(id) else { return };
        if desk.applied.contains(&offer) || desk.hired.is_some() {
            return; // duplicate (resent) application
        }
        desk.applied.push(offer);
        if o.0 || o.1 {
            desk.pending.push((offer, due)); // other companies without a reply: silence
        }
        self.send_portal(id, false);
    }

    /// Replies that are due: interview invitations / other companies' answers.
    fn deliver_replies(&mut self, id: u16) {
        let tick = self.tick;
        let Some(p) = self.players.get_mut(&id) else { return };
        let nick = p.nick.clone();
        let Stage::Portal(desk) = &mut p.stage else { return };
        let due: Vec<u8> = desk.pending.iter().filter(|(_, t)| *t <= tick).map(|(o, _)| *o).collect();
        if due.is_empty() {
            return;
        }
        desk.pending.retain(|(_, t)| *t > tick);
        for offer in due {
            let Some(o) = self.cfg.recruitment.offer(offer) else { continue };
            if o.hiring {
                desk.invited.push(offer);
                desk.mail(
                    RECRUITER,
                    format!("Zaproszenie na rozmowę: {}", o.title),
                    format!(
                        "Cześć {nick}!\n\nDziękujemy za zgłoszenie na stanowisko {}. Zapraszamy na krótką rozmowę online — \
                         kilka pytań, zero stresu (prawie). Kliknij „Dołącz do rozmowy”, kiedy tylko możesz.\n\nZespół Startup Sim",
                        o.title
                    ),
                    proto::portal_action::JOIN_INTERVIEW,
                    offer,
                );
            } else if let Some(reply) = &o.reply {
                desk.mail(&o.company, format!("Re: {}", o.title), reply.clone(), proto::portal_action::NONE, 0);
            }
        }
        self.send_portal(id, true);
    }

    fn handle_portal_action(&mut self, id: u16, action: u8, arg: u8) {
        match action {
            proto::portal_action::JOIN_INTERVIEW => {
                let p = self.players.get_mut(&id).unwrap();
                let Stage::Portal(desk) = &mut p.stage else { return };
                if !desk.invited.contains(&arg) || desk.attempt.is_some() || desk.hired.is_some() {
                    return;
                }
                p.attempts = p.attempts.wrapping_add(1);
                desk.attempt = self.cfg.recruitment.start(arg, p.attempts, &mut self.rng);
                self.send_portal(id, false);
            }
            proto::portal_action::GO_TO_OFFICE => {
                let p = self.players.get_mut(&id).unwrap();
                let Stage::Portal(desk) = &p.stage else { return };
                let Some(dept) = desk.hired else { return };
                p.stage = Stage::Working;
                p.department = dept;
                let msg = format!(
                    "* player {id} '{}' goes to the office: {}",
                    p.nick,
                    self.cfg.recruitment.department_name(dept).unwrap_or("?")
                );
                self.log(msg);
                if self.cfg.start_access & crate::map::access::CARD != 0 {
                    self.give_new(id, item_kind::EMPLOYEE_CARD); // load tests: straight in with a card
                }
            }
            _ => {}
        }
    }

    fn handle_answer(&mut self, id: u16, attempt_no: u8, index: u8, choice: u8) {
        let p = self.players.get_mut(&id).unwrap();
        let nick = p.nick.clone();
        let addr = p.addr;
        let Stage::Portal(desk) = &mut p.stage else { return };
        let Some(a) = desk.attempt.as_mut() else { return };
        if a.number != attempt_no || !a.answer(index, choice) {
            return; // stale / duplicate: the resend loop shows the current state
        }
        if !a.finished() {
            self.send_portal(id, false);
            return;
        }
        let r = &self.cfg.recruitment;
        let (score, total, offer) = (a.score(), a.total(), a.offer);
        let passed = score >= r.pass_score;
        let o = r.offer(offer).expect("attempt for a known offer");
        let result = Packet::RecruitResult {
            attempt: attempt_no,
            passed,
            score: score as u8,
            total: total as u8,
            department: if passed { o.department } else { 0 },
        };
        desk.attempt = None;
        desk.invited.retain(|x| *x != offer);
        if passed {
            desk.hired = Some(o.department);
            desk.mail(
                RECRUITER,
                "Zaproszenie na dzień próbny".into(),
                format!(
                    "Gratulacje, {nick}!\n\nRozmowa na stanowisko {} poszła świetnie ({score}/{total}). Zapraszamy na dzień \
                     próbny do biura: zgłoś się na portierni — portier zaprowadzi Cię na recepcję, a w HR podpiszesz umowę \
                     i odbierzesz kartę.\n\nDo zobaczenia!",
                    o.title
                ),
                proto::portal_action::GO_TO_OFFICE,
                0,
            );
        } else {
            desk.applied.retain(|x| *x != offer); // may apply again
            desk.mail(
                RECRUITER,
                format!("Dziękujemy za rozmowę: {}", o.title),
                format!(
                    "Cześć {nick},\n\ndziękujemy za rozmowę ({score}/{total}). Tym razem szukamy kogoś innego, ale nie \
                     przejmuj się — zapraszamy do ponownej aplikacji. Pytania będą inne!\n\nZespół Startup Sim"
                ),
                proto::portal_action::NONE,
                0,
            );
        }
        self.send(addr, &result);
        self.send(addr, &result); // tiny packet; a duplicate makes loss unlikely
        self.send_portal(id, true);
    }

    /// Name (and official department) of a player or NPC.
    fn info_of(&self, id: u16) -> Option<PlayerInfoEntry> {
        match self.players.get(&id) {
            Some(p) => Some(PlayerInfoEntry {
                id,
                nick: p.nick.clone(),
                department: if p.contract { p.department } else { 0 },
                gender: p.profile.gender,
                appearance: p.profile.appearance,
            }),
            // A computer is introduced by its owner's name.
            None if self.computers.iter().any(|c| c.handle == id) => {
                let owner = self.computers.iter().find(|c| c.handle == id)?.owner();
                self.info_of(owner).map(|e| PlayerInfoEntry { id, ..e })
            }
            None => self.npcs.iter().find(|n| n.id == id).map(|n| PlayerInfoEntry {
                id,
                nick: n.name.clone(),
                department: 0,
                gender: proto::gender::OTHER,
                appearance: proto::Appearance::default(),
            }),
        }
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
            // Player ids: 1 .. NPC_ID_BASE-1 (NPCs use the ids above).
            self.next_id = if self.next_id + 1 >= npc::NPC_ID_BASE { 1 } else { self.next_id + 1 };
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

    // ------------------------------------------------------------- items

    fn label_for(&self, pid: u16, k: u8) -> String {
        let Some(p) = self.players.get(&pid) else { return String::new() };
        let dept = self.cfg.recruitment.department_name(p.department).unwrap_or("");
        match k {
            item_kind::GUEST_PASS => format!("Dzień próbny: {}", p.nick),
            item_kind::EMPLOYEE_CARD if !dept.is_empty() => format!("{} · {dept}", p.nick),
            item_kind::EMPLOYEE_CARD => p.nick.clone(),
            item_kind::LAPTOP => format!("Laptop: {}", p.nick),
            item_kind::COFFEE => "Gorąca, z ekspresu".into(),
            _ => String::new(),
        }
    }

    /// Create a new item for `pid` (labelled for them) and hand it over.
    fn give_new(&mut self, pid: u16, k: u8) {
        let label = self.label_for(pid, k);
        let expires = (k == item_kind::COFFEE).then_some(self.tick + coffee::DRINK_TICKS);
        let owner = if k == item_kind::COFFEE { 0 } else { pid };
        let item = Item { id: self.next_item_id, kind: k, label, expires, owner };
        self.next_item_id += 1;
        self.give(pid, item);
    }

    /// Put an item into a player's inventory; if it doesn't fit, it lands on
    /// the floor at their feet.
    fn give(&mut self, pid: u16, item: Item) {
        let Some(p) = self.players.get_mut(&pid) else { return };
        match p.inventory.add(item) {
            Ok(()) => refresh(p),
            Err(item) => {
                let (floor, pos) = (p.body.floor, p.body.pos);
                self.drop_at(floor, pos, item);
            }
        }
    }

    fn drop_at(&mut self, floor: u8, pos: Pos, item: Item) {
        let handle = self.alloc_handle();
        self.dropped.push(Dropped { handle, item, floor, pos });
    }

    /// Entity id for an item on the floor or a computer on a desk.
    fn alloc_handle(&mut self) -> u16 {
        let next = |h: u16| if h + 1 >= npc::NPC_ID_BASE { DROP_HANDLE_BASE } else { h + 1 };
        let mut handle = self.next_drop_handle;
        while self.dropped.iter().any(|d| d.handle == handle) || self.computers.iter().any(|c| c.handle == handle) {
            handle = next(handle);
        }
        self.next_drop_handle = next(handle);
        handle
    }

    fn handle_item_action(&mut self, id: u16, action: u8, slot: u8) {
        let Some(p) = self.players.get_mut(&id) else { return };
        if !matches!(p.stage, Stage::Working) {
            return;
        }
        let say = |line: &str| (id, line.to_string(), None);
        match action {
            proto::item_action::TAKE_OUT => match p.inventory.take_out(slot as usize) {
                Ok(()) => refresh(p),
                Err(r) => self.pending_says.push(say(r.line())),
            },
            proto::item_action::PUT_AWAY => match p.inventory.put_away() {
                Ok(()) => refresh(p),
                Err(r) => self.pending_says.push(say(r.line())),
            },
            proto::item_action::DROP => {
                if let Some(item) = p.inventory.take_hands() {
                    refresh(p);
                    let (floor, pos) = (p.body.floor, p.body.pos);
                    self.drop_at(floor, pos, item);
                }
            }
            proto::item_action::GIVE => {
                let (floor, pos) = (p.body.floor, p.body.pos);
                if p.inventory.hands_free() {
                    return;
                }
                let target = self
                    .players
                    .values()
                    .filter(|o| o.id != id && matches!(o.stage, Stage::Working) && o.body.floor == floor)
                    .map(|o| (o.id, dist2(o.body.pos, pos)))
                    .filter(|&(_, d)| d <= GIVE_RADIUS * GIVE_RADIUS)
                    .min_by_key(|&(_, d)| d)
                    .map(|(t, _)| t);
                let Some(target) = target else {
                    self.pending_says.push(say("Nie ma nikogo obok."));
                    return;
                };
                let item = self.players.get_mut(&id).unwrap().inventory.take_hands().unwrap();
                let name = inventory::display_name(item.kind);
                let to = self.players.get_mut(&target).unwrap();
                let to_nick = to.nick.clone();
                match to.inventory.add(item) {
                    Ok(()) => {
                        refresh(to);
                        refresh(self.players.get_mut(&id).unwrap());
                        let from_nick = self.players[&id].nick.clone();
                        self.pending_says.push((id, format!("Proszę, {to_nick} — {}.", name.to_lowercase()), Some(target)));
                        self.log(format!("* item: {from_nick} gave {name} to {to_nick}"));
                    }
                    Err(item) => {
                        self.players.get_mut(&id).unwrap().inventory.hands = Some(item); // give it back
                        self.pending_says.push(say(&format!("{to_nick} nie ma już wolnych rąk ani kieszeni.")));
                    }
                }
            }
            proto::item_action::USE => {
                let Some(held) = &p.inventory.hands else { return };
                let line = match held.kind {
                    item_kind::COFFEE => {
                        p.inventory.take_hands();
                        refresh(p);
                        coffee::lines::DRUNK.to_string()
                    }
                    item_kind::EMPLOYEE_CARD => format!("Karta pracownika: {}.", held.label),
                    item_kind::GUEST_PASS => "Przepustka gościa — ważna do końca dnia.".into(),
                    item_kind::LAPTOP => format!("{} — położę go na wolnym biurku w swoim dziale (E).", held.label),
                    _ => return,
                };
                self.pending_says.push(say(&line));
            }
            _ => {}
        }
    }

    /// Pick up the nearest item on the floor within reach, if any.
    fn try_pickup(&mut self, pid: u16, body: &Body) -> Option<String> {
        let i = self
            .dropped
            .iter()
            .enumerate()
            .filter(|(_, d)| d.floor == body.floor && dist2(d.pos, body.pos) <= PICKUP_RADIUS * PICKUP_RADIUS)
            .min_by_key(|(_, d)| dist2(d.pos, body.pos))
            .map(|(i, _)| i)?;
        let p = self.players.get_mut(&pid)?;
        let d = self.dropped.remove(i);
        let name = inventory::display_name(d.item.kind);
        match p.inventory.add(d.item) {
            Ok(()) => {
                refresh(p);
                Some(format!("Podniesione: {}.", name.to_lowercase()))
            }
            Err(item) => {
                self.dropped.insert(i, Dropped { item, ..d });
                Some(inventory::Refusal::HandsFull.line().to_string())
            }
        }
    }

    // --------------------------------------------------------- computers

    /// `--start-employed`: contract, department, card and laptop, and a spot
    /// in front of a desk of the department.
    fn employ(&mut self, id: u16) {
        let dept = if id % 2 == 1 { 1 } else { 2 };
        let Some(dept_name) = self.cfg.recruitment.department_name(dept).map(str::to_string) else { return };
        let seat = {
            let n = self.players.values().filter(|p| p.contract && p.department == dept).count();
            let desks: Vec<&Workstation> = self.workstations.iter().filter(|w| w.room_name == dept_name).collect();
            desks.get(n % desks.len().max(1)).and_then(|w| {
                let m = self.building.floor(w.floor)?;
                [1, -1].iter().map(|dy| (w.tile.x, w.tile.y + dy)).find(|&(x, y)| !m.is_blocked(x, y)).map(|(x, y)| (w.floor, x, y))
            })
        };
        let Some(p) = self.players.get_mut(&id) else { return };
        p.department = dept;
        p.contract = true;
        if let Some((floor, x, y)) = seat {
            p.body = Body::at(floor, Pos::tile_center(x, y));
            p.room = self.building.floor(floor).map_or(0, |m| m.room_at_tile(x, y));
        }
        self.give_new(id, item_kind::EMPLOYEE_CARD);
        self.give_new(id, item_kind::LAPTOP);
    }

    /// Hired employees = messenger accounts.
    fn accounts(&self) -> Vec<Account> {
        self.players
            .values()
            .filter(|p| p.contract && matches!(p.stage, Stage::Working))
            .map(|p| Account { id: p.id, nick: p.nick.clone(), department: p.department })
            .collect()
    }

    /// E at a desk: open the computer on it, or put the laptop in hands down.
    /// `None` = no desk in reach (the E goes on to picking things up);
    /// `Some(line)` = handled, with an optional speech line.
    fn use_desk(&mut self, pid: u16, body: &Body) -> Option<Option<String>> {
        let ws = computer::workstation_in_reach(&self.workstations, body)?;
        if let Some(c) = self.computers.iter_mut().find(|c| c.station == ws) {
            if c.user.is_some_and(|u| u != pid) {
                return Some(Some(computer::lines::BUSY.into()));
            }
            c.user = Some(pid);
            let handle = c.handle;
            self.players.get_mut(&pid)?.at_computer = Some(handle);
            self.send_computer(pid);
            return Some(None);
        }
        let p = self.players.get(&pid)?;
        if p.inventory.held_kind() != item_kind::LAPTOP {
            return None;
        }
        let dept = self.cfg.recruitment.department_name(p.department).unwrap_or("");
        if !p.contract || dept.is_empty() {
            return Some(Some(computer::lines::NO_DEPARTMENT.into()));
        }
        if self.workstations[ws].room_name != dept {
            return Some(Some(computer::lines::NOT_MY_DEPARTMENT.into()));
        }
        let handle = self.alloc_handle();
        let p = self.players.get_mut(&pid)?;
        let item = p.inventory.take_hands()?;
        refresh(p);
        self.computers.push(Computer { handle, station: ws, item, locked: false, user: None });
        Some(Some(computer::lines::PLACED.into()))
    }

    /// Stop looking at the screen (the client closes it when the
    /// AT_COMPUTER status bit clears).
    fn end_session(&mut self, pid: u16) {
        let Some(p) = self.players.get_mut(&pid) else { return };
        if let Some(h) = p.at_computer.take() {
            if let Some(c) = self.computers.iter_mut().find(|c| c.handle == h && c.user == Some(pid)) {
                c.user = None;
            }
        }
    }

    /// Users who walked away (or whose computer is gone) leave the screen.
    fn check_computer_sessions(&mut self) {
        let mut gone = Vec::new();
        for p in self.players.values() {
            let Some(h) = p.at_computer else { continue };
            let ok = self.computers.iter().find(|c| c.handle == h).is_some_and(|c| {
                c.user == Some(p.id) && computer::in_leave_range(&self.workstations[c.station], &p.body)
            });
            if !ok {
                gone.push(p.id);
            }
        }
        for pid in gone {
            self.end_session(pid);
        }
    }

    fn computer_packet(&self, pid: u16) -> Option<Packet> {
        let h = self.players.get(&pid)?.at_computer?;
        let c = self.computers.iter().find(|c| c.handle == h)?;
        let accounts = self.accounts();
        let convs = match accounts.iter().find(|a| a.id == c.owner()) {
            Some(acc) if !c.locked => self
                .messenger
                .conversations(acc, &accounts, |d| self.cfg.recruitment.department_name(d).map(str::to_string))
                .into_iter()
                .map(|i| proto::ConvEntry { conv: i.conv, unread: i.unread, title: i.title })
                .collect(),
            _ => Vec::new(),
        };
        Some(Packet::Computer { handle: h, owner: c.owner(), locked: c.locked, convs })
    }

    fn send_computer(&mut self, pid: u16) {
        if let Some(pk) = self.computer_packet(pid) {
            let addr = self.players[&pid].addr;
            self.send(addr, &pk);
        }
    }

    /// Send messages, split so each datagram fits.
    fn send_chat(&mut self, addr: SocketAddr, conv: u16, msgs: &[computer::Msg]) {
        let mut chunk: Vec<proto::ChatEntry> = Vec::new();
        let mut size = proto::HEADER_LEN + 3;
        for m in msgs {
            let len = 4 + 2 + 2 + m.nick.len().min(proto::MAX_NICK_BYTES) + 2 + m.text.len().min(proto::MAX_CHAT_BYTES);
            if size + len > proto::MAX_PACKET && !chunk.is_empty() {
                self.send(addr, &Packet::Chat { conv, messages: std::mem::take(&mut chunk) });
                size = proto::HEADER_LEN + 3;
            }
            size += len;
            chunk.push(proto::ChatEntry { id: m.id, from: m.from, nick: m.nick.clone(), text: m.text.clone() });
        }
        self.send(addr, &Packet::Chat { conv, messages: chunk });
    }

    fn handle_computer_action(&mut self, pid: u16, action: u8, conv: u16, arg: u32, text: &str) {
        use proto::computer_action as a;
        let Some(h) = self.players.get(&pid).and_then(|p| p.at_computer) else { return };
        let Some(ci) = self.computers.iter().position(|c| c.handle == h && c.user == Some(pid)) else { return };
        let say = |line: &str| (pid, line.to_string(), None);
        let owner = self.computers[ci].owner();
        match action {
            a::CLOSE => self.end_session(pid),
            a::LOCK => {
                self.computers[ci].locked = true;
                self.end_session(pid);
            }
            a::UNLOCK => {
                if pid == owner {
                    self.computers[ci].locked = false;
                    self.send_computer(pid);
                } else {
                    self.pending_says.push(say(computer::lines::LOCKED));
                }
            }
            a::TAKE => {
                if !self.players[&pid].inventory.hands_free() {
                    self.pending_says.push(say(computer::lines::HANDS_FULL));
                    return;
                }
                self.end_session(pid);
                let c = self.computers.remove(ci);
                if owner != pid {
                    let (who, whose) = (self.players[&pid].nick.clone(), c.item.label.clone());
                    self.log(format!("* computer: {who} took {whose}"));
                }
                self.give(pid, c.item);
                self.pending_says.push(say(computer::lines::TAKEN));
            }
            a::SYNC | a::SEND => {
                if self.computers[ci].locked {
                    return;
                }
                let accounts = self.accounts();
                let Some(acc) = accounts.iter().find(|x| x.id == owner).cloned() else { return };
                let addr = self.players[&pid].addr;
                if action == a::SYNC {
                    if let Some(msgs) = self.messenger.sync(&acc, conv, arg, &accounts) {
                        self.send_chat(addr, conv, &msgs);
                    }
                    return;
                }
                let tick = self.tick;
                let p = self.players.get_mut(&pid).unwrap();
                if arg == p.last_chat_nonce || p.last_chat_tick.is_some_and(|t| tick.wrapping_sub(t) < computer::SEND_COOLDOWN_TICKS) {
                    return; // retry of a message already posted, or spam
                }
                let Some(msg) = self.messenger.post(&acc, conv, text, &accounts) else { return };
                let p = self.players.get_mut(&pid).unwrap();
                p.last_chat_nonce = arg;
                p.last_chat_tick = Some(tick);
                let typed_by = p.nick.clone();
                if pid == owner {
                    self.log(format!("* chat: {} -> conv {conv}", acc.nick));
                } else {
                    self.log(format!("* chat: {typed_by} as {} -> conv {conv}", acc.nick));
                }
                // Live push to everyone looking at a screen of an account in
                // the audience (the others get it from SYNC / unread counts).
                let mut pushes = Vec::new();
                for (account, seen_as) in self.messenger.audience(&acc, conv, &accounts) {
                    for viewer in self.players.values() {
                        let Some(vh) = viewer.at_computer else { continue };
                        if self.computers.iter().any(|c| c.handle == vh && c.owner() == account && !c.locked) {
                            pushes.push((viewer.addr, viewer.id, seen_as));
                        }
                    }
                }
                for (addr, vid, seen_as) in pushes {
                    self.send_chat(addr, seen_as, std::slice::from_ref(&msg));
                    self.send_computer(vid);
                }
            }
            _ => {}
        }
    }

    fn remove_player(&mut self, id: u16, why: &str) {
        if let Some(mut p) = self.players.remove(&id) {
            coffee::release(&mut self.machines, &p.cup);
            for c in &mut self.computers {
                if c.user == Some(id) {
                    c.user = None;
                }
            }
            // Other people's things they carried stay in the building...
            let carried: Vec<Item> = std::mem::take(&mut p.inventory).items().cloned().collect();
            for item in carried.into_iter().filter(|i| i.owner != 0 && i.owner != id) {
                self.drop_at(p.body.floor, p.body.pos, item);
            }
            // ...their own go with them (no persistent accounts yet): laptop on
            // a desk, card lent to someone, anything on the floor.
            self.computers.retain(|c| c.owner() != id);
            self.dropped.retain(|d| d.item.owner != id);
            for other in self.players.values_mut() {
                let before = other.inventory.clone();
                other.inventory.remove_owned_by(id);
                if other.inventory != before {
                    refresh(other);
                }
            }
            self.messenger.forget(id);
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
        let mut talks: Vec<(u16, Body)> = Vec::new();
        let mut coffee_says: Vec<(u16, String, Option<u16>)> = std::mem::take(&mut self.pending_says);
        let mut coffee_ready = Vec::new();
        let mut portal_resend = Vec::new();
        for p in self.players.values_mut() {
            if !matches!(p.stage, Stage::Working) {
                p.inputs.clear(); // not in the world yet
                portal_resend.push(p.id);
                continue;
            }
            let mut moved = false;
            for _ in 0..MAX_INPUTS_PER_TICK {
                let Some((seq, bits)) = p.inputs.pop_front() else { break };
                let before = p.body;
                p.body = sim::step(&self.building, p.body, bits);
                p.last_processed_seq = seq;
                // E pressed (edge) without riding the elevator: talk to an NPC nearby.
                let pressed = bits & sim::IN_INTERACT != 0 && before.prev_input & sim::IN_INTERACT == 0;
                if pressed && p.body.floor == before.floor {
                    talks.push((p.id, p.body));
                }
                moved |= p.body.pos != before.pos || p.body.floor != before.floor;
                let (dx, dy) = sim::input_dir(bits);
                let facing = if dy > 0 { 0 } else if dy < 0 { 1 } else if dx < 0 { 2 } else if dx > 0 { 3 } else { p.flags & 3 };
                p.flags = facing;
            }
            if moved {
                p.flags |= 0b100;
            }
            p.room = self.building.floor(p.body.floor).map_or(0, |m| m.room_at(p.body.pos.x, p.body.pos.y));
            if coffee::tick_cup(&mut p.cup, self.tick) {
                coffee_ready.push(p.id);
            }
            if !p.inventory.expire(self.tick).is_empty() {
                refresh(p);
                coffee_says.push((p.id, coffee::lines::COLD.to_string(), None));
            }
            p.flags = (p.flags & 0x3f) | status_bits(p) << proto::status::FLAGS_SHIFT;
        }
        for pid in coffee_ready {
            let free = self.players[&pid].inventory.hands_free();
            coffee_says.push((pid, if free { coffee::lines::READY } else { coffee::lines::WAITING }.to_string(), None));
            self.give_new(pid, item_kind::COFFEE); // no free hands: it waits on the floor
        }

        for id in portal_resend {
            self.deliver_replies(id);
            if self.tick % PORTAL_RESEND_TICKS == 0 {
                self.send_portal(id, self.tick % (2 * PORTAL_RESEND_TICKS) == 0);
            }
        }

        // 3. NPCs: conversations, then their own behaviour.
        let mut events = Vec::new();
        for (pid, body) in talks {
            let p = &self.players[&pid];
            let dept = self.cfg.recruitment.department_name(p.department).map(str::to_string);
            let nearest = self
                .npcs
                .iter_mut()
                .filter(|n| n.in_talk_range(&body))
                // NPCs at their post first (a porter still standing next to
                // the guest he just brought mustn't shadow the receptionist).
                .min_by_key(|n| (!n.is_idle(), (n.body.pos.x - body.pos.x).abs() + (n.body.pos.y - body.pos.y).abs()));
            let hands_free = p.inventory.hands_free();
            if let Some(n) = nearest {
                events.extend(n.interact(&self.building, pid, body.access, dept.as_deref(), hands_free));
            } else if let Some(i) = coffee::machine_in_reach(&self.machines, &body) {
                let p = self.players.get_mut(&pid).unwrap();
                let line = match coffee::use_machine(&mut self.machines, i, &mut p.cup, hands_free, self.tick) {
                    coffee::Outcome::Started => coffee::lines::BREWING,
                    coffee::Outcome::Busy => coffee::lines::BUSY,
                    coffee::Outcome::HandsFull => coffee::lines::HANDS_FULL,
                };
                p.flags = (p.flags & 0x3f) | status_bits(p) << proto::status::FLAGS_SHIFT;
                coffee_says.push((pid, line.to_string(), None));
            } else if let Some(said) = self.use_desk(pid, &body) {
                coffee_says.extend(said.map(|line| (pid, line, None)));
            } else if let Some(line) = self.try_pickup(pid, &body) {
                coffee_says.push((pid, line, None));
            }
        }
        self.check_computer_sessions();

        let bodies: HashMap<u16, Body> =
            self.players.values().filter(|p| matches!(p.stage, Stage::Working)).map(|p| (p.id, p.body)).collect();
        for n in &mut self.npcs {
            events.extend(n.tick(&self.building, &bodies));
        }
        let mut says: Vec<(u16, String, Option<u16>)> = coffee_says;
        for e in events {
            match e {
                npc::Event::Give { player, item } => self.give_new(player, item),
                npc::Event::Take { player, item } => {
                    if let Some(p) = self.players.get_mut(&player) {
                        if p.inventory.remove_kind(item).is_some() {
                            refresh(p);
                        }
                    }
                }
                npc::Event::Say { npc, text, to } => says.push((npc, text, to)),
                npc::Event::Contract { player } => {
                    if let Some(p) = self.players.get_mut(&player) {
                        p.contract = true;
                        let msg = format!(
                            "* player {player} '{}' signed a contract: {}",
                            p.nick,
                            self.cfg.recruitment.department_name(p.department).unwrap_or("-")
                        );
                        self.log(msg);
                    }
                    // Everyone gets the updated PlayerInfo (department) again.
                    for other in self.players.values_mut() {
                        other.known.remove(&player);
                    }
                }
            }
        }

        // 4. Interest management: group entities by (floor, room).
        let mut groups: HashMap<(u8, u16), Vec<EntityState>> = HashMap::new();
        for p in self.players.values().filter(|p| matches!(p.stage, Stage::Working)) {
            groups.entry((p.body.floor, p.room)).or_default().push(EntityState {
                id: p.id,
                kind: proto::kind::PLAYER,
                x: p.body.pos.x,
                y: p.body.pos.y,
                flags: p.flags,
                held: p.inventory.held_kind(),
            });
        }
        for n in &self.npcs {
            groups.entry((n.body.floor, n.room)).or_default().push(EntityState {
                id: n.id,
                kind: proto::kind::NPC,
                x: n.body.pos.x,
                y: n.body.pos.y,
                flags: n.flags,
                held: 0,
            });
        }
        for c in &self.computers {
            let w = &self.workstations[c.station];
            let pos = Pos::tile_center(w.tile.x, w.tile.y);
            let room = self.building.floor(w.floor).map_or(0, |m| m.room_at(pos.x, pos.y));
            groups.entry((w.floor, room)).or_default().push(EntityState {
                id: c.handle,
                kind: proto::kind::COMPUTER,
                x: pos.x,
                y: pos.y,
                flags: computer::entity_flags(c),
                held: c.item.kind,
            });
        }
        for d in &self.dropped {
            let room = self.building.floor(d.floor).map_or(0, |m| m.room_at(d.pos.x, d.pos.y));
            groups.entry((d.floor, room)).or_default().push(EntityState {
                id: d.handle,
                kind: proto::kind::ITEM,
                x: d.pos.x,
                y: d.pos.y,
                flags: 0,
                held: d.item.kind,
            });
        }

        // 5. Snapshots + PlayerInfo for newly visible entities.
        let tick = self.tick;
        let ids: Vec<u16> =
            self.players.values().filter(|p| matches!(p.stage, Stage::Working)).map(|p| p.id).collect();
        let mut outgoing: Vec<(SocketAddr, u16, Packet)> = Vec::new();
        for id in ids {
            let p = &self.players[&id];
            let also: &[u16] = self.building.floor(p.body.floor).map_or(&[], |m| m.visible_from(p.room));
            let visible: Vec<EntityState> = std::iter::once(&p.room)
                .chain(also)
                .filter_map(|r| groups.get(&(p.body.floor, *r)))
                .flatten()
                .filter(|e| e.id != id)
                .copied()
                .collect();
            self.stats.max_visible = self.stats.max_visible.max(visible.len());
            let new_infos: Vec<PlayerInfoEntry> = visible
                .iter()
                .filter(|e| !p.known.contains(&e.id))
                .filter_map(|e| self.info_of(e.id))
                .collect();
            let me = SelfState {
                x: p.body.pos.x,
                y: p.body.pos.y,
                floor: p.body.floor,
                room: p.room,
                lock: p.body.lock,
                prev_input: p.body.prev_input,
                access: p.body.access,
                status: status_bits(p),
            };
            for f in proto::snapshot_fragments(tick, p.last_processed_seq, me, &visible) {
                outgoing.push((p.addr, id, f));
            }
            if p.inv_dirty || tick % INVENTORY_RESEND_TICKS == 0 {
                outgoing.push((p.addr, id, inventory_packet(&p.inventory)));
            }
            if tick % COMPUTER_RESEND_TICKS == 0 {
                if let Some(pk) = self.computer_packet(id) {
                    outgoing.push((p.addr, id, pk));
                }
            }
            for chunk in new_infos.chunks(INFO_PER_PACKET) {
                outgoing.push((p.addr, id, Packet::PlayerInfo { players: chunk.to_vec() }));
            }
            let p = self.players.get_mut(&id).unwrap();
            p.known.extend(new_infos.iter().map(|e| e.id));
            p.inv_dirty = false;
        }
        // 6. Speech (NPCs, and players' own "thought" lines): to everyone in
        //    the speaker's room, plus the addressee.
        for (npc_id, text, to) in says {
            let speaker = match self.npcs.iter().find(|n| n.id == npc_id) {
                Some(n) => Some(((n.body.floor, n.room), n.name.clone())),
                None => self.players.get(&npc_id).map(|p| ((p.body.floor, p.room), p.nick.clone())),
            };
            let Some((place, name)) = speaker else { continue };
            let info = self.info_of(npc_id).unwrap_or(PlayerInfoEntry {
                id: npc_id,
                nick: name,
                department: 0,
                gender: proto::gender::OTHER,
                appearance: proto::Appearance::default(),
            });
            for p in self.players.values_mut() {
                if (p.body.floor, p.room) == place || Some(p.id) == to || p.id == npc_id {
                    // Name first, so the line isn't shown as "?".
                    if p.id != npc_id && p.known.insert(npc_id) {
                        outgoing.push((p.addr, p.id, Packet::PlayerInfo { players: vec![info.clone()] }));
                    }
                    outgoing.push((p.addr, p.id, Packet::Say { id: npc_id, text: text.clone() }));
                }
            }
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

fn status_bits(p: &Player) -> u8 {
    (if p.cup.brewing() { proto::status::BREWING } else { 0 })
        | (if p.at_computer.is_some() { proto::status::AT_COMPUTER } else { 0 })
}

/// After an inventory change: access follows the carried items.
fn refresh(p: &mut Player) {
    p.body.access = p.inventory.access();
    p.inv_dirty = true;
}

fn dist2(a: Pos, b: Pos) -> i32 {
    let (dx, dy) = (a.x - b.x, a.y - b.y);
    dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
}

fn inventory_packet(inv: &Inventory) -> Packet {
    let slot = |it: &Option<Item>| match it {
        Some(i) => proto::SlotInfo { kind: i.kind, id: i.id, label: i.label.clone() },
        None => proto::SlotInfo::default(),
    };
    let mut slots = vec![slot(&inv.hands)];
    slots.extend(inv.pockets.iter().map(slot));
    Packet::Inventory { slots }
}

/// Check and normalise a character profile. `None` = reject.
pub fn validate_profile(mut p: Profile) -> Option<Profile> {
    let clean = |s: &str| s.chars().filter(|c| !c.is_control()).collect::<String>().trim().to_string();
    p.city = clean(&p.city);
    p.email = clean(&p.email).to_lowercase();
    let email_ok = {
        let e = &p.email;
        let parts: Vec<&str> = e.split('@').collect();
        parts.len() == 2
            && !parts[0].is_empty()
            && parts[1].contains('.')
            && !parts[1].starts_with('.')
            && !parts[1].ends_with('.')
            && !e.contains(char::is_whitespace)
    };
    let ok = p.gender <= proto::gender::OTHER
        && (18..=70).contains(&p.age)
        && !p.city.is_empty()
        && email_ok
        && p.appearance.is_valid();
    ok.then_some(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Appearance;

    fn good() -> Profile {
        Profile { gender: 0, age: 27, city: " Łódź ".into(), email: "Ola@Poczta.PL".into(), appearance: Appearance::default() }
    }

    #[test]
    fn profile_is_normalised() {
        let p = validate_profile(good()).unwrap();
        assert_eq!((p.city.as_str(), p.email.as_str()), ("Łódź", "ola@poczta.pl"));
    }

    #[test]
    fn bad_profiles_are_rejected() {
        let cases: Vec<Profile> = vec![
            Profile { age: 12, ..good() },
            Profile { age: 90, ..good() },
            Profile { gender: 7, ..good() },
            Profile { city: "   ".into(), ..good() },
            Profile { email: "ola".into(), ..good() },
            Profile { email: "ola@poczta".into(), ..good() },
            Profile { email: "o la@poczta.pl".into(), ..good() },
            Profile { email: "@poczta.pl".into(), ..good() },
            Profile { appearance: Appearance { shirt: 99, ..Appearance::default() }, ..good() },
        ];
        for c in cases {
            assert!(validate_profile(c.clone()).is_none(), "{c:?}");
        }
    }
}
