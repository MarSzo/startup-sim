//! The E key: whatever is in reach (a desk, an NPC, a machine, a spot...),
//! and what the NPCs do in response.

use std::collections::HashMap;

use crate::coffee;
use crate::computer;
use crate::inventory::kind as item_kind;
use crate::lights;
use crate::npc;
use crate::sim::Body;

use super::player::refresh;
use super::{Say, Server};

impl Server {
    /// This tick's E presses, in order. NPC conversations answer with
    /// events, applied together with the NPCs' own (`apply_npc_events`).
    pub(super) fn handle_interactions(&mut self, presses: &[(u16, Body)]) -> Vec<npc::Event> {
        let mut events = Vec::new();
        for &(pid, body) in presses {
            self.interact(pid, &body, &mut events);
        }
        events
    }

    /// One E press: the first thing in reach, in priority order.
    fn interact(&mut self, pid: u16, body: &Body, events: &mut Vec<npc::Event>) {
        let Some(p) = self.players.get(&pid) else { return };
        let hands_free = p.inventory.hands_free();
        let holds_laptop = p.inventory.held_kind() == item_kind::LAPTOP;
        let lunch_waiting = self.lunch_orders.iter().any(|o| o.owner == pid && o.delivered);
        // A desk right in front of you (a laptop on it, or one in your
        // hands) wins over talking to someone further away.
        let desk_here = computer::workstation_in_reach(&self.workstations, body)
            .is_some_and(|ws| holds_laptop || self.computers.iter().any(|c| c.station == ws));
        if desk_here {
            self.use_desk_and_say(pid, body);
            return;
        }
        // The key hook / the cabinet right behind the reception desk win
        // over talking to the receptionist; the storeroom's shelves too.
        if self.use_supplies(pid, body) {
            return;
        }
        // NPCs at their post first (a porter still standing next to the
        // guest he just brought mustn't shadow the receptionist). At a shop
        // shelf only somebody right next to you (the guard walks the aisles).
        let shelf_here = crate::shop::shelf_in_reach(&self.shelves, body).is_some();
        let close = |n: &npc::Npc| super::dist2(n.body.pos, body.pos) <= crate::needs::USE_RADIUS * crate::needs::USE_RADIUS;
        let nearest = self
            .npcs
            .iter()
            .enumerate()
            .filter(|(_, n)| n.in_talk_range(body) && (!shelf_here || close(n)))
            .min_by_key(|(_, n)| (!n.is_idle(), (n.body.pos.x - body.pos.x).abs() + (n.body.pos.y - body.pos.y).abs()))
            .map(|(i, _)| i);
        if let Some(i) = nearest {
            let n = &mut self.npcs[i];
            if n.role == npc::Role::Receptionist && lunch_waiting {
                // The courier left a box for you.
                let npc_id = n.id;
                let line = self.pick_up_lunch(pid);
                self.says.push(Say::addressed(npc_id, line, pid));
            } else {
                events.extend(n.interact(&self.building, pid, body.access, hands_free));
            }
            return;
        }
        if self.use_kitchen(pid, body) || self.use_coffee_machine(pid, body) || self.use_desk_and_say(pid, body) {
            return;
        }
        if let Some(line) = self.take_treat(pid, body) {
            self.says.push(Say::new(pid, line));
        } else if let Some(s) = self.switches.iter().find(|s| lights::in_reach(s, body)).copied() {
            let on = self.lights.toggle((s.floor, s.room));
            self.sound(crate::protocol::sound::SWITCH, pid);
            self.says.push(Say::new(pid, if on { lights::lines::ON } else { lights::lines::OFF }));
        } else if let Some(said) = self.use_spot(pid, body) {
            self.says.extend(said.map(|line| Say::new(pid, line)));
        } else if self.use_elevator(pid, body) {
            // The call button, or the panel in the cabin opened.
        } else if self.show_shelf(pid, body) {
            // The shelf window opened.
        } else if let Some(line) = self.try_go_home(pid, body) {
            self.says.push(Say::new(pid, line));
        } else if self.players.get(&pid).is_some_and(|p| !matches!(p.stage, super::player::Stage::Working)) {
            // Went home just now.
        } else if let Some(line) = self.try_pickup(pid, body) {
            self.says.push(Say::new(pid, line));
        } else {
            self.search_hideout(pid, body);
        }
    }

    /// E at a desk; `false` = no desk (or nothing to do there).
    fn use_desk_and_say(&mut self, pid: u16, body: &Body) -> bool {
        let Some(said) = self.use_desk(pid, body) else { return false };
        self.says.extend(said.map(|line| Say::new(pid, line)));
        true
    }

    /// E at a coffee machine; `false` = none in reach. Coffee goes into a
    /// clean mug from the cupboard, held in your hands.
    fn use_coffee_machine(&mut self, pid: u16, body: &Body) -> bool {
        let Some(i) = coffee::machine_in_reach(&self.machines, body) else { return false };
        let Some(p) = self.players.get_mut(&pid) else { return true };
        let line = match p.inventory.held_kind() {
            item_kind::CUP => match coffee::use_machine(&mut self.machines, i, &mut p.cup, true, self.tick) {
                coffee::Outcome::Started => {
                    p.inventory.take_hands(); // the mug goes under the spout
                    refresh(p);
                    coffee::lines::BREWING
                }
                coffee::Outcome::Busy => coffee::lines::BUSY,
                coffee::Outcome::HandsFull => coffee::lines::HANDS_FULL,
            },
            item_kind::EMPTY_CUP => crate::kitchen::lines::DIRTY_MUG,
            _ if self.kitchen.is_some() => crate::kitchen::lines::NEED_MUG,
            _ => match coffee::use_machine(&mut self.machines, i, &mut p.cup, p.inventory.hands_free(), self.tick) {
                coffee::Outcome::Started => coffee::lines::BREWING,
                coffee::Outcome::Busy => coffee::lines::BUSY,
                coffee::Outcome::HandsFull => coffee::lines::HANDS_FULL,
            },
        };
        if line == coffee::lines::BREWING {
            self.sound(crate::protocol::sound::COFFEE, pid);
        }
        self.says.push(Say::new(pid, line));
        true
    }

    /// NPCs walk and react to the players in the building.
    pub(super) fn tick_npcs(&mut self) -> Vec<npc::Event> {
        let bodies: HashMap<u16, Body> = self.players.values().filter(|p| p.in_building()).map(|p| (p.id, p.body)).collect();
        let mut events = Vec::new();
        for n in &mut self.npcs {
            events.extend(n.tick(&self.building, &bodies));
        }
        events
    }

    /// Carry out what the NPCs decided.
    pub(super) fn apply_npc_events(&mut self, events: Vec<npc::Event>) {
        for e in events {
            match e {
                npc::Event::Give { player, item } => self.give_new(player, item),
                npc::Event::Take { player, item } => {
                    if let Some(p) = self.players.get_mut(&player) {
                        if p.inventory.remove_kind(item).is_some() {
                            refresh(p);
                        }
                    }
                }
                npc::Event::Say { npc, text, to } => self.says.push(Say { speaker: npc, text, to, reach: super::Reach::Room }),
                npc::Event::Caught { npc, player } => self.caught(npc, player),
                npc::Event::Arrived { npc } => {
                    self.passerby_arrived(npc); // (the cleaner: see tick_cleaning)
                }
                npc::Event::Escaped { npc, player } => self.escaped(npc, player),
                npc::Event::Meeting { npc, player } => {
                    if let Some(line) = self.start_meeting(npc, player) {
                        self.says.push(Say::addressed(npc, line, player));
                    }
                }
                npc::Event::Checkout { npc, player } => {
                    if let Some(line) = self.checkout(player) {
                        self.says.push(Say::addressed(npc, line, player));
                    }
                }
                npc::Event::Contract { player } => self.sign_contract(player),
                npc::Event::ShowContract { npc, player } => self.show_contract(npc, player),
                npc::Event::SawOut { player, .. } => self.saw_out(player),
            }
        }
    }
}
