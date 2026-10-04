//! End-to-end: real server on a random port, raw UDP test clients.

#![allow(clippy::unwrap_used)] // test / dev tool: a panic is the right report

use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use game::building::{default_building_path, Building, Place};
use game::map::{access, Tile};
use game::nav::Walker;
use game::net::LinkConditions;
use game::npc::{lines, NPC_ID_BASE};
use game::protocol::{self as proto, Appearance, Packet, Profile};
use game::recruitment::{default_recruitment_path, Recruitment};
use game::security;
use game::server::{Config, Server};
use game::sim::{self, Body, Pos, IN_RIGHT};

/// The inventory comes with the change and again every 2 s; a wait for it
/// right after another one may have missed the first (slow CI machines).
const INVENTORY_WAIT: Duration = Duration::from_secs(3);

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
// Where to stand in the kitchenette (floor 1, its lower left corner) and at
// the shop's till (floor 0).
const CUPBOARD: Tile = Tile { x: 21, y: 15 };
const COFFEE: Tile = Tile { x: 20, y: 12 };
const SINK: Tile = Tile { x: 20, y: 15 };
const DISHWASHER: Tile = Tile { x: 20, y: 14 };
const FRIDGE: Tile = Tile { x: 20, y: 13 };
const TILL: Tile = Tile { x: 20, y: 53 };

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

thread_local! {
    /// Fixed weather for servers started on this test thread (None = changing).
    static WEATHER: std::cell::Cell<Option<u8>> = const { std::cell::Cell::new(Some(game::weather::kind::CLOUDY)) };
    /// When the cleaner starts her round on servers started on this thread.
    static CLEANING_AT: std::cell::Cell<u32> = const { std::cell::Cell::new(game::cleaning::ROUND_AT) };
}

fn start_server_cfg(start_access: u8, skip_recruitment: bool, start_employed: bool) -> (SocketAddr, u32) {
    start_server_at(start_access, skip_recruitment, start_employed, 8 * 60, 1)
}

/// ... with the game clock starting at `start_minute`, `time_scale` faster.
fn start_server_at(
    start_access: u8,
    skip_recruitment: bool,
    start_employed: bool,
    start_minute: u32,
    time_scale: u32,
) -> (SocketAddr, u32) {
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
        needs_speed: 1,
        start_minute,
        time_scale,
        weather: WEATHER.with(|w| w.get()),
        treats_now: true,
        stale_fruit_percent: game::treats::STALE_FRUIT_PERCENT,
        cleaning_at: CLEANING_AT.with(|c| c.get()),
        cleaning_spread: 0,
        start_cigarettes: false,
        save_path: None,
        allow_guests: true,
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
    /// A logged-in client: packets sealed with the session key.
    crypto: Option<std::cell::RefCell<game::crypto::Session>>,
}

impl Client {
    fn socket_for(server: SocketAddr) -> UdpSocket {
        let sock = UdpSocket::bind(if server.is_ipv6() { "[::1]:0" } else { "127.0.0.1:0" }).unwrap();
        sock.connect(server).unwrap();
        sock.set_read_timeout(Some(Duration::from_millis(20))).unwrap();
        sock
    }

    fn connect(server: SocketAddr, nick: &str) -> (Client, u32) {
        Client::connect_with(server, nick, "")
    }

    /// With a login ticket ("" = a guest).
    fn connect_with(server: SocketAddr, nick: &str, ticket: &str) -> (Client, u32) {
        let sock = Client::socket_for(server);
        sock.send(&Packet::Connect { nonce: 42, nick: nick.into(), profile: test_profile(), ticket: ticket.into() }.encode()).unwrap();
        let mut c = Client { sock, id: 0, token: 0, seq: 0, crypto: None };
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
        assert!(n <= proto::MAX_DATAGRAM);
        if let Some(c) = &self.crypto {
            if buf[3] == game::crypto::SEALED {
                let mut c = c.borrow_mut();
                let (counter, inner) = c.keys.open(game::crypto::Dir::ToClient, 8, &buf[..n]).expect("sealed by the server");
                assert!(c.window.accept(counter), "no replays");
                assert!(inner.len() <= proto::MAX_PACKET);
                return Some(Packet::decode(&inner).expect("server sent a valid packet"));
            }
        }
        assert!(n <= proto::MAX_PACKET);
        Some(Packet::decode(&buf[..n]).expect("server sent a valid packet"))
    }

    /// Send a packet (sealed for a logged-in client).
    fn send(&self, p: &Packet) {
        let mut bytes = p.encode();
        if let Some(c) = &self.crypto {
            let mut c = c.borrow_mut();
            c.send_counter += 1;
            bytes = c.keys.seal(game::crypto::Dir::ToServer, &game::crypto::session_prefix(self.token), c.send_counter, &bytes);
        }
        self.sock.send(&bytes).unwrap();
    }

    /// A logged-in client: a sealed Connect with the ticket and the key
    /// from the login API.
    fn connect_sealed(server: SocketAddr, ticket: &str, key: &str) -> Client {
        Client::connect_sealed_as(server, ticket, key, test_profile()).expect("Welcome")
    }

    /// A sealed Connect with this character; Err(reject reason).
    fn connect_sealed_as(server: SocketAddr, ticket: &str, key: &str, profile: Profile) -> Result<Client, u8> {
        use game::crypto::{connect_prefix, from_hex, Dir, Keys, Session};
        let sock = Client::socket_for(server);
        let keys = Keys::derive(&from_hex::<32>(key).expect("key"));
        let raw = from_hex::<32>(ticket).expect("ticket");
        let connect = Packet::Connect { nonce: 42, nick: String::new(), profile, ticket: ticket.into() }.encode();
        // Like the real client: the counter only goes up (a repeated one is a replay).
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        sock.send(&keys.seal(Dir::ToServer, &connect_prefix(&raw), n, &connect)).unwrap();
        let mut c = Client {
            sock,
            id: 0,
            token: 0,
            seq: 0,
            crypto: Some(std::cell::RefCell::new(Session { keys, send_counter: n, window: Default::default() })),
        };
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            match c.recv() {
                Some(Packet::Welcome { player_id, token, .. }) => {
                    c.id = player_id;
                    c.token = token;
                    return Ok(c);
                }
                Some(Packet::Reject { reason }) => return Err(reason),
                _ => {}
            }
        }
        panic!("no answer to the sealed Connect");
    }

    fn send_inputs(&mut self, bits: u8, n: usize) {
        self.seq += n as u32;
        let p = Packet::Input { token: self.token, ack_tick: 0, last_seq: self.seq, inputs: vec![bits; n] };
        self.send(&p);
    }

    /// Latest snapshot (tick, room, pos, visible ids) seen within `wait`.
    #[allow(clippy::type_complexity)]
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
            self.send(&p);
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
        self.send(&p);
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
        self.send(&Packet::Ping { token: self.token, client_time: 1 });
    }
}

#[test]
fn handshake_interest_and_timeout() {
    let (addr, crc) = start_server();
    let b0 = building();
    let outside = b0.floor(0).unwrap().room_by_name("Na zewnątrz").unwrap().id;
    let lobby = b0.floor(0).unwrap().room_by_name("Wiatrołap").unwrap().id;

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
    b.walk_to(&b0, b_body, (0, Tile { x: 31, y: 55 }), &[&a]);
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
    sock.send_to(&Packet::Connect { nonce: 1, nick: "   ".into(), profile: test_profile(), ticket: String::new() }.encode(), addr).unwrap();
    let mut buf = [0u8; 2048];
    let n = sock.recv(&mut buf).unwrap();
    assert_eq!(Packet::decode(&buf[..n]).unwrap(), Packet::Reject { reason: proto::reject::BAD_NICK });

    let mut bad = Packet::Connect { nonce: 1, nick: "x".into(), profile: test_profile(), ticket: String::new() }.encode();
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
    assert_eq!(Packet::decode(&buf[..n]).unwrap(), Packet::Disconnect { token: 12345, reason: proto::disconnect::SESSION_UNKNOWN });
}

#[test]
fn other_floors_are_invisible_and_state_matches_prediction() {
    let (addr, _) = start_server_with(access::CARD);
    let b0 = building();
    let (a, _) = Client::connect(addr, "downstairs");
    let (mut b, _) = Client::connect(addr, "upstairs");
    let spawn = b0.spawns()[1];
    let start = Body { access: access::CARD, ..Body::at(spawn.0, Pos::tile_center(spawn.1.x, spawn.1.y)) };
    // Up the stairs to the corridor on floor 1: A stays outside.
    let predicted = b.walk_to(&b0, start, (1, Tile { x: 32, y: 20 }), &[&a]);
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
    let server = Body { floor, pos: Pos { x, y }, prev_input: prev, lock, access: access::CARD, slow: false, drunk: 0 };
    assert_eq!(server, predicted, "server state == client prediction, bit for bit");
    assert_eq!(b0.floor(1).unwrap().room_name(room), "Korytarz");

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

    // Without a pass the stairs (and the lifts) are closed.
    assert!(Walker::to(&b0, &start, (0, Tile { x: 24, y: 42 })).is_none(), "no path without a pass");

    // Walk to the porter's desk and press E across it.
    let body = g.walk_to(&b0, start, (0, Tile { x: 34, y: 49 }), &[]);
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
            Some(Packet::PlayerInfo { players }) => porter_named |= players.iter().any(|p| p.nick == "Pani Wiesia"),
            _ => {}
        }
    }
    assert!(welcomed, "porter greets");
    assert!(got_pass, "guest pass granted");
    assert!(porter_named, "porter comes out into the lobby and is visible by name");

    // Follow him up to the reception; he announces the arrival there.
    let body = Body { access: access::GUEST, ..body };
    let end = g.walk_to(&b0, body, (1, Tile { x: 36, y: 36 }), &[]);
    assert_eq!(end.floor, 1);
    assert!(g.wait_for_line(lines::ARRIVED, Duration::from_secs(8)).is_some(), "porter reached the reception");

    // Reception takes us to HR.
    let body = g.press_e(&b0, end);
    assert!(g.wait_for_line(lines::RECEPTION_WELCOME, Duration::from_secs(1)).is_some(), "reception greets");
    let at_hr = g.walk_to(&b0, body, (1, Tile { x: 47, y: 14 }), &[]);
    assert!(g.wait_for_line(lines::RECEPTION_ARRIVED, Duration::from_secs(6)).is_some(), "receptionist reached HR");

    // HR: the contract (signed), the card replaces the guest pass.
    g.press_e(&b0, at_hr);
    answer_dialog(&g, game::pay::CONTRACT_ID, 0);
    let access = g.wait_for_line(&game::pay::lines::signed(None, true), Duration::from_secs(1));
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
    assert!(offers.len() >= 5, "several companies on the portal");
    let open: Vec<u8> = offers.values().filter(|o| o.company == "Startup Sim sp. z o.o.").map(|o| o.vacancies).collect();
    assert_eq!(open, [1, 0, 1, 0], "a young startup: only two openings at the start (programmer, sales)");

    // Another company answers with a (funny) rejection; a silent one never does.
    let apply = |offer: u8| {
        c.send(&Packet::Apply { token: c.token, offer, motivation: "Bo lubię kawę.".into(), salary: 8000, form: 1, student: false });
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
        let Packet::Mail { action, arg, .. } =
            recv_until(&|p| matches!(p, Packet::Mail { action, .. } if *action == act::JOIN_INTERVIEW), false)
        else {
            unreachable!()
        };
        c.send(&Packet::PortalAction { token: c.token, action, arg });
        loop {
            match recv_until(&|p| matches!(p, Packet::Question { .. } | Packet::RecruitResult { .. }), false) {
                Packet::Question { attempt, index, text, options, .. } => {
                    let q = bank.set("programming").unwrap().questions.iter().find(|q| q.text == text).expect("known question");
                    let right = options.iter().position(|o| *o == q.options[0]).unwrap() as u8;
                    let choice = if want_correct { right } else { (right + 1) % options.len() as u8 };
                    c.send(&Packet::Answer { token: c.token, attempt, index, choice });
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
    c.send(&Packet::PortalAction { token: c.token, action, arg: 0 });
    let b = building();
    let Packet::Snapshot { floor, room, self_access, .. } = recv_until(&|p| matches!(p, Packet::Snapshot { .. }), true) else {
        unreachable!()
    };
    assert_eq!((floor, b.floor(0).unwrap().room_name(room), self_access), (0, "Na zewnątrz", 0));
}

#[test]
fn a_mug_left_in_the_chill_room_is_collected_by_the_cleaner() {
    use game::cleaning::lines as cl;
    use game::coffee::lines as coffee_lines;
    use game::inventory::kind as item_kind;
    use proto::item_action as act;
    // 10:00, the cleaner comes at 10:05 (25 s).
    CLEANING_AT.with(|c| c.set(10 * 60 + 5));
    let (addr, _) = start_server_at(access::CARD, true, true, 10 * 60, 1);
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola");
    let ws = game::computer::find_workstations(&b);
    let w = ws.iter().find(|w| w.department == 1).unwrap();
    let body = Body { access: access::CARD, ..Body::at(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1)) };
    let action = |c: &Client, a: u8| c.send(&Packet::ItemAction { token: c.token, action: a, slot: 0 });
    let hands = |c: &Client, k: u8| {
        wait_for(c, &[], Duration::from_millis(2500), |p| match p {
            Packet::Inventory { slots } if slots[0].kind == k => Some(()),
            _ => None,
        })
        .is_some()
    };
    // A mug from the cupboard, a coffee, drunk: a dirty mug in hands (the
    // laptop stays at the desk).
    action(&ola, act::DROP);
    std::thread::sleep(Duration::from_millis(100));
    let at = ola.walk_to(&b, body, (1, CUPBOARD), &[]);
    let at = ola.press_e(&b, at);
    cupboard_take(&ola, 0);
    assert!(hands(&ola, item_kind::CUP), "a clean mug from the cupboard");
    let at = ola.walk_to(&b, at, (1, COFFEE), &[]);
    let at = ola.press_e(&b, at);
    assert!(ola.wait_for_line(coffee_lines::READY, Duration::from_secs(5)).is_some());
    action(&ola, act::USE);
    assert!(hands(&ola, item_kind::EMPTY_CUP), "dirty mug after the coffee");
    // Washed up at the kitchen sink, another coffee, the mug left on the floor.
    let at = ola.walk_to(&b, at, (1, SINK), &[]);
    let at = ola.press_e(&b, at);
    assert!(ola.wait_for_line(game::kitchen::lines::WASHED, Duration::from_millis(800)).is_some());
    assert!(hands(&ola, item_kind::CUP));
    let at = ola.walk_to(&b, at, (1, COFFEE), &[]);
    let at = ola.press_e(&b, at);
    assert!(ola.wait_for_line(coffee_lines::READY, Duration::from_secs(5)).is_some());
    action(&ola, act::USE);
    assert!(hands(&ola, item_kind::EMPTY_CUP));
    action(&ola, act::DROP);
    assert!(hands(&ola, item_kind::NONE));
    // At 10:04 the cleaner comes up from the service room and takes it.
    assert!(ola.wait_for_line(&cl::few(1), Duration::from_secs(40)).is_some(), "the cleaner collected the mug");
    let _ = at;
}

#[test]
fn smoking_inside_sets_off_the_fire_alarm_and_the_smoker_pays() {
    use game::fire::lines as fl;
    use game::inventory::kind as item_kind;
    use proto::item_action as act;
    let (addr, _) = start_server_at(access::CARD, true, true, 10 * 60, 1); // hired: 200 zł
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola");
    let wait = Duration::from_millis(1500);
    let ws = game::computer::find_workstations(&b);
    let w = ws.iter().find(|w| w.department == 1).unwrap();
    let body = Body { access: access::CARD, ..Body::at(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1)) };
    let action = |c: &Client, a: u8, slot: u8| c.send(&Packet::ItemAction { token: c.token, action: a, slot });
    let said = |line: String| move |p: &Packet| matches!(p, Packet::Say { text, .. } if *text == line).then_some(());
    let alarm = |c: &Client, on: u8, wait: Duration| {
        wait_for(c, &[], wait, |p| matches!(p, Packet::Clock { alarm, .. } if *alarm == on).then_some(())).is_some()
    };
    action(&ola, act::DROP, 0); // the laptop stays at the desk
    std::thread::sleep(Duration::from_millis(100));
    // A pack of cigarettes from the shop, paid for.
    let body = ola.walk_to(&b, body, (0, Tile { x: 20, y: 47 }), &[]);
    ola.send(&Packet::ShopTake { token: ola.token, shelf: 5, kind: item_kind::CIGARETTES });
    std::thread::sleep(Duration::from_millis(150));
    let body = ola.walk_to(&b, body, (0, TILL), &[]);
    while ola.recv().is_some() {}
    let body = ola.press_e(&b, body);
    assert!(wait_for(&ola, &[], wait, said("Razem 18,00 zł. Dziękuję! Zostało Ci 182,00 zł.".into())).is_some());
    // Into the hall by the lifts (it has a smoke detector) and light up.
    let _body = ola.walk_to(&b, body, (0, Tile { x: 31, y: 44 }), &[]);
    let slot = wait_for(&ola, &[], Duration::from_millis(2500), |p| match p {
        Packet::Inventory { slots } => slots[1..].iter().position(|s| s.kind == item_kind::CIGARETTES),
        _ => None,
    })
    .expect("cigarettes in a pocket");
    action(&ola, act::TAKE_OUT, slot as u8);
    std::thread::sleep(Duration::from_millis(150));
    action(&ola, act::USE, 0);
    assert!(wait_for(&ola, &[], wait, said(fl::LIT_INSIDE.into())).is_some());
    let smoky = wait_for(&ola, &[], Duration::from_millis(4000), |p| match p {
        Packet::Smoke { floor: 0, rooms } if !rooms.is_empty() => Some(()),
        _ => None,
    });
    assert!(smoky.is_some(), "smoke in the hall");
    // Some seconds later the detector goes off; the firefighter comes, checks and
    // fines Ola; back at the engine the alarm is over.
    assert!(alarm(&ola, 1, Duration::from_secs(25)), "fire alarm");
    assert!(wait_for(&ola, &[], Duration::from_secs(30), said(fl::fined(182_00))).is_some(), "fined");
    assert!(alarm(&ola, 0, Duration::from_secs(30)), "alarm over");
}

#[test]
fn the_light_switch_turns_the_room_lamp_on_and_off_for_everybody() {
    use game::lights::lines as ll;
    let (addr, _) = start_server_cfg(access::CARD, true, true);
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola");
    let ws = game::computer::find_workstations(&b);
    let w = ws.iter().find(|w| w.department == 1).unwrap();
    let body = Body { access: access::CARD, ..Body::at(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1)) };
    let it = b.floor(1).unwrap().room_by_name("Produkt / IT").unwrap();
    let [sx, sy] = it.switch.unwrap();
    let lamps = |c: &Client| {
        wait_for(c, &[], Duration::from_millis(2500), |p| match p {
            Packet::Lights { floor: 1, rooms } => Some(rooms.clone()),
            _ => None,
        })
    };
    assert_eq!(lamps(&ola), Some(vec![]), "lamps start the day off");
    let at = ola.walk_to(&b, body, (1, Tile { x: sx, y: sy }), &[]);
    while ola.recv().is_some() {}
    let at = ola.press_e(&b, at);
    assert!(ola.wait_for_line(ll::ON, Duration::from_millis(800)).is_some());
    assert_eq!(lamps(&ola), Some(vec![it.id]));
    ola.press_e(&b, at);
    assert!(ola.wait_for_line(ll::OFF, Duration::from_millis(800)).is_some());
    let off = wait_for(&ola, &[], Duration::from_millis(2500), |p| match p {
        Packet::Lights { floor: 1, rooms } if rooms.is_empty() => Some(()),
        _ => None,
    });
    assert!(off.is_some());
}

#[test]
fn kitchenette_mugs_dishwasher_and_fridge() {
    use game::coffee::lines as coffee_lines;
    use game::inventory::kind as item_kind;
    use game::kitchen::{action as fa, lines as kl};
    use proto::item_action as act;
    let (addr, _) = start_server_cfg(access::CARD, true, true);
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola");
    let ws = game::computer::find_workstations(&b);
    let w = ws.iter().find(|w| w.department == 1).unwrap();
    let body = Body { access: access::CARD, ..Body::at(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1)) };
    let action = |c: &Client, a: u8| c.send(&Packet::ItemAction { token: c.token, action: a, slot: 0 });
    let hands = |c: &Client, k: u8| {
        wait_for(c, &[], Duration::from_millis(2500), |p| match p {
            Packet::Inventory { slots } if slots[0].kind == k => Some(()),
            _ => None,
        })
        .is_some()
    };
    let fridge = |c: &Client, a: u8| c.send(&Packet::FridgeAction { token: c.token, action: a, arg: 0 });
    action(&ola, act::DROP); // the laptop stays at the desk
    std::thread::sleep(Duration::from_millis(100));
    // No mug, no coffee.
    let at = ola.walk_to(&b, body, (1, COFFEE), &[]);
    let at = ola.press_e(&b, at);
    assert!(ola.wait_for_line(kl::NEED_MUG, Duration::from_millis(800)).is_some());
    // Mug -> coffee -> dirty mug -> dishwasher, switched on.
    let at = ola.walk_to(&b, at, (1, CUPBOARD), &[]);
    let at = ola.press_e(&b, at);
    cupboard_take(&ola, 0);
    assert!(hands(&ola, item_kind::CUP));
    let at = ola.walk_to(&b, at, (1, COFFEE), &[]);
    let at = ola.press_e(&b, at);
    assert!(ola.wait_for_line(coffee_lines::READY, Duration::from_secs(5)).is_some());
    action(&ola, act::USE);
    assert!(hands(&ola, item_kind::EMPTY_CUP));
    let at = ola.press_e(&b, at);
    assert!(ola.wait_for_line(kl::DIRTY_MUG, Duration::from_millis(800)).is_some(), "no coffee into a dirty mug");
    let at = ola.walk_to(&b, at, (1, DISHWASHER), &[]);
    let at = ola.press_e(&b, at);
    assert!(ola.wait_for_line(&kl::loaded(1), Duration::from_millis(800)).is_some());
    let at = ola.press_e(&b, at);
    assert!(ola.wait_for_line(kl::DW_STARTED, Duration::from_millis(800)).is_some());
    // Another coffee, with milk from the fridge; a free water.
    let at = ola.walk_to(&b, at, (1, CUPBOARD), &[]);
    let at = ola.press_e(&b, at);
    cupboard_take(&ola, 0);
    assert!(hands(&ola, item_kind::CUP));
    let at = ola.walk_to(&b, at, (1, COFFEE), &[]);
    let at = ola.press_e(&b, at);
    assert!(ola.wait_for_line(coffee_lines::READY, Duration::from_secs(5)).is_some());
    let at = ola.walk_to(&b, at, (1, FRIDGE), &[]);
    while ola.recv().is_some() {}
    ola.press_e(&b, at);
    let opened = wait_for(&ola, &[], Duration::from_millis(800), |p| match p {
        Packet::Fridge { milk, water, juice, .. } => Some((*milk, *water, *juice)),
        _ => None,
    });
    assert_eq!(opened, Some((5, 4, 2)));
    fridge(&ola, fa::MILK);
    assert!(hands(&ola, item_kind::LATTE), "coffee with milk");
    fridge(&ola, fa::TAKE_WATER);
    let after = wait_for(&ola, &[], Duration::from_millis(800), |p| match p {
        Packet::Fridge { milk: 4, water: 3, .. } => Some(()),
        _ => None,
    });
    assert!(after.is_some());
}

#[test]
fn going_home_early_from_the_tram_stop_pays_and_speeds_the_day_up() {
    use game::commute::lines as cl;
    use proto::place;
    let (addr, _) = start_server_at(access::CARD, true, true, 10 * 60, 1); // hired, comes by tram
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola");
    let ws = game::computer::find_workstations(&b);
    let w = ws.iter().find(|w| w.department == 1).unwrap();
    let body = Body { access: access::CARD, ..Body::at(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1)) };
    // E anywhere else: nothing about going home.
    let at = ola.walk_to(&b, body, (0, Tile { x: 36, y: 69 }), &[]);
    while ola.recv().is_some() {}
    let at = ola.press_e(&b, at);
    assert!(ola.wait_for_line(cl::GO_HOME_ASK, Duration::from_millis(800)).is_some(), "asked first");
    ola.press_e(&b, at);
    let home = clock_until(&ola, Duration::from_millis(1500), |_, _, pl, _, _| pl == place::HOME);
    let (_, m0, _, _, pay) = home.expect("at home");
    assert!(pay > 0, "paid for the time worked");
    // Nobody left at work: the day flies by (night speed).
    let _ = wait_for(&ola, &[], Duration::from_millis(2500), |_| None::<()>); // keeps pinging
    let later = clock_until(&ola, Duration::from_millis(1500), |_, _, _, _, _| true).expect("clock");
    assert!(later.1 >= m0 + 15, "fast forward: {} -> {}", m0, later.1);
    // "Skip the waiting": straight to the next morning's commute.
    ola.send(&Packet::SkipWait { token: ola.token });
    let skipping = wait_for(&ola, &[], Duration::from_millis(1000), |p| matches!(p, Packet::Clock { skip: 2, .. }).then_some(()));
    assert!(skipping.is_some(), "time flies");
    let morning = clock_until(&ola, Duration::from_millis(9000), |day, _, pl, _, _| pl == place::COMMUTING && day >= 2);
    assert!(morning.is_some(), "the next morning in a few seconds");
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
    // Both take a mug from the cupboard and go to the machine.
    let at_a = a.walk_to(&b, spawn(0), (1, CUPBOARD), &[&c]);
    let at_a = a.press_e(&b, at_a);
    cupboard_take(&a, 0);
    assert!(a.wait_for_line(&game::kitchen::lines::took_mug(7), Duration::from_millis(800)).is_some());
    let at_a = a.walk_to(&b, at_a, (1, COFFEE), &[&c]);
    let at_c = c.walk_to(&b, spawn(1), (1, CUPBOARD), &[&a]);
    let at_c = c.press_e(&b, at_c);
    cupboard_take(&c, 0);
    assert!(c.wait_for_line(&game::kitchen::lines::took_mug(6), Duration::from_millis(800)).is_some());
    let at_c = c.walk_to(&b, at_c, (1, Tile { x: 20, y: 11 }), &[&a]);

    // A presses E: brewing starts; A's own bubble says so.
    a.press_e(&b, at_a);
    let seen = a.wait_for_line(coffee_lines::BREWING, Duration::from_millis(500));
    assert!(seen.is_some(), "A starts brewing");
    // C, standing next to it, hears the machine.
    let heard = wait_for(&c, &[&a], Duration::from_millis(500), |p| {
        matches!(p, Packet::Sound { sounds } if sounds.iter().any(|s| s.0 == proto::sound::COFFEE)).then_some(())
    });
    assert!(heard.is_some(), "C hears the coffee machine");
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
                others_see |= entities.iter().any(|e| e.id == a.id && e.held == item_kind::COFFEE);
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
    sock.send_to(&Packet::Connect { nonce: 1, nick: "Zly".into(), profile: bad, ticket: String::new() }.encode(), addr).unwrap();
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
        who.send(&Packet::ItemAction { token: who.token, action, slot });
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
/// A server dialog `id` opened (the cupboard, the contract): answer `choice`.
fn answer_dialog(c: &Client, id: u8, choice: u8) {
    let open = wait_for(c, &[], Duration::from_secs(2), |p| matches!(p, Packet::Dialog { id: d, .. } if *d == id).then_some(()));
    assert!(open.is_some(), "dialog {id} opens");
    c.send(&Packet::DialogAnswer { token: c.token, id, choice });
}

/// E at the cupboard opened its window (mugs, knives): take option `choice`
/// (0 = a mug).
fn cupboard_take(c: &Client, choice: u8) {
    answer_dialog(c, game::mischief::CUPBOARD_ID, choice);
}

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
        who.send(&p);
    };
    let said = |line: &'static str| move |p: &Packet| matches!(p, Packet::Say { text, .. } if text == line).then_some(());
    let screen = |p: &Packet| match p {
        Packet::Computer { owner, locked, convs, .. } => Some((*owner, *locked, convs.clone())),
        _ => None,
    };
    let status = |p: &Packet| match p {
        Packet::Snapshot { self_activity, .. } => Some(*self_activity),
        _ => None,
    };
    let at_computer = |c: &Client, keep: &[&Client], want: bool| {
        wait_for(c, keep, Duration::from_millis(800), |p| status(p).filter(|s| (*s == proto::activity::COMPUTER) == want)).is_some()
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
    assert_eq!(titles, ["#ogólny", "#produkt-it", "Ewa", "Kuba"]);
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
        let it: Vec<_> = ws.iter().filter(|w| w.department == 1).collect();
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
    assert!(convs.iter().any(|c| c.title == "#biznes") && !convs.iter().any(|c| c.title == "#produkt-it"));
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
    ewa.send(&Packet::ItemAction { token: ewa.token, action: proto::item_action::DROP, slot: 0 });
    std::thread::sleep(Duration::from_millis(100));
    action(&ewa, ca::TAKE, 0, 0, "");
    let hands = wait_for(&ewa, &[&ola, &kuba], INVENTORY_WAIT, |p| match p {
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

#[test]
fn needs_fruit_sofa_and_the_wrong_bathroom() {
    use game::inventory::kind as item_kind;
    use game::needs::lines as nl;
    use proto::item_action as act;
    let (addr, _) = start_server_cfg(0, true, true);
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola"); // female, IT, at the first IT desk
    let wait = Duration::from_millis(800);
    let said = |line: &'static str| move |p: &Packet| matches!(p, Packet::Say { text, .. } if text.starts_with(line)).then_some(());
    let hunger = |c: &Client| wait_for(c, &[], wait, |p| if let Packet::Stats { hunger, .. } = p { Some(*hunger) } else { None });
    let activity = |c: &Client| {
        let mut last = None;
        let deadline = Instant::now() + Duration::from_millis(300);
        while Instant::now() < deadline {
            c.ping();
            while let Some(p) = c.recv() {
                if let Packet::Snapshot { self_activity, .. } = p {
                    last = Some(self_activity);
                }
            }
        }
        last
    };
    let ws = game::computer::find_workstations(&b);
    let w = ws.iter().find(|w| w.department == 1).unwrap();
    let mut body = Body { access: access::CARD, ..Body::at(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1)) };

    // Laptop down first (hands free), then fruit from the bowl in the chill room.
    body = ola.press_e(&b, body);
    assert!(wait_for(&ola, &[], wait, said(game::computer::lines::PLACED)).is_some());
    body = ola.walk_to(&b, body, (1, Tile { x: 41, y: 8 }), &[]);
    body = ola.press_e(&b, body);
    assert!(wait_for(&ola, &[], wait, said(nl::FRUIT)).is_some());
    let slot = wait_for(&ola, &[], INVENTORY_WAIT, |p| match p {
        Packet::Inventory { slots } => slots[1..].iter().position(|s| s.kind == item_kind::FRUIT),
        _ => None,
    })
    .expect("fruit in a pocket");
    let before = hunger(&ola).unwrap();
    ola.send(&Packet::ItemAction { token: ola.token, action: act::TAKE_OUT, slot: slot as u8 });
    std::thread::sleep(Duration::from_millis(120));
    ola.send(&Packet::ItemAction { token: ola.token, action: act::USE, slot: 0 });
    // (Sometimes the fruit is stale: a different line, same meal.)
    let ate = wait_for(&ola, &[], wait, |p| match p {
        Packet::Say { text, .. } if text.starts_with("Mniam") || text == game::treats::lines::STALE_EATEN => Some(()),
        _ => None,
    });
    assert!(ate.is_some());
    let after = wait_for(&ola, &[], Duration::from_millis(1500), |p| match p {
        Packet::Stats { hunger, .. } if *hunger + 15 <= before => Some(*hunger),
        _ => None,
    });
    assert!(after.is_some(), "fruit lowers hunger (was {before})");

    // Sofa: sitting shows as an activity; stepping away ends it.
    body = ola.walk_to(&b, body, (1, Tile { x: 32, y: 10 }), &[]);
    body = ola.press_e(&b, body);
    assert!(wait_for(&ola, &[], wait, said(nl::SOFA)).is_some());
    assert_eq!(activity(&ola), Some(proto::activity::SOFA));
    ola.send_inputs(IN_RIGHT, 3);
    body = sim::step(&b, sim::step(&b, sim::step(&b, body, IN_RIGHT), IN_RIGHT), IN_RIGHT);
    assert_eq!(activity(&ola), Some(proto::activity::NONE), "got up");

    // Ola is female: the men's room works, but it's embarrassing.
    body = ola.walk_to(&b, body, (1, Tile { x: 37, y: 22 }), &[]); // the men's WC
    ola.press_e(&b, body);
    assert!(wait_for(&ola, &[], wait, said(nl::WRONG_BATHROOM)).is_some());
    assert_eq!(activity(&ola), Some(proto::activity::TOILET));
}

#[test]
fn toilet_stall_hides_who_is_inside_and_locks() {
    use game::stalls::lines as sl;
    let (addr, _) = start_server_cfg(0, true, true);
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola"); // IT
    let (mut kuba, _) = Client::connect(addr, "Kuba"); // Biznes
    let wait = Duration::from_millis(800);
    let seat = |dept: u8| {
        let ws = game::computer::find_workstations(&b);
        let w = ws.iter().find(|w| w.department == dept).unwrap();
        Body { access: access::CARD, ..Body::at(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1)) }
    };
    let said = |line: &'static str| move |p: &Packet| matches!(p, Packet::Say { text, .. } if text == line).then_some(());
    let door_action = |c: &Client| c.send(&Packet::DoorAction { token: c.token });
    let locked_doors = |c: &Client, keep: &Client| {
        wait_for(c, &[keep], wait, |p| if let Packet::Doors { tiles, .. } = p { Some(tiles.clone()) } else { None })
    };

    // Ola goes into women's stall 1 (without locking), Kuba waits by the sinks.
    let o = ola.walk_to(&b, seat(1), (1, Tile { x: 4, y: 45 }), &[&kuba]);
    let k = kuba.walk_to(&b, seat(2), (1, Tile { x: 7, y: 45 }), &[&ola]);
    assert!(!visible_ids(&kuba, &[&ola], wait).contains(&ola.id), "nobody sees who is in the stall");
    assert!(visible_ids(&ola, &[&kuba], wait).contains(&kuba.id), "from the stall you see the bathroom");

    // Kuba opens the unlocked door (steps into the doorway): now he sees her.
    let k_door = kuba.walk_to(&b, k, (1, Tile { x: 5, y: 45 }), &[&ola]);
    assert!(visible_ids(&kuba, &[&ola], wait).contains(&ola.id), "an open door shows who is inside");
    // Too late to lock with Kuba in the doorway.
    // (Drain first: during Kuba's walk Ola's socket buffer filled up with
    // snapshots, and a full buffer drops new datagrams.)
    while ola.recv().is_some() {}
    door_action(&ola);
    assert!(wait_for(&ola, &[&kuba], wait, said(sl::IN_DOORWAY)).is_some());

    // Kuba steps back, Ola locks: the door is solid, Kuba can't get in or see her.
    kuba.walk_to(&b, k_door, (1, Tile { x: 7, y: 45 }), &[&ola]);
    while ola.recv().is_some() {}
    door_action(&ola);
    assert!(wait_for(&ola, &[&kuba], wait, said(sl::LOCKED)).is_some());
    assert!(locked_doors(&kuba, &ola).is_some_and(|t| t.contains(&(5, 45))));
    kuba.send_inputs(sim::IN_LEFT, 6);
    std::thread::sleep(Duration::from_millis(60));
    kuba.send_inputs(sim::IN_LEFT, 6);
    let (_, room, (x, _), ids, _) = kuba.latest_snapshot(Duration::from_millis(300)).unwrap();
    assert!(x >= 6 * sim::TILE_UNITS, "stopped at the locked door (x = {x})");
    assert!(!ids.contains(&ola.id));
    assert_eq!(b.floor(1).unwrap().room_name(room), "Łazienka damska (zachód)");
    let _ = o;

    // Ola leaves the game while locked in: the stall opens by itself.
    ola.send(&Packet::Disconnect { token: ola.token, reason: proto::disconnect::CLIENT_QUIT });
    let opened = wait_for(&kuba, &[], wait, |p| match p {
        Packet::Doors { tiles, .. } if !tiles.contains(&(5, 45)) => Some(()),
        _ => None,
    });
    assert!(opened.is_some(), "unlocked when the person inside left");
}

#[test]
fn elevator_is_called_waited_for_and_ridden() {
    use game::elevator::lines as el;
    let (addr, _) = start_server_cfg(0, true, true);
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola"); // IT, floor 1
    let wait = Duration::from_millis(800);
    let said = |pred: fn(&str) -> bool| move |p: &Packet| matches!(p, Packet::Say { text, .. } if pred(text)).then_some(());
    let ws = game::computer::find_workstations(&b);
    let w = ws.iter().find(|w| w.department == 1).unwrap();
    let start = Body { access: access::CARD, ..Body::at(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1)) };

    // In front of the elevator on floor 1: the car is downstairs, doors shut.
    let body = ola.walk_to(&b, start, (1, Tile { x: 37, y: 44 }), &[]);
    // (Lift A: the left one, doors at x 36..38.)
    let doors = |c: &Client| {
        wait_for(c, &[], wait, |p| {
            if let Packet::Doors { tiles, lifts, .. } = p {
                Some((tiles.clone(), lifts[0].floor, lifts[0].target))
            } else {
                None
            }
        })
    };
    let (tiles, lift, _) = doors(&ola).unwrap();
    assert!(tiles.contains(&(37, 43)) && lift == 0);
    let body = ola.press_e(&b, body);
    assert!(wait_for(&ola, &[], wait, said(|t| t == el::CALLED)).is_some());
    // Walking into the closed doors doesn't work.
    while ola.recv().is_some() {}
    ola.send_inputs(sim::IN_UP, 6);
    let (_, _, (_, y), _, _) = ola.latest_snapshot(Duration::from_millis(200)).unwrap();
    assert!(y >= 44 * sim::TILE_UNITS, "doors closed while the car is away");
    // ~3 s later it arrives and opens.
    let opened = wait_for(&ola, &[], Duration::from_millis(4000), |p| match p {
        Packet::Doors { tiles, lifts, .. } if lifts[0].floor == 1 && !tiles.contains(&(37, 43)) => Some(()),
        _ => None,
    });
    assert!(opened.is_some(), "the car came up and opened");
    // Step in, choose the floor (the other one: ground floor), ride.
    let body = Body { pos: Pos { x: body.pos.x, y: 44 * sim::TILE_UNITS + sim::HALF_H }, ..body };
    let body = ola.walk_to(&b, body, (1, Tile { x: 37, y: 42 }), &[]);
    ola.press_e(&b, body);
    assert!(wait_for(&ola, &[], wait, said(|t| t.starts_with("Jedziemy na: Parter"))).is_some());
    let arrived = wait_for(&ola, &[], Duration::from_millis(5000), |p| match p {
        Packet::Snapshot { floor: 0, self_x, self_y, .. } => Some((*self_x, *self_y)),
        _ => None,
    });
    let (x, y) = arrived.expect("arrived at the ground floor");
    assert_eq!(Pos { x, y }.tile(), (37, 42), "same spot in the cabin, other floor");
}

#[test]
fn the_two_lifts_run_on_their_own() {
    use game::elevator::lines as el;
    let (addr, _) = start_server_cfg(0, true, true);
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola"); // IT, floor 1
    let wait = Duration::from_millis(800);
    let ws = game::computer::find_workstations(&b);
    let w = ws.iter().find(|w| w.department == 1).unwrap();
    let start = Body { access: access::CARD, ..Body::at(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1)) };
    // In front of the right-hand lift (B, doors at x 40..42): call it.
    let body = ola.walk_to(&b, start, (1, Tile { x: 41, y: 44 }), &[]);
    while ola.recv().is_some() {}
    ola.press_e(&b, body);
    assert!(wait_for(&ola, &[], wait, |p| matches!(p, Packet::Say { text, .. } if text == el::CALLED).then_some(())).is_some());
    // B comes up and opens; A stays downstairs with its doors shut.
    let state = wait_for(&ola, &[], Duration::from_millis(4000), |p| match p {
        Packet::Doors { tiles, lifts, .. } if lifts.len() == 2 && lifts[1].floor == 1 && !lifts[1].moving && !tiles.contains(&(41, 43)) => {
            Some((lifts[0], tiles.contains(&(37, 43))))
        }
        _ => None,
    });
    let (a, a_shut) = state.expect("lift B came up and opened");
    assert_eq!((a.floor, a.moving), (0, false), "lift A didn't move");
    assert!(a_shut, "lift A's doors stay shut");
}

#[test]
fn toilet_dirty_hands_witness_and_washing() {
    use game::needs::lines as nl;
    let (addr, _) = start_server_cfg(0, true, true);
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola"); // IT
    let (mut kuba, _) = Client::connect(addr, "Kuba"); // Biznes
    let wait = Duration::from_millis(800);
    let seat = |dept: u8| {
        let ws = game::computer::find_workstations(&b);
        let w = ws.iter().find(|w| w.department == dept).unwrap();
        Body { access: access::CARD, ..Body::at(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1)) }
    };
    let said = |line: String| move |p: &Packet| matches!(p, Packet::Say { text, .. } if *text == line).then_some(());
    let dirty = |c: &Client, keep: &Client, want: bool| {
        wait_for(c, &[keep], Duration::from_millis(1500), |p| match p {
            Packet::Stats { flags, .. } if (flags & proto::STATS_DIRTY_HANDS != 0) == want => Some(()),
            _ => None,
        })
        .is_some()
    };

    // Kuba waits in the women's bathroom by the stalls; Ola uses a toilet.
    let k = kuba.walk_to(&b, seat(2), (1, Tile { x: 7, y: 47 }), &[&ola]);
    let o = ola.walk_to(&b, seat(1), (1, Tile { x: 4, y: 45 }), &[&kuba]);
    let o = ola.press_e(&b, o);
    assert!(dirty(&ola, &kuba, true), "toilet -> dirty hands");
    // Out of the bathroom without washing: Kuba notices.
    while ola.recv().is_some() {}
    let o = ola.walk_to(&b, o, (1, Tile { x: 13, y: 46 }), &[&kuba]);
    assert!(wait_for(&ola, &[&kuba], wait, said("Ej, Ola, a ręce?!".into())).is_some());
    // Back to the sink: 5 s of washing, clean hands.
    let o = ola.walk_to(&b, o, (1, Tile { x: 9, y: 45 }), &[&kuba]);
    ola.press_e(&b, o);
    assert!(wait_for(&ola, &[&kuba], wait, said(nl::WASHING.into())).is_some());
    assert!(wait_for(&ola, &[&kuba], Duration::from_millis(6000), said(nl::WASHED.into())).is_some());
    assert!(dirty(&ola, &kuba, false), "washed");
    let _ = k;
}

#[test]
fn shop_take_from_shelf_alarm_and_pay() {
    use game::inventory::kind as item_kind;
    use game::shop::lines as sl;
    let (addr, _) = start_server_cfg(0, true, true); // hired: 200 zł advance
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola");
    let wait = Duration::from_millis(800);
    let said = |pred: fn(&str) -> bool| move |p: &Packet| matches!(p, Packet::Say { text, .. } if pred(text)).then_some(());
    let money = |c: &Client| {
        wait_for(c, &[], Duration::from_millis(1200), |p| if let Packet::Stats { money, .. } = p { Some(*money) } else { None })
    };
    // (Resent every 2 s, so a missed one comes again.)
    let inventory = |c: &Client| {
        wait_for(c, &[], Duration::from_millis(2500), |p| if let Packet::Inventory { slots } = p { Some(slots.clone()) } else { None })
    };
    let take = |c: &Client, shelf: u8, kind: u8| c.send(&Packet::ShopTake { token: c.token, shelf, kind });
    let ws = game::computer::find_workstations(&b);
    let w = ws.iter().find(|w| w.department == 1).unwrap();
    let body = Body { access: access::CARD, ..Body::at(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1)) };
    assert_eq!(money(&ola), Some(200_00));

    // Downstairs to the sandwich shelf: E shows what's on it.
    let body = ola.walk_to(&b, body, (0, Tile { x: 22, y: 47 }), &[]);
    while ola.recv().is_some() {}
    let body = ola.press_e(&b, body);
    let goods = wait_for(&ola, &[], wait, |p| match p {
        Packet::Shelf { shelf: 1, goods, .. } => Some(goods.iter().map(|g| g.kind).collect::<Vec<_>>()),
        _ => None,
    });
    assert_eq!(goods, Some(vec![item_kind::SANDWICH_CHEESE, item_kind::SANDWICH_HAM, item_kind::WRAP]));
    take(&ola, 1, item_kind::SANDWICH_HAM);
    let slots = wait_for(&ola, &[], INVENTORY_WAIT, |p| match p {
        Packet::Inventory { slots } if slots.iter().any(|s| s.kind == item_kind::SANDWICH_HAM) => Some(slots.clone()),
        _ => None,
    })
    .expect("sandwich in a pocket");
    assert!(slots.iter().any(|s| s.label.contains("niezapłacone")));

    // Walking out without paying: beep, the guard comes after Ola and the
    // sandwich goes back.
    let out = ola.walk_to(&b, body, (0, Tile { x: 22, y: 59 }), &[]);
    assert!(wait_for(&ola, &[], wait, said(|t| t == sl::ALARM)).is_some());
    assert!(wait_for(&ola, &[], Duration::from_millis(3000), said(|t| t == security::lines::GUARD_CAUGHT)).is_some());
    assert!(inventory(&ola).is_some_and(|s| s.iter().all(|s| s.kind != item_kind::SANDWICH_HAM)));
    // Caught: can't walk for a moment (inputs acknowledged, ignored).
    let held_at = wait_for(&ola, &[], wait, |p| match p {
        Packet::Snapshot { self_x, self_y, self_activity, .. } if *self_activity == proto::activity::HELD => Some((*self_x, *self_y)),
        _ => None,
    })
    .expect("held by the guard");
    ola.send_inputs(sim::IN_LEFT, 6);
    let after = wait_for(&ola, &[], Duration::from_millis(300), |p| match p {
        Packet::Snapshot { self_x, self_y, last_input_seq, .. } if *last_input_seq == ola.seq => Some((*self_x, *self_y)),
        _ => None,
    });
    assert_eq!(after, Some(held_at), "no walking while held");
    let _ = wait_for(&ola, &[], Duration::from_millis(3200), |_| None::<()>); // let go after 3 s
    let out = Body { pos: Pos { x: held_at.0, y: held_at.1 }, ..out };

    // Back in, take it again, pay at the till, eat it.
    let body = ola.walk_to(&b, out, (0, Tile { x: 22, y: 47 }), &[]);
    take(&ola, 1, item_kind::SANDWICH_HAM);
    std::thread::sleep(Duration::from_millis(150));
    let body = ola.walk_to(&b, body, (0, TILL), &[]);
    while ola.recv().is_some() {}
    let body = ola.press_e(&b, body);
    assert!(wait_for(&ola, &[], wait, said(|t| t.starts_with("Razem 14,00 zł. Dziękuję!"))).is_some());
    assert_eq!(money(&ola), Some(186_00));
    let slots = inventory(&ola).unwrap();
    let pocket = slots[1..].iter().position(|s| s.kind == item_kind::SANDWICH_HAM).expect("paid sandwich");
    assert!(!slots[pocket + 1].label.contains("niezapłacone"));

    // Once more without paying the same day: the guard calls the police, a
    // patrol car pulls up and the officer fines Ola (all she has left).
    let body = ola.walk_to(&b, body, (0, Tile { x: 22, y: 47 }), &[]);
    take(&ola, 1, item_kind::WRAP);
    std::thread::sleep(Duration::from_millis(150));
    let _out = ola.walk_to(&b, body, (0, Tile { x: 22, y: 59 }), &[]);
    assert!(wait_for(&ola, &[], Duration::from_millis(3000), said(|t| t == security::lines::GUARD_POLICE_AGAIN)).is_some());
    let fine = security::lines::police_fine(186_00);
    assert!(wait_for(&ola, &[], Duration::from_millis(8000), |p| matches!(p, Packet::Say { text, .. } if *text == fine).then_some(()))
        .is_some());
    assert_eq!(money(&ola), Some(0));
}

/// Latest Clock packet matching `f` within `wait`.
fn clock_until(c: &Client, wait: Duration, f: impl Fn(u16, u16, u8, u16, u32) -> bool) -> Option<(u16, u16, u8, u16, u32)> {
    wait_for(c, &[], wait, |p| match p {
        Packet::Clock { day, minute, place, arrive, pay, .. } if f(*day, *minute, *place, *arrive, *pay) => {
            Some((*day, *minute, *place, *arrive, *pay))
        }
        _ => None,
    })
}

#[test]
fn office_closes_at_ten_pm_and_pays_the_day() {
    use proto::place;
    // 21:50, daytime 60x faster: 10 game minutes = 10 s / 60... ~2 s.
    let (addr, _) = start_server_at(0, true, true, 21 * 60 + 50, 60);
    let (ola, _) = Client::connect(addr, "Ola");
    let (day, _, pl, _, _) = clock_until(&ola, Duration::from_millis(800), |_, _, _, _, _| true).expect("clock");
    assert_eq!((day, pl), (2, place::BUILDING), "hired = day 2, in the building");
    let home = clock_until(&ola, Duration::from_millis(6000), |_, _, pl, _, _| pl == place::HOME);
    let (_, minute, _, _, pay) = home.expect("sent home at 22:00");
    assert!(minute >= 22 * 60, "evening: {minute}");
    // ~10 game minutes worked at 30 zł/h = ~5 zł.
    assert!((4_00..=6_00).contains(&pay), "paid for the minutes worked: {pay}");
    // No more snapshots: out of the building.
    while ola.recv().is_some() {}
    ola.ping();
    std::thread::sleep(Duration::from_millis(200));
    let mut snapshots = 0;
    while let Some(p) = ola.recv() {
        snapshots += matches!(p, Packet::Snapshot { .. }) as u32;
    }
    assert_eq!(snapshots, 0);
}

#[test]
fn morning_commute_choice_ride_and_arrival() {
    use game::commute::mode;
    use proto::place;
    // 5:58 (night: at home), daytime 450x faster so the morning goes quickly
    // (but the car still gets in before the office closes at 22:00).
    let (addr, _) = start_server_at(0, true, true, 5 * 60 + 58, 450);
    let (ola, _) = Client::connect(addr, "Ola");
    let (day, _, pl, _, _) = clock_until(&ola, Duration::from_millis(800), |_, _, _, _, _| true).expect("clock");
    assert_eq!((day, pl), (2, place::HOME), "hired at night: at home until the morning");
    // Morning: choose the car before leaving.
    let depart = wait_for(&ola, &[], Duration::from_millis(3000), |p| match p {
        Packet::Clock { place: place::COMMUTING, depart, day: 3, .. } if *depart != proto::NO_TIME => Some(*depart),
        _ => None,
    })
    .expect("morning: a departure time");
    assert!((6 * 60 + 15..=8 * 60 + 45).contains(&depart), "leaves 6:15-8:45: {depart}");
    ola.send(&Packet::CommuteChoice { token: ola.token, mode: mode::CAR });
    // Leaves: pays for fuel; arrives, rides in, gets out on the car park.
    let paid = wait_for(&ola, &[], Duration::from_millis(5000), |p| match p {
        Packet::Clock { mode: m, money, arrive, .. } if *m == mode::CAR && *arrive != proto::NO_TIME => Some(*money),
        _ => None,
    });
    assert_eq!(paid, Some(200_00 - 12_00), "fuel: 12 zł");
    let riding = wait_for(&ola, &[], Duration::from_millis(8000), |p| match p {
        Packet::Snapshot { self_activity, .. } if *self_activity == proto::activity::RIDING => Some(()),
        _ => None,
    });
    assert!(riding.is_some(), "rides in the car");
    // (Along the whole street from the east edge: ~7 s.)
    let out = wait_for(&ola, &[], Duration::from_millis(12000), |p| match p {
        Packet::Snapshot { self_activity, self_x, self_y, floor: 0, .. } if *self_activity != proto::activity::RIDING => {
            Some(Pos { x: *self_x, y: *self_y }.tile())
        }
        _ => None,
    });
    let (tx, ty) = out.expect("got out");
    assert!((65..=67).contains(&ty) && tx < 31, "on the outside car park: ({tx},{ty})");
}

#[test]
fn rain_soaks_you_outdoors() {
    WEATHER.with(|w| w.set(Some(game::weather::kind::RAIN)));
    let (addr, _) = start_server_with(0); // spawns on the sidewalk: outdoors
    let (ola, _) = Client::connect(addr, "Ola");
    let hygiene = |c: &Client| {
        wait_for(c, &[], Duration::from_millis(1200), |p| if let Packet::Stats { hygiene, .. } = p { Some(*hygiene) } else { None })
    };
    let soaked = wait_for(&ola, &[], Duration::from_millis(1500), |p| {
        matches!(p, Packet::Say { text, .. } if text == game::weather::lines::SOAKED).then_some(())
    });
    assert!(soaked.is_some(), "told it's pouring");
    let before = hygiene(&ola).unwrap();
    let weather =
        wait_for(&ola, &[], Duration::from_millis(1500), |p| if let Packet::Clock { weather, .. } = p { Some(*weather) } else { None });
    assert_eq!(weather, Some(game::weather::kind::RAIN));
    std::thread::sleep(Duration::from_millis(100));
    ola.ping();
    let after = wait_for(&ola, &[], Duration::from_millis(3000), |p| match p {
        Packet::Stats { hygiene, .. } if *hygiene + 1 < before => Some(*hygiene),
        _ => None,
    });
    assert!(after.is_some(), "hygiene drops in the rain (was {before})");
}

#[test]
fn calendar_meeting_with_the_ceo() {
    use game::board::topic;
    use proto::computer_action as ca;
    // 9:49: book the 10:00 slot; the board-room door opens from 9:50.
    let (addr, _) = start_server_at(0, true, true, 9 * 60 + 49, 1);
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola"); // IT, at her desk upstairs
    let wait = Duration::from_millis(800);
    let access =
        |c: &Client| wait_for(c, &[], wait, |p| if let Packet::Snapshot { self_access, .. } = p { Some(*self_access) } else { None });
    let ws = game::computer::find_workstations(&b);
    let w = ws.iter().find(|w| w.department == 1).unwrap();
    let body = Body { access: access::CARD, ..Body::at(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1)) };
    // Laptop on the desk, sit down: the calendar comes with the screen.
    let body = ola.press_e(&b, body);
    std::thread::sleep(Duration::from_millis(150));
    let body = ola.press_e(&b, body);
    let slots = wait_for(&ola, &[], Duration::from_millis(1500), |p| match p {
        Packet::Calendar { slots, .. } => Some(slots.clone()),
        _ => None,
    })
    .expect("calendar");
    assert_eq!(slots.first(), Some(&(600, proto::slot::FREE)), "10:00 is free");
    ola.send(&Packet::CalendarBook { token: ola.token, start: 600, topic: topic::CHAT });
    let mine = wait_for(&ola, &[], wait, |p| match p {
        Packet::Calendar { mine_start, mine_topic, .. } if *mine_start == 600 => Some(*mine_topic),
        _ => None,
    });
    assert_eq!(mine, Some(topic::CHAT));
    ola.send(&Packet::ComputerAction { token: ola.token, action: ca::CLOSE, conv: 0, arg: 0, text: String::new() });
    assert_eq!(access(&ola).map(|a| a & access::BOARD), Some(0), "9:49: the door is still closed");
    // By 9:50 the door lets her in: walk into the board room, talk to the CEO.
    let open = wait_for(&ola, &[], Duration::from_millis(8000), |p| match p {
        Packet::Snapshot { self_access, .. } if self_access & access::BOARD != 0 => Some(()),
        _ => None,
    });
    assert!(open.is_some(), "door open around the meeting");
    let body = Body { access: access::CARD | access::BOARD, ..body };
    let body = ola.walk_to(&b, body, (1, Tile { x: 20, y: 18 }), &[]); // by the CEO
    while ola.recv().is_some() {}
    ola.press_e(&b, body);
    let dialog = wait_for(&ola, &[], wait, |p| match p {
        Packet::Dialog { id, options, .. } if *id != 0 => Some((*id, options.len())),
        _ => None,
    });
    let (id, n) = dialog.expect("the CEO talks");
    assert_eq!(n, 3);
    ola.send(&Packet::DialogAnswer { token: ola.token, id, choice: 0 });
    let closed = wait_for(&ola, &[], wait, |p| matches!(p, Packet::Dialog { id: 0, .. }).then_some(()));
    assert!(closed.is_some(), "meeting over");
}

#[test]
fn sweets_tray_in_the_chill_room() {
    use game::inventory::kind as item_kind;
    let (addr, _) = start_server_cfg(0, true, true); // treats_now: a tray is out
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola");
    let wait = Duration::from_millis(800);
    let ws = game::computer::find_workstations(&b);
    let w = ws.iter().find(|w| w.department == 1).unwrap();
    let body = Body { access: access::CARD, ..Body::at(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1)) };
    let body = ola.walk_to(&b, body, (1, Tile { x: 35, y: 8 }), &[]);
    while ola.recv().is_some() {}
    let tray = |c: &Client| {
        wait_for(c, &[], wait, |p| match p {
            Packet::Snapshot { entities, .. } => entities.iter().find(|e| e.kind == proto::kind::TRAY).map(|e| (e.held, e.activity)),
            _ => None,
        })
    };
    let (sweet, pieces) = tray(&ola).expect("a tray on the table");
    assert!([item_kind::DONUT, item_kind::COOKIE, item_kind::CHEESECAKE].contains(&sweet));
    assert!((4..=8).contains(&pieces));
    ola.press_e(&b, body);
    let got = wait_for(&ola, &[], Duration::from_millis(2500), |p| match p {
        Packet::Inventory { slots } if slots.iter().any(|s| s.kind == sweet) => Some(()),
        _ => None,
    });
    assert!(got.is_some(), "a sweet in the pocket");
    let after = if pieces == 1 { None } else { tray(&ola).map(|t| t.1) };
    if pieces > 1 {
        assert_eq!(after, Some(pieces - 1), "one piece fewer");
    }
}

#[test]
fn lunch_ordered_in_the_app_and_picked_up_at_the_reception() {
    use game::inventory::kind as item_kind;
    use proto::computer_action as ca;
    // 10:30, daytime 30x faster: the courier comes in a few seconds.
    let (addr, _) = start_server_at(0, true, true, 10 * 60 + 30, 30);
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola"); // IT, at her desk, 200 zł
    let ws = game::computer::find_workstations(&b);
    let w = ws.iter().find(|w| w.department == 1).unwrap();
    let body = Body { access: access::CARD, ..Body::at(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1)) };
    let body = ola.press_e(&b, body);
    std::thread::sleep(Duration::from_millis(150));
    let body = ola.press_e(&b, body);
    let dishes = wait_for(&ola, &[], Duration::from_millis(1500), |p| match p {
        Packet::LunchMenu { state: 0, dishes, .. } => Some(dishes.len()),
        _ => None,
    });
    assert_eq!(dishes, Some(6), "the lunch app with the menu");
    ola.send(&Packet::LunchOrder { token: ola.token, dish: item_kind::KEBAB });
    let ordered = wait_for(&ola, &[], Duration::from_millis(1500), |p| match p {
        Packet::LunchMenu { state: 1, dish, .. } => Some(*dish),
        _ => None,
    });
    assert_eq!(ordered, Some(item_kind::KEBAB));
    let money = wait_for(&ola, &[], Duration::from_millis(1500), |p| if let Packet::Stats { money, .. } = p { Some(*money) } else { None });
    assert_eq!(money, Some(200_00 - 25_00));
    ola.send(&Packet::ComputerAction { token: ola.token, action: ca::CLOSE, conv: 0, arg: 0, text: String::new() });
    // To the reception while the courier is on the way; told when it's there.
    let body = ola.walk_to(&b, body, (1, Tile { x: 36, y: 36 }), &[]);
    let told = wait_for(&ola, &[], Duration::from_millis(8000), |p| {
        matches!(p, Packet::Say { text, .. } if text.starts_with("Kurier był! Kebab")).then_some(())
    });
    assert!(told.is_some(), "the reception says the courier came");
    ola.press_e(&b, body);
    let got = wait_for(&ola, &[], Duration::from_millis(2500), |p| match p {
        Packet::Inventory { slots } if slots[0].kind == item_kind::KEBAB => Some(()),
        _ => None,
    });
    assert!(got.is_some(), "the lunch box in her hands");
}

#[test]
fn a_filled_position_is_gone_for_the_others() {
    let (addr, _) = start_server_full(0, false); // job portal
    let bank = Recruitment::load(&default_recruitment_path()).unwrap();
    let (ala, _) = Client::connect(addr, "Ala");
    let (bob, _) = Client::connect(addr, "Bob");
    // Both apply for the one programmer position; both get invited.
    for c in [&ala, &bob] {
        c.send(&Packet::Apply { token: c.token, offer: 1, motivation: "Kocham kod.".into(), salary: 8000, form: 1, student: false });
    }
    let invited = |c: &Client, other: &Client| {
        wait_for(c, &[other], Duration::from_millis(3000), |p| match p {
            Packet::Mail { action, arg: 1, .. } if *action == proto::portal_action::JOIN_INTERVIEW => Some(()),
            _ => None,
        })
    };
    assert!(invited(&ala, &bob).is_some() && invited(&bob, &ala).is_some());
    // Ala takes the interview and passes.
    ala.send(&Packet::PortalAction { token: ala.token, action: proto::portal_action::JOIN_INTERVIEW, arg: 1 });
    let mut passed = false;
    let deadline = Instant::now() + Duration::from_millis(5000);
    while !passed && Instant::now() < deadline {
        ala.ping();
        bob.ping();
        while bob.recv().is_some() {}
        while let Some(p) = ala.recv() {
            match p {
                Packet::Question { attempt, index, text, options, .. } => {
                    let q = bank.set("programming").unwrap().questions.iter().find(|q| q.text == text).unwrap();
                    let right = options.iter().position(|o| *o == q.options[0]).unwrap() as u8;
                    ala.send(&Packet::Answer { token: ala.token, attempt, index, choice: right });
                }
                Packet::RecruitResult { passed: true, .. } => passed = true,
                _ => {}
            }
        }
    }
    assert!(passed, "Ala got the job");
    // Bob, still waiting with his invitation, hears it's been filled; the
    // portal shows no free programmer place any more.
    let filled = wait_for(&bob, &[&ala], Duration::from_millis(3000), |p| {
        matches!(p, Packet::Mail { subject, .. } if subject.starts_with("Stanowisko obsadzone")).then_some(())
    });
    assert!(filled.is_some(), "Bob is told the position is filled");
    let free = wait_for(&bob, &[&ala], Duration::from_millis(3000), |p| match p {
        Packet::JobOffers { offers } => offers.iter().find(|o| o.id == 1).map(|o| o.vacancies),
        _ => None,
    });
    assert_eq!(free, Some(0));
}

#[test]
fn founder_founds_the_company_and_hires_from_the_panel() {
    use game::company::action as ca;
    use proto::computer_action as pc;
    let (addr, _) = start_server_full(0, false); // job portal
    let b = building();
    let bank = Recruitment::load(&default_recruitment_path()).unwrap();
    let (mut ola, _) = Client::connect(addr, "Ola");
    let clock = |c: &Client, keep: &[&Client]| {
        wait_for(c, keep, Duration::from_millis(1500), |p| match p {
            Packet::Clock { company, founded, .. } => Some((company.clone(), *founded)),
            _ => None,
        })
    };
    assert_eq!(clock(&ola, &[]).map(|c| c.1), Some(false), "no founder yet");
    let act = |c: &Client, action: u8, target: u16, value: u8, text: &str| {
        c.send(&Packet::CompanyAction { token: c.token, action, target, value, text: text.into() });
    };
    act(&ola, ca::FOUND, 0, 0, "Pixel Pierogi sp. z o.o.");
    let founded = wait_for(&ola, &[], Duration::from_millis(1500), |p| match p {
        Packet::Clock { company, founded: true, .. } => Some(company.clone()),
        _ => None,
    });
    assert_eq!(founded.as_deref(), Some("Pixel Pierogi sp. z o.o."));
    // In the board room with a card and a laptop: put it on the table, sit down.
    let body = Body { access: access::CARD | access::BOARD, ..Body::at(1, Pos::tile_center(50, 18)) };
    let body = ola.press_e(&b, body);
    std::thread::sleep(Duration::from_millis(150));
    ola.press_e(&b, body);
    let panel = wait_for(&ola, &[], Duration::from_millis(2500), |p| match p {
        Packet::CompanyOffers { name, offers, .. } => Some((name.clone(), offers.len())),
        _ => None,
    });
    assert_eq!(panel, Some(("Pixel Pierogi sp. z o.o.".into(), 4)), "the company panel");

    // Bob sees the new name on the portal, applies, passes: waits for Ola.
    let (bob, _) = Client::connect(addr, "Bob");
    let name = wait_for(&bob, &[&ola], Duration::from_millis(2000), |p| match p {
        Packet::JobOffers { offers } => offers.iter().find(|o| o.id == 1).map(|o| o.company.clone()),
        _ => None,
    });
    assert_eq!(name.as_deref(), Some("Pixel Pierogi sp. z o.o."));
    bob.send(&Packet::Apply { token: bob.token, offer: 1, motivation: "Chcę pierogi.".into(), salary: 8000, form: 1, student: false });
    let invited = wait_for(&bob, &[&ola], Duration::from_millis(3000), |p| {
        matches!(p, Packet::Mail { action, arg: 1, .. } if *action == proto::portal_action::JOIN_INTERVIEW).then_some(())
    });
    assert!(invited.is_some());
    bob.send(&Packet::PortalAction { token: bob.token, action: proto::portal_action::JOIN_INTERVIEW, arg: 1 });
    let deadline = Instant::now() + Duration::from_millis(5000);
    let mut awaiting = false;
    while !awaiting && Instant::now() < deadline {
        bob.ping();
        ola.ping();
        while ola.recv().is_some() {}
        while let Some(p) = bob.recv() {
            match p {
                Packet::Question { attempt, index, text, options, .. } => {
                    let q = bank.set("programming").unwrap().questions.iter().find(|q| q.text == text).unwrap();
                    let right = options.iter().position(|o| *o == q.options[0]).unwrap() as u8;
                    bob.send(&Packet::Answer { token: bob.token, attempt, index, choice: right });
                }
                Packet::Mail { subject, .. } if subject == "Decyzja zarządu wkrótce" => awaiting = true,
                _ => {}
            }
        }
    }
    assert!(awaiting, "Bob waits for the founder's decision");
    // Ola sees him in the panel and hires him; Bob gets the trial-day invitation.
    let cand = wait_for(&ola, &[&bob], Duration::from_millis(2500), |p| match p {
        Packet::CompanyPeople { candidates, .. } => candidates.iter().find(|c| c.4 == "Bob").map(|c| c.0),
        _ => None,
    });
    let bob_id = cand.expect("Bob among the candidates");
    act(&ola, ca::HIRE, bob_id, 0, "");
    let hired = wait_for(&bob, &[&ola], Duration::from_millis(2500), |p| {
        matches!(p, Packet::Mail { action, .. } if *action == proto::portal_action::GO_TO_OFFICE).then_some(())
    });
    assert!(hired.is_some(), "hired by the founder");
    // More places for designers.
    act(&ola, ca::SET_PLACES, 2, 2, "");
    let places = wait_for(&ola, &[&bob], Duration::from_millis(2500), |p| match p {
        Packet::CompanyOffers { offers, .. } => offers.iter().find(|o| o.id == 2).map(|o| o.places),
        _ => None,
    });
    assert_eq!(places, Some(2));
    // Bob is on the team (hired, contract still to sign) - and gets fired.
    let staff = wait_for(&ola, &[&bob], Duration::from_millis(2500), |p| match p {
        Packet::CompanyPeople { staff, .. } => staff.iter().find(|s| s.0 == bob_id).map(|s| s.1),
        _ => None,
    });
    assert_eq!(staff, Some(1), "Bob in IT");
    act(&ola, ca::FIRE, bob_id, 0, "");
    let fired = wait_for(&bob, &[&ola], Duration::from_millis(2500), |p| {
        matches!(p, Packet::Mail { subject, .. } if subject == "Rozwiązanie umowy").then_some(())
    });
    assert!(fired.is_some(), "Bob fired");

    // A new position: Office manager in Biznes, with the general questions.
    act(&ola, ca::ADD_POSITION, 0, 2, "Office manager\nnie-ma-takiego\n");
    let refused = wait_for(&ola, &[&bob], Duration::from_millis(1500), |p| {
        matches!(p, Packet::Say { text, .. } if text == game::server::position_lines::BAD_SET).then_some(())
    });
    assert!(refused.is_some(), "no such question set");
    act(&ola, ca::ADD_POSITION, 0, 2, "Office manager\ngeneral\nOgarniasz biuro, kawę i ludzi.");
    let added = wait_for(&ola, &[&bob], Duration::from_millis(2500), |p| match p {
        Packet::CompanyOffers { offers, sets, .. } if !sets.is_empty() => {
            offers.iter().find(|o| o.title == "Office manager").map(|o| (o.id, o.department, o.set.clone(), o.places, sets.len()))
        }
        _ => None,
    });
    let (new_id, dept, set, places, sets) = added.expect("the new position in the panel");
    assert!(new_id >= game::company::FIRST_CUSTOM_ID);
    assert_eq!((dept, set.as_str(), places, sets), (2, "general", 1, 5));
    // Ewa finds it on the portal, applies and gets the general questions.
    let (ewa, _) = Client::connect(addr, "Ewa");
    let seen = wait_for(&ewa, &[&ola, &bob], Duration::from_millis(2500), |p| match p {
        Packet::JobOffers { offers } => offers.iter().find(|o| o.id == new_id).map(|o| (o.title.clone(), o.vacancies)),
        _ => None,
    });
    assert_eq!(seen, Some(("Office manager".into(), 1)));
    ewa.send(&Packet::Apply { token: ewa.token, offer: new_id, motivation: String::new(), salary: 8000, form: 1, student: false });
    assert!(wait_for(&ewa, &[&ola, &bob], Duration::from_millis(3000), |p| {
        matches!(p, Packet::Mail { action, arg, .. } if *action == proto::portal_action::JOIN_INTERVIEW && *arg == new_id).then_some(())
    })
    .is_some());
    ewa.send(&Packet::PortalAction { token: ewa.token, action: proto::portal_action::JOIN_INTERVIEW, arg: new_id });
    let question = wait_for(&ewa, &[&ola, &bob], Duration::from_millis(2500), |p| match p {
        Packet::Question { text, .. } => Some(text.clone()),
        _ => None,
    })
    .expect("an interview question");
    assert!(bank.set("general").unwrap().questions.iter().any(|q| q.text == question), "from the general set: {question}");
    // Ola closes the position mid-interview: Ewa hears the recruitment is over.
    act(&ola, ca::REMOVE_POSITION, new_id as u16, 0, "");
    let closed = wait_for(&ewa, &[&ola, &bob], Duration::from_millis(2500), |p| {
        matches!(p, Packet::Mail { subject, .. } if subject == "Rekrutacja zakończona: Office manager").then_some(())
    });
    assert!(closed.is_some(), "Ewa is told");
    let gone = wait_for(&ola, &[&bob, &ewa], Duration::from_millis(2500), |p| match p {
        Packet::CompanyOffers { offers, part: 0, .. } => Some(offers.iter().any(|o| o.id == new_id)),
        _ => None,
    });
    assert_eq!(gone, Some(false), "gone from the panel");
    let _ = pc::CLOSE;
}

#[test]
fn task_board_per_department_and_work_mail() {
    use game::computer::lines as pc;
    use proto::{mail_action as ma, task_action as ta};
    let (addr, _) = start_server_cfg(0, true, true); // hired: card + laptop, at a desk
    let b = building();
    let (mut ola, _) = Client::connect(addr, "Ola"); // id 1: IT
    let (mut kuba, _) = Client::connect(addr, "Kuba"); // id 2: Biznes
    let (mut ewa, _) = Client::connect(addr, "Ewa"); // id 3: IT
    let wait = Duration::from_millis(800);
    let nobody = Body::at(1, Pos::tile_center(0, 0));
    let said = |line: &'static str| move |p: &Packet| matches!(p, Packet::Say { text, .. } if text == line).then_some(());
    // Everybody puts the laptop down and sits at it.
    let sit = |me: &mut Client, others: [&Client; 2]| {
        me.press_e(&b, nobody);
        assert!(wait_for(me, &others, wait, said(pc::PLACED)).is_some());
        me.press_e(&b, nobody);
        assert!(wait_for(me, &others, wait, |p| matches!(p, Packet::Computer { .. }).then_some(())).is_some());
    };
    sit(&mut ola, [&kuba, &ewa]);
    sit(&mut kuba, [&ola, &ewa]);
    sit(&mut ewa, [&ola, &kuba]);
    let task = |c: &Client, nonce: u16, action: u8, task: u16, arg: u8, text: &str| {
        c.send(&Packet::TaskAction { token: c.token, nonce, action, task, arg, text: text.into() });
    };
    let board = |c: &Client, keep: &[&Client], done: u16| {
        wait_for(c, keep, Duration::from_millis(800), |p| match p {
            Packet::TaskBoard { done: d, part: 0, members, tasks, .. } if *d == done => Some((members.clone(), tasks.clone())),
            _ => None,
        })
    };

    // Ola adds an urgent card; a retry with the same nonce adds nothing.
    task(&ola, 1, ta::CREATE, 0, 2, "Naprawić logowanie\nPo zmianie hasła nie działa.");
    task(&ola, 1, ta::CREATE, 0, 2, "Naprawić logowanie\nPo zmianie hasła nie działa.");
    std::thread::sleep(Duration::from_millis(150));
    task(&ola, 1, ta::SYNC, 0, 0, "");
    let (members, cards) = board(&ola, &[&kuba, &ewa], 1).expect("Ola's board");
    assert_eq!(members, ["Ewa", "Ola"], "the IT department");
    assert_eq!(cards.len(), 1, "the retry was ignored");
    let id = cards[0].id;
    assert_eq!((cards[0].column, cards[0].priority, cards[0].author.as_str()), (0, 2, "Ola"));

    // Ola assigns it to Ewa: Ewa gets an e-mail and moves the card on.
    task(&ola, 2, ta::ASSIGN, id, 0, "Ewa");
    let mail = |c: &Client, nonce: u16, action: u8, id: u16, to: &str, subject: &str, body: &str| {
        let p = Packet::MailAction { token: c.token, nonce, action, id, to: to.into(), subject: subject.into(), body: body.into() };
        c.send(&p);
    };
    std::thread::sleep(Duration::from_millis(150));
    mail(&ewa, 0, ma::SYNC, 0, "", "", "");
    let got = wait_for(&ewa, &[&ola, &kuba], wait, |p| match p {
        Packet::WorkMail { from, subject, .. } if from == "Tablica zadań" => Some(subject.clone()),
        _ => None,
    });
    assert_eq!(got.as_deref(), Some("Nowe zadanie: Naprawić logowanie"));
    task(&ewa, 1, ta::MOVE, id, 1, "");
    task(&ewa, 2, ta::COMMENT, id, 0, "Biorę się za to");
    std::thread::sleep(Duration::from_millis(150));
    task(&ola, 0, ta::SYNC, id, 0, "");
    let (_, cards) = board(&ola, &[&kuba, &ewa], 2).expect("board again");
    assert_eq!((cards[0].column, cards[0].assignee.as_str(), cards[0].comments), (1, "Ewa", 1));
    let detail = wait_for(&ola, &[&kuba, &ewa], wait, |p| match p {
        Packet::TaskDetail { id: i, desc, comments } if *i == id => Some((desc.clone(), comments.clone())),
        _ => None,
    });
    assert_eq!(detail, Some(("Po zmianie hasła nie działa.".into(), vec![("Ewa".into(), "Biorę się za to".into())])));

    // Biznes has its own (empty) board.
    task(&kuba, 0, ta::SYNC, 0, 0, "");
    let (members, cards) = board(&kuba, &[&ola, &ewa], 0).expect("Kuba's board");
    assert_eq!((members, cards.len()), (vec!["Kuba".to_string()], 0));

    // Ewa writes to Ola; Ola reads it and puts it in the trash.
    mail(&ewa, 3, ma::SEND, 0, "Ola", "Kawa?", "O 12 w kuchni.");
    assert!(wait_for(&ewa, &[&ola, &kuba], wait, said(game::workmail::lines::SENT)).is_some());
    mail(&ola, 0, ma::SYNC, 0, "", "", "");
    let mid = wait_for(&ola, &[&kuba, &ewa], wait, |p| match p {
        Packet::WorkMail { id, from, body, .. } if from == "Ewa" && body == "O 12 w kuchni." => Some(*id),
        _ => None,
    })
    .expect("Ola got the mail");
    mail(&ola, 1, ma::TRASH, mid, "", "", "");
    let state = wait_for(&ola, &[&kuba, &ewa], wait, |p| match p {
        Packet::MailState { done: 1, ids, trashed } => Some((ids.clone(), trashed.clone())),
        _ => None,
    });
    assert_eq!(state.map(|(ids, t)| (ids.contains(&mid), t)), Some((true, vec![mid])));
}

#[test]
fn voice_reaches_the_room_and_whispers_only_the_one_next_to_you() {
    let (addr, _) = start_server_cfg(0, true, true); // hired, standing at their desks
    let b = building();
    let (ola, _) = Client::connect(addr, "Ola"); // id 1: IT
    let (kuba, _) = Client::connect(addr, "Kuba"); // id 2: Biznes (another room)
    let (mut ewa, _) = Client::connect(addr, "Ewa"); // id 3: IT, the next desk
    let say = |c: &Client, seq: u16, whisper: u8| {
        c.send(&Packet::Voice { token: c.token, seq, whisper, data: vec![1, 2, 3, 4] });
    };
    let heard = |c: &Client, keep: &[&Client], seq: u16| {
        wait_for(c, keep, Duration::from_millis(400), |p| match p {
            Packet::VoiceFrom { speaker, seq: s, whisper, data } if *s == seq => Some((*speaker, *whisper, data.clone())),
            _ => None,
        })
    };
    std::thread::sleep(Duration::from_millis(200));

    // Ola talks: Ewa (same room) hears it, Kuba (Biznes) doesn't.
    say(&ola, 1, 0);
    assert_eq!(heard(&ewa, &[&ola, &kuba], 1), Some((ola.id, 0, vec![1, 2, 3, 4])));
    say(&ola, 2, 0);
    assert_eq!(heard(&kuba, &[&ola, &ewa], 2), None, "another room");

    // Ewa sits at the next desk: a whisper reaches her (and only her).
    say(&ola, 3, 1);
    assert_eq!(heard(&ewa, &[&ola, &kuba], 3).map(|h| h.1), Some(1), "whispered to Ewa");
    say(&ola, 4, 1);
    assert_eq!(heard(&kuba, &[&ola, &ewa], 4), None);

    // Ewa walks to the far end of the room: the room still hears Ola, a
    // whisper doesn't reach anyone.
    let ws = game::computer::find_workstations(&b);
    let it: Vec<_> = ws.iter().filter(|w| w.department == 1).collect();
    let seat = |i: usize| Body { access: access::CARD, ..Body::at(it[i].floor, Pos::tile_center(it[i].tile.x, it[i].tile.y + 1)) };
    let m1 = b.floor(1).unwrap();
    let room_of = |w: &&game::computer::Workstation| m1.room_at_tile(w.tile.x, w.tile.y);
    let far = it.iter().rfind(|w| room_of(w) == room_of(&it[0])).unwrap(); // same room, far end
    ewa.walk_to(&b, seat(1), (1, Tile { x: far.tile.x, y: far.tile.y + 1 }), &[&ola, &kuba]);
    std::thread::sleep(Duration::from_millis(200));
    say(&ola, 5, 1);
    assert_eq!(heard(&ewa, &[&ola, &kuba], 5), None, "too far for a whisper");
    say(&ola, 6, 0);
    assert_eq!(heard(&ewa, &[&ola, &kuba], 6).map(|h| h.1), Some(0), "the room still hears");
}

/// The real server binary with a save file: play, Ctrl+C (SIGINT), start
/// again — the character, the laptop on the desk, the task board and the
/// mail are all still there.
#[cfg(unix)]
#[test]
fn progress_survives_a_server_restart() {
    use game::computer::lines as pc;
    use game::inventory::kind as item_kind;
    use proto::{mail_action as ma, task_action as ta};
    use std::process::{Child, Command, Stdio};
    let dir = std::env::temp_dir().join(format!("startup-sim-restart-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let save = dir.join("world.db");
    let port = UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let start = || -> Child {
        let child = Command::new(env!("CARGO_BIN_EXE_server"))
            .args(["--bind", &addr.to_string(), "--save", save.to_str().unwrap(), "--start-employed"])
            .args(["--start-time", "10:00", "--weather", "clouds", "--stats-secs", "3600"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_millis(800));
        child
    };
    let stop = |mut child: Child| {
        Command::new("kill").args(["-INT", &child.id().to_string()]).status().unwrap();
        let status = child.wait().unwrap();
        assert!(status.success(), "clean shutdown: {status}");
    };
    // The login API (HTTPS, the server's own certificate) next to the game port.
    let api = |path: &str, body: &str| -> serde_json::Value {
        let out = Command::new("curl")
            .args(["-sk", "-X", "POST", "-H", "content-type: application/json", "-d", body])
            .arg(format!("https://127.0.0.1:{}/api/{path}", port + 1))
            .output()
            .unwrap();
        serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null)
    };
    let ticket = |v: &serde_json::Value| v["ticket"].as_str().unwrap_or_default().to_string();
    let wait = Duration::from_millis(1500);
    let nobody = Body::at(1, Pos::tile_center(0, 0));
    let said = |line: &'static str| move |p: &Packet| matches!(p, Packet::Say { text, .. } if text == line).then_some(());
    let computer = |p: &Packet| match p {
        Packet::Computer { owner, .. } => Some(*owner),
        _ => None,
    };
    let clock = |p: &Packet| match p {
        Packet::Clock { money, day, place, .. } => Some((*money, *day, *place)),
        _ => None,
    };

    // Day one: the laptop on the desk, a card on the board, a note-to-self.
    let b = building();
    let server = start();
    let reg = api("register", r#"{"nick":"Ola","password":"tajne-haslo"}"#);
    assert_eq!(reg["ok"], true, "registered: {reg}");
    assert_eq!(reg["character"], false);
    // A client crash report lands in <save dir>/crashes.
    let report = api("crash", r#"{"version":"0.1.0","os":"macOS","log":"...\nProgram crashed with signal 11\n"}"#);
    assert_eq!(report["ok"], true, "crash report: {report}");
    let file = save.parent().unwrap().join("crashes").join(format!("{}.txt", report["id"].as_str().unwrap()));
    assert!(std::fs::read_to_string(&file).unwrap().contains("signal 11"), "saved as {}", file.display());
    // Guests are off on a server with accounts; a wrong ticket is refused.
    let refused = |c: &Client| {
        wait_for(c, &[], Duration::from_millis(500), |p| match p {
            Packet::Reject { reason } => Some(*reason),
            _ => None,
        })
    };
    let s = Client::socket_for(addr);
    let guest = Client { sock: s, id: 0, token: 0, seq: 0, crypto: None };
    guest.send(&Packet::Connect { nonce: 1, nick: "Ola".into(), profile: test_profile(), ticket: String::new() });
    assert_eq!(refused(&guest), Some(proto::reject::GUESTS_OFF));
    guest.send(&Packet::Connect { nonce: 2, nick: "Ola".into(), profile: test_profile(), ticket: "zly".into() });
    assert_eq!(refused(&guest), Some(proto::reject::BAD_TICKET));
    // A sniffed ticket alone is useless: the Connect must be sealed with the key.
    guest.send(&Packet::Connect { nonce: 3, nick: "Ola".into(), profile: test_profile(), ticket: ticket(&reg) });
    assert_eq!(refused(&guest), Some(proto::reject::BAD_TICKET));
    let key = |v: &serde_json::Value| v["key"].as_str().unwrap_or_default().to_string();
    let mut ola = Client::connect_sealed(addr, &ticket(&reg), &key(&reg));
    ola.press_e(&b, nobody);
    assert!(wait_for(&ola, &[], wait, said(pc::PLACED)).is_some());
    ola.press_e(&b, nobody);
    assert_eq!(wait_for(&ola, &[], wait, computer), Some(ola.id));
    let task = Packet::TaskAction { token: ola.token, nonce: 1, action: ta::CREATE, task: 0, arg: 2, text: "Przetrwać restart".into() };
    ola.send(&task);
    let mail = Packet::MailAction {
        token: ola.token,
        nonce: 1,
        action: ma::SEND,
        id: 0,
        to: "Ola".into(),
        subject: "Notatka".into(),
        body: "Nie zapomnij.".into(),
    };
    ola.send(&mail);
    assert!(wait_for(&ola, &[], wait, |p| matches!(p, Packet::MailState { done: 1, .. }).then_some(())).is_some());
    let (money, day, _) = wait_for(&ola, &[], wait, clock).expect("clock");
    std::thread::sleep(Duration::from_millis(300));
    stop(server);

    // The server comes back: Ola logs in again (the account knows her
    // character now) and is back.
    let server = start();
    // E-mails are unique: Ewa's new character can't take Ola's (Ola is
    // offline, her saved character counts), another one is fine.
    let ewa = api("register", r#"{"nick":"Ewa","password":"haslo-ewy-1"}"#);
    let olas = Profile { email: "TEST@firma.pl".into(), ..test_profile() };
    assert_eq!(Client::connect_sealed_as(addr, &ticket(&ewa), &key(&ewa), olas).err(), Some(proto::reject::EMAIL_TAKEN));
    let own = Profile { email: "ewa@firma.pl".into(), ..test_profile() };
    let ewa_client = Client::connect_sealed_as(addr, &ticket(&ewa), &key(&ewa), own).expect("Ewa with her own e-mail");
    ewa_client.send(&Packet::Disconnect { token: ewa_client.token, reason: 0 });
    let login = api("login", r#"{"nick":"ola","password":"tajne-haslo"}"#);
    assert_eq!((login["ok"].as_bool(), login["nick"].as_str(), login["character"].as_bool()), (Some(true), Some("Ola"), Some(true)));
    let mut ola = Client::connect_sealed(addr, &ticket(&login), &key(&login));
    // Recorded packets played again go nowhere: a sealed ping sent twice is
    // answered once; the recorded Connect makes no new session (Ola stays).
    {
        use game::crypto::{connect_prefix, from_hex, session_prefix, Dir, Keys};
        let keys = Keys::derive(&from_hex::<32>(&key(&login)).unwrap());
        let ping = Packet::Ping { token: ola.token, client_time: 77 }.encode();
        let sealed = keys.seal(Dir::ToServer, &session_prefix(ola.token), 1_000, &ping);
        ola.sock.send(&sealed).unwrap();
        ola.sock.send(&sealed).unwrap();
        ola.crypto.as_ref().unwrap().borrow_mut().send_counter = 1_000; // go on after it
        let mut pongs = 0;
        let deadline = Instant::now() + Duration::from_millis(500);
        while Instant::now() < deadline {
            if let Some(Packet::Pong { client_time: 77, .. }) = ola.recv() {
                pongs += 1;
            }
        }
        assert_eq!(pongs, 1, "the replay was dropped");
        let raw = from_hex::<32>(&ticket(&login)).unwrap();
        let connect = Packet::Connect { nonce: 42, nick: String::new(), profile: test_profile(), ticket: ticket(&login) }.encode();
        let spy = Client::socket_for(addr);
        spy.send(&keys.seal(Dir::ToServer, &connect_prefix(&raw), 1, &connect)).unwrap();
        std::thread::sleep(Duration::from_millis(200));
        let mut buf = [0u8; 2048];
        spy.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
        assert!(spy.recv(&mut buf).is_err(), "no Welcome for a replayed Connect");
        ola.ping();
        assert!(
            wait_for(&ola, &[], Duration::from_millis(500), |p| matches!(p, Packet::Pong { .. }).then_some(())).is_some(),
            "Ola still in"
        );
    }
    let (money2, day2, place) = wait_for(&ola, &[], wait, clock).expect("clock after restart");
    assert_eq!((money2, day2, place), (money, day, proto::place::BUILDING), "the same character, at work");
    let inv = wait_for(&ola, &[], Duration::from_secs(3), |p| match p {
        Packet::Inventory { slots } => Some(slots.iter().map(|s| s.kind).collect::<Vec<_>>()),
        _ => None,
    })
    .expect("inventory");
    assert!(inv.contains(&item_kind::EMPLOYEE_CARD) && !inv.contains(&item_kind::LAPTOP), "card kept, laptop still on the desk: {inv:?}");
    // Walk in from the entrance to her desk: the laptop logs in as Ola.
    let ws = game::computer::find_workstations(&b);
    let desk = ws.iter().find(|w| w.department == 1).unwrap();
    let s = b.spawns()[1 % b.spawns().len()]; // Ewa came in first, at the first spot
    let body = Body { access: access::CARD, ..Body::at(s.0, Pos::tile_center(s.1.x, s.1.y)) };
    let at = ola.walk_to(&b, body, (desk.floor, Tile { x: desk.tile.x, y: desk.tile.y + 1 }), &[]);
    ola.press_e(&b, at);
    assert_eq!(wait_for(&ola, &[], wait, computer), Some(ola.id), "her laptop, her account");
    ola.send(&Packet::TaskAction { token: ola.token, nonce: 0, action: ta::SYNC, task: 0, arg: 0, text: String::new() });
    let titles = wait_for(&ola, &[], wait, |p| match p {
        Packet::TaskBoard { tasks, .. } => Some(tasks.iter().map(|t| t.title.clone()).collect::<Vec<_>>()),
        _ => None,
    });
    assert_eq!(titles, Some(vec!["Przetrwać restart".to_string()]));
    let sync = Packet::MailAction {
        token: ola.token,
        nonce: 0,
        action: ma::SYNC,
        id: 0,
        to: String::new(),
        subject: String::new(),
        body: String::new(),
    };
    ola.send(&sync);
    let subject = wait_for(&ola, &[], wait, |p| match p {
        Packet::WorkMail { subject, .. } => Some(subject.clone()),
        _ => None,
    });
    assert_eq!(subject.as_deref(), Some("Notatka"));
    stop(server);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn nicks_are_unique_for_guests_too() {
    let (addr, _) = start_server();
    let (ola, _) = Client::connect(addr, "Ola");
    let sock = Client::socket_for(addr);
    let other = Client { sock, id: 0, token: 0, seq: 0, crypto: None };
    other.send(&Packet::Connect { nonce: 5, nick: "OLA".into(), profile: test_profile(), ticket: String::new() });
    let reason = wait_for(&other, &[&ola], Duration::from_millis(500), |p| match p {
        Packet::Reject { reason } => Some(*reason),
        _ => None,
    });
    assert_eq!(reason, Some(proto::reject::NICK_TAKEN), "someone plays as Ola already");
}
