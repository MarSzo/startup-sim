//! One floor of the building, loaded from `client/maps/floorN.json`.
//!
//! The same JSON files are read by the server and the client, so collision,
//! room zones and floor links are identical on both sides. See
//! `building.rs` for the multi-floor container.

use std::collections::HashMap;

use serde::Deserialize;

use crate::sim::TILE_UNITS;

/// Room id used for tiles that belong to no room (walls).
pub const NO_ROOM: u16 = 0;

/// Access rights a character can hold (bitmask in `sim::Body::access`).
pub mod access {
    /// Visitor pass from the porter (trial day), valid for the session.
    pub const GUEST: u8 = 1;
    /// Employee card (after the HR contract - not obtainable yet).
    pub const CARD: u8 = 2;
    /// Staff/service areas (technical room).
    pub const SERVICE: u8 = 4;

    /// Rights that satisfy a tile's `access` requirement from the legend.
    pub fn required(name: &str) -> Option<u8> {
        match name {
            "card" => Some(GUEST | CARD),
            "service" => Some(SERVICE),
            _ => None,
        }
    }
}

/// Movement directions (for one-way passages such as exiting the gates).
pub mod dir {
    pub const UP: u8 = 1;
    pub const DOWN: u8 = 2;
    pub const LEFT: u8 = 3;
    pub const RIGHT: u8 = 4;

    pub fn parse(name: &str) -> Option<u8> {
        match name {
            "up" => Some(UP),
            "down" => Some(DOWN),
            "left" => Some(LEFT),
            "right" => Some(RIGHT),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct LegendEntry {
    #[serde(rename = "type")]
    kind: String,
    solid: bool,
    /// Access requirement ("card", "service"): blocks characters without it.
    #[serde(default)]
    access: Option<String>,
    /// Direction in which the tile can always be passed (e.g. leaving
    /// through the gates without a card).
    #[serde(default)]
    free_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NpcFile {
    kind: String,
    name: String,
    home: [i32; 2],
    #[serde(default)]
    escort_to: Option<[i32; 3]>,
}

/// NPC placed on this floor (server-side characters).
#[derive(Debug, Clone)]
pub struct NpcDef {
    pub kind: String,
    pub name: String,
    pub home: Tile,
    /// Where the porter escorts newcomers: (floor, tile).
    pub escort_to: Option<(u8, Tile)>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RoomDef {
    pub id: u16,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    /// Keys of rooms whose people are also visible from here (e.g. the
    /// porter's lodge seen through its open door). Interest management only.
    #[serde(default)]
    pub see: Vec<String>,
    /// Bathrooms: "female" / "male".
    #[serde(default)]
    pub gender: Option<String>,
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
    #[serde(default)]
    npcs: Vec<NpcFile>,
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
    /// Rights that open the tile (0 = no requirement).
    need: Vec<u8>,
    free_dir: Vec<u8>,
    /// Tile char -> legend type ("desk", "coffee_machine", ...).
    types: HashMap<u8, String>,
    room: Vec<u16>,
    pub rooms: Vec<RoomDef>,
    pub links: Vec<Link>,
    pub spawns: Vec<Tile>,
    pub npcs: Vec<NpcDef>,
    /// Room id -> other room ids visible from it (from `RoomDef::see`).
    see: HashMap<u16, Vec<u16>>,
    /// Doors locked right now (toilet stalls): solid for everyone. Changed
    /// by the server; clients learn it from the `Doors` packet.
    closed: Vec<bool>,
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
        let mut need = Vec::with_capacity(w * h);
        let mut free_dir = Vec::with_capacity(w * h);
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
                need.push(match &entry.access {
                    Some(a) => access::required(a).ok_or_else(|| format!("tile '{key}': unknown access '{a}'"))?,
                    None => 0,
                });
                free_dir.push(match &entry.free_dir {
                    Some(d) => dir::parse(d).ok_or_else(|| format!("tile '{key}': unknown free_dir '{d}'"))?,
                    None => 0,
                });
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
        let mut see = HashMap::new();
        for def in file.room_defs.values() {
            let ids: Result<Vec<u16>, String> = def
                .see
                .iter()
                .map(|k| file.room_defs.get(k).map(|d| d.id).ok_or(format!("room '{}': unknown see '{k}'", def.name)))
                .collect();
            see.insert(def.id, ids?);
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
            need,
            free_dir,
            types: file.legend.iter().filter_map(|(k, v)| k.bytes().next().map(|c| (c, v.kind.clone()))).collect(),
            room,
            rooms,
            links,
            spawns: file.spawns.iter().map(|s| Tile { x: s[0], y: s[1] }).collect(),
            npcs: file
                .npcs
                .iter()
                .map(|n| NpcDef {
                    kind: n.kind.clone(),
                    name: n.name.clone(),
                    home: Tile { x: n.home[0], y: n.home[1] },
                    escort_to: n.escort_to.map(|e| (e[0] as u8, Tile { x: e[1], y: e[2] })),
                })
                .collect(),
            see,
            closed: vec![false; w * h],
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

    /// Whether a tile is solid (walls, furniture). Out-of-map tiles are solid.
    /// Ignores access rules - see `blocks` for movement.
    pub fn is_blocked(&self, tx: i32, ty: i32) -> bool {
        self.idx(tx, ty).map_or(true, |i| self.solid[i])
    }

    /// Whether a character with rights `access`, moving in direction `d`
    /// (`dir::*`), is stopped by this tile. The only collision rule used by
    /// the simulation (mirrored in movement.gd).
    pub fn blocks(&self, tx: i32, ty: i32, access: u8, d: u8) -> bool {
        match self.idx(tx, ty) {
            None => true,
            Some(i) => {
                self.solid[i]
                    || self.closed[i]
                    || (self.need[i] != 0 && access & self.need[i] == 0 && self.free_dir[i] != d)
            }
        }
    }

    /// Lock / unlock a door (see `closed`).
    pub fn set_closed(&mut self, tx: i32, ty: i32, on: bool) {
        if let Some(i) = self.idx(tx, ty) {
            self.closed[i] = on;
        }
    }

    /// All closed doors (x, y).
    pub fn closed_tiles(&self) -> Vec<(i32, i32)> {
        self.closed.iter().enumerate().filter(|(_, c)| **c).map(|(i, _)| (i as i32 % self.width, i as i32 / self.width)).collect()
    }

    pub fn is_closed(&self, tx: i32, ty: i32) -> bool {
        self.idx(tx, ty).is_some_and(|i| self.closed[i])
    }

    pub fn tile_char(&self, tx: i32, ty: i32) -> Option<char> {
        self.idx(tx, ty).map(|i| self.tile_kind[i] as char)
    }

    /// Legend type of a tile ("desk", "coffee_machine", ...).
    pub fn tile_type(&self, tx: i32, ty: i32) -> Option<&str> {
        self.idx(tx, ty).and_then(|i| self.types.get(&self.tile_kind[i])).map(|s| s.as_str())
    }

    /// Rights that open a tile (0 = none needed).
    pub fn need(&self, tx: i32, ty: i32) -> u8 {
        self.idx(tx, ty).map_or(0, |i| self.need[i])
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

    /// Other rooms whose people are visible from `room`.
    pub fn visible_from(&self, room: u16) -> &[u16] {
        self.see.get(&room).map_or(&[], |v| v.as_slice())
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
    fn gates_need_a_pass_except_on_the_way_out() {
        use super::{access, dir};
        let b = b();
        let m = b.floor(0).unwrap();
        assert_eq!(m.tile_char(28, 21), Some('B'));
        assert!(m.blocks(28, 21, 0, dir::UP), "no pass: can't enter");
        assert!(!m.blocks(28, 21, 0, dir::DOWN), "no pass: can always leave");
        assert!(!m.blocks(28, 21, access::GUEST, dir::UP), "guest pass opens");
        assert!(!m.blocks(28, 21, access::CARD, dir::UP), "employee card opens");
        assert_eq!(m.tile_char(43, 13), Some('L'));
        assert!(m.blocks(43, 13, access::GUEST | access::CARD, dir::UP), "service room stays closed");
        assert!(!m.blocks(43, 13, access::SERVICE, dir::UP));
        assert!(m.blocks(0, 0, 0xff, dir::UP), "walls block everyone");
    }

    #[test]
    fn reachability_depends_on_access() {
        use super::access;
        let b = b();
        let spawn = (0, b.floor(0).unwrap().spawns[0]);
        let public = ["outside", "parking", "entrance", "reception", "shop", "smoking"];
        for f in [0u8, 1] {
            let m = b.floor(f).unwrap();
            for r in &m.rooms {
                let tile = m.room_tiles(r.id).into_iter().find(|t| m.need(t.x, t.y) == 0).unwrap();
                let target = (f, tile);
                let guest = b.find_path(spawn, target, access::GUEST).is_some();
                let nobody = b.find_path(spawn, target, 0).is_some();
                let staff = b.find_path(spawn, target, access::CARD | access::SERVICE).is_some();
                assert!(staff, "floor {f} {} unreachable even for staff", r.name);
                assert_eq!(guest, r.kind != "service", "floor {f} {} with a guest pass", r.name);
                let is_public = f == 0 && public.contains(&r.kind.as_str()) && r.name != "Parking wewnętrzny";
                assert_eq!(nobody, is_public, "floor {f} {} without any pass", r.name);
            }
        }
    }

    #[test]
    fn lodge_and_lobby_see_each_other() {
        let b = b();
        let m = b.floor(0).unwrap();
        let lobby = m.room_by_name("Wejście").unwrap().id;
        let lodge = m.room_by_name("Portiernia").unwrap().id;
        assert_eq!(m.visible_from(lobby), &[lodge]);
        assert_eq!(m.visible_from(lodge), &[lobby]);
        assert!(m.visible_from(m.room_by_name("Hol").unwrap().id).is_empty());
    }

    #[test]
    fn porter_is_placed_in_the_lodge() {
        let b = b();
        let m = b.floor(0).unwrap();
        assert_eq!(m.npcs.len(), 1);
        let p = &m.npcs[0];
        assert_eq!((p.kind.as_str(), p.name.as_str()), ("porter", "Portier"));
        assert_eq!(m.room_name(m.room_at_tile(p.home.x, p.home.y)), "Portiernia");
        let (f, t) = p.escort_to.unwrap();
        let m1 = b.floor(f).unwrap();
        assert_eq!(m1.room_name(m1.room_at_tile(t.x, t.y)), "Recepcja");
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
