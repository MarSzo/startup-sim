//! Sweets on the chill-room table.

use crate::clock;
use crate::computer;
use crate::npc;
use crate::sim::Body;
use crate::shop;
use crate::treats::{self, Tray};

use super::Server;

impl Server {
    // ------------------------------------------------------------ treats

    /// Today's tray drops (1-2 random times, 9:00-16:00, still ahead).
    pub(super) fn schedule_treats(&mut self) {
        let day0 = (self.clock.day - 1) * clock::MIN_PER_DAY;
        let now = self.clock.total_minutes();
        let n = self.rng.u32(treats::DROPS_MIN..=treats::DROPS_MAX);
        let mut drops: Vec<u32> = (0..n)
            .map(|_| day0 + self.rng.u32(treats::DROP_FROM..=treats::DROP_TO))
            .filter(|&t| t > now)
            .collect();
        drops.sort();
        self.treat_drops = drops;
    }

    /// A fresh tray on the chill-room table; HR tells everyone.
    pub(super) fn put_tray(&mut self) {
        let kind = treats::KINDS[self.rng.usize(..treats::KINDS.len())];
        let pieces = self.rng.u8(treats::PIECES_MIN..=treats::PIECES_MAX);
        let handle = self.alloc_handle();
        self.tray = Some(Tray { handle, kind, pieces });
        let text = treats::announcement(kind, pieces);
        if let Some(hr) = self.npcs.iter().find(|n| n.role == npc::Role::Hr) {
            let (id, name) = (hr.id, hr.name.clone());
            self.messenger.post_system(computer::conv::GENERAL, id, &name, &text);
        }
        self.log(format!("* treats: {text}"));
    }

    /// E at the tray: one piece.
    pub(super) fn take_treat(&mut self, pid: u16, body: &Body) -> Option<String> {
        if !treats::in_reach(&self.building, body) {
            return None;
        }
        let tray = self.tray.as_mut()?;
        let kind = tray.kind;
        if !self.players.get(&pid)?.inventory.has_room() {
            return Some(treats::lines::HANDS_FULL.into());
        }
        tray.pieces = tray.pieces.saturating_sub(1);
        if tray.pieces == 0 {
            self.tray = None;
        }
        let name = shop::product(kind).map_or("?", |p| p.name);
        let item = self.mint_item(kind, name);
        self.give(pid, item);
        Some(treats::lines::TAKEN.into())
    }
}
