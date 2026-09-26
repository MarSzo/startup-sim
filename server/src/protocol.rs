//! Binary UDP protocol. See `docs/PROTOCOL.md`.
//!
//! Every packet: `magic u16 | version u8 | type u8 | payload`, little-endian.
//! `client/net/protocol.gd` mirrors this module; parity is checked against
//! `tests/golden/packets.json`.

pub const MAGIC: u16 = 0x5354; // "ST"
pub const VERSION: u8 = 6;
pub const HEADER_LEN: usize = 4;
/// Hard upper bound for any datagram we send.
pub const MAX_PACKET: usize = 1200;
pub const MAX_NICK_BYTES: usize = 16;
/// Max UTF-8 bytes of longer texts (speech, offers, questions, options).
pub const MAX_TEXT_BYTES: usize = 240;
pub const MAX_SAY_BYTES: usize = MAX_TEXT_BYTES;
/// Max answer options of a recruitment question.
pub const MAX_OPTIONS: usize = 4;
/// Max inputs carried in one Input packet.
pub const MAX_INPUTS_PER_PACKET: usize = 8;

/// Fixed part of a Snapshot packet (header + fields before the entity list).
pub const SNAPSHOT_FIXED_LEN: usize = HEADER_LEN + 4 + 4 + 1 + 1 + (4 + 4 + 1 + 2 + 1 + 1 + 1 + 1) + 1;
pub const ENTITY_LEN: usize = 12;
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

/// Activity bits: `Snapshot::self_status` bits 0..1, and the same two
/// bits at 6..7 of every entity's `flags`.
pub mod status {
    pub const HOLDING_COFFEE: u8 = 1;
    pub const BREWING: u8 = 2;
    /// Shift of the status bits inside `EntityState::flags`.
    pub const FLAGS_SHIFT: u8 = 6;
}

/// Entity kinds. Only players exist now; NPCs will use the same snapshot slot.
pub mod kind {
    pub const PLAYER: u8 = 0;
    pub const NPC: u8 = 1;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityState {
    pub id: u16,
    pub kind: u8,
    pub x: i32,
    pub y: i32,
    /// Bit 0-1: facing (0 down, 1 up, 2 left, 3 right); bit 2: moving. Rest reserved.
    pub flags: u8,
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
        /// Receiver's activity bits (not simulated): see `status`.
        self_status: u8,
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
    /// Candidate applies for an offer: starts a new attempt.
    Apply { token: u32, offer: u8 },
    /// Current recruitment question (resent every second until answered).
    Question { attempt: u8, index: u8, total: u8, text: String, options: Vec<String> },
    Answer { token: u32, attempt: u8, index: u8, choice: u8 },
    /// Outcome of an attempt. On success the player spawns in the world.
    RecruitResult { attempt: u8, passed: bool, score: u8, total: u8, department: u8 },
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
                self_status,
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
                w.u8(*self_status);
                let n = entities.len().min(MAX_ENTITIES_PER_SNAPSHOT);
                w.u8(n as u8);
                for e in &entities[..n] {
                    w.u16(e.id);
                    w.u8(e.kind);
                    w.i32(e.x);
                    w.i32(e.y);
                    w.u8(e.flags);
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
                    w.str16(&o.title, MAX_TEXT_BYTES);
                    w.str16(&o.description, MAX_TEXT_BYTES);
                }
            }
            Packet::Apply { token, offer } => {
                w.u32(*token);
                w.u8(*offer);
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
                let self_status = r.u8()?;
                let n = r.u8()? as usize;
                let mut entities = Vec::with_capacity(n);
                for _ in 0..n {
                    entities.push(EntityState { id: r.u16()?, kind: r.u8()?, x: r.i32()?, y: r.i32()?, flags: r.u8()? });
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
                    self_status,
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
                        title: r.str16(MAX_TEXT_BYTES)?,
                        description: r.str16(MAX_TEXT_BYTES)?,
                    });
                }
                Packet::JobOffers { offers }
            }
            ty::APPLY => Packet::Apply { token: r.u32()?, offer: r.u8()? },
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
    pub status: u8,
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
            self_status: me.status,
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
                self_status: 1,
                entities: vec![
                    EntityState { id: 3, kind: kind::PLAYER, x: 4096, y: 8192, flags: 0b101 },
                    EntityState { id: 65535, kind: kind::NPC, x: -1, y: 2_000_000, flags: 0 },
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
                    OfferInfo { id: 1, department: 1, title: "Programista/ka".into(), description: "Owocowe czwartki.".into() },
                    OfferInfo { id: 2, department: 2, title: "Marketing i sprzedaż".into(), description: "Kubek z logo.".into() },
                ],
            },
        ),
        ("apply", Packet::Apply { token: 0x01020304, offer: 2 }),
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
        assert_eq!(Packet::decode(&[0x54, 0x53, 9, 1]), Err(DecodeError::BadVersion(9)));
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
                b[3] = rng.u8(1..=16);
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
        let ents: Vec<EntityState> = (0..250)
            .map(|i| EntityState { id: i, kind: kind::PLAYER, x: i as i32 * 100, y: -(i as i32), flags: 0 })
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
        assert_eq!(seen, 250);
        let full = &frags[0].encode();
        assert_eq!(full.len(), SNAPSHOT_FIXED_LEN + MAX_ENTITIES_PER_SNAPSHOT * ENTITY_LEN);
    }

    #[test]
    fn empty_room_still_sends_one_fragment() {
        let frags = snapshot_fragments(1, 0, SelfState::default(), &[]);
        assert_eq!(frags.len(), 1);
    }
}
