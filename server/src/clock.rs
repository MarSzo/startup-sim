//! Game clock (shared by the whole server): days, hours, office hours.
//!
//! 1 game hour = 5 real minutes during the day. The office is open 6:00 -
//! 22:00; at 22:00 everybody in the building goes home and the night is
//! fast-forwarded (22:00 -> 6:00 in one real minute). In the morning every
//! employee gets a random arrival time between 7:00 and 10:00.
//!
//! Time is kept in game deciseconds since midnight (integer drift per tick).

/// Game deciseconds per game minute.
pub const DS_PER_MIN: u32 = 600;
pub const MIN_PER_DAY: u32 = 24 * 60;
/// Daytime: 1 game hour (36 000 ds) per 5 real minutes (6 000 ticks).
pub const DAY_DS_PER_TICK: u32 = 6;
/// Night (8 game hours) in one real minute (1 200 ticks).
pub const NIGHT_DS_PER_TICK: u32 = 8 * 60 * DS_PER_MIN / 1200;
/// Skipping the wait at home: 10 game minutes per tick (a night in ~5 s).
pub const SKIP_DS_PER_TICK: u32 = 10 * DS_PER_MIN;
pub const OPEN_MIN: u32 = 6 * 60;
pub const CLOSE_MIN: u32 = 22 * 60;
/// Morning arrivals: 7:00 - 10:00 (minutes after opening).
pub const ARRIVAL_FROM: u32 = 60;
pub const ARRIVAL_TO: u32 = 4 * 60;
/// Salary: 30 zł per game hour = 50 gr per game minute.
pub const PAY_PER_MIN: i64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    /// 22:00: office closes, everybody goes home, payday.
    Evening,
    /// 6:00: a new day.
    Morning,
}

#[derive(Debug, Clone)]
pub struct Clock {
    /// World day (1, 2, ...).
    pub day: u32,
    /// Game deciseconds since midnight.
    pub ds: u32,
    /// Daytime speed multiplier (dev / tests).
    pub scale: u32,
    /// Everybody is at home: the daytime runs at the night rate.
    pub fast: bool,
    /// Everybody at home asked to skip the waiting: faster still.
    pub skip: bool,
}

impl Clock {
    pub fn new(start_min: u32, scale: u32) -> Clock {
        Clock { day: 1, ds: (start_min % MIN_PER_DAY) * DS_PER_MIN, scale: scale.max(1), fast: false, skip: false }
    }

    pub fn minute(&self) -> u32 {
        self.ds / DS_PER_MIN
    }

    /// Minutes since day 1, 00:00 (for arrival times).
    pub fn total_minutes(&self) -> u32 {
        (self.day - 1) * MIN_PER_DAY + self.minute()
    }

    pub fn is_night(&self) -> bool {
        let m = self.minute();
        !(OPEN_MIN..CLOSE_MIN).contains(&m)
    }

    /// Game deciseconds that pass per tick right now.
    pub fn rate(&self) -> u32 {
        if self.skip {
            SKIP_DS_PER_TICK
        } else if self.is_night() || self.fast {
            NIGHT_DS_PER_TICK
        } else {
            DAY_DS_PER_TICK * self.scale
        }
    }

    /// One server tick; returns 22:00 / 6:00 when passed.
    pub fn tick(&mut self) -> Option<Transition> {
        let before = self.total_minutes();
        // Never jump over a boundary in one step (fast scales, night rate).
        let mut add = self.rate();
        let to_boundary = |ds: u32, b: u32| {
            let bd = b * DS_PER_MIN;
            if ds < bd {
                bd - ds
            } else {
                MIN_PER_DAY * DS_PER_MIN - ds + bd
            }
        };
        add = add.min(to_boundary(self.ds, CLOSE_MIN)).min(to_boundary(self.ds, OPEN_MIN));
        self.ds += add;
        if self.ds >= MIN_PER_DAY * DS_PER_MIN {
            self.ds -= MIN_PER_DAY * DS_PER_MIN;
            self.day += 1;
        }
        let after = self.total_minutes();
        if after == before {
            return None;
        }
        match self.minute() {
            m if m == CLOSE_MIN && self.ds.is_multiple_of(DS_PER_MIN) => Some(Transition::Evening),
            m if m == OPEN_MIN && self.ds.is_multiple_of(DS_PER_MIN) => Some(Transition::Morning),
            _ => None,
        }
    }
}

/// "08:05".
pub fn hhmm(minute: u32) -> String {
    format!("{:02}:{:02}", (minute / 60) % 24, minute % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_hour_is_five_minutes_and_the_night_one_minute() {
        let mut c = Clock::new(8 * 60, 1);
        for _ in 0..6000 {
            assert_eq!(c.tick(), None);
        }
        assert_eq!(hhmm(c.minute()), "09:00", "5 real minutes (6000 ticks) = 1 game hour");
        // Run to the evening: exactly one Evening, then the night flies by.
        let mut c = Clock::new(21 * 60 + 59, 1);
        let mut ticks = 0;
        let mut seen = Vec::new();
        while seen.len() < 2 {
            if let Some(t) = c.tick() {
                seen.push((t, c.day, hhmm(c.minute()), ticks));
            }
            ticks += 1;
        }
        assert_eq!((seen[0].0, seen[0].1, seen[0].2.as_str()), (Transition::Evening, 1, "22:00"));
        assert_eq!((seen[1].0, seen[1].1, seen[1].2.as_str()), (Transition::Morning, 2, "06:00"));
        let night_ticks = seen[1].3 - seen[0].3;
        assert!((1195..=1205).contains(&night_ticks), "night takes ~1 real minute: {night_ticks}");
        assert!(!c.is_night());
    }

    #[test]
    fn fast_scale_still_stops_at_the_boundaries() {
        let mut c = Clock::new(21 * 60, 1000);
        let evening = (0..200).find_map(|_| c.tick());
        assert_eq!(evening, Some(Transition::Evening));
        assert_eq!(hhmm(c.minute()), "22:00");
    }
}
