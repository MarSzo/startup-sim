//! Smoking anywhere, smoke and the fire brigade (backlog): a cigarette can
//! be lit anywhere (cigarettes in hands, F). Indoors it fills the room with
//! smoke, which drifts into the neighbouring rooms and slowly clears. People
//! in a smoky room complain. Rooms with a smoke detector (`"detector": true`
//! in the map) raise the fire alarm when the smoke gets thick: everybody has
//! to leave the building, a fire engine comes, a firefighter checks the room,
//! airs it, and the smoker pays for the false alarm.

use std::collections::HashMap;

use crate::building::Building;
use crate::outside::Outside;
use crate::sim::Pos;

/// Smoke concentration of a room: 0..=MAX (amount of smoke per floor tile).
pub const MAX: u16 = 1000;
/// Smoke a burning cigarette gives off per tick (spread over the room: a
/// 250-tile office gets thick in ~10 s, a toilet stall at once).
pub const PER_TICK: u32 = 600;
/// Clears by 1 (concentration) every this many ticks.
pub const DECAY_EVERY: u32 = 5;
/// A doorway lets through the concentration difference times this many
/// tiles' worth of smoke, every `DRIFT_EVERY` ticks.
pub const DOORWAY: u32 = 3;
pub const DRIFT_EVERY: u32 = 10;
/// People notice (and complain) from this level.
pub const NOTICEABLE: u16 = 120;
/// A detector goes off at this level.
pub const ALARM: u16 = 400;
/// Fine for a false alarm (grosze): 500 zł, or what's in the wallet.
pub const FINE: i64 = 50_000;
/// Stress: breathing smoke (once per smoky spell), staying in during the alarm.
pub const SMOKY_STRESS: i32 = 5;
pub const STAYING_IN_STRESS: i32 = 3;
/// The firefighter looks around the room for this long (5 s).
pub const CHECK_TICKS: u32 = 100;
/// Reminders to leave during the alarm, every 8 s.
pub const NAG_TICKS: u32 = 160;

/// Smoke per (floor, room), drifting through doorways into neighbouring
/// rooms; the open air takes it away.
#[derive(Debug, Default)]
pub struct Smoke {
    /// Amount of smoke per room.
    amount: HashMap<(u8, u16), u32>,
    /// Floor tiles per room (its "volume").
    tiles: HashMap<(u8, u16), u32>,
    /// Rooms that share a doorway (each pair once).
    links: Vec<((u8, u16), (u8, u16))>,
    /// Open air: smoke disappears there.
    open: Vec<(u8, u16)>,
    /// Last smoker per room (to blame for the alarm): player, where.
    pub smoker: HashMap<(u8, u16), (u16, Pos)>,
}

impl Smoke {
    pub fn new(b: &Building) -> Smoke {
        let mut s = Smoke::default();
        for (f, m) in b.active_floors() {
            for r in &m.rooms {
                if r.outdoor || r.kind == "smoking" {
                    s.open.push((f, r.id));
                }
            }
            for t in m.walkable_tiles() {
                *s.tiles.entry((f, m.room_at_tile(t.x, t.y))).or_default() += 1;
            }
            for (a, c) in m.room_adjacency() {
                s.links.push(((f, a), (f, c)));
            }
        }
        s
    }

    pub fn is_open_air(&self, place: (u8, u16)) -> bool {
        self.open.contains(&place)
    }

    fn volume(&self, place: (u8, u16)) -> u32 {
        self.tiles.get(&place).copied().unwrap_or(1).max(1)
    }

    /// Somebody smokes here this tick.
    pub fn puff(&mut self, place: (u8, u16), player: u16, pos: Pos) {
        if self.is_open_air(place) {
            return;
        }
        let cap = MAX as u32 * self.volume(place);
        let a = self.amount.entry(place).or_default();
        *a = (*a + PER_TICK).min(cap);
        self.smoker.insert(place, (player, pos));
    }

    /// Drift through doorways and clear slowly.
    pub fn tick(&mut self, tick: u32) {
        if self.amount.is_empty() {
            return;
        }
        if tick.is_multiple_of(DRIFT_EVERY) {
            for i in 0..self.links.len() {
                let (a, c) = self.links[i];
                let (ca, cc) = (self.get(a) as u32, self.get(c) as u32);
                if ca == cc {
                    continue;
                }
                let (from, to, diff) = if ca > cc { (a, c, ca - cc) } else { (c, a, cc - ca) };
                let (vf, vt) = (self.volume(from), self.volume(to));
                // Never more than it takes to even them out.
                let even = if self.is_open_air(to) { diff * vf } else { diff * vf * vt / (vf + vt) };
                let flow = (diff * DOORWAY).min(even);
                if flow == 0 {
                    continue;
                }
                let f = self.amount.entry(from).or_default();
                *f = f.saturating_sub(flow);
                if !self.is_open_air(to) {
                    *self.amount.entry(to).or_default() += flow;
                }
            }
        }
        if tick.is_multiple_of(DECAY_EVERY) {
            let tiles = &self.tiles;
            for (k, a) in self.amount.iter_mut() {
                *a = a.saturating_sub(tiles.get(k).copied().unwrap_or(1).max(1));
            }
        }
        self.amount.retain(|_, a| *a > 0);
        let amount = &self.amount;
        self.smoker.retain(|k, _| amount.contains_key(k));
    }

    /// Concentration in a room.
    pub fn get(&self, place: (u8, u16)) -> u16 {
        let a = self.amount.get(&place).copied().unwrap_or(0);
        (a / self.volume(place)).min(MAX as u32) as u16
    }

    /// Rooms with smoke and their concentration.
    pub fn levels(&self) -> Vec<((u8, u16), u16)> {
        self.amount.keys().map(|&k| (k, self.get(k))).filter(|(_, l)| *l > 0).collect()
    }

    pub fn is_clear(&self) -> bool {
        self.amount.is_empty()
    }

    /// Air the room out (the firefighter opens the windows).
    pub fn clear(&mut self, place: (u8, u16)) {
        self.amount.remove(&place);
    }

    /// Levels on a floor for the client, scaled to 0..=255.
    pub fn floor_levels(&self, floor: u8) -> Vec<(u16, u8)> {
        let mut v: Vec<(u16, u8)> = self
            .levels()
            .into_iter()
            .filter(|((f, _), _)| *f == floor)
            .map(|((_, r), l)| (r, (l as u32 * 255 / MAX as u32) as u8))
            .filter(|(_, l)| *l > 0)
            .collect();
        v.sort();
        v
    }
}

/// The fire alarm in progress.
#[derive(Debug, Clone)]
pub struct Alarm {
    /// Where the detector went off.
    pub place: (u8, u16),
    /// Who smoked there (if known) and where they stood.
    pub smoker: Option<u16>,
    pub spot: Pos,
    pub truck: u16,
    pub firefighter: Option<u16>,
    /// Checking the room until this tick (after arriving).
    pub checking_until: Option<u32>,
    pub done: bool,
}

/// Where the firefighter gets off the engine; the engine's street stop.
pub fn crew_spawn(o: &Outside) -> Pos {
    Pos::tile_center(o.fire.x, o.fire.y)
}
pub fn truck_path(o: &Outside) -> Vec<Pos> {
    vec![o.street_east(), o.street(o.fire.x)]
}
pub fn truck_exit(o: &Outside) -> Pos {
    o.street_west_off(8)
}

pub mod lines {
    use crate::shop::zl;
    pub const LIT: &str = "Pstryk. Papieros zapalony.";
    pub const LIT_INSIDE: &str = "Pstryk… Jeden szybki, nikt nie zauważy.";
    pub const SMOKY: &str = "Kto tu pali?! Nie da się oddychać…";
    pub const ALARM: &str = "ALARM POŻAROWY! Proszę opuścić budynek najbliższym wyjściem!";
    pub const GET_OUT: &str = "Alarm wyje — trzeba wyjść na zewnątrz!";
    pub const ARRIVED: &str = "Straż pożarna! Gdzie się pali?";
    pub const CHECKING: &str = "Sprawdzam pomieszczenie…";
    pub fn verdict(room: &str) -> String {
        format!("Fałszywy alarm — ktoś palił papierosa w pomieszczeniu „{room}”. Wietrzymy i można wracać.")
    }
    pub fn fined(fine: i64) -> String {
        if fine == 0 {
            return "Za wywołanie fałszywego alarmu należy się kara, ale portfel pusty… Upomnienie.".into();
        }
        format!("Za fałszywy alarm płaci sprawca: {}. Palić wolno tylko w strefie palenia!", zl(fine))
    }
    pub fn post(time: &str, room: &str, smoker: Option<&str>) -> String {
        let who = smoker.map_or(String::new(), |n| format!(" Sprawca: {n}."));
        format!(
            "Komunikat administracji: o {time} czujka dymu w pomieszczeniu „{room}” uruchomiła alarm pożarowy i przyjechała \
             straż. Przyczyna: papieros.{who} Przypominamy — palimy tylko w strefie palenia przed budynkiem."
        )
    }
    pub const ALL_CLEAR: &str = "Koniec alarmu, można wracać do środka.";
    pub const SHAME: &str = "Ups… to przeze mnie ta cała akcja.";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::default_building_path;

    fn setup() -> (Building, Smoke) {
        let b = Building::load(&default_building_path()).unwrap();
        let s = Smoke::new(&b);
        (b, s)
    }

    fn room(b: &Building, f: u8, name: &str) -> (u8, u16) {
        (f, b.floor(f).unwrap().room_by_name(name).unwrap().id)
    }

    #[test]
    fn smoke_fills_a_room_drifts_next_door_and_clears() {
        let (b, mut s) = setup();
        let stall = room(&b, 4, "WC męskie");
        let bath = room(&b, 4, "Łazienka męska");
        let corridor = room(&b, 4, "Korytarz");
        for t in 0..600 {
            if t == 40 {
                assert!(s.get(stall) >= ALARM, "a stall is thick at once: {:?}", s.levels());
            }
            s.puff(stall, 7, Pos::tile_center(37, 22));
            s.tick(t);
        }
        assert!(s.get(stall) > s.get(bath) && s.get(bath) >= NOTICEABLE, "into the bathroom: {:?}", s.levels());
        assert_eq!(s.smoker.get(&stall).map(|x| x.0), Some(7));
        let mut corridor_max = 0;
        for t in 600..20_000 {
            s.tick(t);
            corridor_max = corridor_max.max(s.get(corridor));
        }
        assert!(corridor_max > 0, "and out into the corridor");
        assert!(s.is_clear(), "cleared: {:?}", s.levels());
    }

    #[test]
    fn open_air_takes_the_smoke_away_and_detectors_are_where_expected() {
        let (b, mut s) = setup();
        let outside = room(&b, 0, "Strefa palenia");
        s.puff(outside, 1, Pos::tile_center(10, 46));
        assert_eq!(s.get(outside), 0);
        let m1 = b.floor(4).unwrap();
        let has = |name: &str| m1.room_by_name(name).unwrap().detector;
        assert!(has("Korytarz") && has("Produkt / IT") && has("Zarząd"));
        assert!(!has("Łazienka męska") && !has("WC męskie") && !has("Chill room"));
        for p in truck_path(&b.outside) {
            let (x, y) = p.tile();
            assert_eq!(b.floor(0).unwrap().tile_type(x, y), Some("street"));
        }
        let (x, y) = crew_spawn(&b.outside).tile();
        assert_eq!(b.floor(0).unwrap().tile_type(x, y), Some("sidewalk"));
    }
}
