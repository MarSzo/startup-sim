//! Golden files shared with the Godot client tests (`client/tests/run_tests.gd`).
//! Regenerate with `UPDATE_GOLDEN=1 cargo test --test golden`.

use std::path::PathBuf;

use game::building::{default_building_path, Building};
use game::map::Tile;
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

#[test]
fn movement_vectors() {
    let b = Building::load(&default_building_path()).unwrap();
    let mut rng = fastrand::Rng::with_seed(42);
    let mut cases = Vec::new();
    let body_json = |x: &Body| json!([x.floor, x.pos.x, x.pos.y, x.prev_input, x.lock]);

    // Random walks from interesting spots (walls, furniture, doors, gates).
    let starts = [
        Body::at(0, Pos::tile_center(33, 35)),                     // spawn, sidewalk
        Body::at(0, Pos::tile_center(33, 27)),                     // lobby
        Body::at(0, Pos::tile_center(34, 23)),                     // below the card gates
        Body::at(0, Pos::tile_center(45, 23)),                     // shop, shelves
        Body::at(0, Pos { x: 41 * 256 + 3, y: 30 * 256 - 1 }),     // odd offsets in a door
        Body::at(0, Pos::tile_center(6, 8)),                       // parking between cars
        Body::at(0, Pos::tile_center(34, 11)),                     // stairwell below the flight
        Body::at(0, Pos::tile_center(26, 10)),                     // elevator cabin
        Body::at(1, Pos::tile_center(34, 10)),                     // stairs arrival upstairs
        Body::at(1, Pos::tile_center(10, 22)),                     // corridor upstairs
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

    // Scripted: spawn -> stairs up -> chill room -> elevator down.
    let mut body = Body::at(0, Pos::tile_center(33, 35));
    let start = body;
    let (mut inputs, mut states) = (Vec::new(), Vec::new());
    let mut record = |body: &mut Body, input: u8| {
        *body = sim::step(&b, *body, input);
        inputs.push(input);
        states.push(body_json(body));
    };
    let chill = b.find_room("Chill room").unwrap().1.id;
    let goal = b.floor(1).unwrap().room_tiles(chill)[20];
    let mut w = Walker::to(&b, &body, (1, goal)).unwrap();
    while !w.done() {
        let i = w.next_input(&body);
        record(&mut body, i);
    }
    let mut w = Walker::to(&b, &body, (1, Tile { x: 26, y: 10 })).unwrap();
    while !w.done() {
        let i = w.next_input(&body);
        record(&mut body, i);
    }
    for input in [0, sim::IN_INTERACT, sim::IN_INTERACT, 0, sim::IN_INTERACT, 0] {
        record(&mut body, input);
    }
    assert_eq!(body.floor, 1, "rode down and back up");
    cases.push(json!({ "start": body_json(&start), "inputs": inputs, "states": states }));

    check("movement_vectors.json", json!({ "building_crc": b.crc, "cases": cases }), false);
}
