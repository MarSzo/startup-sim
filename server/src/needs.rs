//! Character needs (GDD 9a, step 5): hunger, energy, stress, bladder,
//! bowels, hygiene with "dirty hands" after the toilet (washed at a sink or
//! with hand sanitizer), alcohol and health (fights).
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
/// Sobering up: 20 points a game hour (5 real minutes) - a full 100 in 25.
pub const ALCOHOL_DOWN: i32 = per_tick(25);
/// Throwing up: at this many points of alcohol (once, until sober again).
pub const VOMIT_AT: i32 = 75;
/// After throwing up, drinking on to this: asleep where they stand.
pub const PASS_OUT_AT: i32 = 100;
/// Sober enough again (below this) to throw up another time.
const VOMIT_REARM: i32 = 40;
/// Throwing up gets rid of some of it.
const VOMIT_RELIEF: i32 = 10;
/// Bowels fill slowly on their own (100 in 90 real minutes), faster after
/// a meal (half of the hunger it took away).
pub const BOWELS_UP: i32 = per_tick(90);
/// Sitting on the toilet empties full bowels in 10 s.
pub const BOWELS_RELIEF: i32 = MAX / (10 * 20);
/// Health comes back by itself: 100 in 50 real minutes (10 a game hour).
pub const HEALTH_UP: i32 = per_tick(50);
/// Knocked out: wakes up with this much health.
pub const WAKE_HEALTH: i32 = 30;
/// Enough in the bladder / bowels to go on purpose (floor, machine, mug).
pub const ON_PURPOSE: i32 = 15;

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
const MUST_POOP: i32 = 85;

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
    pub const MUST_POOP: &str = "Coś mi się kotłuje w brzuchu… Szybko do kibla!";
    pub const POOP_ACCIDENT: &str = "O nie… Za późno. I to na grubo.";
    pub const RELIEVED_BIG: &str = "Uff… lżej o kilogram.";
    pub const URINAL: &str = "Pisuar, zamek w dół…";
}

/// Something to use with E (found on the map by tile type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotKind {
    Sofa,
    Toilet,
    /// Standing: only the bladder.
    Urinal,
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
                    Some("urinal") => SpotKind::Urinal,
                    Some("ashtray") => SpotKind::Ashtray,
                    Some("fruit_bowl") => SpotKind::FruitBowl,
                    Some("sink") | Some("kitchen_sink") => SpotKind::Sink,
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
    /// Sitting on the toilet: bladder and bowels.
    Toilet,
    /// Standing at a urinal: only the bladder.
    Urinal,
    Smoking {
        until: u32,
    },
    Washing {
        until: u32,
    },
}

/// Things to tell the player (speech bubble "to self") or the room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Warn(&'static str),
    /// Bladder hit 100: said to the whole room.
    Accident,
    /// Rest finished by itself (toilet empty, cigarette out).
    RestDone(&'static str),
    /// Too much to drink: throws up right here (a puddle).
    Vomit,
    /// Drank on after throwing up: falls asleep where they stand.
    PassOut,
    /// Bowels hit 100: a pile on the floor (said to the room).
    PoopAccident,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    /// Alcohol, 0..100 points (beer +15, wine +30, a mini bottle +25).
    #[serde(default)]
    pub alcohol: i32,
    /// Has thrown up since last being (almost) sober.
    #[serde(default)]
    pub vomited: bool,
    /// Bowels, 0..100 (100 = an accident on the floor).
    #[serde(default)]
    pub bowels: i32,
    /// Health, 100 = fine, 0 = knocked out (fights).
    #[serde(default = "full")]
    pub health: i32,
    /// This visit to the toilet was a big one (for the line at the end).
    #[serde(skip)]
    big_one: bool,
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
            alcohol: 0,
            vomited: false,
            bowels: 10 * SCALE,
            health: MAX,
            big_one: false,
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
const W_MUST_POOP: u8 = 64;

fn full() -> i32 {
    MAX
}

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
        self.bowels += BOWELS_UP;
        self.health += HEALTH_UP;
        self.alcohol = (self.alcohol - ALCOHOL_DOWN).max(0);
        if self.vomited && pts(self.alcohol) < VOMIT_REARM {
            self.vomited = false;
        }
        let neglected = [
            pts(self.hunger) >= HUNGRY,
            pts(self.energy) <= TIRED,
            pts(self.bladder) >= MUST_GO,
            pts(self.hygiene) < SMELLY,
            pts(self.bowels) >= MUST_POOP,
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
                // Anything more than this tick's drift: a big one.
                self.big_one |= self.bowels > 2 * BOWELS_UP;
                self.bladder -= TOILET_RELIEF + BLADDER_UP + if self.upset { UPSET_RATE } else { 0 };
                self.bowels -= BOWELS_RELIEF + BOWELS_UP;
                if self.bladder <= 0 && self.bowels <= 0 {
                    rest = None;
                    self.upset = false; // the toilet sorts the stomach out
                    ev.push(Event::RestDone(if self.big_one { lines::RELIEVED_BIG } else { lines::RELIEVED }));
                    self.big_one = false;
                }
            }
            Some(Rest::Urinal) => {
                self.bladder -= TOILET_RELIEF + BLADDER_UP + if self.upset { UPSET_RATE } else { 0 };
                if self.bladder <= 0 {
                    rest = None;
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
            Some(Rest::Washing { until }) if tick >= until => {
                rest = None;
                self.dirty_hands = false;
                self.hygiene += 40 * SCALE;
                ev.push(Event::RestDone(lines::WASHED));
            }
            Some(Rest::Washing { .. }) | None => {}
        }
        if self.bladder >= MAX && !matches!(rest, Some(Rest::Toilet | Rest::Urinal)) {
            self.bladder = 0;
            self.stress += 30 * SCALE;
            ev.push(Event::Accident);
        }
        if self.bowels >= MAX && rest != Some(Rest::Toilet) {
            self.bowels = 0;
            self.stress += 30 * SCALE;
            self.hygiene -= 25 * SCALE;
            ev.push(Event::PoopAccident);
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
            (W_MUST_POOP, pts(self.bowels) >= MUST_POOP, pts(self.bowels) < MUST_POOP - 10, lines::MUST_POOP),
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
        for v in
            [&mut self.hunger, &mut self.energy, &mut self.stress, &mut self.bladder, &mut self.hygiene, &mut self.bowels, &mut self.health]
        {
            *v = (*v).clamp(0, MAX);
        }
    }

    /// A drink (`points` of alcohol): throws up at `VOMIT_AT`, and having
    /// thrown up, passes out at `PASS_OUT_AT`.
    pub fn drink_alcohol(&mut self, points: i32) -> Option<Event> {
        self.alcohol = (self.alcohol + points * SCALE).min(MAX);
        let level = pts(self.alcohol);
        if self.vomited && level >= PASS_OUT_AT {
            return Some(Event::PassOut);
        }
        if !self.vomited && level >= VOMIT_AT {
            self.vomited = true;
            self.alcohol -= VOMIT_RELIEF * SCALE;
            self.hygiene -= 15 * SCALE;
            self.stress += 10 * SCALE;
            self.clamp();
            return Some(Event::Vomit);
        }
        None
    }

    /// Asleep it off: wakes up with less in the blood.
    pub fn sleep_it_off(&mut self) {
        self.alcohol = self.alcohol.min(60 * SCALE);
        self.energy += 30 * SCALE;
        self.clamp();
    }

    /// Alcohol in points (0..100), for the HUD.
    pub fn alcohol_points(&self) -> u8 {
        ((self.alcohol + SCALE / 2) / SCALE).clamp(0, 100) as u8
    }

    /// How drunk it shows: 0 sober, 1 tipsy (25+), 2 drunk (50+), 3 very
    /// drunk (75+).
    pub fn drunk_tier(&self) -> u8 {
        match pts(self.alcohol) {
            l if l >= 75 => 3,
            l if l >= 50 => 2,
            l if l >= 25 => 1,
            _ => 0,
        }
    }

    /// Walks unsteadily (`sim::Body::drunk`): 1 from 50, 2 from 75.
    pub fn stagger(&self) -> u8 {
        self.drunk_tier().saturating_sub(1)
    }

    /// Breathalyser reading in thousandths of per mille (100 points = 3 ‰).
    pub fn promille_milli(&self) -> u32 {
        (self.alcohol.max(0) as i64 * 3000 / MAX as i64) as u32
    }

    /// Walks slowly: exhausted, or about to burst.
    pub fn slow(&self) -> bool {
        pts(self.energy) <= SLOW_ENERGY || pts(self.bladder) >= SLOW_BLADDER
    }

    /// A night at home: slept, had breakfast, a shower, sober and well again
    /// - the morning's state (the commute changes it a bit on the way).
    pub fn rested_at_home(&mut self) {
        *self = Needs::default();
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
        self.bowels += 10 * SCALE;
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
        // What goes in must come out: half of a meal ends up in the bowels.
        self.bowels += (-e.hunger).max(0) * SCALE / 2;
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

    /// Peeing on purpose (the floor, a machine, a mug): needs something in
    /// the bladder; empties it.
    pub fn pee_now(&mut self) -> bool {
        if pts(self.bladder) < ON_PURPOSE {
            return false;
        }
        self.bladder = 0;
        self.upset = false;
        self.dirty_hands = true;
        true
    }

    /// Pooping on the floor on purpose: needs something in the bowels.
    pub fn poop_now(&mut self) -> bool {
        if pts(self.bowels) < ON_PURPOSE {
            return false;
        }
        self.bowels = 0;
        self.dirty_hands = true;
        self.hygiene -= 10 * SCALE;
        self.clamp();
        true
    }

    /// Something was off with that drink (someone peed in it).
    pub fn disgusted(&mut self) {
        self.stress += 20 * SCALE;
        self.clamp();
    }

    /// A pill from the first-aid cabinet (`inventory::kind`).
    pub fn medicine(&mut self, kind: u8) {
        use crate::inventory::kind as k;
        match kind {
            k::PAINKILLER => {
                self.health += 20 * SCALE;
                self.alcohol -= 15 * SCALE; // the headache, at least
            }
            k::CHARCOAL => {
                self.upset = false;
                self.bowels -= 30 * SCALE;
            }
            k::VITAMIN => {
                self.energy += 10 * SCALE;
                self.stress -= 5 * SCALE;
            }
            k::PLASTER => self.health += 10 * SCALE,
            _ => {}
        }
        self.alcohol = self.alcohol.max(0);
        self.clamp();
    }

    /// Hit for `points`; true = knocked out (health at 0).
    pub fn hurt(&mut self, points: i32) -> bool {
        self.health -= points * SCALE;
        self.stress += points / 2 * SCALE;
        self.clamp();
        self.health <= 0
    }

    /// Coming round after a knockout.
    pub fn come_round(&mut self) {
        self.health = self.health.max(WAKE_HEALTH * SCALE);
        self.clamp();
    }

    pub fn bowels_points(&self) -> u8 {
        ((self.bowels + SCALE / 2) / SCALE).clamp(0, 100) as u8
    }

    pub fn health_points(&self) -> u8 {
        ((self.health + SCALE / 2) / SCALE).clamp(0, 100) as u8
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
        let mut n = Needs { bladder: 95 * SCALE, bowels: 0, ..Needs::default() };
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
        assert!(ev.contains(&Event::RestDone(lines::RELIEVED_BIG)) && !n.upset, "cured");
    }

    #[test]
    fn bowels_fill_after_meals_and_empty_only_sitting_down() {
        let mut n = Needs { bowels: 0, bladder: 50 * SCALE, ..Needs::default() };
        n.apply(crate::shop::Effect { hunger: -40, energy: 0, stress: 0, bladder: 0 });
        assert_eq!(n.bowels_points(), 20, "half of the meal");
        // The urinal: only the bladder.
        let (rest, ev) = run(&mut n, Some(Rest::Urinal), 6 * 20);
        assert_eq!(rest, None);
        assert!(ev.contains(&Event::RestDone(lines::RELIEVED)));
        assert!(n.bowels_points() >= 20 && n.points()[3] == 0);
        // Sitting: the bowels too.
        let (rest, ev) = run(&mut n, Some(Rest::Toilet), 6 * 20);
        assert_eq!((rest, n.bowels_points()), (None, 0));
        assert!(ev.contains(&Event::RestDone(lines::RELIEVED_BIG)));
        // Nothing to give: no pooping on purpose; full: an accident.
        assert!(!n.poop_now() && !n.pee_now());
        n.bowels = MAX - 10;
        let (_, ev) = run(&mut n, None, 20);
        assert!(ev.contains(&Event::PoopAccident) && n.bowels_points() < 5);
        n.bowels = 30 * SCALE;
        assert!(n.poop_now() && n.bowels_points() == 0 && n.dirty_hands);
    }

    #[test]
    fn hits_hurt_knock_out_and_health_comes_back() {
        let mut n = Needs::default();
        assert_eq!(n.health_points(), 100);
        for _ in 0..2 {
            assert!(!n.hurt(35));
        }
        assert!(n.hurt(35), "the third stab: out");
        assert_eq!(n.health_points(), 0);
        n.come_round();
        assert_eq!(n.health_points(), 30);
        run(&mut n, None, 6000); // a game hour
        assert_eq!(n.health_points(), 40);
        // Old saves (no health): full.
        let old: Needs = serde_json::from_str(
            r#"{"hunger":0,"energy":0,"stress":0,"bladder":0,"hygiene":0,"dirty_hands":false,"upset":false,"warned":0}"#,
        )
        .unwrap();
        assert_eq!((old.health_points(), old.bowels_points()), (100, 0));
    }

    #[test]
    fn alcohol_shows_staggers_and_wears_off() {
        let mut n = Needs::default();
        assert_eq!((n.drunk_tier(), n.stagger()), (0, 0));
        assert_eq!(n.drink_alcohol(30), None);
        assert_eq!((n.drunk_tier(), n.stagger()), (1, 0));
        assert_eq!(n.drink_alcohol(30), None);
        assert_eq!((n.drunk_tier(), n.stagger()), (2, 1));
        assert_eq!(n.drink_alcohol(15), Some(Event::Vomit), "75: throws up");
        assert_eq!(n.alcohol_points(), 65);
        assert_eq!(n.drink_alcohol(30), None, "95: not yet");
        assert_eq!(n.drink_alcohol(15), Some(Event::PassOut), "100 after throwing up: asleep");
        assert_eq!(n.promille_milli(), 3000);
        // 20 points a game hour (5 real minutes = 6000 ticks).
        let before = n.alcohol_points();
        run(&mut n, None, 6000);
        assert_eq!(before - n.alcohol_points(), 20);
        run(&mut n, None, 6000 * 5);
        assert_eq!(n.alcohol_points(), 0);
        assert!(!n.vomited, "sober again: could throw up again");
    }

    #[test]
    fn map_has_the_spots_and_gendered_toilets() {
        let b = Building::load(&default_building_path()).unwrap();
        let spots = find_spots(&b);
        let count = |k| spots.iter().filter(|s| s.kind == k).count();
        assert!(count(SpotKind::Sofa) >= 1 && count(SpotKind::Ashtray) >= 1 && count(SpotKind::FruitBowl) == 1);
        assert!(count(SpotKind::Sink) >= 4 && count(SpotKind::Sanitizer) >= 3);
        let toilets: Vec<_> = spots.iter().filter(|s| s.kind == SpotKind::Toilet).collect();
        assert!(toilets.iter().any(|t| t.gender.as_deref() == Some("female")));
        assert!(toilets.iter().any(|t| t.gender.as_deref() == Some("male")));
    }
}
