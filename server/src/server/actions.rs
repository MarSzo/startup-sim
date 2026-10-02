//! R: the menu of mischief (peeing on the floor, into the coffee machine or
//! somebody's mug, pooping on the floor), its answers, and the kitchen
//! cupboard's contents (mugs, knives).

use crate::coffee;
use crate::inventory::kind as item_kind;
use crate::kitchen;
use crate::mischief::{self, lines};
use crate::protocol::{self as proto, Packet};

use super::player::refresh;
use super::{dist2, Say, Server};

/// One option of the R menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Deed {
    PeeFloor,
    PoopFloor,
    /// Into this coffee machine (index in `Server::machines`).
    PeeMachine(usize),
    /// Into the mug this player holds.
    PeeCup(u16),
}

impl Server {
    /// `Action` from the client: R (menu) or X (attack).
    pub(super) fn handle_action(&mut self, id: u16, action: u8) {
        let Some(p) = self.players.get(&id) else { return };
        if !p.in_building() || self.tick < p.held_until || p.riding.is_some() {
            return;
        }
        match action {
            proto::action::MENU => self.open_menu(id),
            proto::action::ATTACK => self.attack(id),
            _ => {}
        }
    }

    /// What can be done here, as a dialog.
    fn open_menu(&mut self, id: u16) {
        let Some(p) = self.players.get(&id) else { return };
        let body = p.body;
        let mut deeds = vec![Deed::PeeFloor, Deed::PoopFloor];
        if let Some(i) = coffee::machine_in_reach(&self.machines, &body) {
            deeds.push(Deed::PeeMachine(i));
        }
        let reach = mischief::REACH;
        let mut cups: Vec<(i32, u16)> = self
            .players
            .values()
            .filter(|o| o.id != id && o.in_building() && o.body.floor == body.floor)
            .filter(|o| matches!(o.inventory.held_kind(), item_kind::COFFEE | item_kind::LATTE))
            .map(|o| (dist2(o.body.pos, body.pos), o.id))
            .filter(|&(d, _)| d <= reach * reach)
            .collect();
        cups.sort_unstable();
        deeds.extend(cups.into_iter().take(2).map(|(_, o)| Deed::PeeCup(o)));
        let mut options: Vec<String> = deeds.iter().map(|d| self.deed_label(*d)).collect();
        options.push(lines::CANCEL.into());
        let Some(p) = self.players.get_mut(&id) else { return };
        p.deeds = deeds;
        let addr = p.addr;
        let packet = Packet::Dialog { id: mischief::MENU_ID, npc: id, text: lines::MENU.into(), options };
        self.send(addr, &packet);
    }

    fn deed_label(&self, d: Deed) -> String {
        match d {
            Deed::PeeFloor => lines::PEE_FLOOR.into(),
            Deed::PoopFloor => lines::POOP_FLOOR.into(),
            Deed::PeeMachine(_) => lines::PEE_MACHINE.into(),
            Deed::PeeCup(o) => lines::pee_cup(&self.nick_of_player(o)),
        }
    }

    /// The answer to the R menu or the cupboard; false if `dialog` is neither.
    pub(super) fn answer_mischief(&mut self, pid: u16, dialog: u8, choice: u8) -> bool {
        if dialog != mischief::MENU_ID && dialog != mischief::CUPBOARD_ID {
            return false;
        }
        let Some(p) = self.players.get_mut(&pid) else { return true };
        let addr = p.addr;
        let deeds = std::mem::take(&mut p.deeds);
        let cupboard = std::mem::take(&mut p.cupboard);
        self.send(addr, &Packet::Dialog { id: 0, npc: 0, text: String::new(), options: Vec::new() });
        if dialog == mischief::CUPBOARD_ID {
            if let Some(&k) = cupboard.get(usize::from(choice)) {
                self.take_from_cupboard(pid, k);
            }
            return true;
        }
        if let Some(&deed) = deeds.get(usize::from(choice)) {
            self.do_deed(pid, deed);
        }
        true
    }

    fn do_deed(&mut self, pid: u16, deed: Deed) {
        let tick = self.tick;
        let Some(p) = self.players.get_mut(&pid) else { return };
        if tick < p.held_until {
            return;
        }
        let (floor, pos) = (p.body.floor, p.body.pos);
        let poop = deed == Deed::PoopFloor;
        let ok = if poop { p.needs.poop_now() } else { p.needs.pee_now() };
        if !ok {
            self.says.push(Say::new(pid, if poop { lines::NO_POOP } else { lines::NO_PEE }));
            return;
        }
        // Standing there for a moment (squatting, or with the zip down).
        p.held_until = tick + if poop { mischief::POOP_TICKS } else { mischief::PEE_TICKS };
        p.held_activity = if poop { proto::activity::POOPING } else { proto::activity::PEEING };
        p.rest = None;
        let nick = p.nick.clone();
        self.sounds.push((if poop { proto::sound::POOP } else { proto::sound::PEE }, floor, pos));
        let mut victim = None;
        let line = match deed {
            Deed::PeeFloor => {
                self.leave_puddle(floor, pos, proto::puddle::PEE);
                lines::DID_PEE_FLOOR
            }
            Deed::PoopFloor => {
                self.leave_puddle(floor, pos, proto::puddle::POOP);
                lines::DID_POOP_FLOOR
            }
            Deed::PeeMachine(i) => match self.machines.get_mut(i) {
                Some(m) => {
                    m.tainted = mischief::MACHINE_DOSES;
                    lines::DID_PEE_MACHINE
                }
                None => lines::TOO_LATE,
            },
            Deed::PeeCup(o) => {
                let cup = self
                    .players
                    .get_mut(&o)
                    .and_then(|v| v.inventory.hands.as_mut())
                    .filter(|i| matches!(i.kind, item_kind::COFFEE | item_kind::LATTE));
                match cup {
                    Some(cup) => {
                        cup.tainted = true;
                        victim = Some(o);
                        lines::DID_PEE_CUP
                    }
                    None => lines::TOO_LATE,
                }
            }
        };
        self.says.push(Say::new(pid, line));
        if let Some(v) = victim {
            self.says.push(Say::new(v, lines::OBLIVIOUS));
        }
        self.witnesses_react(pid, victim);
        self.log(format!("* mischief: {nick} {deed:?}"));
    }

    /// Somebody else in the room saw it (the first one says so).
    fn witnesses_react(&mut self, pid: u16, victim: Option<u16>) {
        let Some(p) = self.players.get(&pid) else { return };
        let (floor, room) = (p.body.floor, p.room);
        let witness = self
            .players
            .values()
            .find(|o| o.id != pid && Some(o.id) != victim && o.in_building() && o.body.floor == floor && o.room == room);
        if let Some(w) = witness.map(|w| w.id) {
            self.says.push(Say::addressed(w, lines::WITNESS, pid));
        }
    }

    /// E at the cupboard with free hands: a look inside (mugs, knives).
    pub(super) fn open_cupboard(&mut self, pid: u16) {
        let Some(k) = self.kitchen.as_ref() else { return };
        let (mugs, knives) = (k.mugs, k.knives);
        let mut kinds = Vec::new();
        let mut options = Vec::new();
        if mugs > 0 {
            kinds.push(item_kind::CUP);
            options.push(kitchen::lines::TAKE_MUG.to_string());
        }
        if knives > 0 {
            kinds.push(item_kind::KNIFE);
            options.push(kitchen::lines::TAKE_KNIFE.to_string());
        }
        kinds.push(0);
        options.push(kitchen::lines::CLOSE.to_string());
        let Some(p) = self.players.get_mut(&pid) else { return };
        p.cupboard = kinds;
        let addr = p.addr;
        let text = kitchen::lines::cupboard(mugs, knives);
        self.send(addr, &Packet::Dialog { id: mischief::CUPBOARD_ID, npc: pid, text, options });
    }

    fn take_from_cupboard(&mut self, pid: u16, kind: u8) {
        let Some(k) = self.kitchen.as_mut() else { return };
        let free = self.players.get(&pid).is_some_and(|p| p.inventory.hands_free());
        let line = match kind {
            _ if !free => kitchen::lines::HANDS_FULL.to_string(),
            item_kind::CUP if k.mugs > 0 => {
                k.mugs -= 1;
                let left = k.mugs;
                self.give_new(pid, item_kind::CUP);
                kitchen::lines::took_mug(left)
            }
            item_kind::CUP => kitchen::lines::NO_MUGS.to_string(),
            item_kind::KNIFE if k.knives > 0 => {
                k.knives -= 1;
                self.give_new(pid, item_kind::KNIFE);
                kitchen::lines::TOOK_KNIFE.to_string()
            }
            item_kind::KNIFE => kitchen::lines::NO_KNIVES.to_string(),
            _ => return,
        };
        self.says.push(Say::new(pid, line));
    }

    /// A knife back in the cupboard (E with it in hands).
    pub(super) fn put_knife_back(&mut self, pid: u16) {
        let Some(p) = self.players.get_mut(&pid) else { return };
        let Some(k) = self.kitchen.as_mut() else { return };
        p.inventory.take_hands();
        k.knives = (k.knives + 1).min(kitchen::KNIVES);
        refresh(p);
        self.says.push(Say::new(pid, kitchen::lines::PUT_KNIFE));
    }
}
