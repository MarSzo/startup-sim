//! One floor of the building, loaded from `client/maps/floorN.json`.
//!
//! The same JSON files are read by the server and the client, so collision,
//! room zones and floor links are identical on both sides. See
//! `building.rs` for the multi-floor container.

use std::collections::{HashMap, VecDeque};

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
    /// Access requirement (e.g. "card", "service"). Not enforced yet: the
    /// tile's `solid` flag decides collision for now.
    #[serde(default)]
    access: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RoomDef {
    pub id: u16,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn contains(&self, tx: i32, ty: i32) -> bool {
        tx >= self.x && ty >= self.y && tx < self.x + self.w && ty < self.y + self.h
    }
}

/// Connection between floors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkKind {
    /// Entering the area moves you to `to_floor` at tile `to`.
    Stairs { to_floor: u8, to: Tile },
    /// Pressing "interact" inside the cabin moves you to the next active floor
    /// that has a cabin with the same `id` (same x/y on every floor).
    Elevator { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub area: Rect,
    pub kind: LinkKind,
}

#[derive(Debug, Deserialize)]
struct LinkFile {
    kind: String,
    area: [i32; 4],
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    to_floor: Option<u8>,
    #[serde(default)]
    to: Option<[i32; 2]>,
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
    links: Vec<LinkFile>,
    #[serde(default)]
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
    solid: Vec<bool>,
    tile_kind: Vec<u8>,
    access: Vec<Option<String>>,
    room: Vec<u16>,
    pub rooms: Vec<RoomDef>,
    pub links: Vec<Link>,
    pub spawns: Vec<Tile>,
}

impl Map {
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
        let mut access = Vec::with_capacity(w * h);
        let mut room = Vec::with_capacity(w * h);
        for (y, (trow, rrow)) in file.tiles.iter().zip(&file.rooms).enumerate() {
            if trow.len() != w || rrow.len() != w || !trow.is_ascii() || !rrow.is_ascii() {
                return Err(format!("row {y}: expected {w} ASCII chars"));
            }
            for (tc, rc) in trow.bytes().zip(rrow.bytes()) {
                let key = (tc as char).to_string();
                let entry = file.legend.get(&key).ok_or_else(|| format!("row {y}: tile '{key}' not in legend"))?;
                solid.push(entry.solid);
                tile_kind.push(tc);
                access.push(entry.access.clone());
                let rkey = (rc as char).to_string();
                room.push(match file.room_defs.get(&rkey) {
                    Some(def) => def.id,
                    None if rc == b'-' => NO_ROOM,
                    None => return Err(format!("row {y}: room '{rkey}' not in room_defs")),
                });
            }
        }
        let mut links = Vec::new();
        for l in &file.links {
            let area = Rect { x: l.area[0], y: l.area[1], w: l.area[2], h: l.area[3] };
            let kind = match l.kind.as_str() {
                "stairs" => {
                    let (Some(to_floor), Some(to)) = (l.to_floor, l.to) else {
                        return Err("stairs link needs to_floor and to".into());
                    };
                    LinkKind::Stairs { to_floor, to: Tile { x: to[0], y: to[1] } }
                }
                "elevator" => LinkKind::Elevator { id: l.id.clone().ok_or("elevator link needs id")? },
                other => return Err(format!("unknown link kind '{other}'")),
            };
            links.push(Link { area, kind });
        }
        let mut rooms: Vec<RoomDef> = file.room_defs.into_values().collect();
        rooms.sort_by_key(|r| r.id);
        let map = Map {
            id: file.id,
            floor: file.floor,
            width: w as i32,
            height: h as i32,
            solid,
            tile_kind,
            access,
            room,
            rooms,
            links,
            spawns: file.spawns.iter().map(|s| Tile { x: s[0], y: s[1] }).collect(),
        };
        for s in &map.spawns {
            if map.is_blocked(s.x, s.y) {
                return Err(format!("spawn {s:?} is blocked"));
            }
        }
        for l in &map.links {
            for ty in l.area.y..l.area.y + l.area.h {
                for tx in l.area.x..l.area.x + l.area.w {
                    if map.is_blocked(tx, ty) {
                        return Err(format!("link area {:?} contains blocked tile ({tx},{ty})", l.area));
                    }
                }
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
    /// Future: access cards will add a player context here (and in movement.gd).
    pub fn is_blocked(&self, tx: i32, ty: i32) -> bool {
        self.idx(tx, ty).map_or(true, |i| self.solid[i])
    }

    pub fn tile_char(&self, tx: i32, ty: i32) -> Option<char> {
        self.idx(tx, ty).map(|i| self.tile_kind[i] as char)
    }

    /// Access requirement of a tile ("card", "service"), if any.
    pub fn access(&self, tx: i32, ty: i32) -> Option<&str> {
        self.idx(tx, ty).and_then(|i| self.access[i].as_deref())
    }

    pub fn room_at_tile(&self, tx: i32, ty: i32) -> u16 {
        self.idx(tx, ty).map_or(NO_ROOM, |i| self.room[i])
    }

    /// Room at a position in sub-pixel units.
    pub fn room_at(&self, x: i32, y: i32) -> u16 {
        self.room_at_tile(x.div_euclid(TILE_UNITS), y.div_euclid(TILE_UNITS))
    }

    pub fn room_by_name(&self, name: &str) -> Option<&RoomDef> {
        self.rooms.iter().find(|r| r.name.to_lowercase() == name.to_lowercase())
    }

    pub fn room_name(&self, id: u16) -> &str {
        self.rooms.iter().find(|r| r.id == id).map_or("-", |r| r.name.as_str())
    }

    pub fn link_at(&self, tx: i32, ty: i32) -> Option<&Link> {
        self.links.iter().find(|l| l.area.contains(tx, ty))
    }

    /// All walkable tiles belonging to a room.
    pub fn room_tiles(&self, id: u16) -> Vec<Tile> {
        self.walkable_tiles().into_iter().filter(|t| self.room_at_tile(t.x, t.y) == id).collect()
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

    /// 4-connected BFS path on this floor from `from` to `to` (both inclusive).
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
            let ti = self.idx(t.x, t.y).unwrap();
            if ti == goal {
                let w = self.width as usize;
                let mut path = vec![to];
                let mut cur = goal;
                while cur != start {
                    cur = prev[cur];
                    path.push(Tile { x: (cur % w) as i32, y: (cur / w) as i32 });
                }
                path.reverse();
                return Some(path);
            }
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

#[cfg(test)]
mod tests {
    use crate::building::{default_building_path, Building};

    fn b() -> Building {
        Building::load(&default_building_path()).expect("building loads")
    }

    #[test]
    fn ground_floor_has_gdd_rooms() {
        let b = b();
        let m = b.floor(0).unwrap();
        assert_eq!((m.width, m.height), (60, 48));
        for name in [
            "Na zewnątrz",
            "Parking zewnętrzny",
            "Strefa palenia",
            "Wejście",
            "Portiernia",
            "Parking wewnętrzny",
            "Sklep",
            "Hol",
            "Winda",
            "Klatka schodowa",
            "Zaplecze techniczne",
        ] {
            assert!(m.room_by_name(name).is_some(), "floor 0 missing {name}");
        }
        assert!(!m.spawns.is_empty());
        let outside = m.room_by_name("Na zewnątrz").unwrap().id;
        for s in &m.spawns {
            assert_eq!(m.room_at_tile(s.x, s.y), outside, "spawn is outside");
        }
    }

    #[test]
    fn first_floor_has_gdd_rooms() {
        let b = b();
        let m = b.floor(1).unwrap();
        for name in [
            "Recepcja",
            "IT / Produkt",
            "Biznes",
            "Zarząd",
            "HR",
            "Korytarz",
            "Chill room",
            "Łazienka damska",
            "Łazienka męska",
            "Winda",
            "Klatka schodowa",
        ] {
            assert!(m.room_by_name(name).is_some(), "floor 1 missing {name}");
        }
        assert!(m.spawns.is_empty());
    }

    #[test]
    fn walls_and_out_of_bounds_block() {
        let b = b();
        let m = b.floor(0).unwrap();
        assert!(m.is_blocked(0, 0), "fence");
        assert!(m.is_blocked(-1, 5));
        assert!(m.is_blocked(60, 5));
        assert!(!m.is_blocked(1, 1), "grass");
        assert!(b.floor(1).unwrap().is_blocked(1, 1), "void around floor 1");
    }

    #[test]
    fn gates_open_but_marked_and_service_door_locked() {
        let b = b();
        let m = b.floor(0).unwrap();
        assert_eq!(m.tile_char(28, 21), Some('B'));
        assert!(!m.is_blocked(28, 21), "card gates are open for now");
        assert_eq!(m.access(28, 21), Some("card"));
        assert_eq!(m.tile_char(43, 13), Some('L'));
        assert!(m.is_blocked(43, 13), "service room is locked");
        assert_eq!(m.access(43, 13), Some("service"));
    }

    #[test]
    fn every_room_reachable_from_spawn_except_locked() {
        let b = b();
        let spawn = (0, b.floor(0).unwrap().spawns[0]);
        for f in [0u8, 1] {
            let m = b.floor(f).unwrap();
            for r in &m.rooms {
                let target = m.room_tiles(r.id)[0];
                let reachable = b.find_path(spawn, (f, target)).is_some();
                assert_eq!(reachable, r.kind != "service", "floor {f} room {}", r.name);
            }
        }
    }

    #[test]
    fn floors_share_elevator_and_stairs_geometry() {
        let b = b();
        let (a, c) = (b.floor(0).unwrap(), b.floor(1).unwrap());
        assert_eq!(a.links.len(), 2);
        for (la, lc) in a.links.iter().zip(&c.links) {
            assert_eq!(la.area, lc.area);
        }
    }

    #[test]
    fn rejects_malformed_maps() {
        use super::Map;
        assert!(Map::from_bytes(b"{}").is_err());
        let bad = br##"{"version":1,"id":"x","floor":0,"tile_px":16,"width":2,"height":1,
            "legend":{"#":{"type":"wall","solid":true}},"tiles":["#"],"rooms":["--"],
            "room_defs":{},"spawns":[]}"##;
        assert!(Map::from_bytes(bad).is_err());
        let blocked_link = br##"{"version":1,"id":"x","floor":0,"tile_px":16,"width":1,"height":1,
            "legend":{"#":{"type":"wall","solid":true}},"tiles":["#"],"rooms":["-"],
            "room_defs":{},"links":[{"kind":"elevator","id":"m","area":[0,0,1,1]}]}"##;
        assert!(Map::from_bytes(blocked_link).is_err());
    }
}
