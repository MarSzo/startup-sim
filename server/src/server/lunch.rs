//! Lunch orders from the computer, delivered to the reception.

use crate::clock;
use crate::lunch;
use crate::npc;
use crate::protocol::{self as proto, Packet};
use crate::shop;
use crate::weather;

use super::{Say, Server};

impl Server {
    // ------------------------------------------------------------- lunch

    pub(super) fn lunch_packet(&self, pid: u16) -> Option<Packet> {
        let account = self.calendar_account(pid)?;
        let now = self.clock.minute();
        let order = self.lunch_orders.iter().find(|o| o.owner == account);
        let (state, dish, arrives) = match order {
            Some(o) if o.delivered => (lunch::state::WAITING, o.dish, proto::NO_TIME),
            Some(o) => (lunch::state::ORDERED, o.dish, (o.arrives % clock::MIN_PER_DAY) as u16),
            None if !(lunch::ORDER_FROM..=lunch::ORDER_TO).contains(&now) => (lunch::state::CLOSED, 0, proto::NO_TIME),
            None => (lunch::state::NONE, 0, proto::NO_TIME),
        };
        let dishes = lunch::MENU
            .iter()
            .filter_map(|(k, restaurant, eta)| {
                shop::product(*k).map(|p| proto::Dish {
                    kind: *k,
                    price: super::shop::price(p.price),
                    eta: *eta as u8,
                    name: p.name.into(),
                    restaurant: (*restaurant).into(),
                })
            })
            .collect();
        Some(Packet::LunchMenu { state, dish, arrives, dishes })
    }

    pub(super) fn send_lunch(&mut self, pid: u16) {
        if let Some(pk) = self.lunch_packet(pid) {
            self.send_to(pid, &pk);
        }
    }

    /// Order in the app: paid from the computer owner's wallet.
    pub(super) fn handle_lunch_order(&mut self, pid: u16, dish: u8) {
        let Some(account) = self.calendar_account(pid) else { return };
        let now = self.clock.minute();
        let Some(&(_, _, eta)) = lunch::menu_entry(dish) else { return };
        let Some(price) = shop::product(dish).map(|p| p.price) else { return };
        let busy = self.lunch_orders.iter().any(|o| o.owner == account);
        let open = (lunch::ORDER_FROM..=lunch::ORDER_TO).contains(&now);
        let Some(p) = self.players.get_mut(&account) else { return };
        if busy || !open || p.money < price {
            self.send_lunch(pid);
            return;
        }
        p.money -= price;
        let spread = self.rng.u32(0..=2 * lunch::ETA_SPREAD);
        let rain = if weather::wet(self.weather.now) { lunch::RAIN_DELAY } else { 0 };
        let minutes = (eta + spread).saturating_sub(lunch::ETA_SPREAD) + rain;
        let arrives = self.clock.total_minutes() + minutes;
        self.lunch_orders.push(lunch::Order { owner: account, dish, arrives, delivered: false });
        let who = if pid == account { String::new() } else { format!(" (zamówione przez {})", self.nick(pid)) };
        let name = shop::product(dish).map_or("?", |p| p.name);
        self.log(format!("* lunch: {} orders {name}, arrives {}{who}", self.nick(account), clock::hhmm(arrives % clock::MIN_PER_DAY)));
        self.send_lunch(pid);
    }

    /// The courier arrives: the receptionist tells the owner.
    pub(super) fn tick_lunch(&mut self) {
        let now = self.clock.total_minutes();
        let receptionist = self.npcs.iter().find(|n| n.role == npc::Role::Receptionist).map(|n| n.id);
        let mut arrived = Vec::new();
        for o in &mut self.lunch_orders {
            if !o.delivered && now >= o.arrives {
                o.delivered = true;
                arrived.push((o.owner, o.dish));
            }
        }
        for (owner, dish) in arrived {
            let name = shop::product(dish).map_or("?", |p| p.name);
            if let Some(r) = receptionist {
                self.says.push(Say::addressed(r, lunch::lines::arrived(name), owner));
            }
            if let Some(nick) = self.players.get(&owner).map(|p| p.nick.clone()) {
                let body = format!("Kurier zostawił: {name}.\nOdbierz na recepcji (piętro 1) — E przy biurku recepcji.");
                self.office_mail(&nick, "Obiady do biura", "Twój obiad czeka na recepcji", &body);
            }
        }
    }

    /// E at the reception with a box waiting.
    pub(super) fn pick_up_lunch(&mut self, pid: u16) -> String {
        let Some(i) = self.lunch_orders.iter().position(|o| o.owner == pid && o.delivered) else { return String::new() };
        if !self.players.get(&pid).is_some_and(|p| p.inventory.hands_free()) {
            return lunch::lines::HANDS_FULL.into();
        }
        let o = self.lunch_orders.remove(i);
        let name = shop::product(o.dish).map_or("?", |p| p.name);
        let item = self.mint_item(o.dish, format!("{name} (na wynos)"));
        self.give(pid, item);
        lunch::lines::handed_over(&name.to_lowercase())
    }
}
