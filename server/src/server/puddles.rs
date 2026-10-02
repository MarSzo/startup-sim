//! Toilet accidents (and throwing up, and pooping on the floor) leave a
//! puddle where they happened; it stays on the floor until the cleaner mops it up on her
//! round, or the office closes (22:00).

use crate::sim::Pos;

use super::Server;

/// A puddle on the floor; `handle` is its entity id in snapshots.
pub(super) struct Puddle {
    pub(super) handle: u16,
    pub(super) floor: u8,
    pub(super) pos: Pos,
    /// What it is (`protocol::puddle`; the entity's `held`).
    pub(super) kind: u8,
}

/// At most this many puddles at once; beyond it the oldest dries up. Keeps
/// the handle space and the snapshots in check.
const MAX_PUDDLES: usize = 256;

impl Server {
    /// An accident at (`floor`, `pos`): a puddle right there.
    pub(super) fn leave_puddle(&mut self, floor: u8, pos: Pos, kind: u8) {
        if self.puddles.len() >= MAX_PUDDLES {
            self.puddles.remove(0);
        }
        let handle = self.alloc_handle();
        self.puddles.push(Puddle { handle, floor, pos, kind });
    }
}
