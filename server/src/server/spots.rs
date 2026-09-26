//! Sofas, toilets, ashtrays, sinks and the fruit bowl.

use crate::cleaning;
use crate::inventory::{self, kind as item_kind, Item};
use crate::needs::{self, Rest, SpotKind};
use crate::protocol as proto;
use crate::sim::Body;
use crate::shop;

use super::player::refresh;
use super::Server;

/// Fruit in the bowl (label of the item).
pub(super) const FRUITS: [&str; 4] = ["Jabłko", "Banan", "Gruszka", "Mandarynka"];

impl Server {
    /// E at a sofa, toilet, ashtray or the fruit bowl. `None` = nothing in
    /// reach; `Some(line)` = handled. E while resting gets you up.
    pub(super) fn use_spot(&mut self, pid: u16, body: &Body) -> Option<Option<String>> {
        let p = self.players.get_mut(&pid)?;
        if p.rest.take().is_some() {
            return Some(None);
        }
        let spot = needs::spot_in_reach(&self.spots, body)?.clone();
        let (floor, pos) = (p.body.floor, p.body.pos);
        let line = match spot.kind {
            SpotKind::FruitBowl => {
                if !p.inventory.has_room() {
                    return Some(Some(needs::lines::HANDS_FULL.into()));
                }
                let fruit = FRUITS[self.rng.usize(..FRUITS.len())];
                // Now and then the fruit is past its best (you can tell, if you look).
                let stale = self.rng.u32(0..100) < self.cfg.stale_fruit_percent;
                let label = if stale { format!("{fruit} (nie pierwszej świeżości)") } else { fruit.into() };
                let item = Item { stale, ..self.mint_item(item_kind::FRUIT, label) };
                self.give(pid, item);
                format!("{} {}", needs::lines::FRUIT, fruit)
            }
            SpotKind::Sofa => {
                p.rest = Some((Rest::Sofa, floor, pos));
                needs::lines::SOFA.into()
            }
            SpotKind::Ashtray => {
                // One cigarette from a paid pack.
                let slot = std::iter::once(&mut p.inventory.hands)
                    .chain(p.inventory.pockets.iter_mut())
                    .find(|s| s.as_ref().is_some_and(|i| i.kind == item_kind::CIGARETTES && !i.unpaid && i.count > 0));
                let Some(slot) = slot else {
                    return Some(Some(shop::lines::NO_CIGARETTES.into()));
                };
                inventory::take_piece(slot);
                refresh(p);
                p.rest = Some((Rest::Smoking { until: self.tick + needs::SMOKE_TICKS }, floor, pos));
                needs::lines::SMOKE.into()
            }
            SpotKind::Sink if p.inventory.held_kind() == item_kind::EMPTY_CUP => {
                p.inventory.take_hands();
                refresh(p);
                cleaning::lines::WASHED.into()
            }
            SpotKind::Sink => {
                p.rest = Some((Rest::Washing { until: self.tick + needs::WASH_TICKS }, floor, pos));
                needs::lines::WASHING.into()
            }
            SpotKind::Sanitizer => {
                p.needs.sanitize();
                needs::lines::SANITIZED.into()
            }
            SpotKind::Toilet => {
                p.rest = Some((Rest::Toilet, floor, pos));
                p.needs.use_toilet();
                let mine = match p.profile.gender {
                    proto::gender::FEMALE => Some("female"),
                    proto::gender::MALE => Some("male"),
                    _ => None,
                };
                match (mine, spot.gender.as_deref()) {
                    (Some(m), Some(g)) if m != g => {
                        p.needs.add_stress(5);
                        needs::lines::WRONG_BATHROOM.into()
                    }
                    _ => needs::lines::TOILET.into(),
                }
            }
        };
        Some(Some(line))
    }
}
