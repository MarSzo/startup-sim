//! The TV and the boombox in the chill room: the remote / the boombox in
//! hands + F picks a channel / a track (a dialog); the state goes to
//! everybody in the building as `Media`. The remote and the boombox come
//! back to their places every morning if they went missing.

use crate::inventory::kind as item_kind;
use crate::map::Tile;
use crate::media::{self, lines};
use crate::protocol::Packet;
use crate::sim::Pos;

use super::{dist2, Say, Server};

/// A TV on a wall.
pub(super) struct Screen {
    pub(super) floor: u8,
    /// The left end of the screen.
    pub(super) tile: Tile,
    /// The room it faces (the remote works from there).
    pub(super) room: u16,
    pub(super) channel: u8,
    pub(super) started: u32,
}

/// The TVs of the building: runs of "tv" tiles (one screen each).
pub(super) fn find_screens(b: &crate::building::Building) -> Vec<Screen> {
    let mut out = Vec::new();
    for (f, m) in b.active_floors() {
        for y in 0..m.height {
            for x in 0..m.width {
                if m.tile_type(x, y) != Some("tv") || m.tile_type(x - 1, y) == Some("tv") {
                    continue;
                }
                let room =
                    [(0, -1), (0, 1), (-1, 0), (1, 0)].iter().map(|(dx, dy)| m.room_at_tile(x + dx, y + dy)).find(|&r| r != 0).unwrap_or(0);
                out.push(Screen { floor: f, tile: Tile { x, y }, room, channel: 0, started: 0 });
            }
        }
    }
    out
}

impl Server {
    /// F with the remote: the channels (if there's a TV here).
    pub(super) fn use_remote(&mut self, pid: u16) {
        let Some(p) = self.players.get(&pid) else { return };
        if self.screen_near(pid).is_none() {
            self.says.push(Say::new(pid, lines::NOT_HERE));
            return;
        }
        let mut options: Vec<String> = media::CHANNELS.iter().map(|c| (*c).to_string()).collect();
        options.push(lines::OFF.into());
        let packet = Packet::Dialog { id: media::TV_DIALOG, npc: pid, text: lines::TV_ASK.into(), options, items: Vec::new() };
        self.send(p.addr, &packet);
    }

    /// F with the boombox: the tracks.
    pub(super) fn use_boombox(&mut self, pid: u16) {
        let Some(p) = self.players.get(&pid) else { return };
        let mut options: Vec<String> = media::TRACKS.iter().map(|t| (*t).to_string()).collect();
        options.push(lines::OFF.into());
        let packet = Packet::Dialog { id: media::BOOMBOX_DIALOG, npc: pid, text: lines::MUSIC_ASK.into(), options, items: Vec::new() };
        self.send(p.addr, &packet);
    }

    /// The TV this player may switch: in the room it faces, close enough.
    fn screen_near(&self, pid: u16) -> Option<usize> {
        let p = self.players.get(&pid)?;
        self.screens
            .iter()
            .enumerate()
            .filter(|(_, s)| s.floor == p.body.floor && s.room == p.room)
            .map(|(i, s)| (i, dist2(Pos::tile_center(s.tile.x, s.tile.y), p.body.pos)))
            .filter(|&(_, d)| d <= media::REMOTE_REACH * media::REMOTE_REACH)
            .min_by_key(|&(_, d)| d)
            .map(|(i, _)| i)
    }

    /// The answer to the channel / track question; false if neither.
    pub(super) fn answer_media(&mut self, pid: u16, dialog: u8, choice: u8) -> bool {
        if dialog != media::TV_DIALOG && dialog != media::BOOMBOX_DIALOG {
            return false;
        }
        let Some(p) = self.players.get(&pid) else { return true };
        let (addr, held) = (p.addr, p.inventory.held_kind());
        self.send(addr, &Packet::Dialog { id: 0, npc: pid, text: String::new(), options: Vec::new(), items: Vec::new() });
        let pick = usize::from(choice);
        let tick = self.tick;
        if dialog == media::TV_DIALOG {
            let holds = held == item_kind::REMOTE;
            let Some(i) = self.screen_near(pid).filter(|_| holds) else { return true };
            let s = &mut self.screens[i];
            let (channel, line) = match media::CHANNELS.get(pick) {
                Some(name) => (pick as u8 + 1, lines::tv_on(name)),
                None => (0, lines::TV_OFF.to_string()),
            };
            let was_off = s.channel == 0;
            s.channel = channel;
            s.started = tick;
            self.says.push(Say::new(pid, line));
            if was_off && channel > 0 {
                let text = format!("W chill roomie leci telewizja: {}", media::CHANNELS[pick]);
                self.notify_building(pid, crate::protocol::notice::FUN, &text);
            }
        } else {
            if held != item_kind::BOOMBOX {
                return true;
            }
            let (track, line) = match media::TRACKS.get(pick) {
                Some(name) => (pick as u8 + 1, lines::music_on(name)),
                None => (0, lines::MUSIC_OFF.to_string()),
            };
            self.music = (track > 0).then_some((track, tick));
            self.says.push(Say::new(pid, line));
            if track > 0 {
                let text = format!("{} puszcza muzykę: {}", self.nick_of_player(pid), media::TRACKS[pick]);
                self.notify_building(pid, crate::protocol::notice::FUN, &text);
            }
        }
        self.media_dirty = true;
        true
    }

    /// Where the boombox is: (floor, position, holder - 0 on the floor).
    pub(super) fn boombox_at(&self) -> Option<(u8, Pos, u16)> {
        let held = self.players.values().find(|p| p.in_building() && p.inventory.has(item_kind::BOOMBOX));
        if let Some(p) = held {
            return Some((p.body.floor, p.body.pos, p.id));
        }
        self.dropped.iter().find(|d| d.item.kind == item_kind::BOOMBOX).map(|d| (d.floor, d.pos, 0))
    }

    /// `Media` to everybody in the building: every second, and on change.
    pub(super) fn tick_media(&mut self) {
        if !self.media_dirty && !self.tick.is_multiple_of(media::SEND_TICKS) {
            return;
        }
        self.media_dirty = false;
        let byte = |v: i32| u8::try_from(v).unwrap_or(0);
        let screens = self.screens.iter().map(|s| (s.floor, byte(s.tile.x), byte(s.tile.y), s.channel, s.started)).collect();
        let mut music = Vec::new();
        if let Some((track, started)) = self.music {
            match self.boombox_at() {
                Some((floor, pos, holder)) => music.push((track, started, floor, pos.x, pos.y, holder)),
                None => self.music = None, // it left the building: silence
            }
        }
        let packet = Packet::Media { screens, music };
        let to: Vec<u16> = self.players.values().filter(|p| p.in_building()).map(|p| p.id).collect();
        for pid in to {
            self.send_to(pid, &packet);
        }
    }

    /// The remote and the boombox back at their places if nobody has them
    /// and they aren't lying around (every morning, and at the start).
    pub(super) fn ensure_media_items(&mut self) {
        for (kind, place) in [(item_kind::REMOTE, self.building.remote), (item_kind::BOOMBOX, self.building.boombox)] {
            let Some((floor, tile)) = place else { continue };
            let somewhere = self.players.values().any(|p| p.inventory.has(kind)) || self.dropped.iter().any(|d| d.item.kind == kind);
            if !somewhere {
                let item = self.mint_item(kind, "");
                self.drop_at(floor, Pos::tile_center(tile.x, tile.y), item);
            }
        }
    }
}
