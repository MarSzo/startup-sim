//! Going home early: E where you came in - by your own car or bike, at the
//! west end of the sidewalk (on foot), at the tram stop or the taxi stand -
//! twice (a question, then yes). Paid for the time worked, home until the
//! morning. With everybody at home in the daytime, the clock runs fast.

use std::collections::HashSet;

use crate::commute::{self, mode};
use crate::protocol::{notice, Packet};
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
    }

    /// `SkipWait` ("Pomiń czekanie", from home): a vote of everybody playing
    /// now; during one, it counts as a yes.
    pub(super) fn handle_skip_wait(&mut self, pid: u16) {
        if self.clock.skip {
            return;
        }
        if self.skip_vote.is_some() {
            self.cast_skip_vote(pid, true);
            return;
        }
        let Some(p) = self.players.get(&pid) else { return };
        if !matches!(p.stage, Stage::Home { .. }) {
            return;
        }
        let nick = p.nick.clone();
        let mut yes = HashSet::new();
        yes.insert(pid);
        self.skip_vote = Some(SkipVote { yes, no: HashSet::new(), until: self.tick + VOTE_TICKS });
        self.log(format!("* {nick} asks to skip the waiting (vote)"));
        let ask = Packet::Dialog {
            id: VOTE_ID,
            npc: pid,
            text: lines::ask(&nick),
            options: vec![lines::YES.into(), lines::NO.into()],
            items: Vec::new(),
        };
        let others: Vec<u16> = self.players.keys().copied().filter(|&o| o != pid).collect();
        for o in others {
            self.send_to(o, &ask);
        }
        self.clock_dirty = true;
        self.check_skip_vote();
    }

    /// The answer to the vote's question; false if it isn't that dialog.
    pub(super) fn answer_skip_vote(&mut self, pid: u16, dialog: u8, choice: u8) -> bool {
        if dialog != VOTE_ID {
            return false;
        }
        self.send_to(pid, &Packet::Dialog { id: 0, npc: pid, text: String::new(), options: Vec::new(), items: Vec::new() });
        self.cast_skip_vote(pid, choice == 0);
        true
    }

    fn cast_skip_vote(&mut self, pid: u16, yes: bool) {
        let Some(v) = self.skip_vote.as_mut() else { return };
        v.yes.remove(&pid);
        v.no.remove(&pid);
        if yes {
            v.yes.insert(pid);
        } else {
            v.no.insert(pid);
        }
        self.check_skip_vote();
    }

    /// Time's up: whoever didn't answer isn't for it.
    pub(super) fn tick_skip_vote(&mut self) {
        if self.skip_vote.as_ref().is_some_and(|v| self.tick >= v.until) {
            self.end_skip_vote(false);
        } else if self.skip_vote.is_some() {
            self.check_skip_vote();
        }
    }

    /// More than half of those playing now said yes: passed; it can't get
    /// there any more: failed.
    fn check_skip_vote(&mut self) {
        let Some(v) = self.skip_vote.as_ref() else { return };
        let voters = |s: &HashSet<u16>| s.iter().filter(|id| self.players.contains_key(id)).count();
        let (n, yes, no) = (self.players.len(), voters(&v.yes), voters(&v.no));
        if 2 * yes > n {
            self.end_skip_vote(true);
        } else if 2 * no >= n {
            self.end_skip_vote(false);
        }
    }

    /// Passed: everybody at work goes home (paid) and time flies to the
    /// morning (until someone's at work again).
    fn end_skip_vote(&mut self, passed: bool) {
        let Some(v) = self.skip_vote.take() else { return };
        let close = Packet::Dialog { id: 0, npc: 0, text: String::new(), options: Vec::new(), items: Vec::new() };
        let ids: Vec<u16> = self.players.keys().copied().collect();
        for &pid in &ids {
            if !v.yes.contains(&pid) && !v.no.contains(&pid) {
                self.send_to(pid, &close);
            }
            self.notify(pid, notice::INFO, if passed { lines::PASSED } else { lines::FAILED });
        }
        self.log(format!("* skip vote {}: {} for, {} against", if passed { "passed" } else { "failed" }, v.yes.len(), v.no.len()));
        if passed {
            let at_work: Vec<u16> = self.players.values().filter(|p| p.in_building()).map(|p| p.id).collect();
            for pid in at_work {
                self.go_home(pid);
            }
            self.clock.skip = true;
        }
        self.clock_dirty = true;
    }
}

/// The vote's dialog id (the breathalyser uses 200..=248).
pub const VOTE_ID: u8 = 249;
/// How long the vote stays open (30 s).
const VOTE_TICKS: u32 = 600;

/// "Skip the waiting" for everybody: who said yes / no, until when.
pub(super) struct SkipVote {
    yes: HashSet<u16>,
    no: HashSet<u16>,
    until: u32,
}

pub mod lines {
    pub const YES: &str = "Tak — do domu i do rana";
    pub const NO: &str = "Nie, pracuję dalej";
    pub const PASSED: &str = "Głosowanie przeszło: koniec dnia, wszyscy do domu — czas leci do rana.";
    pub const FAILED: &str = "Głosowanie nie przeszło — gramy dalej.";
    pub fn ask(nick: &str) -> String {
        format!("{nick} proponuje koniec dnia: wszyscy do domu (z wypłatą) i pomijamy czekanie do rana. Zgadzasz się?")
    }
}
