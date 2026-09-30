//! Connections: datagram dispatch, handshake, address migration, timeouts
//! and leaving.

use std::net::SocketAddr;
use std::time::Instant;

use crate::coffee;
use crate::commute;
use crate::crypto;
use crate::inventory::{kind as item_kind, Item};
use crate::map::access;
use crate::net::canonical;
use crate::protocol::{self as proto, DecodeError, Packet, PlayerInfoEntry, Profile};
use crate::shop;
use crate::sim::{self, Body, Pos};

use super::items::DROP_HANDLE_BASE;
use super::player::{clean_text, refresh, validate_profile, Player, Stage};
use super::portal::MAX_VACANCIES;
use super::snapshot::INFO_PER_PACKET;
use super::{Server, MAX_INPUT_QUEUE, TICK_HZ};

impl Server {
    pub(super) fn handle_datagram(&mut self, addr: SocketAddr, data: &[u8], now: Instant) {
        // Sealed (a logged-in player): check, decrypt, then as usual.
        let mut keys = None;
        let mut sealed_ticket = String::new();
        let mut sealed_token = None;
        let opened;
        let data = match data.get(3) {
            Some(&crypto::SEALED) if data.len() >= 8 && data[2] == proto::VERSION => {
                let token = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                let Some(p) = self.by_token.get(&token).and_then(|id| self.players.get_mut(id)) else {
                    self.send(addr, &Packet::Disconnect { token, reason: proto::disconnect::SESSION_UNKNOWN });
                    return;
                };
                let Some(c) = p.crypto.as_mut() else { return };
                let Some((counter, inner)) = c.keys.open(crypto::Dir::ToServer, 8, data) else { return };
                if !c.window.accept(counter) {
                    return; // a replay
                }
                sealed_token = Some(token);
                opened = inner;
                &opened[..]
            }
            Some(&crypto::SEALED_CONNECT) if data.len() >= 4 + crypto::TICKET_BYTES && data[2] == proto::VERSION => {
                let ticket: String = data[4..4 + crypto::TICKET_BYTES].iter().map(|b| format!("{b:02x}")).collect();
                let Some(key) = self.auth.as_ref().and_then(|a| a.ticket_key(&ticket)) else {
                    self.send(addr, &Packet::Reject { reason: proto::reject::BAD_TICKET });
                    return;
                };
                let k = crypto::Keys::derive(&key);
                let Some((counter, inner)) = k.open(crypto::Dir::ToServer, 4 + crypto::TICKET_BYTES, data) else { return };
                if !self.auth.as_ref().is_some_and(|a| a.accept_connect(&ticket, counter)) {
                    return; // a recorded Connect played again
                }
                keys = Some(k);
                sealed_ticket = ticket;
                opened = inner;
                &opened[..]
            }
            _ => data,
        };
        let packet = match Packet::decode(data) {
            Ok(p) => p,
            Err(DecodeError::BadVersion(_)) => {
                self.send(addr, &Packet::Reject { reason: proto::reject::BAD_VERSION });
                return;
            }
            Err(_) => return,
        };
        if let Packet::Connect { nonce, nick, profile, ticket } = packet {
            if keys.is_some() && ticket != sealed_ticket {
                return; // sealed with one ticket, asking for another
            }
            self.handle_connect(addr, nonce, &nick, profile, &ticket, keys, now);
            return;
        }
        // Every other packet is identified by its session token.
        let Some(token) = session_token(&packet) else { return };
        if sealed_token.is_some_and(|t| t != token) {
            return; // sealed for one session, speaking for another
        }
        // A logged-in session only takes sealed packets.
        if sealed_token.is_none() && self.by_token.get(&token).and_then(|id| self.players.get(id)).is_some_and(|p| p.crypto.is_some()) {
            return;
        }
        let Some(&id) = self.by_token.get(&token) else {
            // Unknown/expired session: tell the client so it can reconnect.
            if matches!(packet, Packet::Input { .. } | Packet::Ping { .. }) {
                self.send(addr, &Packet::Disconnect { token, reason: proto::disconnect::SESSION_UNKNOWN });
            }
            return;
        };
        let Some(p) = self.players.get_mut(&id) else { return };
        p.last_heard = now;
        // Address migration: only packets that prove liveness *now* (a ping or
        // new inputs) may move the session, so a late reordered packet from the
        // old address cannot pull it back.
        let fresh = match &packet {
            Packet::Ping { .. } => true,
            Packet::Input { last_seq, .. } => *last_seq > p.last_received_seq,
            _ => false,
        };
        if fresh && p.addr != addr {
            self.migrate(id, addr);
        }
        match packet {
            Packet::Input { last_seq, inputs, .. } => self.queue_inputs(id, last_seq, &inputs),
            Packet::InfoRequest { ids, .. } => {
                let entries: Vec<PlayerInfoEntry> = ids.iter().filter_map(|&i| self.info_of(i)).collect();
                if let Some(p) = self.players.get_mut(&id) {
                    p.known.extend(entries.iter().map(|e| e.id));
                }
                for chunk in entries.chunks(INFO_PER_PACKET) {
                    self.send_to(id, &Packet::PlayerInfo { players: chunk.to_vec() });
                }
            }
            Packet::Ping { client_time, .. } => {
                let server_tick = self.tick;
                self.send_to(id, &Packet::Pong { client_time, server_tick });
            }
            Packet::Disconnect { .. } => self.remove_player(id, "left"),
            Packet::Apply { offer, .. } => self.handle_apply(id, offer),
            Packet::PortalAction { action, arg, .. } => self.handle_portal_action(id, action, arg),
            Packet::Answer { attempt, index, choice, .. } => self.handle_answer(id, attempt, index, choice),
            Packet::ItemAction { action, slot, .. } => self.handle_item_action(id, action, slot),
            Packet::ComputerAction { action, conv, arg, text, .. } => self.handle_computer_action(id, action, conv, arg, &text),
            Packet::DoorAction { .. } => self.handle_door_action(id),
            Packet::ShopTake { shelf, kind, .. } => self.handle_shop_take(id, shelf, kind),
            Packet::CalendarBook { start, topic, .. } => self.handle_calendar_book(id, u32::from(start), topic),
            Packet::DialogAnswer { id: dialog, choice, .. } => self.handle_dialog_answer(id, dialog, choice),
            Packet::LunchOrder { dish, .. } => self.handle_lunch_order(id, dish),
            Packet::FridgeAction { action, arg, .. } => self.handle_fridge_action(id, action, arg),
            Packet::SkipWait { .. } => self.handle_skip_wait(id),
            Packet::Voice { seq, whisper, data, .. } => self.handle_voice(id, seq, whisper, data),
            Packet::TaskAction { nonce, action, task, arg, text, .. } => self.handle_task_action(id, nonce, action, task, arg, &text),
            Packet::MailAction { nonce, action, id: mid, to, subject, body, .. } => {
                self.handle_mail_action(id, nonce, action, mid, &to, &subject, &body)
            }
            Packet::CompanyAction { action, target, value, text, .. } => self.handle_company_action(id, action, target, value, &text),
            Packet::CommuteChoice { mode, .. } => self.handle_commute_choice(id, mode),
            _ => {}
        }
    }

    /// New inputs into the player's queue (a packet repeats the last few:
    /// only unseen sequence numbers are taken).
    fn queue_inputs(&mut self, id: u16, last_seq: u32, inputs: &[u8]) {
        let Some(p) = self.players.get_mut(&id) else { return };
        let n = u32::try_from(inputs.len()).unwrap_or(u32::MAX);
        for (back, &bits) in (0..n).rev().zip(inputs) {
            let seq = last_seq.wrapping_sub(back);
            if seq > p.last_received_seq {
                p.inputs.push_back((seq, bits));
                p.last_received_seq = seq;
            }
        }
        while p.inputs.len() > MAX_INPUT_QUEUE {
            if let Some((seq, _)) = p.inputs.pop_front() {
                p.last_processed_seq = seq;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_connect(
        &mut self,
        addr: SocketAddr,
        nonce: u32,
        nick: &str,
        profile: Profile,
        ticket: &str,
        keys: Option<crypto::Keys>,
        now: Instant,
    ) {
        if let Some(&id) = self.by_addr.get(&addr) {
            if self.players.get(&id).is_some_and(|p| p.nonce == nonce) {
                // Our Welcome was lost; resend it.
                if let Some(welcome) = self.welcome(id) {
                    self.send(addr, &welcome);
                }
                return;
            }
            self.remove_player(id, "reconnected");
        }
        // An account (a ticket from logging in) or a guest.
        let (nick, guest) = if !ticket.is_empty() {
            // An account's Connect must come sealed with its key.
            let redeemed = if keys.is_some() { self.auth.as_ref().and_then(|a| a.redeem(ticket)) } else { None };
            match redeemed {
                Some(n) => (n, false),
                None => {
                    self.send(addr, &Packet::Reject { reason: proto::reject::BAD_TICKET });
                    return;
                }
            }
        } else if !self.cfg.allow_guests {
            self.send(addr, &Packet::Reject { reason: proto::reject::GUESTS_OFF });
            return;
        } else {
            (clean_text(nick), true)
        };
        if nick.is_empty() {
            self.send(addr, &Packet::Reject { reason: proto::reject::BAD_NICK });
            return;
        }
        // Nicks are unique: a guest may not take an account's nick, a saved
        // character's or one playing right now (not case-sensitive).
        let same = |n: &str| n.to_lowercase() == nick.to_lowercase();
        let nick_taken = guest
            && (self.auth.as_ref().is_some_and(|a| a.is_registered(&nick))
                || self.offline.characters.keys().any(|n| same(n))
                || self.players.values().any(|p| same(&p.nick)));
        if nick_taken {
            self.send(addr, &Packet::Reject { reason: proto::reject::NICK_TAKEN });
            return;
        }
        // The same account logged in again (another computer): the old
        // session goes — and so does a guest playing under the account's nick.
        if !guest {
            let old: Vec<u16> = self.players.values().filter(|p| p.nick.to_lowercase() == nick.to_lowercase()).map(|p| p.id).collect();
            for id in old {
                self.remove_player(id, "logged in elsewhere");
            }
        }
        let Some(profile) = validate_profile(profile) else {
            self.send(addr, &Packet::Reject { reason: proto::reject::BAD_PROFILE });
            return;
        };
        // A new account character: its e-mail must be unique (a saved one
        // keeps its own).
        if !guest && !self.offline.characters.contains_key(&nick) && self.email_taken(&profile.email, &nick) {
            self.send(addr, &Packet::Reject { reason: proto::reject::EMAIL_TAKEN });
            return;
        }
        if self.players.len() >= self.cfg.max_players {
            self.send(addr, &Packet::Reject { reason: proto::reject::SERVER_FULL });
            return;
        }
        let Some((spawn_floor, spawn)) = self.next_spawn_point() else {
            self.send(addr, &Packet::Reject { reason: proto::reject::SERVER_FULL });
            return;
        };
        let id = self.alloc_id();
        let token = loop {
            let t = self.rng.u32(1..);
            if !self.by_token.contains_key(&t) {
                break t;
            }
        };
        let skip = self.cfg.skip_recruitment;
        let stage = if skip { Stage::Working } else { Stage::Portal(Box::default()) };
        let body = Body::at(spawn_floor, spawn);
        let mut player = Player::new(id, token, nonce, addr, nick, profile, stage, body, now);
        player.guest = guest;
        player.crypto = keys.map(crypto::Session::new);
        player.room = self.room_of(spawn_floor, spawn);
        self.log(format!("+ player {} '{}' from {} ({} online)", id, player.nick, canonical(addr), self.players.len() + 1));
        self.players.insert(id, player);
        self.by_addr.insert(addr, id);
        self.by_token.insert(token, id);
        let restored = !guest && self.restore(id);
        if self.cfg.start_employed && !restored {
            self.start_employed(id);
        } else if skip && self.cfg.start_access & access::CARD != 0 {
            self.give_new(id, item_kind::EMPLOYEE_CARD); // load tests: straight in with a card
        }
        if let Some(welcome) = self.welcome(id) {
            self.send(addr, &welcome);
            self.send(addr, &Packet::Departments { list: self.cfg.recruitment.department_list() });
        }
        // A saved character: its own look and name, as the server knows them.
        if restored {
            if let Some(info) = self.info_of(id) {
                self.send(addr, &Packet::PlayerInfo { players: vec![info] });
            }
        }
        if !skip {
            self.send_portal(id, true);
        }
    }

    /// Spawn points are used in turn.
    fn next_spawn_point(&mut self) -> Option<(u8, Pos)> {
        let spawns = self.building.spawns();
        let (floor, tile) = *spawns.get(self.next_spawn % spawns.len().max(1))?;
        self.next_spawn = self.next_spawn.wrapping_add(1);
        Some((floor, Pos::tile_center(tile.x, tile.y)))
    }

    /// `--start-employed`: hired, day 2 (at home if it's night), and with
    /// `--start-cigarettes` a paid pack in the pocket.
    fn start_employed(&mut self, id: u16) {
        self.employ(id);
        let night = self.clock.is_night();
        if let Some(p) = self.players.get_mut(&id) {
            p.day = 2;
            if night {
                p.stage = Stage::Home { arrive_at: None };
            }
        }
        if self.cfg.start_cigarettes {
            let pack = shop::product(item_kind::CIGARETTES).map_or(1, |p| p.count.max(1));
            let item = Item { owner: id, count: pack, ..self.mint_item(item_kind::CIGARETTES, "") };
            self.give(id, item);
        }
    }

    /// The commute for tomorrow morning can be changed until leaving home.
    fn handle_commute_choice(&mut self, id: u16, mode: u8) {
        let Some(p) = self.players.get_mut(&id) else { return };
        if commute::mode(mode).is_some() && matches!(p.stage, Stage::Home { arrive_at: None }) {
            p.commute_mode = mode;
            self.clock_dirty = true;
        }
    }

    /// Players silent for longer than the timeout are dropped.
    pub(super) fn drop_timed_out(&mut self, now: Instant) {
        let timeout = self.cfg.client_timeout;
        let stale: Vec<(u16, SocketAddr, u32)> =
            self.players.values().filter(|p| now.duration_since(p.last_heard) > timeout).map(|p| (p.id, p.addr, p.token)).collect();
        for (id, addr, token) in stale {
            self.send(addr, &Packet::Disconnect { token, reason: proto::disconnect::TIMEOUT });
            self.remove_player(id, "timed out");
        }
    }

    fn welcome(&self, id: u16) -> Option<Packet> {
        let p = self.players.get(&id)?;
        Some(Packet::Welcome {
            nonce: p.nonce,
            player_id: id,
            token: p.token,
            tick_hz: TICK_HZ as u8,
            input_hz: sim::INPUT_HZ as u8,
            map_crc: self.building.crc,
            server_tick: self.tick,
        })
    }

    /// A free player id: 1 .. DROP_HANDLE_BASE-1 (entity handles and NPCs
    /// use the ids above). `max_players` keeps plenty free.
    pub(super) fn alloc_id(&mut self) -> u16 {
        loop {
            let id = self.next_id;
            self.next_id = if self.next_id + 1 >= DROP_HANDLE_BASE { 1 } else { self.next_id + 1 };
            if !self.players.contains_key(&id) {
                return id;
            }
        }
    }

    fn migrate(&mut self, id: u16, addr: SocketAddr) {
        let Some(p) = self.players.get_mut(&id) else { return };
        let old = std::mem::replace(&mut p.addr, addr);
        if self.by_addr.get(&old) == Some(&id) {
            self.by_addr.remove(&old);
        }
        self.by_addr.insert(addr, id);
        self.log(format!("~ player {id} moved {} -> {}", canonical(old), canonical(addr)));
    }

    pub(super) fn remove_player(&mut self, id: u16, why: &str) {
        let Some(mut p) = self.players.remove(&id) else { return };
        self.remember_leaving(&p);
        let persistent = self.persistent() && !p.guest;
        coffee::release(&mut self.machines, &p.cup);
        for c in &mut self.computers {
            if c.user == Some(id) {
                c.user = None;
            }
        }
        // Other people's things they carried stay in the building...
        let carried: Vec<Item> = std::mem::take(&mut p.inventory).items().cloned().collect();
        self.return_mugs_of(&carried);
        for item in carried.into_iter().filter(|i| i.owner != 0 && i.owner != id) {
            self.drop_at(p.body.floor, p.body.pos, item);
        }
        // ...their own go with them (no persistent accounts yet): laptop on
        // a desk, card lent to someone, anything on the floor.
        self.computers.retain(|c| c.owner() != id); // (persistent: already handed to the save)
        self.vehicles.retain(|v| v.owner != id);
        self.lunch_orders.retain(|o| o.owner != id);
        self.company.candidates.retain(|c| c.player != id);
        self.company.hired_on.remove(&id);
        if self.company.founder == Some(id) {
            self.company.founder = None; // the company stays; someone may found it anew
            self.clock_dirty = true;
        }
        // Their job is free again (a persistent world keeps it for them).
        if let Some(pos) = p.position.and_then(|o| self.positions.iter_mut().find(|x| x.id == o)).filter(|_| !persistent) {
            pos.places = (pos.places + 1).min(MAX_VACANCIES);
        }
        self.dropped.retain(|d| d.item.owner != id || d.item.kind == item_kind::EMPTY_CUP); // mugs stay
        for other in self.players.values_mut() {
            if other.inventory.remove_owned_by(id) {
                refresh(other);
            }
            other.known.remove(&id);
        }
        self.messenger.forget(id);
        self.by_token.remove(&p.token);
        if self.by_addr.get(&p.addr) == Some(&id) {
            self.by_addr.remove(&p.addr);
        }
        self.log(format!("- player {} '{}' {} ({} online)", id, p.nick, why, self.players.len()));
    }
}

/// The session token of a packet a connected client sends; `None` for
/// packets only the server sends (and `Connect`).
fn session_token(packet: &Packet) -> Option<u32> {
    match packet {
        Packet::Input { token, .. }
        | Packet::InfoRequest { token, .. }
        | Packet::Ping { token, .. }
        | Packet::Disconnect { token, .. }
        | Packet::Apply { token, .. }
        | Packet::Answer { token, .. }
        | Packet::PortalAction { token, .. }
        | Packet::ItemAction { token, .. }
        | Packet::ComputerAction { token, .. }
        | Packet::DoorAction { token }
        | Packet::ShopTake { token, .. }
        | Packet::CommuteChoice { token, .. }
        | Packet::CalendarBook { token, .. }
        | Packet::DialogAnswer { token, .. }
        | Packet::LunchOrder { token, .. }
        | Packet::FridgeAction { token, .. }
        | Packet::SkipWait { token }
        | Packet::Voice { token, .. }
        | Packet::TaskAction { token, .. }
        | Packet::MailAction { token, .. }
        | Packet::CompanyAction { token, .. } => Some(*token),
        _ => None,
    }
}
