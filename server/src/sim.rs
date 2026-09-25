//! Deterministic movement and wall collision.
//!
//! All positions are integer sub-pixel units (1 px = 16 units, 1 tile = 256).
//! `client/sim/movement.gd` implements exactly the same algorithm; parity is
//! checked by the golden vectors in `tests/golden/movement_vectors.json`.

use crate::map::Map;

pub const SUBPIXELS: i32 = 16;
pub const TILE_UNITS: i32 = 16 * SUBPIXELS;

/// Input steps per second (client samples input at this rate).
pub const INPUT_HZ: u32 = 60;
/// Movement per input step: 24 units = 1.5 px -> 90 px/s.
pub const SPEED: i32 = 24;
/// Per-axis movement on a diagonal step (24 / sqrt(2) ~= 17).
pub const SPEED_DIAG: i32 = 17;

/// Collision box half extents (box is 10 x 8 px, centered on the position).
pub const HALF_W: i32 = 5 * SUBPIXELS;
pub const HALF_H: i32 = 4 * SUBPIXELS;

pub const IN_UP: u8 = 1;
pub const IN_DOWN: u8 = 2;
pub const IN_LEFT: u8 = 4;
pub const IN_RIGHT: u8 = 8;
/// Bits 4..7 are reserved for future actions (interact, run, ...).
pub const IN_MOVE_MASK: u8 = 0x0f;

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

/// Apply one input step. Movement is resolved per axis (X, then Y) so the
/// player slides along walls.
pub fn step(map: &Map, pos: Pos, input: u8) -> Pos {
    let (dx, dy) = input_dir(input);
    if dx == 0 && dy == 0 {
        return pos;
    }
    let speed = if dx != 0 && dy != 0 { SPEED_DIAG } else { SPEED };
    let mut p = pos;
    if dx != 0 {
        p.x = move_x(map, p, dx * speed);
    }
    if dy != 0 {
        p.y = move_y(map, p, dy * speed);
    }
    p
}

fn move_x(map: &Map, p: Pos, mx: i32) -> i32 {
    let nx = p.x + mx;
    let ty0 = tile_of(p.y - HALF_H);
    let ty1 = tile_of(p.y + HALF_H - 1);
    if mx > 0 {
        let tx = tile_of(nx + HALF_W - 1);
        if (ty0..=ty1).any(|ty| map.is_blocked(tx, ty)) {
            return tx * TILE_UNITS - HALF_W;
        }
    } else {
        let tx = tile_of(nx - HALF_W);
        if (ty0..=ty1).any(|ty| map.is_blocked(tx, ty)) {
            return (tx + 1) * TILE_UNITS + HALF_W;
        }
    }
    nx
}

fn move_y(map: &Map, p: Pos, my: i32) -> i32 {
    let ny = p.y + my;
    let tx0 = tile_of(p.x - HALF_W);
    let tx1 = tile_of(p.x + HALF_W - 1);
    if my > 0 {
        let ty = tile_of(ny + HALF_H - 1);
        if (tx0..=tx1).any(|tx| map.is_blocked(tx, ty)) {
            return ty * TILE_UNITS - HALF_H;
        }
    } else {
        let ty = tile_of(ny - HALF_H);
        if (tx0..=tx1).any(|tx| map.is_blocked(tx, ty)) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::default_map_path;

    fn map() -> Map {
        Map::load(&default_map_path()).unwrap()
    }

    fn run(map: &Map, mut p: Pos, input: u8, n: usize) -> Pos {
        for _ in 0..n {
            p = step(map, p, input);
        }
        p
    }

    #[test]
    fn moves_at_constant_speed() {
        let m = map();
        let p = Pos::tile_center(40, 30); // lobby
        assert_eq!(step(&m, p, IN_RIGHT), Pos { x: p.x + SPEED, y: p.y });
        assert_eq!(step(&m, p, IN_UP), Pos { x: p.x, y: p.y - SPEED });
        assert_eq!(step(&m, p, IN_UP | IN_LEFT), Pos { x: p.x - SPEED_DIAG, y: p.y - SPEED_DIAG });
        assert_eq!(step(&m, p, IN_LEFT | IN_RIGHT), p, "opposite keys cancel");
        assert_eq!(step(&m, p, 0), p);
    }

    #[test]
    fn stops_flush_against_wall() {
        let m = map();
        // Lobby spans x 32..=50; wall at x=51 (except door rows 30,31).
        let p = run(&m, Pos::tile_center(45, 27), IN_RIGHT, 200);
        assert_eq!(p.x, 51 * TILE_UNITS - HALF_W);
        let p = run(&m, Pos::tile_center(45, 27), IN_UP, 200);
        assert_eq!(p.y, 25 * TILE_UNITS + HALF_H);
        assert!(box_is_free(&m, p));
    }

    #[test]
    fn slides_along_wall_on_diagonal() {
        let m = map();
        let start = Pos::tile_center(45, 26);
        let p = run(&m, start, IN_UP | IN_RIGHT, 20);
        assert_eq!(p.y, 25 * TILE_UNITS + HALF_H, "pinned to top wall");
        assert!(p.x > start.x, "still slides right");
    }

    #[test]
    fn passes_through_door() {
        let m = map();
        // Door lobby <-> shop at x=51, y=30..31. Walk right from lobby row 30/31 boundary.
        let start = Pos { x: 49 * TILE_UNITS, y: 31 * TILE_UNITS };
        let p = run(&m, start, IN_RIGHT, 100);
        assert!(p.x > 52 * TILE_UNITS, "entered the shop: {p:?}");
        assert_eq!(m.room_at(p.x, p.y), m.room_by_name("Sklep").unwrap().id);
    }

    #[test]
    fn furniture_blocks() {
        let m = map();
        // Desk row at y=3, x=23..28 in the open space; walk down from y=1.
        let p = run(&m, Pos::tile_center(25, 1), IN_DOWN, 100);
        assert_eq!(p.y, 3 * TILE_UNITS - HALF_H);
    }

    #[test]
    fn never_ends_inside_walls_random_walk() {
        let m = map();
        let mut rng = fastrand::Rng::with_seed(7);
        let mut p = Pos::tile_center(40, 35);
        for _ in 0..50_000 {
            p = step(&m, p, rng.u8(0..16));
            assert!(box_is_free(&m, p), "box overlaps wall at {p:?}");
        }
    }
}
