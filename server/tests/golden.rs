//! Golden files shared with the Godot client tests (`client/tests/run_tests.gd`).
//! Regenerate with `UPDATE_GOLDEN=1 cargo test --test golden`.

#![allow(clippy::unwrap_used)] // test / dev tool: a panic is the right report

use std::path::PathBuf;

use game::building::{default_building_path, Building};
use game::map::access;
use game::nav::Walker;
use game::protocol::{golden_samples, to_hex};
use game::sim::{self, Body, Pos};
use serde_json::{json, Value};

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn check(name: &str, value: Value, pretty: bool) {
    let path = golden_dir().join(name);
    let text = if pretty { serde_json::to_string_pretty(&value) } else { serde_json::to_string(&value) }.unwrap() + "\n";
    if std::env::var("UPDATE_GOLDEN").is_ok() || !path.exists() {
        std::fs::write(&path, &text).unwrap();
        return;
    }
    let on_disk = std::fs::read_to_string(&path).unwrap();
    assert!(on_disk == text, "{name} differs from generated output; run UPDATE_GOLDEN=1 cargo test --test golden");
}

#[test]
fn packets() {
    let items: Vec<Value> = golden_samples()
        .into_iter()
        .map(|(name, p)| json!({ "name": name, "hex": to_hex(&p.encode()), "debug": format!("{p:?}") }))
        .collect();
    check("packets.json", json!({ "packets": items }), true);
}

/// Sealed packets: the Godot client must produce and read the same bytes.
#[test]
fn sealed() {
    use game::crypto::{connect_prefix, session_prefix, Dir, Keys};
    let key = [0x42u8; 32];
    let keys = Keys::derive(&key);
    let inner = golden_samples().into_iter().find(|(n, _)| *n == "ping").unwrap().1.encode();
    let to_server = keys.seal(Dir::ToServer, &session_prefix(0x01020304), 7, &inner);
    let to_client = keys.seal(Dir::ToClient, &session_prefix(0x01020304), 9, &inner);
    let ticket = [0xabu8; 32];
    let connect = keys.seal(Dir::ToServer, &connect_prefix(&ticket), 1, &inner);
    check(
        "sealed.json",
        json!({
            "key": to_hex(&key),
            "token": 0x01020304u32,
            "inner": to_hex(&inner),
            "to_server_counter": 7,
            "to_server": to_hex(&to_server),
            "to_client_counter": 9,
            "to_client": to_hex(&to_client),
            "ticket": to_hex(&ticket),
            "connect": to_hex(&connect),
        }),
        true,
    );
}

#[test]
fn movement_vectors() {
    let b = Building::load(&default_building_path()).unwrap();
    let mut rng = fastrand::Rng::with_seed(42);
    let mut cases = Vec::new();
    let body_json = |x: &Body| json!([x.floor, x.pos.x, x.pos.y, x.prev_input, x.lock, x.access, x.slow as u8, x.drunk]);

    // Random walks from interesting spots (walls, furniture, doors, card doors).
    let guest = |b: Body| Body { access: access::GUEST, ..b };
    let starts = [
        Body::at(0, Pos::tile_center(30, 59)),                        // spawn, sidewalk
        Body::at(0, Pos::tile_center(28, 42)),                        // the stairwell's card door, no pass
        Body::at(0, Pos::tile_center(37, 45)),                        // at the lifts without a pass
        Body::at(0, Pos::tile_center(35, 11)),                        // garage gate, no card
        Body::at(0, Pos::tile_center(31, 55)),                        // the draught lobby
        guest(Body::at(0, Pos::tile_center(28, 42))),                 // the stairwell's door with a pass
        Body::at(0, Pos::tile_center(22, 47)),                        // shop, shelves
        Body::at(0, Pos { x: 31 * 256 + 3, y: 53 * 256 - 1 }),        // odd offsets in a door
        Body::at(0, Pos::tile_center(23, 18)),                        // parking between cars
        Body::at(0, Pos::tile_center(24, 42)),                        // stairwell by the flight
        Body::at(0, Pos::tile_center(37, 41)),                        // elevator cabin
        Body::at(4, Pos::tile_center(25, 41)),                        // stairs arrival upstairs
        Body::at(4, Pos::tile_center(32, 20)),                        // corridor upstairs
        Body { slow: true, ..Body::at(4, Pos::tile_center(34, 10)) }, // exhausted, chill room
        Body { slow: true, ..Body::at(0, Pos::tile_center(24, 42)) }, // slow on the stairs
        Body { drunk: 1, ..Body::at(4, Pos::tile_center(32, 20)) },   // tipsy, corridor
        Body { drunk: 2, ..Body::at(4, Pos::tile_center(32, 20)) },   // drunk, corridor
        Body { drunk: 2, ..Body::at(0, Pos::tile_center(22, 47)) },   // drunk among shelves
    ];
    for start in starts {
        let mut body = start;
        let (mut inputs, mut states) = (Vec::new(), Vec::new());
        let mut held = 0u8;
        for _ in 0..400 {
            if rng.u8(0..10) == 0 {
                held = rng.u8(0..32); // includes the interact bit
            }
            body = sim::step(&b, body, held);
            inputs.push(held);
            states.push(body_json(&body));
        }
        cases.push(json!({ "start": body_json(&start), "inputs": inputs, "states": states }));
    }

    // Scripted (with a guest pass): spawn -> hall -> stairs up -> chill room -> back down.
    let mut body = guest(Body::at(0, Pos::tile_center(30, 59)));
    let start = body;
    let (mut inputs, mut states) = (Vec::new(), Vec::new());
    let mut record = |body: &mut Body, input: u8| {
        *body = sim::step(&b, *body, input);
        inputs.push(input);
        states.push(body_json(body));
    };
    let chill = b.find_room("Chill room").unwrap().1.id;
    let goal = b.floor(4).unwrap().room_tiles(chill)[20];
    let mut w = Walker::to(&b, &body, (4, goal)).unwrap();
    while !w.done() {
        let i = w.next_input(&body);
        record(&mut body, i);
    }
    // Back down the stairs (through the stairwell) to the lobby.
    let lobby = b.find_room("Hol").unwrap().1.id;
    let goal = b.floor(0).unwrap().room_tiles(lobby)[10];
    let mut w = Walker::to(&b, &body, (0, goal)).unwrap();
    while !w.done() {
        let i = w.next_input(&body);
        record(&mut body, i);
    }
    assert_eq!(body.floor, 0, "down through the stairwell");
    cases.push(json!({ "start": body_json(&start), "inputs": inputs, "states": states }));

    check("movement_vectors.json", json!({ "building_crc": b.crc, "cases": cases }), false);
}
