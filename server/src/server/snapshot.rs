//! Interest management, snapshots, speech and periodic state resends.

use std::collections::HashMap;
use std::net::SocketAddr;

use crate::computer;
use crate::protocol::{self as proto, EntityState, Packet, PlayerInfoEntry, SelfState};
use crate::sim::Pos;
use crate::treats;

use super::items::inventory_packet;
use super::player::{activity, Player};
use super::{Say, Server};

/// PlayerInfo entries per packet (worst case 2 + 1 + 16 B each -> ~1100 B).
pub(super) const INFO_PER_PACKET: usize = 55;
/// Resend the inventory this often (ticks).
const INVENTORY_RESEND_TICKS: u32 = 40;
/// Resend the game time this often (ticks).
const CLOCK_RESEND_TICKS: u32 = 20;
/// Send the character's needs (and the doors) this often (ticks).
const STATS_EVERY_TICKS: u32 = 10;
/// Resend the computer screen state this often (ticks).
const COMPUTER_RESEND_TICKS: u32 = 20;

/// Sounds carry this far on a floor: 28 tiles.
const HEAR_RADIUS: i32 = 28 * crate::sim::TILE_UNITS;

/// A packet queued for a player: (address, player id, packet).
pub(super) type Outgoing = (SocketAddr, u16, Packet);

/// Entities by (floor, room).
type Groups = HashMap<(u8, u16), Vec<EntityState>>;

impl Server {
    /// Everything the clients get at the end of a tick: snapshots, state
    /// screens, speech and the clock, sent in that order.
    pub(super) fn send_updates(&mut self) {
        let mut out = std::mem::take(&mut self.outbox);
        let groups = self.interest_groups();
        self.queue_snapshots(&groups, &mut out);
        self.queue_says(&mut out);
        self.queue_sounds(&mut out);
        self.queue_world_state(&mut out);
        for (addr, id, packet) in out.drain(..) {
            let n = self.send(addr, &packet);
            let n = n as u64;
            if matches!(packet, Packet::Snapshot { .. }) {
                self.stats.snapshot_bytes += n;
            }
            if let Some(p) = self.players.get_mut(&id) {
                p.bytes_out += n;
            }
        }
        self.outbox = out;
    }

    /// Group every visible entity by (floor, room).
    fn interest_groups(&self) -> Groups {
        let mut groups = Groups::with_capacity(64);
        let mut put = |place: (u8, u16), e: EntityState| groups.entry(place).or_default().push(e);
        let at = |floor: u8, pos: Pos| (floor, self.building.room_at(floor, pos));
        for v in &self.vehicles {
            let flags = v.facing | if v.moving { 0b100 } else { 0 };
            put(at(0, v.pos), entity(v.handle, proto::kind::VEHICLE, v.pos, flags, v.kind, 0));
        }
        // Riders are inside their vehicle: not shown.
        for p in self.players.values().filter(|p| p.in_building() && p.riding.is_none()) {
            let e = entity(p.id, proto::kind::PLAYER, p.body.pos, p.flags, p.inventory.held_kind(), activity(p, self.tick));
            put((p.body.floor, p.room), e);
        }
        for n in &self.npcs {
            put((n.body.floor, n.room), entity(n.id, proto::kind::NPC, n.body.pos, n.flags, 0, 0));
        }
        for c in &self.computers {
            let Some(w) = self.workstations.get(c.station) else { continue };
            let pos = Pos::tile_center(w.tile.x, w.tile.y);
            put(at(w.floor, pos), entity(c.handle, proto::kind::COMPUTER, pos, computer::entity_flags(c), c.item.kind, 0));
        }
        if let Some(t) = &self.tray {
            let (floor, pos) = treats::tray_pos();
            put(at(floor, pos), entity(t.handle, proto::kind::TRAY, pos, 0, t.kind, t.pieces));
        }
        for d in &self.dropped {
            put(at(d.floor, d.pos), entity(d.handle, proto::kind::ITEM, d.pos, 0, d.item.kind, 0));
        }
        groups
    }

    /// Per player in the building: the snapshot of what they see, names of
    /// newly visible entities, and the periodic screens (inventory, doors,
    /// needs, computer, calendar, dialog, lunch, company panel).
    fn queue_snapshots(&mut self, groups: &Groups, out: &mut Vec<Outgoing>) {
        let tick = self.tick;
        let ids: Vec<u16> = self.players.values().filter(|p| p.in_building()).map(|p| p.id).collect();
        let mut visible = Vec::new();
        for id in ids {
            let Some(p) = self.players.get(&id) else { continue };
            visible.clear();
            visible.extend(self.visible_places(p).iter().filter_map(|k| groups.get(k)).flatten().filter(|e| e.id != id).copied());
            self.stats.max_visible = self.stats.max_visible.max(visible.len());
            let new_infos: Vec<PlayerInfoEntry> =
                visible.iter().filter(|e| !p.known.contains(&e.id)).filter_map(|e| self.info_of(e.id)).collect();
            let me = SelfState {
                x: p.body.pos.x,
                y: p.body.pos.y,
                floor: p.body.floor,
                room: p.room,
                lock: p.body.lock,
                prev_input: p.body.prev_input,
                access: p.body.access,
                slow: p.body.slow,
                activity: activity(p, self.tick),
            };
            let addr = p.addr;
            let mut queue = |packet: Packet| out.push((addr, id, packet));
            for f in proto::snapshot_fragments(tick, p.last_processed_seq, me, &visible) {
                queue(f);
            }
            if p.inv_dirty || tick.is_multiple_of(INVENTORY_RESEND_TICKS) {
                queue(inventory_packet(&p.inventory));
            }
            if self.doors_dirty || tick.is_multiple_of(STATS_EVERY_TICKS) {
                queue(self.doors_packet(p.body.floor));
            }
            if tick.is_multiple_of(STATS_EVERY_TICKS) {
                queue(stats_packet(p));
            }
            if tick.is_multiple_of(COMPUTER_RESEND_TICKS) {
                let screens = [self.computer_packet(id), self.calendar_packet(id), self.dialog_packet(id), self.lunch_packet(id)];
                screens.into_iter().flatten().chain(self.company_packets(id)).for_each(&mut queue);
            }
            for chunk in new_infos.chunks(INFO_PER_PACKET) {
                queue(Packet::PlayerInfo { players: chunk.to_vec() });
            }
            if let Some(p) = self.players.get_mut(&id) {
                p.known.extend(new_infos.iter().map(|e| e.id));
                p.inv_dirty = false;
            }
        }
        self.doors_dirty = false;
    }

    /// (floor, room) places a player sees: their room, rooms visible from it
    /// and, from a balcony, the street below (same grid: the client draws
    /// them over its view of the floor below).
    fn visible_places(&self, p: &Player) -> Vec<(u8, u16)> {
        let also: &[u16] = self.building.floor(p.body.floor).map_or(&[], |m| m.visible_from(p.room));
        std::iter::once(&p.room).chain(also).map(|r| (p.body.floor, *r)).chain(self.building.below(p.body.floor, p.room)).collect()
    }

    /// Speech (NPCs, and players' own "thought" lines): to everyone in the
    /// speaker's room or seeing into it (e.g. from a toilet stall), plus the
    /// addressee.
    fn queue_says(&mut self, out: &mut Vec<Outgoing>) {
        for Say { speaker, text, to } in std::mem::take(&mut self.says) {
            let who = match self.npcs.iter().find(|n| n.id == speaker) {
                Some(n) => Some(((n.body.floor, n.room), n.name.clone())),
                None => self.players.get(&speaker).map(|p| ((p.body.floor, p.room), p.nick.clone())),
            };
            let Some((place, name)) = who else { continue };
            let info = self.info_of(speaker).unwrap_or(PlayerInfoEntry {
                id: speaker,
                nick: name,
                department: 0,
                gender: proto::gender::OTHER,
                appearance: proto::Appearance::default(),
            });
            for p in self.players.values_mut() {
                let here = (p.body.floor, p.room) == place;
                let sees = (p.body.floor == place.0
                    && self.building.floor(p.body.floor).is_some_and(|m| m.visible_from(p.room).contains(&place.1)))
                    || self.building.below(p.body.floor, p.room).contains(&place);
                if here || sees || Some(p.id) == to || p.id == speaker {
                    // Name first, so the line isn't shown as "?".
                    if p.id != speaker && p.known.insert(speaker) {
                        out.push((p.addr, p.id, Packet::PlayerInfo { players: vec![info.clone()] }));
                    }
                    out.push((p.addr, p.id, Packet::Say { id: speaker, text: text.clone() }));
                }
            }
        }
    }

    /// This tick's sounds to everyone in the building on the same floor
    /// within `HEAR_RADIUS`.
    fn queue_sounds(&mut self, out: &mut Vec<Outgoing>) {
        let sounds = std::mem::take(&mut self.sounds);
        if sounds.is_empty() {
            return;
        }
        let r2 = (HEAR_RADIUS as i64) * (HEAR_RADIUS as i64);
        for p in self.players.values().filter(|p| p.in_building()) {
            let heard: Vec<(u8, i32, i32)> = sounds
                .iter()
                .filter(|(_, f, pos)| {
                    let (dx, dy) = ((pos.x - p.body.pos.x) as i64, (pos.y - p.body.pos.y) as i64);
                    *f == p.body.floor && dx * dx + dy * dy <= r2
                })
                .map(|&(k, _, pos)| (k, pos.x, pos.y))
                .take(proto::MAX_SOUNDS)
                .collect();
            if !heard.is_empty() {
                out.push((p.addr, p.id, Packet::Sound { sounds: heard }));
            }
        }
    }

    /// Game time for everyone (also at home / on the portal); smoke and
    /// lights of the floor for those in the building.
    fn queue_world_state(&mut self, out: &mut Vec<Outgoing>) {
        let tick = self.tick;
        if !self.clock_dirty && !tick.is_multiple_of(CLOCK_RESEND_TICKS) {
            return;
        }
        for p in self.players.values() {
            out.push((p.addr, p.id, self.clock_packet(p)));
            if p.in_building() && tick.is_multiple_of(CLOCK_RESEND_TICKS) {
                let floor = p.body.floor;
                out.push((p.addr, p.id, Packet::Smoke { floor, rooms: self.smoke.floor_levels(floor) }));
                out.push((p.addr, p.id, Packet::Lights { floor, rooms: self.lights.floor_on(floor) }));
            }
        }
        self.clock_dirty = false;
    }

    /// Name (and official department) of a player or NPC.
    pub(super) fn info_of(&self, id: u16) -> Option<PlayerInfoEntry> {
        if let Some(p) = self.players.get(&id) {
            return Some(PlayerInfoEntry {
                id,
                nick: p.nick.clone(),
                department: if p.contract { p.department } else { 0 },
                gender: p.profile.gender,
                appearance: p.profile.appearance,
            });
        }
        // A computer is introduced by its owner's name.
        if let Some(c) = self.computers.iter().find(|c| c.handle == id) {
            return self.info_of(c.owner()).map(|e| PlayerInfoEntry { id, ..e });
        }
        self.npcs.iter().find(|n| n.id == id).map(|n| PlayerInfoEntry {
            id,
            nick: n.name.clone(),
            department: 0,
            gender: proto::gender::OTHER,
            appearance: proto::Appearance::default(),
        })
    }
}

fn entity(id: u16, kind: u8, pos: Pos, flags: u8, held: u8, activity: u8) -> EntityState {
    EntityState { id, kind, x: pos.x, y: pos.y, flags, held, activity }
}

/// The character's needs and wallet.
fn stats_packet(p: &Player) -> Packet {
    let [hunger, energy, stress, bladder, hygiene] = p.needs.points();
    let flags = if p.needs.dirty_hands { proto::STATS_DIRTY_HANDS } else { 0 } | if p.needs.upset { proto::STATS_UPSET } else { 0 };
    let money = u32::try_from(p.money.max(0)).unwrap_or(u32::MAX);
    Packet::Stats { hunger, energy, stress, bladder, hygiene, flags, money }
}
