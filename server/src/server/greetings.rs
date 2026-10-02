//! Small talk from the staff: Pani Wiesia greets everybody coming into the
//! building (with a joke, like an auntie at a wedding), and the cashier asks
//! about the hot dog when somebody comes up to the counter with goods.

use crate::npc::{self, Role};
use crate::sim::TILE_UNITS;

use super::{dist2, Say, Server};

/// Pani Wiesia greets the same person at most this often (10 minutes).
const GREET_AGAIN_TICKS: u32 = 10 * 60 * 20;
/// "At the counter": this close to the cashier (2.5 tiles); asked again
/// only after stepping further than 4 tiles away.
const COUNTER_REACH: i32 = TILE_UNITS * 5 / 2;
const COUNTER_LEFT: i32 = TILE_UNITS * 4;

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
}
