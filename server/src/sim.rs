//! Deterministic movement, wall collision and floor transitions.
//!
//! All positions are integer sub-pixel units (1 px = 16 units, 1 tile = 256).
//! `client/sim/movement.gd` implements exactly the same algorithm; parity is
//! checked by the golden vectors in `tests/golden/movement_vectors.json`.

use crate::building::Building;
use crate::map::{dir, LinkKind, Map};

pub const SUBPIXELS: i32 = 16;
pub const TILE_UNITS: i32 = 16 * SUBPIXELS;

/// Input steps per second (client samples input at this rate).
pub const INPUT_HZ: u32 = 60;
/// Movement per input step: 24 units = 1.5 px -> 90 px/s.
pub const SPEED: i32 = 24;
/// Per-axis movement on a diagonal step (24 / sqrt(2) ~= 17).
pub const SPEED_DIAG: i32 = 17;
/// Exhausted or desperate for the toilet (`Body::slow`): 14 units -> 52 px/s.
pub const SPEED_SLOW: i32 = 14;
pub const SPEED_SLOW_DIAG: i32 = 10;

/// Collision box half extents (box is 10 x 8 px, centered on the position).
pub const HALF_W: i32 = 5 * SUBPIXELS;
pub const HALF_H: i32 = 4 * SUBPIXELS;

pub const IN_UP: u8 = 1;
pub const IN_DOWN: u8 = 2;
pub const IN_LEFT: u8 = 4;
pub const IN_RIGHT: u8 = 8;
/// Interact (E): uses the elevator now; doors, shop, NPCs later.
pub const IN_INTERACT: u8 = 16;
/// Bits 5..7 are reserved for future actions (run, ...).
pub const IN_MOVE_MASK: u8 = 0x0f;

/// Stairs re-trigger lock (see `step`).
pub const LOCK_NONE: u8 = 0;
/// Just changed floors; the same movement keys are still held.
pub const LOCK_HELD: u8 = 1;
/// Keys changed since the transition; unlocks once off the link area.
pub const LOCK_RELEASED: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    /// Center of a tile.
    pub fn tile_center(tx: i32, ty: i32) -> Pos {
        Pos { x: tx * TILE_UNITS + TILE_UNITS / 2, y: ty * TILE_UNITS + TILE_UNITS / 2 }
    }

    pub fn tile(self) -> (i32, i32) {
        (self.x.div_euclid(TILE_UNITS), self.y.div_euclid(TILE_UNITS))
    }
}

fn tile_of(v: i32) -> i32 {
    v.div_euclid(TILE_UNITS)
}

/// Direction from input bits: (dx, dy) in {-1, 0, 1}.
pub fn input_dir(input: u8) -> (i32, i32) {
    let dx = (input & IN_RIGHT != 0) as i32 - (input & IN_LEFT != 0) as i32;
    let dy = (input & IN_DOWN != 0) as i32 - (input & IN_UP != 0) as i32;
    (dx, dy)
}

/// Move by one input step on a single floor, for a character with rights
/// `access`. Movement is resolved per axis (X, then Y) so the player slides
/// along walls.
pub fn move_on(map: &Map, pos: Pos, input: u8, access: u8) -> Pos {
    move_at(map, pos, input, access, false)
}

/// `move_on` at normal or slow (`Body::slow`) speed.
pub fn move_at(map: &Map, pos: Pos, input: u8, access: u8, slow: bool) -> Pos {
    let (dx, dy) = input_dir(input);
    if dx == 0 && dy == 0 {
        return pos;
    }
    let speed = match (dx != 0 && dy != 0, slow) {
        (true, false) => SPEED_DIAG,
        (false, false) => SPEED,
        (true, true) => SPEED_SLOW_DIAG,
        (false, true) => SPEED_SLOW,
    };
    let mut p = pos;
    if dx != 0 {
        p.x = move_x(map, p, dx * speed, access);
    }
    if dy != 0 {
        p.y = move_y(map, p, dy * speed, access);
    }
    p
}

fn move_x(map: &Map, p: Pos, mx: i32, access: u8) -> i32 {
    let nx = p.x + mx;
    let ty0 = tile_of(p.y - HALF_H);
    let ty1 = tile_of(p.y + HALF_H - 1);
    if mx > 0 {
        let tx = tile_of(nx + HALF_W - 1);
        if (ty0..=ty1).any(|ty| map.blocks(tx, ty, access, dir::RIGHT)) {
            return tx * TILE_UNITS - HALF_W;
        }
    } else {
        let tx = tile_of(nx - HALF_W);
        if (ty0..=ty1).any(|ty| map.blocks(tx, ty, access, dir::LEFT)) {
            return (tx + 1) * TILE_UNITS + HALF_W;
        }
    }
    nx
}

fn move_y(map: &Map, p: Pos, my: i32, access: u8) -> i32 {
    let ny = p.y + my;
    let tx0 = tile_of(p.x - HALF_W);
    let tx1 = tile_of(p.x + HALF_W - 1);
    if my > 0 {
        let ty = tile_of(ny + HALF_H - 1);
        if (tx0..=tx1).any(|tx| map.blocks(tx, ty, access, dir::DOWN)) {
            return ty * TILE_UNITS - HALF_H;
        }
    } else {
        let ty = tile_of(ny - HALF_H);
        if (tx0..=tx1).any(|tx| map.blocks(tx, ty, access, dir::UP)) {
            return (ty + 1) * TILE_UNITS + HALF_H;
        }
    }
    ny
}

/// True if the collision box at `p` overlaps no blocked tile.
pub fn box_is_free(map: &Map, p: Pos) -> bool {
    for ty in tile_of(p.y - HALF_H)..=tile_of(p.y + HALF_H - 1) {
        for tx in tile_of(p.x - HALF_W)..=tile_of(p.x + HALF_W - 1) {
            if map.is_blocked(tx, ty) {
                return false;
            }
        }
    }
    true
}

/// Full simulated state of a character. Everything `step` depends on is
/// here, and the server sends all of it back in each snapshot so the client
/// can replay unacknowledged inputs from exactly the same state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Body {
    pub floor: u8,
    pub pos: Pos,
    /// Input applied on the previous step (for edge-triggered actions).
    pub prev_input: u8,
    /// `LOCK_*`: prevents bouncing straight back after taking the stairs
    /// while the movement key is still held.
    pub lock: u8,
    /// Rights (`map::access::*`): which gates/doors open. Changed only by the
    /// server (porter, HR...); the client learns it from snapshots.
    pub access: u8,
    /// Walks slowly (exhausted / needs the toilet badly). Set by the server
    /// from the character's needs; the client learns it from snapshots.
    pub slow: bool,
}

impl Body {
    pub fn at(floor: u8, pos: Pos) -> Body {
        Body { floor, pos, prev_input: 0, lock: LOCK_NONE, access: 0, slow: false }
    }
}

/// One input step (1/60 s) in the building: movement, then floor links.
///
/// - Stairs: stepping onto a stairs tile moves you to its arrival tile on the
///   other floor, unless locked. The lock is set on arrival and released once
///   the movement keys change *and* you are off any link area - so holding
///   "up" after arriving doesn't take you straight back down.
/// - The elevator is not part of the simulation: the server moves the
///   people in the cabin when it arrives (see `elevator.rs`).
pub fn step(b: &Building, body: Body, input: u8) -> Body {
    let Some(map) = b.floor(body.floor) else { return body };
    let mut n = body;
    n.pos = move_at(map, body.pos, input, body.access, body.slow);
    if n.lock == LOCK_HELD && (input & IN_MOVE_MASK) != (body.prev_input & IN_MOVE_MASK) {
        n.lock = LOCK_RELEASED;
    }
    let (tx, ty) = n.pos.tile();
    let link = map.link_at(tx, ty).map(|l| &l.kind);
    if n.lock == LOCK_RELEASED && link.is_none() {
        n.lock = LOCK_NONE;
    }
    match link {
        Some(LinkKind::Stairs { to_floor, to }) if n.lock == LOCK_NONE && b.floor(*to_floor).is_some() => {
            n.floor = *to_floor;
            n.pos = Pos::tile_center(to.x, to.y);
            n.lock = LOCK_HELD;
        }
        _ => {}
    }
    n.prev_input = input;
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::default_building_path;

    fn building() -> Building {
        Building::load(&default_building_path()).unwrap()
    }

    fn run(map: &Map, mut p: Pos, input: u8, n: usize) -> Pos {
        for _ in 0..n {
            p = move_on(map, p, input, 0);
        }
        p
    }

    fn walk(b: &Building, mut body: Body, input: u8, n: usize) -> Body {
        for _ in 0..n {
            body = step(b, body, input);
        }
        body
    }

    /// Hold `input` until the floor changes (or `max` steps).
    fn until_floor_change(b: &Building, mut body: Body, input: u8, max: usize) -> Body {
        let f = body.floor;
        for _ in 0..max {
            body = step(b, body, input);
            if body.floor != f {
                break;
            }
        }
        body
    }

    #[test]
    fn moves_at_constant_speed() {
        let b = building();
        let m = b.floor(0).unwrap();
        let p = Pos::tile_center(31, 49); // the hall, by the gates
        assert_eq!(move_on(m, p, IN_RIGHT, 0), Pos { x: p.x + SPEED, y: p.y });
        assert_eq!(move_on(m, p, IN_UP, 0), Pos { x: p.x, y: p.y - SPEED });
        assert_eq!(move_on(m, p, IN_UP | IN_LEFT, 0), Pos { x: p.x - SPEED_DIAG, y: p.y - SPEED_DIAG });
        assert_eq!(move_on(m, p, IN_LEFT | IN_RIGHT, 0), p, "opposite keys cancel");
        assert_eq!(move_on(m, p, IN_INTERACT, 0), p, "interact alone doesn't move");
    }

    #[test]
    fn stops_flush_against_wall() {
        let b = building();
        let m = b.floor(0).unwrap();
        // The hall by the gates: the porter's desk (x=35) on the right.
        let p = run(m, Pos::tile_center(31, 49), IN_RIGHT, 200);
        assert_eq!(p.x, 35 * TILE_UNITS - HALF_W);
        // The gates (row 46) stop you without a pass.
        let p = run(m, Pos::tile_center(31, 49), IN_UP, 200);
        assert_eq!(p.y, 47 * TILE_UNITS + HALF_H);
        assert!(box_is_free(m, p));
    }

    #[test]
    fn gates_need_a_pass_to_enter_but_not_to_leave() {
        use crate::map::access;
        let b = building();
        let room = |body: &Body| {
            let m = b.floor(body.floor).unwrap();
            m.room_name(m.room_at(body.pos.x, body.pos.y)).to_string()
        };
        let lobby = Body::at(0, Pos::tile_center(31, 48)); // below the gates (row 46)
        let stuck = walk(&b, lobby, IN_UP, 100);
        assert_eq!(stuck.pos.y, 47 * TILE_UNITS + HALF_H, "no pass: stopped at the gate");
        let guest = walk(&b, Body { access: access::GUEST, ..lobby }, IN_UP, 100);
        assert!(guest.pos.y < 46 * TILE_UNITS, "guest pass opens the gate");
        let employee = walk(&b, Body { access: access::CARD, ..lobby }, IN_UP, 100);
        assert!(employee.pos.y < 46 * TILE_UNITS, "employee card opens the gate");
        // Leaving: from the lifts, without any pass, down through the gate.
        let out = walk(&b, Body::at(0, Pos::tile_center(31, 44)), IN_DOWN, 100);
        assert!(out.pos.y > 47 * TILE_UNITS, "exit is free");
        // Garage gate (row 13, from the drive to the north): same rules.
        let garage = walk(&b, Body::at(0, Pos::tile_center(35, 11)), IN_DOWN, 100);
        assert_eq!(room(&garage), "Na zewnątrz", "no pass: can't drive in");
        let car = walk(&b, Body { access: access::CARD, ..Body::at(0, Pos::tile_center(35, 11)) }, IN_DOWN, 100);
        assert_eq!(room(&car), "Parking wewnętrzny");
    }

    #[test]
    fn slides_along_wall_on_diagonal() {
        let b = building();
        let m = b.floor(0).unwrap();
        let start = Pos::tile_center(24, 45); // shop, wall above
        let p = run(m, start, IN_UP | IN_RIGHT, 20);
        assert_eq!(p.y, 45 * TILE_UNITS + HALF_H, "pinned to top wall");
        assert!(p.x > start.x, "still slides right");
    }

    #[test]
    fn passes_through_door() {
        let b = building();
        let m = b.floor(0).unwrap();
        // Hall -> the draught lobby, door at x 31..32, row 53.
        let p = run(m, Pos { x: 32 * TILE_UNITS, y: 51 * TILE_UNITS }, IN_DOWN, 40);
        assert!(p.y > 54 * TILE_UNITS, "went through: {p:?}");
        assert_eq!(m.room_name(m.room_at(p.x, p.y)), "Wiatrołap");
    }

    #[test]
    fn furniture_and_locked_door_block() {
        let b = building();
        let m = b.floor(0).unwrap();
        let p = run(m, Pos::tile_center(22, 48), IN_UP, 100); // shop shelf at row 46
        assert_eq!(p.y, 47 * TILE_UNITS + HALF_H);
        let p = run(m, Pos::tile_center(40, 45), IN_RIGHT, 100); // locked door at x=43
        assert_eq!(p.x, 43 * TILE_UNITS - HALF_W);
    }

    /// Stairwell map (between floors 0 and 1).
    const MID: u8 = 4;

    #[test]
    fn stairs_go_through_the_stairwell_and_landing() {
        let b = building();
        // From the hall through the stairwell door (27,42), left onto the flight.
        let body = until_floor_change(&b, Body::at(0, Pos::tile_center(29, 42)), IN_LEFT, 300);
        assert_eq!(body.floor, MID, "into the stairwell");
        assert_eq!(body.pos, Pos::tile_center(32, 13), "bottom of the first flight");
        assert_eq!(body.lock, LOCK_HELD);
        // Keep holding UP: up the flight onto the landing, stays in the stairwell.
        let body = walk(&b, body, IN_UP, 200);
        assert_eq!(body.floor, MID);
        let m = b.floor(MID).unwrap();
        assert_eq!(m.room_name(m.room_at(body.pos.x, body.pos.y)), "Półpiętro");
        assert!(body.pos.tile().1 <= 6, "on the landing: {:?}", body.pos.tile());
        // Across the landing and down the second flight: floor 1.
        let body = walk(&b, body, IN_RIGHT, 60);
        let body = until_floor_change(&b, body, IN_DOWN, 300);
        assert_eq!(body.floor, 1, "the second flight leads to floor 1");
        assert_eq!(body.pos, Pos::tile_center(25, 41));
        // Keep holding DOWN: no bouncing back.
        let body = walk(&b, body, IN_DOWN, 60);
        assert_eq!(body.floor, 1);
        // And back: floor 1's flight -> stairwell (second flight) -> ground floor.
        let body = walk(&b, body, IN_UP, 40);
        let body = until_floor_change(&b, body, IN_LEFT, 300);
        assert_eq!((body.floor, body.pos), (MID, Pos::tile_center(36, 13)));
        let body = walk(&b, body, IN_UP, 200);
        let body = walk(&b, body, IN_LEFT, 60);
        let body = until_floor_change(&b, body, IN_DOWN, 300);
        assert_eq!((body.floor, body.pos), (0, Pos::tile_center(24, 42)), "down to the ground floor");
    }

    #[test]
    fn interact_in_the_elevator_cabin_changes_nothing_in_the_simulation() {
        let b = building();
        let body = walk(&b, Body::at(0, Pos::tile_center(37, 45)), IN_UP, 60); // into the cabin
        assert_eq!(b.floor(0).unwrap().tile_type(body.pos.tile().0, body.pos.tile().1), Some("elevator"));
        let after = step(&b, step(&b, body, 0), IN_INTERACT);
        assert_eq!(after.floor, 0, "the server moves the cabin, not the simulation");
    }

    #[test]
    fn never_ends_inside_walls_random_walk() {
        let b = building();
        let mut rng = fastrand::Rng::with_seed(7);
        let mut body = Body::at(0, Pos::tile_center(30, 59));
        let mut floors_seen = [false; 2];
        let mut held = 0;
        for _ in 0..200_000 {
            if rng.u8(0..20) == 0 {
                held = rng.u8(0..32);
            }
            body = step(&b, body, held);
            floors_seen[body.floor as usize] = true;
            assert!(box_is_free(b.floor(body.floor).unwrap(), body.pos), "box overlaps wall at {body:?}");
        }
        assert!(floors_seen[0]);
    }
}
