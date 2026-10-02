//! Small talk from the staff: Pani Wiesia greets everybody coming into the
//! building (with a joke, like an auntie at a wedding), the cashier asks
//! about the hot dog when somebody comes up to the counter with goods, the
//! receptionist asks whether you've ordered lunch, and Pani Maria (the
//! cleaner) tells everybody near her about her life and the town.

use crate::npc::{self, Role};
use crate::sim::TILE_UNITS;

use super::{dist2, Say, Server};

/// Pani Wiesia greets the same person at most this often (10 minutes).
const GREET_AGAIN_TICKS: u32 = 10 * 60 * 20;
/// "At the counter": this close to the cashier (2.5 tiles); asked again
/// only after stepping further than 4 tiles away.
const COUNTER_REACH: i32 = TILE_UNITS * 5 / 2;
const COUNTER_LEFT: i32 = TILE_UNITS * 4;
/// The receptionist asks when you pass this close (4 tiles), 9:00-13:00.
const RECEPTION_REACH: i32 = TILE_UNITS * 4;
const LUNCH_ASK_FROM: u32 = 9 * 60;
const LUNCH_ASK_UNTIL: u32 = 13 * 60;
/// Pani Maria: a story for each person near her (3.5 tiles) about once a
/// minute, and never two stories closer than 12 s.
const MARIA_REACH: i32 = TILE_UNITS * 7 / 2;
const MARIA_EACH_TICKS: u32 = 60 * 20;
const MARIA_GAP_TICKS: u32 = 12 * 20;

impl Server {
    /// Room changes this tick: (player, floor, from, to). Coming into the
    /// porter's hall from outside (the draught lobby, the car park) earns a
    /// hello from Pani Wiesia - if she's at her desk.
    pub(super) fn greet_entering(&mut self, entered: &[(u16, u8, u16, u16)]) {
        let Some(porter) = self.npcs.iter().find(|n| n.role == Role::Porter && n.at_home()) else { return };
        let (pid_porter, floor, hall) = (porter.id, porter.body.floor, porter.room);
        for &(pid, f, from, to) in entered {
            if f != floor || to != hall || !self.is_way_in(f, from) {
                continue;
            }
            let tick = self.tick;
            if self.porter_greeted.get(&pid).is_some_and(|&t| tick < t + GREET_AGAIN_TICKS) {
                continue;
            }
            self.porter_greeted.insert(pid, tick);
            let line = npc::porter::hello(&self.nick_of_player(pid), self.porter_joke);
            self.porter_joke += 1;
            self.says.push(Say::addressed(pid_porter, line, pid));
        }
    }

    /// A room you come into the building from.
    fn is_way_in(&self, floor: u8, room: u16) -> bool {
        let Some(m) = self.building.floor(floor) else { return false };
        m.rooms.iter().find(|r| r.id == room).is_some_and(|r| r.outdoor || matches!(r.kind.as_str(), "entrance" | "parking"))
    }

    /// The cashier: "Jaka parówka jest, wariacie?" to whoever comes up to the
    /// counter with goods to pay for (once, until they step away).
    pub(super) fn tick_cashier(&mut self) {
        let Some(c) = self.cashier.and_then(|id| self.npcs.iter().find(|n| n.id == id)) else { return };
        let (cid, floor, at) = (c.id, c.body.floor, c.body.pos);
        let mut ask = Vec::new();
        for p in self.players.values() {
            let d = dist2(p.body.pos, at);
            let here = p.in_building() && p.body.floor == floor;
            let goods = p.inventory.items().any(|i| i.unpaid);
            if here && goods && d <= COUNTER_REACH * COUNTER_REACH {
                if !self.cashier_asked.contains(&p.id) {
                    ask.push(p.id);
                }
            } else if !here || !goods || d > COUNTER_LEFT * COUNTER_LEFT {
                self.cashier_asked.remove(&p.id);
            }
        }
        self.cashier_asked.retain(|id| self.players.contains_key(id));
        for pid in ask {
            self.cashier_asked.insert(pid);
            self.says.push(Say::addressed(cid, npc::lines::CASHIER_HOTDOG, pid));
        }
    }

    /// The receptionist: "ordered lunch yet?" to employees passing by
    /// (9:00-13:00, once a day, unless they have).
    pub(super) fn tick_reception(&mut self) {
        let minute = self.clock.minute();
        if !(LUNCH_ASK_FROM..LUNCH_ASK_UNTIL).contains(&minute) || !self.tick.is_multiple_of(10) {
            return;
        }
        let Some(r) = self.npcs.iter().find(|n| n.role == Role::Receptionist && n.at_home()) else { return };
        let (rid, floor, at) = (r.id, r.body.floor, r.body.pos);
        let day = self.clock.day;
        let ask: Vec<u16> = self
            .players
            .values()
            .filter(|p| p.contract && p.in_building() && p.body.floor == floor)
            .filter(|p| dist2(p.body.pos, at) <= RECEPTION_REACH * RECEPTION_REACH)
            .filter(|p| self.lunch_asked.get(&p.id) != Some(&day) && !self.lunch_orders.iter().any(|o| o.owner == p.id))
            .map(|p| p.id)
            .collect();
        for pid in ask {
            self.lunch_asked.insert(pid, day);
            self.says.push(Say::addressed(rid, crate::pay::lines::LUNCH, pid));
        }
    }

    /// Pani Maria talks to whoever is near her: a story about once a minute
    /// each (at her post, on her round - always).
    pub(super) fn tick_maria(&mut self) {
        if self.tick < self.maria_next || self.clock.is_night() {
            return;
        }
        let Some(m) = self.npcs.iter().find(|n| n.role == Role::Cleaner) else { return };
        let (mid, floor, at) = (m.id, m.body.floor, m.body.pos);
        let tick = self.tick;
        let listener = self
            .players
            .values()
            .filter(|p| p.in_building() && p.body.floor == floor && dist2(p.body.pos, at) <= MARIA_REACH * MARIA_REACH)
            .filter(|p| self.maria_told.get(&p.id).is_none_or(|&t| tick >= t + MARIA_EACH_TICKS))
            .map(|p| p.id)
            .min();
        let Some(pid) = listener else { return };
        self.maria_told.insert(pid, tick);
        self.maria_next = tick + MARIA_GAP_TICKS;
        let stories = crate::cleaning::lines::STORIES;
        let story = stories[self.maria_story % stories.len()];
        self.maria_story += 1;
        self.says.push(Say::addressed(mid, story, pid));
    }
}
