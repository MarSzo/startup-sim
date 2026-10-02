//! Path following for simulated characters (load-test bots now, NPCs later).
//! Produces input bits, so a follower moves exactly like a player would.

use crate::building::{Building, Place};
use crate::sim::{Body, Pos, IN_DOWN, IN_LEFT, IN_RIGHT, IN_UP, SPEED};

pub struct Walker {
    path: Vec<Place>,
    i: usize,
}

impl Walker {
    pub fn new(path: Vec<Place>) -> Walker {
        Walker { path, i: 0 }
    }

    /// Plan a path through the building; `None` if unreachable.
    pub fn to(b: &Building, body: &Body, goal: Place) -> Option<Walker> {
        let (tx, ty) = body.pos.tile();
        let start = (body.floor, crate::map::Tile { x: tx, y: ty });
        b.find_path(start, goal, body.access).map(Walker::new)
    }

    pub fn done(&self) -> bool {
        self.i >= self.path.len()
    }

    /// Waypoints not reached yet.
    pub fn remaining(&self) -> &[Place] {
        &self.path[self.i.min(self.path.len())..]
    }

    /// Input for the next step: head for the current waypoint's tile center,
    /// advancing waypoints as they are reached. Waypoints on other floors are
    /// skipped once the stairs have moved us (stairs tile -> arrival tile).
    pub fn next_input(&mut self, body: &Body) -> u8 {
        while self.i < self.path.len() {
            let (floor, t) = self.path[self.i];
            if floor != body.floor {
                self.i += 1;
                continue;
            }
            let bits = steer(body.pos, Pos::tile_center(t.x, t.y));
            if bits != 0 {
                return bits;
            }
            self.i += 1;
        }
        0
    }
}

/// Input bits moving `from` towards `to`; 0 once within half a step.
pub fn steer(from: Pos, to: Pos) -> u8 {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let mut bits = 0;
    if dx > SPEED / 2 {
        bits |= IN_RIGHT;
    } else if dx < -SPEED / 2 {
        bits |= IN_LEFT;
    }
    if dy > SPEED / 2 {
        bits |= IN_DOWN;
    } else if dy < -SPEED / 2 {
        bits |= IN_UP;
    }
    bits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::default_building_path;
    use crate::sim::step;

    #[test]
    fn walks_from_spawn_to_every_room_upstairs() {
        let b = Building::load(&default_building_path()).unwrap();
        let spawn = b.spawns()[0];
        let m1 = b.floor(4).unwrap();
        // Staff-only (service) rooms and the walled-up second lift aside.
        for room in m1.rooms.iter().filter(|r| !matches!(r.kind.as_str(), "service" | "elevator")) {
            // A tile off the stairs flight (standing there would teleport you).
            let tiles: Vec<_> = m1
                .room_tiles(room.id)
                .into_iter()
                .filter(|t| !matches!(m1.link_at(t.x, t.y).map(|l| &l.kind), Some(crate::map::LinkKind::Stairs { .. })))
                .collect();
            let goal = tiles[tiles.len() / 2];
            let mut body = Body::at(spawn.0, Pos::tile_center(spawn.1.x, spawn.1.y));
            // A guest; the board room needs a meeting (BOARD) on top.
            body.access = crate::map::access::GUEST | crate::map::access::BOARD;
            let mut w = Walker::to(&b, &body, (4, goal)).unwrap_or_else(|| panic!("no path to {}", room.name));
            let mut steps = 0;
            while !w.done() && steps < 20_000 {
                body = step(&b, body, w.next_input(&body));
                steps += 1;
            }
            assert!(w.done(), "{} not reached", room.name);
            assert_eq!((body.floor, body.pos.tile()), (4, (goal.x, goal.y)), "{}", room.name);
        }
    }
}
