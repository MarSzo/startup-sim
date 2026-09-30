//! The whole building: all floors listed in `client/maps/building.json`.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::map::{dir, LinkKind, Map, NpcDef, RoomDef, Tile};
use crate::outside::Outside;
use crate::sim::Pos;

#[derive(Debug, Deserialize)]
struct FloorEntry {
    floor: u8,
    file: Option<String>,
    name: String,
    #[serde(default)]
    locked: bool,
}

#[derive(Debug, Deserialize)]
struct BuildingFile {
    version: u32,
    floors: Vec<FloorEntry>,
}

#[derive(Debug)]
pub struct Floor {
    pub name: String,
    /// Locked floors exist in the design but can't be entered yet (floor 2).
    pub locked: bool,
    pub map: Option<Map>,
}

#[derive(Debug)]
pub struct Building {
    /// Indexed by floor number.
    pub floors: Vec<Floor>,
    /// CRC32 over building.json followed by every floor file, in floor order.
    /// Sent in `Welcome`; the client computes the same over its copies.
    pub crc: u32,
    /// Street, stops, parking (from the ground floor's places).
    pub outside: Outside,
    /// Where the treats tray stands; where a new founder appears.
    pub tray: Option<Place>,
    pub founder: Option<Place>,
}

/// A position in the building: (floor, tile).
pub type Place = (u8, Tile);

pub fn default_building_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../client/maps/building.json")
}

impl Building {
    pub fn load(path: &Path) -> Result<Building, String> {
        let dir = path.parent().unwrap_or(Path::new("."));
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let file: BuildingFile = serde_json::from_slice(&bytes).map_err(|e| format!("building json: {e}"))?;
        if file.version != 1 {
            return Err(format!("unsupported building version {}", file.version));
        }
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&bytes);
        let mut floors = Vec::new();
        for (i, f) in file.floors.iter().enumerate() {
            if f.floor as usize != i {
                return Err("floors must be listed in order 0, 1, 2, ...".into());
            }
            let map = match &f.file {
                Some(name) => {
                    let p = dir.join(name);
                    let mb = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                    hasher.update(&mb);
                    let m = Map::from_bytes(&mb).map_err(|e| format!("{name}: {e}"))?;
                    if m.floor != f.floor {
                        return Err(format!("{name}: floor {} != {}", m.floor, f.floor));
                    }
                    Some(m)
                }
                None => None,
            };
            floors.push(Floor { name: f.name.clone(), locked: f.locked, map });
        }
        let ground = floors.first().and_then(|f| f.map.as_ref()).ok_or("building has no ground floor")?;
        let outside = Outside::from_map(ground)?;
        let place = |pick: fn(&crate::map::Places) -> Option<[i32; 2]>| {
            floors.iter().enumerate().find_map(|(i, f)| f.map.as_ref().and_then(|m| pick(&m.places)).map(|[x, y]| (i as u8, Tile { x, y })))
        };
        let tray = place(|p| p.tray);
        let founder = place(|p| p.founder);
        let b = Building { floors, crc: hasher.finalize(), outside, tray, founder };
        if b.spawns().is_empty() {
            return Err("building has no spawns".into());
        }
        for (f, m) in b.active_floors() {
            for l in &m.links {
                if let LinkKind::Stairs { to_floor, to } = &l.kind {
                    let Some(dest) = b.floor(*to_floor) else {
                        return Err(format!("floor {f}: stairs lead to inactive floor {to_floor}"));
                    };
                    if dest.is_blocked(to.x, to.y) || dest.link_at(to.x, to.y).is_some() {
                        return Err(format!("floor {f}: stairs arrival {to:?} must be free and outside links"));
                    }
                }
            }
        }
        Ok(b)
    }

    /// Map of an active (existing, unlocked) floor.
    pub fn floor(&self, f: u8) -> Option<&Map> {
        self.floors.get(f as usize).filter(|fl| !fl.locked).and_then(|fl| fl.map.as_ref())
    }

    /// Mutable map of an active floor (locking doors).
    pub fn floor_mut(&mut self, f: u8) -> Option<&mut Map> {
        self.floors.get_mut(f as usize).filter(|fl| !fl.locked).and_then(|fl| fl.map.as_mut())
    }

    /// Room at a position on an active floor (0 = no room / no such floor).
    pub fn room_at(&self, floor: u8, pos: Pos) -> u16 {
        self.floor(floor).map_or(0, |m| m.room_at(pos.x, pos.y))
    }

    pub fn floor_name(&self, f: u8) -> &str {
        self.floors.get(f as usize).map_or("?", |fl| fl.name.as_str())
    }

    /// Rooms of the floor below seen from `room` (a balcony): (floor, room).
    pub fn below(&self, floor: u8, room: u16) -> Vec<(u8, u16)> {
        let (Some(m), Some(down)) = (self.floor(floor), floor.checked_sub(1).and_then(|f| self.floor(f))) else { return vec![] };
        let Some(def) = m.rooms.iter().find(|r| r.id == room) else { return vec![] };
        def.below.iter().filter_map(|name| down.room_by_name(name)).map(|r| (floor - 1, r.id)).collect()
    }

    pub fn active_floors(&self) -> impl Iterator<Item = (u8, &Map)> {
        (0..self.floors.len() as u8).filter_map(|f| self.floor(f).map(|m| (f, m)))
    }

    pub fn spawns(&self) -> Vec<Place> {
        self.active_floors().flat_map(|(f, m)| m.spawns.iter().map(move |t| (f, *t))).collect()
    }

    /// Next active floor (cyclically, going up) with an elevator cabin `id`.
    pub fn next_elevator_floor(&self, from: u8, id: &str) -> Option<u8> {
        let n = self.floors.len() as u8;
        (1..n)
            .map(|k| (from + k) % n)
            .find(|&f| self.floor(f).is_some_and(|m| m.links.iter().any(|l| matches!(&l.kind, LinkKind::Elevator { id: i } if i == id))))
    }

    pub fn find_room(&self, name: &str) -> Option<(u8, &RoomDef)> {
        self.active_floors().find_map(|(f, m)| m.room_by_name(name).map(|r| (f, r)))
    }

    /// All NPC definitions with their floor.
    pub fn npcs(&self) -> Vec<(u8, NpcDef)> {
        self.active_floors().flat_map(|(f, m)| m.npcs.iter().map(move |n| (f, n.clone()))).collect()
    }

    /// BFS across floors for a character with rights `access`. Stepping onto a
    /// stairs tile continues at its arrival tile on the other floor; the path
    /// contains the stairs tile followed by the arrival. One-way tiles (gates)
    /// are respected. (Elevators need an interact press and are not used.)
    pub fn find_path(&self, from: Place, to: Place, access: u8) -> Option<Vec<Place>> {
        let valid = |p: &Place| self.floor(p.0).is_some_and(|m| !m.is_blocked(p.1.x, p.1.y));
        if !valid(&from) || !valid(&to) {
            return None;
        }
        // node -> (previous node, stairs tile passed on the way, if any)
        let mut prev: HashMap<Place, (Place, Option<Place>)> = HashMap::new();
        prev.insert(from, (from, None));
        let mut queue = VecDeque::from([from]);
        while let Some(cur) = queue.pop_front() {
            if cur == to {
                let mut path = vec![to];
                let mut n = to;
                while n != from {
                    let (p, via) = prev[&n];
                    if let Some(v) = via {
                        path.push(v);
                    }
                    path.push(p);
                    n = p;
                }
                path.reverse();
                path.dedup();
                return Some(path);
            }
            let (f, t) = cur;
            let Some(m) = self.floor(f) else { continue };
            for (dx, dy, d) in [(1, 0, dir::RIGHT), (-1, 0, dir::LEFT), (0, 1, dir::DOWN), (0, -1, dir::UP)] {
                let nt = Tile { x: t.x + dx, y: t.y + dy };
                if m.blocks(nt.x, nt.y, access, d) {
                    continue;
                }
                let (next, via) = match m.link_at(nt.x, nt.y).map(|l| &l.kind) {
                    Some(LinkKind::Stairs { to_floor, to }) if self.floor(*to_floor).is_some() => ((*to_floor, *to), Some((f, nt))),
                    _ => ((f, nt), None),
                };
                if let std::collections::hash_map::Entry::Vacant(e) = prev.entry(next) {
                    e.insert((cur, via));
                    queue.push_back(next);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b() -> Building {
        Building::load(&default_building_path()).unwrap()
    }

    #[test]
    fn loads_two_active_floors_and_locked_third() {
        let b = b();
        assert_eq!(b.floors.len(), 4);
        assert!(b.floor(0).is_some() && b.floor(1).is_some());
        assert!(b.floor(2).is_none() && b.floors[2].locked);
        assert_eq!(b.floor_name(1), "Piętro 1");
        assert!(b.floor(3).is_some(), "the stairwell between 0 and 1 is a map of its own");
    }

    #[test]
    fn elevator_cycles_between_active_floors() {
        let b = b();
        for lift in ["A", "B"] {
            assert_eq!(b.next_elevator_floor(0, lift), Some(1));
            assert_eq!(b.next_elevator_floor(1, lift), Some(0), "locked floor 2 is skipped");
        }
        assert_eq!(b.next_elevator_floor(0, "nope"), None);
    }

    #[test]
    fn path_goes_upstairs() {
        let b = b();
        let spawn = b.spawns()[0];
        let (f, room) = b.find_room("Chill room").unwrap();
        let goal = b.floor(f).unwrap().room_tiles(room.id)[0];
        let path = b.find_path(spawn, (f, goal), crate::map::access::GUEST).expect("reachable");
        assert!(b.find_path(spawn, (f, goal), 0).is_none(), "gates stop visitors without a pass");
        assert_eq!(path.first(), Some(&spawn));
        assert_eq!(path.last(), Some(&(1, goal)));
        // Ground floor -> stairwell (landing) -> floor 1.
        let floors: Vec<u8> = path.iter().map(|p| p.0).fold(Vec::new(), |mut v, f| {
            if v.last() != Some(&f) {
                v.push(f);
            }
            v
        });
        assert_eq!(floors, [0, 3, 1]);
        let i = path.iter().position(|p| p.0 == 1).unwrap();
        assert_eq!(b.floor(3).unwrap().tile_char(path[i - 1].1.x, path[i - 1].1.y), Some('S'));
        assert_eq!(path[i].1, Tile { x: 25, y: 41 }, "arrival tile (the stairwell upstairs)");
        for w in path.windows(2) {
            if w[0].0 == w[1].0 {
                let d = (w[0].1.x - w[1].1.x).abs() + (w[0].1.y - w[1].1.y).abs();
                assert_eq!(d, 1, "4-connected steps: {:?}", w);
            }
        }
    }

    #[test]
    fn the_balcony_is_off_the_kitchenette_outdoors_and_looks_down_outside() {
        let b = Building::load(&default_building_path()).unwrap();
        let m = b.floor(1).unwrap();
        let balcony = m.room_by_name("Balkon").unwrap();
        assert!(balcony.outdoor);
        let kitchen = m.room_by_name("Aneks kuchenny").unwrap().id;
        let t = m.room_tiles(kitchen)[0];
        let path = b.find_path((1, t), (1, Tile { x: 25, y: 4 }), 0).expect("from the kitchenette to the balcony");
        assert!(!path.is_empty());
        let below = b.below(1, balcony.id);
        let street = b.floor(0).unwrap().room_by_name("Na zewnątrz").unwrap().id;
        assert_eq!(below, vec![(0, street)]);
        assert!(b.below(1, kitchen).is_empty());
    }
}
