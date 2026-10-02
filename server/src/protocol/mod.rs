//! Binary UDP protocol. See `docs/PROTOCOL.md`.
//!
//! Every packet: `magic u16 | version u8 | type u8 | payload`, little-endian.
//! `client/net/protocol.gd` mirrors this module; parity is checked against
//! `tests/golden/packets.json`.
//!
//! Layout: [`codec`] (byte reader / writer), `encode` / `decode` (one match
//! arm per packet type), [`snapshot_fragments`] (MTU-sized snapshots) and
//! [`golden_samples`] (parity vectors for the GDScript mirror). A new packet
//! type needs: a `ty` id, a `Packet` variant, an arm in `type_id`, `encode`
//! and `decode`, and a golden sample.

mod codec;
mod decode;
mod encode;
mod golden;
mod snapshot;
#[cfg(test)]
mod tests;

pub use codec::truncate_utf8;
pub use golden::{golden_samples, to_hex};
pub use snapshot::{snapshot_fragments, SelfState};

pub const MAGIC: u16 = 0x5354; // "ST"
pub const VERSION: u8 = 41;
pub const HEADER_LEN: usize = 4;
/// Hard upper bound for any datagram we send.
/// A game packet at most (sealed, it grows by up to 48 B to `MAX_DATAGRAM`).
pub const MAX_PACKET: usize = 1152;
/// A UDP datagram at most (a sealed packet included).
pub const MAX_DATAGRAM: usize = 1200;
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
pub const SNAPSHOT_FIXED_LEN: usize = HEADER_LEN + 4 + 4 + 1 + 1 + (4 + 4 + 1 + 2 + 1 + 1 + 1 + 1 + 1 + 1) + 1;
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
    pub const CALENDAR: u8 = 31;
    pub const CALENDAR_BOOK: u8 = 32;
    pub const DIALOG: u8 = 33;
    pub const DIALOG_ANSWER: u8 = 34;
    pub const LUNCH_MENU: u8 = 35;
    pub const LUNCH_ORDER: u8 = 36;
    pub const COMPANY_OFFERS: u8 = 37;
    pub const COMPANY_PEOPLE: u8 = 38;
    pub const COMPANY_ACTION: u8 = 39;
    pub const SMOKE: u8 = 40;
    pub const LIGHTS: u8 = 41;
    pub const FRIDGE: u8 = 42;
    pub const FRIDGE_ACTION: u8 = 43;
    pub const SKIP_WAIT: u8 = 44;
    pub const SOUND: u8 = 45;
    pub const TASK_ACTION: u8 = 46;
    pub const TASK_BOARD: u8 = 47;
    pub const TASK_DETAIL: u8 = 48;
    pub const MAIL_ACTION: u8 = 49;
    pub const WORK_MAIL: u8 = 50;
    pub const MAIL_STATE: u8 = 51;
    pub const VOICE: u8 = 52;
    pub const VOICE_FROM: u8 = 53;
    pub const DEPARTMENTS: u8 = 54;
    pub const ACTION: u8 = 55;
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
    /// The login expired (log in again / refresh).
    pub const BAD_TICKET: u8 = 5;
    /// This server needs an account (no guests).
    pub const GUESTS_OFF: u8 = 6;
    /// The nick is taken (an account, or someone playing under it now).
    pub const NICK_TAKEN: u8 = 7;
    /// Another character already has this e-mail.
    pub const EMAIL_TAKEN: u8 = 8;
}

/// A login ticket's length at most (bytes, hex).
pub const MAX_TICKET_BYTES: usize = 64;

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
    /// Stopped by the guard / the police (can't move for a moment).
    pub const HELD: u8 = 8;
    /// Throwing up (a moment; leaves a puddle).
    pub const VOMITING: u8 = 9;
    /// Passed out drunk (asleep on the floor for a while).
    pub const PASSED_OUT: u8 = 10;
    /// Knocked out in a fight (on the floor, stars).
    pub const KNOCKED_OUT: u8 = 11;
    /// Throwing a punch / stabbing (a moment).
    pub const ATTACKING: u8 = 12;
    /// Peeing standing up (urinal, floor, a machine, a mug).
    pub const PEEING: u8 = 13;
    /// Squatting: pooping on the floor.
    pub const POOPING: u8 = 14;
}

/// `EntityState::held` of a puddle (`kind::PUDDLE`).
pub mod puddle {
    pub const PEE: u8 = 0;
    pub const VOMIT: u8 = 1;
    pub const POOP: u8 = 2;
}

/// Form of employment (`Apply::form`, the contract).
pub mod employment {
    /// Umowa o pracę (an employment contract).
    pub const EMPLOYMENT: u8 = 1;
    /// B2B (own company: more on paper, no advance).
    pub const B2B: u8 = 2;
    /// Umowa zlecenie (only students under 26).
    pub const MANDATE: u8 = 3;
}

/// `Action::action` (C→S).
pub mod action {
    /// R: the menu of what can be done here (answered with a `Dialog`).
    pub const MENU: u8 = 1;
    /// X: punch (or stab, with a knife in hands) the nearest person.
    pub const ATTACK: u8 = 2;
}

/// `Sound` kinds: things happening in the world that others hear too.
pub mod sound {
    pub const COFFEE: u8 = 1;
    pub const TILL: u8 = 2;
    pub const GATE_ALARM: u8 = 3;
    pub const DING: u8 = 4;
    pub const LOCK: u8 = 5;
    pub const SWITCH: u8 = 6;
    pub const FLUSH: u8 = 7;
    pub const TAP: u8 = 8;
    pub const LIGHTER: u8 = 9;
    pub const DISHWASHER: u8 = 10;
    pub const FRIDGE: u8 = 11;
    pub const CUPBOARD: u8 = 12;
    pub const PICKUP: u8 = 13;
    pub const DROP: u8 = 14;
    pub const EAT: u8 = 15;
    pub const DRINK: u8 = 16;
    pub const WHISTLE: u8 = 17;
    /// A burp after a beer (the client plays it a moment later).
    pub const BURP: u8 = 18;
    pub const VOMIT: u8 = 19;
    pub const PUNCH: u8 = 20;
    pub const STAB: u8 = 21;
    pub const PEE: u8 = 22;
    pub const POOP: u8 = 23;
}

/// A department of the company in `Departments`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepartmentInfo {
    pub id: u8,
    /// Next to a nick ("Ola · IT").
    pub short: String,
    pub name: String,
}

/// Most departments in one `Departments` packet.
pub const MAX_DEPARTMENTS: usize = 32;

/// One elevator in `Doors`: the floor it is at (or left), where it is
/// heading (`NO_FLOOR` = standing), whether it is moving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lift {
    pub floor: u8,
    pub target: u8,
    pub moving: bool,
}

/// A position of our startup in the founder's panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanyOffer {
    pub id: u8,
    pub places: u8,
    pub department: u8,
    /// Question set id (`QuestionSet::id`).
    pub set: String,
    pub title: String,
    pub description: String,
}

/// A card on the task board (without its description and comments).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCard {
    pub id: u16,
    pub column: u8,
    pub priority: u8,
    pub comments: u8,
    pub title: String,
    pub author: String,
    pub assignee: String,
}

pub mod task_action {
    pub const SYNC: u8 = 0;
    pub const CREATE: u8 = 1;
    pub const MOVE: u8 = 2;
    pub const ASSIGN: u8 = 3;
    pub const PRIORITY: u8 = 4;
    pub const COMMENT: u8 = 5;
    pub const DELETE: u8 = 6;
    pub const EDIT: u8 = 7;
}

pub mod mail_action {
    pub const SYNC: u8 = 0;
    pub const SEND: u8 = 1;
    pub const TRASH: u8 = 2;
    pub const RESTORE: u8 = 3;
    pub const EMPTY_TRASH: u8 = 4;
}

/// Text limits (bytes) of the board and mail packets.
pub const TASK_TITLE_MAX: usize = 80;
pub const TASK_TEXT_MAX: usize = 400;
pub const TASK_COMMENT_MAX: usize = 120;
/// Comments in a TaskDetail.
pub const DETAIL_COMMENTS: usize = 6;
pub const MAIL_SUBJECT_MAX: usize = 80;
pub const MAIL_BODY_MAX: usize = 400;
/// Board members / mail ids in one packet at most.
pub const MAX_MEMBERS: usize = 24;
pub const MAX_MAIL_IDS: usize = 64;

/// Bytes of one voice frame at most.
pub const MAX_VOICE_BYTES: usize = 800;

/// Most sounds in one `Sound` packet.
pub const MAX_SOUNDS: usize = 64;

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

/// `Calendar` slot states.
pub mod slot {
    pub const FREE: u8 = 0;
    pub const TAKEN: u8 = 1;
    pub const MINE: u8 = 2;
    pub const PAST: u8 = 3;
}

/// `Clock::arrive` when there is no arrival time.
pub const NO_TIME: u16 = 0xFFFF;

/// `Lift::target` when the elevator isn't heading anywhere.
pub const NO_FLOOR: u8 = 255;

/// `EntityState::flags` bit: walks slowly (exhausted / needs the toilet).
pub const FLAG_SLOW: u8 = 0x40;
/// `EntityState::flags` bit 3 for players (NPC looks use bits 3-5): an open
/// umbrella (outdoors in the rain).
pub const FLAG_UMBRELLA: u8 = 0x08;

/// `EntityState::flags` bits 4-5 for players: how drunk it shows (0 sober,
/// 1 tipsy, 2 drunk, 3 very drunk).
pub const FLAG_DRUNK_SHIFT: u8 = 4;
pub const FLAG_DRUNK_MASK: u8 = 0x30;

/// `EntityState::flags` bit: low hygiene (a smell cloud others can see).
pub const FLAG_SMELLY: u8 = 0x80;
/// `Stats::flags` bit: dirty hands (after the toilet, until washed).
pub const STATS_DIRTY_HANDS: u8 = 1;
/// `Stats::flags` bit: upset stomach (stale fruit) - run to the toilet.
pub const STATS_UPSET: u8 = 2;

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
    /// A tray of sweets in the chill room (`held`: the sweet, `activity`:
    /// pieces left).
    pub const TRAY: u8 = 5;
    /// A puddle left by a toilet accident (no state; gone at 22:00).
    pub const PUDDLE: u8 = 6;
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

/// A dish in the lunch app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dish {
    pub kind: u8,
    /// Grosze.
    pub price: u32,
    /// Typical delivery, game minutes.
    pub eta: u8,
    pub name: String,
    pub restaurant: String,
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
    /// Open positions (our startup; 0 for other companies).
    pub vacancies: u8,
    /// Pay range, zł a month gross (0, 0 = not given).
    pub salary_min: u32,
    pub salary_max: u32,
    pub company: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    /// `ticket` = from logging in over HTTPS (the account's nick wins over
    /// `nick`); empty = a guest (only if the server allows guests).
    Connect {
        nonce: u32,
        nick: String,
        profile: Profile,
        ticket: String,
    },
    Welcome {
        nonce: u32,
        player_id: u16,
        token: u32,
        tick_hz: u8,
        input_hz: u8,
        map_crc: u32,
        server_tick: u32,
    },
    Reject {
        reason: u8,
    },
    /// `inputs` are consecutive, oldest first; the last one has seq `last_seq`.
    Input {
        token: u32,
        ack_tick: u32,
        last_seq: u32,
        inputs: Vec<u8>,
    },
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
        /// Receiver's `sim::Body::drunk` (simulated: staggering).
        self_drunk: u8,
        /// Receiver's activity (not simulated): see `activity`.
        self_activity: u8,
        entities: Vec<EntityState>,
    },
    PlayerInfo {
        players: Vec<PlayerInfoEntry>,
    },
    InfoRequest {
        token: u32,
        ids: Vec<u16>,
    },
    Ping {
        token: u32,
        client_time: u32,
    },
    Pong {
        client_time: u32,
        server_tick: u32,
    },
    Disconnect {
        token: u32,
        reason: u8,
    },
    /// Something an entity (NPC) says; shown as a speech bubble.
    Say {
        id: u16,
        text: String,
    },
    /// Job portal: the offers (resent every second while on the portal).
    JobOffers {
        offers: Vec<OfferInfo>,
    },
    /// Application form sent for an offer (`motivation`: free text).
    Apply {
        token: u32,
        offer: u8,
        motivation: String,
        /// Expected pay, zł a month gross.
        salary: u32,
        /// `employment::*`.
        form: u8,
        /// "I'm a student" (needed for a contract of mandate, with age < 26).
        student: bool,
    },
    /// Current recruitment question (resent every second until answered).
    Question {
        attempt: u8,
        index: u8,
        total: u8,
        text: String,
        options: Vec<String>,
    },
    Answer {
        token: u32,
        attempt: u8,
        index: u8,
        choice: u8,
    },
    /// Outcome of an interview (a mail follows).
    RecruitResult {
        attempt: u8,
        passed: bool,
        score: u8,
        total: u8,
        department: u8,
    },
    /// A message in the in-game mailbox (resent while on the desktop; the
    /// client dedupes by `id`). `action`: `portal_action::*` button.
    Mail {
        id: u8,
        from: String,
        subject: String,
        body: String,
        action: u8,
        arg: u8,
    },
    /// Desktop button pressed (join interview / go to the office).
    PortalAction {
        token: u32,
        action: u8,
        arg: u8,
    },
    /// Owner's inventory: hands first, then the pockets (sent on change and
    /// every 2 s).
    Inventory {
        slots: Vec<SlotInfo>,
    },
    /// Do something with an item (`item_action::*`).
    ItemAction {
        token: u32,
        action: u8,
        slot: u8,
    },
    /// Screen of the computer the receiver sits at (resent while seated).
    Computer {
        handle: u16,
        owner: u16,
        locked: bool,
        convs: Vec<ConvEntry>,
    },
    ComputerAction {
        token: u32,
        action: u8,
        conv: u16,
        arg: u32,
        text: String,
    },
    /// Messages of a conversation (sync reply or live push).
    Chat {
        conv: u16,
        messages: Vec<ChatEntry>,
    },
    /// Character needs, 0..=100 each (sent to the owner twice a second).
    /// ... and the wallet (`money`, grosze).
    Stats {
        hunger: u8,
        energy: u8,
        stress: u8,
        bladder: u8,
        hygiene: u8,
        /// Alcohol, 0..100 (75 throws up, 100 after that: passes out).
        alcohol: u8,
        /// Bowels, 0..100 (100 = an accident).
        bowels: u8,
        /// Health, 100 = fine, 0 = knocked out.
        health: u8,
        flags: u8,
        money: u32,
    },
    /// A shop shelf the receiver pressed E at: what's on it.
    Shelf {
        shelf: u8,
        title: String,
        goods: Vec<ShelfItem>,
    },
    /// Take one `kind` off shelf `shelf` (unpaid, into the inventory).
    ShopTake {
        token: u32,
        shelf: u8,
        kind: u8,
    },
    /// Game time (shared) + the receiver's day: personal day number, minute
    /// of the day (0..1439), night (office closed), where they are
    /// (`place`), arrival time (minute of the day or `NO_TIME`), last payday
    /// (grosze, game minutes worked) and minutes worked today.
    /// Plus how the receiver commutes today (`mode`: 1 on foot, 2 bike, 3 car, 4
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
        /// The company: its name and whether it has a founder (else the
        /// portal offers to found it).
        company: String,
        founded: bool,
        /// Fire alarm in the building: 1 = evacuate (fire.rs).
        alarm: u8,
        /// Skipping the wait at home: 0 no, 1 asked (waiting for the others
        /// at home), 2 time is flying.
        skip: u8,
    },
    /// Morning choice of how to get to work (before the departure).
    CommuteChoice {
        token: u32,
        mode: u8,
    },
    /// The board's calendar for today, as seen by the account of the computer
    /// the receiver sits at: its own booking (`mine_start` / `mine_topic`,
    /// `NO_TIME` = none) and every slot (`slot::*`).
    Calendar {
        mine_start: u16,
        mine_topic: u8,
        slots: Vec<(u16, u8)>,
    },
    /// Book `start` (minute of today) for `topic` (`board::topic`); topic 0
    /// cancels the booking.
    CalendarBook {
        token: u32,
        start: u16,
        topic: u8,
    },
    /// A conversation with an NPC (meeting): question and answers. `id` 0 =
    /// no conversation (close the window). Resent while open.
    Dialog {
        id: u8,
        npc: u16,
        text: String,
        options: Vec<String>,
    },
    DialogAnswer {
        token: u32,
        id: u8,
        choice: u8,
    },
    /// Lunch app (for the account of the computer the receiver sits at):
    /// order state (`lunch::state`), the dish ordered, its arrival (minute
    /// of the day or `NO_TIME`), and the menu.
    LunchMenu {
        state: u8,
        dish: u8,
        arrives: u16,
        dishes: Vec<Dish>,
    },
    LunchOrder {
        token: u32,
        dish: u8,
    },
    /// Company panel (the founder's computer): the name, the question sets
    /// to choose from (id, name; part 0) and the positions, in parts of at
    /// most `MAX_PACKET` (the client puts them together).
    CompanyOffers {
        name: String,
        part: u8,
        parts: u8,
        sets: Vec<(String, String)>,
        offers: Vec<CompanyOffer>,
    },
    /// Company panel: candidates (player, offer, score, total, nick) and
    /// staff (player, department, hired on day, reprimands, nick).
    CompanyPeople {
        candidates: Vec<(u16, u8, u8, u8, String)>,
        staff: Vec<(u16, u8, u16, u8, String)>,
    },
    /// Found the company (from the portal) / run it (panel): `company::action`.
    CompanyAction {
        token: u32,
        action: u8,
        target: u16,
        value: u8,
        text: String,
    },
    /// Smoke on the receiver's floor: (room, level 1..=255); rooms not listed
    /// are clear. Every second.
    Smoke {
        floor: u8,
        rooms: Vec<(u16, u8)>,
    },
    /// Lamps switched on on the receiver's floor (rooms); every second.
    Lights {
        floor: u8,
        rooms: Vec<u16>,
    },
    /// The fridge (E at it, and after every change while open): what's
    /// stored (item kind, label), portions of milk, free water and juice.
    Fridge {
        items: Vec<(u8, String)>,
        milk: u8,
        water: u8,
        juice: u8,
    },
    /// Take / put / pour milk (`kitchen::action`), `arg` = stored item index.
    FridgeAction {
        token: u32,
        action: u8,
        arg: u8,
    },
    /// At home: "skip the waiting" (to the morning / departure; once
    /// everybody at home asked).
    SkipWait {
        token: u32,
    },
    /// R (menu of actions) / X (attack): `action::*`.
    Action {
        token: u32,
        action: u8,
    },
    /// Sounds heard this tick on the receiver's floor: (kind, x, y) in
    /// sub-pixels (`sound::*`), at most `MAX_SOUNDS`.
    Sound {
        sounds: Vec<(u8, i32, i32)>,
    },
    /// The task board of the computer owner's department (`task_action`),
    /// acting as the owner. `nonce` dedupes retries (0 = none); `task` = the
    /// card (SYNC: whose details to send), `arg` = column / priority, `text`
    /// = title + "\n" + description, a nick or a comment.
    TaskAction {
        token: u32,
        nonce: u16,
        action: u8,
        task: u16,
        arg: u8,
        text: String,
    },
    /// The board (answer to every TaskAction), in parts of at most
    /// `MAX_PACKET`: `done` = the last applied nonce; `members` (part 0) =
    /// the department's employees.
    TaskBoard {
        dept: u8,
        done: u16,
        part: u8,
        parts: u8,
        members: Vec<String>,
        tasks: Vec<TaskCard>,
    },
    /// One card's description and its latest comments (nick, text).
    TaskDetail {
        id: u16,
        desc: String,
        comments: Vec<(String, String)>,
    },
    /// Work mail as the computer owner (`mail_action`): SYNC (`id` = the
    /// newest one the client has), SEND (`to`, `subject`, `body`), TRASH /
    /// RESTORE `id`, EMPTY_TRASH.
    MailAction {
        token: u32,
        nonce: u16,
        action: u8,
        id: u16,
        to: String,
        subject: String,
        body: String,
    },
    /// One mail of the owner's inbox (sent after a SYNC for newer ids).
    WorkMail {
        id: u16,
        from: String,
        to: String,
        subject: String,
        body: String,
        day: u16,
        minute: u16,
    },
    /// Answer to every MailAction: the last applied nonce, the ids in the
    /// inbox and which of them are in the trash.
    MailState {
        done: u16,
        ids: Vec<u16>,
        trashed: Vec<u16>,
    },
    /// Push-to-talk: one voice frame (opaque to the server: 16 kHz IMA
    /// ADPCM, see PROTOCOL.md) to the room, or whispered to the nearest
    /// person within reach (`whisper` = 1).
    Voice {
        token: u32,
        seq: u16,
        whisper: u8,
        data: Vec<u8>,
    },
    /// A voice frame relayed from `speaker`.
    VoiceFrom {
        speaker: u16,
        seq: u16,
        whisper: u8,
        data: Vec<u8>,
    },
    /// Closed doors (locked toilet stalls, elevator doors) on the receiver's
    /// floor: solid for the simulation. Plus every elevator, in the order
    /// the map lists them (floors ascending, links in file order, grouped by
    /// id). Sent on change and every 0.5 s.
    Doors {
        floor: u8,
        tiles: Vec<(u8, u8)>,
        lifts: Vec<Lift>,
    },
    /// Lock / unlock the stall the sender is in.
    DoorAction {
        token: u32,
    },
    /// The company's departments (ids used everywhere else): after
    /// `Welcome` and every 5 s.
    Departments {
        list: Vec<DepartmentInfo>,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    TooShort,
    BadMagic,
    BadVersion(u8),
    UnknownType(u8),
    Invalid(&'static str),
}
