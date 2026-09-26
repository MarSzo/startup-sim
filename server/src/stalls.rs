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
    use crate::sim::{self, Body, IN_RIGHT, IN_LEFT};

    #[test]
    fn six_stalls_each_its_own_room_that_sees_the_bathroom() {
        let b = Building::load(&default_building_path()).unwrap();
        let stalls = find_stalls(&b);
        assert_eq!(stalls.len(), 6);
        let m = b.floor(1).unwrap();
        for s in &stalls {
            let def = m.rooms.iter().find(|r| r.id == s.room).unwrap();
            assert_eq!(def.kind, "stall");
            assert_eq!(m.visible_from(s.room).len(), 1, "a stall sees its bathroom");
            let bath = m.visible_from(s.room)[0];
            assert!(!m.visible_from(bath).contains(&s.room), "...but not the other way round");
        }
    }

    #[test]
    fn a_locked_door_is_solid_for_everyone() {
        let mut b = Building::load(&default_building_path()).unwrap();
        let s = find_stalls(&b).into_iter().find(|s| s.door.x == 46 && s.door.y == 27).expect("women's stall 1");
        // From the walkway right of the door, walk left into the stall.
        let start = Body { access: access::CARD, ..Body::at(1, Pos::tile_center(48, 27)) };
        let walk = |b: &Building| (0..60).fold(start, |body, _| sim::step(b, body, IN_LEFT));
        assert!(walk(&b).pos.x < Pos::tile_center(46, 27).x, "open: walks in");
        b.floor_mut(1).unwrap().set_closed(s.door.x, s.door.y, true);
        let stopped = walk(&b);
        assert_eq!(stopped.pos.x, 47 * TILE_UNITS + HALF_W, "locked: stops at the door");
        assert!(!touches(stopped.pos, s.door));
        // And from inside you can't get out either.
        let inside = Body { access: access::CARD, ..Body::at(1, Pos::tile_center(45, 27)) };
        let out = (0..60).fold(inside, |body, _| sim::step(&b, body, IN_RIGHT));
        assert!(out.pos.x < 46 * TILE_UNITS, "stays inside");
    }
}
