//! Tile map shared with the Godot client (`client/maps/*.json`).
//!
//! The same JSON file is read by the server and the client, so collision and
//! room zones are identical on both sides. The server sends the CRC32 of the
//! raw file in `Welcome`; the client refuses to play on a mismatch.

use std::collections::{HashMap, VecDeque};
use std::path::Path;

use serde::Deserialize;

use crate::sim::TILE_UNITS;

/// Room id used for tiles that belong to no room (walls).
pub const NO_ROOM: u16 = 0;

#[derive(Debug, Deserialize)]
struct LegendEntry {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    kind: String,
    solid: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RoomDef {
    pub id: u16,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
}

/// A door tile. `access` is reserved for access cards (not implemented yet).
#[derive(Debug, Deserialize, Clone)]
pub struct Door {
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub access: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MapFile {
    version: u32,
    id: String,
    floor: u8,
    tile_px: u32,
    width: usize,
    height: usize,
    legend: HashMap<String, LegendEntry>,
    tiles: Vec<String>,
    rooms: Vec<String>,
    room_defs: HashMap<String, RoomDef>,
    #[serde(default)]
    doors: Vec<Door>,
    spawns: Vec<[i32; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tile {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug)]
pub struct Map {
    pub id: String,
    pub floor: u8,
    pub width: i32,
    pub height: i32,
    /// CRC32 of the raw map file bytes.
    pub crc: u32,
    solid: Vec<bool>,
    tile_kind: Vec<u8>,
    room: Vec<u16>,
    pub rooms: Vec<RoomDef>,
    pub doors: Vec<Door>,
    pub spawns: Vec<Tile>,
}

impl Map {
    pub fn load(path: &Path) -> Result<Map, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Map::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Map, String> {
        let file: MapFile = serde_json::from_slice(bytes).map_err(|e| format!("map json: {e}"))?;
        if file.version != 1 {
            return Err(format!("unsupported map version {}", file.version));
        }
        if file.tile_px != 16 {
            return Err("tile_px must be 16".into());
        }
        let (w, h) = (file.width, file.height);
        if file.tiles.len() != h || file.rooms.len() != h {
            return Err("tiles/rooms row count != height".into());
        }
        let mut solid = Vec::with_capacity(w * h);
        let mut tile_kind = Vec::with_capacity(w * h);
        let mut room = Vec::with_capacity(w * h);
        for (y, (trow, rrow)) in file.tiles.iter().zip(&file.rooms).enumerate() {
            if trow.len() != w || rrow.len() != w || !trow.is_ascii() || !rrow.is_ascii() {
                return Err(format!("row {y}: expected {w} ASCII chars"));
            }
            for (tc, rc) in trow.bytes().zip(rrow.bytes()) {
                let key = (tc as char).to_string();
                let entry = file
                    .legend
                    .get(&key)
                    .ok_or_else(|| format!("row {y}: tile '{key}' not in legend"))?;
                solid.push(entry.solid);
                tile_kind.push(tc);
                let rkey = (rc as char).to_string();
                room.push(match file.room_defs.get(&rkey) {
                    Some(def) => def.id,
                    None if rc == b'-' => NO_ROOM,
                    None => return Err(format!("row {y}: room '{rkey}' not in room_defs")),
                });
            }
        }
        let mut rooms: Vec<RoomDef> = file.room_defs.into_values().collect();
        rooms.sort_by_key(|r| r.id);
        let map = Map {
            id: file.id,
            floor: file.floor,
            width: w as i32,
            height: h as i32,
            crc: crc32fast::hash(bytes),
            solid,
            tile_kind,
            room,
            rooms,
            doors: file.doors,
            spawns: file.spawns.iter().map(|s| Tile { x: s[0], y: s[1] }).collect(),
        };
        if map.spawns.is_empty() {
            return Err("map has no spawns".into());
        }
        for s in &map.spawns {
            if map.is_blocked(s.x, s.y) {
                return Err(format!("spawn {s:?} is blocked"));
            }
        }
        Ok(map)
    }

    fn idx(&self, tx: i32, ty: i32) -> Option<usize> {
        if tx < 0 || ty < 0 || tx >= self.width || ty >= self.height {
            None
        } else {
            Some((ty * self.width + tx) as usize)
        }
    }

    /// Whether a tile blocks movement. Out-of-map tiles are blocked.
    /// Future: locked doors / access cards will take a context argument here.
    pub fn is_blocked(&self, tx: i32, ty: i32) -> bool {
        self.idx(tx, ty).map_or(true, |i| self.solid[i])
    }

    pub fn tile_char(&self, tx: i32, ty: i32) -> Option<char> {
        self.idx(tx, ty).map(|i| self.tile_kind[i] as char)
    }

    pub fn room_at_tile(&self, tx: i32, ty: i32) -> u16 {
        self.idx(tx, ty).map_or(NO_ROOM, |i| self.room[i])
    }

    /// Room at a position in sub-pixel units.
    pub fn room_at(&self, x: i32, y: i32) -> u16 {
        self.room_at_tile(x.div_euclid(TILE_UNITS), y.div_euclid(TILE_UNITS))
    }

    pub fn room_by_name(&self, name: &str) -> Option<&RoomDef> {
        self.rooms.iter().find(|r| r.name.eq_ignore_ascii_case(name))
    }

    pub fn room_name(&self, id: u16) -> &str {
        self.rooms.iter().find(|r| r.id == id).map_or("-", |r| r.name.as_str())
    }

    /// All walkable tiles belonging to a room.
    pub fn room_tiles(&self, id: u16) -> Vec<Tile> {
        let mut out = Vec::new();
        for ty in 0..self.height {
            for tx in 0..self.width {
                if self.room_at_tile(tx, ty) == id && !self.is_blocked(tx, ty) {
                    out.push(Tile { x: tx, y: ty });
                }
            }
        }
        out
    }

    pub fn walkable_tiles(&self) -> Vec<Tile> {
        let mut out = Vec::new();
        for ty in 0..self.height {
            for tx in 0..self.width {
                if !self.is_blocked(tx, ty) {
                    out.push(Tile { x: tx, y: ty });
                }
            }
        }
        out
    }

    /// 4-connected BFS path from `from` to `to` (both inclusive).
    /// Used by bots now and by NPCs later.
    pub fn find_path(&self, from: Tile, to: Tile) -> Option<Vec<Tile>> {
        if self.is_blocked(to.x, to.y) || self.is_blocked(from.x, from.y) {
            return None;
        }
        let n = (self.width * self.height) as usize;
        let mut prev = vec![usize::MAX; n];
        let start = self.idx(from.x, from.y)?;
        let goal = self.idx(to.x, to.y)?;
        prev[start] = start;
        let mut queue = VecDeque::from([from]);
        while let Some(t) = queue.pop_front() {
            if self.idx(t.x, t.y) == Some(goal) {
                let mut path = vec![to];
                let mut cur = goal;
                while cur != start {
                    cur = prev[cur];
                    let w = self.width as usize;
                    path.push(Tile { x: (cur % w) as i32, y: (cur / w) as i32 });
                }
                path.reverse();
                return Some(path);
            }
            let ti = self.idx(t.x, t.y).unwrap();
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (t.x + dx, t.y + dy);
                if let Some(ni) = self.idx(nx, ny) {
                    if prev[ni] == usize::MAX && !self.solid[ni] {
                        prev[ni] = ti;
                        queue.push_back(Tile { x: nx, y: ny });
                    }
                }
            }
        }
        None
    }
}

/// Default location of the shared map file, relative to the server crate.
pub fn default_map_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../client/maps/floor0.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn floor0() -> Map {
        Map::load(&default_map_path()).expect("floor0 loads")
    }

    #[test]
    fn loads_shared_map() {
        let m = floor0();
        assert_eq!(m.width, 64);
        assert_eq!(m.height, 40);
        assert_eq!(m.floor, 0);
        for name in ["Wejście", "Portiernia", "Korytarz", "Sklep", "Parking wewnętrzny", "Open space"] {
            assert!(m.room_by_name(name).is_some(), "missing room {name}");
        }
    }

    #[test]
    fn walls_and_out_of_bounds_block() {
        let m = floor0();
        assert!(m.is_blocked(0, 0));
        assert!(m.is_blocked(-1, 5));
        assert!(m.is_blocked(64, 5));
        assert!(!m.is_blocked(1, 1));
    }

    #[test]
    fn doors_are_walkable_and_belong_to_public_side() {
        let m = floor0();
        let corridor = m.room_by_name("Korytarz").unwrap().id;
        assert_eq!(m.tile_char(19, 21), Some('D'));
        assert!(!m.is_blocked(19, 21));
        assert_eq!(m.room_at_tile(19, 21), corridor);
        assert!(!m.doors.is_empty());
    }

    #[test]
    fn every_room_reachable_from_spawn() {
        let m = floor0();
        let spawn = m.spawns[0];
        for r in &m.rooms {
            let target = m.room_tiles(r.id)[0];
            assert!(m.find_path(spawn, target).is_some(), "room {} unreachable", r.name);
        }
    }

    #[test]
    fn rejects_malformed_maps() {
        assert!(Map::from_bytes(b"{}").is_err());
        let bad = br##"{"version":1,"id":"x","floor":0,"tile_px":16,"width":2,"height":1,
            "legend":{"#":{"type":"wall","solid":true}},"tiles":["#"],"rooms":["--"],
            "room_defs":{},"spawns":[[0,0]]}"##;
        assert!(Map::from_bytes(bad).is_err());
    }
}
