//! Weather (shared by the whole server): sun, clouds, rain, storm, fog.
//!
//! Changes every 1-3 game hours with plausible transitions (clouds before
//! rain, a storm only out of rain). It matters outdoors: rain and storms soak
//! you (hygiene down, stress up) unless you carry an umbrella; the sun is a
//! little relaxing. It also changes the morning commute (see `commute.rs`).

pub mod kind {
    pub const SUNNY: u8 = 1;
    pub const CLOUDY: u8 = 2;
    pub const RAIN: u8 = 3;
    pub const STORM: u8 = 4;
    pub const FOG: u8 = 5;
}

/// Game minutes between changes.
pub const CHANGE_MIN: u32 = 60;
pub const CHANGE_MAX: u32 = 180;

/// "deszcz" / "rain" ... -> kind (for `--weather`).
pub fn parse(s: &str) -> Option<u8> {
    match s.to_lowercase().as_str() {
        "sun" | "sunny" | "slonce" | "słońce" => Some(kind::SUNNY),
        "clouds" | "cloudy" | "chmury" => Some(kind::CLOUDY),
        "rain" | "deszcz" => Some(kind::RAIN),
        "storm" | "burza" => Some(kind::STORM),
        "fog" | "mgla" | "mgła" => Some(kind::FOG),
        _ => None,
    }
}

pub fn name(k: u8) -> &'static str {
    match k {
        kind::SUNNY => "słonecznie",
        kind::CLOUDY => "pochmurno",
        kind::RAIN => "deszcz",
        kind::STORM => "burza",
        kind::FOG => "mgła",
        _ => "?",
    }
}

/// Next weather: (candidate, weight) per current state.
fn transitions(k: u8) -> &'static [(u8, u32)] {
    use kind::*;
    match k {
        SUNNY => &[(SUNNY, 40), (CLOUDY, 45), (FOG, 15)],
        CLOUDY => &[(SUNNY, 35), (CLOUDY, 25), (RAIN, 35), (FOG, 5)],
        RAIN => &[(CLOUDY, 45), (RAIN, 30), (STORM, 20), (SUNNY, 5)],
        STORM => &[(RAIN, 70), (CLOUDY, 30)],
        FOG => &[(SUNNY, 50), (CLOUDY, 50)],
        _ => &[(SUNNY, 1)],
    }
}

pub fn wet(k: u8) -> bool {
    matches!(k, kind::RAIN | kind::STORM)
}

/// Needs change per server tick outdoors, in `needs::SCALE` units:
/// (hygiene, stress). Rain: -0.5 hygiene / s, storm twice that.
pub fn outdoor_effect(k: u8, umbrella: bool) -> (i32, i32) {
    use crate::needs::SCALE;
    match k {
        kind::RAIN if !umbrella => (-SCALE / 40, SCALE / 100),
        kind::STORM if !umbrella => (-SCALE / 20, SCALE / 50),
        // Even with an umbrella a storm is no fun.
        kind::STORM => (0, SCALE / 200),
        kind::SUNNY => (0, -SCALE / 200),
        _ => (0, 0),
    }
}

pub mod lines {
    pub const SOAKED: &str = "Ale leje! Przemoczenie gwarantowane.";
    pub const SOAKED_ON_THE_WAY: &str = "Po drodze złapał mnie deszcz… wszystko mokre.";
    pub const UMBRELLA: &str = "Dobrze, że mam parasol.";
}

#[derive(Debug, Clone)]
pub struct Weather {
    pub now: u8,
    /// Game minute (`Clock::total_minutes`) of the next change.
    pub next_change: u32,
}

impl Weather {
    pub fn new(now_minutes: u32) -> Weather {
        Weather { now: kind::SUNNY, next_change: now_minutes + CHANGE_MIN }
    }

    /// The same weather all the time (dev / tests).
    pub fn fixed(k: u8) -> Weather {
        Weather { now: k, next_change: u32::MAX }
    }

    /// Returns true when the weather changed.
    pub fn tick(&mut self, now_minutes: u32, rng: &mut fastrand::Rng) -> bool {
        if now_minutes < self.next_change {
            return false;
        }
        self.next_change = now_minutes + rng.u32(CHANGE_MIN..=CHANGE_MAX);
        let options = transitions(self.now);
        let total: u32 = options.iter().map(|(_, w)| w).sum();
        let mut pick = rng.u32(0..total);
        let before = self.now;
        for &(k, w) in options {
            if pick < w {
                self.now = k;
                break;
            }
            pick -= w;
        }
        self.now != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_every_one_to_three_hours_and_storms_come_out_of_rain() {
        let mut w = Weather::new(0);
        let mut rng = fastrand::Rng::with_seed(7);
        let mut seen = std::collections::HashSet::new();
        let mut prev = w.now;
        let mut changes = 0;
        for minute in 0..60 * 24 * 30 {
            let before_change = w.next_change;
            if w.tick(minute, &mut rng) {
                changes += 1;
                assert!(minute >= before_change);
                if w.now == kind::STORM {
                    assert!(matches!(prev, kind::RAIN), "storm after {prev}");
                }
            }
            if minute == before_change {
                let gap = w.next_change - minute;
                assert!((CHANGE_MIN..=CHANGE_MAX).contains(&gap));
            }
            prev = w.now;
            seen.insert(w.now);
        }
        assert_eq!(seen.len(), 5, "every kind of weather shows up in a month");
        assert!(changes > 100);
    }

    #[test]
    fn umbrella_keeps_you_dry() {
        assert!(outdoor_effect(kind::RAIN, false).0 < 0);
        assert_eq!(outdoor_effect(kind::RAIN, true), (0, 0));
        assert!(outdoor_effect(kind::SUNNY, false).1 < 0, "the sun relaxes");
    }
}
