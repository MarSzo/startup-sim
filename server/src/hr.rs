//! The HR app on the office computer: the contract, its annexes (raises),
//! leave days and leave requests. A day on leave is spent at home - paid on
//! an employment contract, unpaid on B2B / a contract of mandate.

/// Leave days at the start, and one more per this many days worked.
pub const START_LEAVE_DAYS: u8 = 2;
pub const DAYS_PER_LEAVE_DAY: u8 = 5;
/// A worked day: at least this many minutes at the office.
pub const WORKED_DAY_MINUTES: u32 = 60;
/// Leave can be asked for the next 1..=7 days.
pub const PLAN_AHEAD: u32 = 7;
/// Requests kept (older ones drop off), annexes sent.
pub const MAX_REQUESTS: usize = 10;
pub const MAX_ANNEXES_SENT: usize = 6;
/// A paid day of leave: this many hours at the contract's rate.
pub const LEAVE_HOURS: i64 = 8;

/// `HrAction::action`.
pub mod action {
    pub const SHOW: u8 = 1;
    /// `arg` = the (player's) day.
    pub const REQUEST: u8 = 2;
    /// `arg` = the request id.
    pub const CANCEL: u8 = 3;
}

/// `LeaveRequest::status`.
pub mod status {
    pub const APPROVED: u8 = 1;
    pub const REJECTED: u8 = 2;
    pub const CANCELLED: u8 = 3;
    pub const TAKEN: u8 = 4;
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Annex {
    pub day: u32,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LeaveRequest {
    pub id: u8,
    /// The player's day (`Player::day`).
    pub day: u32,
    pub status: u8,
}

/// A player's HR file (saved with the character).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HrFile {
    pub annexes: Vec<Annex>,
    pub leave_days: u8,
    /// Days worked towards the next leave day.
    pub worked: u8,
    pub requests: Vec<LeaveRequest>,
    pub next_id: u8,
    /// Today is a day off.
    pub on_leave: bool,
}

impl HrFile {
    /// Signed: the contract is the first entry, and the starting leave.
    pub fn signed(&mut self, day: u32, text: String) {
        self.annexes.push(Annex { day, text });
        self.leave_days = self.leave_days.max(START_LEAVE_DAYS);
    }

    pub fn annex(&mut self, day: u32, text: String) {
        self.annexes.push(Annex { day, text });
    }

    /// A leave request for `day` (`today` = the player's day now): decided at
    /// once - approved if there are days left. Err = not a valid day.
    pub fn request(&mut self, today: u32, day: u32) -> Result<LeaveRequest, &'static str> {
        if day <= today || day > today + PLAN_AHEAD {
            return Err(lines::BAD_DAY);
        }
        if self.requests.iter().any(|r| r.day == day && r.status == status::APPROVED) {
            return Err(lines::ALREADY);
        }
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let approved = self.leave_days > 0;
        if approved {
            self.leave_days -= 1;
        }
        let r = LeaveRequest { id: self.next_id, day, status: if approved { status::APPROVED } else { status::REJECTED } };
        self.requests.push(r);
        if self.requests.len() > MAX_REQUESTS {
            self.requests.remove(0);
        }
        Ok(r)
    }

    /// Cancel an approved leave that hasn't started: the day comes back.
    pub fn cancel(&mut self, today: u32, id: u8) -> bool {
        let Some(r) = self.requests.iter_mut().find(|r| r.id == id && r.status == status::APPROVED && r.day > today) else {
            return false;
        };
        r.status = status::CANCELLED;
        self.leave_days = self.leave_days.saturating_add(1);
        true
    }

    /// The morning of `day`: is it a day off?
    pub fn morning(&mut self, day: u32) -> bool {
        self.on_leave = false;
        if let Some(r) = self.requests.iter_mut().find(|r| r.day == day && r.status == status::APPROVED) {
            r.status = status::TAKEN;
            self.on_leave = true;
        }
        self.on_leave
    }

    /// A day at the office done: towards the next leave day.
    pub fn worked_a_day(&mut self) {
        self.worked += 1;
        if self.worked >= DAYS_PER_LEAVE_DAY {
            self.worked = 0;
            self.leave_days = self.leave_days.saturating_add(1);
        }
    }
}

pub mod lines {
    pub const BAD_DAY: &str = "Urlop można zaplanować na najbliższe 7 dni (od jutra).";
    pub const ALREADY: &str = "Na ten dzień urlop już jest.";
    pub fn contract(title: &str, form: &str, salary: &str) -> String {
        format!("Umowa: {title}, {form}, {salary} brutto / mies.")
    }
    pub fn raise(n: usize, rate: &str, salary: &str) -> String {
        format!("Aneks nr {n}: podwyżka — stawka {rate}/h (ok. {salary} brutto / mies.)")
    }
    pub fn approved(day: u32) -> String {
        format!("Wniosek urlopowy na dzień {day}: zaakceptowany. Udanego odpoczynku!")
    }
    pub fn rejected(day: u32) -> String {
        format!("Wniosek urlopowy na dzień {day}: odrzucony — brak dni urlopu do wykorzystania.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leave_is_planned_ahead_paid_from_the_pool_and_cancellable() {
        let mut f = HrFile::default();
        f.signed(3, "Umowa".into());
        assert_eq!(f.leave_days, START_LEAVE_DAYS);
        assert_eq!(f.request(3, 3), Err(lines::BAD_DAY), "not today");
        assert_eq!(f.request(3, 11), Err(lines::BAD_DAY), "not more than a week ahead");
        let a = f.request(3, 4).unwrap();
        assert_eq!(a.status, status::APPROVED);
        assert_eq!(f.request(3, 4), Err(lines::ALREADY));
        assert_eq!(f.request(3, 5).unwrap().status, status::APPROVED);
        assert_eq!(f.request(3, 6).unwrap().status, status::REJECTED, "no days left");
        assert!(f.cancel(3, a.id));
        assert!(!f.cancel(3, a.id), "once");
        assert_eq!(f.leave_days, 1);
        // The morning of day 5: off; day 6: at work again.
        assert!(!f.morning(4));
        assert!(f.morning(5) && f.on_leave);
        assert!(!f.morning(6) && !f.on_leave);
        assert!(!f.cancel(6, 2), "taken leave can't be cancelled");
        for _ in 0..DAYS_PER_LEAVE_DAY {
            f.worked_a_day();
        }
        assert_eq!(f.leave_days, 2, "a new day after five worked");
    }
}
