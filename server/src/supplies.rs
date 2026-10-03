//! What the office keeps for itself: the first-aid cabinet at the
//! reception, the storeroom upstairs (cola, cookies - the key hangs at the
//! reception and can only be taken while nobody's at the desk) and rolling
//! your own cigarettes (the minigame's result decides the quality).

use crate::inventory::kind;

/// Dialog id of a cabinet / the storeroom shelves (the last free one).
pub const DIALOG: u8 = 255;
/// Every morning: this many of each medicine, of cola and of cookies.
pub const MEDICINES: [(u8, &str); 4] = [
    (kind::PAINKILLER, "Apap — na ból głowy i kaca"),
    (kind::CHARCOAL, "Węgiel aktywny — na brzuch"),
    (kind::VITAMIN, "Witamina C — na zmęczenie"),
    (kind::PLASTER, "Plaster — na rany po bójkach"),
];
pub const MEDICINE_STOCK: u8 = 3;
pub const STOREROOM: [(u8, &str); 2] = [(kind::COLA, "Coca-Cola"), (kind::STORE_COOKIES, "Ciastka")];
pub const STOREROOM_STOCK: u8 = 6;
/// The liquor cabinet in a meeting room (its key hidden somewhere).
pub const BAR: [(u8, &str); 3] = [(kind::WHISKY, "Whisky"), (kind::COGNAC, "Koniak"), (kind::VODKA, "Wódka")];
pub const BAR_STOCK: u8 = 2;
/// Where the cabinet's key may be hidden (tile types you can search with E).
pub const HIDING: [&str; 3] = ["plant", "bin", "wardrobe"];
/// The receptionist's lunch break (minutes of the day): the key is free.
pub const BREAK_FROM: u32 = 12 * 60;
pub const BREAK_UNTIL: u32 = 12 * 60 + 30;
/// Reach of the hook, the cabinet, the storeroom shelves (1.5 tiles).
pub const REACH: i32 = crate::sim::TILE_UNITS * 3 / 2;
/// A roll below this falls apart.
pub const CRUMBLES_BELOW: u8 = 30;

/// What's left today (`kind` -> pieces).
#[derive(Debug, Clone, Default)]
pub struct Stock(pub std::collections::BTreeMap<u8, u8>);

impl Stock {
    pub fn morning() -> Stock {
        let mut s = Stock::default();
        for (k, _) in MEDICINES {
            s.0.insert(k, MEDICINE_STOCK);
        }
        for (k, _) in STOREROOM {
            s.0.insert(k, STOREROOM_STOCK);
        }
        for (k, _) in BAR {
            s.0.insert(k, BAR_STOCK);
        }
        s
    }

    pub fn left(&self, k: u8) -> u8 {
        self.0.get(&k).copied().unwrap_or(0)
    }

    pub fn take(&mut self, k: u8) -> bool {
        match self.0.get_mut(&k) {
            Some(n) if *n > 0 => {
                *n -= 1;
                true
            }
            _ => false,
        }
    }
}

/// How the roll came out (0..100) in words.
pub fn roll_name(quality: u8) -> &'static str {
    match quality {
        q if q < CRUMBLES_BELOW => "Skręt (rozsypujący się)",
        q if q < 55 => "Skręt (krzywy)",
        q if q < 85 => "Skręt (zgrabny)",
        _ => "Skręt (idealny)",
    }
}

pub mod lines {
    pub const CABINET: &str = "Apteczka: co bierzesz?";
    pub const STOREROOM: &str = "Magazynek: zapasy firmy (ciii…).";
    pub const CLOSE: &str = "Nic, dziękuję";
    pub fn item(name: &str, left: u8) -> String {
        format!("{name} ({left})")
    }
    pub const EMPTY: &str = "Pusto — trzeba poczekać do jutra.";
    pub const HANDS_FULL: &str = "Najpierw muszę coś odłożyć.";
    pub const KEY_NO: &str = "Klucz do magazynku? A po co Panu/Pani? Nie ma mowy.";
    pub const KEY_TAKEN: &str = "Ciii… klucz do magazynku. Nikt nie widział.";
    pub const KEY_BACK: &str = "Klucz z powrotem na haczyk.";
    pub const KEY_GONE: &str = "Haczyk pusty — ktoś już wziął klucz.";
    pub const ROLL_FIRST: &str = "Tytoń i bibułki — trzeba skręcić (F, mini-gra).";
    pub const ROLLED: &str = "Skręcone!";
    pub const CRUMBLED: &str = "Rozsypał się w palcach… Szkoda tytoniu.";
    pub const PAINKILLER: &str = "Apap. Głowa przestaje pękać.";
    pub const CHARCOAL: &str = "Węgiel. Brzuch się uspokaja.";
    pub const VITAMIN: &str = "Witamina C. Od razu jakby więcej energii.";
    pub const PLASTER: &str = "Plaster. Od razu lepiej.";
    pub const COLA: &str = "Zimna cola z magazynu. Smakuje lepiej, bo za darmo.";
    pub const COOKIES: &str = "Ciastka z magazynu. Kradzione nie tuczy.";
    pub const KEY: &str = "Klucz do magazynku na piętrze (piwnica, a jednak na piętrze).";
    pub const BAR: &str = "Barek: co nalewamy?";
    pub const BAR_LOCKED: &str = "Barek zamknięty na kluczyk. Ciekawe, gdzie go schowali…";
    pub const BAR_KEY: &str = "Mały kluczyk. Pasuje do barku w sali spotkań?";
    pub const FOUND_KEY: &str = "O! Mały kluczyk… Do czego on może być?";
    pub fn nothing(kind: &str) -> &'static str {
        match kind {
            "plant" => "Grzebiesz w doniczce… tylko ziemia i pet.",
            "bin" => "W koszu? Ogryzek, kubek i stare CV. Nic.",
            _ => "W szafie płaszcz i parasol. Nic więcej.",
        }
    }
    pub const WHISKY: &str = "Whisky z barku zarządu. Smakuje jak premia.";
    pub const COGNAC: &str = "Koniak. Elegancko, jak na spotkaniu z inwestorem.";
    pub const VODKA: &str = "Wódeczka z barku. Na zdrowie!";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stock_runs_out_and_rolls_have_names() {
        let mut s = Stock::morning();
        for _ in 0..MEDICINE_STOCK {
            assert!(s.take(kind::PAINKILLER));
        }
        assert!(!s.take(kind::PAINKILLER) && s.left(kind::COLA) == STOREROOM_STOCK);
        assert!(!s.take(kind::BEER), "not in stock at all");
        assert_eq!(roll_name(10), "Skręt (rozsypujący się)");
        assert_eq!(roll_name(95), "Skręt (idealny)");
    }
}
