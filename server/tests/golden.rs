//! Golden files shared with the Godot client tests (`client/tests/run_tests.gd`).
//! Regenerate with `UPDATE_GOLDEN=1 cargo test --test golden`.

use std::path::PathBuf;

use game::map::{default_map_path, Map};
use game::protocol::{golden_samples, to_hex};
use game::sim::{self, Pos};
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
    let map = Map::load(&default_map_path()).unwrap();
    let mut rng = fastrand::Rng::with_seed(42);
    let starts = [
        Pos::tile_center(40, 36),                  // lobby spawn
        Pos::tile_center(50, 27),                  // lobby corner
        Pos::tile_center(21, 21),                  // corridor next to parking door
        Pos::tile_center(24, 2),                   // open space above desks
        Pos::tile_center(52, 31),                  // shop doorway
        Pos { x: 19 * 256 + 3, y: 22 * 256 - 1 },  // odd offsets inside a door
        Pos::tile_center(4, 5),                    // parking between cars
        Pos::tile_center(30, 33),                  // reception
    ];
    let mut cases = Vec::new();
    for start in starts {
        let mut p = start;
        let mut inputs = Vec::new();
        let mut positions = Vec::new();
        let mut held = 0u8;
        for _ in 0..400 {
            if rng.u8(0..10) == 0 {
                held = rng.u8(0..16);
            }
            p = sim::step(&map, p, held);
            inputs.push(held);
            positions.push([p.x, p.y]);
        }
        cases.push(json!({ "start": [start.x, start.y], "inputs": inputs, "positions": positions }));
    }
    check("movement_vectors.json", json!({ "map": "floor0", "map_crc": map.crc, "cases": cases }), false);
}
