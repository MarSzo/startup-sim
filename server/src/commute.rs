//! Getting to work (backlog 9b): every morning the employee picks how to
//! commute - on foot, by bike, car, taxi or tram. The choice sets the travel
//! time, the cost, what the trip does to the needs, and how you arrive: the
//! car drives in from the street and parks on the outside car park, the bike
//! stops at the rack, the taxi drops you at the kerb, the tram at its stop.
//!
//! Vehicles are server entities (`kind::VEHICLE`) following a list of
//! waypoints; the arriving player rides inside (hidden, the camera follows)
//! and gets out at the vehicle's stop.

use crate::map::Tile;
use crate::outside::Outside;
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
    /// Patrol car (security.rs): not anybody's commute.
    pub const POLICE: u8 = 5;
    /// Fire engine (fire.rs).
    pub const FIRE_ENGINE: u8 = 6;
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

/// Where you go home from without a vehicle of your own: on foot the west
/// end of the sidewalk, the tram stop, the taxi stand (floor 0 tiles), with
/// how far from it (tiles).
pub fn home_spot(o: &Outside, mode_id: u8) -> Option<(Pos, i32)> {
    let at = |t: Tile| tile(t.x, t.y);
    match mode_id {
        mode::WALK => Some((at(o.walk_home), 2)),
        mode::TAXI => Some((at(o.taxi), 2)),
        mode::TRAM => Some((at(o.tram_stop), 2)),
        _ => None,
    }
}

pub mod lines {
    pub const LATE: &str = "Spóźnienie… Oby nikt nie zauważył.";
    pub const TOO_POOR: &str = "Nie stać mnie dziś na przejazd — idę pieszo.";
    pub const GO_HOME_ASK: &str = "Wracam do domu? To koniec dnia pracy (wypłata za przepracowany czas). E jeszcze raz — tak.";
    pub fn went_home(minutes: u32) -> String {
        format!("Do domu! Dziś przepracowane: {} h {} min.", minutes / 60, minutes % 60)
    }
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
    pub fn for_mode(o: &Outside, mode_id: u8, handle: u16, owner: u16, slot: usize) -> Option<Vehicle> {
        let street = o.street_y;
        let (kind, path, stop, dwell, alight, parks, speed) = match mode_id {
            mode::CAR => {
                // In from the east along the street, into the car park, park
                // in one of the free bays (the extra cars share the last one).
                let bay = o.car_bays[slot.min(o.car_bays.len() - 1)];
                let x = bay.x;
                let path =
                    vec![o.street_east(), tile(x, street), tile(x, bay.y - 2), Pos { x: tile(x, bay.y).x, y: (bay.y + 1) * TILE_UNITS }];
                (vehicle::CAR, path, 3, 0, tile(x + 2, bay.y - 1), true, 128)
            }
            mode::BIKE => {
                let rack = o.bike_rack[slot % o.bike_rack.len()];
                let path = vec![tile(18, street), tile(rack.x, street), tile(rack.x, rack.y + 1), tile(rack.x, rack.y)];
                (vehicle::BIKE, path, 3, 0, tile(rack.x, rack.y + 1), true, 64)
            }
            mode::TAXI => {
                let path = vec![o.street_east(), tile(o.taxi.x, street), tile(1, street)];
                (vehicle::TAXI, path, 1, 30, tile(o.taxi.x, o.taxi.y), false, 128)
            }
            mode::TRAM => {
                let y = tile(0, o.tram_y).y;
                let path = vec![Pos { x: (o.width + 2) * TILE_UNITS, y }, tile(o.tram_stop.x, o.tram_y), Pos { x: -6 * TILE_UNITS, y }];
                (vehicle::TRAM, path, 1, 40, tile(o.tram_stop.x, o.tram_stop.y), false, 96)
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

    /// A patrol car: drives up to the entrance and waits there (`leave`).
    pub fn police(o: &Outside, handle: u16) -> Vehicle {
        Vehicle::emergency(handle, vehicle::POLICE, crate::security::car_path(o))
    }

    /// A fire engine: to the building, waits (`leave`).
    pub fn fire_engine(o: &Outside, handle: u16) -> Vehicle {
        Vehicle::emergency(handle, vehicle::FIRE_ENGINE, crate::fire::truck_path(o))
    }

    fn emergency(handle: u16, kind: u8, path: Vec<Pos>) -> Vehicle {
        Vehicle {
            handle,
            kind,
            owner: 0,
            pos: path[0],
            path,
            next: 1,
            speed: 128,
            stop: usize::MAX - 1,
            dwell: 0,
            rider: None,
            alight: Pos { x: 0, y: 0 },
            parks: true,
            facing: 2,
            moving: true,
        }
    }

    /// A parked car / bike drives home: out onto the street, then west.
    pub fn depart(&mut self, o: &Outside) {
        let street = o.street(0).y;
        self.path.push(Pos { x: self.pos.x, y: street });
        self.leave(o.street_west_off(6));
        self.owner = 0;
    }

    /// Drive off the map (then `VehicleEvent::Gone`).
    pub fn leave(&mut self, exit: Pos) {
        self.parks = false;
        self.path.push(exit);
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
        let b = Building::load(&default_building_path()).unwrap();
        assert!(Vehicle::for_mode(&b.outside, mode::WALK, 1, 1, 0).is_none(), "on foot: no vehicle");
        let m = b.floor(0).unwrap();
        for id in [mode::CAR, mode::BIKE, mode::TAXI, mode::TRAM] {
            let mut v = Vehicle::for_mode(&b.outside, id, 1, 7, 0).unwrap();
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
        let o = Building::load(&default_building_path()).unwrap().outside;
        let a = Vehicle::for_mode(&o, mode::CAR, 1, 1, 0).unwrap().alight;
        let b = Vehicle::for_mode(&o, mode::CAR, 2, 2, 1).unwrap().alight;
        assert_ne!(a, b);
    }
}
