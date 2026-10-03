//! Items: on the floor, in hands and pockets, handing over, using.

use std::collections::HashSet;
use std::fmt::Write as _;

use crate::coffee;
use crate::fire;
use crate::inventory::{self, kind as item_kind, Inventory, Item};
use crate::needs::{self, Rest};
use crate::npc;
use crate::protocol::{self as proto, Packet};
use crate::shop;
use crate::sim::{self, Body, Pos};
use crate::treats;

use super::player::refresh;
use super::{dist2, Say, Server};

/// An item lying on the floor; `handle` is its entity id in snapshots.
pub(super) struct Dropped {
    pub(super) handle: u16,
    pub(super) item: Item,
    pub(super) floor: u8,
    pub(super) pos: Pos,
}

/// Entity ids of items on the floor, laptops, vehicles and the tray
/// (players below, NPCs from `NPC_ID_BASE`).
pub(super) const DROP_HANDLE_BASE: u16 = 0xE000;
/// At most this many items on the floor; beyond it the oldest nobody's item
/// goes (someone tidied up). Keeps the handle space from running out and
/// snapshots small, whatever players do with the fruit bowl.
const MAX_DROPPED: usize = 1024;
/// Reach for picking up / handing over items.
pub(super) const PICKUP_RADIUS: i32 = sim::TILE_UNITS * 5 / 4;
const GIVE_RADIUS: i32 = sim::TILE_UNITS * 2;

impl Server {
    fn label_for(&self, pid: u16, k: u8) -> String {
        let Some(p) = self.players.get(&pid) else { return String::new() };
        let dept = self.cfg.recruitment.department_name(p.department).unwrap_or("");
        match k {
            item_kind::GUEST_PASS => format!("Dzień próbny: {}", p.nick),
            item_kind::EMPLOYEE_CARD if !dept.is_empty() => format!("{} · {dept}", p.nick),
            item_kind::EMPLOYEE_CARD => p.nick.clone(),
            item_kind::LAPTOP => format!("Laptop: {}", p.nick),
            item_kind::COFFEE => "Gorąca, z ekspresu".into(),
            item_kind::EMPTY_CUP => "Po kawie".into(),
            item_kind::CUP => "Z kuchennej szafki".into(),
            _ => String::new(),
        }
    }

    /// Create a new item for `pid` (labelled for them) and hand it over.
    pub(super) fn give_new(&mut self, pid: u16, k: u8) {
        self.give_new_tainted(pid, k, false);
    }

    /// `give_new`, maybe with something nasty in it (peed-in coffee).
    pub(super) fn give_new_tainted(&mut self, pid: u16, k: u8, tainted: bool) {
        let label = self.label_for(pid, k);
        let coffee = k == item_kind::COFFEE;
        let item = Item {
            expires: coffee.then_some(self.tick + coffee::DRINK_TICKS),
            owner: if coffee { 0 } else { pid },
            tainted,
            ..self.mint_item(k, label)
        };
        self.give(pid, item);
    }

    /// Put an item into a player's inventory; if it doesn't fit, it lands on
    /// the floor at their feet.
    pub(super) fn give(&mut self, pid: u16, item: Item) {
        let Some(p) = self.players.get_mut(&pid) else { return };
        match p.inventory.add(item) {
            Ok(()) => refresh(p),
            Err(item) => {
                let (floor, pos) = (p.body.floor, p.body.pos);
                self.drop_at(floor, pos, item);
            }
        }
    }

    pub(super) fn drop_at(&mut self, floor: u8, pos: Pos, item: Item) {
        if self.dropped.len() >= MAX_DROPPED {
            let oldest = self.dropped.iter().position(|d| d.item.owner == 0).unwrap_or(0);
            self.dropped.remove(oldest);
        }
        let handle = self.alloc_handle();
        self.dropped.push(Dropped { handle, item, floor, pos });
    }

    /// A free entity id for an item on the floor, a laptop on a desk, a
    /// vehicle or the tray.
    pub(super) fn alloc_handle(&mut self) -> u16 {
        let next = |h: u16| if h + 1 >= npc::NPC_ID_BASE { DROP_HANDLE_BASE } else { h + 1 };
        let used: HashSet<u16> = self
            .dropped
            .iter()
            .map(|d| d.handle)
            .chain(self.computers.iter().map(|c| c.handle))
            .chain(self.vehicles.iter().map(|v| v.handle))
            .chain(self.tray.as_ref().map(|t| t.handle))
            .chain(self.puddles.iter().map(|p| p.handle))
            .collect();
        let mut handle = self.next_drop_handle;
        // Bounded: MAX_DROPPED + laptops + vehicles + puddles is far below the span.
        for _ in DROP_HANDLE_BASE..npc::NPC_ID_BASE {
            if !used.contains(&handle) {
                break;
            }
            handle = next(handle);
        }
        debug_assert!(!used.contains(&handle), "entity handles exhausted");
        self.next_drop_handle = next(handle);
        handle
    }

    pub(super) fn handle_item_action(&mut self, id: u16, action: u8, slot: u8) {
        let Some(p) = self.players.get_mut(&id) else { return };
        if !p.in_building() {
            return;
        }
        match action {
            proto::item_action::TAKE_OUT => match p.inventory.take_out(usize::from(slot)) {
                Ok(()) => refresh(p),
                Err(r) => self.says.push(Say::new(id, r.line())),
            },
            proto::item_action::PUT_AWAY => match p.inventory.put_away() {
                Ok(()) => refresh(p),
                Err(r) => self.says.push(Say::new(id, r.line())),
            },
            proto::item_action::DROP => {
                if let Some(item) = p.inventory.take_hands() {
                    refresh(p);
                    let (floor, pos) = (p.body.floor, p.body.pos);
                    self.drop_at(floor, pos, item);
                    self.sounds.push((crate::protocol::sound::DROP, floor, pos));
                }
            }
            proto::item_action::GIVE => self.give_to_nearest(id),
            proto::item_action::USE => self.use_held(id),
            _ => {}
        }
    }

    /// G: hand what you hold to the nearest person within reach.
    fn give_to_nearest(&mut self, id: u16) {
        let Some(p) = self.players.get(&id) else { return };
        if p.inventory.hands_free() {
            return;
        }
        let (floor, pos) = (p.body.floor, p.body.pos);
        let target = self
            .players
            .values()
            .filter(|o| o.id != id && o.in_building() && o.body.floor == floor)
            .map(|o| (o.id, dist2(o.body.pos, pos)))
            .filter(|&(_, d)| d <= GIVE_RADIUS * GIVE_RADIUS)
            .min_by_key(|&(_, d)| d)
            .map(|(t, _)| t);
        let Some(target) = target else {
            self.says.push(Say::new(id, "Nie ma nikogo obok."));
            return;
        };
        let Some(item) = self.players.get_mut(&id).and_then(|p| p.inventory.take_hands()) else { return };
        let name = inventory::display_name(item.kind);
        let Some(to) = self.players.get_mut(&target) else { return };
        let to_nick = to.nick.clone();
        let refused = match to.inventory.add(item) {
            Ok(()) => {
                refresh(to);
                None
            }
            Err(item) => Some(item),
        };
        let Some(from) = self.players.get_mut(&id) else { return };
        match refused {
            None => {
                refresh(from);
                let line = format!("* item: {} gave {name} to {to_nick}", from.nick);
                self.says.push(Say::addressed(id, format!("Proszę, {to_nick} — {}.", name.to_lowercase()), target));
                self.log(line);
            }
            Some(item) => {
                from.inventory.hands = Some(item); // give it back
                self.says.push(Say::new(id, format!("{to_nick} nie ma już wolnych rąk ani kieszeni.")));
            }
        }
    }

    /// F: drink, eat, smoke or look at what you hold.
    pub(super) fn use_held(&mut self, id: u16) {
        let tick = self.tick;
        let Some(p) = self.players.get_mut(&id) else { return };
        let Some(held) = &p.inventory.hands else { return };
        let snd = match held.kind {
            item_kind::COFFEE | item_kind::LATTE | item_kind::COLA | item_kind::WHISKY | item_kind::COGNAC | item_kind::VODKA => {
                Some(crate::protocol::sound::DRINK)
            }
            item_kind::STORE_COOKIES => Some(crate::protocol::sound::EAT),
            item_kind::FRUIT if !p.needs.is_full() => Some(crate::protocol::sound::EAT),
            item_kind::CIGARETTES if !held.unpaid => Some(crate::protocol::sound::LIGHTER),
            item_kind::WATER
            | item_kind::ENERGY_DRINK
            | item_kind::JUICE
            | item_kind::BEER
            | item_kind::WINE
            | item_kind::MALPKA
            | item_kind::MILK
                if !held.unpaid =>
            {
                Some(crate::protocol::sound::DRINK)
            }
            k if !held.unpaid && k != item_kind::UMBRELLA => crate::shop::product(k).map(|_| crate::protocol::sound::EAT),
            _ => None,
        };
        if let Some(s) = snd {
            self.sounds.push((s, p.body.floor, p.body.pos));
        }
        let mut drink: Option<Option<needs::Event>> = None;
        let line = match held.kind {
            item_kind::COFFEE | item_kind::LATTE => {
                let latte = held.kind == item_kind::LATTE;
                let tainted = held.tainted;
                p.inventory.take_hands();
                p.needs.drink_coffee();
                if latte {
                    p.needs.add_stress(-4); // smoother
                }
                refresh(p);
                self.give_new(id, item_kind::EMPTY_CUP);
                if tainted {
                    self.drank_pee(id);
                } else {
                    self.says.push(Say::new(id, coffee::lines::DRUNK));
                }
                return;
            }
            item_kind::EMPTY_CUP => "Brudny kubek. Do zlewu albo do zmywarki w kuchni.".into(),
            item_kind::CUP => "Czysty kubek — pod ekspres i gotowe.".into(),
            item_kind::FRUIT if p.needs.is_full() => needs::lines::NOT_HUNGRY.into(),
            item_kind::FRUIT => {
                let what = held.label.to_lowercase();
                let stale = held.stale;
                p.inventory.take_hands();
                let yuck = p.needs.eat_fruit();
                refresh(p);
                if stale {
                    p.needs.upset_stomach();
                    treats::lines::STALE_EATEN.to_string()
                } else if yuck {
                    format!("{} ({what})", needs::lines::YUCK)
                } else {
                    format!("Mniam, {what}.")
                }
            }
            item_kind::EMPLOYEE_CARD => format!("Karta pracownika: {}.", held.label),
            item_kind::GUEST_PASS => "Przepustka gościa — ważna do końca dnia.".into(),
            _ if held.unpaid => shop::lines::PAY_FIRST.into(),
            // Light up right here - wherever that is.
            item_kind::CIGARETTES => {
                inventory::take_piece(&mut p.inventory.hands);
                refresh(p);
                p.rest = Some((Rest::Smoking { until: tick + needs::SMOKE_TICKS }, p.body.floor, p.body.pos));
                if self.smoke.is_open_air((p.body.floor, p.room)) { fire::lines::LIT } else { fire::lines::LIT_INSIDE }.into()
            }
            item_kind::LAPTOP => format!("{} — położę go na wolnym biurku w swoim dziale (E).", held.label),
            item_kind::BREATHALYSER => {
                let department = p.department;
                return self.breath_test(id, department);
            }
            item_kind::KNIFE => return self.attack(id),
            item_kind::REMOTE => return self.use_remote(id),
            item_kind::ROLLED if !held.unpaid => return self.smoke_roll(id),
            item_kind::TOBACCO if !held.unpaid => crate::supplies::lines::ROLL_FIRST.into(),
            item_kind::STORE_KEY => crate::supplies::lines::KEY.into(),
            item_kind::BAR_KEY => crate::supplies::lines::BAR_KEY.into(),
            item_kind::WHISKY | item_kind::COGNAC | item_kind::VODKA => {
                let k = held.kind;
                p.inventory.take_hands();
                p.needs.apply(crate::shop::Effect { hunger: 0, energy: -5, stress: -20, bladder: 5 });
                drink = Some(p.needs.drink_alcohol(crate::drunk::alcohol_of(k)));
                refresh(p);
                match k {
                    item_kind::WHISKY => crate::supplies::lines::WHISKY,
                    item_kind::COGNAC => crate::supplies::lines::COGNAC,
                    _ => crate::supplies::lines::VODKA,
                }
                .into()
            }
            item_kind::COLA => {
                p.inventory.take_hands();
                p.needs.apply(crate::shop::Effect { hunger: 0, energy: 15, stress: -3, bladder: 12 });
                refresh(p);
                crate::supplies::lines::COLA.into()
            }
            item_kind::STORE_COOKIES => {
                p.inventory.take_hands();
                p.needs.apply(crate::shop::Effect { hunger: -12, energy: 3, stress: -5, bladder: 0 });
                refresh(p);
                crate::supplies::lines::COOKIES.into()
            }
            item_kind::PAINKILLER | item_kind::CHARCOAL | item_kind::VITAMIN | item_kind::PLASTER => {
                let k = held.kind;
                p.inventory.take_hands();
                p.needs.medicine(k);
                refresh(p);
                match k {
                    item_kind::PAINKILLER => crate::supplies::lines::PAINKILLER,
                    item_kind::CHARCOAL => crate::supplies::lines::CHARCOAL,
                    item_kind::VITAMIN => crate::supplies::lines::VITAMIN,
                    _ => crate::supplies::lines::PLASTER,
                }
                .into()
            }
            item_kind::BOOMBOX => return self.use_boombox(id),
            k => {
                let Some(prod) = shop::product(k) else { return };
                p.inventory.take_hands();
                p.needs.apply(prod.effect);
                let alcohol = crate::drunk::alcohol_of(k);
                if alcohol > 0 {
                    drink = Some(p.needs.drink_alcohol(alcohol));
                }
                refresh(p);
                prod.line.to_string()
            }
        };
        self.says.push(Say::new(id, line));
        if let Some(event) = drink {
            self.after_drink(id, event);
        }
    }

    /// After a beer (wine, vodka): a burp - or throwing up / passing out.
    fn after_drink(&mut self, id: u16, event: Option<needs::Event>) {
        let tick = self.tick;
        let Some(p) = self.players.get_mut(&id) else { return };
        let (floor, pos) = (p.body.floor, p.body.pos);
        match event {
            Some(needs::Event::Vomit) => self.throw_up(id, crate::drunk::lines::VOMIT, "drunk"),
            Some(needs::Event::PassOut) => {
                p.held_until = tick + crate::drunk::PASS_OUT_TICKS;
                p.held_activity = crate::protocol::activity::PASSED_OUT;
                p.passed_out = true;
                p.rest = None;
                self.says.push(Say::new(id, crate::drunk::lines::PASS_OUT));
                let text = format!("{} zasnął/zasnęła pijany(a) na podłodze.", self.nick_of_player(id));
                self.notify_room_of(id, crate::protocol::notice::ALERT, &text);
                self.log(format!("* drunk: {} passed out", self.nick_of_player(id)));
            }
            _ => self.sounds.push((crate::protocol::sound::BURP, floor, pos)),
        }
    }

    /// Throwing up right here (drink, cigarettes, a "special" coffee): a
    /// moment in place, the sound, `line`, a puddle.
    pub(super) fn throw_up(&mut self, id: u16, line: &str, why: &str) {
        let tick = self.tick;
        let Some(p) = self.players.get_mut(&id) else { return };
        let (floor, pos) = (p.body.floor, p.body.pos);
        p.held_until = tick + crate::drunk::VOMIT_TICKS;
        p.held_activity = crate::protocol::activity::VOMITING;
        p.rest = None;
        self.sounds.push((crate::protocol::sound::VOMIT, floor, pos));
        self.says.push(Say::new(id, line));
        self.leave_puddle(floor, pos, crate::protocol::puddle::VOMIT);
        let text = format!("{} zwymiotował(a) na podłogę. Fuj.", self.nick_of_player(id));
        self.notify_room_of(id, crate::protocol::notice::ALERT, &text);
        self.log(format!("* {why}: {} threw up", self.nick_of_player(id)));
    }

    /// Drank something somebody peed in: disgust, and half the time it
    /// comes back up.
    pub(super) fn drank_pee(&mut self, id: u16) {
        let Some(p) = self.players.get_mut(&id) else { return };
        p.needs.disgusted();
        if self.rng.u32(0..100) < crate::mischief::SICK_PERCENT {
            self.throw_up(id, crate::mischief::lines::TASTE_SICK, "tainted drink");
        } else {
            self.says.push(Say::new(id, crate::mischief::lines::TASTE));
        }
    }

    pub(super) fn nick_of_player(&self, id: u16) -> String {
        self.players.get(&id).map(|p| p.nick.clone()).unwrap_or_default()
    }

    /// Pick up the nearest item on the floor within reach, if any.
    pub(super) fn try_pickup(&mut self, pid: u16, body: &Body) -> Option<String> {
        let i = self
            .dropped
            .iter()
            .enumerate()
            .filter(|(_, d)| d.floor == body.floor && dist2(d.pos, body.pos) <= PICKUP_RADIUS * PICKUP_RADIUS)
            .min_by_key(|(_, d)| dist2(d.pos, body.pos))
            .map(|(i, _)| i)?;
        let p = self.players.get_mut(&pid)?;
        let d = self.dropped.remove(i);
        let name = inventory::display_name(d.item.kind);
        match p.inventory.add(d.item) {
            Ok(()) => {
                refresh(p);
                self.sounds.push((crate::protocol::sound::PICKUP, body.floor, body.pos));
                Some(format!("Podniesione: {}.", name.to_lowercase()))
            }
            Err(item) => {
                self.dropped.insert(i, Dropped { item, ..d });
                Some(inventory::Refusal::HandsFull.line().to_string())
            }
        }
    }
}

/// The hands and pockets as the owner sees them.
pub(super) fn inventory_packet(inv: &Inventory) -> Packet {
    let slot = |it: &Option<Item>| match it {
        Some(i) => {
            let mut label = i.label.clone();
            if i.count > 1 {
                let _ = write!(label, " ({} szt.)", i.count);
            }
            if i.unpaid {
                let price = shop::product(i.kind).map_or(0, |p| p.price);
                let _ = write!(label, " — niezapłacone, {}", shop::zl(price));
            }
            proto::SlotInfo { kind: i.kind, id: i.id, label }
        }
        None => proto::SlotInfo::default(),
    };
    let mut slots = Vec::with_capacity(1 + inv.pockets.len());
    slots.push(slot(&inv.hands));
    slots.extend(inv.pockets.iter().map(slot));
    Packet::Inventory { slots }
}
