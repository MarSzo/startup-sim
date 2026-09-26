//! Character needs (GDD 9a, step 5): hunger, energy, stress, bladder, and
//! hygiene with "dirty hands" after the toilet (washed at a sink or with
//! hand sanitizer).
//!
//! Values are fixed-point (`SCALE` units per point, 0..=100 points) so the
//! per-tick drift stays an integer. Hunger, stress and bladder grow (100 =
//! bad), energy falls (0 = bad). Restoring: fruit (chill room), coffee, the
//! sofa, the toilet and a smoke break. Soft consequences: warnings, stress
//! from neglected needs, a toilet "accident", and walking slowly when
//! exhausted or desperate for the toilet (`sim::Body::slow`).

use crate::building::Building;
use crate::map::Tile;
use crate::sim::{Body, Pos, TILE_UNITS};

pub const SCALE: i32 = 10_000;
pub const MAX: i32 = 100 * SCALE;
const TICKS_PER_MIN: i32 = 60 * 20;

/// Drift per tick: a full 0..100 swing in N minutes.
const fn per_tick(minutes: i32) -> i32 {
    MAX / (minutes * TICKS_PER_MIN)
}

pub const HUNGER_UP: i32 = per_tick(25);
pub const ENERGY_DOWN: i32 = per_tick(35);
pub const BLADDER_UP: i32 = per_tick(20);
pub const HYGIENE_DOWN: i32 = per_tick(60);
/// Washing hands at a sink takes 5 s.
pub const WASH_TICKS: u32 = 5 * 20;
/// Extra stress per neglected need.
pub const STRESS_NEGLECT: i32 = per_tick(15);
/// Stress fades slowly while nothing is neglected.
pub const STRESS_CALM: i32 = per_tick(60);
pub const SOFA_ENERGY: i32 = per_tick(4);
pub const SOFA_STRESS: i32 = per_tick(5);
/// Emptying a full bladder takes 8 s.
pub const TOILET_RELIEF: i32 = MAX / (8 * 20);
pub const SMOKE_TICKS: u32 = 30 * 20;
/// Upset stomach: +1 bladder point per second on top.
pub const UPSET_RATE: i32 = SCALE / 20;
/// A whole cigarette: -25 stress.
pub const SMOKE_STRESS: i32 = 25 * SCALE / SMOKE_TICKS as i32;

/// Reach for the sofa, toilet, ashtray and fruit bowl (like the coffee machine).
pub const USE_RADIUS: i32 = TILE_UNITS * 3 / 2;

// Thresholds (points).
const HUNGRY: i32 = 70;
const TIRED: i32 = 25;
const MUST_GO: i32 = 80;
const STRESSED: i32 = 80;
const SLOW_ENERGY: i32 = 10;
const SLOW_BLADDER: i32 = 90;
/// Below this you smell (visible to others) and it stresses you.
pub const SMELLY: i32 = 25;
const UNWASHED: i32 = 35;

pub mod lines {
    pub const HUNGRY: &str = "Burczy mi w brzuchu… Może owoc z chill roomu?";
    pub const TIRED: &str = "Oczy mi się zamykają… Kawa albo sofa.";
    pub const MUST_GO: &str = "Muszę do toalety! Szybko!";
    pub const STRESSED: &str = "Zaraz wybuchnę… Potrzebuję przerwy.";
    pub const EXHAUSTED: &str = "Ledwo powłóczę nogami…";
    pub const ACCIDENT: &str = "Ups… Za późno. Nikt nie widział, prawda?";
    pub const RELIEVED: &str = "Ulga!";
    pub const SOFA: &str = "Chwila odpoczynku…";
    pub const TOILET: &str = "Zajęte!";
    pub const SMOKE: &str = "Dymek i do roboty.";
    pub const SMOKE_DONE: &str = "Dobra, wracam do roboty.";
    pub const FRUIT: &str = "Owocowe czwartki, codziennie!";
    pub const WRONG_BATHROOM: &str = "Ups… to chyba nie ta łazienka.";
    pub const HANDS_FULL: &str = "Najpierw muszę coś odłożyć.";
    pub const NOT_HUNGRY: &str = "Na razie wystarczy jedzenia.";
    pub const WASHING: &str = "Mydło, woda, 30 sekund… no dobra, 5.";
    pub const WASHED: &str = "Czyste ręce!";
    pub const SANITIZED: &str = "Psik, psik — zdezynfekowane.";
    pub const UNWASHED: &str = "Przydałoby się trochę higieny…";
    pub const YUCK: &str = "Fuj… brudnymi rękami.";
}

/// Something to use with E (found on the map by tile type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotKind {
    Sofa,
    Toilet,
    Ashtray,
    FruitBowl,
    Sink,
    Sanitizer,
}

#[derive(Debug, Clone)]
pub struct Spot {
    pub kind: SpotKind,
    pub floor: u8,
    pub tile: Tile,
    /// "female" / "male" for toilets in a gendered bathroom.
    pub gender: Option<String>,
}

pub fn find_spots(b: &Building) -> Vec<Spot> {
    let mut out = Vec::new();
    for (f, m) in b.active_floors() {
        for y in 0..m.height {
            for x in 0..m.width {
                let kind = match m.tile_type(x, y) {
                    Some("sofa") => SpotKind::Sofa,
                    Some("toilet") => SpotKind::Toilet,
                    Some("ashtray") => SpotKind::Ashtray,
                    Some("fruit_bowl") => SpotKind::FruitBowl,
                    Some("sink") => SpotKind::Sink,
                    Some("sanitizer") => SpotKind::Sanitizer,
                    _ => continue,
                };
                // A toilet in the wall belongs to the room it faces: take the
                // gender of any bathroom next to it.
                let gender = [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .filter_map(|(dx, dy)| {
                        let rid = m.room_at_tile(x + dx, y + dy);
                        m.rooms.iter().find(|r| r.id == rid).and_then(|r| r.gender.clone())
                    })
                    .next();
                out.push(Spot { kind, floor: f, tile: Tile { x, y }, gender });
            }
        }
    }
    out
}

/// Nearest spot within reach.
pub fn spot_in_reach<'a>(spots: &'a [Spot], body: &Body) -> Option<&'a Spot> {
    spots
        .iter()
        .filter(|s| s.floor == body.floor)
        .map(|s| {
            let c = Pos::tile_center(s.tile.x, s.tile.y);
            (s, (c.x - body.pos.x).pow(2) + (c.y - body.pos.y).pow(2))
        })
        .filter(|&(_, d)| d <= USE_RADIUS * USE_RADIUS)
        .min_by_key(|&(_, d)| d)
        .map(|(s, _)| s)
}

/// A resting activity; ends when the character moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rest {
    Sofa,
    Toilet,
    Smoking { until: u32 },
    Washing { until: u32 },
}

/// Things to tell the player (speech bubble "to self") or the room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Warn(&'static str),
    /// Bladder hit 100: said to the whole room.
    Accident,
    /// Rest finished by itself (toilet empty, cigarette out).
    RestDone(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Needs {
    pub hunger: i32,
    pub energy: i32,
    pub stress: i32,
    pub bladder: i32,
    /// 100 = clean, 0 = smelly.
    pub hygiene: i32,
    /// After the toilet, until washed / sanitized.
    pub dirty_hands: bool,
    /// Upset stomach (stale fruit): the bladder fills fast until the toilet.
    pub upset: bool,
    /// Warnings already given (bit per threshold), re-armed on recovery.
    warned: u8,
}

impl Default for Needs {
    fn default() -> Self {
        Needs {
            hunger: 20 * SCALE,
            energy: 90 * SCALE,
            stress: 10 * SCALE,
            bladder: 10 * SCALE,
            hygiene: 90 * SCALE,
            dirty_hands: false,
            upset: false,
            warned: 0,
        }
    }
}

const W_HUNGRY: u8 = 1;
const W_TIRED: u8 = 2;
const W_MUST_GO: u8 = 4;
const W_STRESSED: u8 = 8;
const W_EXHAUSTED: u8 = 16;
const W_UNWASHED: u8 = 32;

fn pts(v: i32) -> i32 {
    v / SCALE
}

impl Needs {
    /// One server tick. `rest` = current resting activity; returns
    /// whether it is still going and what to say.
    pub fn tick(&mut self, rest: Option<Rest>, tick: u32) -> (Option<Rest>, Vec<Event>) {
        let mut ev = Vec::new();
        let mut rest = rest;
        self.hunger += HUNGER_UP;
        // Starving drains energy twice as fast.
        self.energy -= if pts(self.hunger) >= 100 { 2 * ENERGY_DOWN } else { ENERGY_DOWN };
        self.bladder += BLADDER_UP + if self.upset { UPSET_RATE } else { 0 };
        self.hygiene -= HYGIENE_DOWN;
        let neglected = [
            pts(self.hunger) >= HUNGRY,
            pts(self.energy) <= TIRED,
            pts(self.bladder) >= MUST_GO,
            pts(self.hygiene) < SMELLY,
        ]
            .iter()
            .filter(|&&b| b)
            .count() as i32;
        self.stress += if neglected > 0 { neglected * STRESS_NEGLECT } else { -STRESS_CALM };
        match rest {
            Some(Rest::Sofa) => {
                self.energy += SOFA_ENERGY;
                self.stress -= SOFA_STRESS;
            }
            Some(Rest::Toilet) => {
                self.bladder -= TOILET_RELIEF + BLADDER_UP + if self.upset { UPSET_RATE } else { 0 };
                if self.bladder <= 0 {
                    rest = None;
                    self.upset = false; // the toilet sorts the stomach out
                    ev.push(Event::RestDone(lines::RELIEVED));
                }
            }
            Some(Rest::Smoking { until }) => {
                self.stress -= SMOKE_STRESS;
                if tick >= until {
                    rest = None;
                    ev.push(Event::RestDone(lines::SMOKE_DONE));
                }
            }
            Some(Rest::Washing { until }) => {
                if tick >= until {
                    rest = None;
                    self.dirty_hands = false;
                    self.hygiene += 40 * SCALE;
                    ev.push(Event::RestDone(lines::WASHED));
                }
            }
            None => {}
        }
        if self.bladder >= MAX && rest != Some(Rest::Toilet) {
            self.bladder = 0;
            self.stress += 30 * SCALE;
            ev.push(Event::Accident);
        }
        self.clamp();
        self.warn(&mut ev);
        (rest, ev)
    }

    fn warn(&mut self, ev: &mut Vec<Event>) {
        let checks = [
            (W_HUNGRY, pts(self.hunger) >= HUNGRY, pts(self.hunger) < HUNGRY - 10, lines::HUNGRY),
            (W_TIRED, pts(self.energy) <= TIRED, pts(self.energy) > TIRED + 10, lines::TIRED),
            (W_MUST_GO, pts(self.bladder) >= MUST_GO, pts(self.bladder) < MUST_GO - 10, lines::MUST_GO),
            (W_STRESSED, pts(self.stress) >= STRESSED, pts(self.stress) < STRESSED - 10, lines::STRESSED),
            (W_EXHAUSTED, pts(self.energy) <= SLOW_ENERGY, pts(self.energy) > SLOW_ENERGY + 5, lines::EXHAUSTED),
            (W_UNWASHED, pts(self.hygiene) <= UNWASHED, pts(self.hygiene) > UNWASHED + 10, lines::UNWASHED),
        ];
        for (bit, bad, ok, line) in checks {
            if bad && self.warned & bit == 0 {
                self.warned |= bit;
                ev.push(Event::Warn(line));
            } else if ok {
                self.warned &= !bit;
            }
        }
    }

    fn clamp(&mut self) {
        for v in [&mut self.hunger, &mut self.energy, &mut self.stress, &mut self.bladder, &mut self.hygiene] {
            *v = (*v).clamp(0, MAX);
        }
    }

    /// Walks slowly: exhausted, or about to burst.
    pub fn slow(&self) -> bool {
        pts(self.energy) <= SLOW_ENERGY || pts(self.bladder) >= SLOW_BLADDER
    }

    pub fn drink_coffee(&mut self) {
        self.energy += 25 * SCALE;
        self.bladder += 8 * SCALE;
        self.stress -= 3 * SCALE;
        self.clamp();
    }

    /// Returns true if eaten with dirty hands (yuck: stress).
    pub fn eat_fruit(&mut self) -> bool {
        self.hunger -= 20 * SCALE;
        self.energy += 3 * SCALE;
        let yuck = self.dirty_hands;
        if yuck {
            self.stress += 5 * SCALE;
        }
        self.clamp();
        yuck
    }

    /// Sitting down on the toilet.
    pub fn use_toilet(&mut self) {
        self.dirty_hands = true;
        self.hygiene -= 5 * SCALE;
        self.clamp();
    }

    /// Stale fruit: the stomach rebels - off to the toilet, quickly.
    pub fn upset_stomach(&mut self) {
        self.upset = true;
        self.bladder = self.bladder.max(70 * SCALE);
        self.stress += 5 * SCALE;
        self.clamp();
    }

    /// Hand sanitizer: clean hands at once, a little hygiene.
    pub fn sanitize(&mut self) {
        self.dirty_hands = false;
        self.hygiene += 10 * SCALE;
        self.clamp();
    }

    pub fn smelly(&self) -> bool {
        pts(self.hygiene) < SMELLY
    }

    /// Weather outdoors (per tick, `SCALE` units).
    pub fn weather(&mut self, hygiene: i32, stress: i32) {
        self.hygiene += hygiene;
        self.stress += stress;
        self.clamp();
    }

    /// Eating / drinking shop goods.
    pub fn apply(&mut self, e: crate::shop::Effect) {
        self.hunger += e.hunger * SCALE;
        self.energy += e.energy * SCALE;
        self.stress += e.stress * SCALE;
        self.bladder += e.bladder * SCALE;
        self.clamp();
    }

    /// Embarrassment (e.g. the other bathroom).
    pub fn add_stress(&mut self, points: i32) {
        self.stress += points * SCALE;
        self.clamp();
    }

    pub fn is_full(&self) -> bool {
        pts(self.hunger) < 5
    }

    /// Rounded points for the HUD: hunger, energy, stress, bladder, hygiene.
    pub fn points(&self) -> [u8; 5] {
        let r = |v: i32| ((v + SCALE / 2) / SCALE).clamp(0, 100) as u8;
        [r(self.hunger), r(self.energy), r(self.stress), r(self.bladder), r(self.hygiene)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::default_building_path;

    fn run(n: &mut Needs, rest: Option<Rest>, ticks: u32) -> (Option<Rest>, Vec<Event>) {
        let mut r = rest;
        let mut all = Vec::new();
        for t in 0..ticks {
            let (nr, ev) = n.tick(r, t);
            r = nr;
            all.extend(ev);
        }
        (r, all)
    }

    #[test]
    fn needs_drift_at_the_agreed_pace() {
        let mut n = Needs::default();
        run(&mut n, None, 5 * 60 * 20); // 5 minutes
        let [h, e, _, b, hy] = n.points();
        assert_eq!(hy, 82, "hygiene -8 in 5 min (100..0 in 60)");
        assert_eq!(h, 40, "hunger +20 in 5 min (0..100 in 25)");
        assert!((75..=76).contains(&e), "energy -14 in 5 min: {e}");
        assert_eq!(b, 35, "bladder +25 in 5 min");
    }

    #[test]
    fn warnings_come_once_and_the_bladder_has_its_limits() {
        let mut n = Needs { bladder: 79 * SCALE, ..Needs::default() };
        let (_, ev) = run(&mut n, None, 20 * 60 * 3); // -> 94
        assert_eq!(ev.iter().filter(|e| **e == Event::Warn(lines::MUST_GO)).count(), 1);
        assert!(n.slow(), "about to burst: walks slowly");
        let (_, ev) = run(&mut n, None, 20 * 60 * 2);
        assert!(ev.contains(&Event::Accident));
        assert!(!n.slow() && n.points()[3] < 10, "accident resets the bladder");
    }

    #[test]
    fn toilet_sofa_and_smoking_restore() {
        let mut n = Needs { bladder: 95 * SCALE, ..Needs::default() };
        let (rest, ev) = run(&mut n, Some(Rest::Toilet), 9 * 20);
        assert_eq!((rest, n.points()[3]), (None, 0));
        assert!(ev.contains(&Event::RestDone(lines::RELIEVED)));

        let mut n = Needs { energy: 5 * SCALE, stress: 60 * SCALE, ..Needs::default() };
        assert!(n.slow());
        run(&mut n, Some(Rest::Sofa), 60 * 20);
        assert!(n.points()[1] >= 25 && n.points()[2] <= 50, "{:?}", n.points());
        assert!(!n.slow());

        let mut n = Needs { stress: 60 * SCALE, ..Needs::default() };
        let (rest, _) = run(&mut n, Some(Rest::Smoking { until: SMOKE_TICKS }), SMOKE_TICKS + 1);
        assert_eq!(rest, None);
        assert!((34..=36).contains(&n.points()[2]), "-25 stress per cigarette: {:?}", n.points());
    }

    #[test]
    fn coffee_and_fruit() {
        let mut n = Needs { hunger: 80 * SCALE, energy: 20 * SCALE, ..Needs::default() };
        assert!(!n.eat_fruit());
        n.drink_coffee();
        assert_eq!(n.points()[..4], [60, 48, 7, 18]);
    }

    #[test]
    fn toilet_dirties_hands_washing_and_sanitizer_clean_them() {
        let mut n = Needs::default();
        n.use_toilet();
        assert!(n.dirty_hands);
        assert!(n.eat_fruit(), "yuck");
        let (rest, ev) = run(&mut n, Some(Rest::Washing { until: WASH_TICKS }), WASH_TICKS + 1);
        assert_eq!(rest, None);
        assert!(ev.contains(&Event::RestDone(lines::WASHED)) && !n.dirty_hands);
        assert_eq!(n.points()[4], 100, "washing: +40 hygiene (capped)");
        let mut n = Needs { hygiene: 20 * SCALE, dirty_hands: true, ..Needs::default() };
        assert!(n.smelly());
        n.sanitize();
        assert!(!n.dirty_hands && !n.smelly() && n.points()[4] == 30, "sanitizer: clean hands, +10 only");
    }

    #[test]
    fn stale_fruit_sends_you_running_and_the_toilet_cures_it() {
        let mut n = Needs::default();
        n.upset_stomach();
        assert!(n.points()[3] >= 70);
        let (_, ev) = run(&mut n, None, 20 * 20); // 20 s
        assert!(n.points()[3] >= 90 && n.slow(), "about to burst in ~20 s: {:?}", n.points());
        assert!(!ev.contains(&Event::Accident));
        let (_, ev) = run(&mut n, None, 20 * 20);
        assert!(ev.contains(&Event::Accident), "didn't make it");
        let mut n = Needs::default();
        n.upset_stomach();
        let (rest, ev) = run(&mut n, Some(Rest::Toilet), 20 * 20);
        assert_eq!(rest, None);
        assert!(ev.contains(&Event::RestDone(lines::RELIEVED)) && !n.upset, "cured");
    }

    #[test]
    fn map_has_the_spots_and_gendered_toilets() {
        let b = Building::load(&default_building_path()).unwrap();
        let spots = find_spots(&b);
        let count = |k| spots.iter().filter(|s| s.kind == k).count();
        assert!(count(SpotKind::Sofa) >= 1 && count(SpotKind::Ashtray) >= 1 && count(SpotKind::FruitBowl) == 1);
        assert!(count(SpotKind::Sink) >= 4 && count(SpotKind::Sanitizer) == 3);
        let toilets: Vec<_> = spots.iter().filter(|s| s.kind == SpotKind::Toilet).collect();
        assert!(toilets.iter().any(|t| t.gender.as_deref() == Some("female")));
        assert!(toilets.iter().any(|t| t.gender.as_deref() == Some("male")));
    }
}
