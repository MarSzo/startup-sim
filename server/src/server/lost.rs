//! Now and then somebody lost walks up to a player outside: "Excuse me,
//! where's number 50? It's next door, isn't it?" (it's at the other end of
//! the street). The answer decides what they do.

use std::collections::HashMap;

use crate::map::Tile;
use crate::npc::{Npc, NPC_ID_BASE};
use crate::protocol::Packet;
use crate::sim::Pos;

use super::{dist2, Say, Server};

/// Checked every 30 s; a player outside gets asked with this chance, at
/// most once every 10 minutes.
const CHECK_TICKS: u32 = 30 * 20;
const CHANCE_PERCENT: u32 = 25;
const AGAIN_TICKS: u32 = 10 * 60 * 20;
/// The player has to be this close when the passer-by arrives (4 tiles),
/// and answer within 30 s.
const NEAR: i32 = crate::sim::TILE_UNITS * 4;
const ANSWER_TICKS: u32 = 30 * 20;
/// Ids of passers-by (police from +0x0F00).
const ID_BASE: u16 = NPC_ID_BASE + 0x0E00;

pub(super) mod lines {
    pub const ASK: &str = "Przepraszam, gdzie jest numer 50? To chyba tu obok?";
    pub const OTHER_END: &str = "Na drugim końcu ulicy, w tamtą stronę";
    pub const HERE: &str = "Tak, to tutaj";
    pub const DUNNO: &str = "Nie wiem, sorry";
    pub const THANKS: &str = "Aaa, na drugim końcu! Dziękuję bardzo, miłego dnia!";
    pub const OK_HERE: &str = "Dziękuję! To wchodzę.";
    pub const TRICKED: &str = "Hmm… tu jest 48. Wprowadził(a) mnie Pan/Pani w błąd!";
    pub const PITY: &str = "Szkoda… zapytam kogoś innego.";
    pub const GONE: &str = "No nic… sam(a) poszukam.";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Approaching,
    /// The question is open: (dialog id, until tick).
    Asking(u8, u32),
    /// Off to the building's door, believing it's no. 50.
    ToTheDoor,
    Leaving,
}

/// The passer-by out there right now.
pub(super) struct Lost {
    pub(super) npc: u16,
    pub(super) player: u16,
    pub(super) phase: Phase,
}

#[derive(Default)]
pub(super) struct Passersby {
    pub(super) now: Option<Lost>,
    asked: HashMap<u16, u32>,
    next_id: u16,
}

impl Server {
    /// Every 30 s: maybe somebody lost walks up to a player outside.
    pub(super) fn tick_lost(&mut self) {
        if let Some(Lost { phase: Phase::Asking(_, until), npc, player }) = self.passersby.now {
            if self.tick >= until {
                self.says.push(Say::addressed(npc, lines::GONE, player));
                self.passerby_leaves(false);
            }
        }
        if self.passersby.now.is_some() || !self.tick.is_multiple_of(CHECK_TICKS) || self.clock.is_night() {
            return;
        }
        let tick = self.tick;
        let outside: Vec<u16> = self
            .players
            .values()
            .filter(|p| p.in_building() && p.riding.is_none() && p.body.floor == 0 && self.outdoor_rooms.contains(&(0, p.room)))
            .filter(|p| self.passersby.asked.get(&p.id).is_none_or(|&t| tick >= t + AGAIN_TICKS))
            .map(|p| p.id)
            .collect();
        let Some(&pid) = outside.get(self.rng.usize(..outside.len().max(1))) else { return };
        if self.rng.u32(0..100) >= CHANCE_PERCENT {
            return;
        }
        self.send_passerby(pid);
    }

    /// A passer-by from the west end of the sidewalk, walking up to `pid`.
    pub(super) fn send_passerby(&mut self, pid: u16) {
        let Some(target) = self.players.get(&pid).map(|p| p.body.pos.tile()) else { return };
        let start = self.building.outside.walk_arrival;
        let id = ID_BASE + self.passersby.next_id % 0x100;
        self.passersby.next_id = self.passersby.next_id.wrapping_add(1);
        let mut n = Npc::passerby(&self.building, id, Pos::tile_center(start.x, start.y));
        if !n.go_to(&self.building, (0, Tile { x: target.0, y: target.1 })) {
            return;
        }
        self.npcs.push(n);
        self.passersby.asked.insert(pid, self.tick);
        self.passersby.now = Some(Lost { npc: id, player: pid, phase: Phase::Approaching });
    }

    /// The passer-by got where they were going (`Event::Arrived`); false if
    /// `npc` isn't them.
    pub(super) fn passerby_arrived(&mut self, npc: u16) -> bool {
        let Some(Lost { npc: id, player, phase }) = self.passersby.now else { return false };
        if id != npc {
            return false;
        }
        match phase {
            Phase::Approaching => {
                let at = self.npcs.iter().find(|n| n.id == id).map(|n| (n.body.floor, n.body.pos));
                let near = self
                    .players
                    .get(&player)
                    .zip(at)
                    .is_some_and(|(p, (f, pos))| p.in_building() && p.body.floor == f && dist2(p.body.pos, pos) <= NEAR * NEAR);
                if !near {
                    self.passerby_leaves(false);
                    return true;
                }
                let Some(p) = self.players.get_mut(&player) else { return true };
                let dialog = p.next_dialog();
                let addr = p.addr;
                self.says.push(Say::addressed(id, lines::ASK, player));
                let options = vec![lines::OTHER_END.into(), lines::HERE.into(), lines::DUNNO.into()];
                self.send(addr, &Packet::Dialog { id: dialog, npc: id, text: lines::ASK.into(), options });
                self.passersby.now = Some(Lost { npc: id, player, phase: Phase::Asking(dialog, self.tick + ANSWER_TICKS) });
            }
            Phase::ToTheDoor => {
                self.says.push(Say::addressed(id, lines::TRICKED, player));
                self.passerby_leaves(true);
            }
            Phase::Leaving => {
                self.npcs.retain(|n| n.id != id);
                self.passersby.now = None;
            }
            Phase::Asking(..) => {}
        }
        true
    }

    /// The answer to "where's no. 50?"; false if not that question.
    pub(super) fn answer_lost(&mut self, pid: u16, dialog: u8, choice: u8) -> bool {
        let Some(Lost { npc, player, phase: Phase::Asking(id, _) }) = self.passersby.now else { return false };
        if player != pid || id != dialog {
            return false;
        }
        if let Some(addr) = self.players.get(&pid).map(|p| p.addr) {
            self.send(addr, &Packet::Dialog { id: 0, npc, text: String::new(), options: Vec::new() });
        }
        match choice {
            0 => {
                self.says.push(Say::addressed(npc, lines::THANKS, pid));
                self.passerby_leaves(true);
            }
            1 => {
                self.says.push(Say::addressed(npc, lines::OK_HERE, pid));
                // The draught lobby's glass doors, from the sidewalk.
                let door = (0, Tile { x: 31, y: 58 });
                let walks = self.npcs.iter_mut().find(|n| n.id == npc).is_some_and(|n| n.go_to(&self.building, door));
                if walks {
                    self.passersby.now = Some(Lost { npc, player, phase: Phase::ToTheDoor });
                } else {
                    self.passerby_leaves(true);
                }
            }
            _ => {
                self.says.push(Say::addressed(npc, lines::PITY, pid));
                self.passerby_leaves(false);
            }
        }
        true
    }

    /// Off along the sidewalk: east (to no. 50) or back west.
    fn passerby_leaves(&mut self, east: bool) {
        let Some(Lost { npc, player, .. }) = self.passersby.now else { return };
        let o = &self.building.outside;
        let end = if east { Tile { x: o.width - 3, y: o.walk_home.y } } else { o.walk_home };
        let walks = self.npcs.iter_mut().find(|n| n.id == npc).is_some_and(|n| n.go_to(&self.building, (0, end)));
        if walks {
            self.passersby.now = Some(Lost { npc, player, phase: Phase::Leaving });
        } else {
            self.npcs.retain(|n| n.id != npc);
            self.passersby.now = None;
        }
    }
}
