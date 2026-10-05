//! The elevator: call it (E at its doors), wait for it, step in, choose the
//! floor (E in the cabin opens the panel of floor buttons) and ride.
//!
//! Every floor has a cabin area and door tiles; the doors are closed
//! (`Map::set_closed`, solid for everyone) unless the car stands at that
//! floor with the doors open. When the car arrives, whoever stands in the
//! cabin area of the floor it left is moved to the same spot on the new
//! floor - the ride itself is not part of the movement simulation.

use crate::building::Building;
use crate::map::{LinkKind, Rect, Tile};
use crate::sim::{Body, Pos, TILE_UNITS};
use crate::stalls::touches;

/// Doors stay open this long after arriving / being called (4 s).
pub const DOOR_OPEN_TICKS: u32 = 80;
/// Travel time per floor (3 s).
pub const TRAVEL_TICKS: u32 = 60;
/// After choosing a floor inside, the doors close in 1 s.
pub const CLOSE_AFTER_PRESS: u32 = 20;
/// Someone in the doorway: the doors reopen for this long.
pub const DOORWAY_HOLD: u32 = 10;
/// Most people the car takes; with more in the cabin it won't leave.
pub const CAPACITY: usize = 6;
/// Overloaded: the doors stay open this long, then it checks again.
pub const OVERLOAD_HOLD: u32 = 40;
/// Reach of the call button (around the doors).
pub const CALL_RADIUS: i32 = TILE_UNITS * 3 / 2;

pub mod lines {
    pub const CALLED: &str = "Wzywam windę…";
    pub const ON_ITS_WAY: &str = "Winda już jedzie.";
    pub const HERE: &str = "Winda już jest.";
    pub const RIDING: &str = "Jedziemy…";
    pub const OVERLOAD: &str = "Przeciążenie! Maksymalnie 6 osób — ktoś musi wysiąść.";
    pub const NO_CARD: &str = "Winda tylko z kartą — przepustkę da portier.";
}

#[derive(Debug, Clone)]
pub struct Elevator {
    pub id: String,
    /// Floor the car is at (or left from, while moving).
    pub floor: u8,
    /// Moving: (target floor, arrival tick).
    pub moving: Option<(u8, u32)>,
    /// Doors open while `tick < open_until` (and not moving).
    pub open_until: u32,
    /// Floors waiting for the car, in order.
    pub calls: Vec<u8>,
    pub cabins: Vec<(u8, Rect)>,
    pub doors: Vec<(u8, Tile)>,
    /// Doors state after the last tick (for change detection).
    doors_open: bool,
}

/// What a tick did, for the server.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Update {
    /// Arrived: (from floor, to floor) - move the people in the cabin.
    pub arrived: Option<(u8, u8)>,
    /// Doors opened or closed somewhere: resend `Doors`.
    pub doors_changed: bool,
    /// Wanted to leave with too many people inside (floor): say so.
    pub overloaded: Option<u8>,
}

pub fn find_elevators(b: &Building) -> Vec<Elevator> {
    let mut out: Vec<Elevator> = Vec::new();
    for (f, m) in b.active_floors() {
        for l in &m.links {
            let LinkKind::Elevator { id } = &l.kind else { continue };
            let i = match out.iter().position(|e| &e.id == id) {
                Some(i) => i,
                None => {
                    out.push(Elevator {
                        id: id.clone(),
                        floor: f,
                        moving: None,
                        open_until: 0,
                        calls: Vec::new(),
                        cabins: Vec::new(),
                        doors: Vec::new(),
                        doors_open: false,
                    });
                    out.len() - 1
                }
            };
            out[i].cabins.push((f, l.area));
            // Door tiles: elevator doors right next to the cabin.
            let a = l.area;
            for y in a.y - 1..=a.y + a.h {
                for x in a.x - 1..=a.x + a.w {
                    if m.tile_type(x, y) == Some("elevator_door") {
                        out[i].doors.push((f, Tile { x, y }));
                    }
                }
            }
        }
    }
    out
}

impl Elevator {
    pub fn is_open_at(&self, floor: u8, tick: u32) -> bool {
        self.moving.is_none() && self.floor == floor && tick < self.open_until
    }

    pub fn in_cabin(&self, floor: u8, pos: Pos) -> bool {
        let (tx, ty) = pos.tile();
        self.cabins.iter().any(|(f, r)| *f == floor && r.contains(tx, ty))
    }

    pub fn door_in_reach(&self, body: &Body) -> bool {
        self.door_distance(body).is_some()
    }

    /// Squared distance to the nearest of this lift's doors on the body's
    /// floor, if within reach of the call button.
    pub fn door_distance(&self, body: &Body) -> Option<i32> {
        self.doors
            .iter()
            .filter(|(f, _)| *f == body.floor)
            .map(|(_, t)| {
                let c = Pos::tile_center(t.x, t.y);
                (c.x - body.pos.x).pow(2) + (c.y - body.pos.y).pow(2)
            })
            .filter(|&d| d <= CALL_RADIUS * CALL_RADIUS)
            .min()
    }

    /// Target floor while moving, else the next call.
    pub fn heading(&self) -> Option<u8> {
        self.moving.map(|m| m.0).or_else(|| self.calls.first().copied())
    }

    /// Call button on `floor`.
    pub fn call(&mut self, floor: u8, tick: u32) -> &'static str {
        if self.moving.is_none() && self.floor == floor {
            self.open_until = self.open_until.max(tick + DOOR_OPEN_TICKS);
            return lines::HERE;
        }
        if self.heading() == Some(floor) || self.calls.contains(&floor) {
            return lines::ON_ITS_WAY;
        }
        self.calls.push(floor);
        lines::CALLED
    }

    /// Button inside the cabin (standing on `floor`): ride to the next floor
    /// (with two floors: the other one). Returns the target.
    /// Floors this lift stops at (has a cabin on), bottom up.
    pub fn floors(&self) -> Vec<u8> {
        let mut v: Vec<u8> = self.cabins.iter().map(|(f, _)| *f).collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// The buttons on the panel inside the cabin standing at `floor`: the
    /// other floors it stops at. Empty while riding (or not here).
    pub fn buttons(&self, floor: u8) -> Vec<u8> {
        if self.moving.is_some() || self.floor != floor {
            return Vec::new();
        }
        self.floors().into_iter().filter(|&f| f != floor).collect()
    }

    /// Floor button `target` pressed inside the cabin standing at `floor`:
    /// it goes there first, the doors close in 1 s. `None` = riding, not
    /// here, or no such floor.
    pub fn press_floor(&mut self, floor: u8, target: u8, tick: u32) -> Option<u8> {
        if !self.buttons(floor).contains(&target) {
            return None;
        }
        self.calls.retain(|&c| c != target);
        self.calls.insert(0, target);
        self.open_until = self.open_until.min(tick + CLOSE_AFTER_PRESS);
        Some(target)
    }

    /// One server tick. `doorway` = positions (floor, pos) of everyone, so
    /// the doors never close on somebody.
    pub fn tick(&mut self, tick: u32, people: &[(u8, Pos)]) -> Update {
        let mut up = Update::default();
        if let Some((target, at)) = self.moving {
            if tick >= at {
                up.arrived = Some((self.floor, target));
                self.floor = target;
                self.moving = None;
                self.open_until = tick + DOOR_OPEN_TICKS;
                self.calls.retain(|&c| c != target);
            }
        } else if tick >= self.open_until {
            let floor = self.floor;
            let blocked = self.doors_open
                && people.iter().any(|(f, p)| *f == floor && self.doors.iter().any(|(df, t)| *df == floor && touches(*p, *t)));
            let aboard = people.iter().filter(|(f, p)| *f == floor && self.in_cabin(floor, *p)).count();
            let wants_to_leave = self.calls.first().is_some_and(|&c| c != floor);
            if blocked {
                self.open_until = tick + DOORWAY_HOLD;
            } else if wants_to_leave && aboard > CAPACITY {
                // Too many people: doors stay open, nobody goes anywhere.
                self.open_until = tick + OVERLOAD_HOLD;
                up.overloaded = Some(floor);
            } else if let Some(&c) = self.calls.first() {
                if c == self.floor {
                    self.calls.remove(0);
                    self.open_until = tick + DOOR_OPEN_TICKS;
                } else {
                    let dist = (c as i32 - self.floor as i32).unsigned_abs();
                    self.moving = Some((c, tick + TRAVEL_TICKS * dist));
                }
            }
        }
        let open = self.moving.is_none() && tick < self.open_until;
        up.doors_changed = open != self.doors_open || up.arrived.is_some();
        self.doors_open = open;
        up
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::default_building_path;

    fn run(e: &mut Elevator, from: u32, to: u32, people: &[(u8, Pos)]) -> Vec<(u32, Update)> {
        (from..to).map(|t| (t, e.tick(t, people))).filter(|(_, u)| *u != Update::default()).collect()
    }

    #[test]
    fn call_wait_ride_arrive() {
        let b = Building::load(&default_building_path()).unwrap();
        let mut e = find_elevators(&b).remove(0);
        assert_eq!(e.cabins.len(), 3, "a cabin on every active floor");
        assert_eq!(e.doors.len(), 9, "3 door tiles per floor");
        assert_eq!(e.floors(), [0, 3, 4], "locked floors 1 and 2 are skipped");
        assert_eq!(e.floor, 0);
        assert!(!e.is_open_at(0, 0), "doors start closed");
        // Called from floor 4: travels there (4 storeys, 3 s each), opens.
        assert_eq!(e.call(4, 0), lines::CALLED);
        assert_eq!(e.call(4, 1), lines::ON_ITS_WAY);
        let ev = run(&mut e, 0, 400, &[]);
        let arrival = ev.iter().find(|(_, u)| u.arrived.is_some()).unwrap();
        assert_eq!(arrival.1.arrived, Some((0, 4)));
        assert_eq!(arrival.0, 4 * TRAVEL_TICKS);
        assert!(!e.is_open_at(4, 400), "and closes again after a while");
        // Called where it stands: just opens.
        assert_eq!(e.call(4, 500), lines::HERE);
        assert!(e.is_open_at(4, 501));
        // Inside: the panel offers the other floors; floor 3 pressed -> doors
        // close in 1 s, then it rides one storey down.
        assert_eq!(e.buttons(4), [0, 3]);
        assert_eq!(e.press_floor(4, 4, 510), None, "no button for the floor it stands on");
        assert_eq!(e.press_floor(4, 1, 510), None, "no button for a locked floor");
        assert_eq!(e.press_floor(4, 3, 510), Some(3));
        let ev = run(&mut e, 501, 700, &[]);
        let arrival = ev.iter().find(|(_, u)| u.arrived.is_some()).unwrap();
        assert_eq!(arrival.1.arrived, Some((4, 3)));
        assert_eq!(arrival.0, 510 + CLOSE_AFTER_PRESS + TRAVEL_TICKS);
        // Down to the ground floor: three storeys.
        e.call(3, 800);
        assert_eq!(e.press_floor(3, 0, 801), Some(0));
        let ev = run(&mut e, 800, 1100, &[]);
        let arrival = ev.iter().find(|(_, u)| u.arrived.is_some()).unwrap();
        assert_eq!(arrival.1.arrived, Some((3, 0)));
        assert_eq!(arrival.0, 801 + CLOSE_AFTER_PRESS + 3 * TRAVEL_TICKS);
    }

    #[test]
    fn two_lifts_side_by_side_each_with_its_own_doors() {
        let b = Building::load(&default_building_path()).unwrap();
        let lifts = find_elevators(&b);
        assert_eq!(lifts.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["A", "B"]);
        for e in &lifts {
            assert_eq!(e.doors.len(), 9, "{}: 3 door tiles per floor", e.id);
        }
        assert!(lifts[0].doors.iter().all(|d| !lifts[1].doors.contains(d)), "no shared doors");
        // In front of each: that one's call button is the nearer.
        let (a, c) = (&lifts[0], &lifts[1]);
        let near_a = Body::at(0, Pos::tile_center(37, 44));
        let near_b = Body::at(0, Pos::tile_center(41, 44));
        assert!(a.door_distance(&near_a).unwrap() < c.door_distance(&near_a).unwrap_or(i32::MAX));
        assert!(c.door_distance(&near_b).unwrap() < a.door_distance(&near_b).unwrap_or(i32::MAX));
        assert!(a.door_distance(&Body::at(0, Pos::tile_center(30, 50))).is_none(), "out of reach");
    }

    #[test]
    fn at_most_six_people_ride() {
        let b = Building::load(&default_building_path()).unwrap();
        let mut e = find_elevators(&b).remove(0);
        let (_, cabin) = e.cabins.iter().find(|(f, _)| *f == 0).copied().unwrap();
        assert_eq!((cabin.w, cabin.h), (3, 2), "a small cabin: 6 people is a squeeze");
        let spot = |i: i32| (0u8, Pos::tile_center(cabin.x + i % 3, cabin.y + (i / 3) % 2));
        let seven: Vec<(u8, Pos)> = (0..7).map(spot).collect();
        e.call(0, 0);
        assert_eq!(e.press_floor(0, 4, 1), Some(4));
        let ev = run(&mut e, 0, 200, &seven);
        assert!(ev.iter().any(|(_, u)| u.overloaded == Some(0)));
        assert!(ev.iter().all(|(_, u)| u.arrived.is_none()), "7 people: it doesn't leave");
        assert!(e.is_open_at(0, 199), "doors stay open");
        // One gets out: off it goes.
        let ev = run(&mut e, 200, 600, &seven[..6]);
        assert!(ev.iter().any(|(_, u)| u.arrived == Some((0, 4))));
    }

    #[test]
    fn doors_dont_close_on_someone_in_the_doorway() {
        let b = Building::load(&default_building_path()).unwrap();
        let mut e = find_elevators(&b).remove(0);
        e.call(0, 0);
        let (_, door) = e.doors[0];
        let blocker = [(0u8, Pos::tile_center(door.x, door.y))];
        run(&mut e, 0, DOOR_OPEN_TICKS + 50, &blocker);
        assert!(e.is_open_at(0, DOOR_OPEN_TICKS + 49), "still open while someone stands in the doors");
        run(&mut e, DOOR_OPEN_TICKS + 50, DOOR_OPEN_TICKS + 80, &[]);
        assert!(!e.is_open_at(0, DOOR_OPEN_TICKS + 80));
    }
}
