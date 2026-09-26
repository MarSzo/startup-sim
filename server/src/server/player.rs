//! Players: session and character state.

use std::collections::{HashSet, VecDeque};
use std::net::SocketAddr;
use std::time::Instant;

use crate::clock;
use crate::coffee::Cup;
use crate::commute;
use crate::inventory::Inventory;
use crate::needs::{Needs, Rest};
use crate::protocol::{self as proto, Profile};
use crate::recruitment::Attempt;
use crate::sim::{Body, Pos};

/// Where a connected player is in the game.
pub(super) enum Stage {
    /// At home, on the computer desktop: job portal, mail, online interview.
    /// Not in the world yet.
    Portal(Box<Desk>),
    /// Hired and in the building.
    Working,
    /// Hired, out of the building: at home for the night (`None`) or on the
    /// way to work, arriving at game minute `Some(t)` (`Clock::total_minutes`).
    Home { arrive_at: Option<u32> },
}

/// Desktop state of a candidate (GDD 9a, step 2).
#[derive(Default)]
pub(super) struct Desk {
    /// Offers applied for (shown as "applied" on the portal).
    pub(super) applied: Vec<u8>,
    /// Replies to send: (offer, due tick).
    pub(super) pending: Vec<(u8, u32)>,
    /// Offers with an interview invitation.
    pub(super) invited: Vec<u8>,
    /// Online interview in progress.
    pub(super) attempt: Option<Attempt>,
    /// Passed an interview: department, waiting for "go to the office".
    pub(super) hired: Option<u8>,
    /// Passed an interview, waiting for the founder's decision (offer).
    pub(super) awaiting: Option<u8>,
    pub(super) inbox: Vec<MailMsg>,
    pub(super) next_mail: u8,
}

pub(super) struct MailMsg {
    pub(super) id: u8,
    pub(super) from: String,
    pub(super) subject: String,
    pub(super) body: String,
    pub(super) action: u8,
    pub(super) arg: u8,
}

impl Desk {
    pub(super) fn mail(&mut self, from: &str, subject: String, body: String, action: u8, arg: u8) {
        self.next_mail = self.next_mail.wrapping_add(1).max(1);
        self.inbox.push(MailMsg { id: self.next_mail, from: from.into(), subject, body, action, arg });
        if self.inbox.len() > 12 {
            self.inbox.remove(0);
        }
    }
}

/// A conversation with a board member.
#[derive(Debug, Clone, Copy)]
pub(super) struct Talk {
    /// The meeting: world day and start minute (unique per day).
    pub(super) day: u32,
    pub(super) start: u32,
    pub(super) npc: u16,
    pub(super) id: u8,
    pub(super) good: u32,
}

pub(super) struct Player {
    pub(super) id: u16,
    /// Character from the creation screen (age, city, e-mail stay here).
    pub(super) profile: Profile,
    pub(super) stage: Stage,
    /// Department of the position the player was recruited for (0 = none).
    pub(super) department: u8,
    /// Contract signed at HR: the department is official (shown to others).
    pub(super) contract: bool,
    /// Recruitment attempts so far (numbers the attempts).
    pub(super) attempts: u8,
    /// The job offer (position) they were hired for - frees up when they leave.
    pub(super) position: Option<u8>,
    /// Coffee being brewed.
    pub(super) cup: Cup,
    /// Pockets and hands.
    pub(super) inventory: Inventory,
    /// Inventory changed: send it to the owner this tick.
    pub(super) inv_dirty: bool,
    /// Handle of the computer whose screen the player is looking at.
    pub(super) at_computer: Option<u16>,
    /// Hunger, energy, stress, bladder.
    pub(super) needs: Needs,
    /// Wallet, grosze.
    pub(super) money: i64,
    /// Personal day number (1 = looking for a job).
    pub(super) day: u32,
    /// Game deciseconds worked today (salary at 22:00).
    pub(super) worked_ds: u64,
    /// Last payday: amount (grosze) and game minutes worked.
    pub(super) last_pay: (i64, u32),
    /// How they commute (`commute::mode`); the last choice is kept.
    pub(super) commute_mode: u8,
    /// Morning departure (game minute, `Clock::total_minutes`), until they leave.
    pub(super) depart_at: Option<u32>,
    /// Riding a vehicle to work (its handle): hidden, no input.
    pub(super) riding: Option<u16>,
    /// Already told "it's pouring" (until back indoors).
    pub(super) soaked_said: bool,
    /// Left the shop with unpaid goods today (the second time = police).
    pub(super) thefts_today: u8,
    /// Already complained about the smoke in this room.
    pub(super) smoke_said: bool,
    /// Last "get out, the alarm!" reminder (tick).
    pub(super) alarm_nag: u32,
    /// Asked "going home?" - a second E before this tick confirms.
    pub(super) home_ask_until: u32,
    /// Stopped by the guard / the police until this tick (no walking).
    pub(super) held_until: u32,
    /// At home: asked to skip the waiting (`SkipWait`).
    pub(super) skip_wait: bool,
    /// Salary, grosze per game hour (raises from the CEO).
    pub(super) pay_rate: i64,
    /// World day of the last raise request (cooldown).
    pub(super) last_raise_day: Option<u32>,
    /// Talking to a board member: meeting index, NPC, dialog id, good answers.
    pub(super) talk: Option<Talk>,
    pub(super) next_dialog_id: u8,
    /// Sofa / toilet / smoke break, and where it started (moving ends it).
    pub(super) rest: Option<(Rest, u8, Pos)>,
    /// Messenger spam guard / retry dedupe.
    pub(super) last_chat_tick: Option<u32>,
    pub(super) last_chat_nonce: u32,
    pub(super) token: u32,
    pub(super) nonce: u32,
    pub(super) addr: SocketAddr,
    pub(super) nick: String,
    pub(super) body: Body,
    pub(super) room: u16,
    pub(super) flags: u8,
    pub(super) last_heard: Instant,
    pub(super) inputs: VecDeque<(u32, u8)>,
    pub(super) last_received_seq: u32,
    pub(super) last_processed_seq: u32,
    /// Ids whose PlayerInfo this client has been sent.
    pub(super) known: HashSet<u16>,
    pub(super) bytes_out: u64,
}

impl Player {
    /// A freshly connected player, standing at `body`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        id: u16,
        token: u32,
        nonce: u32,
        addr: SocketAddr,
        nick: String,
        profile: Profile,
        stage: Stage,
        body: Body,
        now: Instant,
    ) -> Player {
        Player {
            id,
            profile,
            stage,
            department: 0,
            contract: false,
            attempts: 0,
            position: None,
            cup: Cup::None,
            inventory: Inventory::default(),
            inv_dirty: true,
            at_computer: None,
            needs: Needs::default(),
            money: 0,
            day: 1,
            worked_ds: 0,
            last_pay: (0, 0),
            commute_mode: commute::mode::TRAM,
            depart_at: None,
            riding: None,
            soaked_said: false,
            thefts_today: 0,
            smoke_said: false,
            alarm_nag: 0,
            home_ask_until: 0,
            held_until: 0,
            skip_wait: false,
            pay_rate: clock::PAY_PER_MIN * 60,
            last_raise_day: None,
            talk: None,
            next_dialog_id: 0,
            rest: None,
            last_chat_tick: None,
            last_chat_nonce: 0,
            token,
            nonce,
            addr,
            nick,
            body,
            room: 0,
            flags: 0,
            last_heard: now,
            inputs: VecDeque::new(),
            last_received_seq: 0,
            last_processed_seq: 0,
            known: HashSet::new(),
            bytes_out: 0,
        }
    }

    pub(super) fn in_building(&self) -> bool {
        matches!(self.stage, Stage::Working)
    }

    /// The next dialog id (never 0: 0 closes the dialog window).
    pub(super) fn next_dialog(&mut self) -> u8 {
        self.next_dialog_id = self.next_dialog_id.wrapping_add(1).max(1);
        self.next_dialog_id
    }
}

/// What others see the player doing (one at a time, most visible first).
pub(super) fn activity(p: &Player, tick: u32) -> u8 {
    use proto::activity as a;
    if p.riding.is_some() {
        return a::RIDING;
    }
    if tick < p.held_until {
        return a::HELD;
    }
    match (p.at_computer, p.rest.map(|r| r.0)) {
        (Some(_), _) => a::COMPUTER,
        (_, Some(Rest::Toilet)) => a::TOILET,
        (_, Some(Rest::Sofa)) => a::SOFA,
        (_, Some(Rest::Smoking { .. })) => a::SMOKING,
        (_, Some(Rest::Washing { .. })) => a::WASHING,
        _ if p.cup.brewing() => a::BREWING,
        _ => a::NONE,
    }
}

/// After an inventory change: access follows the carried items.
pub(super) fn refresh(p: &mut Player) {
    p.body.access = p.inventory.access();
    p.inv_dirty = true;
}

/// Control characters out, whitespace trimmed (nicks, profile fields).
pub(super) fn clean_text(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect::<String>().trim().to_string()
}

/// A plausible e-mail: `local@domain.tld`, no whitespace.
fn is_email(e: &str) -> bool {
    let Some((local, domain)) = e.split_once('@') else { return false };
    !local.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !e.contains(char::is_whitespace)
}

/// Check and normalise a character profile. `None` = reject.
pub fn validate_profile(mut p: Profile) -> Option<Profile> {
    p.city = clean_text(&p.city);
    p.email = clean_text(&p.email).to_lowercase();
    let ok = p.gender <= proto::gender::OTHER
        && (18..=70).contains(&p.age)
        && !p.city.is_empty()
        && is_email(&p.email)
        && p.appearance.is_valid();
    ok.then_some(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Appearance;

    fn good() -> Profile {
        Profile { gender: 0, age: 27, city: " Łódź ".into(), email: "Ola@Poczta.PL".into(), appearance: Appearance::default() }
    }

    #[test]
    fn profile_is_normalised() {
        let p = validate_profile(good()).unwrap();
        assert_eq!((p.city.as_str(), p.email.as_str()), ("Łódź", "ola@poczta.pl"));
    }

    #[test]
    fn bad_profiles_are_rejected() {
        let cases: Vec<Profile> = vec![
            Profile { age: 12, ..good() },
            Profile { age: 90, ..good() },
            Profile { gender: 7, ..good() },
            Profile { city: "   ".into(), ..good() },
            Profile { email: "ola".into(), ..good() },
            Profile { email: "ola@poczta".into(), ..good() },
            Profile { email: "o la@poczta.pl".into(), ..good() },
            Profile { email: "@poczta.pl".into(), ..good() },
            Profile { appearance: Appearance { shirt: 99, ..Appearance::default() }, ..good() },
        ];
        for c in cases {
            assert!(validate_profile(c.clone()).is_none(), "{c:?}");
        }
    }
}
