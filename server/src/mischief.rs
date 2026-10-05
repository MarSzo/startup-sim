//! Toilet humour and fights: what can be done to things and to people (the
//! R menu and X), how much it hurts, and what's said about it.

use crate::sim::TILE_UNITS;

/// Drank something somebody peed in: this often it comes back up.
pub const SICK_PERCENT: u32 = 50;
/// How long peeing / pooping on purpose takes (standing there).
pub const PEE_TICKS: u32 = 3 * 20;
pub const POOP_TICKS: u32 = 4 * 20;
/// Reach for a machine, a mug in somebody's hands or a punch: 1.5 tiles.
pub const REACH: i32 = TILE_UNITS * 3 / 2;
/// Coffees that come out "special" after peeing into a machine.
pub const MACHINE_DOSES: u8 = 3;
/// Health taken by a punch / a stab.
pub const PUNCH_DAMAGE: i32 = 10;
pub const KNIFE_DAMAGE: i32 = 35;
/// Between two punches (1 s) / stabs (1.5 s).
pub const PUNCH_COOLDOWN: u32 = 20;
pub const KNIFE_COOLDOWN: u32 = 30;
/// The swing shows this long (`activity::ATTACKING`).
pub const SWING_TICKS: u32 = 8;
/// Knocked out: a minute on the floor.
pub const KNOCKOUT_TICKS: u32 = 60 * 20;
/// Five cigarettes one after another: sick. "After another" = lit within
/// this long of the last one going out (30 s).
pub const CHAIN_SMOKES: u8 = 5;
pub const CHAIN_GAP_TICKS: u32 = 30 * 20;
/// Dialog ids of the R menu and the cupboard (after the reprimand's 200..250).
pub const MENU_ID: u8 = 250;
pub const CUPBOARD_ID: u8 = 251;

pub mod lines {
    pub const MENU: &str = "Co by tu zmalować?";
    pub const CANCEL: &str = "Nic, jednak nie";
    pub const NOTHING: &str = "Nic tu nie zmaluję.";
    pub const PEE_FLOOR: &str = "Nasikaj na podłogę";
    pub const POOP_FLOOR: &str = "Zesraj się na podłogę";
    pub const PEE_MACHINE: &str = "Nasikaj do ekspresu";
    pub fn pee_cup(nick: &str) -> String {
        format!("Nasikaj do kubka: {nick}")
    }
    pub const DID_PEE_FLOOR: &str = "Ech, nie chciało mi się iść do łazienki.";
    pub const DID_POOP_FLOOR: &str = "No co? Natura wzywa.";
    pub const DID_PEE_MACHINE: &str = "Specjalny dodatek do kawy. Hehe.";
    pub const DID_PEE_CUP: &str = "Psst… mała niespodzianka.";
    pub const NO_PEE: &str = "Nie chce mi się teraz sikać.";
    pub const NO_POOP: &str = "Nie mam teraz czym.";
    pub const TOO_LATE: &str = "Już za późno, nie ma tu tego.";
    pub const WITNESS: &str = "Ej! Ja wszystko widzę!";
    pub const OBLIVIOUS: &str = "Hm? Coś mi kapnęło na rękę…";
    pub const TASTE: &str = "Fuj! Co to za smak?! Ktoś coś dosypał…";
    pub const TASTE_SICK: &str = "Fuj! To… to nie jest kawa! Bleeeh…";
    pub const OUCH: [&str; 4] = ["Auć!", "Ała! Za co?!", "Ej, spokojnie!", "Oddam ci!"];
    pub const STABBED: &str = "Aaa! Nóż! Ratunku!";
    pub const KNOCKED_OUT: &str = "Ooo… gwiazdki…";
    pub const COME_ROUND: &str = "Co… co się stało? Kto mnie walnął?";
    pub const CHAIN_SMOKE_SICK: &str = "Pięć pod rząd… niedobrze mi… Bleeeh…";
    pub const GUARD_CAUGHT: &str = "Spokój! Bijatyki tu nie będzie. Chwilę postoisz.";
    pub fn knife_reprimand(nick: &str, n: u8, fired: bool) -> String {
        if fired {
            format!("{nick},\n\nnóż w biurze to koniec. To już {n}. nagana. Rozwiązujemy umowę.\n\nZarząd")
        } else {
            format!("{nick},\n\nza atak nożem na współpracownika udzielamy nagany ({n}/3). Przy trzeciej — zwolnienie.\n\nZarząd")
        }
    }
}
