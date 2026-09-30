//! Voice chat relay: push-to-talk frames go to everyone in the speaker's
//! room, or — whispered — only to the nearest person within reach. The
//! server never decodes, stores nor logs them.

use crate::protocol::{Packet, MAX_VOICE_BYTES};
use crate::sim::TILE_UNITS;

use super::Server;

/// A whisper reaches this far (1.5 tiles).
pub const WHISPER_RADIUS: i32 = 3 * TILE_UNITS / 2;
/// Frames a player may send per second (40 ms frames = 25/s), and a burst.
const FRAMES_PER_SEC: u32 = 30;
const BURST_FRAMES: u32 = 10;
/// The allowance counts 1/20 of a frame per unit (refilled every tick).
const PER_FRAME: u32 = 20;

impl Server {
    pub(super) fn handle_voice(&mut self, pid: u16, seq: u16, whisper: u8, data: Vec<u8>) {
        let tick = self.tick;
        let Some(p) = self.players.get_mut(&pid) else { return };
        if !p.in_building() || p.riding.is_some() || data.is_empty() || data.len() > MAX_VOICE_BYTES {
            return;
        }
        // Rate limit (a token bucket in ticks).
        let refill = tick.wrapping_sub(p.voice_tick).min(1000) * FRAMES_PER_SEC;
        p.voice_allowance = (p.voice_allowance + refill).min(BURST_FRAMES * PER_FRAME);
        p.voice_tick = tick;
        if p.voice_allowance < PER_FRAME {
            return;
        }
        p.voice_allowance -= PER_FRAME;
        let (floor, room, pos) = (p.body.floor, p.room, p.body.pos);
        let listeners: Vec<_> = if whisper != 0 {
            // The nearest person within reach, and only them.
            self.players
                .values()
                .filter(|o| o.id != pid && o.in_building() && o.body.floor == floor)
                .map(|o| {
                    (
                        o,
                        (o.body.pos.x - pos.x) as i64 * (o.body.pos.x - pos.x) as i64
                            + (o.body.pos.y - pos.y) as i64 * (o.body.pos.y - pos.y) as i64,
                    )
                })
                .filter(|(_, d)| *d <= (WHISPER_RADIUS as i64) * (WHISPER_RADIUS as i64))
                .min_by_key(|(o, d)| (*d, o.id))
                .map(|(o, _)| o.addr)
                .into_iter()
                .collect()
        } else {
            self.players
                .values()
                .filter(|o| o.id != pid && o.in_building() && o.body.floor == floor && o.room == room)
                .map(|o| o.addr)
                .collect()
        };
        let pk = Packet::VoiceFrom { speaker: pid, seq, whisper: (whisper != 0) as u8, data };
        for addr in listeners {
            self.send(addr, &pk);
        }
    }
}
