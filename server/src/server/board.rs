//! The board: calendar, meetings and dialogs.

use crate::board::{self, Meeting};
use crate::clock;
use crate::computer;
use crate::npc;
use crate::protocol::{self as proto, Packet};

use super::player::Talk;
use super::{Say, Server};

impl Server {
    // ------------------------------------------------------------- board

    /// The account whose calendar the player sees: the owner of the unlocked
    /// computer they sit at.
    pub(super) fn calendar_account(&self, pid: u16) -> Option<u16> {
        let h = self.players.get(&pid)?.at_computer?;
        let c = self.computers.iter().find(|c| c.handle == h && !c.locked)?;
        self.players.get(&c.owner()).filter(|o| o.contract).map(|o| o.id)
    }

    pub(super) fn calendar_packet(&self, pid: u16) -> Option<Packet> {
        let account = self.calendar_account(pid)?;
        let (day, now) = (self.clock.day, self.clock.minute());
        let mine = self
            .meetings
            .iter()
            .find(|m| m.day == day && m.owner == account && matches!(m.state, board::State::Booked | board::State::Talking(_)));
        let slots = board::slots()
            .map(|start| {
                let taken = self
                    .meetings
                    .iter()
                    .find(|m| m.day == day && m.start == start && matches!(m.state, board::State::Booked | board::State::Talking(_)));
                let state = match taken {
                    Some(m) if m.owner == account => proto::slot::MINE,
                    Some(_) => proto::slot::TAKEN,
                    None if start < now + board::BOOK_AHEAD => proto::slot::PAST,
                    None => proto::slot::FREE,
                };
                (start as u16, state)
            })
            .collect();
        Some(Packet::Calendar {
            mine_start: mine.map_or(proto::NO_TIME, |m| m.start as u16),
            mine_topic: mine.map_or(board::topic::NONE, |m| m.topic),
            slots,
        })
    }

    pub(super) fn send_calendar(&mut self, pid: u16) {
        if let Some(pk) = self.calendar_packet(pid) {
            self.send_to(pid, &pk);
        }
    }

    /// Book / change / cancel (topic 0) the account's meeting for today.
    pub(super) fn handle_calendar_book(&mut self, pid: u16, start: u32, topic: u8) {
        let Some(account) = self.calendar_account(pid) else { return };
        let (day, now) = (self.clock.day, self.clock.minute());
        let is_mine = |m: &Meeting| m.day == day && m.owner == account && m.state == board::State::Booked;
        if topic == board::topic::NONE {
            self.meetings.retain(|m| !is_mine(m));
        } else {
            let valid_slot = board::slots().any(|s| s == start) && start >= now + board::BOOK_AHEAD;
            let free = !self.meetings.iter().any(|m| {
                m.day == day && m.start == start && m.owner != account && matches!(m.state, board::State::Booked | board::State::Talking(_))
            });
            if !valid_slot || !free || board::steps(topic).is_empty() {
                self.send_calendar(pid);
                return;
            }
            self.meetings.retain(|m| !is_mine(m));
            self.meetings.push(Meeting { day, start, owner: account, topic, state: board::State::Booked });
            let who = if pid == account { String::new() } else { format!(" (wpisane przez {})", self.nick(pid)) };
            self.log(format!("* calendar: {} books {} at {}{who}", self.nick(account), board::topic_name(topic), clock::hhmm(start)));
            let nick = self.nick(account).to_string();
            let body = format!(
                "Temat: {}.\nGodzina: {} w sali zarządu (piętro 1). Drzwi otworzą się 10 min wcześniej.",
                board::topic_name(topic),
                clock::hhmm(start)
            );
            self.office_mail(&nick, "Kalendarz", &format!("Spotkanie z zarządem o {}", clock::hhmm(start)), &body);
        }
        self.send_calendar(pid);
    }

    /// Door, missed meetings, old days.
    pub(super) fn tick_meetings(&mut self) {
        let (day, now) = (self.clock.day, self.clock.minute());
        self.meetings.retain(|m| m.day + 1 >= day);
        // The board-room door lets in whoever has a meeting now.
        for p in self.players.values_mut() {
            // The founder is on the board: the door is always open for them.
            let open = self.company.founder == Some(p.id) || self.meetings.iter().any(|m| m.owner == p.id && m.door_open(day, now));
            p.body.access = p.inventory.access() | if open { crate::map::access::BOARD } else { 0 };
        }
        let ceo = self.npcs.iter().find(|n| n.role == npc::Role::Ceo).map(|n| n.id);
        let mut missed = Vec::new();
        for m in &mut self.meetings {
            if m.day == day && m.state == board::State::Booked && now > m.start + board::GRACE {
                m.state = board::State::Missed;
                missed.push((m.owner, m.start));
            }
        }
        for (owner, start) in missed {
            if let Some(p) = self.players.get_mut(&owner) {
                p.needs.add_stress(5);
            }
            if let Some(c) = ceo {
                self.says.push(Say::addressed(c, board::lines::missed(start), owner));
            }
        }
        // Leaving the board room ends the conversation.
        let board_room = self.board_room;
        let left: Vec<u16> =
            self.players.values().filter(|p| p.talk.is_some() && Some((p.body.floor, p.room)) != board_room).map(|p| p.id).collect();
        for pid in left {
            self.end_talk(pid, None);
        }
    }

    /// E at the CEO / co-founder.
    pub(super) fn start_meeting(&mut self, npc_id: u16, pid: u16) -> Option<String> {
        let role = self.npcs.iter().find(|n| n.id == npc_id)?.role;
        let who = if role == npc::Role::CoFounder { board::Who::CoFounder } else { board::Who::Ceo };
        let (day, now) = (self.clock.day, self.clock.minute());
        if self.players.get(&pid)?.talk.is_some() {
            self.send_dialog(pid);
            return None;
        }
        let Some(m) = self.meetings.iter_mut().find(|m| m.day == day && m.owner == pid && m.state == board::State::Booked) else {
            return Some(board::lines::NO_MEETING.into());
        };
        if board::who(m.topic) != who {
            let other = if who == board::Who::Ceo { "ze Wspólniczką" } else { "z Prezesem" };
            return Some(format!("Twoje spotkanie jest {other}."));
        }
        if !m.can_talk(day, now) {
            return Some(board::lines::NOT_YET.into());
        }
        m.state = board::State::Talking(0);
        let (day, start) = (m.day, m.start);
        let p = self.players.get_mut(&pid)?;
        let id = p.next_dialog();
        p.talk = Some(Talk { day, start, npc: npc_id, id, good: 0 });
        self.send_dialog(pid);
        None
    }

    /// The meeting a conversation is about.
    fn talk_meeting(&self, pid: u16, t: &Talk) -> Option<usize> {
        self.meetings.iter().position(|m| m.day == t.day && m.start == t.start && m.owner == pid)
    }

    pub(super) fn dialog_packet(&self, pid: u16) -> Option<Packet> {
        let t = self.players.get(&pid)?.talk.as_ref()?;
        let m = &self.meetings[self.talk_meeting(pid, t)?];
        let board::State::Talking(step) = m.state else { return None };
        let s = board::steps(m.topic).get(step)?;
        Some(Packet::Dialog { id: t.id, npc: t.npc, text: s.text.into(), options: s.options.iter().map(|o| o.to_string()).collect() })
    }

    pub(super) fn send_dialog(&mut self, pid: u16) {
        if let Some(pk) = self.dialog_packet(pid) {
            self.send_to(pid, &pk);
        }
    }

    pub(super) fn handle_dialog_answer(&mut self, pid: u16, dialog: u8, choice: u8) {
        if self.answer_reprimand(pid, dialog, choice) {
            return;
        }
        let Some(t) = self.players.get(&pid).and_then(|p| p.talk) else { return };
        if t.id != dialog {
            return; // stale (resend of an answered question)
        }
        let Some(mi) = self.talk_meeting(pid, &t) else { return };
        let m = self.meetings[mi].clone();
        let board::State::Talking(step) = m.state else { return };
        let steps = board::steps(m.topic);
        let Some(s) = steps.get(step) else { return };
        let choice = usize::from(choice).min(s.replies.len() - 1);
        let good = t.good + u32::from(choice == s.good);
        self.says.push(Say::addressed(t.npc, s.replies[choice], pid));
        if step + 1 < steps.len() {
            self.meetings[mi].state = board::State::Talking(step + 1);
            let Some(p) = self.players.get_mut(&pid) else { return };
            let id = p.next_dialog();
            p.talk = Some(Talk { id, good, ..t });
            self.send_dialog(pid);
            return;
        }
        let outcome = self.meeting_outcome(pid, &m, good, t.npc);
        self.end_talk(pid, Some(outcome));
    }

    /// What the meeting achieved.
    fn meeting_outcome(&mut self, pid: u16, m: &Meeting, good: u32, npc_id: u16) -> String {
        let day = self.clock.day;
        let chance_roll = self.rng.u32(0..100);
        let Some(p) = self.players.get_mut(&pid) else { return String::new() };
        match m.topic {
            board::topic::RAISE => {
                if p.last_raise_day.is_some_and(|d| day < d + board::RAISE_COOLDOWN_DAYS) {
                    return board::lines::RAISE_TOO_SOON.into();
                }
                p.last_raise_day = Some(day);
                let days_worked = p.day.saturating_sub(1);
                if chance_roll < board::raise_chance(days_worked, good > 0) {
                    p.pay_rate += board::RAISE_STEP;
                    board::lines::RAISE_YES.into()
                } else {
                    board::lines::RAISE_NO.into()
                }
            }
            board::topic::IDEA => match good {
                2 => {
                    p.needs.add_stress(-10);
                    let text = format!("Brawa dla {} za świetny pomysł na produkt! Wdrażamy.", p.nick);
                    let name = self.npcs.iter().find(|n| n.id == npc_id).map_or("Zarząd", |n| n.name.as_str());
                    self.messenger.post_system(computer::conv::GENERAL, npc_id, name, &text);
                    board::lines::IDEA_GREAT.into()
                }
                1 => {
                    p.needs.add_stress(-3);
                    board::lines::IDEA_OK.into()
                }
                _ => {
                    p.needs.add_stress(3);
                    board::lines::IDEA_BAD.into()
                }
            },
            board::topic::COMPLAINT => {
                p.needs.add_stress(-8);
                board::lines::THANKS.into()
            }
            _ => {
                p.needs.add_stress(-5);
                board::lines::THANKS.into()
            }
        }
    }

    /// Close the conversation (optionally with the last line).
    pub(super) fn end_talk(&mut self, pid: u16, last: Option<String>) {
        let Some(t) = self.players.get_mut(&pid).and_then(|p| p.talk.take()) else { return };
        if let Some(mi) = self.talk_meeting(pid, &t) {
            self.meetings[mi].state = board::State::Done;
        }
        if let Some(line) = last {
            self.says.push(Say::addressed(t.npc, line, pid));
        }
        let close = Packet::Dialog { id: 0, npc: t.npc, text: String::new(), options: Vec::new() };
        self.send_to(pid, &close);
        self.send_to(pid, &close); // tiny packet; a duplicate makes loss unlikely
    }
}
