//! The cleaner's afternoon round.

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

use crate::cleaning;
use crate::computer;
use crate::inventory::kind as item_kind;
use crate::map::Tile;
use crate::npc;

use super::items::PICKUP_RADIUS;
use super::{dist2, Say, Server};

/// When the round runs and how it's going.
#[derive(Default)]
pub(super) struct Cleaning {
    /// The round in progress.
    round: Option<Round>,
    /// World day the last round started.
    last_day: u32,
    /// Today's start (minute of the day) and the world day it was drawn for.
    start: (u32, u32),
}

/// The cleaner's afternoon round in progress.
#[derive(Default)]
struct Round {
    collected: u32,
    /// Puddles mopped up.
    mopped: u32,
    /// Mugs per player who left them.
    by_owner: HashMap<u16, u32>,
    /// Rooms she already grumbled about.
    grumbled: HashSet<(u8, u16)>,
    /// Mugs and puddles she can't get to (e.g. in a locked stall).
    unreachable: HashSet<u16>,
    /// Wiping up where the mugs stood until this tick.
    busy_until: u32,
}

impl Server {
    /// The cleaner's afternoon round: from today's start (15:00-16:00), she walks to the
    /// nearest mug or puddle left around (her floor first), collects / mops
    /// what's in reach, grumbles about messy rooms, and at the end reports.
    pub(super) fn tick_cleaning(&mut self) {
        let Some(ci) = self.npcs.iter().position(|n| n.role == npc::Role::Cleaner) else { return };
        let cleaner = self.npcs[ci].id;
        let Some(mut round) = self.cleaning.round.take() else {
            self.maybe_start_round(cleaner);
            return;
        };
        if !self.npcs[ci].is_idle() || self.tick < round.busy_until {
            self.cleaning.round = Some(round);
            return; // on her way / wiping the table
        }
        let (floor, pos) = (self.npcs[ci].body.floor, self.npcs[ci].body.pos);
        let room = self.room_of(floor, pos);
        // Collect what's in reach; a room full of mugs = a grumble.
        let in_room = self
            .dropped
            .iter()
            .filter(|d| d.item.kind == item_kind::EMPTY_CUP && d.floor == floor && self.room_of(d.floor, d.pos) == room)
            .count();
        let in_room = u32::try_from(in_room).unwrap_or(u32::MAX);
        if in_room >= cleaning::ROOM_COMPLAINT && round.grumbled.insert((floor, room)) {
            self.says.push(Say::new(cleaner, cleaning::lines::room_mess(in_room)));
        }
        let reach = PICKUP_RADIUS * 2;
        let mut picked = Vec::new();
        self.dropped.retain(|d| {
            let here = d.item.kind == item_kind::EMPTY_CUP && d.floor == floor && dist2(d.pos, pos) <= reach * reach;
            if here {
                picked.push(d.item.owner);
            }
            !here
        });
        let puddles = self.puddles.len();
        let here = |p: &super::puddles::Puddle| p.floor == floor && dist2(p.pos, pos) <= reach * reach;
        let blood = self.puddles.iter().any(|p| here(p) && p.kind == crate::protocol::puddle::BLOOD);
        self.puddles.retain(|p| !here(p));
        if self.puddles.len() < puddles {
            round.mopped += u32::try_from(puddles - self.puddles.len()).unwrap_or(u32::MAX);
            self.says.push(Say::new(cleaner, if blood { cleaning::lines::BLOOD } else { cleaning::lines::PUDDLE }));
        }
        if !picked.is_empty() || self.puddles.len() < puddles {
            round.busy_until = self.tick + cleaning::WIPE_TICKS;
            for owner in picked {
                round.collected += 1;
                *round.by_owner.entry(owner).or_default() += 1;
            }
            self.cleaning.round = Some(round);
            return;
        }
        // Next mug or puddle: this floor first, then the nearest.
        let mugs = self.dropped.iter().filter(|d| d.item.kind == item_kind::EMPTY_CUP).map(|d| (d.handle, d.floor, d.pos));
        let next = mugs
            .chain(self.puddles.iter().map(|p| (p.handle, p.floor, p.pos)))
            .filter(|(handle, _, _)| !round.unreachable.contains(handle))
            .min_by_key(|&(_, f, p)| (f != floor, dist2(p, pos)));
        match next {
            Some((handle, f, p)) => {
                let (x, y) = p.tile();
                if !self.npcs[ci].go_to(&self.building, (f, Tile { x, y })) {
                    round.unreachable.insert(handle);
                }
                self.cleaning.round = Some(round);
            }
            None => self.finish_round(ci, round),
        }
    }

    /// Draw today's start time; start once it's time (once a day).
    fn maybe_start_round(&mut self, cleaner: u16) {
        let day = self.clock.day;
        if self.cleaning.start.0 != day {
            let spread = if self.cfg.cleaning_spread > 0 { self.rng.u32(0..self.cfg.cleaning_spread) } else { 0 };
            self.cleaning.start = (day, self.cfg.cleaning_at + spread);
        }
        if !self.clock.is_night() && self.clock.minute() >= self.cleaning.start.1 && self.cleaning.last_day != day {
            self.cleaning.last_day = day;
            self.cleaning.round = Some(Round::default());
            // She rinses the coffee machines first (whatever got in them).
            for m in &mut self.machines {
                m.tainted = 0;
            }
            self.says.push(Say::new(cleaner, cleaning::lines::START));
            self.log("* cleaning round starts");
        }
    }

    /// No mugs or puddles left: the verdict, a post for the messiest, back
    /// to her room.
    fn finish_round(&mut self, ci: usize, round: Round) {
        let cleaner = self.npcs[ci].id;
        let n = round.collected;
        let line = match n {
            0 if round.mopped > 0 => None, // not spotless: she's had her say
            0 => Some(cleaning::lines::SPOTLESS.to_string()),
            n if n < cleaning::DAY_COMPLAINT => Some(cleaning::lines::few(n)),
            n => Some(cleaning::lines::done_many(n)),
        };
        if let Some(line) = line {
            self.says.push(Say::new(cleaner, line));
        }
        if n >= cleaning::DAY_COMPLAINT {
            let record = round
                .by_owner
                .iter()
                .filter_map(|(o, k)| self.players.get(o).map(|p| (p.nick.as_str(), *k)))
                .max_by_key(|&(nick, k)| (k, Reverse(nick)));
            let text = cleaning::lines::post(n, record);
            let name = self.npcs[ci].name.clone();
            self.messenger.post_system(computer::conv::GENERAL, cleaner, &name, &text);
        }
        // The mugs go in the dishwasher, and on it goes.
        let now = self.clock.total_minutes();
        if let Some(k) = self.kitchen.as_mut() {
            k.cleaner_load(u8::try_from(n).unwrap_or(u8::MAX), now);
        }
        self.npcs[ci].return_home(&self.building);
        self.log(format!("* cleaning round done: {n} mugs, {} puddles", round.mopped));
    }
}
