//! End-to-end: real server on a random port, raw UDP test clients.

use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use game::map::{default_map_path, Map};
use game::net::LinkConditions;
use game::protocol::{self as proto, Packet};
use game::server::{Config, Server};
use game::sim::{IN_RIGHT, IN_UP};

fn start_server() -> (SocketAddr, u32) {
    let map = Map::load(&default_map_path()).unwrap();
    let crc = map.crc;
    let cfg = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        link: LinkConditions::default(),
        max_players: 16,
        stats_every: Duration::from_secs(3600),
        client_timeout: Duration::from_millis(600),
    };
    let mut server = Server::new(map, cfg).unwrap();
    let addr = server.local_addr();
    std::thread::spawn(move || server.run());
    (addr, crc)
}

struct Client {
    sock: UdpSocket,
    id: u16,
    token: u32,
    seq: u32,
}

impl Client {
    fn connect(server: SocketAddr, nick: &str) -> (Client, u32) {
        let sock = UdpSocket::bind("127.0.0.1:0").unwrap();
        sock.connect(server).unwrap();
        sock.set_read_timeout(Some(Duration::from_millis(20))).unwrap();
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

    fn ping(&self) {
        self.sock.send(&Packet::Ping { token: self.token, client_time: 1 }.encode()).unwrap();
    }
}

#[test]
fn handshake_interest_and_timeout() {
    let (addr, crc) = start_server();
    let map = Map::load(&default_map_path()).unwrap();
    let lobby = map.room_by_name("Wejście").unwrap().id;

    let (a, a_crc) = Client::connect(addr, "Ala");
    let (mut b, _) = Client::connect(addr, "Bob");
    assert_eq!(a_crc, crc);
    assert_ne!(a.id, b.id);

    // Both spawn in the lobby and see each other; A learns B's nick.
    let mut got_info = false;
    let mut saw_b = false;
    let deadline = Instant::now() + Duration::from_millis(500);
    while Instant::now() < deadline {
        a.ping();
        b.ping();
        match a.recv() {
            Some(Packet::PlayerInfo { players }) => got_info |= players.iter().any(|p| p.id == b.id && p.nick == "Bob"),
            Some(Packet::Snapshot { room, entities, .. }) => {
                assert_eq!(room, lobby);
                saw_b |= entities.iter().any(|e| e.id == b.id);
            }
            _ => {}
        }
    }
    assert!(saw_b && got_info, "A should see B and get its nick");

    // B walks right one tile, then up through the corridor door and out of the lobby.
    for _ in 0..2 {
        b.send_inputs(IN_RIGHT, 6);
        a.ping();
        std::thread::sleep(Duration::from_millis(50));
    }
    for _ in 0..40 {
        b.send_inputs(IN_UP, 6);
        a.ping();
        std::thread::sleep(Duration::from_millis(50));
    }
    a.ping();
    let (_, b_room, _, _, b_ack) = b.latest_snapshot(Duration::from_millis(300)).unwrap();
    a.ping();
    assert_eq!(b_ack, b.seq, "server processed all inputs");
    assert_ne!(b_room, lobby, "B left the lobby");
    let (_, a_room, _, a_visible, _) = a.latest_snapshot(Duration::from_millis(200)).unwrap();
    assert_eq!(a_room, lobby);
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
