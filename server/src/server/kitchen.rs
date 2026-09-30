//! The kitchenette: the mug cupboard, the dishwasher and the fridge (E),
//! the fridge window's actions, the dishwasher cycle and the morning restock.

use crate::inventory::{kind as item_kind, Item};
use crate::kitchen::{self, action, lines};
use crate::map::Tile;
use crate::protocol::Packet;
use crate::sim::{Body, Pos};

use super::player::refresh;
use super::{Say, Server};

/// What in the kitchenette is being used.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Thing {
    Cupboard,
    Dishwasher,
    Fridge,
}

fn dist2(t: Tile, body: &Body) -> i32 {
    let c = Pos::tile_center(t.x, t.y);
    (c.x - body.pos.x).pow(2) + (c.y - body.pos.y).pow(2)
}

impl Server {
    /// E in the kitchenette; `false` = nothing of it in reach (or the
    /// coffee machine / a sink is nearer - they have their own handling).
    pub(super) fn use_kitchen(&mut self, pid: u16, body: &Body) -> bool {
        let Some(k) = &self.kitchen else { return false };
        let mut best: Option<(i32, Thing)> = None;
        for (t, thing) in [(k.cupboard, Thing::Cupboard), (k.dishwasher, Thing::Dishwasher), (k.fridge, Thing::Fridge)] {
            if k.near(t, body) {
                let d = dist2(t, body);
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, thing));
                }
            }
        }
        let Some((d, thing)) = best else { return false };
        // The machine or a spot (sink, fruit bowl, sanitizer) right in front
        // of you wins.
        let machine_nearer = self.machines.iter().any(|m| m.floor == body.floor && dist2(m.tile, body) < d);
        let spot_nearer = crate::needs::spot_in_reach(&self.spots, body).is_some_and(|s| dist2(s.tile, body) < d);
        if machine_nearer || spot_nearer {
            return false;
        }
        match thing {
            Thing::Cupboard => self.use_cupboard(pid),
            Thing::Dishwasher => self.use_dishwasher(pid),
            Thing::Fridge => {
                self.send_fridge(pid);
            }
        }
        true
    }

    fn use_cupboard(&mut self, pid: u16) {
        self.sound(crate::protocol::sound::CUPBOARD, pid);
        let Some(p) = self.players.get_mut(&pid) else { return };
        let Some(k) = self.kitchen.as_mut() else { return };
        let line = match p.inventory.held_kind() {
            item_kind::CUP => {
                p.inventory.take_hands();
                k.return_mugs(1);
                refresh(p);
                lines::PUT_MUG.to_string()
            }
            item_kind::EMPTY_CUP => lines::DIRTY_NOT_HERE.to_string(),
            _ if !p.inventory.hands_free() => lines::HANDS_FULL.to_string(),
            _ if k.mugs == 0 => lines::NO_MUGS.to_string(),
            _ => {
                k.mugs -= 1;
                let left = k.mugs;
                self.give_new(pid, item_kind::CUP);
                lines::took_mug(left)
            }
        };
        self.says.push(Say::new(pid, line));
    }

    fn use_dishwasher(&mut self, pid: u16) {
        let now = self.clock.total_minutes();
        let Some(p) = self.players.get_mut(&pid) else { return };
        let Some(k) = self.kitchen.as_mut() else { return };
        let line = if p.inventory.held_kind() == item_kind::EMPTY_CUP {
            if k.washed > 0 {
                lines::DW_UNLOAD_FIRST.to_string()
            } else if let Some(t) = k.running_until {
                lines::dw_running(t.saturating_sub(now).max(1))
            } else if k.dirty >= kitchen::DISHWASHER_CAP {
                lines::DW_FULL.to_string()
            } else {
                p.inventory.take_hands();
                refresh(p);
                k.dirty += 1;
                lines::loaded(k.dirty)
            }
        } else if k.washed > 0 {
            let n = k.washed;
            k.washed = 0;
            k.return_mugs(n);
            lines::dw_unloaded(n)
        } else if let Some(t) = k.running_until {
            lines::dw_running(t.saturating_sub(now).max(1))
        } else if k.dirty > 0 {
            k.running_until = Some(now + kitchen::WASH_MINUTES);
            if let Some(p) = self.players.get(&pid) {
                self.sounds.push((crate::protocol::sound::DISHWASHER, p.body.floor, p.body.pos));
            }
            lines::DW_STARTED.to_string()
        } else {
            lines::DW_EMPTY.to_string()
        };
        self.says.push(Say::new(pid, line));
    }

    fn fridge_packet(&self) -> Option<Packet> {
        let k = self.kitchen.as_ref()?;
        Some(Packet::Fridge {
            items: k.stored.iter().map(|i| (i.kind, i.label.clone())).collect(),
            milk: k.milk,
            water: k.water,
            juice: k.juice,
        })
    }

    fn send_fridge(&mut self, pid: u16) {
        self.sound(crate::protocol::sound::FRIDGE, pid);
        if let Some(pk) = self.fridge_packet() {
            self.send_to(pid, &pk);
        }
    }

    /// A button in the fridge window (standing at the fridge).
    pub(super) fn handle_fridge_action(&mut self, pid: u16, act: u8, arg: u8) {
        let Some(body) = self.players.get(&pid).filter(|p| p.in_building()).map(|p| p.body) else { return };
        if !self.kitchen.as_ref().is_some_and(|k| k.near(k.fridge, &body)) {
            return;
        }
        let line = match act {
            action::TAKE => self.fridge_take(pid, arg as usize),
            action::PUT => self.fridge_put(pid),
            action::TAKE_WATER | action::TAKE_JUICE => self.fridge_free_drink(pid, act),
            action::MILK => self.fridge_milk(pid),
            _ => None,
        };
        if let Some(line) = line {
            self.says.push(Say::new(pid, line));
        }
        self.send_fridge(pid);
    }

    fn fridge_take(&mut self, pid: u16, i: usize) -> Option<String> {
        let k = self.kitchen.as_mut()?;
        let p = self.players.get_mut(&pid)?;
        if i >= k.stored.len() {
            return None;
        }
        if !p.inventory.has_room() {
            return Some(lines::HANDS_FULL.into());
        }
        let item = k.stored.remove(i);
        let name = crate::inventory::display_name(item.kind).to_lowercase();
        self.give(pid, item);
        Some(format!("Wyjmuję z lodówki: {name}."))
    }

    fn fridge_put(&mut self, pid: u16) -> Option<String> {
        let k = self.kitchen.as_mut()?;
        let p = self.players.get_mut(&pid)?;
        let held = p.inventory.hands.as_ref()?;
        if held.unpaid || !kitchen::fridge_worthy(held.kind) {
            return Some(lines::NOT_FOR_FRIDGE.into());
        }
        if held.kind == item_kind::MILK {
            p.inventory.take_hands();
            refresh(p);
            k.milk = (k.milk + kitchen::MILK_PER_CARTON).min(kitchen::MILK_MAX);
            return Some(lines::carton(k.milk));
        }
        if k.stored.len() >= kitchen::FRIDGE_SLOTS {
            return Some(lines::FRIDGE_FULL.into());
        }
        let nick = p.nick.clone();
        let mut item: Item = p.inventory.take_hands()?;
        refresh(p);
        let name = crate::inventory::display_name(item.kind).to_string();
        item.label = format!("{name} ({nick})");
        k.stored.push(item);
        Some(lines::stored(&name))
    }

    fn fridge_free_drink(&mut self, pid: u16, act: u8) -> Option<String> {
        let k = self.kitchen.as_mut()?;
        let p = self.players.get(&pid)?;
        if !p.inventory.has_room() {
            return Some(lines::HANDS_FULL.into());
        }
        let (left, kind) = if act == action::TAKE_WATER { (&mut k.water, item_kind::WATER) } else { (&mut k.juice, item_kind::JUICE) };
        if *left == 0 {
            return Some(lines::NONE_LEFT.into());
        }
        *left -= 1;
        self.give_new(pid, kind);
        Some("Firmowe, za darmo. Dzięki, szefie!".into())
    }

    fn fridge_milk(&mut self, pid: u16) -> Option<String> {
        let k = self.kitchen.as_mut()?;
        let p = self.players.get_mut(&pid)?;
        if p.inventory.held_kind() != item_kind::COFFEE {
            return Some(lines::MILK_NEEDS_COFFEE.into());
        }
        if k.milk == 0 {
            return Some(lines::NO_MILK.into());
        }
        k.milk -= 1;
        if let Some(cup) = p.inventory.hands.as_mut() {
            cup.kind = item_kind::LATTE; // same mug, same coffee going cold
            cup.label = "Z mlekiem".into();
        }
        refresh(p);
        Some(lines::milk_added(k.milk))
    }

    /// The dishwasher's cycle.
    pub(super) fn tick_kitchen(&mut self) {
        let now = self.clock.total_minutes();
        if let Some(k) = self.kitchen.as_mut() {
            if k.tick(now) {
                self.log("* dishwasher done");
            }
        }
    }

    /// Mugs somebody took away (left the game): back to the cupboard.
    pub(super) fn return_mugs_of(&mut self, items: &[Item]) {
        let n =
            items.iter().filter(|i| matches!(i.kind, item_kind::CUP | item_kind::EMPTY_CUP | item_kind::COFFEE | item_kind::LATTE)).count();
        if let Some(k) = self.kitchen.as_mut() {
            k.return_mugs(u8::try_from(n).unwrap_or(u8::MAX));
        }
    }
}
