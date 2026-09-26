//! Load-test bots: N virtual clients that connect, then walk around the
//! building (stairs included) using BFS paths. A share of them heads to (and
//! wanders inside) one chosen room, on any floor.
//!
//! Each bot predicts its own movement exactly like the real client and
//! reconciles with the server, so the log also reports misprediction counts.

use std::collections::VecDeque;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use game::args::Args;
use game::building::{default_building_path, Building, Place};
use game::nav::Walker;
use game::protocol::{self as proto, Packet};
use game::sim::{self, Body, Pos};

const HELP: &str = "\
Startup sim - load-test bots

USAGE: cargo run --release --bin bots -- [OPTIONS]

OPTIONS:
  --server <addr>       server address, IPv4 or [IPv6]  [default: 127.0.0.1:7777]
  --count <n>           number of bots                  [default: 50]
  --room <name>         room some bots gather in        [default: Chill room]
  --room-share <0..1>   fraction of bots in that room   [default: 0.5]
  --all-in-room         same as --room-share 1
  --duration <secs>     stop after N seconds (0 = run forever) [default: 0]
  --map <path>          building JSON (must match the server)

Bots can only get past the card gates if the server runs with
--start-with-card; otherwise they stay in the public area.
";

const INPUT_REDUNDANCY: usize = 4;

enum State {
    Connecting { nonce: u32, next_send: Instant },
    Playing,
}

struct Bot {
    nick: String,
    sock: UdpSocket,
    state: State,
    id: u16,
    token: u32,
    seq: u32,
    pending: VecDeque<(u32, u8)>,
    pred: Body,
    have_pos: bool,
    last_ack: u32,
    last_tick: u32,
    floor: u8,
    room: u16,
    gather: bool,
    walker: Option<Walker>,
    idle_frames: u32,
    stuck_frames: u32,
    next_ping: Instant,
    // stats
    rtt_ms: f64,
    bytes_in: u64,
    visible: usize,
    corrections: u64,
}

fn main() {
    let args = Args::from_env();
    if args.flag("help") {
        print!("{HELP}");
        return;
    }
    let server: SocketAddr = args
        .str("server")
        .unwrap_or("127.0.0.1:7777")
        .to_socket_addrs()
        .ok()
        .and_then(|mut a| a.next())
        .expect("valid --server address");
    let count: usize = args.get("count", 50);
    let map_path = args.str("map").map(PathBuf::from).unwrap_or_else(default_building_path);
    let building = Building::load(&map_path).expect("building loads");
    let room_name = args.str("room").unwrap_or("Chill room").to_string();
    let (target_floor, target) = building
        .find_room(&room_name)
        .map(|(f, r)| (f, r.id))
        .unwrap_or_else(|| panic!("no room named '{room_name}'"));
    let share: f64 = if args.flag("all-in-room") { 1.0 } else { args.get("room-share", 0.5) };
    let duration: u64 = args.get("duration", 0);
    let n_room = ((count as f64) * share).round() as usize;

    // Goal pools; link tiles (stairs flights, elevator cabins) excluded.
    let pool = |f: u8, tiles: Vec<game::map::Tile>| -> Vec<Place> {
        let m = building.floor(f).unwrap();
        tiles.into_iter().filter(|t| m.link_at(t.x, t.y).is_none()).map(|t| (f, t)).collect()
    };
    let room_tiles = pool(target_floor, building.floor(target_floor).unwrap().room_tiles(target));
    let all_tiles: Vec<Place> = building.active_floors().flat_map(|(f, m)| pool(f, m.walkable_tiles())).collect();
    let start = Instant::now();
    let mut rng = fastrand::Rng::new();

    let mut bots: Vec<Bot> = (0..count)
        .map(|i| {
            let local = if server.is_ipv6() { "[::]:0" } else { "0.0.0.0:0" };
            let sock = UdpSocket::bind(local).expect("bind");
            sock.connect(server).expect("connect");
            sock.set_nonblocking(true).unwrap();
            Bot {
                nick: format!("bot_{i:02}"),
                sock,
                state: State::Connecting { nonce: rng.u32(..), next_send: start + Duration::from_millis(20 * i as u64) },
                id: 0,
                token: 0,
                seq: 0,
                pending: VecDeque::new(),
                pred: Body::at(0, Pos { x: 0, y: 0 }),
                have_pos: false,
                last_ack: 0,
                last_tick: 0,
                floor: 0,
                room: 0,
                gather: i < n_room,
                walker: None,
                idle_frames: 0,
                stuck_frames: 0,
                next_ping: start,
                rtt_ms: 0.0,
                bytes_in: 0,
                visible: 0,
                corrections: 0,
            }
        })
        .collect();

    println!("{count} bots -> {server}; {n_room} of them gather in '{room_name}' (floor {target_floor}, room {target})");
    let frame = Duration::from_nanos(1_000_000_000 / sim::INPUT_HZ as u64);
    let mut next_frame = Instant::now();
    let mut next_stats = Instant::now() + Duration::from_secs(5);
    let mut buf = [0u8; 2048];

    loop {
        let now = Instant::now();
        if duration > 0 && now.duration_since(start) > Duration::from_secs(duration) {
            for b in &bots {
                if matches!(b.state, State::Playing) {
                    let _ = b.sock.send(&Packet::Disconnect { token: b.token, reason: proto::disconnect::CLIENT_QUIT }.encode());
                }
            }
            break;
        }
        let client_ms = now.duration_since(start).as_millis() as u32;
        for b in bots.iter_mut() {
            // --- receive ---
            while let Ok(n) = b.sock.recv(&mut buf) {
                b.bytes_in += n as u64;
                let Ok(p) = Packet::decode(&buf[..n]) else { continue };
                b.handle(p, &building, client_ms);
            }
            // --- send ---
            match b.state {
                State::Connecting { nonce, next_send } => {
                    if now >= next_send {
                        let _ = b.sock.send(&Packet::Connect { nonce, nick: b.nick.clone() }.encode());
                        b.state = State::Connecting { nonce, next_send: now + Duration::from_millis(500) };
                    }
                }
                State::Playing => {
                    if b.have_pos {
                        let bits = b.think(&building, &room_tiles, &all_tiles, &mut rng);
                        b.seq += 1;
                        b.pending.push_back((b.seq, bits));
                        b.pred = sim::step(&building, b.pred, bits);
                        let k = b.pending.len().min(INPUT_REDUNDANCY);
                        let inputs: Vec<u8> = b.pending.iter().skip(b.pending.len() - k).map(|&(_, i)| i).collect();
                        let pkt = Packet::Input { token: b.token, ack_tick: b.last_tick, last_seq: b.seq, inputs };
                        let _ = b.sock.send(&pkt.encode());
                    }
                    if now >= b.next_ping {
                        let _ = b.sock.send(&Packet::Ping { token: b.token, client_time: client_ms }.encode());
                        b.next_ping = now + Duration::from_secs(1);
                    }
                }
            }
        }

        if now >= next_stats {
            let playing: Vec<&Bot> = bots.iter().filter(|b| matches!(b.state, State::Playing)).collect();
            let n = playing.len().max(1) as f64;
            let in_room = playing.iter().filter(|b| (b.floor, b.room) == (target_floor, target)).count();
            println!(
                "[{:>6.1}s] connected {}/{} | in '{}' {} | rtt avg {:.1} ms | recv avg {:.1} KB/s/bot | visible avg {:.1} max {} | mispredictions {}",
                now.duration_since(start).as_secs_f64(),
                playing.len(),
                bots.len(),
                room_name,
                in_room,
                playing.iter().map(|b| b.rtt_ms).sum::<f64>() / n,
                playing.iter().map(|b| b.bytes_in).sum::<u64>() as f64 / n / 5.0 / 1024.0,
                playing.iter().map(|b| b.visible).sum::<usize>() as f64 / n,
                playing.iter().map(|b| b.visible).max().unwrap_or(0),
                playing.iter().map(|b| b.corrections).sum::<u64>(),
            );
            for b in bots.iter_mut() {
                b.bytes_in = 0;
            }
            next_stats += Duration::from_secs(5);
        }

        next_frame += frame;
        let now = Instant::now();
        if next_frame > now {
            std::thread::sleep(next_frame - now);
        } else {
            next_frame = now; // overloaded: don't try to catch up
        }
    }
}

impl Bot {
    fn handle(&mut self, p: Packet, building: &Building, client_ms: u32) {
        match p {
            Packet::Welcome { nonce, player_id, token, map_crc, .. } => {
                if let State::Connecting { nonce: n, .. } = self.state {
                    if n == nonce {
                        assert_eq!(map_crc, building.crc, "server uses a different building");
                        self.id = player_id;
                        self.token = token;
                        self.state = State::Playing;
                    }
                }
            }
            Packet::Reject { reason } => eprintln!("{} rejected: {reason}", self.nick),
            Packet::Snapshot {
                tick,
                last_input_seq,
                frag_idx,
                self_x,
                self_y,
                floor,
                room,
                self_lock,
                self_prev_input,
                self_access,
                entities,
                ..
            } => {
                if tick < self.last_tick {
                    return; // out of order
                }
                if frag_idx == 0 || tick != self.last_tick {
                    self.visible = 0;
                }
                self.visible += entities.len();
                self.last_tick = tick;
                self.floor = floor;
                self.room = room;
                if last_input_seq < self.last_ack {
                    return;
                }
                self.last_ack = last_input_seq;
                // Reconcile: server state + replay of unacknowledged inputs.
                while self.pending.front().is_some_and(|&(s, _)| s <= last_input_seq) {
                    self.pending.pop_front();
                }
                let mut body = Body {
                    floor,
                    pos: Pos { x: self_x, y: self_y },
                    prev_input: self_prev_input,
                    lock: self_lock,
                    access: self_access,
                };
                for &(_, bits) in &self.pending {
                    body = sim::step(building, body, bits);
                }
                if self.have_pos && body != self.pred {
                    self.corrections += 1;
                }
                self.pred = body;
                self.have_pos = true;
            }
            Packet::Pong { client_time, .. } => {
                let rtt = client_ms.wrapping_sub(client_time) as f64;
                self.rtt_ms = if self.rtt_ms == 0.0 { rtt } else { self.rtt_ms * 0.8 + rtt * 0.2 };
            }
            Packet::Disconnect { .. } => {
                eprintln!("{} disconnected by server, reconnecting", self.nick);
                self.state = State::Connecting { nonce: fastrand::u32(..), next_send: Instant::now() };
                self.have_pos = false;
                self.pending.clear();
                self.walker = None;
                self.seq = 0;
                self.last_ack = 0;
                self.last_tick = 0;
            }
            _ => {}
        }
    }

    /// Choose this frame's input bits.
    fn think(&mut self, building: &Building, room_tiles: &[Place], all_tiles: &[Place], rng: &mut fastrand::Rng) -> u8 {
        if self.idle_frames > 0 {
            self.idle_frames -= 1;
            return 0;
        }
        if self.walker.as_ref().is_none_or(|w| w.done()) {
            let pool = if self.gather { room_tiles } else { all_tiles };
            let goal = pool[rng.usize(..pool.len())];
            self.walker = Walker::to(building, &self.pred, goal);
            self.stuck_frames = 0;
            if self.walker.is_none() {
                return 0; // unreachable goal (e.g. locked room); try another next frame
            }
        }
        let bits = self.walker.as_mut().unwrap().next_input(&self.pred);
        if bits == 0 {
            self.idle_frames = rng.u32(0..120); // pause up to 2 s at the goal
            return 0;
        }
        // Replan if no progress for a second.
        if sim::step(building, self.pred, bits).pos == self.pred.pos {
            self.stuck_frames += 1;
            if self.stuck_frames > 60 {
                self.walker = None;
            }
        }
        bits
    }
}
