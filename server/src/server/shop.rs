//! The shop: shelves and the till.

use crate::inventory::Item;
use crate::protocol::{self as proto, Packet};
use crate::shop;
use crate::sim::Body;

use super::player::refresh;
use super::{Say, Server};

impl Server {
    /// E at a shelf: the list of goods; `false` = no shelf in reach.
    pub(super) fn show_shelf(&mut self, pid: u16, body: &Body) -> bool {
        let Some(s) = shop::shelf_in_reach(&self.shelves, body) else { return false };
        let goods = s
            .goods
            .iter()
            .filter_map(|&k| shop::product(k))
            .map(|p| proto::ShelfItem { kind: p.kind, price: price(p.price), name: p.name.into() })
            .collect();
        let packet = Packet::Shelf { shelf: s.id, title: s.title.into(), goods };
        self.send_to(pid, &packet);
        true
    }

    /// Take a product off a shelf (unpaid).
    pub(super) fn handle_shop_take(&mut self, pid: u16, shelf: u8, kind: u8) {
        let Some(body) = self.players.get(&pid).map(|p| p.body) else { return };
        let Some(s) = shop::shelf_in_reach(&self.shelves, &body).filter(|s| s.id == shelf) else { return };
        if !s.goods.contains(&kind) {
            return;
        }
        let Some(prod) = shop::product(kind) else { return };
        let item = Item { count: prod.count, unpaid: true, ..self.mint_item(kind, prod.name) };
        let Some(p) = self.players.get_mut(&pid) else { return };
        match p.inventory.add(item) {
            Ok(()) => refresh(p),
            Err(_) => self.says.push(Say::new(pid, shop::lines::NO_ROOM)),
        }
    }

    /// Pay for everything unpaid; the cashier's answer.
    pub(super) fn checkout(&mut self, pid: u16) -> Option<String> {
        let p = self.players.get_mut(&pid)?;
        let total: i64 = p.inventory.items().filter(|i| i.unpaid).filter_map(|i| shop::product(i.kind)).map(|pr| pr.price).sum();
        if total == 0 {
            return Some(shop::lines::NOTHING_TO_PAY.into());
        }
        if p.money < total {
            return Some(shop::lines::too_poor(total, p.money));
        }
        p.money -= total;
        p.inventory.mark_paid();
        refresh(p);
        let left = p.money;
        let line = format!("* shop: {} paid {}", p.nick, shop::zl(total));
        self.log(line);
        self.sound(crate::protocol::sound::TILL, pid);
        Some(shop::lines::paid(total, left))
    }
}

/// A price (grosze) for the wire.
pub(super) fn price(grosze: i64) -> u32 {
    u32::try_from(grosze.max(0)).unwrap_or(u32::MAX)
}
