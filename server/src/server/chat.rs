//! Typed chat (Enter in the game): to the room, a whisper to the person
//! next to you (/s), a shout to the whole floor (/k). And notifications for
//! the corner of the screen.

use crate::protocol::{self as proto, Packet};
use crate::sim::TILE_UNITS;

use super::player::clean_text;
use super::{dist2, Say, Server};

/// One line every half a second at most (20 ticks a second).
const CHAT_GAP_TICKS: u32 = 10;
/// A whisper reaches this far (2 tiles).
const WHISPER_REACH: i32 = TILE_UNITS * 2;

pub(super) mod lines {
    pub const NOBODY_TO_WHISPER: &str = "Nikogo obok — nie ma komu szeptać.";
    pub fn whisper(text: &str) -> String {
        format!("(szeptem) {text}")
    }
    pub fn shout(text: &str) -> String {
        format!("(krzyczy) {}", text.to_uppercase())
    }
}

impl Server {
    /// `ChatSay`: a line typed in the game.
    pub(super) fn handle_chat_say(&mut self, pid: u16, text: &str) {
        let tick = self.tick;
        let Some(p) = self.players.get_mut(&pid) else { return };
        if !p.in_building() || tick < p.next_chat {
            return;
        }
        p.next_chat = tick + CHAT_GAP_TICKS;
        let text = clean_text(text);
        let text = proto::truncate_utf8(&text, proto::MAX_SAY_BYTES - 16).to_string();
        let (cmd, rest) = match text.split_once(' ') {
            Some((c, r)) if c == "/s" || c == "/k" => (c, r.trim().to_string()),
            _ => ("", text.clone()),
        };
        if rest.is_empty() {
            return;
        }
        let (floor, pos) = (p.body.floor, p.body.pos);
        match cmd {
            "/s" => {
                let near = self
                    .players
                    .values()
                    .filter(|o| o.id != pid && o.in_building() && o.body.floor == floor)
                    .map(|o| (o.id, dist2(o.body.pos, pos)))
                    .filter(|&(_, d)| d <= WHISPER_REACH * WHISPER_REACH)
                    .min_by_key(|&(_, d)| d)
                    .map(|(o, _)| o);
                match near {
                    Some(to) => self.says.push(Say::whisper(pid, lines::whisper(&rest), to)),
                    None => self.says.push(Say::whisper(pid, lines::NOBODY_TO_WHISPER, pid)),
                }
            }
            "/k" => self.says.push(Say::shout(pid, lines::shout(&rest))),
            _ => self.says.push(Say::new(pid, rest)),
        }
    }

    /// A notification for `pid` (if online).
    pub(super) fn notify(&mut self, pid: u16, icon: u8, text: impl Into<String>) {
        if self.players.contains_key(&pid) {
            self.send_to(pid, &Packet::Notice { icon, text: text.into() });
        }
    }

    /// The same, for a player by nick (mail is addressed by nick).
    pub(super) fn notify_nick(&mut self, nick: &str, icon: u8, text: impl Into<String>) {
        if let Some(pid) = self.players.values().find(|p| p.nick == nick).map(|p| p.id) {
            self.notify(pid, icon, text);
        }
    }

    /// Everybody in the building (but `except`).
    pub(super) fn notify_building(&mut self, except: u16, icon: u8, text: &str) {
        let to: Vec<u16> = self.players.values().filter(|p| p.in_building() && p.id != except).map(|p| p.id).collect();
        for pid in to {
            self.notify(pid, icon, text);
        }
    }

    /// Everybody in the room of `pid` (but them).
    pub(super) fn notify_room_of(&mut self, pid: u16, icon: u8, text: &str) {
        let Some(p) = self.players.get(&pid) else { return };
        let place = (p.body.floor, p.room);
        let to: Vec<u16> =
            self.players.values().filter(|o| o.id != pid && o.in_building() && (o.body.floor, o.room) == place).map(|o| o.id).collect();
        for o in to {
            self.notify(o, icon, text);
        }
    }
}
