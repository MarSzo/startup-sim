//! Binary UDP protocol. See `docs/PROTOCOL.md`.
//!
//! Every packet: `magic u16 | version u8 | type u8 | payload`, little-endian.
//! `client/net/protocol.gd` mirrors this module; parity is checked against
//! `tests/golden/packets.json`.

pub const MAGIC: u16 = 0x5354; // "ST"
pub const VERSION: u8 = 18;
pub const HEADER_LEN: usize = 4;
/// Hard upper bound for any datagram we send.
pub const MAX_PACKET: usize = 1200;
pub const MAX_NICK_BYTES: usize = 16;
/// Max UTF-8 bytes of longer texts (speech, offers, questions, options).
pub const MAX_TEXT_BYTES: usize = 240;
pub const MAX_SAY_BYTES: usize = MAX_TEXT_BYTES;
/// Max UTF-8 bytes of a mail body.
pub const MAX_MAIL_BYTES: usize = 600;
/// Max answer options of a recruitment question.
pub const MAX_OPTIONS: usize = 4;
/// Messenger message text (a 200-char message of 2-byte letters fits whole).
pub const MAX_CHAT_BYTES: usize = 400;
/// Conversations in one `Computer` packet (2+1+2+24 B each -> < 1200 B).
pub const MAX_CONVS: usize = 40;
/// Max inputs carried in one Input packet.
pub const MAX_INPUTS_PER_PACKET: usize = 8;

/// Fixed part of a Snapshot packet (header + fields before the entity list).
pub const SNAPSHOT_FIXED_LEN: usize = HEADER_LEN + 4 + 4 + 1 + 1 + (4 + 4 + 1 + 2 + 1 + 1 + 1 + 1 + 1) + 1;
pub const ENTITY_LEN: usize = 14;
/// Entities per snapshot fragment so a fragment never exceeds `MAX_PACKET`.
pub const MAX_ENTITIES_PER_SNAPSHOT: usize = (MAX_PACKET - SNAPSHOT_FIXED_LEN) / ENTITY_LEN;

pub mod ty {
    pub const CONNECT: u8 = 1;
    pub const WELCOME: u8 = 2;
    pub const REJECT: u8 = 3;
    pub const INPUT: u8 = 4;
    pub const SNAPSHOT: u8 = 5;
    pub const PLAYER_INFO: u8 = 6;
    pub const INFO_REQUEST: u8 = 7;
    pub const PING: u8 = 8;
    pub const PONG: u8 = 9;
    pub const DISCONNECT: u8 = 10;
    pub const SAY: u8 = 11;
    pub const JOB_OFFERS: u8 = 12;
    pub const APPLY: u8 = 13;
    pub const QUESTION: u8 = 14;
    pub const ANSWER: u8 = 15;
    pub const RECRUIT_RESULT: u8 = 16;
    pub const MAIL: u8 = 17;
    pub const PORTAL_ACTION: u8 = 18;
    pub const INVENTORY: u8 = 19;
    pub const ITEM_ACTION: u8 = 20;
    pub const COMPUTER: u8 = 21;
    pub const COMPUTER_ACTION: u8 = 22;
    pub const CHAT: u8 = 23;
    pub const STATS: u8 = 24;
    pub const DOORS: u8 = 25;
    pub const DOOR_ACTION: u8 = 26;
    pub const SHELF: u8 = 27;
    pub const SHOP_TAKE: u8 = 28;
    pub const CLOCK: u8 = 29;
    pub const COMMUTE_CHOICE: u8 = 30;
}

/// `ItemAction::action`.
pub mod item_action {
    /// Pocket `slot` -> hands.
    pub const TAKE_OUT: u8 = 1;
    /// Hands -> a free pocket.
    pub const PUT_AWAY: u8 = 2;
    /// Put what's in hands on the floor.
    pub const DROP: u8 = 3;
    /// Hand it to the nearest player (within reach).
    pub const GIVE: u8 = 4;
    /// Use what's in hands (drink coffee, show the card...).
    pub const USE: u8 = 5;
}

/// `Mail::action` / `PortalAction::action`.
/// `ComputerAction::action`.
pub mod computer_action {
    /// Leave the screen.
    pub const CLOSE: u8 = 1;
    /// Lock the computer (and leave). Anyone may.
    pub const LOCK: u8 = 2;
    /// Unlock (owner only).
    pub const UNLOCK: u8 = 3;
    /// Take the laptop off the desk (needs free hands). Anyone may.
    pub const TAKE: u8 = 4;
    /// Send messages of `conv` newer than `arg` (`Chat` reply).
    pub const SYNC: u8 = 5;
    /// Post `text` to `conv`; `arg` = client nonce (retries are deduped).
    pub const SEND: u8 = 6;
}

pub mod portal_action {
    pub const NONE: u8 = 0;
    /// Join the online interview for offer `arg`.
    pub const JOIN_INTERVIEW: u8 = 1;
    /// Hired: go to the office (spawn in the world).
    pub const GO_TO_OFFICE: u8 = 2;
}

pub mod reject {
    pub const SERVER_FULL: u8 = 1;
    pub const BAD_VERSION: u8 = 2;
    pub const BAD_NICK: u8 = 3;
    /// Invalid character profile (age, e-mail, city, appearance).
    pub const BAD_PROFILE: u8 = 4;
}

/// Character appearance: indices into the client's palettes.
pub mod appearance {
    pub const SKINS: u8 = 4;
    pub const HAIR_STYLES: u8 = 6;
    pub const HAIR_COLORS: u8 = 7;
    pub const SHIRTS: u8 = 10;
    pub const PANTS: u8 = 5;
}

pub mod gender {
    pub const FEMALE: u8 = 0;
    pub const MALE: u8 = 1;
    pub const OTHER: u8 = 2;
}

pub const MAX_CITY_BYTES: usize = 48;
pub const MAX_EMAIL_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Appearance {
    pub skin: u8,
    pub hair_style: u8,
    pub hair_color: u8,
    pub shirt: u8,
    pub pants: u8,
}

impl Appearance {
    pub fn is_valid(&self) -> bool {
        use appearance::*;
        self.skin < SKINS && self.hair_style < HAIR_STYLES && self.hair_color < HAIR_COLORS && self.shirt < SHIRTS && self.pants < PANTS
    }
}

/// Character profile from the creation screen. Only name, gender and
/// appearance are shared with other players; age, city and e-mail stay on
/// the server (the character's CV).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Profile {
    pub gender: u8,
    pub age: u8,
    pub city: String,
    pub email: String,
    pub appearance: Appearance,
}

pub mod disconnect {
    pub const CLIENT_QUIT: u8 = 0;
    pub const TIMEOUT: u8 = 1;
    pub const KICKED: u8 = 2;
    pub const SERVER_SHUTDOWN: u8 = 3;
    /// Reply to a packet whose token matches no session (expired / server
    /// restarted). The client should start a fresh `Connect`.
    pub const SESSION_UNKNOWN: u8 = 4;
}

/// What a character is doing (not simulated): `Snapshot::self_activity` and
/// `EntityState::activity`.
pub mod activity {
    pub const NONE: u8 = 0;
    /// Sitting at a computer (the client shows its screen while set).
    pub const COMPUTER: u8 = 1;
    pub const BREWING: u8 = 2;
    /// Resting on a sofa.
    pub const SOFA: u8 = 3;
    pub const TOILET: u8 = 4;
    pub const SMOKING: u8 = 5;
    /// Washing hands at a sink.
    pub const WASHING: u8 = 6;
    /// Riding a vehicle to work (hidden; the camera follows).
    pub const RIDING: u8 = 7;
}

/// `Clock::place`: where the receiver is.
pub mod place {
    /// In the building.
    pub const BUILDING: u8 = 0;
    /// At home for the night (after 22:00).
    pub const HOME: u8 = 1;
    /// On the way to work (morning, until `arrive`).
    pub const COMMUTING: u8 = 2;
    /// At home looking for a job (the portal).
    pub const PORTAL: u8 = 3;
}

/// `Clock::arrive` when there is no arrival time.
pub const NO_TIME: u16 = 0xFFFF;

/// `Doors::lift_target` when the elevator isn't heading anywhere.
pub const NO_FLOOR: u8 = 255;

/// `EntityState::flags` bit: walks slowly (exhausted / needs the toilet).
pub const FLAG_SLOW: u8 = 0x40;
/// `EntityState::flags` bit 3 for players (NPC looks use bits 3-5): an open
/// umbrella (outdoors in the rain).
pub const FLAG_UMBRELLA: u8 = 0x08;

/// `EntityState::flags` bit: low hygiene (a smell cloud others can see).
pub const FLAG_SMELLY: u8 = 0x80;
/// `Stats::flags` bit: dirty hands (after the toilet, until washed).
pub const STATS_DIRTY_HANDS: u8 = 1;

/// Entity kinds. Only players exist now; NPCs will use the same snapshot slot.
pub mod kind {
    pub const PLAYER: u8 = 0;
    pub const NPC: u8 = 1;
    /// An item lying on the floor (`EntityState::held` = item kind).
    pub const ITEM: u8 = 2;
    /// A laptop on a desk (`flags`: bit 0 locked, bit 1 in use; the owner's
    /// name comes as its `PlayerInfo`).
    pub const COMPUTER: u8 = 3;
    /// A vehicle (`held`: 1 car, 2 bike, 3 taxi, 4 tram; `flags` bits 0-2
    /// facing + moving like players).
    pub const VEHICLE: u8 = 4;
}

/// One conversation in the messenger sidebar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvEntry {
    pub conv: u16,
    pub unread: u8,
    pub title: String,
}

/// One product on a shop shelf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelfItem {
    pub kind: u8,
    /// Grosze.
    pub price: u32,
    pub name: String,
}

/// One messenger message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatEntry {
    pub id: u32,
    pub from: u16,
    pub nick: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityState {
    pub id: u16,
    pub kind: u8,
    pub x: i32,
    pub y: i32,
    /// Bit 0-1: facing (0 down, 1 up, 2 left, 3 right); bit 2: moving;
    /// bits 3-5 look; bit 6 slow (`FLAG_SLOW`); bit 7 smelly (`FLAG_SMELLY`).
    pub flags: u8,
    /// Item kind in hands (`inventory::kind`), or the item itself for `kind::ITEM`.
    pub held: u8,
    /// `activity::*`.
    pub activity: u8,
}

/// One inventory slot as sent to its owner.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SlotInfo {
    pub kind: u8,
    pub id: u32,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInfoEntry {
    pub id: u16,
    pub nick: String,
    /// Department (after signing the contract; 0 = none / NPC).
    pub department: u8,
    pub gender: u8,
    pub appearance: Appearance,
}

/// A job offer on the portal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferInfo {
    pub id: u8,
    pub department: u8,
    /// This player has already applied (pending, invited or answered).
    pub applied: bool,
    pub company: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    Connect { nonce: u32, nick: String, profile: Profile },
    Welcome { nonce: u32, player_id: u16, token: u32, tick_hz: u8, input_hz: u8, map_crc: u32, server_tick: u32 },
    Reject { reason: u8 },
    /// `inputs` are consecutive, oldest first; the last one has seq `last_seq`.
    Input { token: u32, ack_tick: u32, last_seq: u32, inputs: Vec<u8> },
    Snapshot {
        tick: u32,
        last_input_seq: u32,
        frag_idx: u8,
        frag_cnt: u8,
        self_x: i32,
        self_y: i32,
        floor: u8,
        room: u16,
        /// Receiver's `sim::Body::lock` and `prev_input`: with position and
        /// floor this is the full simulation state the client replays from.
        self_lock: u8,
        self_prev_input: u8,
        /// Receiver's rights (`map::access::*`): part of the simulated state.
        self_access: u8,
        /// Receiver's `sim::Body::slow` (simulated: movement speed).
        self_slow: u8,
        /// Receiver's activity (not simulated): see `activity`.
        self_activity: u8,
        entities: Vec<EntityState>,
    },
    PlayerInfo { players: Vec<PlayerInfoEntry> },
    InfoRequest { token: u32, ids: Vec<u16> },
    Ping { token: u32, client_time: u32 },
    Pong { client_time: u32, server_tick: u32 },
    Disconnect { token: u32, reason: u8 },
    /// Something an entity (NPC) says; shown as a speech bubble.
    Say { id: u16, text: String },
    /// Job portal: the offers (resent every second while on the portal).
    JobOffers { offers: Vec<OfferInfo> },
    /// Application form sent for an offer (`motivation`: free text).
    Apply { token: u32, offer: u8, motivation: String },
    /// Current recruitment question (resent every second until answered).
    Question { attempt: u8, index: u8, total: u8, text: String, options: Vec<String> },
    Answer { token: u32, attempt: u8, index: u8, choice: u8 },
    /// Outcome of an interview (a mail follows).
    RecruitResult { attempt: u8, passed: bool, score: u8, total: u8, department: u8 },
    /// A message in the in-game mailbox (resent while on the desktop; the
    /// client dedupes by `id`). `action`: `portal_action::*` button.
    Mail { id: u8, from: String, subject: String, body: String, action: u8, arg: u8 },
    /// Desktop button pressed (join interview / go to the office).
    PortalAction { token: u32, action: u8, arg: u8 },
    /// Owner's inventory: hands first, then the pockets (sent on change and
    /// every 2 s).
    Inventory { slots: Vec<SlotInfo> },
    /// Do something with an item (`item_action::*`).
    ItemAction { token: u32, action: u8, slot: u8 },
    /// Screen of the computer the receiver sits at (resent while seated).
    Computer { handle: u16, owner: u16, locked: bool, convs: Vec<ConvEntry> },
    ComputerAction { token: u32, action: u8, conv: u16, arg: u32, text: String },
    /// Messages of a conversation (sync reply or live push).
    Chat { conv: u16, messages: Vec<ChatEntry> },
    /// Character needs, 0..=100 each (sent to the owner twice a second).
    /// ... and the wallet (`money`, grosze).
    Stats { hunger: u8, energy: u8, stress: u8, bladder: u8, hygiene: u8, flags: u8, money: u32 },
    /// A shop shelf the receiver pressed E at: what's on it.
    Shelf { shelf: u8, title: String, goods: Vec<ShelfItem> },
    /// Take one `kind` off shelf `shelf` (unpaid, into the inventory).
    ShopTake { token: u32, shelf: u8, kind: u8 },
    /// Game time (shared) + the receiver's day: personal day number, minute
    /// of the day (0..1439), night (office closed), where they are
    /// (`place`), arrival time (minute of the day or `NO_TIME`), last payday
    /// (grosze, game minutes worked) and minutes worked today.
    /// + how the receiver commutes today (`mode`: 1 on foot, 2 bike, 3 car, 4
    /// taxi, 5 tram), departure (minute or `NO_TIME`) and the wallet (grosze).
    Clock {
        day: u16,
        minute: u16,
        night: bool,
        place: u8,
        arrive: u16,
        pay: u32,
        pay_minutes: u16,
        today_minutes: u16,
        mode: u8,
        depart: u16,
        money: u32,
        /// `weather::kind` (1 sun, 2 clouds, 3 rain, 4 storm, 5 fog).
        weather: u8,
    },
    /// Morning choice of how to get to work (before the departure).
    CommuteChoice { token: u32, mode: u8 },
    /// Closed doors (locked toilet stalls, elevator doors) on the receiver's
    /// floor: solid for the simulation. Plus the elevator: the floor it is at
    /// and where it is heading (`NO_FLOOR` = standing). Sent on change and
    /// every 0.5 s.
    Doors { floor: u8, tiles: Vec<(u8, u8)>, lift_floor: u8, lift_target: u8, lift_moving: bool },
    /// Lock / unlock the stall the sender is in.
    DoorAction { token: u32 },
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    TooShort,
    BadMagic,
    BadVersion(u8),
    UnknownType(u8),
    Invalid(&'static str),
}

struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn str8(&mut self, s: &str) {
        let b = truncate_utf8(s, MAX_NICK_BYTES).as_bytes();
        self.u8(b.len() as u8);
        self.0.extend_from_slice(b);
    }
    fn str16(&mut self, s: &str, max: usize) {
        let b = truncate_utf8(s, max).as_bytes();
        self.u16(b.len() as u16);
        self.0.extend_from_slice(b);
    }
    fn appearance(&mut self, a: &Appearance) {
        for v in [a.skin, a.hair_style, a.hair_color, a.shirt, a.pants] {
            self.u8(v);
        }
    }
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.pos + n > self.b.len() {
            return Err(DecodeError::TooShort);
        }
        let s = &self.b[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, DecodeError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn str8(&mut self) -> Result<String, DecodeError> {
        let n = self.u8()? as usize;
        if n > MAX_NICK_BYTES {
            return Err(DecodeError::Invalid("string too long"));
        }
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| DecodeError::Invalid("bad utf8"))
    }
    fn appearance(&mut self) -> Result<Appearance, DecodeError> {
        Ok(Appearance { skin: self.u8()?, hair_style: self.u8()?, hair_color: self.u8()?, shirt: self.u8()?, pants: self.u8()? })
    }
    fn str16(&mut self, max: usize) -> Result<String, DecodeError> {
        let n = self.u16()? as usize;
        if n > max {
            return Err(DecodeError::Invalid("string too long"));
        }
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| DecodeError::Invalid("bad utf8"))
    }
}

/// Cut a string to at most `max` bytes on a char boundary.
pub fn truncate_utf8(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

impl Packet {
    pub fn type_id(&self) -> u8 {
        match self {
            Packet::Connect { .. } => ty::CONNECT,
            Packet::Welcome { .. } => ty::WELCOME,
            Packet::Reject { .. } => ty::REJECT,
            Packet::Input { .. } => ty::INPUT,
            Packet::Snapshot { .. } => ty::SNAPSHOT,
            Packet::PlayerInfo { .. } => ty::PLAYER_INFO,
            Packet::InfoRequest { .. } => ty::INFO_REQUEST,
            Packet::Ping { .. } => ty::PING,
            Packet::Pong { .. } => ty::PONG,
            Packet::Disconnect { .. } => ty::DISCONNECT,
            Packet::Say { .. } => ty::SAY,
            Packet::JobOffers { .. } => ty::JOB_OFFERS,
            Packet::Apply { .. } => ty::APPLY,
            Packet::Question { .. } => ty::QUESTION,
            Packet::Answer { .. } => ty::ANSWER,
            Packet::RecruitResult { .. } => ty::RECRUIT_RESULT,
            Packet::Mail { .. } => ty::MAIL,
            Packet::PortalAction { .. } => ty::PORTAL_ACTION,
            Packet::Inventory { .. } => ty::INVENTORY,
            Packet::ItemAction { .. } => ty::ITEM_ACTION,
            Packet::Computer { .. } => ty::COMPUTER,
            Packet::ComputerAction { .. } => ty::COMPUTER_ACTION,
            Packet::Chat { .. } => ty::CHAT,
            Packet::Stats { .. } => ty::STATS,
            Packet::Shelf { .. } => ty::SHELF,
            Packet::ShopTake { .. } => ty::SHOP_TAKE,
            Packet::Clock { .. } => ty::CLOCK,
            Packet::CommuteChoice { .. } => ty::COMMUTE_CHOICE,
            Packet::Doors { .. } => ty::DOORS,
            Packet::DoorAction { .. } => ty::DOOR_ACTION,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer(Vec::with_capacity(64));
        w.u16(MAGIC);
        w.u8(VERSION);
        w.u8(self.type_id());
        match self {
            Packet::Connect { nonce, nick, profile } => {
                w.u32(*nonce);
                w.str8(nick);
                w.u8(profile.gender);
                w.u8(profile.age);
                w.appearance(&profile.appearance);
                w.str16(&profile.city, MAX_CITY_BYTES);
                w.str16(&profile.email, MAX_EMAIL_BYTES);
            }
            Packet::Welcome { nonce, player_id, token, tick_hz, input_hz, map_crc, server_tick } => {
                w.u32(*nonce);
                w.u16(*player_id);
                w.u32(*token);
                w.u8(*tick_hz);
                w.u8(*input_hz);
                w.u32(*map_crc);
                w.u32(*server_tick);
            }
            Packet::Reject { reason } => w.u8(*reason),
            Packet::Input { token, ack_tick, last_seq, inputs } => {
                w.u32(*token);
                w.u32(*ack_tick);
                w.u32(*last_seq);
                let n = inputs.len().min(MAX_INPUTS_PER_PACKET);
                w.u8(n as u8);
                for &i in &inputs[inputs.len() - n..] {
                    w.u8(i);
                }
            }
            Packet::Snapshot {
                tick,
                last_input_seq,
                frag_idx,
                frag_cnt,
                self_x,
                self_y,
                floor,
                room,
                self_lock,
                self_prev_input,
                self_access,
                self_slow,
                self_activity,
                entities,
            } => {
                w.u32(*tick);
                w.u32(*last_input_seq);
                w.u8(*frag_idx);
                w.u8(*frag_cnt);
                w.i32(*self_x);
                w.i32(*self_y);
                w.u8(*floor);
                w.u16(*room);
                w.u8(*self_lock);
                w.u8(*self_prev_input);
                w.u8(*self_access);
                w.u8(*self_slow);
                w.u8(*self_activity);
                let n = entities.len().min(MAX_ENTITIES_PER_SNAPSHOT);
                w.u8(n as u8);
                for e in &entities[..n] {
                    w.u16(e.id);
                    w.u8(e.kind);
                    w.i32(e.x);
                    w.i32(e.y);
                    w.u8(e.flags);
                    w.u8(e.held);
                    w.u8(e.activity);
                }
            }
            Packet::PlayerInfo { players } => {
                w.u8(players.len().min(255) as u8);
                for p in players.iter().take(255) {
                    w.u16(p.id);
                    w.str8(&p.nick);
                    w.u8(p.department);
                    w.u8(p.gender);
                    w.appearance(&p.appearance);
                }
            }
            Packet::InfoRequest { token, ids } => {
                w.u32(*token);
                w.u8(ids.len().min(255) as u8);
                for &id in ids.iter().take(255) {
                    w.u16(id);
                }
            }
            Packet::Ping { token, client_time } => {
                w.u32(*token);
                w.u32(*client_time);
            }
            Packet::Pong { client_time, server_tick } => {
                w.u32(*client_time);
                w.u32(*server_tick);
            }
            Packet::Disconnect { token, reason } => {
                w.u32(*token);
                w.u8(*reason);
            }
            Packet::Say { id, text } => {
                w.u16(*id);
                w.str16(text, MAX_SAY_BYTES);
            }
            Packet::JobOffers { offers } => {
                w.u8(offers.len().min(16) as u8);
                for o in offers.iter().take(16) {
                    w.u8(o.id);
                    w.u8(o.department);
                    w.u8(o.applied as u8);
                    w.str16(&o.company, MAX_TEXT_BYTES);
                    w.str16(&o.title, MAX_TEXT_BYTES);
                    w.str16(&o.description, MAX_TEXT_BYTES);
                }
            }
            Packet::Apply { token, offer, motivation } => {
                w.u32(*token);
                w.u8(*offer);
                w.str16(motivation, MAX_TEXT_BYTES);
            }
            Packet::Question { attempt, index, total, text, options } => {
                w.u8(*attempt);
                w.u8(*index);
                w.u8(*total);
                w.str16(text, MAX_TEXT_BYTES);
                w.u8(options.len().min(MAX_OPTIONS) as u8);
                for o in options.iter().take(MAX_OPTIONS) {
                    w.str16(o, MAX_TEXT_BYTES);
                }
            }
            Packet::Answer { token, attempt, index, choice } => {
                w.u32(*token);
                w.u8(*attempt);
                w.u8(*index);
                w.u8(*choice);
            }
            Packet::RecruitResult { attempt, passed, score, total, department } => {
                w.u8(*attempt);
                w.u8(*passed as u8);
                w.u8(*score);
                w.u8(*total);
                w.u8(*department);
            }
            Packet::Mail { id, from, subject, body, action, arg } => {
                w.u8(*id);
                w.str16(from, MAX_TEXT_BYTES);
                w.str16(subject, MAX_TEXT_BYTES);
                w.str16(body, MAX_MAIL_BYTES);
                w.u8(*action);
                w.u8(*arg);
            }
            Packet::PortalAction { token, action, arg } => {
                w.u32(*token);
                w.u8(*action);
                w.u8(*arg);
            }
            Packet::Inventory { slots } => {
                w.u8(slots.len().min(8) as u8);
                for sl in slots.iter().take(8) {
                    w.u8(sl.kind);
                    w.u32(sl.id);
                    w.str16(&sl.label, MAX_TEXT_BYTES);
                }
            }
            Packet::ItemAction { token, action, slot } => {
                w.u32(*token);
                w.u8(*action);
                w.u8(*slot);
            }
            Packet::Computer { handle, owner, locked, convs } => {
                w.u16(*handle);
                w.u16(*owner);
                w.u8(*locked as u8);
                w.u8(convs.len().min(MAX_CONVS) as u8);
                for c in convs.iter().take(MAX_CONVS) {
                    w.u16(c.conv);
                    w.u8(c.unread);
                    w.str16(&c.title, MAX_NICK_BYTES + 8);
                }
            }
            Packet::ComputerAction { token, action, conv, arg, text } => {
                w.u32(*token);
                w.u8(*action);
                w.u16(*conv);
                w.u32(*arg);
                w.str16(text, MAX_CHAT_BYTES);
            }
            Packet::Doors { floor, tiles, lift_floor, lift_target, lift_moving } => {
                w.u8(*floor);
                w.u8(tiles.len().min(255) as u8);
                for (x, y) in tiles.iter().take(255) {
                    w.u8(*x);
                    w.u8(*y);
                }
                w.u8(*lift_floor);
                w.u8(*lift_target);
                w.u8(*lift_moving as u8);
            }
            Packet::DoorAction { token } => w.u32(*token),
            Packet::Stats { hunger, energy, stress, bladder, hygiene, flags, money } => {
                w.u8(*hunger);
                w.u8(*energy);
                w.u8(*stress);
                w.u8(*bladder);
                w.u8(*hygiene);
                w.u8(*flags);
                w.u32(*money);
            }
            Packet::Shelf { shelf, title, goods } => {
                w.u8(*shelf);
                w.str16(title, MAX_TEXT_BYTES);
                w.u8(goods.len().min(16) as u8);
                for g in goods.iter().take(16) {
                    w.u8(g.kind);
                    w.u32(g.price);
                    w.str16(&g.name, MAX_TEXT_BYTES);
                }
            }
            Packet::ShopTake { token, shelf, kind } => {
                w.u32(*token);
                w.u8(*shelf);
                w.u8(*kind);
            }
            Packet::Clock { day, minute, night, place, arrive, pay, pay_minutes, today_minutes, mode, depart, money, weather } => {
                w.u16(*day);
                w.u16(*minute);
                w.u8(*night as u8);
                w.u8(*place);
                w.u16(*arrive);
                w.u32(*pay);
                w.u16(*pay_minutes);
                w.u16(*today_minutes);
                w.u8(*mode);
                w.u16(*depart);
                w.u32(*money);
                w.u8(*weather);
            }
            Packet::CommuteChoice { token, mode } => {
                w.u32(*token);
                w.u8(*mode);
            }
            Packet::Chat { conv, messages } => {
                w.u16(*conv);
                w.u8(messages.len().min(255) as u8);
                for m in messages.iter().take(255) {
                    w.u32(m.id);
                    w.u16(m.from);
                    w.str16(&m.nick, MAX_NICK_BYTES);
                    w.str16(&m.text, MAX_CHAT_BYTES);
                }
            }
        }
        w.0
    }

    pub fn decode(b: &[u8]) -> Result<Packet, DecodeError> {
        let mut r = Reader { b, pos: 0 };
        if r.u16()? != MAGIC {
            return Err(DecodeError::BadMagic);
        }
        let version = r.u8()?;
        if version != VERSION {
            return Err(DecodeError::BadVersion(version));
        }
        let t = r.u8()?;
        let p = match t {
            ty::CONNECT => {
                let (nonce, nick) = (r.u32()?, r.str8()?);
                let (gender, age, appearance) = (r.u8()?, r.u8()?, r.appearance()?);
                let city = r.str16(MAX_CITY_BYTES)?;
                let email = r.str16(MAX_EMAIL_BYTES)?;
                Packet::Connect { nonce, nick, profile: Profile { gender, age, city, email, appearance } }
            }
            ty::WELCOME => Packet::Welcome {
                nonce: r.u32()?,
                player_id: r.u16()?,
                token: r.u32()?,
                tick_hz: r.u8()?,
                input_hz: r.u8()?,
                map_crc: r.u32()?,
                server_tick: r.u32()?,
            },
            ty::REJECT => Packet::Reject { reason: r.u8()? },
            ty::INPUT => {
                let token = r.u32()?;
                let ack_tick = r.u32()?;
                let last_seq = r.u32()?;
                let n = r.u8()? as usize;
                if n > MAX_INPUTS_PER_PACKET {
                    return Err(DecodeError::Invalid("too many inputs"));
                }
                Packet::Input { token, ack_tick, last_seq, inputs: r.take(n)?.to_vec() }
            }
            ty::SNAPSHOT => {
                let tick = r.u32()?;
                let last_input_seq = r.u32()?;
                let frag_idx = r.u8()?;
                let frag_cnt = r.u8()?;
                let self_x = r.i32()?;
                let self_y = r.i32()?;
                let floor = r.u8()?;
                let room = r.u16()?;
                let self_lock = r.u8()?;
                let self_prev_input = r.u8()?;
                let self_access = r.u8()?;
                let self_slow = r.u8()?;
                let self_activity = r.u8()?;
                let n = r.u8()? as usize;
                let mut entities = Vec::with_capacity(n);
                for _ in 0..n {
                    entities.push(EntityState { id: r.u16()?, kind: r.u8()?, x: r.i32()?, y: r.i32()?, flags: r.u8()?, held: r.u8()?, activity: r.u8()? });
                }
                Packet::Snapshot {
                    tick,
                    last_input_seq,
                    frag_idx,
                    frag_cnt,
                    self_x,
                    self_y,
                    floor,
                    room,
                    self_lock,
                    self_prev_input,
                    self_access,
                    self_slow,
                    self_activity,
                    entities,
                }
            }
            ty::PLAYER_INFO => {
                let n = r.u8()? as usize;
                let mut players = Vec::with_capacity(n);
                for _ in 0..n {
                    players.push(PlayerInfoEntry {
                        id: r.u16()?,
                        nick: r.str8()?,
                        department: r.u8()?,
                        gender: r.u8()?,
                        appearance: r.appearance()?,
                    });
                }
                Packet::PlayerInfo { players }
            }
            ty::INFO_REQUEST => {
                let token = r.u32()?;
                let n = r.u8()? as usize;
                let mut ids = Vec::with_capacity(n);
                for _ in 0..n {
                    ids.push(r.u16()?);
                }
                Packet::InfoRequest { token, ids }
            }
            ty::PING => Packet::Ping { token: r.u32()?, client_time: r.u32()? },
            ty::PONG => Packet::Pong { client_time: r.u32()?, server_tick: r.u32()? },
            ty::DISCONNECT => Packet::Disconnect { token: r.u32()?, reason: r.u8()? },
            ty::SAY => Packet::Say { id: r.u16()?, text: r.str16(MAX_SAY_BYTES)? },
            ty::JOB_OFFERS => {
                let n = r.u8()? as usize;
                if n > 16 {
                    return Err(DecodeError::Invalid("too many offers"));
                }
                let mut offers = Vec::with_capacity(n);
                for _ in 0..n {
                    offers.push(OfferInfo {
                        id: r.u8()?,
                        department: r.u8()?,
                        applied: r.u8()? != 0,
                        company: r.str16(MAX_TEXT_BYTES)?,
                        title: r.str16(MAX_TEXT_BYTES)?,
                        description: r.str16(MAX_TEXT_BYTES)?,
                    });
                }
                Packet::JobOffers { offers }
            }
            ty::APPLY => Packet::Apply { token: r.u32()?, offer: r.u8()?, motivation: r.str16(MAX_TEXT_BYTES)? },
            ty::QUESTION => {
                let (attempt, index, total) = (r.u8()?, r.u8()?, r.u8()?);
                let text = r.str16(MAX_TEXT_BYTES)?;
                let n = r.u8()? as usize;
                if n > MAX_OPTIONS {
                    return Err(DecodeError::Invalid("too many options"));
                }
                let mut options = Vec::with_capacity(n);
                for _ in 0..n {
                    options.push(r.str16(MAX_TEXT_BYTES)?);
                }
                Packet::Question { attempt, index, total, text, options }
            }
            ty::ANSWER => Packet::Answer { token: r.u32()?, attempt: r.u8()?, index: r.u8()?, choice: r.u8()? },
            ty::RECRUIT_RESULT => {
                let attempt = r.u8()?;
                let passed = match r.u8()? {
                    0 => false,
                    1 => true,
                    _ => return Err(DecodeError::Invalid("bad bool")),
                };
                Packet::RecruitResult { attempt, passed, score: r.u8()?, total: r.u8()?, department: r.u8()? }
            }
            ty::MAIL => Packet::Mail {
                id: r.u8()?,
                from: r.str16(MAX_TEXT_BYTES)?,
                subject: r.str16(MAX_TEXT_BYTES)?,
                body: r.str16(MAX_MAIL_BYTES)?,
                action: r.u8()?,
                arg: r.u8()?,
            },
            ty::PORTAL_ACTION => Packet::PortalAction { token: r.u32()?, action: r.u8()?, arg: r.u8()? },
            ty::INVENTORY => {
                let n = r.u8()? as usize;
                if n > 8 {
                    return Err(DecodeError::Invalid("too many slots"));
                }
                let mut slots = Vec::with_capacity(n);
                for _ in 0..n {
                    slots.push(SlotInfo { kind: r.u8()?, id: r.u32()?, label: r.str16(MAX_TEXT_BYTES)? });
                }
                Packet::Inventory { slots }
            }
            ty::ITEM_ACTION => Packet::ItemAction { token: r.u32()?, action: r.u8()?, slot: r.u8()? },
            ty::COMPUTER => {
                let (handle, owner, locked) = (r.u16()?, r.u16()?, r.u8()? != 0);
                let n = r.u8()? as usize;
                if n > MAX_CONVS {
                    return Err(DecodeError::Invalid("too many conversations"));
                }
                let mut convs = Vec::with_capacity(n);
                for _ in 0..n {
                    convs.push(ConvEntry { conv: r.u16()?, unread: r.u8()?, title: r.str16(MAX_NICK_BYTES + 8)? });
                }
                Packet::Computer { handle, owner, locked, convs }
            }
            ty::COMPUTER_ACTION => Packet::ComputerAction {
                token: r.u32()?,
                action: r.u8()?,
                conv: r.u16()?,
                arg: r.u32()?,
                text: r.str16(MAX_CHAT_BYTES)?,
            },
            ty::DOORS => {
                let floor = r.u8()?;
                let n = r.u8()? as usize;
                let mut tiles = Vec::with_capacity(n);
                for _ in 0..n {
                    tiles.push((r.u8()?, r.u8()?));
                }
                Packet::Doors { floor, tiles, lift_floor: r.u8()?, lift_target: r.u8()?, lift_moving: r.u8()? != 0 }
            }
            ty::DOOR_ACTION => Packet::DoorAction { token: r.u32()? },
            ty::STATS => Packet::Stats {
                hunger: r.u8()?,
                energy: r.u8()?,
                stress: r.u8()?,
                bladder: r.u8()?,
                hygiene: r.u8()?,
                flags: r.u8()?,
                money: r.u32()?,
            },
            ty::SHELF => {
                let shelf = r.u8()?;
                let title = r.str16(MAX_TEXT_BYTES)?;
                let n = r.u8()? as usize;
                if n > 16 {
                    return Err(DecodeError::Invalid("too many goods"));
                }
                let mut goods = Vec::with_capacity(n);
                for _ in 0..n {
                    goods.push(ShelfItem { kind: r.u8()?, price: r.u32()?, name: r.str16(MAX_TEXT_BYTES)? });
                }
                Packet::Shelf { shelf, title, goods }
            }
            ty::SHOP_TAKE => Packet::ShopTake { token: r.u32()?, shelf: r.u8()?, kind: r.u8()? },
            ty::CLOCK => Packet::Clock {
                day: r.u16()?,
                minute: r.u16()?,
                night: r.u8()? != 0,
                place: r.u8()?,
                arrive: r.u16()?,
                pay: r.u32()?,
                pay_minutes: r.u16()?,
                today_minutes: r.u16()?,
                mode: r.u8()?,
                depart: r.u16()?,
                money: r.u32()?,
                weather: r.u8()?,
            },
            ty::COMMUTE_CHOICE => Packet::CommuteChoice { token: r.u32()?, mode: r.u8()? },
            ty::CHAT => {
                let conv = r.u16()?;
                let n = r.u8()? as usize;
                let mut messages = Vec::with_capacity(n.min(64));
                for _ in 0..n {
                    messages.push(ChatEntry {
                        id: r.u32()?,
                        from: r.u16()?,
                        nick: r.str16(MAX_NICK_BYTES)?,
                        text: r.str16(MAX_CHAT_BYTES)?,
                    });
                }
                Packet::Chat { conv, messages }
            }
            other => return Err(DecodeError::UnknownType(other)),
        };
        if r.pos != b.len() {
            return Err(DecodeError::Invalid("trailing bytes"));
        }
        Ok(p)
    }
}

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
    pub activity: u8,
}

/// Split a room's entity list into snapshot fragments that each fit in `MAX_PACKET`.
pub fn snapshot_fragments(tick: u32, last_input_seq: u32, me: SelfState, entities: &[EntityState]) -> Vec<Packet> {
    let chunks: Vec<&[EntityState]> = if entities.is_empty() {
        vec![&[]]
    } else {
        entities.chunks(MAX_ENTITIES_PER_SNAPSHOT).collect()
    };
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
            self_activity: me.activity,
            entities: c.to_vec(),
        })
        .collect()
}

/// Sample packets used by golden tests on both sides (Rust and GDScript).
pub fn golden_samples() -> Vec<(&'static str, Packet)> {
    vec![
        (
            "connect",
            Packet::Connect {
                nonce: 0xDEADBEEF,
                nick: "Zażółć".into(),
                profile: Profile {
                    gender: gender::FEMALE,
                    age: 27,
                    city: "Łódź".into(),
                    email: "ola@poczta.pl".into(),
                    appearance: Appearance { skin: 1, hair_style: 4, hair_color: 2, shirt: 9, pants: 3 },
                },
            },
        ),
        (
            "welcome",
            Packet::Welcome { nonce: 0xDEADBEEF, player_id: 7, token: 0x01020304, tick_hz: 20, input_hz: 60, map_crc: 0xCAFEBABE, server_tick: 1234 },
        ),
        ("reject", Packet::Reject { reason: reject::SERVER_FULL }),
        ("input", Packet::Input { token: 0x01020304, ack_tick: 1200, last_seq: 99, inputs: vec![0, 1, 9, 6] }),
        (
            "snapshot",
            Packet::Snapshot {
                tick: 1234,
                last_input_seq: 99,
                frag_idx: 0,
                frag_cnt: 1,
                self_x: 10_000,
                self_y: -5,
                floor: 1,
                room: 6,
                self_lock: 2,
                self_prev_input: 17,
                self_access: 5,
                self_slow: 1,
                self_activity: 3,
                entities: vec![
                    EntityState { id: 3, kind: kind::PLAYER, x: 4096, y: 8192, flags: 0b101, held: 3, activity: 1 },
                    EntityState { id: 65535, kind: kind::NPC, x: -1, y: 2_000_000, flags: 0x40, held: 0, activity: 0 },
                ],
            },
        ),
        (
            "player_info",
            Packet::PlayerInfo {
                players: vec![
                    PlayerInfoEntry {
                        id: 3,
                        nick: "Ala".into(),
                        department: 1,
                        gender: gender::FEMALE,
                        appearance: Appearance { skin: 2, hair_style: 1, hair_color: 3, shirt: 4, pants: 0 },
                    },
                    PlayerInfoEntry { id: 4, nick: "bot_07".into(), department: 0, gender: gender::MALE, appearance: Appearance::default() },
                ],
            },
        ),
        ("info_request", Packet::InfoRequest { token: 0x01020304, ids: vec![3, 4, 500] }),
        ("ping", Packet::Ping { token: 0x01020304, client_time: 777_000 }),
        ("pong", Packet::Pong { client_time: 777_000, server_tick: 1234 }),
        ("disconnect", Packet::Disconnect { token: 0x01020304, reason: disconnect::TIMEOUT }),
        ("say", Packet::Say { id: 61440, text: "Dzień dobry! Proszę za mną.".into() }),
        (
            "job_offers",
            Packet::JobOffers {
                offers: vec![
                    OfferInfo {
                        id: 1,
                        department: 1,
                        applied: true,
                        company: "Startup Sim sp. z o.o.".into(),
                        title: "Programista/ka".into(),
                        description: "Owocowe czwartki.".into(),
                    },
                    OfferInfo {
                        id: 12,
                        department: 0,
                        applied: false,
                        company: "Pizzeria u Stefana".into(),
                        title: "Dostawca/Dostawczyni".into(),
                        description: "Własny rower.".into(),
                    },
                ],
            },
        ),
        ("apply", Packet::Apply { token: 0x01020304, offer: 2, motivation: "Lubię kawę i wyzwania.".into() }),
        (
            "question",
            Packet::Question {
                attempt: 3,
                index: 1,
                total: 3,
                text: "Co oznacza kod HTTP 404?".into(),
                options: vec!["Nie znaleziono zasobu".into(), "Skończyła się kawa".into(), "Wszystko w porządku".into()],
            },
        ),
        ("answer", Packet::Answer { token: 0x01020304, attempt: 3, index: 1, choice: 2 }),
        ("recruit_result", Packet::RecruitResult { attempt: 3, passed: true, score: 2, total: 3, department: 1 }),
        (
            "mail",
            Packet::Mail {
                id: 2,
                from: "Startup Sim — Rekrutacja".into(),
                subject: "Zaproszenie na rozmowę".into(),
                body: "Cześć Ola! Zapraszamy na rozmowę online.".into(),
                action: portal_action::JOIN_INTERVIEW,
                arg: 1,
            },
        ),
        ("portal_action", Packet::PortalAction { token: 0x01020304, action: portal_action::GO_TO_OFFICE, arg: 0 }),
        (
            "inventory",
            Packet::Inventory {
                slots: vec![
                    SlotInfo { kind: 3, id: 77, label: "Laptop: Ola".into() },
                    SlotInfo { kind: 2, id: 76, label: "Ola · IT / Produkt".into() },
                    SlotInfo::default(),
                    SlotInfo::default(),
                ],
            },
        ),
        ("item_action", Packet::ItemAction { token: 0x01020304, action: item_action::TAKE_OUT, slot: 2 }),
        (
            "computer",
            Packet::Computer {
                handle: 0xE001,
                owner: 3,
                locked: false,
                convs: vec![
                    ConvEntry { conv: 1, unread: 0, title: "#ogólny".into() },
                    ConvEntry { conv: 17, unread: 2, title: "#it-produkt".into() },
                    ConvEntry { conv: 0x8004, unread: 1, title: "Kuba".into() },
                ],
            },
        ),
        (
            "computer_action",
            Packet::ComputerAction { token: 0x01020304, action: computer_action::SEND, conv: 17, arg: 42, text: "Kto zjadł mój jogurt?".into() },
        ),
        ("stats", Packet::Stats { hunger: 35, energy: 80, stress: 12, bladder: 64, hygiene: 22, flags: STATS_DIRTY_HANDS, money: 187_50 }),
        ("doors", Packet::Doors { floor: 1, tiles: vec![(46, 27), (54, 31)], lift_floor: 0, lift_target: 1, lift_moving: true }),
        ("door_action", Packet::DoorAction { token: 0x01020304 }),
        (
            "shelf",
            Packet::Shelf {
                shelf: 1,
                title: "Kanapki".into(),
                goods: vec![
                    ShelfItem { kind: 10, price: 12_00, name: "Kanapka z serem".into() },
                    ShelfItem { kind: 11, price: 14_00, name: "Kanapka z szynką".into() },
                ],
            },
        ),
        ("shop_take", Packet::ShopTake { token: 0x01020304, shelf: 1, kind: 11 }),
        (
            "clock",
            Packet::Clock {
                day: 2,
                minute: 8 * 60 + 12,
                night: false,
                place: place::COMMUTING,
                arrive: 9 * 60 + 5,
                pay: 230_00,
                pay_minutes: 460,
                today_minutes: 0,
                mode: 2,
                depart: 8 * 60 + 40,
                money: 186_00,
                weather: 3,
            },
        ),
        ("commute_choice", Packet::CommuteChoice { token: 0x01020304, mode: 5 }),
        (
            "chat",
            Packet::Chat {
                conv: 17,
                messages: vec![
                    ChatEntry { id: 5, from: 3, nick: "Ola".into(), text: "Deploy w piątek?".into() },
                    ChatEntry { id: 6, from: 4, nick: "Kuba".into(), text: "Nigdy w życiu.".into() },
                ],
            },
        ),
    ]
}

pub fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_samples() {
        for (name, p) in golden_samples() {
            let bytes = p.encode();
            assert!(bytes.len() <= MAX_PACKET, "{name}");
            assert_eq!(Packet::decode(&bytes).as_ref(), Ok(&p), "{name}");
        }
    }

    #[test]
    fn header_layout() {
        let b = Packet::Reject { reason: 1 }.encode();
        assert_eq!(b, vec![0x54, 0x53, VERSION, ty::REJECT, 1]);
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(Packet::decode(&[]), Err(DecodeError::TooShort));
        assert_eq!(Packet::decode(&[0, 0, 1, 1]), Err(DecodeError::BadMagic));
        assert_eq!(Packet::decode(&[0x54, 0x53, 99, 1]), Err(DecodeError::BadVersion(99)));
        assert_eq!(Packet::decode(&[0x54, 0x53, VERSION, 200]), Err(DecodeError::UnknownType(200)));
        let mut b = Packet::Ping { token: 1, client_time: 2 }.encode();
        b.push(0);
        assert!(Packet::decode(&b).is_err(), "trailing byte");
    }

    #[test]
    fn truncated_packets_never_panic() {
        for (_, p) in golden_samples() {
            let b = p.encode();
            for n in 0..b.len() {
                assert!(Packet::decode(&b[..n]).is_err());
            }
        }
    }

    #[test]
    fn random_bytes_never_panic() {
        let mut rng = fastrand::Rng::with_seed(1);
        for _ in 0..20_000 {
            let len = rng.usize(0..64);
            let mut b: Vec<u8> = (0..len).map(|_| rng.u8(..)).collect();
            if len >= 4 && rng.bool() {
                b[0] = 0x54;
                b[1] = 0x53;
                b[2] = VERSION;
                b[3] = rng.u8(1..=30);
            }
            let _ = Packet::decode(&b);
        }
    }

    #[test]
    fn long_speech_is_truncated_on_char_boundary() {
        let p = Packet::Say { id: 1, text: "ż".repeat(200) }; // 400 bytes
        let b = p.encode();
        assert!(b.len() <= MAX_PACKET);
        match Packet::decode(&b).unwrap() {
            Packet::Say { text, .. } => assert_eq!(text, "ż".repeat(MAX_SAY_BYTES / 2)),
            _ => unreachable!(),
        }
    }

    #[test]
    fn nick_is_truncated_on_char_boundary() {
        let p = Packet::Connect { nonce: 1, nick: "ąąąąąąąąąą".into(), profile: Profile::default() }; // 20 bytes
        match Packet::decode(&p.encode()).unwrap() {
            Packet::Connect { nick, .. } => assert_eq!(nick, "ąąąąąąąą"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn snapshot_fragments_fit_mtu() {
        let ents: Vec<EntityState> = (0..240)
            .map(|i| EntityState { id: i, kind: kind::PLAYER, x: i as i32 * 100, y: -(i as i32), flags: 0, held: 0, activity: 0 })
            .collect();
        let frags = snapshot_fragments(5, 6, SelfState { x: 1, y: 2, room: 3, ..Default::default() }, &ents);
        assert_eq!(frags.len(), 3);
        let mut seen = 0;
        for (i, f) in frags.iter().enumerate() {
            let b = f.encode();
            assert!(b.len() <= MAX_PACKET, "fragment {i} is {} B", b.len());
            match Packet::decode(&b).unwrap() {
                Packet::Snapshot { frag_idx, frag_cnt, entities, .. } => {
                    assert_eq!((frag_idx as usize, frag_cnt), (i, 3));
                    seen += entities.len();
                }
                _ => unreachable!(),
            }
        }
        assert_eq!(seen, 240);
        let full = &frags[0].encode();
        assert_eq!(full.len(), SNAPSHOT_FIXED_LEN + MAX_ENTITIES_PER_SNAPSHOT * ENTITY_LEN);
    }

    #[test]
    fn empty_room_still_sends_one_fragment() {
        let frags = snapshot_fragments(1, 0, SelfState::default(), &[]);
        assert_eq!(frags.len(), 1);
    }
}
