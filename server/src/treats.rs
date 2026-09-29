//! Sweets in the chill room (backlog 9b): now and then HR puts a tray of
//! doughnuts, cookies or cheesecake on the chill-room table - a limited
//! number, first come first served - and announces it on #ogólny.
//!
//! (Also here: fruit from the bowl is sometimes past its best - see
//! `STALE_FRUIT_PERCENT`; eating it sends you running to the toilet.)

use crate::building::Building;
use crate::inventory::kind;
use crate::sim::{Body, Pos, TILE_UNITS};

/// Tray drops per working day, between these hours.
pub const DROPS_MIN: u32 = 1;
pub const DROPS_MAX: u32 = 2;
pub const DROP_FROM: u32 = 9 * 60;
pub const DROP_TO: u32 = 16 * 60;
/// Pieces on a tray.
pub const PIECES_MIN: u8 = 4;
pub const PIECES_MAX: u8 = 8;
/// Reach of the tray (standing at the table).
pub const REACH: i32 = TILE_UNITS * 3 / 2;
/// Chance that fruit from the bowl is stale.
pub const STALE_FRUIT_PERCENT: u32 = 15;

/// Where the tray stands: on the chill-room table (the map's "tray" place).
pub fn tray_pos(b: &Building) -> Option<(u8, Pos)> {
    b.tray.map(|(f, t)| (f, Pos::tile_center(t.x, t.y)))
}

pub const KINDS: [u8; 3] = [kind::DONUT, kind::COOKIE, kind::CHEESECAKE];

pub fn announcement(k: u8, pieces: u8) -> String {
    let what = match k {
        kind::DONUT => "pączki",
        kind::COOKIE => "ciastka",
        _ => "sernik",
    };
    format!("W chill roomie czekają {what} ({pieces} szt.)! Kto pierwszy, ten lepszy.")
}

pub mod lines {
    pub const TAKEN: &str = "Mniam, słodkie się należy.";
    pub const HANDS_FULL: &str = "Nie mam gdzie tego włożyć.";
    pub const STALE_EATEN: &str = "Oj… chyba owoc był nieświeży. Do łazienki, szybko!";
    pub const CURED: &str = "Uff. Żołądek wrócił do normy.";
}

#[derive(Debug, Clone)]
pub struct Tray {
    pub handle: u16,
    pub kind: u8,
    pub pieces: u8,
}

pub fn in_reach(b: &Building, body: &Body) -> bool {
    tray_pos(b).is_some_and(|(floor, p)| body.floor == floor && (p.x - body.pos.x).pow(2) + (p.y - body.pos.y).pow(2) <= REACH * REACH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::{default_building_path, Building};

    #[test]
    fn the_tray_stands_on_the_chill_room_table_and_can_be_reached() {
        let b = Building::load(&default_building_path()).unwrap();
        let (f, p) = tray_pos(&b).unwrap();
        let m = b.floor(f).unwrap();
        let (tx, ty) = p.tile();
        assert_eq!(m.tile_type(tx, ty), Some("table"));
        assert_eq!(m.room_name(m.room_at(p.x, p.y)), "Chill room");
        // From the free tile below the table.
        assert!(in_reach(&b, &Body::at(f, Pos::tile_center(tx, ty + 1))));
        assert!(!in_reach(&b, &Body::at(f, Pos::tile_center(tx - 5, ty + 2))));
    }
}
