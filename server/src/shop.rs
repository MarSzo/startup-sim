//! The shop on the ground floor: take goods off the shelves (unpaid), pay at
//! the till (cashier NPC), and don't walk out without paying - the security
//! gate beeps and the goods stay in the shop.
//!
//! Money is in grosze (1 zł = 100 gr). Goods are items (`inventory`) whose
//! kind is a product id; eating / drinking them (F) changes the needs.

use crate::building::Building;
use crate::inventory::kind;
use crate::map::Rect;
use crate::sim::{Body, Pos, TILE_UNITS};

/// Advance paid with the contract (until there is a daily salary).
pub const ADVANCE: i64 = 200_00;

/// Reach of a shelf (the tile in front of you).
pub const SHELF_REACH: i32 = TILE_UNITS * 5 / 4;

/// What eating / drinking does to the needs (points; + / -).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Effect {
    pub hunger: i32,
    pub energy: i32,
    pub stress: i32,
    pub bladder: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Product {
    pub kind: u8,
    pub name: &'static str,
    /// Grosze.
    pub price: i64,
    pub effect: Effect,
    /// Pieces in one package (cigarettes).
    pub count: u8,
    /// What you say when you have it.
    pub line: &'static str,
}

const fn e(hunger: i32, energy: i32, stress: i32, bladder: i32) -> Effect {
    Effect { hunger, energy, stress, bladder }
}

pub const PRODUCTS: &[Product] = &[
    Product {
        kind: kind::SANDWICH_CHEESE,
        name: "Kanapka z serem",
        price: 12_00,
        effect: e(-35, 0, 0, 0),
        count: 1,
        line: "Kanapka z serem — klasyk.",
    },
    Product {
        kind: kind::SANDWICH_HAM,
        name: "Kanapka z szynką",
        price: 14_00,
        effect: e(-40, 0, 0, 0),
        count: 1,
        line: "Kanapka z szynką, mniam.",
    },
    Product { kind: kind::WRAP, name: "Wrap wege", price: 13_00, effect: e(-35, 2, -2, 0), count: 1, line: "Wrap wege. Lekko i zdrowo." },
    Product {
        kind: kind::BURGER,
        name: "Hamburger",
        price: 18_00,
        effect: e(-50, -5, -3, 0),
        count: 1,
        line: "Burger! Teraz by się drzemka przydała…",
    },
    Product { kind: kind::FRIES, name: "Frytki", price: 9_00, effect: e(-25, -2, -3, 0), count: 1, line: "Frytki, tłusto i pysznie." },
    Product { kind: kind::BUN, name: "Drożdżówka", price: 6_00, effect: e(-15, 5, 0, 0), count: 1, line: "Drożdżówka z budyniem." },
    Product { kind: kind::BAR, name: "Batonik", price: 5_00, effect: e(-10, 8, 0, 0), count: 1, line: "Batonik — cukier leci." },
    Product { kind: kind::CHIPS, name: "Chipsy", price: 7_00, effect: e(-12, 0, -4, 0), count: 1, line: "Chrup, chrup." },
    Product { kind: kind::WATER, name: "Woda", price: 4_00, effect: e(0, 3, -1, 10), count: 1, line: "Woda. Nawodnienie to podstawa." },
    Product {
        kind: kind::ENERGY_DRINK,
        name: "Energetyk",
        price: 8_00,
        effect: e(0, 30, 8, 10),
        count: 1,
        line: "Energetyk! Serce przyspiesza…",
    },
    Product { kind: kind::JUICE, name: "Sok pomarańczowy", price: 6_00, effect: e(-5, 5, 0, 8), count: 1, line: "Sok pomarańczowy." },
    Product {
        kind: kind::MILK,
        name: "Mleko (karton)",
        price: 4_50,
        effect: e(-5, 2, 0, 6),
        count: 1,
        line: "Łyk mleka prosto z kartonu. Nikt nie widział.",
    },
    Product { kind: kind::BEER, name: "Piwo", price: 7_00, effect: e(0, -5, -15, 20), count: 1, line: "Piwko… w pracy? Cicho sza." },
    Product { kind: kind::WINE, name: "Wino", price: 25_00, effect: e(0, -10, -25, 10), count: 1, line: "Wino. To był ciężki dzień." },
    Product {
        kind: kind::MALPKA,
        name: "Małpka (Żołądkowa Gorzka z miętą)",
        price: 10_00,
        effect: e(0, -8, -20, 5),
        count: 1,
        line: "Na odwagę przed review.",
    },
    Product { kind: kind::CIGARETTES, name: "Papierosy", price: 18_00, effect: e(0, 0, 0, 0), count: 20, line: "" },
    Product { kind: kind::TOBACCO, name: "Tytoń do skręcania", price: 22_00, effect: e(0, 0, 0, 0), count: 10, line: "" },
    Product { kind: kind::UMBRELLA, name: "Parasol", price: 25_00, effect: e(0, 0, 0, 0), count: 1, line: "" },
    // Chill-room sweets (free, not on the shelves - see `treats`).
    Product { kind: kind::DONUT, name: "Pączek", price: 0, effect: e(-12, 6, -5, 0), count: 1, line: "Pączek z różą. Niebo." },
    Product { kind: kind::COOKIE, name: "Ciastko", price: 0, effect: e(-6, 4, -4, 0), count: 1, line: "Ciastko do kawy — idealnie." },
    Product { kind: kind::CHEESECAKE, name: "Kawałek sernika", price: 0, effect: e(-15, 3, -6, 0), count: 1, line: "Sernik jak u babci." },
    // Lunch (ordered in the app, delivered to the reception - see `lunch`).
    Product { kind: kind::PIEROGI, name: "Pierogi ruskie", price: 24_00, effect: e(-55, -5, -5, 0), count: 1, line: "Pierogi jak u mamy." },
    Product {
        kind: kind::PIZZA,
        name: "Pizza margherita",
        price: 32_00,
        effect: e(-60, -8, -6, 0),
        count: 1,
        line: "Pizza! Kawałek dla kolegi? Nie.",
    },
    Product {
        kind: kind::SUSHI,
        name: "Zestaw sushi",
        price: 45_00,
        effect: e(-45, 2, -8, 5),
        count: 1,
        line: "Sushi, pałeczki, elegancja.",
    },
    Product {
        kind: kind::SCHNITZEL,
        name: "Schabowy z ziemniakami",
        price: 22_00,
        effect: e(-65, -10, -4, 0),
        count: 1,
        line: "Schabowy. Teraz by się drzemka przydała…",
    },
    Product {
        kind: kind::SALAD,
        name: "Sałatka z kurczakiem",
        price: 27_00,
        effect: e(-40, 4, -3, 0),
        count: 1,
        line: "Sałatka — lekko i zdrowo.",
    },
    Product { kind: kind::KEBAB, name: "Kebab", price: 25_00, effect: e(-60, -8, -5, 0), count: 1, line: "Kebab, ostry sos. Warto było." },
];

pub fn product(k: u8) -> Option<&'static Product> {
    PRODUCTS.iter().find(|p| p.kind == k)
}

/// A shelf (rectangle of shelf tiles) with what's on it.
#[derive(Debug, Clone)]
pub struct Shelf {
    pub id: u8,
    pub title: &'static str,
    pub floor: u8,
    pub area: Rect,
    pub goods: &'static [u8],
}

/// What each shelf holds (where it stands comes from the map's places).
const SHELF_GOODS: [(u8, &str, &[u8]); 6] = {
    use kind::*;
    [
        (1, "Kanapki", &[SANDWICH_CHEESE, SANDWICH_HAM, WRAP]),
        (2, "Fast food", &[BURGER, FRIES]),
        (3, "Przekąski", &[BUN, BAR, CHIPS]),
        (4, "Napoje", &[WATER, ENERGY_DRINK, JUICE, MILK]),
        (5, "Alkohol i papierosy", &[BEER, WINE, MALPKA, CIGARETTES, TOBACCO]),
        (6, "Parasole", &[UMBRELLA]),
    ]
};

/// The shop's shelves, placed where the map says.
pub fn shelves(b: &Building) -> Vec<Shelf> {
    let mut out = Vec::new();
    for (f, m) in b.active_floors() {
        for (id, area) in crate::outside::shelf_areas(&m.places) {
            if let Some(&(_, title, goods)) = SHELF_GOODS.iter().find(|g| g.0 == id) {
                out.push(Shelf { id, title, floor: f, area, goods });
            }
        }
    }
    out
}

/// Shelf nearest to `body` within reach.
pub fn shelf_in_reach<'a>(shelves: &'a [Shelf], body: &Body) -> Option<&'a Shelf> {
    shelves
        .iter()
        .filter(|s| s.floor == body.floor)
        .filter_map(|s| {
            let mut best = i32::MAX;
            for y in s.area.y..s.area.y + s.area.h {
                for x in s.area.x..s.area.x + s.area.w {
                    let c = Pos::tile_center(x, y);
                    best = best.min((c.x - body.pos.x).pow(2) + (c.y - body.pos.y).pow(2));
                }
            }
            (best <= SHELF_REACH * SHELF_REACH).then_some((s, best))
        })
        .min_by_key(|&(_, d)| d)
        .map(|(s, _)| s)
}

/// "12,50 zł".
pub fn zl(gr: i64) -> String {
    format!("{},{:02} zł", gr / 100, (gr % 100).abs())
}

pub mod lines {
    pub const NO_ROOM: &str = "Nie mam już gdzie tego włożyć.";
    pub const NOTHING_TO_PAY: &str = "Dzień dobry! Proszę coś wybrać z półek — płaci się tutaj.";
    pub const ALARM: &str = "Piiip! Piiip! Niezapłacony towar!";
    pub const PAY_FIRST: &str = "Najpierw trzeba zapłacić przy kasie.";
    pub const NO_CIGARETTES: &str = "Nie mam papierosów — sklep jest na parterze.";
    pub fn paid(total: i64, left: i64) -> String {
        format!("Razem {}. Dziękuję! Zostało Ci {}.", super::zl(total), super::zl(left))
    }
    pub fn too_poor(total: i64, money: i64) -> String {
        format!("Razem {}, a masz tylko {}. Odłóż coś na półkę.", super::zl(total), super::zl(money))
    }
}

/// Every shelf is on the map, on shelf tiles (catches map edits).
pub fn check(b: &Building) -> Result<(), String> {
    let all = shelves(b);
    for (id, title, _) in SHELF_GOODS {
        if !all.iter().any(|s| s.id == id) {
            return Err(format!("shelf {id} ({title}) is not on the map"));
        }
    }
    for s in all {
        let m = b.floor(s.floor).ok_or("shop floor missing")?;
        for y in s.area.y..s.area.y + s.area.h {
            for x in s.area.x..s.area.x + s.area.w {
                if m.tile_type(x, y) != Some("shelf") {
                    return Err(format!("shelf {} ({}): ({x},{y}) is not a shelf tile", s.id, s.title));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::default_building_path;

    #[test]
    fn shelves_are_on_the_map_and_every_product_is_sold_once() {
        let b = Building::load(&default_building_path()).unwrap();
        check(&b).unwrap();
        let mut sold: Vec<u8> = shelves(&b).iter().flat_map(|s| s.goods.iter().copied()).collect();
        sold.sort();
        let lunch: Vec<u8> = crate::lunch::MENU.iter().map(|m| m.0).collect();
        let mut all: Vec<u8> = PRODUCTS.iter().filter(|p| p.price > 0 && !lunch.contains(&p.kind)).map(|p| p.kind).collect();
        all.sort();
        assert_eq!(sold, all);
    }

    #[test]
    fn a_malpka_costs_ten_zloty_and_fits_a_pocket() {
        let m = product(kind::MALPKA).unwrap();
        assert_eq!(m.price, 10_00);
        assert!(m.name.contains("Żołądkowa Gorzka z miętą"));
        assert!(crate::inventory::is_small(kind::MALPKA), "a 100 ml bottle goes in a pocket");
        let b = Building::load(&default_building_path()).unwrap();
        let shelf = shelves(&b).into_iter().find(|s| s.goods.contains(&kind::MALPKA)).unwrap();
        assert_eq!(shelf.title, "Alkohol i papierosy");
    }

    #[test]
    fn money_formatting() {
        assert_eq!(zl(ADVANCE), "200,00 zł");
        assert_eq!(zl(12_50), "12,50 zł");
        assert_eq!(zl(5), "0,05 zł");
    }

    #[test]
    fn reach_picks_the_shelf_in_front() {
        let s = shelves(&Building::load(&default_building_path()).unwrap());
        let body = Body::at(0, Pos::tile_center(22, 47)); // under the sandwiches
        let got = shelf_in_reach(&s, &body).map(|s| s.id);
        assert!(matches!(got, Some(1)), "{got:?}");
        assert!(shelf_in_reach(&s, &Body::at(0, Pos::tile_center(24, 55))).is_none());
    }
}
