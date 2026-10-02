//! Snapshots split into MTU-sized fragments.

use super::*;

/// Receiver's own state carried in every snapshot fragment.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelfState {
    pub x: i32,
    pub y: i32,
    pub floor: u8,
    pub room: u16,
    pub lock: u8,
    pub prev_input: u8,
    pub access: u8,
    pub slow: bool,
    pub drunk: u8,
    pub activity: u8,
}

/// Split a room's entity list into snapshot fragments that each fit in `MAX_PACKET`.
pub fn snapshot_fragments(tick: u32, last_input_seq: u32, me: SelfState, entities: &[EntityState]) -> Vec<Packet> {
    let chunks: Vec<&[EntityState]> = if entities.is_empty() { vec![&[]] } else { entities.chunks(MAX_ENTITIES_PER_SNAPSHOT).collect() };
    let cnt = chunks.len().min(255) as u8;
    chunks
        .into_iter()
        .take(255)
        .enumerate()
        .map(|(i, c)| Packet::Snapshot {
            tick,
            last_input_seq,
            frag_idx: i as u8,
            frag_cnt: cnt,
            self_x: me.x,
            self_y: me.y,
            floor: me.floor,
            room: me.room,
            self_lock: me.lock,
            self_prev_input: me.prev_input,
            self_access: me.access,
            self_slow: me.slow as u8,
            self_drunk: me.drunk,
            self_activity: me.activity,
            entities: c.to_vec(),
        })
        .collect()
}
