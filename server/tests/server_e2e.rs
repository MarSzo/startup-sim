//! End-to-end: real server on a random port, raw UDP test clients.

use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use game::building::{default_building_path, Building, Place};
use game::map::{access, Tile};
use game::npc::{lines, NPC_ID_BASE};
use game::nav::Walker;
use game::net::LinkConditions;
use game::protocol::{self as proto, Packet};
use game::recruitment::{default_recruitment_path, Recruitment};
use game::server::{Config, Server};
use game::sim::{self, Body, Pos, IN_RIGHT};

fn building() -> Building {
    Building::load(&default_building_path()).unwrap()
}

/// Server on a random port, dual-stack. Returns (IPv4 loopback addr, building crc).
fn start_server() -> (SocketAddr, u32) {
    start_server_with(0)
}

/// Same, with every player starting with rights `start_access`.
fn start_server_with(start_access: u8) -> (SocketAddr, u32) {
    start_server_full(start_access, true)
}

/// Full control: `skip_recruitment = false` puts new players on the job portal.
fn start_server_full(start_access: u8, skip_recruitment: bool) -> (SocketAddr, u32) {
    let map = building();
    let crc = map.crc;
    let cfg = Config {
        bind: "[::]:0".parse().unwrap(),
        link: LinkConditions::default(),
        max_players: 16,
        stats_every: Duration::from_secs(3600),
        client_timeout: Duration::from_millis(600),
        start_access,
        recruitment: Recruitment::load(&default_recruitment_path()).unwrap(),
        skip_recruitment,
    };
    let mut server = Server::new(map, cfg).unwrap();
    let port = server.local_addr().port();
    std::thread::spawn(move || server.run());
    (SocketAddr::from(([127, 0, 0, 1], port)), crc)
}

fn v6(addr: SocketAddr) -> SocketAddr {
    format!("[::1]:{}", addr.port()).parse().unwrap()
}

struct Client {
    sock: UdpSocket,
    id: u16,
    token: u32,
    seq: u32,
}

impl Client {
    fn socket_for(server: SocketAddr) -> UdpSocket {
        let sock = UdpSocket::bind(if server.is_ipv6() { "[::1]:0" } else { "127.0.0.1:0" }).unwrap();
        sock.connect(server).unwrap();
        sock.set_read_timeout(Some(Duration::from_millis(20))).unwrap();
        sock
    }

    fn connect(server: SocketAddr, nick: &str) -> (Client, u32) {
        let sock = Client::socket_for(server);
        sock.send(&Packet::Connect { nonce: 42, nick: nick.into() }.encode()).unwrap();
        let mut c = Client { sock, id: 0, token: 0, seq: 0 };
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if let Some(Packet::Welcome { player_id, token, map_crc, nonce, .. }) = c.recv() {
                assert_eq!(nonce, 42);
                c.id = player_id;
                c.token = token;
                return (c, map_crc);
            }
        }
        panic!("no Welcome");
    }

    fn recv(&self) -> Option<Packet> {
        let mut buf = [0u8; 2048];
        let n = self.sock.recv(&mut buf).ok()?;
        assert!(n <= proto::MAX_PACKET);
        Some(Packet::decode(&buf[..n]).expect("server sent a valid packet"))
    }

    fn send_inputs(&mut self, bits: u8, n: usize) {
        self.seq += n as u32;
        let p = Packet::Input { token: self.token, ack_tick: 0, last_seq: self.seq, inputs: vec![bits; n] };
        self.sock.send(&p.encode()).unwrap();
    }

    /// Latest snapshot (tick, room, pos, visible ids) seen within `wait`.
    fn latest_snapshot(&self, wait: Duration) -> Option<(u32, u16, (i32, i32), Vec<u16>, u32)> {
        let deadline = Instant::now() + wait;
        let mut last = None;
        while Instant::now() < deadline {
            if let Some(Packet::Snapshot { tick, room, self_x, self_y, entities, last_input_seq, .. }) = self.recv() {
                last = Some((tick, room, (self_x, self_y), entities.iter().map(|e| e.id).collect(), last_input_seq));
            }
        }
        last
    }

    /// Walk to `goal` like a real client: predict locally with the shared
    /// simulation, send 6 inputs every 50 ms (the server's per-tick budget).
    /// Returns the predicted final body.
    fn walk_to(&mut self, b: &Building, body: Body, goal: Place, others: &[&Client]) -> Body {
        let mut body = body;
        let mut w = Walker::to(b, &body, goal).expect("reachable");
        while !w.done() {
            let mut batch = Vec::new();
            for _ in 0..6 {
                let i = w.next_input(&body);
                body = sim::step(b, body, i);
                batch.push(i);
            }
            self.seq += batch.len() as u32;
            let p = Packet::Input { token: self.token, ack_tick: 0, last_seq: self.seq, inputs: batch };
            self.sock.send(&p.encode()).unwrap();
            for o in others {
                o.ping();
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        body
    }

    /// Press E once (input 0 then interact); returns the predicted body.
    fn press_e(&mut self, b: &Building, body: Body) -> Body {
        self.seq += 2;
        let p = Packet::Input { token: self.token, ack_tick: 0, last_seq: self.seq, inputs: vec![0, sim::IN_INTERACT] };
        self.sock.send(&p.encode()).unwrap();
        sim::step(b, sim::step(b, body, 0), sim::IN_INTERACT)
    }

    /// Wait (pinging) until an NPC says `line`; returns the latest self_access seen.
    fn wait_for_line(&self, line: &str, wait: Duration) -> Option<u8> {
        let deadline = Instant::now() + wait;
        let mut access = None;
        let mut heard = false;
        while Instant::now() < deadline {
            self.ping();
            match self.recv() {
                Some(Packet::Say { text, .. }) => heard |= text == line,
                Some(Packet::Snapshot { self_access, .. }) => access = Some(self_access),
                _ => {}
            }
            if heard && access.is_some() {
                break;
            }
        }
        heard.then_some(access.unwrap_or(0))
    }

    fn ping(&self) {
        self.sock.send(&Packet::Ping { token: self.token, client_time: 1 }.encode()).unwrap();
    }
}

#[test]
fn handshake_interest_and_timeout() {
    let (addr, crc) = start_server();
    let b0 = building();
    let outside = b0.floor(0).unwrap().room_by_name("Na zewnątrz").unwrap().id;
    let lobby = b0.floor(0).unwrap().room_by_name("Wejście").unwrap().id;

    let (a, a_crc) = Client::connect(addr, "Ala");
    let (mut b, _) = Client::connect(addr, "Bob");
    assert_eq!(a_crc, crc);
    assert_ne!(a.id, b.id);

    // Both spawn outside and see each other; A learns B's nick.
    let mut got_info = false;
    let mut saw_b = false;
    let deadline = Instant::now() + Duration::from_millis(500);
    while Instant::now() < deadline {
        a.ping();
        b.ping();
        match a.recv() {
            Some(Packet::PlayerInfo { players }) => got_info |= players.iter().any(|p| p.id == b.id && p.nick == "Bob"),
            Some(Packet::Snapshot { room, entities, .. }) => {
                assert_eq!(room, outside);
                saw_b |= entities.iter().any(|e| e.id == b.id);
            }
            _ => {}
        }
    }
    assert!(saw_b && got_info, "A should see B and get its nick");

    // B walks in through the glass doors into the lobby.
    let spawn = b0.spawns()[1];
    let b_body = Body::at(spawn.0, Pos::tile_center(spawn.1.x, spawn.1.y));
    b.walk_to(&b0, b_body, (0, Tile { x: 33, y: 28 }), &[&a]);
    a.ping();
    let (_, b_room, _, _, b_ack) = b.latest_snapshot(Duration::from_millis(300)).unwrap();
    a.ping();
    assert_eq!(b_ack, b.seq, "server processed all inputs");
    assert_eq!(b_room, lobby, "B entered the lobby");
    let (_, a_room, _, a_visible, _) = a.latest_snapshot(Duration::from_millis(200)).unwrap();
    assert_eq!(a_room, outside);
    assert!(!a_visible.contains(&b.id), "B no longer visible to A after changing rooms");

    // A goes silent -> gets a timeout Disconnect.
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut timed_out = false;
    while Instant::now() < deadline && !timed_out {
        b.ping();
        if let Some(Packet::Disconnect { reason, .. }) = a.recv() {
            assert_eq!(reason, proto::disconnect::TIMEOUT);
            timed_out = true;
        }
    }
    assert!(timed_out);
}

#[test]
fn rejects_empty_nick_and_bad_version() {
    let (addr, _) = start_server();
    let sock = UdpSocket::bind("127.0.0.1:0").unwrap();
    sock.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
    sock.send_to(&Packet::Connect { nonce: 1, nick: "   ".into() }.encode(), addr).unwrap();
    let mut buf = [0u8; 2048];
    let n = sock.recv(&mut buf).unwrap();
    assert_eq!(Packet::decode(&buf[..n]).unwrap(), Packet::Reject { reason: proto::reject::BAD_NICK });

    let mut bad = Packet::Connect { nonce: 1, nick: "x".into() }.encode();
    bad[2] = 99;
    sock.send_to(&bad, addr).unwrap();
    let n = sock.recv(&mut buf).unwrap();
    assert_eq!(Packet::decode(&buf[..n]).unwrap(), Packet::Reject { reason: proto::reject::BAD_VERSION });
}

/// Collect entity ids seen in snapshots for `wait`, pinging to stay alive.
fn visible_ids(c: &Client, others: &[&Client], wait: Duration) -> Vec<u16> {
    let deadline = Instant::now() + wait;
    let mut ids = Vec::new();
    while Instant::now() < deadline {
        c.ping();
        for o in others {
            o.ping();
        }
        if let Some(Packet::Snapshot { entities, .. }) = c.recv() {
            ids = entities.iter().map(|e| e.id).collect();
        }
    }
    ids
}

#[test]
fn ipv4_and_ipv6_clients_share_one_world() {
    let (addr4, _) = start_server();
    let (a, _) = Client::connect(addr4, "ipv4");
    let (b, _) = Client::connect(v6(addr4), "ipv6");
    assert!(visible_ids(&a, &[&b], Duration::from_millis(300)).contains(&b.id));
    assert!(visible_ids(&b, &[&a], Duration::from_millis(300)).contains(&a.id));
}

#[test]
fn session_survives_address_change() {
    let (addr, _) = start_server();
    let (mut a, _) = Client::connect(addr, "roamer");
    let (b, _) = Client::connect(addr, "watcher");
    assert!(visible_ids(&b, &[&a], Duration::from_millis(300)).contains(&a.id));

    // "Wi-Fi -> LTE": same token, new socket (new source port), even another IP family.
    let old = std::mem::replace(&mut a.sock, Client::socket_for(v6(addr)));
    a.send_inputs(IN_RIGHT, 3);
    a.ping();
    let (_, _, _, _, ack) = a.latest_snapshot(Duration::from_millis(300)).expect("snapshots follow the new address");
    assert_eq!(ack, a.seq, "inputs from the new address are applied");

    // Old address gets nothing new; the player keeps its id for others.
    while old.recv(&mut [0u8; 2048]).is_ok() {}
    std::thread::sleep(Duration::from_millis(150));
    assert!(old.recv(&mut [0u8; 2048]).is_err(), "old address no longer receives");
    assert!(visible_ids(&b, &[&a], Duration::from_millis(200)).contains(&a.id));

    // A late, stale input from the old address must not steal the session back.
    old.send(&Packet::Input { token: a.token, ack_tick: 0, last_seq: 1, inputs: vec![0] }.encode()).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert!(a.latest_snapshot(Duration::from_millis(200)).is_some(), "still served on the new address");
}

#[test]
fn unknown_token_is_told_to_reconnect() {
    let (addr, _) = start_server();
    let sock = Client::socket_for(addr);
    sock.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
    sock.send(&Packet::Ping { token: 12345, client_time: 0 }.encode()).unwrap();
    let mut buf = [0u8; 2048];
    let n = sock.recv(&mut buf).unwrap();
    assert_eq!(
        Packet::decode(&buf[..n]).unwrap(),
        Packet::Disconnect { token: 12345, reason: proto::disconnect::SESSION_UNKNOWN }
    );
}

#[test]
fn other_floors_are_invisible_and_state_matches_prediction() {
    let (addr, _) = start_server_with(access::CARD);
    let b0 = building();
    let (a, _) = Client::connect(addr, "downstairs");
    let (mut b, _) = Client::connect(addr, "upstairs");
    let spawn = b0.spawns()[1];
    let start = Body { access: access::CARD, ..Body::at(spawn.0, Pos::tile_center(spawn.1.x, spawn.1.y)) };
    // Up the stairs to the reception on floor 1: A stays outside.
    let predicted = b.walk_to(&b0, start, (1, Tile { x: 25, y: 18 }), &[&a]);
    assert_eq!(predicted.floor, 1);

    let deadline = Instant::now() + Duration::from_millis(400);
    let mut last = None;
    while Instant::now() < deadline {
        a.ping();
        if let Some(Packet::Snapshot { floor, room, self_x, self_y, self_lock, self_prev_input, last_input_seq, .. }) = b.recv() {
            last = Some((floor, room, self_x, self_y, self_lock, self_prev_input, last_input_seq));
        }
    }
    let (floor, room, x, y, lock, prev, ack) = last.expect("B gets snapshots");
    assert_eq!(ack, b.seq);
    let server = Body { floor, pos: Pos { x, y }, prev_input: prev, lock, access: access::CARD };
    assert_eq!(server, predicted, "server state == client prediction, bit for bit");
    assert_eq!(b0.floor(1).unwrap().room_name(room), "Recepcja");

    assert!(!visible_ids(&a, &[&b], Duration::from_millis(200)).contains(&b.id), "A (floor 0) can't see B");
    assert!(!visible_ids(&b, &[&a], Duration::from_millis(200)).contains(&a.id), "B (floor 1) can't see A");
}

#[test]
fn onboarding_porter_reception_hr_card() {
    let (addr, _) = start_server();
    let b0 = building();
    let (mut g, _) = Client::connect(addr, "Nowy");
    let spawn = b0.spawns()[0];
    let start = Body::at(spawn.0, Pos::tile_center(spawn.1.x, spawn.1.y));

    // Without a pass the gates stop you: walking to the hall ends in the lobby.
    assert!(Walker::to(&b0, &start, (0, Tile { x: 34, y: 17 })).is_none(), "no path without a pass");

    // Walk to the lodge door and press E next to the porter.
    let body = g.walk_to(&b0, start, (0, Tile { x: 27, y: 29 }), &[]);
    let body = g.press_e(&b0, body);

    let (mut welcomed, mut got_pass, mut porter_named) = (false, false, false);
    let deadline = Instant::now() + Duration::from_millis(800);
    while Instant::now() < deadline {
        g.ping();
        match g.recv() {
            Some(Packet::Say { id, text }) => {
                assert!(id >= NPC_ID_BASE);
                welcomed |= text == lines::WELCOME_ESCORT;
            }
            Some(Packet::Snapshot { self_access, .. }) => got_pass |= self_access == access::GUEST,
            Some(Packet::PlayerInfo { players }) => porter_named |= players.iter().any(|p| p.nick == "Portier"),
            _ => {}
        }
    }
    assert!(welcomed, "porter greets");
    assert!(got_pass, "guest pass granted");
    assert!(porter_named, "porter comes out into the lobby and is visible by name");

    // Follow him up to the reception; he announces the arrival there.
    let body = Body { access: access::GUEST, ..body };
    let end = g.walk_to(&b0, body, (1, Tile { x: 32, y: 18 }), &[]);
    assert_eq!(end.floor, 1);
    assert!(g.wait_for_line(lines::ARRIVED, Duration::from_secs(8)).is_some(), "porter reached the reception");

    // Reception takes us to HR.
    let body = g.press_e(&b0, end);
    assert!(g.wait_for_line(lines::RECEPTION_WELCOME, Duration::from_secs(1)).is_some(), "reception greets");
    let at_hr = g.walk_to(&b0, body, (1, Tile { x: 43, y: 8 }), &[]);
    assert!(g.wait_for_line(lines::RECEPTION_ARRIVED, Duration::from_secs(6)).is_some(), "receptionist reached HR");

    // HR: contract signed, the card replaces the guest pass.
    g.press_e(&b0, at_hr);
    let access = g.wait_for_line(lines::HR_SIGNED, Duration::from_secs(1));
    let deadline = Instant::now() + Duration::from_millis(300);
    let mut latest = access;
    while Instant::now() < deadline {
        if let Some(Packet::Snapshot { self_access, .. }) = g.recv() {
            latest = Some(self_access);
        }
    }
    assert!(access.is_some(), "HR signed the contract");
    assert_eq!(latest, Some(access::CARD), "employee card, guest pass gone");
}

#[test]
fn job_portal_rejects_then_hires_and_spawns() {
    let (addr, _) = start_server_full(0, false);
    let bank = Recruitment::load(&default_recruitment_path()).unwrap();
    let (c, _) = Client::connect(addr, "Kandydat");

    // Next packets: the portal. No snapshots while not hired.
    let recv_until = |pred: &dyn Fn(&Packet) -> bool| -> Packet {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            c.ping();
            if let Some(p) = c.recv() {
                assert!(!matches!(p, Packet::Snapshot { .. }), "no world before being hired");
                if pred(&p) {
                    return p;
                }
            }
        }
        panic!("expected packet not received");
    };
    let Packet::JobOffers { offers } = recv_until(&|p| matches!(p, Packet::JobOffers { .. })) else { unreachable!() };
    let titles: Vec<&str> = offers.iter().map(|o| o.title.as_str()).collect();
    assert_eq!(titles, vec!["Programista/ka", "Marketing i sprzedaż"]);

    // Answers: wrong on purpose, then right (the data's first option is correct).
    let answer_all = |want_correct: bool| -> Packet {
        c.sock.send(&Packet::Apply { token: c.token, offer: 1 }.encode()).unwrap();
        loop {
            match recv_until(&|p| matches!(p, Packet::Question { .. } | Packet::RecruitResult { .. })) {
                Packet::Question { attempt, index, text, options, .. } => {
                    let q = bank.offer(1).unwrap().questions.iter().find(|q| q.text == text).expect("known question");
                    let right = options.iter().position(|o| *o == q.options[0]).unwrap() as u8;
                    let choice = if want_correct { right } else { (right + 1) % options.len() as u8 };
                    c.sock.send(&Packet::Answer { token: c.token, attempt, index, choice }.encode()).unwrap();
                }
                result => return result,
            }
        }
    };
    let failed = answer_all(false);
    assert!(matches!(failed, Packet::RecruitResult { passed: false, score: 0, total: 3, department: 0, .. }), "{failed:?}");
    recv_until(&|p| matches!(p, Packet::JobOffers { .. })); // back on the portal
    let hired = answer_all(true);
    assert!(matches!(hired, Packet::RecruitResult { passed: true, score: 3, total: 3, department: 1, .. }), "{hired:?}");

    // Now in the world: outside, in front of the building, no pass yet.
    let b = building();
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut spawned = None;
    while Instant::now() < deadline && spawned.is_none() {
        c.ping();
        if let Some(Packet::Snapshot { floor, room, self_access, .. }) = c.recv() {
            spawned = Some((floor, room, self_access));
        }
    }
    let (floor, room, access) = spawned.expect("snapshots after hiring");
    assert_eq!((floor, b.floor(0).unwrap().room_name(room), access), (0, "Na zewnątrz", 0));
}
