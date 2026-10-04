//! The reception's first-aid cabinet and key hook, the storeroom upstairs,
//! the receptionist's lunch break, and rolling cigarettes.

use crate::inventory::{self, kind as item_kind, Item};
use crate::map::Tile;
use crate::needs::{self, Rest};
use crate::npc::Role;
use crate::protocol::Packet;
use crate::sim::{Body, Pos};
use crate::supplies::{self, lines, Stock};

use super::player::refresh;
use super::{dist2, Say, Server};

/// Where things are (from the map's tile types).
#[derive(Default)]
pub(super) struct Supplies {
    pub(super) stock: Stock,
    pub(super) key_on_hook: bool,
    pub(super) hook: Option<(u8, Tile)>,
    pub(super) cabinet: Option<(u8, Tile)>,
    /// The storeroom (floor, room) and its shelf tiles.
    pub(super) storeroom: Option<(u8, u16)>,
    pub(super) shelves: Vec<(u8, Tile)>,
    /// The receptionist is on her lunch break.
    pub(super) on_break: bool,
    /// The liquor cabinet; the places its key may hide; where it is today
    /// (an index in `hiding`; None = somebody found it).
    pub(super) liquor: Option<(u8, Tile)>,
    pub(super) hiding: Vec<(u8, Tile, &'static str)>,
    pub(super) bar_key_at: Option<usize>,
}

impl Supplies {
    pub(super) fn find(b: &crate::building::Building) -> Supplies {
        let mut s = Supplies { stock: Stock::morning(), key_on_hook: true, ..Supplies::default() };
        for (f, m) in b.active_floors() {
            for y in 0..m.height {
                for x in 0..m.width {
                    match m.tile_type(x, y) {
                        Some("key_hook") => s.hook = Some((f, Tile { x, y })),
                        Some("medicine_cabinet") => s.cabinet = Some((f, Tile { x, y })),
                        Some("liquor_cabinet") => s.liquor = Some((f, Tile { x, y })),
                        Some(t) => {
                            // Somewhere a player can get to (not outdoors, not the closed zone).
                            let room =
                                [(0, 1), (0, -1), (1, 0), (-1, 0)].iter().map(|(dx, dy)| m.room_at_tile(x + dx, y + dy)).find(|&r| r != 0);
                            let inside =
                                room.and_then(|r| m.rooms.iter().find(|d| d.id == r)).is_some_and(|d| !d.outdoor && d.kind != "service");
                            if let Some(&kind) = supplies::HIDING.iter().find(|&&h| h == t).filter(|_| inside) {
                                s.hiding.push((f, Tile { x, y }, kind));
                            }
                        }
                        None => {}
                    }
                }
            }
            if let Some(r) = m.rooms.iter().find(|r| r.kind == "storage") {
                s.storeroom = Some((f, r.id));
                for y in 0..m.height {
                    for x in 0..m.width {
                        let near_room = [(0, 1), (0, -1), (1, 0), (-1, 0)].iter().any(|(dx, dy)| m.room_at_tile(x + dx, y + dy) == r.id);
                        if m.tile_type(x, y) == Some("shelf") && near_room {
                            s.shelves.push((f, Tile { x, y }));
                        }
                    }
                }
            }
        }
        s
    }
}

fn near(at: Option<(u8, Tile)>, body: &Body) -> bool {
    at.is_some_and(|(f, t)| f == body.floor && dist2(Pos::tile_center(t.x, t.y), body.pos) <= supplies::REACH * supplies::REACH)
}

impl Server {
    /// E at the key hook, the first-aid cabinet or a storeroom shelf;
    /// false = none of them in reach.
    pub(super) fn use_supplies(&mut self, pid: u16, body: &Body) -> bool {
        if near(self.supplies.hook, body) {
            self.use_hook(pid);
        } else if near(self.supplies.liquor, body) {
            if self.players.get(&pid).is_some_and(|p| p.inventory.has(item_kind::BAR_KEY)) {
                let options = supplies::BAR.to_vec();
                self.supply_dialog(pid, lines::BAR, &options);
            } else {
                self.says.push(Say::new(pid, lines::BAR_LOCKED));
            }
        } else if near(self.supplies.cabinet, body) {
            let options = supplies::MEDICINES.to_vec();
            self.supply_dialog(pid, lines::CABINET, &options);
        } else if self.supplies.storeroom == Some((body.floor, self.room_of(body.floor, body.pos)))
            && self.supplies.shelves.iter().any(|&s| near(Some(s), body))
        {
            let options = supplies::STOREROOM.to_vec();
            self.supply_dialog(pid, lines::STOREROOM, &options);
        } else {
            return false;
        }
        true
    }

    /// The key: taken only while the receptionist isn't at her desk; put
    /// back by anybody.
    fn use_hook(&mut self, pid: u16) {
        let away = self.npcs.iter().find(|n| n.role == Role::Receptionist).is_none_or(|n| !n.at_home());
        let Some(p) = self.players.get_mut(&pid) else { return };
        if p.inventory.held_kind() == item_kind::STORE_KEY {
            p.inventory.take_hands();
            refresh(p);
            self.supplies.key_on_hook = true;
            self.says.push(Say::new(pid, lines::KEY_BACK));
            return;
        }
        if !self.supplies.key_on_hook {
            self.says.push(Say::new(pid, lines::KEY_GONE));
            return;
        }
        if !away {
            let r = self.npcs.iter().find(|n| n.role == Role::Receptionist).map(|n| n.id);
            self.says.push(Say::addressed(r.unwrap_or(pid), lines::KEY_NO, pid));
            return;
        }
        if !p.inventory.has_room() {
            self.says.push(Say::new(pid, lines::HANDS_FULL));
            return;
        }
        self.supplies.key_on_hook = false;
        self.give_new(pid, item_kind::STORE_KEY);
        self.says.push(Say::new(pid, lines::KEY_TAKEN));
        self.log(format!("* storeroom key taken by {}", self.nick_of_player(pid)));
    }

    fn supply_dialog(&mut self, pid: u16, text: &str, options: &[(u8, &str)]) {
        let mut kinds: Vec<u8> = options.iter().map(|o| o.0).collect();
        let mut labels: Vec<String> = options.iter().map(|(k, name)| lines::item(name, self.supplies.stock.left(*k))).collect();
        kinds.push(0);
        labels.push(lines::CLOSE.into());
        let Some(p) = self.players.get_mut(&pid) else { return };
        p.supply_menu = kinds;
        let addr = p.addr;
        self.send(addr, &Packet::Dialog { id: supplies::DIALOG, npc: pid, text: text.into(), options: labels });
    }

    /// The answer to a cabinet / the storeroom; false if not that dialog.
    pub(super) fn answer_supplies(&mut self, pid: u16, dialog: u8, choice: u8) -> bool {
        if dialog != supplies::DIALOG {
            return false;
        }
        let Some(p) = self.players.get_mut(&pid) else { return true };
        let menu = std::mem::take(&mut p.supply_menu);
        let (addr, room) = (p.addr, p.inventory.has_room());
        self.send(addr, &Packet::Dialog { id: 0, npc: pid, text: String::new(), options: Vec::new() });
        let Some(&kind) = menu.get(usize::from(choice)).filter(|&&k| k != 0) else { return true };
        if !room {
            self.says.push(Say::new(pid, lines::HANDS_FULL));
        } else if self.supplies.stock.take(kind) {
            self.give_new(pid, kind);
        } else {
            self.says.push(Say::new(pid, lines::EMPTY));
        }
        true
    }

    /// The minigame's result: one roll from the pack in hands.
    pub(super) fn handle_roll(&mut self, pid: u16, quality: u8) {
        let Some(p) = self.players.get_mut(&pid) else { return };
        let ok = p.inventory.hands.as_ref().is_some_and(|i| i.kind == item_kind::TOBACCO && !i.unpaid && i.count > 0);
        if !ok || !p.inventory.pockets.iter().any(Option::is_none) {
            return;
        }
        inventory::take_piece(&mut p.inventory.hands);
        refresh(p);
        let quality = quality.min(100);
        let roll = Item { quality, ..self.mint_item(item_kind::ROLLED, supplies::roll_name(quality)) };
        self.give(pid, roll);
        self.says.push(Say::new(pid, lines::ROLLED));
    }

    /// F with a roll: smoke it (a good one calms you more) - or it crumbles.
    pub(super) fn smoke_roll(&mut self, pid: u16) {
        let tick = self.tick;
        let Some(p) = self.players.get_mut(&pid) else { return };
        let Some(roll) = p.inventory.take_hands() else { return };
        refresh(p);
        if roll.quality < supplies::CRUMBLES_BELOW {
            self.says.push(Say::new(pid, lines::CRUMBLED));
            return;
        }
        p.needs.add_stress(-(i32::from(roll.quality) / 10));
        p.rest = Some((Rest::Smoking { until: tick + needs::SMOKE_TICKS }, p.body.floor, p.body.pos));
        let (floor, pos) = (p.body.floor, p.body.pos);
        self.sounds.push((crate::protocol::sound::LIGHTER, floor, pos));
        let lit = if self.smoke.is_open_air((floor, self.room_of(floor, pos))) {
            crate::fire::lines::LIT
        } else {
            crate::fire::lines::LIT_INSIDE
        };
        self.says.push(Say::new(pid, lit));
    }

    /// E at a plant, a bin, a wardrobe: is the liquor cabinet's key there?
    /// false = none in reach.
    pub(super) fn search_hideout(&mut self, pid: u16, body: &Body) -> bool {
        let at = self
            .supplies
            .hiding
            .iter()
            .enumerate()
            .filter(|(_, h)| near(Some((h.0, h.1)), body))
            .min_by_key(|(_, h)| dist2(Pos::tile_center(h.1.x, h.1.y), body.pos))
            .map(|(i, h)| (i, h.2));
        let Some((i, kind)) = at else { return false };
        let room = self.players.get(&pid).is_some_and(|p| p.inventory.has_room());
        if self.supplies.bar_key_at == Some(i) && room {
            self.supplies.bar_key_at = None;
            self.give_new(pid, item_kind::BAR_KEY);
            self.says.push(Say::new(pid, lines::FOUND_KEY));
            self.log(format!("* {} found the liquor cabinet's key", self.nick_of_player(pid)));
        } else {
            self.says.push(Say::new(pid, lines::nothing(kind)));
        }
        true
    }

    /// The liquor cabinet's key: somewhere new (if nobody has it).
    pub(super) fn hide_bar_key(&mut self) {
        let somewhere = self.players.values().any(|p| p.inventory.has(item_kind::BAR_KEY))
            || self.dropped.iter().any(|d| d.item.kind == item_kind::BAR_KEY);
        if somewhere || self.supplies.hiding.is_empty() {
            self.supplies.bar_key_at = None;
            return;
        }
        self.supplies.bar_key_at = Some(self.rng.usize(..self.supplies.hiding.len()));
    }

    /// The morning: the shelves and the cabinet full again, the key back on
    /// its hook if nobody has it.
    pub(super) fn restock_supplies(&mut self) {
        self.supplies.stock = Stock::morning();
        self.hide_bar_key();
        let somewhere = self.players.values().any(|p| p.inventory.has(item_kind::STORE_KEY))
            || self.dropped.iter().any(|d| d.item.kind == item_kind::STORE_KEY);
        if !somewhere {
            self.supplies.key_on_hook = true;
        }
    }

    /// The receptionist's lunch break (12:00-12:30): off to the kitchenette.
    pub(super) fn tick_lunch_break(&mut self) {
        let minute = self.clock.minute();
        let on = (supplies::BREAK_FROM..supplies::BREAK_UNTIL).contains(&minute);
        if on == self.supplies.on_break {
            return;
        }
        // To the kitchenette: the first free tile next to the fridge.
        let kitchen = self.kitchen.as_ref().and_then(|k| {
            let m = self.building.floor(k.floor)?;
            [(0, 1), (1, 0), (-1, 0), (0, -1)]
                .into_iter()
                .map(|(dx, dy)| Tile { x: k.fridge.x + dx, y: k.fridge.y + dy })
                .find(|t| !m.is_blocked(t.x, t.y))
                .map(|t| (k.floor, t))
        });
        let Some(n) = self.npcs.iter_mut().find(|n| n.role == Role::Receptionist) else { return };
        if on {
            if !n.is_idle() {
                return; // busy (escorting): she'll go when she can
            }
            if let Some(place) = kitchen {
                n.go_to(&self.building, place);
            }
            self.says.push(Say::new(n.id, "Przerwa obiadowa — zaraz wracam!"));
        } else {
            n.return_home(&self.building);
        }
        self.supplies.on_break = on;
    }
}
