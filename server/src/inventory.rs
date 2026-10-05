//! Items and the player's inventory (GDD 9a, step 3).
//!
//! A character has small pockets and two hands. Small items (pass, card) fit
//! in pockets or hands; big ones (laptop, coffee) only in hands. What you
//! carry decides your access: a pass or card anywhere on you opens the gates
//! (so giving your card away gives away your access).

use crate::map::access;

/// Item kinds (also sent to clients: `EntityState::held`, `Inventory`).
pub mod kind {
    pub const NONE: u8 = 0;
    pub const GUEST_PASS: u8 = 1;
    pub const EMPLOYEE_CARD: u8 = 2;
    pub const LAPTOP: u8 = 3;
    pub const COFFEE: u8 = 4;
    pub const FRUIT: u8 = 5;
    // Shop goods (see `shop::PRODUCTS`).
    pub const SANDWICH_CHEESE: u8 = 10;
    pub const SANDWICH_HAM: u8 = 11;
    pub const WRAP: u8 = 12;
    pub const BURGER: u8 = 13;
    pub const FRIES: u8 = 14;
    pub const BUN: u8 = 15;
    pub const BAR: u8 = 16;
    pub const CHIPS: u8 = 17;
    pub const WATER: u8 = 18;
    pub const ENERGY_DRINK: u8 = 19;
    pub const JUICE: u8 = 20;
    pub const BEER: u8 = 21;
    pub const WINE: u8 = 22;
    pub const CIGARETTES: u8 = 23;
    pub const UMBRELLA: u8 = 24;
    // Sweets from the chill-room tray (free; see `treats`).
    pub const DONUT: u8 = 25;
    pub const COOKIE: u8 = 26;
    pub const CHEESECAKE: u8 = 27;
    // Lunch boxes (ordered in the app, see `lunch`).
    pub const PIEROGI: u8 = 28;
    pub const PIZZA: u8 = 29;
    pub const SUSHI: u8 = 30;
    pub const SCHNITZEL: u8 = 31;
    pub const SALAD: u8 = 32;
    pub const KEBAB: u8 = 33;
    /// What's left of a coffee (cleaning.rs).
    pub const EMPTY_CUP: u8 = 34;
    /// A clean mug from the kitchen cupboard (kitchen.rs).
    pub const CUP: u8 = 35;
    /// A carton of milk (shop) - tops up the fridge.
    pub const MILK: u8 = 36;
    /// Coffee with milk (the fridge's milk).
    pub const LATTE: u8 = 37;
    /// A 100 ml "małpka" of vodka (shop).
    pub const MALPKA: u8 = 38;
    /// The board's breathalyser (F: test the person next to you).
    pub const BREATHALYSER: u8 = 39;
    /// A kitchen knife from the cupboard (for bread… or a fight).
    pub const KNIFE: u8 = 40;
    /// The chill room's TV remote; the boombox (hands only).
    pub const REMOTE: u8 = 41;
    pub const BOOMBOX: u8 = 42;
    /// Rolling tobacco (shop, 10 rolls) and a rolled cigarette (the minigame).
    pub const TOBACCO: u8 = 43;
    pub const ROLLED: u8 = 44;
    /// The storeroom key (the hook at the reception).
    pub const STORE_KEY: u8 = 45;
    /// From the storeroom: cola and cookies.
    pub const COLA: u8 = 46;
    pub const STORE_COOKIES: u8 = 47;
    /// From the first-aid cabinet at the reception.
    pub const PAINKILLER: u8 = 48;
    pub const CHARCOAL: u8 = 49;
    pub const VITAMIN: u8 = 50;
    pub const PLASTER: u8 = 51;
    /// The liquor cabinet's key (hidden somewhere in the office every day)
    /// and what's in the cabinet.
    pub const BAR_KEY: u8 = 52;
    pub const WHISKY: u8 = 53;
    pub const COGNAC: u8 = 54;
    pub const VODKA: u8 = 55;
}

pub const POCKETS: usize = 3;

pub fn is_small(k: u8) -> bool {
    match k {
        kind::GUEST_PASS | kind::EMPLOYEE_CARD | kind::FRUIT => true,
        // Shop goods fit in a pocket, except the bulky ones.
        kind::BURGER | kind::FRIES | kind::WINE => false,
        10..=27 => true,
        kind::MILK | kind::MALPKA | kind::BREATHALYSER | kind::KNIFE | kind::REMOTE => true,
        kind::TOBACCO..=kind::VODKA => true,
        28..=33 => false, // lunch boxes: both hands
        _ => false,
    }
}

pub fn display_name(k: u8) -> &'static str {
    match k {
        kind::GUEST_PASS => "Przepustka gościa",
        kind::EMPLOYEE_CARD => "Karta pracownika",
        kind::LAPTOP => "Laptop",
        kind::COFFEE => "Kawa",
        kind::EMPTY_CUP => "Brudny kubek",
        kind::CUP => "Kubek",
        kind::LATTE => "Kawa z mlekiem",
        kind::FRUIT => "Owoc",
        kind::BREATHALYSER => "Alkomat",
        kind::KNIFE => "Nóż kuchenny",
        kind::REMOTE => "Pilot do telewizora",
        kind::BOOMBOX => "Boombox",
        kind::ROLLED => "Skręt",
        kind::STORE_KEY => "Klucz do magazynku",
        kind::COLA => "Coca-Cola",
        kind::STORE_COOKIES => "Ciastka z magazynu",
        kind::PAINKILLER => "Apap",
        kind::CHARCOAL => "Węgiel aktywny",
        kind::VITAMIN => "Witamina C",
        kind::PLASTER => "Plaster",
        kind::BAR_KEY => "Mały kluczyk",
        kind::WHISKY => "Whisky",
        kind::COGNAC => "Koniak",
        kind::VODKA => "Wódka",
        k => crate::shop::product(k).map_or("?", |p| p.name),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Unique per server run (lets us find a specific item later).
    pub id: u32,
    pub kind: u8,
    /// Personalisation shown to the player, e.g. "Ola · IT / Produkt".
    pub label: String,
    /// Tick after which the item disappears (coffee goes cold).
    pub expires: Option<u32>,
    /// Player it belongs to (laptop: whose account it logs into); 0 = nobody.
    pub owner: u16,
    /// Pieces left (a pack of cigarettes); 1 for everything else.
    pub count: u8,
    /// Taken off a shop shelf, not paid for yet.
    pub unpaid: bool,
    /// Food past its best (fruit): upsets the stomach.
    pub stale: bool,
    /// Somebody peed in it (a drink): disgusting, maybe sick. Not saved.
    pub tainted: bool,
    /// A rolled cigarette: how well (0..100). Not saved.
    pub quality: u8,
}

/// Use up one piece of a pack in `slot` (a cigarette); the empty pack goes.
pub fn take_piece(slot: &mut Option<Item>) {
    if let Some(item) = slot {
        item.count = item.count.saturating_sub(1);
        if item.count == 0 {
            *slot = None;
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inventory {
    pub hands: Option<Item>,
    pub pockets: [Option<Item>; POCKETS],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    HandsFull,
    PocketsFull,
    TooBig,
    Empty,
}

impl Refusal {
    pub fn line(self) -> &'static str {
        match self {
            Refusal::HandsFull => "Mam zajęte ręce.",
            Refusal::PocketsFull => "Nie mam już miejsca w kieszeniach.",
            Refusal::TooBig => "To się nie zmieści do kieszeni.",
            Refusal::Empty => "Nic tu nie ma.",
        }
    }
}

impl Inventory {
    /// Rights from what is carried (pockets or hands).
    pub fn access(&self) -> u8 {
        self.items().fold(0, |a, it| {
            a | match it.kind {
                kind::GUEST_PASS => access::GUEST,
                kind::EMPLOYEE_CARD => access::CARD,
                kind::STORE_KEY => access::KEY,
                _ => 0,
            }
        })
    }

    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.hands.iter().chain(self.pockets.iter().flatten())
    }

    pub fn has(&self, k: u8) -> bool {
        self.items().any(|it| it.kind == k)
    }

    pub fn held_kind(&self) -> u8 {
        self.hands.as_ref().map_or(kind::NONE, |it| it.kind)
    }

    pub fn hands_free(&self) -> bool {
        self.hands.is_none()
    }

    /// Room for one more item: free hands or an empty pocket.
    pub fn has_room(&self) -> bool {
        self.hands_free() || self.pockets.iter().any(Option::is_none)
    }

    /// Store a received item: small ones in a pocket first, then hands.
    pub fn add(&mut self, item: Item) -> Result<(), Item> {
        if is_small(item.kind) {
            if let Some(slot) = self.pockets.iter_mut().find(|s| s.is_none()) {
                *slot = Some(item);
                return Ok(());
            }
        }
        if self.hands.is_none() {
            self.hands = Some(item);
            return Ok(());
        }
        Err(item)
    }

    /// Pocket `slot` -> hands (swapping a small item in hands back into it).
    pub fn take_out(&mut self, slot: usize) -> Result<(), Refusal> {
        let Some(pocket) = self.pockets.get_mut(slot) else { return Err(Refusal::Empty) };
        if pocket.is_none() {
            return Err(Refusal::Empty);
        }
        if let Some(h) = &self.hands {
            if !is_small(h.kind) {
                return Err(Refusal::HandsFull);
            }
        }
        std::mem::swap(pocket, &mut self.hands);
        Ok(())
    }

    /// Hands -> first free pocket.
    pub fn put_away(&mut self) -> Result<(), Refusal> {
        let Some(h) = &self.hands else { return Err(Refusal::Empty) };
        if !is_small(h.kind) {
            return Err(Refusal::TooBig);
        }
        let Some(slot) = self.pockets.iter_mut().find(|s| s.is_none()) else { return Err(Refusal::PocketsFull) };
        *slot = self.hands.take();
        Ok(())
    }

    pub fn take_hands(&mut self) -> Option<Item> {
        self.hands.take()
    }

    pub fn remove_kind(&mut self, k: u8) -> Option<Item> {
        if self.hands.as_ref().is_some_and(|it| it.kind == k) {
            return self.hands.take();
        }
        self.pockets.iter_mut().find(|s| s.as_ref().is_some_and(|it| it.kind == k)).and_then(|s| s.take())
    }

    /// Goods taken off a shop shelf and not paid for go back.
    pub fn remove_unpaid(&mut self) {
        if self.hands.as_ref().is_some_and(|i| i.unpaid) {
            self.hands = None;
        }
        for s in self.pockets.iter_mut() {
            if s.as_ref().is_some_and(|i| i.unpaid) {
                *s = None;
            }
        }
    }

    pub fn mark_paid(&mut self) {
        for it in self.hands.iter_mut().chain(self.pockets.iter_mut().flatten()) {
            it.unpaid = false;
        }
    }

    /// Remove everything belonging to player `owner`; `true` if anything went.
    pub fn remove_owned_by(&mut self, owner: u16) -> bool {
        let mut removed = false;
        for s in std::iter::once(&mut self.hands).chain(self.pockets.iter_mut()) {
            if s.as_ref().is_some_and(|i| i.owner == owner) {
                *s = None;
                removed = true;
            }
        }
        removed
    }

    /// Drop expired items (cold coffee); returns what was removed.
    pub fn expire(&mut self, tick: u32) -> Vec<Item> {
        let mut gone = Vec::new();
        let expired = |it: &Option<Item>| it.as_ref().is_some_and(|i| i.expires.is_some_and(|t| tick >= t));
        if expired(&self.hands) {
            gone.extend(self.hands.take());
        }
        for s in self.pockets.iter_mut() {
            if expired(s) {
                gone.extend(s.take());
            }
        }
        gone
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: u32, k: u8) -> Item {
        Item {
            id,
            kind: k,
            label: String::new(),
            expires: None,
            owner: 0,
            count: 1,
            unpaid: false,
            stale: false,
            tainted: false,
            quality: 0,
        }
    }

    #[test]
    fn access_comes_from_carried_items() {
        let mut inv = Inventory::default();
        assert_eq!(inv.access(), 0);
        inv.add(item(1, kind::GUEST_PASS)).unwrap();
        assert_eq!(inv.access(), access::GUEST);
        inv.take_out(0).unwrap(); // in hands still counts
        assert_eq!(inv.access(), access::GUEST);
        inv.add(item(2, kind::EMPLOYEE_CARD)).unwrap();
        assert_eq!(inv.access(), access::GUEST | access::CARD);
        inv.remove_kind(kind::GUEST_PASS);
        assert_eq!(inv.access(), access::CARD);
    }

    #[test]
    fn small_items_go_to_pockets_big_ones_to_hands() {
        let mut inv = Inventory::default();
        for i in 0..POCKETS as u32 {
            inv.add(item(i, kind::GUEST_PASS)).unwrap();
        }
        assert!(inv.hands_free(), "pockets first");
        inv.add(item(9, kind::EMPLOYEE_CARD)).unwrap();
        assert_eq!(inv.held_kind(), kind::EMPLOYEE_CARD, "then hands");
        assert!(inv.add(item(10, kind::LAPTOP)).is_err(), "full");
        let mut inv = Inventory::default();
        inv.add(item(1, kind::LAPTOP)).unwrap();
        assert_eq!(inv.held_kind(), kind::LAPTOP);
        assert_eq!(inv.put_away(), Err(Refusal::TooBig), "a laptop doesn't fit a pocket");
    }

    #[test]
    fn take_out_and_put_away_swap_pocket_and_hands() {
        let mut inv = Inventory::default();
        inv.add(item(1, kind::GUEST_PASS)).unwrap();
        inv.add(item(2, kind::EMPLOYEE_CARD)).unwrap();
        inv.take_out(1).unwrap();
        assert_eq!(inv.held_kind(), kind::EMPLOYEE_CARD);
        inv.take_out(0).unwrap(); // swap: pass to hands, card back in pocket 0
        assert_eq!(inv.held_kind(), kind::GUEST_PASS);
        assert_eq!(inv.pockets[0].as_ref().unwrap().kind, kind::EMPLOYEE_CARD);
        inv.put_away().unwrap();
        assert!(inv.hands_free());
        assert_eq!(inv.take_out(2), Err(Refusal::Empty));
        let mut busy = Inventory::default();
        busy.add(item(3, kind::LAPTOP)).unwrap();
        busy.pockets[0] = Some(item(4, kind::GUEST_PASS));
        assert_eq!(busy.take_out(0), Err(Refusal::HandsFull), "hands hold a laptop");
    }

    #[test]
    fn coffee_goes_cold() {
        let mut inv = Inventory::default();
        inv.add(Item {
            id: 1,
            kind: kind::COFFEE,
            label: String::new(),
            expires: Some(100),
            owner: 0,
            count: 1,
            unpaid: false,
            stale: false,
            tainted: false,
            quality: 0,
        })
        .unwrap();
        assert!(inv.expire(99).is_empty());
        assert_eq!(inv.expire(100).len(), 1);
        assert!(inv.hands_free());
    }
}
