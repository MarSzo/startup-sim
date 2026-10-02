//! Ordering lunch (backlog 9b): an app on the office computer. Pick a dish
//! from a few (fictional) restaurants, pay from the computer owner's wallet,
//! the courier brings it to the reception on floor 4 after the delivery time
//! (later in the rain); the receptionist lets you know, E at the reception
//! hands over the box.
//!
//! Dishes are shop products (kinds 28..) with a price and an effect, only
//! not on any shelf.

use crate::inventory::kind;

/// Orders are taken 10:00 - 15:00.
pub const ORDER_FROM: u32 = 10 * 60;
pub const ORDER_TO: u32 = 15 * 60;
/// Delivery time varies by this much (minutes, +/-).
pub const ETA_SPREAD: u32 = 10;
/// Rain / storm: the courier is late.
pub const RAIN_DELAY: u32 = 15;

/// (dish kind, restaurant, typical delivery minutes).
pub const MENU: &[(u8, &str, u32)] = &[
    (kind::PIEROGI, "Pierogarnia u Zosi", 40),
    (kind::PIZZA, "Pizza Bella", 45),
    (kind::SUSHI, "Sushi Koi", 50),
    (kind::SCHNITZEL, "Bar Mleczny „Pod Kogutem”", 35),
    (kind::SALAD, "Zielona Miska", 30),
    (kind::KEBAB, "Kebab u Ahmeda", 25),
];

pub fn menu_entry(k: u8) -> Option<&'static (u8, &'static str, u32)> {
    MENU.iter().find(|m| m.0 == k)
}

pub mod lines {
    pub fn arrived(dish: &str) -> String {
        format!("Kurier był! {dish} czeka na recepcji.")
    }
    pub fn handed_over(dish: &str) -> String {
        format!("Proszę — {dish}. Smacznego!")
    }
    pub const HANDS_FULL: &str = "Obiad przyjechał, ale musisz mieć wolne ręce.";
}

/// Order state for the app.
pub mod state {
    pub const NONE: u8 = 0;
    /// On its way (eta).
    pub const ORDERED: u8 = 1;
    /// At the reception, waiting to be picked up.
    pub const WAITING: u8 = 2;
    /// Outside ordering hours.
    pub const CLOSED: u8 = 3;
}

#[derive(Debug, Clone)]
pub struct Order {
    /// Account (player id) that ordered / pays / gets it.
    pub owner: u16,
    pub dish: u8,
    /// Game minute (`Clock::total_minutes`) it arrives at the reception.
    pub arrives: u32,
    pub delivered: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shop;

    #[test]
    fn every_dish_is_a_priced_product_that_feeds_you() {
        for (k, restaurant, eta) in MENU {
            let p = shop::product(*k).unwrap_or_else(|| panic!("dish {k} from {restaurant}"));
            assert!(p.price > 0 && p.effect.hunger <= -40, "{}", p.name);
            assert!((20..=60).contains(eta));
            assert!(!crate::inventory::is_small(*k), "a lunch box takes both hands");
        }
    }
}
