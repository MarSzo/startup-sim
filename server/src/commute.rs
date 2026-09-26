//! Getting to work (backlog 9b): every morning the employee picks how to
//! commute - on foot, by bike, car, taxi or tram. The choice sets the travel
//! time, the cost, what the trip does to the needs, and how you arrive: the
//! car drives in from the street and parks on the outside car park, the bike
//! stops at the rack, the taxi drops you at the kerb, the tram at its stop.
//!
//! Vehicles are server entities (`kind::VEHICLE`) following a list of
//! waypoints; the arriving player rides inside (hidden, the camera follows)
//! and gets out at the vehicle's stop.

use crate::sim::{Pos, TILE_UNITS};

pub mod mode {
    pub const WALK: u8 = 1;
    pub const BIKE: u8 = 2;
    pub const CAR: u8 = 3;
    pub const TAXI: u8 = 4;
    pub const TRAM: u8 = 5;
}

/// Vehicle kinds (`EntityState::held` of a `kind::VEHICLE` entity).
pub mod vehicle {
    pub const CAR: u8 = 1;
    pub const BIKE: u8 = 2;
    pub const TAXI: u8 = 3;
    pub const TRAM: u8 = 4;
}

/// Needs changed by the trip (points).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TripEffect {
    pub energy: i32,
    pub stress: i32,
    pub hygiene: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Mode {
    pub id: u8,
    pub name: &'static str,
    /// Game minutes door to door (the car adds traffic on top).
    pub minutes: u32,
    /// Grosze.
    pub cost: i64,
    pub effect: TripEffect,
}

pub const MODES: [Mode; 5] = [
    Mode { id: mode::WALK, name: "pieszo", minutes: 45, cost: 0, effect: TripEffect { energy: -5, stress: -3, hygiene: -2 } },
    Mode { id: mode::BIKE, name: "rower", minutes: 25, cost: 0, effect: TripEffect { energy: -8, stress: -5, hygiene: -10 } },
    Mode { id: mode::CAR, name: "samochód", minutes: 20, cost: 12_00, effect: TripEffect { energy: 0, stress: 5, hygiene: 0 } },
    Mode { id: mode::TAXI, name: "taksówka", minutes: 15, cost: 35_00, effect: TripEffect { energy: 0, stress: 0, hygiene: 0 } },
    Mode { id: mode::TRAM, name: "tramwaj", minutes: 30, cost: 4_40, effect: TripEffect { energy: -2, stress: 4, hygiene: -3 } },
];

/// Traffic jams: the car takes 0..=20 extra minutes.
pub const MAX_TRAFFIC: u32 = 20;
/// Departure: between 6:15 and 8:45 (minutes after the 6:00 opening).
pub const DEPART_FROM: u32 = 15;
pub const DEPART_TO: u32 = 165;
/// Arriving after 9:00 is late.
pub const LATE_AFTER: u32 = 9 * 60;

pub fn mode(id: u8) -> Option<&'static Mode> {
    MODES.iter().find(|m| m.id == id)
}

pub mod lines {
    pub const LATE: &str = "Spóźnienie… Oby nikt nie zauważył.";
    pub const TOO_POOR: &str = "Nie stać mnie dziś na przejazd — idę pieszo.";
}

fn tile(x: i32, y: i32) -> Pos {
    Pos::tile_center(x, y)
}

/// A vehicle on its way; the rider gets out at waypoint `stop`.
#[derive(Debug, Clone)]
pub struct Vehicle {
    pub handle: u16,
    pub kind: u8,
    /// The employee it belongs to / carries.
    pub owner: u16,
    pub pos: Pos,
    path: Vec<Pos>,
    next: usize,
    /// Sub-pixel units per tick.
    speed: i32,
    /// Index of the waypoint where the rider gets out.
    stop: usize,
    /// Ticks to wait at the stop.
    dwell: u32,
    pub rider: Option<u16>,
    /// Where the rider stands after getting out.
    pub alight: Pos,
    /// Stays at the end of the path (parked car / bike) instead of leaving.
    pub parks: bool,
    /// Facing: 0 down, 1 up, 2 left, 3 right (like players).
    pub facing: u8,
    pub moving: bool,
}

/// What a vehicle did this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleEvent {
    /// Reached its stop: the rider gets out at `alight`.
    Arrived { rider: u16, alight: Pos },
    /// Drove off the map: remove it.
    Gone,
}

impl Vehicle {
    /// The vehicle for commuting `mode_id`; `slot` picks a free parking
    /// space / rack place. None for walking.
    pub fn for_mode(mode_id: u8, handle: u16, owner: u16, slot: usize) -> Option<Vehicle> {
        let (kind, path, stop, dwell, alight, parks, speed) = match mode_id {
            mode::CAR => {
                // In from the right along the street, into the car park, park
                // in one of the free bays (row 43-44).
                let x = 5 + 5 * (slot as i32 % 5);
                let x = if slot >= 5 { 28 } else { x };
                let path = vec![tile(58, 37), tile(x, 37), tile(x, 41), Pos { x: tile(x, 43).x, y: 44 * TILE_UNITS }];
                (vehicle::CAR, path, 3, 0, tile(x + 2, 42), true, 128)
            }
            mode::BIKE => {
                let x = 40 + (slot as i32 % 4);
                let path = vec![tile(18, 37), tile(x, 37), tile(x, 35), tile(x, 34)];
                (vehicle::BIKE, path, 3, 0, tile(x, 35), true, 64)
            }
            mode::TAXI => {
                let path = vec![tile(58, 37), tile(34, 37), tile(1, 37)];
                (vehicle::TAXI, path, 1, 30, tile(34, 36), false, 128)
            }
            mode::TRAM => {
                let path = vec![Pos { x: 62 * TILE_UNITS, y: tile(0, 46).y }, tile(36, 46), Pos { x: -6 * TILE_UNITS, y: tile(0, 46).y }];
                (vehicle::TRAM, path, 1, 40, tile(36, 45), false, 96)
            }
            _ => return None,
        };
        Some(Vehicle {
            handle,
            kind,
            owner,
            pos: path[0],
            path,
            next: 1,
            speed,
            stop,
            dwell,
            rider: Some(owner),
            alight,
            parks,
            facing: 2,
            moving: true,
        })
    }

    pub fn parked(&self) -> bool {
        self.parks && self.next >= self.path.len()
    }

    pub fn tick(&mut self) -> Option<VehicleEvent> {
        if self.next >= self.path.len() {
            self.moving = false;
            return if self.parks { None } else { Some(VehicleEvent::Gone) };
        }
        let at_stop = self.next == self.stop + 1 && self.pos == self.path[self.stop];
        if at_stop && self.dwell > 0 {
            self.dwell -= 1;
            self.moving = false;
            return None;
        }
        let target = self.path[self.next];
        let (dx, dy) = (target.x - self.pos.x, target.y - self.pos.y);
        let step = |d: i32| d.signum() * d.abs().min(self.speed);
        let (mx, my) = (step(dx), if dx == 0 { step(dy) } else { 0 });
        self.pos = Pos { x: self.pos.x + mx, y: self.pos.y + my };
        self.moving = mx != 0 || my != 0;
        if mx != 0 {
            self.facing = if mx < 0 { 2 } else { 3 };
        } else if my != 0 {
            self.facing = if my < 0 { 1 } else { 0 };
        }
        if self.pos == target {
            let reached = self.next;
            self.next += 1;
            if reached == self.stop {
                if let Some(r) = self.rider.take() {
                    return Some(VehicleEvent::Arrived { rider: r, alight: self.alight });
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::{default_building_path, Building};

    fn run(v: &mut Vehicle, max: usize) -> Vec<VehicleEvent> {
        (0..max).filter_map(|_| v.tick()).collect()
    }

    #[test]
    fn every_mode_has_a_price_time_and_way_in() {
        assert_eq!(MODES.len(), 5);
        assert!(Vehicle::for_mode(mode::WALK, 1, 1, 0).is_none(), "on foot: no vehicle");
        let b = Building::load(&default_building_path()).unwrap();
        let m = b.floor(0).unwrap();
        for id in [mode::CAR, mode::BIKE, mode::TAXI, mode::TRAM] {
            let mut v = Vehicle::for_mode(id, 1, 7, 0).unwrap();
            let ev = run(&mut v, 3000);
            let (rider, alight) = match ev.first() {
                Some(VehicleEvent::Arrived { rider, alight }) => (*rider, *alight),
                other => panic!("mode {id}: {other:?}"),
            };
            assert_eq!(rider, 7);
            let (tx, ty) = alight.tile();
            assert!(!m.is_blocked(tx, ty), "mode {id}: gets out on a free tile ({tx},{ty})");
            if v.parks {
                assert!(v.parked(), "mode {id} stays parked");
            } else {
                assert_eq!(ev.last(), Some(&VehicleEvent::Gone), "mode {id} drives off");
            }
        }
    }

    #[test]
    fn cars_take_different_bays() {
        let a = Vehicle::for_mode(mode::CAR, 1, 1, 0).unwrap().alight;
        let b = Vehicle::for_mode(mode::CAR, 2, 2, 1).unwrap().alight;
        assert_ne!(a, b);
    }
}
