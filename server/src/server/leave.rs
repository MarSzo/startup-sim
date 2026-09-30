//! Going home early: E where you came in - by your own car or bike, at the
//! west end of the sidewalk (on foot), at the tram stop or the taxi stand -
//! twice (a question, then yes). Paid for the time worked, home until the
//! morning. With everybody at home in the daytime, the clock runs fast.

use crate::commute::{self, mode};
use crate::sim::{Body, Pos, TILE_UNITS};

use super::player::Stage;
use super::{Say, Server};

/// How long the "going home?" question waits for the second E (5 s).
const CONFIRM_TICKS: u32 = 100;

fn near(a: Pos, b: Pos, tiles: i32) -> bool {
    let r = tiles * TILE_UNITS;
    (a.x - b.x).pow(2) + (a.y - b.y).pow(2) <= r * r
}

impl Server {
    /// E at your way home; `None` = not there.
    pub(super) fn try_go_home(&mut self, pid: u16, body: &Body) -> Option<String> {
        let p = self.players.get(&pid)?;
        if !p.contract || !p.in_building() || body.floor != 0 || self.clock.is_night() {
            return None;
        }
        let own_vehicle = self.vehicles.iter().position(|v| v.owner == pid && v.parked() && near(v.pos, body.pos, 2));
        let at_spot = commute::home_spot(&self.building.outside, p.commute_mode).is_some_and(|(spot, r)| near(spot, body.pos, r));
        let by_vehicle = matches!(p.commute_mode, mode::CAR | mode::BIKE);
        if !(own_vehicle.is_some() || (!by_vehicle && at_spot)) {
            return None;
        }
        if self.tick >= p.home_ask_until {
            let p = self.players.get_mut(&pid)?;
            p.home_ask_until = self.tick + CONFIRM_TICKS;
            return Some(commute::lines::GO_HOME_ASK.into());
        }
        let minutes = (p.worked_ds / crate::clock::DS_PER_MIN as u64) as u32;
        // The car / bike drives off (without its owner-bound removal).
        if let Some(i) = own_vehicle {
            self.vehicles[i].depart(&self.building.outside);
        }
        self.says.push(Say::new(pid, commute::lines::went_home(minutes)));
        self.go_home(pid);
        self.clock_dirty = true;
        None
    }

    /// Everybody's at home (nobody at work, on the way or job hunting): the
    /// rest of the day passes as fast as a night.
    pub(super) fn update_fast_forward(&mut self) {
        let waiting = |p: &super::player::Player| matches!(p.stage, Stage::Home { .. });
        let all_home = !self.players.is_empty() && self.players.values().all(waiting);
        self.clock.fast =
            all_home && self.players.values().all(|p| matches!(p.stage, Stage::Home { arrive_at: None }) && p.depart_at.is_none());
        // "Skip the waiting": everybody is at home (or on the way) and asked.
        let skip = all_home && self.players.values().all(|p| p.skip_wait);
        if skip != self.clock.skip {
            self.clock.skip = skip;
            self.clock_dirty = true;
        }
    }

    /// `SkipWait`: only at home; cleared on arrival at work.
    pub(super) fn handle_skip_wait(&mut self, pid: u16) {
        let Some(p) = self.players.get_mut(&pid) else { return };
        if matches!(p.stage, Stage::Home { .. }) && !p.skip_wait {
            p.skip_wait = true;
            self.clock_dirty = true;
        }
    }
}
