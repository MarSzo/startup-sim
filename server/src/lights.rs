//! Lights (backlog): rooms have windows or not, and a light switch by the
//! door (offices, bathrooms, the chill room...); common areas are always
//! lit. The server keeps which lamps are on (E at the switch toggles, for
//! everybody); the client works out how bright each room is from the time
//! of day, the weather, the windows and the lamps. Lamps start the day off
//! and all go off when the office closes.

use std::collections::HashSet;

use crate::building::Building;
use crate::sim::{Body, Pos, TILE_UNITS};

/// Reach of a light switch: standing right by it (1 tile), so the toilet /
/// sink next to it still gets the E.
pub const REACH: i32 = TILE_UNITS;

pub mod lines {
    pub const ON: &str = "Pstryk — światło włączone.";
    pub const OFF: &str = "Pstryk — światło zgaszone.";
}

/// A light switch: (floor, room, position of the tile by it).
#[derive(Debug, Clone, Copy)]
pub struct Switch {
    pub floor: u8,
    pub room: u16,
    pub pos: Pos,
}

pub fn switches(b: &Building) -> Vec<Switch> {
    let mut out = Vec::new();
    for (f, m) in b.active_floors() {
        for r in &m.rooms {
            if let (true, Some([x, y])) = (r.light == "switch", r.switch) {
                out.push(Switch { floor: f, room: r.id, pos: Pos::tile_center(x, y) });
            }
        }
    }
    out
}

pub fn in_reach(s: &Switch, body: &Body) -> bool {
    body.floor == s.floor && (s.pos.x - body.pos.x).pow(2) + (s.pos.y - body.pos.y).pow(2) <= REACH * REACH
}

/// Lamps that are on, per (floor, room).
#[derive(Debug, Default)]
pub struct Lights {
    pub on: HashSet<(u8, u16)>,
}

impl Lights {
    /// Flip the lamp of a room; true = now on.
    pub fn toggle(&mut self, place: (u8, u16)) -> bool {
        if !self.on.remove(&place) {
            self.on.insert(place);
            true
        } else {
            false
        }
    }

    pub fn floor_on(&self, floor: u8) -> Vec<u16> {
        let mut v: Vec<u16> = self.on.iter().filter(|(f, _)| *f == floor).map(|(_, r)| *r).collect();
        v.sort();
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::default_building_path;

    #[test]
    fn every_switch_room_has_its_switch_inside_and_bathrooms_have_no_windows() {
        let b = Building::load(&default_building_path()).unwrap();
        let sw = switches(&b);
        assert!(sw.len() >= 9, "{}", sw.len());
        for s in &sw {
            let m = b.floor(s.floor).unwrap();
            let (x, y) = s.pos.tile();
            let r = m.room_at_tile(x, y);
            assert!(r == s.room || m.tile_type(x, y).is_some_and(|t| t.contains("door")), "{} at {x},{y}", m.room_name(s.room));
        }
        let m1 = b.floor(1).unwrap();
        assert!(!m1.room_by_name("Łazienka męska").unwrap().windows);
        assert!(m1.room_by_name("Produkt / IT").unwrap().windows);
        assert_eq!(m1.room_by_name("WC męskie").unwrap().lit_by.as_deref(), Some("Łazienka męska"));
        let mut l = Lights::default();
        assert!(l.toggle((1, 5)));
        assert_eq!(l.floor_on(1), vec![5]);
        assert!(!l.toggle((1, 5)));
    }
}
