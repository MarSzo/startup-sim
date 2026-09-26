//! End-to-end: real server on a random port, raw UDP test clients.

use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use game::building::{default_building_path, Building, Place};
use game::map::{access, Tile};
use game::npc::{lines, NPC_ID_BASE};
use game::nav::Walker;
use game::net::LinkConditions;
use game::protocol::{self as proto, Appearance, Packet, Profile};
use game::recruitment::{default_recruitment_path, Recruitment};
use game::server::{Config, Server};
use game::sim::{self, Body, Pos, IN_RIGHT};

fn test_profile() -> Profile {
    Profile {
        gender: proto::gender::FEMALE,
        age: 30,
        city: "Kraków".into(),
        email: "test@firma.pl".into(),
        appearance: Appearance { skin: 2, hair_style: 4, hair_color: 5, shirt: 6, pants: 1 },
    }
}

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
    start_server_cfg(start_access, skip_recruitment, false)
}

fn start_server_cfg(start_access: u8, skip_recruitment: bool, start_employed: bool) -> (SocketAddr, u32) {
    let map = building();
    let crc = map.crc;
    let cfg = Config {
        bind: "[::]:0".parse().unwrap(),
        link: LinkConditions::default(),
        max_players: 16,
        stats_every: Duration::from_secs(3600),
        client_timeout: Duration::from_millis(600),
        start_access,
        recruitment: {
            let mut r = Recruitment::load(&default_recruitment_path()).unwrap();
            r.invite_delay_secs = 0; // replies arrive on the next tick in tests
            r
        },
        skip_recruitment,
        start_employed,
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
        sock.send(&Packet::Connect { nonce: 42, nick: nick.into(), profile: test_profile() }.encode()).unwrap();
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
    sock.send_to(&Packet::Connect { nonce: 1, nick: "   ".into(), profile: test_profile() }.encode(), addr).unwrap();
    let mut buf = [0u8; 2048];
    let n = sock.recv(&mut buf).unwrap();
    assert_eq!(Packet::decode(&buf[..n]).unwrap(), Packet::Reject { reason: proto::reject::BAD_NICK });

    let mut bad = Packet::Connect { nonce: 1, nick: "x".into(), profile: test_profile() }.encode();
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
    // The card is in a pocket and the laptop in hands.
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut inv = None;
    while Instant::now() < deadline && inv.is_none() {
        g.ping();
        if let Some(Packet::Inventory { slots }) = g.recv() {
            inv = Some(slots);
        }
    }
    let slots = inv.expect("inventory");
    use game::inventory::kind as item_kind;
    assert_eq!(slots[0].kind, item_kind::LAPTOP, "laptop in hands");
    assert!(slots[1..].iter().any(|s| s.kind == item_kind::EMPLOYEE_CARD), "card in a pocket");
    assert!(!slots.iter().any(|s| s.kind == item_kind::GUEST_PASS), "guest pass taken back");
}

#[test]
fn desktop_portal_mail_interview_and_office() {
    use proto::portal_action as act;
    let (addr, _) = start_server_full(0, false);
    let bank = Recruitment::load(&default_recruitment_path()).unwrap();
    let (c, _) = Client::connect(addr, "Kandydat");

    // Wait for a packet matching `pred`; no world snapshots before being hired.
    let recv_until = |pred: &dyn Fn(&Packet) -> bool, allow_world: bool| -> Packet {
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            c.ping();
            if let Some(p) = c.recv() {
                assert!(allow_world || !matches!(p, Packet::Snapshot { .. }), "no world before going to the office");
                if pred(&p) {
                    return p;
                }
            }
        }
        panic!("expected packet not received");
    };
    // The offer list comes in several datagrams; merge them by id.
    let mut offers = std::collections::BTreeMap::new();
    while offers.len() < bank.offers.len() {
        let Packet::JobOffers { offers: part } = recv_until(&|p| matches!(p, Packet::JobOffers { .. }), false) else { unreachable!() };
        for o in part {
            offers.insert(o.id, o);
        }
    }
    assert!(offers.len() >= 7, "several companies on the portal");
    assert!(offers.values().filter(|o| o.company == "Startup Sim sp. z o.o.").count() == 4);

    // Another company answers with a (funny) rejection; a silent one never does.
    let apply = |offer: u8| {
        c.sock.send(&Packet::Apply { token: c.token, offer, motivation: "Bo lubię kawę.".into() }.encode()).unwrap();
    };
    apply(12);
    apply(11);
    let Packet::Mail { action, body, .. } = recv_until(&|p| matches!(p, Packet::Mail { from, .. } if from == "Pizzeria u Stefana"), false)
    else {
        unreachable!()
    };
    assert_eq!(action, act::NONE);
    assert!(body.contains("rowerem"));

    // Our startup: application -> invitation mail -> online interview.
    let interview = |want_correct: bool| -> Packet {
        apply(1);
        let Packet::Mail { action, arg, .. } = recv_until(&|p| matches!(p, Packet::Mail { action, .. } if *action == act::JOIN_INTERVIEW), false)
        else {
            unreachable!()
        };
        c.sock.send(&Packet::PortalAction { token: c.token, action, arg }.encode()).unwrap();
        loop {
            match recv_until(&|p| matches!(p, Packet::Question { .. } | Packet::RecruitResult { .. }), false) {
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
    let failed = interview(false);
    assert!(matches!(failed, Packet::RecruitResult { passed: false, score: 0, total: 3, .. }), "{failed:?}");
    recv_until(&|p| matches!(p, Packet::Mail { subject, .. } if subject.starts_with("Dziękujemy za rozmowę")), false);
    let hired = interview(true);
    assert!(matches!(hired, Packet::RecruitResult { passed: true, score: 3, total: 3, department: 1, .. }), "{hired:?}");

    // Invitation to the trial day -> "go to the office" -> in the world.
    let Packet::Mail { action, .. } = recv_until(&|p| matches!(p, Packet::Mail { action, .. } if *action == act::GO_TO_OFFICE), false)
    else {
        unreachable!()
    };
    c.sock.send(&Packet::PortalAction { token: c.token, action, arg: 0 }.encode()).unwrap();
    let b = building();
    let Packet::Snapshot { floor, room, self_access, .. } = recv_until(&|p| matches!(p, Packet::Snapshot { .. }), true) else {
        unreachable!()
    };
    assert_eq!((floor, b.floor(0).unwrap().room_name(room), self_access), (0, "Na zewnątrz", 0));
}

#[test]
fn coffee_machine_brews_one_cup_at_a_time() {
    use game::coffee::lines as coffee_lines;
    use game::inventory::kind as item_kind;
    let (addr, _) = start_server_with(access::CARD);
    let b = building();
    let (mut a, _) = Client::connect(addr, "Kawosz");
    let (mut c, _) = Client::connect(addr, "Drugi");
    let spawn = |i: usize| {
        let s = b.spawns()[i];
        Body { access: access::CARD, ..Body::at(s.0, Pos::tile_center(s.1.x, s.1.y)) }
    };
    // Both walk up to the chill room, in front of the machine (36,26).
    let at_a = a.walk_to(&b, spawn(0), (1, Tile { x: 36, y: 27 }), &[&c]);
    let at_c = c.walk_to(&b, spawn(1), (1, Tile { x: 35, y: 27 }), &[&a]);

    // A presses E: brewing starts; A's own bubble says so.
    a.press_e(&b, at_a);
    let seen = a.wait_for_line(coffee_lines::BREWING, Duration::from_millis(500));
    assert!(seen.is_some(), "A starts brewing");
    // C tries meanwhile: the machine is busy.
    c.press_e(&b, at_c);
    assert!(c.wait_for_line(coffee_lines::BUSY, Duration::from_millis(500)).is_some(), "one at a time");

    // After ~3 s A holds a coffee: in A's own status and in A's flags for C.
    let deadline = Instant::now() + Duration::from_secs(4);
    let (mut ready, mut self_holding) = (false, false);
    while Instant::now() < deadline && !ready {
        c.ping(); // keep C's session alive while we wait (test timeout is 0.6 s)
        a.ping();
        while let Some(p) = a.recv() {
            match p {
                Packet::Say { text, .. } => ready |= text == coffee_lines::READY,
                // Sent in the same tick as the line: don't miss it.
                Packet::Inventory { slots } => self_holding |= slots[0].kind == item_kind::COFFEE,
                _ => {}
            }
        }
        while c.recv().is_some() {}
    }
    assert!(ready, "coffee ready");
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut others_see = false;
    while Instant::now() < deadline && !(self_holding && others_see) {
        a.ping();
        c.ping();
        while let Some(p) = a.recv() {
            if let Packet::Inventory { slots } = p {
                self_holding |= slots[0].kind == item_kind::COFFEE; // slot 0 = hands
            }
        }
        while let Some(p) = c.recv() {
            if let Packet::Snapshot { entities, .. } = p {
                others_see |= entities
                    .iter()
                    .any(|e| e.id == a.id && e.held == item_kind::COFFEE);
            }
        }
    }
    assert!(self_holding, "A's inventory: coffee in hands");
    assert!(others_see, "C sees A with a mug");
}

#[test]
fn profile_is_checked_and_appearance_shared() {
    let (addr, _) = start_server();
    // Invalid e-mail -> rejected.
    let sock = UdpSocket::bind("127.0.0.1:0").unwrap();
    sock.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
    let bad = Profile { email: "nie-email".into(), ..test_profile() };
    sock.send_to(&Packet::Connect { nonce: 1, nick: "Zly".into(), profile: bad }.encode(), addr).unwrap();
    let mut buf = [0u8; 2048];
    let n = sock.recv(&mut buf).unwrap();
    assert_eq!(Packet::decode(&buf[..n]).unwrap(), Packet::Reject { reason: proto::reject::BAD_PROFILE });

    // Others see name, gender and appearance - never age, city or e-mail.
    let (a, _) = Client::connect(addr, "Ola");
    let (b, _) = Client::connect(addr, "Obserwator");
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut seen = None;
    while Instant::now() < deadline && seen.is_none() {
        a.ping();
        b.ping();
        if let Some(Packet::PlayerInfo { players }) = b.recv() {
            seen = players.into_iter().find(|p| p.id == a.id);
        }
    }
    let info = seen.expect("B learns about A");
    assert_eq!(info.nick, "Ola");
    assert_eq!(info.gender, proto::gender::FEMALE);
    assert_eq!(info.appearance, test_profile().appearance);
}

#[test]
fn access_card_can_be_dropped_picked_up_and_handed_over() {
    use game::inventory::kind as item_kind;
    use proto::item_action as act;
    let (addr, _) = start_server_with(access::CARD); // everyone starts with a card
    let b = building();
    let (a, _) = Client::connect(addr, "Ola");
    let (mut c, _) = Client::connect(addr, "Kuba");

    // Latest (access, hands, pockets) of a client, keeping both sessions alive.
    let state = |me: &Client, other: &Client, wait: Duration| {
        let deadline = Instant::now() + wait;
        let (mut acc, mut inv) = (None, None);
        while Instant::now() < deadline {
            me.ping();
            other.ping();
            while let Some(p) = me.recv() {
                match p {
                    Packet::Snapshot { self_access, .. } => acc = Some(self_access),
                    Packet::Inventory { slots } => inv = Some(slots),
                    _ => {}
                }
            }
            while other.recv().is_some() {}
        }
        (acc.unwrap_or(0), inv.unwrap_or_default())
    };
    let item_action = |who: &Client, action: u8, slot: u8| {
        who.sock.send(&Packet::ItemAction { token: who.token, action, slot }.encode()).unwrap();
    };

    let (acc, inv) = state(&a, &c, Duration::from_millis(300));
    assert_eq!(acc, access::CARD);
    let card_slot = inv.iter().position(|s| s.kind == item_kind::EMPLOYEE_CARD).expect("card in pockets") - 1;

    // Ola takes the card out and drops it on the sidewalk: no access any more.
    item_action(&a, act::TAKE_OUT, card_slot as u8);
    std::thread::sleep(Duration::from_millis(120));
    item_action(&a, act::DROP, 0);
    let (acc, inv) = state(&a, &c, Duration::from_millis(300));
    assert_eq!(acc, 0, "no card, no access");
    assert!(inv.iter().all(|s| s.kind == item_kind::NONE));

    // Kuba (with his own card) sees it on the floor, walks there and picks it up (E).
    let spot = b.spawns()[0];
    let kuba = Body { access: access::CARD, ..Body::at(0, Pos::tile_center(b.spawns()[1].1.x, b.spawns()[1].1.y)) };
    let at = c.walk_to(&b, kuba, spot, &[&a]);
    c.press_e(&b, at);
    let (acc, inv) = state(&c, &a, Duration::from_millis(400));
    assert_eq!(acc, access::CARD);
    assert_eq!(inv.iter().filter(|s| s.kind == item_kind::EMPLOYEE_CARD).count(), 2, "Kuba now carries two cards");

    // Kuba hands one back to Ola (G): she is standing right there.
    let slot = inv[1..].iter().position(|s| s.kind == item_kind::EMPLOYEE_CARD).unwrap();
    item_action(&c, act::TAKE_OUT, slot as u8);
    std::thread::sleep(Duration::from_millis(120));
    item_action(&c, act::GIVE, 0);
    let (acc, inv) = state(&a, &c, Duration::from_millis(400));
    assert_eq!(acc, access::CARD, "access came back with the card");
    assert_eq!(inv.iter().filter(|s| s.kind == item_kind::EMPLOYEE_CARD).count(), 1);
}

/// Wait (pinging `keep` too) for a packet matching `f`.
fn wait_for<T>(me: &Client, keep: &[&Client], wait: Duration, mut f: impl FnMut(&Packet) -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        me.ping();
        for k in keep {
            k.ping();
            while k.recv().is_some() {}
        }
        while let Some(p) = me.recv() {
            if let Some(t) = f(&p) {
                return Some(t);
            }
        }
    }
    None
}

#[test]
fn laptop_on_desk_messenger_lock_and_take() {
    use game::computer::{conv, lines as pc};
    use game::inventory::kind as item_kind;
    use proto::computer_action as ca;
    let (addr, _) = start_server_cfg(0, true, true); // hired: card + laptop, at a desk
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola"); // id 1: IT
    let (mut kuba, _) = Client::connect(addr, "Kuba"); // id 2: Biznes
    let (mut ewa, _) = Client::connect(addr, "Ewa"); // id 3: IT, next desk
    assert_eq!((ola.id, kuba.id, ewa.id), (1, 2, 3));
    let action = |who: &Client, action: u8, conv: u16, arg: u32, text: &str| {
        let p = Packet::ComputerAction { token: who.token, action, conv, arg, text: text.into() };
        who.sock.send(&p.encode()).unwrap();
    };
    let said = |line: &'static str| move |p: &Packet| matches!(p, Packet::Say { text, .. } if text == line).then_some(());
    let screen = |p: &Packet| match p {
        Packet::Computer { owner, locked, convs, .. } => Some((*owner, *locked, convs.clone())),
        _ => None,
    };
    let status = |p: &Packet| match p {
        Packet::Snapshot { self_status, .. } => Some(*self_status),
        _ => None,
    };
    let at_computer = |c: &Client, keep: &[&Client], want: bool| {
        wait_for(c, keep, Duration::from_millis(800), |p| {
            status(p).filter(|s| (s & proto::status::AT_COMPUTER != 0) == want)
        })
        .is_some()
    };
    let wait = Duration::from_millis(800);
    let nobody = Body::at(1, Pos::tile_center(0, 0)); // E doesn't move anyone

    // Ola and Kuba put their laptops down (E) and sit at them (E again).
    ola.press_e(&b, nobody);
    assert!(wait_for(&ola, &[&kuba, &ewa], wait, said(pc::PLACED)).is_some());
    kuba.press_e(&b, nobody);
    assert!(wait_for(&kuba, &[&ola, &ewa], wait, said(pc::PLACED)).is_some());
    ola.press_e(&b, nobody);
    let (owner, locked, convs) = wait_for(&ola, &[&kuba, &ewa], wait, screen).expect("Ola's screen");
    assert_eq!((owner, locked), (ola.id, false));
    let titles: Vec<&str> = convs.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, ["#ogólny", "#it-produkt", "Ewa", "Kuba"]);
    assert!(at_computer(&ola, &[&kuba, &ewa], true));

    // Ola says hi on #ogólny (the message comes back to her screen) and walks off
    // without locking: she just closes the screen.
    action(&ola, ca::SEND, conv::GENERAL, 1, "Cześć wszystkim!");
    let echo = wait_for(&ola, &[&kuba, &ewa], wait, |p| match p {
        Packet::Chat { conv: c, messages } if *c == conv::GENERAL => messages.first().cloned(),
        _ => None,
    });
    assert_eq!(echo.map(|m| (m.from, m.text)), Some((ola.id, "Cześć wszystkim!".to_string())));
    action(&ola, ca::CLOSE, 0, 0, "");
    assert!(at_computer(&ola, &[&kuba, &ewa], false));

    // Ewa sneaks to Ola's desk: the computer is logged in as Ola, so her DM to
    // Kuba goes out in Ola's name.
    let seat = |id: u16| -> Body {
        let ws = game::computer::find_workstations(&b);
        let it: Vec<_> = ws.iter().filter(|w| w.room_name == "IT / Produkt").collect();
        let w = it[(id as usize - 1) / 2];
        Body { access: access::CARD, ..Body::at(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1)) }
    };
    let (ola_seat, ewa_seat) = (seat(1), seat(3));
    let at = ewa.walk_to(&b, ewa_seat, (1, Tile { x: ola_seat.pos.tile().0, y: ola_seat.pos.tile().1 }), &[&ola, &kuba]);
    ewa.press_e(&b, at);
    let (owner, locked, _) = wait_for(&ewa, &[&ola, &kuba], wait, screen).expect("Ola's screen for Ewa");
    assert_eq!((owner, locked), (ola.id, false));
    action(&ewa, ca::SEND, conv::DM | kuba.id, 7, "Stawiam wszystkim pizzę!");
    std::thread::sleep(Duration::from_millis(100));

    // Kuba opens his computer: one unread DM from "Ola".
    kuba.press_e(&b, nobody);
    let convs = wait_for(&kuba, &[&ola, &ewa], wait, |p| screen(p).map(|s| s.2)).expect("Kuba's screen");
    let dm = convs.iter().find(|c| c.conv == conv::DM | ola.id).expect("DM with Ola");
    assert_eq!(dm.unread, 1);
    assert!(convs.iter().any(|c| c.title == "#biznes") && !convs.iter().any(|c| c.title == "#it-produkt"));
    action(&kuba, ca::SYNC, conv::DM | ola.id, 0, "");
    let got = wait_for(&kuba, &[&ola, &ewa], wait, |p| match p {
        Packet::Chat { messages, .. } => messages.first().cloned(),
        _ => None,
    });
    assert_eq!(got.map(|m| (m.from, m.nick, m.text)), Some((ola.id, "Ola".into(), "Stawiam wszystkim pizzę!".into())));

    // Ewa locks it (anyone may), then can't unlock it, but can take the laptop.
    action(&ewa, ca::LOCK, 0, 0, "");
    assert!(at_computer(&ewa, &[&ola, &kuba], false));
    ewa.press_e(&b, at);
    let (_, locked, convs) = wait_for(&ewa, &[&ola, &kuba], wait, screen).expect("lock screen");
    assert!(locked && convs.is_empty(), "a locked screen shows nothing");
    action(&ewa, ca::UNLOCK, 0, 0, "");
    assert!(wait_for(&ewa, &[&ola, &kuba], wait, said(pc::LOCKED)).is_some());
    // Her own laptop is still in her hands: she has to put it away first... she can't
    // (too big), so she drops it and takes Ola's.
    action(&ewa, ca::TAKE, 0, 0, "");
    assert!(wait_for(&ewa, &[&ola, &kuba], wait, said(pc::HANDS_FULL)).is_some());
    ewa.sock.send(&Packet::ItemAction { token: ewa.token, action: proto::item_action::DROP, slot: 0 }.encode()).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    action(&ewa, ca::TAKE, 0, 0, "");
    let hands = wait_for(&ewa, &[&ola, &kuba], wait, |p| match p {
        Packet::Inventory { slots } if slots[0].kind == item_kind::LAPTOP && slots[0].label.contains("Ola") => Some(()),
        _ => None,
    });
    assert!(hands.is_some(), "Ewa carries Ola's laptop");
    // The desk is empty now: no computer entity left in Ola's view.
    let computers = wait_for(&ola, &[&kuba, &ewa], wait, |p| match p {
        Packet::Snapshot { entities, .. } => Some(entities.iter().filter(|e| e.kind == proto::kind::COMPUTER).count()),
        _ => None,
    });
    assert_eq!(computers, Some(0), "Kuba's computer is in Biznes, Ola's is gone");
}
