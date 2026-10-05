//! Toilet stalls: small rooms with a door that the person inside can lock.
//!
//! A stall is its own room, so interest management already hides who is
//! inside from the rest of the bathroom (the stall "sees" the bathroom, not
//! the other way round). The door tile belongs to the stall: whoever steps
//! into an unlocked door sees who is in there. A locked door is solid for
//! everyone (`Map::set_closed`), and unlocks when the person who locked it
//! leaves the stall room (or the game).

use crate::building::Building;
use crate::map::Tile;
use crate::sim::{Pos, HALF_H, HALF_W, TILE_UNITS};

#[derive(Debug, Clone)]
pub struct Stall {
    pub floor: u8,
    pub door: Tile,
    /// Room id of the stall.
    pub room: u16,
    /// Who locked it (None = unlocked).
    pub locked_by: Option<u16>,
}

pub mod lines {
    pub const LOCKED: &str = "Klik. Zajęte.";
    pub const UNLOCKED: &str = "Otwieram.";
    pub const IN_DOORWAY: &str = "Ktoś stoi w drzwiach.";
    pub const STEP_IN: &str = "Najpierw wejdę do środka.";
    pub const NO_STALL: &str = "Tu nie ma czego zamknąć.";
}

pub fn find_stalls(b: &Building) -> Vec<Stall> {
    let mut out = Vec::new();
    for (f, m) in b.active_floors() {
        for y in 0..m.height {
            for x in 0..m.width {
                if m.tile_type(x, y) == Some("stall_door") {
                    out.push(Stall { floor: f, door: Tile { x, y }, room: m.room_at_tile(x, y), locked_by: None });
                }
            }
        }
    }
    out
}

/// Whether a character's collision box at `pos` touches the tile.
pub fn touches(pos: Pos, t: Tile) -> bool {
    let (x0, y0) = (t.x * TILE_UNITS, t.y * TILE_UNITS);
    pos.x + HALF_W > x0 && pos.x - HALF_W < x0 + TILE_UNITS && pos.y + HALF_H > y0 && pos.y - HALF_H < y0 + TILE_UNITS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::default_building_path;
    use crate::map::access;
    use crate::sim::{self, Body, IN_LEFT, IN_RIGHT};

    #[test]
    fn every_stall_is_its_own_room_and_one_in_a_bathroom_sees_it() {
        let b = Building::load(&default_building_path()).unwrap();
        let stalls = find_stalls(&b);
        assert_eq!(
            stalls.len(),
            14,
            "the hall toilet, 2 in the corridor, 2 in the women's bathroom by it, 3 in the wing bathrooms, 6 on floor 3"
        );
        for s in &stalls {
            let m = b.floor(s.floor).unwrap();
            let def = m.rooms.iter().find(|r| r.id == s.room).unwrap();
            assert_eq!(def.kind, "stall");
            if def.see.is_empty() {
                continue; // a toilet of its own, straight off the corridor / hall
            }
            assert_eq!(m.visible_from(s.room).len(), 1, "a stall sees its bathroom");
            let bath = m.visible_from(s.room)[0];
            assert!(!m.visible_from(bath).contains(&s.room), "...but not the other way round");
        }
    }

    #[test]
    fn a_locked_door_is_solid_for_everyone() {
        let mut b = Building::load(&default_building_path()).unwrap();
        let s = find_stalls(&b).into_iter().find(|s| s.floor == 4 && s.door.x == 5 && s.door.y == 45).expect("women's stall 1");
        // From the washbasins right of the door, walk left into the stall.
        let start = Body { access: access::CARD, ..Body::at(4, Pos::tile_center(7, 45)) };
        let walk = |b: &Building| (0..60).fold(start, |body, _| sim::step(b, body, IN_LEFT));
        assert!(walk(&b).pos.x < Pos::tile_center(5, 45).x, "open: walks in");
        b.floor_mut(4).unwrap().set_closed(s.door.x, s.door.y, true);
        let stopped = walk(&b);
        assert_eq!(stopped.pos.x, 6 * TILE_UNITS + HALF_W, "locked: stops at the door");
        assert!(!touches(stopped.pos, s.door));
        // And from inside you can't get out either.
        let inside = Body { access: access::CARD, ..Body::at(4, Pos::tile_center(4, 45)) };
        let out = (0..60).fold(inside, |body, _| sim::step(&b, body, IN_RIGHT));
        assert!(out.pos.x < 5 * TILE_UNITS, "stays inside");
    }
}
