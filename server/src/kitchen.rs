//! The chill-room kitchenette (backlog): a cupboard with the office's mugs
//! (a limited number), a sink for washing up by hand, a dishwasher and a
//! fridge.
//!
//! - Coffee needs a clean mug in your hands (take one from the cupboard).
//!   Drunk (or gone cold) it's a dirty mug. Clean mugs go back to the
//!   cupboard; dirty ones get washed at a sink (right away) or loaded into
//!   the dishwasher, which somebody has to start and later unload.
//! - The fridge keeps people's food and drinks (anyone can take anything),
//!   milk for the coffee (a carton from the shop tops it up) and a few free
//!   company drinks, restocked every morning.

use crate::building::Building;
use crate::inventory::{kind, Item};
use crate::map::Tile;
use crate::sim::{Body, Pos, TILE_UNITS};

/// Mugs the office has.
pub const MUGS: u8 = 8;
/// Kitchen knives in the cupboard (back every morning).
pub const KNIVES: u8 = 2;
pub const DISHWASHER_CAP: u8 = 8;
/// A dishwasher cycle (game minutes).
pub const WASH_MINUTES: u32 = 30;
pub const FRIDGE_SLOTS: usize = 10;
pub const MILK_MAX: u8 = 20;
pub const MILK_PER_CARTON: u8 = 10;
/// Free drinks put in the fridge every morning.
pub const FREE_WATER: u8 = 4;
pub const FREE_JUICE: u8 = 2;
/// Reach of the kitchen things (standing in front): 1.5 tiles.
pub const REACH: i32 = TILE_UNITS * 3 / 2;

/// `FridgeAction::action`.
pub mod action {
    /// Take stored item `arg` (index).
    pub const TAKE: u8 = 1;
    /// Put what's in the hands in (a milk carton tops the milk up).
    pub const PUT: u8 = 2;
    pub const TAKE_WATER: u8 = 3;
    pub const TAKE_JUICE: u8 = 4;
    /// Pour milk into the coffee in your hands.
    pub const MILK: u8 = 5;
}

#[derive(Debug, Clone)]
pub struct Kitchen {
    pub floor: u8,
    pub cupboard: Tile,
    pub dishwasher: Tile,
    pub sink: Tile,
    pub fridge: Tile,
    /// Clean mugs in the cupboard.
    pub mugs: u8,
    /// Dishwasher: dirty mugs loaded, clean ones waiting to be unloaded, and
    /// when the running cycle ends (`Clock::total_minutes`).
    pub dirty: u8,
    pub washed: u8,
    pub running_until: Option<u32>,
    pub stored: Vec<Item>,
    pub milk: u8,
    pub water: u8,
    pub juice: u8,
    /// Knives in the cupboard.
    pub knives: u8,
}

impl Kitchen {
    /// The kitchenette of the building (the first one found).
    pub fn find(b: &Building) -> Option<Kitchen> {
        for (f, m) in b.active_floors() {
            let find = |t: &str| -> Option<Tile> {
                (0..m.height)
                    .flat_map(|y| (0..m.width).map(move |x| (x, y)))
                    .find(|&(x, y)| m.tile_type(x, y) == Some(t))
                    .map(|(x, y)| Tile { x, y })
            };
            if let (Some(cupboard), Some(dishwasher), Some(sink), Some(fridge)) =
                (find("cupboard"), find("dishwasher"), find("kitchen_sink"), find("fridge"))
            {
                return Some(Kitchen {
                    floor: f,
                    cupboard,
                    dishwasher,
                    sink,
                    fridge,
                    mugs: MUGS,
                    dirty: 0,
                    washed: 0,
                    running_until: None,
                    stored: Vec::new(),
                    milk: 5,
                    water: FREE_WATER,
                    juice: FREE_JUICE,
                    knives: KNIVES,
                });
            }
        }
        None
    }

    pub fn near(&self, t: Tile, body: &Body) -> bool {
        let c = Pos::tile_center(t.x, t.y);
        body.floor == self.floor && (c.x - body.pos.x).pow(2) + (c.y - body.pos.y).pow(2) <= REACH * REACH
    }

    /// The dishwasher finished (at `now`)?
    pub fn tick(&mut self, now: u32) -> bool {
        if self.running_until.is_some_and(|t| now >= t) {
            self.running_until = None;
            self.washed += self.dirty;
            self.dirty = 0;
            return true;
        }
        false
    }

    /// Mugs that come back from somewhere (a player who left, the cleaner):
    /// straight into the cupboard, never more than the office has.
    pub fn return_mugs(&mut self, n: u8) {
        self.mugs = (self.mugs + n).min(MUGS);
    }

    /// The cleaner's mugs: into the dishwasher (unloading it first) and
    /// switched on; if it's running, she washes them by hand.
    pub fn cleaner_load(&mut self, n: u8, now: u32) {
        if n == 0 {
            return;
        }
        if self.running_until.is_some() {
            self.return_mugs(n);
            return;
        }
        self.return_mugs(self.washed);
        self.washed = 0;
        let room = DISHWASHER_CAP.saturating_sub(self.dirty);
        let load = n.min(room);
        self.dirty += load;
        self.return_mugs(n - load);
        self.running_until = Some(now + WASH_MINUTES);
    }

    /// Every morning: the free drinks are back.
    pub fn restock(&mut self) {
        self.water = FREE_WATER;
        self.juice = FREE_JUICE;
        self.knives = KNIVES;
    }
}

/// Whether an item may go in the fridge.
pub fn fridge_worthy(k: u8) -> bool {
    matches!(k, 10..=22 | 25..=33) || k == kind::MILK || k == kind::FRUIT
}

pub mod lines {
    /// Looking into the cupboard (a dialog): what's inside.
    pub fn cupboard(mugs: u8, knives: u8) -> String {
        let knives = match knives {
            0 => "noży brak (ktoś zabrał)".to_string(),
            1 => "1 nóż kuchenny".to_string(),
            n => format!("{n} noże kuchenne"),
        };
        format!("Szafka: kubki ({mugs}), {knives}, sztućce, okruszki.")
    }
    pub const TAKE_MUG: &str = "Weź kubek";
    pub const TAKE_KNIFE: &str = "Weź nóż";
    pub const CLOSE: &str = "Zamknij szafkę";
    pub const TOOK_KNIFE: &str = "Nóż kuchenny. Do chleba… oczywiście.";
    pub const PUT_KNIFE: &str = "Nóż z powrotem do szafki.";
    pub const NO_KNIVES: &str = "Noży nie ma — ktoś już zabrał.";
    pub fn took_mug(left: u8) -> String {
        format!("Kubek z szafki (zostało {left}).")
    }
    pub const PUT_MUG: &str = "Czysty kubek wraca na półkę.";
    pub const NO_MUGS: &str = "Szafka pusta — wszystkie kubki są brudne. Trzeba pozmywać (zlew / zmywarka).";
    pub const DIRTY_NOT_HERE: &str = "Brudnego nie odkładam do szafki — najpierw umyć.";
    pub const HANDS_FULL: &str = "Mam zajęte ręce.";
    pub const WASHED: &str = "Kubek umyty — czysty, można nalewać.";
    pub fn loaded(n: u8) -> String {
        format!("Kubek w zmywarce ({n}/{}).", super::DISHWASHER_CAP)
    }
    pub const DW_FULL: &str = "Zmywarka pełna — włącz ją (E z pustymi rękami).";
    pub const DW_UNLOAD_FIRST: &str = "W zmywarce są czyste kubki — najpierw rozładuj.";
    pub const DW_STARTED: &str = "Zmywarka ruszyła. Pół godziny i gotowe.";
    pub fn dw_running(min: u32) -> String {
        format!("Zmywarka pracuje, jeszcze ok. {min} min.")
    }
    pub fn dw_unloaded(n: u8) -> String {
        format!("Rozładowane — {n} czystych kubków wraca do szafki.")
    }
    pub const DW_EMPTY: &str = "Zmywarka pusta.";
    pub const NEED_MUG: &str = "Najpierw kubek — jest w szafce obok.";
    pub const DIRTY_MUG: &str = "Ten kubek jest brudny — umyj go w zlewie albo włóż do zmywarki.";
    pub const FRIDGE_FULL: &str = "Lodówka pełna.";
    pub const NOT_FOR_FRIDGE: &str = "Tego się do lodówki nie wkłada.";
    pub fn stored(what: &str) -> String {
        format!("{what} — do lodówki.")
    }
    pub fn milk_added(left: u8) -> String {
        format!("Mleko dolane. (W lodówce jeszcze {left} porcji.)")
    }
    pub const NO_MILK: &str = "Mleko się skończyło — karton jest w sklepie na dole.";
    pub const MILK_NEEDS_COFFEE: &str = "Najpierw kawa w kubku.";
    pub fn carton(total: u8) -> String {
        format!("Karton mleka do lodówki (porcji: {total}).")
    }
    pub const NONE_LEFT: &str = "Już nie ma — jutro rano będą nowe.";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::default_building_path;

    #[test]
    fn the_kitchenette_has_it_all_and_the_dishwasher_cycles() {
        let b = Building::load(&default_building_path()).unwrap();
        let mut k = Kitchen::find(&b).expect("kitchenette");
        let m = b.floor(k.floor).unwrap();
        for t in [k.cupboard, k.dishwasher, k.sink, k.fridge] {
            assert_eq!(m.room_name(m.room_at_tile(t.x, t.y)), "Aneks kuchenny");
        }
        assert_eq!(k.mugs, MUGS);
        k.mugs = 5;
        k.cleaner_load(3, 100);
        assert_eq!((k.dirty, k.running_until), (3, Some(100 + WASH_MINUTES)));
        assert!(!k.tick(110));
        assert!(k.tick(100 + WASH_MINUTES));
        assert_eq!((k.dirty, k.washed), (0, 3));
        k.return_mugs(10);
        assert_eq!(k.mugs, MUGS);
    }
}
