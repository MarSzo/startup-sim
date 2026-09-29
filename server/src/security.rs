//! Shoplifting (backlog): leaving the shop with unpaid goods sets off the
//! gate alarm and the security guard runs after you. Caught = the goods go
//! back and a warning; a second theft the same day, or running away from the
//! guard, brings the police: a patrol car pulls up in front of the building
//! and an officer comes for you wherever you are - a fine from your wallet
//! (or in absentia, if you get away).

use crate::outside::Outside;
use crate::sim::Pos;

/// Police fine (grosze): 300 zł, or whatever is in the wallet.
pub const FINE: i64 = 30_000;
/// Stress: caught by the guard / by the police.
pub const GUARD_STRESS: i32 = 10;
pub const POLICE_STRESS: i32 = 25;
/// The second theft of the day goes straight to the police.
/// Caught: can't move for a moment (3 s by the guard, 6 s by the police).
pub const GUARD_HOLD_TICKS: u32 = 60;
pub const POLICE_HOLD_TICKS: u32 = 120;
pub const THEFTS_FOR_POLICE: u8 = 2;

/// Where the officer gets out of the patrol car (sidewalk by the entrance).
pub fn officer_spawn(o: &Outside) -> Pos {
    Pos::tile_center(o.police.x, o.police.y)
}

/// Street waypoints of the patrol car: in from the east, stop at the
/// entrance, off to the west.
pub fn car_path(o: &Outside) -> Vec<Pos> {
    vec![o.street_east(), o.street(o.police.x)]
}
pub fn car_exit(o: &Outside) -> Pos {
    o.street_west_off(6)
}

/// A police call: the car on its way / parked, the officer once out.
#[derive(Debug, Clone)]
pub struct PoliceCall {
    pub target: u16,
    pub car: u16,
    pub officer: Option<u16>,
}

pub mod lines {
    use crate::shop::zl;
    pub const GUARD_CAUGHT: &str = "Mam cię! Towar wraca na półkę. Następnym razem proszę płacić przy kasie.";
    pub const GUARD_PAID: &str = "A, zapłacone. W porządku — ale następnym razem od razu do kasy.";
    pub const GUARD_POLICE_AGAIN: &str = "To już drugi raz dzisiaj. Wzywam policję.";
    pub const GUARD_ESCAPED: &str = "Ucieka! Dzwonię na policję.";
    pub const CALLED: &str = "Halo, policja? Kradzież w sklepie na parterze.";
    pub fn police_fine(fine: i64) -> String {
        if fine == 0 {
            return "Policja. Kradzież w sklepie — towar zabezpieczony. Portfel pusty, więc tym razem pouczenie. Następnym razem sprawa w sądzie.".into();
        }
        format!("Policja. Kradzież w sklepie — mandat {}, towar zabezpieczony. Proszę się więcej nie wygłupiać.", zl(fine))
    }
    pub fn fine_by_mail(fine: i64) -> String {
        if fine == 0 {
            return "Policja nie złapała mnie na miejscu. Całe szczęście, że i tak nie mam z czego zapłacić.".into();
        }
        format!("Policja nie złapała mnie na miejscu, ale mandat ({}) i tak przyszedł na konto.", zl(fine))
    }
    pub const SHAME: &str = "Ale wstyd… wszyscy patrzą.";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::{default_building_path, Building};

    #[test]
    fn the_patrol_car_stops_on_the_street_and_the_officer_on_the_sidewalk() {
        let b = Building::load(&default_building_path()).unwrap();
        let m = b.floor(0).unwrap();
        for p in car_path(&b.outside) {
            let (x, y) = p.tile();
            assert_eq!(m.tile_type(x, y), Some("street"));
        }
        let (x, y) = officer_spawn(&b.outside).tile();
        assert_eq!(m.tile_type(x, y), Some("sidewalk"));
    }
}
