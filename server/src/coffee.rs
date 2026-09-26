//! Coffee machines: press E next to one -> it brews for a few seconds ->
//! you hold (and drink) a coffee for a while. One person per machine at a
//! time. What coffee *does* (energy, stress...) is still open in the GDD, so
//! for now it is an activity visible to everyone (mug in hand).

use crate::building::Building;
use crate::map::Tile;
use crate::sim::{Body, Pos, TILE_UNITS};

/// Brewing time (ticks at 20 Hz): 3 s.
pub const BREW_TICKS: u32 = 60;
/// How long you hold/drink the coffee: 90 s.
pub const DRINK_TICKS: u32 = 1800;
/// How close you must stand to use a machine (feet to machine tile centre).
pub const USE_RADIUS: i32 = TILE_UNITS * 3 / 2;

pub mod lines {
    pub const BREWING: &str = "Parzę kawę…";
    pub const READY: &str = "Kawa gotowa!";
    pub const BUSY: &str = "Ekspres zajęty — chwilka.";
    pub const ALREADY: &str = "Mam już kawę.";
    pub const FINISHED: &str = "Kawa wypita.";
}

/// Player's coffee state (server-side, per player).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cup {
    #[default]
    None,
    /// Waiting at `machine` until `until`.
    Brewing { machine: usize, until: u32 },
    /// Holding a coffee until `until`.
    Holding { until: u32 },
}

impl Cup {
    /// Bits for `Snapshot::self_status` / entity flags (see PROTOCOL.md).
    pub fn holding(&self) -> bool {
        matches!(self, Cup::Holding { .. })
    }
    pub fn brewing(&self) -> bool {
        matches!(self, Cup::Brewing { .. })
    }
}

#[derive(Debug, Clone)]
pub struct Machine {
    pub floor: u8,
    pub tile: Tile,
    /// Tick until which it's in use.
    pub busy_until: u32,
}

/// What happened, for the server to announce (self speech bubbles).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Started,
    Busy,
    AlreadyHave,
}

pub fn find_machines(b: &Building) -> Vec<Machine> {
    let mut out = Vec::new();
    for (f, m) in b.active_floors() {
        for y in 0..m.height {
            for x in 0..m.width {
                if m.tile_type(x, y) == Some("coffee_machine") {
                    out.push(Machine { floor: f, tile: Tile { x, y }, busy_until: 0 });
                }
            }
        }
    }
    out
}

/// Index of a machine within reach of `body`, if any.
pub fn machine_in_reach(machines: &[Machine], body: &Body) -> Option<usize> {
    machines.iter().position(|m| {
        let c = Pos::tile_center(m.tile.x, m.tile.y);
        let (dx, dy) = (c.x - body.pos.x, c.y - body.pos.y);
        m.floor == body.floor && dx * dx + dy * dy <= USE_RADIUS * USE_RADIUS
    })
}

/// Player pressed E at machine `i`.
pub fn use_machine(machines: &mut [Machine], i: usize, cup: &mut Cup, tick: u32) -> Outcome {
    if !matches!(cup, Cup::None) {
        return Outcome::AlreadyHave;
    }
    if machines[i].busy_until > tick {
        return Outcome::Busy;
    }
    machines[i].busy_until = tick + BREW_TICKS;
    *cup = Cup::Brewing { machine: i, until: tick + BREW_TICKS };
    Outcome::Started
}

/// Advance a player's cup; returns a line to say when something changes.
pub fn tick_cup(cup: &mut Cup, tick: u32) -> Option<&'static str> {
    match *cup {
        Cup::Brewing { until, .. } if tick >= until => {
            *cup = Cup::Holding { until: tick + DRINK_TICKS };
            Some(lines::READY)
        }
        Cup::Holding { until } if tick >= until => {
            *cup = Cup::None;
            Some(lines::FINISHED)
        }
        _ => None,
    }
}

/// A player left: free the machine they were brewing at.
pub fn release(machines: &mut [Machine], cup: &Cup) {
    if let Cup::Brewing { machine, .. } = cup {
        if let Some(m) = machines.get_mut(*machine) {
            m.busy_until = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::default_building_path;

    fn setup() -> (Building, Vec<Machine>) {
        let b = Building::load(&default_building_path()).unwrap();
        let m = find_machines(&b);
        (b, m)
    }

    #[test]
    fn one_machine_in_the_chill_room() {
        let (b, m) = setup();
        assert_eq!(m.len(), 1);
        let map = b.floor(m[0].floor).unwrap();
        assert_eq!(m[0].floor, 1);
        assert_eq!(map.room_name(map.room_at_tile(m[0].tile.x, m[0].tile.y + 1)), "Chill room");
    }

    #[test]
    fn reach_is_about_one_tile_away() {
        let (_, m) = setup();
        let t = m[0].tile;
        let front = Body::at(1, Pos::tile_center(t.x, t.y + 1));
        assert_eq!(machine_in_reach(&m, &front), Some(0));
        assert_eq!(machine_in_reach(&m, &Body::at(1, Pos::tile_center(t.x, t.y + 3))), None);
        assert_eq!(machine_in_reach(&m, &Body::at(0, front.pos)), None, "other floor");
    }

    #[test]
    fn brew_drink_and_share_the_machine() {
        let (_, mut m) = setup();
        let (mut a, mut b) = (Cup::None, Cup::None);
        assert_eq!(use_machine(&mut m, 0, &mut a, 100), Outcome::Started);
        assert!(a.brewing());
        assert_eq!(use_machine(&mut m, 0, &mut b, 110), Outcome::Busy, "one at a time");
        assert_eq!(use_machine(&mut m, 0, &mut a, 110), Outcome::AlreadyHave);
        assert_eq!(tick_cup(&mut a, 100 + BREW_TICKS - 1), None);
        assert_eq!(tick_cup(&mut a, 100 + BREW_TICKS), Some(lines::READY));
        assert!(a.holding());
        assert_eq!(use_machine(&mut m, 0, &mut b, 100 + BREW_TICKS), Outcome::Started, "free again");
        assert_eq!(tick_cup(&mut a, 100 + BREW_TICKS + DRINK_TICKS), Some(lines::FINISHED));
        assert_eq!(a, Cup::None);
    }

    #[test]
    fn leaving_frees_the_machine() {
        let (_, mut m) = setup();
        let mut a = Cup::None;
        use_machine(&mut m, 0, &mut a, 5);
        release(&mut m, &a);
        let mut b = Cup::None;
        assert_eq!(use_machine(&mut m, 0, &mut b, 6), Outcome::Started);
    }
}
