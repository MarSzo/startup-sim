//! Authoritative game server: fixed 20 Hz tick, handshake, input processing,
//! room-based interest management and per-client snapshots.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use crate::board::{self, Meeting};
use crate::building::Building;
use crate::clock::{self, Clock, Transition};
use crate::coffee::{self, Cup, Machine};
use crate::commute::{self, Vehicle, VehicleEvent};
use crate::computer::{self, Account, Computer, Messenger, Workstation};
use crate::elevator::{self, Elevator};
use crate::inventory::{self, kind as item_kind, Inventory, Item};
use crate::net::{canonical, LinkConditions, Net};
use crate::needs::{self, Needs, Rest, Spot, SpotKind};
use crate::npc::{self, Npc};
use crate::protocol::{self as proto, EntityState, Packet, PlayerInfoEntry, Profile, SelfState};
use crate::recruitment::{Attempt, Recruitment};
use crate::sim::{self, Body, Pos};
use crate::shop::{self, Shelf};
use crate::stalls::{self, Stall};
use crate::weather::{self, Weather};

pub const TICK_HZ: u32 = 20;
pub const TICK: Duration = Duration::from_millis(1000 / TICK_HZ as u64);
pub const DEFAULT_CLIENT_TIMEOUT: Duration = Duration::from_secs(5);
/// Max input steps applied per client per tick (3 expected at 60/20 Hz; the
/// slack absorbs jitter, the cap stops speed hacks).
pub const MAX_INPUTS_PER_TICK: usize = 6;
/// Inputs buffered beyond this are dropped (client running ahead / flooding).
pub const MAX_INPUT_QUEUE: usize = 30;
/// PlayerInfo entries per packet (worst case 2 + 1 + 16 B each -> ~1100 B).
const INFO_PER_PACKET: usize = 55;

pub struct Config {
    pub bind: SocketAddr,
    pub link: LinkConditions,
    pub max_players: usize,
    pub stats_every: Duration,
    pub client_timeout: Duration,
    /// Rights every new player starts with (`map::access::*`); 0 in normal
    /// play, CARD for load tests (`--start-with-card`) so bots pass the gates.
    pub start_access: u8,
    /// Job portal offers and quizzes.
    pub recruitment: Recruitment,
    /// Spawn straight into the world (dev / tests), no job portal.
    pub skip_recruitment: bool,
    /// Also already hired (contract, card, laptop) and spawned at a desk of
    /// the department (odd player ids IT, even Biznes). Dev / tests.
    pub start_employed: bool,
    /// Needs change this many times faster (dev / testing; 1 = normal).
    pub needs_speed: u32,
    /// Game time when the server starts (minute of day 1).
    pub start_minute: u32,
    /// Daytime clock speed multiplier (dev / testing; 1 = 1 game hour per 5 min).
    pub time_scale: u32,
    /// Fixed weather (`weather::kind`; dev / tests), None = changing.
    pub weather: Option<u8>,
}

/// Where a connected player is in the game.
enum Stage {
    /// At home, on the computer desktop: job portal, mail, online interview.
    /// Not in the world yet.
    Portal(Box<Desk>),
    /// Hired and in the building.
    Working,
    /// Hired, out of the building: at home for the night (`None`) or on the
    /// way to work, arriving at game minute `Some(t)` (`Clock::total_minutes`).
    Home { arrive_at: Option<u32> },
}

/// Desktop state of a candidate (GDD 9a, step 2).
#[derive(Default)]
struct Desk {
    /// Offers applied for (shown as "applied" on the portal).
    applied: Vec<u8>,
    /// Replies to send: (offer, due tick).
    pending: Vec<(u8, u32)>,
    /// Offers with an interview invitation.
    invited: Vec<u8>,
    /// Online interview in progress.
    attempt: Option<Attempt>,
    /// Passed an interview: department, waiting for "go to the office".
    hired: Option<u8>,
    inbox: Vec<MailMsg>,
    next_mail: u8,
}

struct MailMsg {
    id: u8,
    from: String,
    subject: String,
    body: String,
    action: u8,
    arg: u8,
}

impl Desk {
    fn mail(&mut self, from: &str, subject: String, body: String, action: u8, arg: u8) {
        self.next_mail = self.next_mail.wrapping_add(1).max(1);
        self.inbox.push(MailMsg { id: self.next_mail, from: from.into(), subject, body, action, arg });
        if self.inbox.len() > 12 {
            self.inbox.remove(0);
        }
    }
}

const RECRUITER: &str = "Startup Sim — Rekrutacja";

/// An item lying on the floor; `handle` is its entity id in snapshots.
struct Dropped {
    handle: u16,
    item: Item,
    floor: u8,
    pos: Pos,
}

/// Entity ids of items on the floor (players below, NPCs from 0xF000).
const DROP_HANDLE_BASE: u16 = 0xE000;
/// Reach for picking up / handing over items.
const PICKUP_RADIUS: i32 = sim::TILE_UNITS * 5 / 4;
const GIVE_RADIUS: i32 = sim::TILE_UNITS * 2;
/// Resend the inventory this often (ticks).
const INVENTORY_RESEND_TICKS: u32 = 40;

/// A conversation with a board member.
#[derive(Debug, Clone, Copy)]
struct Talk {
    meeting: usize,
    npc: u16,
    id: u8,
    good: u32,
}

/// Resend the game time this often (ticks).
const CLOCK_RESEND_TICKS: u32 = 20;

/// Send the character's needs this often (ticks).
const STATS_EVERY_TICKS: u32 = 10;
/// Fruit in the bowl (label of the item).
const FRUITS: [&str; 4] = ["Jabłko", "Banan", "Gruszka", "Mandarynka"];

/// Resend the computer screen state this often (ticks).
const COMPUTER_RESEND_TICKS: u32 = 20;

/// Resend the current portal screen this often (ticks) - UDP may drop it.
const PORTAL_RESEND_TICKS: u32 = 20;

struct Player {
    id: u16,
    /// Character from the creation screen (age, city, e-mail stay here).
    profile: Profile,
    stage: Stage,
    /// Department of the position the player was recruited for (0 = none).
    department: u8,
    /// Contract signed at HR: the department is official (shown to others).
    contract: bool,
    /// Recruitment attempts so far (numbers the attempts).
    attempts: u8,
    /// Coffee being brewed.
    cup: Cup,
    /// Pockets and hands.
    inventory: Inventory,
    /// Inventory changed: send it to the owner this tick.
    inv_dirty: bool,
    /// Handle of the computer whose screen the player is looking at.
    at_computer: Option<u16>,
    /// Hunger, energy, stress, bladder.
    needs: Needs,
    /// Wallet, grosze.
    money: i64,
    /// Personal day number (1 = looking for a job).
    day: u32,
    /// Game deciseconds worked today (salary at 22:00).
    worked_ds: u64,
    /// Last payday: amount (grosze) and game minutes worked.
    last_pay: (i64, u32),
    /// How they commute (`commute::mode`); the last choice is kept.
    commute_mode: u8,
    /// Morning departure (game minute, `Clock::total_minutes`), until they leave.
    depart_at: Option<u32>,
    /// Riding a vehicle to work (its handle): hidden, no input.
    riding: Option<u16>,
    /// Already told "it's pouring" (until back indoors).
    soaked_said: bool,
    /// Salary, grosze per game hour (raises from the CEO).
    pay_rate: i64,
    /// World day of the last raise request (cooldown).
    last_raise_day: Option<u32>,
    /// Talking to a board member: meeting index, NPC, dialog id, good answers.
    talk: Option<Talk>,
    next_dialog_id: u8,
    /// Sofa / toilet / smoke break, and where it started (moving ends it).
    rest: Option<(Rest, u8, Pos)>,
    /// Messenger spam guard / retry dedupe.
    last_chat_tick: Option<u32>,
    last_chat_nonce: u32,
    token: u32,
    nonce: u32,
    addr: SocketAddr,
    nick: String,
    body: Body,
    room: u16,
    flags: u8,
    last_heard: Instant,
    inputs: VecDeque<(u32, u8)>,
    last_received_seq: u32,
    last_processed_seq: u32,
    /// Ids whose PlayerInfo this client has been sent.
    known: HashSet<u16>,
    bytes_out: u64,
}

#[derive(Default)]
struct Stats {
    ticks: u64,
    tick_total: Duration,
    tick_max: Duration,
    missed: u64,
    snapshot_bytes: u64,
    max_visible: usize,
}

pub struct Server {
    building: Building,
    npcs: Vec<Npc>,
    machines: Vec<Machine>,
    /// Items lying on the floor.
    dropped: Vec<Dropped>,
    /// Sofas, toilets, ashtrays, the fruit bowl.
    spots: Vec<Spot>,
    /// Toilet stalls and who locked them.
    stalls: Vec<Stall>,
    elevators: Vec<Elevator>,
    shelves: Vec<Shelf>,
    /// Room id of the shop per floor (leaving it with unpaid goods beeps).
    shop_rooms: Vec<(u8, u16)>,
    /// The cashier NPC (says the alarm line).
    cashier: Option<u16>,
    /// Some elevator was moving last tick (resend `Doors` when it starts/stops).
    lift_was_moving: bool,
    clock: Clock,
    /// Vehicles bringing people to work (and parked cars / bikes).
    vehicles: Vec<Vehicle>,
    weather: Weather,
    /// Board meetings (calendar).
    meetings: Vec<Meeting>,
    /// (floor, room) of the board room.
    board_room: Option<(u8, u16)>,
    /// (floor, room) under the open sky.
    outdoor_rooms: Vec<(u8, u16)>,
    /// Send `Clock` to everyone this tick (a day started / ended, someone arrived).
    clock_dirty: bool,
    /// A stall door changed: send `Doors` to everyone this tick.
    doors_dirty: bool,
    /// Desks where a laptop can stand, and the laptops standing on them.
    workstations: Vec<Workstation>,
    computers: Vec<Computer>,
    messenger: Messenger,
    next_item_id: u32,
    next_drop_handle: u16,
    /// Lines players "say" to themselves outside the tick (item actions).
    pending_says: Vec<(u16, String, Option<u16>)>,
    net: Net,
    cfg: Config,
    players: BTreeMap<u16, Player>,
    /// Session lookup: the token identifies the player, not the address, so a
    /// client survives a network change (Wi-Fi <-> LTE, new NAT port).
    by_token: HashMap<u32, u16>,
    /// Only used to dedupe `Connect` retries from the same address.
    by_addr: HashMap<SocketAddr, u16>,
    tick: u32,
    next_id: u16,
    next_spawn: usize,
    rng: fastrand::Rng,
    stats: Stats,
    started: Instant,
}

impl Server {
    pub fn new(building: Building, cfg: Config) -> std::io::Result<Server> {
        let net = Net::bind(cfg.bind, cfg.link)?;
        let board_room =
            building.active_floors().find_map(|(f, m)| m.rooms.iter().find(|r| r.kind == "management").map(|r| (f, r.id)));
        let mut server = Server {
            npcs: Npc::spawn_all(&building),
            machines: coffee::find_machines(&building),
            dropped: Vec::new(),
            workstations: computer::find_workstations(&building),
            spots: needs::find_spots(&building),
            stalls: stalls::find_stalls(&building),
            elevators: elevator::find_elevators(&building),
            shelves: shop::shelves(),
            shop_rooms: building
                .active_floors()
                .flat_map(|(f, m)| m.rooms.iter().filter(|r| r.kind == "shop").map(move |r| (f, r.id)))
                .collect(),
            cashier: None,
            lift_was_moving: false,
            clock: Clock::new(cfg.start_minute, cfg.time_scale),
            vehicles: Vec::new(),
            weather: Weather::new(0),
            meetings: Vec::new(),
            board_room,
            outdoor_rooms: building
                .active_floors()
                .flat_map(|(f, m)| m.rooms.iter().filter(|r| r.outdoor).map(move |r| (f, r.id)))
                .collect(),
            clock_dirty: true,
            doors_dirty: false,
            computers: Vec::new(),
            messenger: Messenger::default(),
            next_item_id: 1,
            next_drop_handle: DROP_HANDLE_BASE,
            pending_says: Vec::new(),
            building,
            net,
            cfg,
            players: BTreeMap::new(),
            by_token: HashMap::new(),
            by_addr: HashMap::new(),
            tick: 0,
            next_id: 1,
            next_spawn: 0,
            rng: fastrand::Rng::new(),
            stats: Stats::default(),
            started: Instant::now(),
        };
        server.sync_elevator_doors(); // doors start closed
        server.weather = match server.cfg.weather {
            Some(k) => Weather::fixed(k),
            None => Weather::new(server.clock.total_minutes()),
        };
        server.cashier = server.npcs.iter().find(|n| n.role == npc::Role::Cashier).map(|n| n.id);
        Ok(server)
    }

    /// Elevator doors: closed (solid) unless the car stands there open.
    fn sync_elevator_doors(&mut self) {
        let tick = self.tick;
        let mut set = Vec::new();
        for e in &self.elevators {
            for (f, t) in &e.doors {
                set.push((*f, *t, !e.is_open_at(*f, tick)));
            }
        }
        for (f, t, closed) in set {
            if let Some(m) = self.building.floor_mut(f) {
                m.set_closed(t.x, t.y, closed);
            }
        }
    }

    /// E at the elevator: the button in the cabin, or the call button at
    /// the doors. `None` = no elevator here.
    fn use_elevator(&mut self, body: &Body) -> Option<String> {
        let tick = self.tick;
        for i in 0..self.elevators.len() {
            let e = &self.elevators[i];
            if e.in_cabin(body.floor, body.pos) {
                let e = &mut self.elevators[i];
                self.doors_dirty = true;
                return Some(match e.press_inside(&self.building, body.floor, tick) {
                    Some(t) => format!("Jedziemy na: {}.", self.building.floor_name(t)),
                    None => elevator::lines::RIDING.to_string(),
                });
            }
            if e.door_in_reach(body) {
                self.doors_dirty = true; // show where the car is heading at once
                return Some(self.elevators[i].call(body.floor, tick).to_string());
            }
        }
        None
    }

    /// Move the elevators; carry the people in a cabin that arrived.
    fn tick_elevators(&mut self) {
        let people: Vec<(u8, Pos)> = self.players.values().map(|p| (p.body.floor, p.body.pos)).collect();
        let tick = self.tick;
        let mut changed = false;
        for i in 0..self.elevators.len() {
            let up = self.elevators[i].tick(tick, &people);
            changed |= up.doors_changed;
            if let Some(floor) = up.overloaded {
                // Somebody in the cabin says it (everyone inside hears it).
                let e = &self.elevators[i];
                let speaker = self.players.values().find(|p| p.body.floor == floor && e.in_cabin(floor, p.body.pos)).map(|p| p.id);
                if let Some(s) = speaker {
                    self.pending_says.push((s, elevator::lines::OVERLOAD.to_string(), None));
                }
            }
            if let Some((from, to)) = up.arrived {
                let e = &self.elevators[i];
                for p in self.players.values_mut() {
                    if matches!(p.stage, Stage::Working) && e.in_cabin(from, p.body.pos) && p.body.floor == from {
                        p.body.floor = to;
                        p.room = self.building.floor(to).map_or(0, |m| m.room_at(p.body.pos.x, p.body.pos.y));
                    }
                }
            }
        }
        if changed || self.elevators.iter().any(|e| e.moving.is_some()) != self.lift_was_moving {
            self.lift_was_moving = self.elevators.iter().any(|e| e.moving.is_some());
            self.sync_elevator_doors();
            self.doors_dirty = true;
        }
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.net.local_addr().expect("bound socket")
    }

    fn log(&self, msg: impl AsRef<str>) {
        println!("[{:>8.2}s] {}", self.started.elapsed().as_secs_f64(), msg.as_ref());
    }

    /// Run forever.
    pub fn run(&mut self) {
        let mut next_tick = Instant::now() + TICK;
        let mut next_stats = Instant::now() + self.cfg.stats_every;
        loop {
            let now = Instant::now();
            if now >= next_tick {
                let t0 = Instant::now();
                self.tick();
                let dt = t0.elapsed();
                self.stats.ticks += 1;
                self.stats.tick_total += dt;
                self.stats.tick_max = self.stats.tick_max.max(dt);
                next_tick += TICK;
                // Fell behind by more than a whole tick: skip, don't spiral.
                let late = Instant::now().saturating_duration_since(next_tick);
                if late >= TICK {
                    let skipped = (late.as_nanos() / TICK.as_nanos()) as u32;
                    self.stats.missed += skipped as u64;
                    self.tick = self.tick.wrapping_add(skipped);
                    next_tick += TICK * skipped;
                }
            }
            if now >= next_stats {
                self.print_stats();
                next_stats += self.cfg.stats_every;
            }
            let now = Instant::now();
            self.net.flush(now);
            while let Some((addr, data)) = self.net.pop_inbound(now) {
                self.handle_datagram(addr, &data, now);
            }
            let mut deadline = next_tick.min(next_stats);
            if let Some(r) = self.net.next_release() {
                deadline = deadline.min(r);
            }
            self.net.recv(deadline.saturating_duration_since(Instant::now()));
        }
    }

    fn send(&mut self, addr: SocketAddr, p: &Packet) -> usize {
        let b = p.encode();
        debug_assert!(b.len() <= proto::MAX_PACKET);
        let n = b.len();
        self.net.send(addr, b);
        n
    }

    fn handle_datagram(&mut self, addr: SocketAddr, data: &[u8], now: Instant) {
        let Ok(packet) = Packet::decode(data) else {
            if let Some(proto::DecodeError::BadVersion(_)) = Packet::decode(data).err() {
                self.send(addr, &Packet::Reject { reason: proto::reject::BAD_VERSION });
            }
            return;
        };
        if let Packet::Connect { nonce, nick, profile } = packet {
            self.handle_connect(addr, nonce, nick, profile, now);
            return;
        }
        // Every other packet is identified by its session token.
        let token = match &packet {
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
            | Packet::DialogAnswer { token, .. } => *token,
            _ => return,
        };
        let Some(&id) = self.by_token.get(&token) else {
            // Unknown/expired session: tell the client so it can reconnect.
            if matches!(packet, Packet::Input { .. } | Packet::Ping { .. }) {
                self.send(addr, &Packet::Disconnect { token, reason: proto::disconnect::SESSION_UNKNOWN });
            }
            return;
        };
        // Address migration: only packets that prove liveness *now* (a ping or
        // new inputs) may move the session, so a late reordered packet from the
        // old address cannot pull it back.
        let fresh = match &packet {
            Packet::Ping { .. } => true,
            Packet::Input { last_seq, .. } => *last_seq > self.players[&id].last_received_seq,
            _ => false,
        };
        if fresh && self.players[&id].addr != addr {
            self.migrate(id, addr);
        }
        self.players.get_mut(&id).unwrap().last_heard = now;
        let addr = self.players[&id].addr;
        match packet {
            Packet::Input { last_seq, inputs, .. } => {
                let p = self.players.get_mut(&id).unwrap();
                let n = inputs.len() as u32;
                for (i, &bits) in inputs.iter().enumerate() {
                    let seq = last_seq.wrapping_sub(n - 1 - i as u32);
                    if seq > p.last_received_seq {
                        p.inputs.push_back((seq, bits));
                        p.last_received_seq = seq;
                    }
                }
                while p.inputs.len() > MAX_INPUT_QUEUE {
                    let (seq, _) = p.inputs.pop_front().unwrap();
                    p.last_processed_seq = seq;
                }
            }
            Packet::InfoRequest { ids, .. } => {
                let entries: Vec<PlayerInfoEntry> = ids.iter().filter_map(|&i| self.info_of(i)).collect();
                let p = self.players.get_mut(&id).unwrap();
                p.known.extend(entries.iter().map(|e| e.id));
                for chunk in entries.chunks(INFO_PER_PACKET) {
                    self.send(addr, &Packet::PlayerInfo { players: chunk.to_vec() });
                }
            }
            Packet::Ping { client_time, .. } => {
                let tick = self.tick;
                self.send(addr, &Packet::Pong { client_time, server_tick: tick });
            }
            Packet::Disconnect { .. } => {
                self.remove_player(id, "left");
            }
            Packet::Apply { offer, .. } => self.handle_apply(id, offer),
            Packet::PortalAction { action, arg, .. } => self.handle_portal_action(id, action, arg),
            Packet::ItemAction { action, slot, .. } => self.handle_item_action(id, action, slot),
            Packet::ComputerAction { action, conv, arg, text, .. } => self.handle_computer_action(id, action, conv, arg, &text),
            Packet::DoorAction { .. } => self.handle_door_action(id),
            Packet::ShopTake { shelf, kind, .. } => self.handle_shop_take(id, shelf, kind),
            Packet::CalendarBook { start, topic, .. } => self.handle_calendar_book(id, start as u32, topic),
            Packet::DialogAnswer { id: dialog, choice, .. } => self.handle_dialog_answer(id, dialog, choice),
            Packet::CommuteChoice { mode, .. } => {
                let p = self.players.get_mut(&id).unwrap();
                if commute::mode(mode).is_some() && matches!(p.stage, Stage::Home { arrive_at: None }) {
                    p.commute_mode = mode;
                    self.clock_dirty = true;
                }
            }
            Packet::Answer { attempt, index, choice, .. } => self.handle_answer(id, attempt, index, choice),
            _ => {}
        }
    }

    fn handle_connect(&mut self, addr: SocketAddr, nonce: u32, nick: String, profile: Profile, now: Instant) {
        if let Some(&id) = self.by_addr.get(&addr) {
            if self.players[&id].nonce == nonce {
                // Our Welcome was lost; resend it.
                let welcome = self.welcome(id);
                self.send(addr, &welcome);
                return;
            }
            self.remove_player(id, "reconnected");
        }
        let nick: String = nick.chars().filter(|c| !c.is_control()).collect::<String>().trim().to_string();
        if nick.is_empty() {
            self.send(addr, &Packet::Reject { reason: proto::reject::BAD_NICK });
            return;
        }
        let Some(profile) = validate_profile(profile) else {
            self.send(addr, &Packet::Reject { reason: proto::reject::BAD_PROFILE });
            return;
        };
        if self.players.len() >= self.cfg.max_players {
            self.send(addr, &Packet::Reject { reason: proto::reject::SERVER_FULL });
            return;
        }
        let id = self.alloc_id();
        let token = loop {
            let t = self.rng.u32(1..);
            if !self.by_token.contains_key(&t) {
                break t;
            }
        };
        let spawns = self.building.spawns();
        let (spawn_floor, spawn) = spawns[self.next_spawn % spawns.len()];
        self.next_spawn += 1;
        let pos = Pos::tile_center(spawn.x, spawn.y);
        let skip = self.cfg.skip_recruitment;
        let player = Player {
            id,
            profile,
            stage: if skip { Stage::Working } else { Stage::Portal(Box::default()) },
            department: 0,
            contract: false,
            attempts: 0,
            cup: Cup::None,
            inventory: Inventory::default(),
            inv_dirty: true,
            at_computer: None,
            needs: Needs::default(),
            money: 0,
            day: 1,
            worked_ds: 0,
            last_pay: (0, 0),
            commute_mode: commute::mode::TRAM,
            depart_at: None,
            riding: None,
            soaked_said: false,
            pay_rate: clock::PAY_PER_MIN * 60,
            last_raise_day: None,
            talk: None,
            next_dialog_id: 0,
            rest: None,
            last_chat_tick: None,
            last_chat_nonce: 0,
            token,
            nonce,
            addr,
            nick,
            body: Body::at(spawn_floor, pos),
            room: self.room_of(spawn_floor, pos),
            flags: 0,
            last_heard: now,
            inputs: VecDeque::new(),
            last_received_seq: 0,
            last_processed_seq: 0,
            known: HashSet::new(),
            bytes_out: 0,
        };
        self.log(format!("+ player {} '{}' from {} ({} online)", id, player.nick, canonical(addr), self.players.len() + 1));
        self.players.insert(id, player);
        self.by_addr.insert(addr, id);
        if self.cfg.start_employed {
            self.employ(id);
            if let Some(p) = self.players.get_mut(&id) {
                p.day = 2;
                if self.clock.is_night() {
                    p.stage = Stage::Home { arrive_at: None };
                }
            }
        } else if skip && self.cfg.start_access & crate::map::access::CARD != 0 {
            self.give_new(id, item_kind::EMPLOYEE_CARD); // load tests: straight in with a card
        }
        self.by_token.insert(token, id);
        let welcome = self.welcome(id);
        self.send(addr, &welcome);
        if !skip {
            self.send_portal(id, true);
        }
    }

    /// (Re)send the desktop state: portal offers, current interview
    /// question and (with `mails`) the whole inbox. Clients dedupe.
    fn send_portal(&mut self, id: u16, mails: bool) {
        let Some(p) = self.players.get(&id) else { return };
        let Stage::Portal(desk) = &p.stage else { return };
        let r = &self.cfg.recruitment;
        // Offers may not fit one datagram: split them (the client merges by id).
        let mut packets = Vec::new();
        let mut chunk = Vec::new();
        let mut size = proto::HEADER_LEN + 1;
        for offer in r.portal(|o| desk.applied.contains(&o)) {
            let len = 3 + 6 + offer.company.len() + offer.title.len() + offer.description.len();
            if size + len > proto::MAX_PACKET && !chunk.is_empty() {
                packets.push(Packet::JobOffers { offers: std::mem::take(&mut chunk) });
                size = proto::HEADER_LEN + 1;
            }
            size += len;
            chunk.push(offer);
        }
        packets.push(Packet::JobOffers { offers: chunk });
        if let Some(q) = desk.attempt.as_ref().and_then(|a| a.current(r).map(|q| (a.number, q))) {
            let (attempt, q) = q;
            packets.push(Packet::Question { attempt, index: q.index, total: q.total, text: q.text, options: q.options });
        }
        if mails {
            for m in &desk.inbox {
                packets.push(Packet::Mail {
                    id: m.id,
                    from: m.from.clone(),
                    subject: m.subject.clone(),
                    body: m.body.clone(),
                    action: m.action,
                    arg: m.arg,
                });
            }
        }
        let addr = p.addr;
        for packet in packets {
            self.send(addr, &packet);
        }
    }

    fn desk(&mut self, id: u16) -> Option<&mut Desk> {
        match &mut self.players.get_mut(&id)?.stage {
            Stage::Portal(d) => Some(d),
            Stage::Working | Stage::Home { .. } => None,
        }
    }

    fn handle_apply(&mut self, id: u16, offer: u8) {
        let due = self.tick + self.cfg.recruitment.invite_delay_secs * TICK_HZ;
        let Some(o) = self.cfg.recruitment.offer(offer).map(|o| (o.hiring, o.reply.is_some())) else { return };
        let Some(desk) = self.desk(id) else { return };
        if desk.applied.contains(&offer) || desk.hired.is_some() {
            return; // duplicate (resent) application
        }
        desk.applied.push(offer);
        if o.0 || o.1 {
            desk.pending.push((offer, due)); // other companies without a reply: silence
        }
        self.send_portal(id, false);
    }

    /// Replies that are due: interview invitations / other companies' answers.
    fn deliver_replies(&mut self, id: u16) {
        let tick = self.tick;
        let Some(p) = self.players.get_mut(&id) else { return };
        let nick = p.nick.clone();
        let Stage::Portal(desk) = &mut p.stage else { return };
        let due: Vec<u8> = desk.pending.iter().filter(|(_, t)| *t <= tick).map(|(o, _)| *o).collect();
        if due.is_empty() {
            return;
        }
        desk.pending.retain(|(_, t)| *t > tick);
        for offer in due {
            let Some(o) = self.cfg.recruitment.offer(offer) else { continue };
            if o.hiring {
                desk.invited.push(offer);
                desk.mail(
                    RECRUITER,
                    format!("Zaproszenie na rozmowę: {}", o.title),
                    format!(
                        "Cześć {nick}!\n\nDziękujemy za zgłoszenie na stanowisko {}. Zapraszamy na krótką rozmowę online — \
                         kilka pytań, zero stresu (prawie). Kliknij „Dołącz do rozmowy”, kiedy tylko możesz.\n\nZespół Startup Sim",
                        o.title
                    ),
                    proto::portal_action::JOIN_INTERVIEW,
                    offer,
                );
            } else if let Some(reply) = &o.reply {
                desk.mail(&o.company, format!("Re: {}", o.title), reply.clone(), proto::portal_action::NONE, 0);
            }
        }
        self.send_portal(id, true);
    }

    fn handle_portal_action(&mut self, id: u16, action: u8, arg: u8) {
        match action {
            proto::portal_action::JOIN_INTERVIEW => {
                let p = self.players.get_mut(&id).unwrap();
                let Stage::Portal(desk) = &mut p.stage else { return };
                if !desk.invited.contains(&arg) || desk.attempt.is_some() || desk.hired.is_some() {
                    return;
                }
                p.attempts = p.attempts.wrapping_add(1);
                desk.attempt = self.cfg.recruitment.start(arg, p.attempts, &mut self.rng);
                self.send_portal(id, false);
            }
            proto::portal_action::GO_TO_OFFICE => {
                let p = self.players.get_mut(&id).unwrap();
                let Stage::Portal(desk) = &p.stage else { return };
                let Some(dept) = desk.hired else { return };
                // Hired: a new day - the first one at work. At night you come
                // in the morning (random arrival, like everybody).
                p.day += 1;
                p.stage = if self.clock.is_night() { Stage::Home { arrive_at: None } } else { Stage::Working };
                p.department = dept;
                self.clock_dirty = true;
                let msg = format!(
                    "* player {id} '{}' goes to the office: {}",
                    p.nick,
                    self.cfg.recruitment.department_name(dept).unwrap_or("?")
                );
                self.log(msg);
                if self.cfg.start_access & crate::map::access::CARD != 0 {
                    self.give_new(id, item_kind::EMPLOYEE_CARD); // load tests: straight in with a card
                }
            }
            _ => {}
        }
    }

    fn handle_answer(&mut self, id: u16, attempt_no: u8, index: u8, choice: u8) {
        let p = self.players.get_mut(&id).unwrap();
        let nick = p.nick.clone();
        let addr = p.addr;
        let Stage::Portal(desk) = &mut p.stage else { return };
        let Some(a) = desk.attempt.as_mut() else { return };
        if a.number != attempt_no || !a.answer(index, choice) {
            return; // stale / duplicate: the resend loop shows the current state
        }
        if !a.finished() {
            self.send_portal(id, false);
            return;
        }
        let r = &self.cfg.recruitment;
        let (score, total, offer) = (a.score(), a.total(), a.offer);
        let passed = score >= r.pass_score;
        let o = r.offer(offer).expect("attempt for a known offer");
        let result = Packet::RecruitResult {
            attempt: attempt_no,
            passed,
            score: score as u8,
            total: total as u8,
            department: if passed { o.department } else { 0 },
        };
        desk.attempt = None;
        desk.invited.retain(|x| *x != offer);
        if passed {
            desk.hired = Some(o.department);
            desk.mail(
                RECRUITER,
                "Zaproszenie na dzień próbny".into(),
                format!(
                    "Gratulacje, {nick}!\n\nRozmowa na stanowisko {} poszła świetnie ({score}/{total}). Zapraszamy na dzień \
                     próbny do biura: zgłoś się na portierni — portier zaprowadzi Cię na recepcję, a w HR podpiszesz umowę \
                     i odbierzesz kartę.\n\nDo zobaczenia!",
                    o.title
                ),
                proto::portal_action::GO_TO_OFFICE,
                0,
            );
        } else {
            desk.applied.retain(|x| *x != offer); // may apply again
            desk.mail(
                RECRUITER,
                format!("Dziękujemy za rozmowę: {}", o.title),
                format!(
                    "Cześć {nick},\n\ndziękujemy za rozmowę ({score}/{total}). Tym razem szukamy kogoś innego, ale nie \
                     przejmuj się — zapraszamy do ponownej aplikacji. Pytania będą inne!\n\nZespół Startup Sim"
                ),
                proto::portal_action::NONE,
                0,
            );
        }
        self.send(addr, &result);
        self.send(addr, &result); // tiny packet; a duplicate makes loss unlikely
        self.send_portal(id, true);
    }

    /// Name (and official department) of a player or NPC.
    fn info_of(&self, id: u16) -> Option<PlayerInfoEntry> {
        match self.players.get(&id) {
            Some(p) => Some(PlayerInfoEntry {
                id,
                nick: p.nick.clone(),
                department: if p.contract { p.department } else { 0 },
                gender: p.profile.gender,
                appearance: p.profile.appearance,
            }),
            // A computer is introduced by its owner's name.
            None if self.computers.iter().any(|c| c.handle == id) => {
                let owner = self.computers.iter().find(|c| c.handle == id)?.owner();
                self.info_of(owner).map(|e| PlayerInfoEntry { id, ..e })
            }
            None => self.npcs.iter().find(|n| n.id == id).map(|n| PlayerInfoEntry {
                id,
                nick: n.name.clone(),
                department: 0,
                gender: proto::gender::OTHER,
                appearance: proto::Appearance::default(),
            }),
        }
    }

    fn room_of(&self, floor: u8, pos: Pos) -> u16 {
        self.building.floor(floor).map_or(0, |m| m.room_at(pos.x, pos.y))
    }

    fn welcome(&self, id: u16) -> Packet {
        let p = &self.players[&id];
        Packet::Welcome {
            nonce: p.nonce,
            player_id: id,
            token: p.token,
            tick_hz: TICK_HZ as u8,
            input_hz: sim::INPUT_HZ as u8,
            map_crc: self.building.crc,
            server_tick: self.tick,
        }
    }

    fn alloc_id(&mut self) -> u16 {
        loop {
            let id = self.next_id;
            // Player ids: 1 .. NPC_ID_BASE-1 (NPCs use the ids above).
            self.next_id = if self.next_id + 1 >= npc::NPC_ID_BASE { 1 } else { self.next_id + 1 };
            if !self.players.contains_key(&id) {
                return id;
            }
        }
    }

    fn migrate(&mut self, id: u16, addr: SocketAddr) {
        let p = self.players.get_mut(&id).unwrap();
        let old = std::mem::replace(&mut p.addr, addr);
        if self.by_addr.get(&old) == Some(&id) {
            self.by_addr.remove(&old);
        }
        self.by_addr.insert(addr, id);
        self.log(format!("~ player {id} moved {} -> {}", canonical(old), canonical(addr)));
    }

    // ------------------------------------------------------------- items

    fn label_for(&self, pid: u16, k: u8) -> String {
        let Some(p) = self.players.get(&pid) else { return String::new() };
        let dept = self.cfg.recruitment.department_name(p.department).unwrap_or("");
        match k {
            item_kind::GUEST_PASS => format!("Dzień próbny: {}", p.nick),
            item_kind::EMPLOYEE_CARD if !dept.is_empty() => format!("{} · {dept}", p.nick),
            item_kind::EMPLOYEE_CARD => p.nick.clone(),
            item_kind::LAPTOP => format!("Laptop: {}", p.nick),
            item_kind::COFFEE => "Gorąca, z ekspresu".into(),
            _ => String::new(),
        }
    }

    /// Create a new item for `pid` (labelled for them) and hand it over.
    fn give_new(&mut self, pid: u16, k: u8) {
        let label = self.label_for(pid, k);
        let expires = (k == item_kind::COFFEE).then_some(self.tick + coffee::DRINK_TICKS);
        let owner = if k == item_kind::COFFEE { 0 } else { pid };
        let item = Item { id: self.next_item_id, kind: k, label, expires, owner, count: 1, unpaid: false };
        self.next_item_id += 1;
        self.give(pid, item);
    }

    /// Put an item into a player's inventory; if it doesn't fit, it lands on
    /// the floor at their feet.
    fn give(&mut self, pid: u16, item: Item) {
        let Some(p) = self.players.get_mut(&pid) else { return };
        match p.inventory.add(item) {
            Ok(()) => refresh(p),
            Err(item) => {
                let (floor, pos) = (p.body.floor, p.body.pos);
                self.drop_at(floor, pos, item);
            }
        }
    }

    fn drop_at(&mut self, floor: u8, pos: Pos, item: Item) {
        let handle = self.alloc_handle();
        self.dropped.push(Dropped { handle, item, floor, pos });
    }

    /// Entity id for an item on the floor or a computer on a desk.
    fn alloc_handle(&mut self) -> u16 {
        let next = |h: u16| if h + 1 >= npc::NPC_ID_BASE { DROP_HANDLE_BASE } else { h + 1 };
        let mut handle = self.next_drop_handle;
        while self.dropped.iter().any(|d| d.handle == handle)
            || self.computers.iter().any(|c| c.handle == handle)
            || self.vehicles.iter().any(|v| v.handle == handle)
        {
            handle = next(handle);
        }
        self.next_drop_handle = next(handle);
        handle
    }

    fn handle_item_action(&mut self, id: u16, action: u8, slot: u8) {
        let Some(p) = self.players.get_mut(&id) else { return };
        if !matches!(p.stage, Stage::Working) {
            return;
        }
        let say = |line: &str| (id, line.to_string(), None);
        match action {
            proto::item_action::TAKE_OUT => match p.inventory.take_out(slot as usize) {
                Ok(()) => refresh(p),
                Err(r) => self.pending_says.push(say(r.line())),
            },
            proto::item_action::PUT_AWAY => match p.inventory.put_away() {
                Ok(()) => refresh(p),
                Err(r) => self.pending_says.push(say(r.line())),
            },
            proto::item_action::DROP => {
                if let Some(item) = p.inventory.take_hands() {
                    refresh(p);
                    let (floor, pos) = (p.body.floor, p.body.pos);
                    self.drop_at(floor, pos, item);
                }
            }
            proto::item_action::GIVE => {
                let (floor, pos) = (p.body.floor, p.body.pos);
                if p.inventory.hands_free() {
                    return;
                }
                let target = self
                    .players
                    .values()
                    .filter(|o| o.id != id && matches!(o.stage, Stage::Working) && o.body.floor == floor)
                    .map(|o| (o.id, dist2(o.body.pos, pos)))
                    .filter(|&(_, d)| d <= GIVE_RADIUS * GIVE_RADIUS)
                    .min_by_key(|&(_, d)| d)
                    .map(|(t, _)| t);
                let Some(target) = target else {
                    self.pending_says.push(say("Nie ma nikogo obok."));
                    return;
                };
                let item = self.players.get_mut(&id).unwrap().inventory.take_hands().unwrap();
                let name = inventory::display_name(item.kind);
                let to = self.players.get_mut(&target).unwrap();
                let to_nick = to.nick.clone();
                match to.inventory.add(item) {
                    Ok(()) => {
                        refresh(to);
                        refresh(self.players.get_mut(&id).unwrap());
                        let from_nick = self.players[&id].nick.clone();
                        self.pending_says.push((id, format!("Proszę, {to_nick} — {}.", name.to_lowercase()), Some(target)));
                        self.log(format!("* item: {from_nick} gave {name} to {to_nick}"));
                    }
                    Err(item) => {
                        self.players.get_mut(&id).unwrap().inventory.hands = Some(item); // give it back
                        self.pending_says.push(say(&format!("{to_nick} nie ma już wolnych rąk ani kieszeni.")));
                    }
                }
            }
            proto::item_action::USE => {
                let Some(held) = &p.inventory.hands else { return };
                let line = match held.kind {
                    item_kind::COFFEE => {
                        p.inventory.take_hands();
                        p.needs.drink_coffee();
                        refresh(p);
                        coffee::lines::DRUNK.to_string()
                    }
                    item_kind::FRUIT if p.needs.is_full() => needs::lines::NOT_HUNGRY.into(),
                    item_kind::FRUIT => {
                        let what = held.label.to_lowercase();
                        p.inventory.take_hands();
                        let yuck = p.needs.eat_fruit();
                        refresh(p);
                        if yuck {
                            format!("{} ({what})", needs::lines::YUCK)
                        } else {
                            format!("Mniam, {what}.")
                        }
                    }
                    item_kind::EMPLOYEE_CARD => format!("Karta pracownika: {}.", held.label),
                    item_kind::GUEST_PASS => "Przepustka gościa — ważna do końca dnia.".into(),
                    _ if held.unpaid => shop::lines::PAY_FIRST.into(),
                    item_kind::CIGARETTES => format!("Zostało {} papierosów. Palić tylko w strefie palenia.", held.count),
                    k if shop::product(k).is_some() => {
                        let prod = shop::product(k).unwrap();
                        p.inventory.take_hands();
                        p.needs.apply(prod.effect);
                        refresh(p);
                        prod.line.to_string()
                    }
                    item_kind::LAPTOP => format!("{} — położę go na wolnym biurku w swoim dziale (E).", held.label),
                    _ => return,
                };
                self.pending_says.push(say(&line));
            }
            _ => {}
        }
    }

    /// Pick up the nearest item on the floor within reach, if any.
    fn try_pickup(&mut self, pid: u16, body: &Body) -> Option<String> {
        let i = self
            .dropped
            .iter()
            .enumerate()
            .filter(|(_, d)| d.floor == body.floor && dist2(d.pos, body.pos) <= PICKUP_RADIUS * PICKUP_RADIUS)
            .min_by_key(|(_, d)| dist2(d.pos, body.pos))
            .map(|(i, _)| i)?;
        let p = self.players.get_mut(&pid)?;
        let d = self.dropped.remove(i);
        let name = inventory::display_name(d.item.kind);
        match p.inventory.add(d.item) {
            Ok(()) => {
                refresh(p);
                Some(format!("Podniesione: {}.", name.to_lowercase()))
            }
            Err(item) => {
                self.dropped.insert(i, Dropped { item, ..d });
                Some(inventory::Refusal::HandsFull.line().to_string())
            }
        }
    }

    // ------------------------------------------------------------- clock

    /// Game time: salary accrues, 22:00 sends everybody home (payday), 6:00
    /// starts a new day with random arrivals, arrivals come in.
    fn tick_clock(&mut self) {
        let rate = self.clock.rate() as u64;
        let transition = self.clock.tick();
        if self.weather.tick(self.clock.total_minutes(), &mut self.rng) {
            self.log(format!("* weather: {}", weather::name(self.weather.now)));
            self.clock_dirty = true;
        }
        if !self.clock.is_night() {
            for p in self.players.values_mut() {
                if p.contract && matches!(p.stage, Stage::Working) {
                    p.worked_ds += rate;
                }
            }
        }
        match transition {
            Some(Transition::Evening) => {
                let ids: Vec<u16> = self.players.values().filter(|p| matches!(p.stage, Stage::Working)).map(|p| p.id).collect();
                for pid in ids {
                    self.go_home(pid);
                }
                self.log(format!("* day {} ends: office closed", self.clock.day));
                self.clock_dirty = true;
            }
            Some(Transition::Morning) => {
                let now = self.clock.total_minutes();
                for p in self.players.values_mut() {
                    p.day += 1;
                    if matches!(p.stage, Stage::Home { .. }) {
                        // Leaves home at a random time; how they travel is
                        // chosen until then (the last choice by default).
                        p.depart_at = Some(now + self.rng.u32(commute::DEPART_FROM..=commute::DEPART_TO));
                    }
                }
                self.log(format!("* day {} starts", self.clock.day));
                self.clock_dirty = true;
            }
            None => {}
        }
        let now = self.clock.total_minutes();
        // Leaving home: pay the fare, the trip takes its time.
        let leaving: Vec<u16> = self
            .players
            .values()
            .filter(|p| matches!(p.stage, Stage::Home { arrive_at: None }) && p.depart_at.is_some_and(|t| now >= t))
            .map(|p| p.id)
            .collect();
        for pid in leaving {
            let traffic = self.rng.u32(0..=commute::MAX_TRAFFIC);
            let p = self.players.get_mut(&pid).unwrap();
            let mut m = commute::mode(p.commute_mode).copied().unwrap_or(commute::MODES[0]);
            if p.money < m.cost {
                m = commute::MODES[0]; // can't afford it: on foot
                p.commute_mode = m.id;
            }
            p.money -= m.cost;
            // Rain and storms make the jams worse.
            let weather_jam = match self.weather.now {
                weather::kind::RAIN => 10,
                weather::kind::STORM => 20,
                _ => 0,
            };
            let minutes = m.minutes + if m.id == commute::mode::CAR { traffic + weather_jam } else { 0 };
            p.stage = Stage::Home { arrive_at: Some(now + minutes) };
            self.clock_dirty = true;
        }
        let arriving: Vec<u16> = self
            .players
            .values()
            .filter(|p| matches!(p.stage, Stage::Home { arrive_at: Some(t) } if now >= t))
            .map(|p| p.id)
            .collect();
        for pid in arriving {
            self.arrive(pid);
        }
    }

    /// 22:00: out of the building; salary for the hours worked today.
    fn go_home(&mut self, pid: u16) {
        self.end_session(pid);
        self.vehicles.retain(|v| v.owner != pid); // the car / bike goes home too
        let Some(p) = self.players.get_mut(&pid) else { return };
        p.rest = None;
        p.riding = None;
        let minutes = (p.worked_ds / clock::DS_PER_MIN as u64) as u32;
        let pay = minutes as i64 * p.pay_rate / 60;
        p.money += pay;
        p.last_pay = (pay, minutes);
        p.worked_ds = 0;
        p.stage = Stage::Home { arrive_at: None };
        let msg = format!("* {} goes home: worked {} min, paid {}", p.nick, minutes, shop::zl(pay));
        self.log(msg);
    }

    /// Morning arrival: on foot along the sidewalk, or riding in a vehicle
    /// that drops them off (see `tick_vehicles`).
    fn arrive(&mut self, pid: u16) {
        let Some(mode) = self.players.get(&pid).map(|p| p.commute_mode) else { return };
        let handle = self.alloc_handle();
        let kind_of = |m: u8| Vehicle::for_mode(m, 0, 0, 0).map(|v| v.kind);
        let slot = self.vehicles.iter().filter(|v| v.parks && Some(v.kind) == kind_of(mode)).count();
        let vehicle = Vehicle::for_mode(mode, handle, pid, slot);
        let (pos, riding) = match &vehicle {
            Some(v) => (v.pos, Some(v.handle)),
            None => (Pos::tile_center(1, 35), None), // walking in from the west
        };
        if let Some(v) = vehicle {
            self.vehicles.push(v);
        }
        let room = self.room_of(0, pos);
        let Some(p) = self.players.get_mut(&pid) else { return };
        p.body = Body::at(0, pos);
        p.room = room;
        p.stage = Stage::Working;
        p.depart_at = None;
        p.riding = riding;
        refresh(p);
        self.clock_dirty = true;
        if riding.is_none() {
            self.trip_done(pid);
        }
    }

    /// Got out (or walked in): what the trip did, and whether it's late.
    fn trip_done(&mut self, pid: u16) {
        let late = self.clock.minute() > commute::LATE_AFTER;
        let Some(p) = self.players.get_mut(&pid) else { return };
        if let Some(m) = commute::mode(p.commute_mode) {
            let e = m.effect;
            p.needs.apply(shop::Effect { hunger: 0, energy: e.energy, stress: e.stress, bladder: 0 });
            p.needs.hygiene = (p.needs.hygiene + e.hygiene * needs::SCALE).clamp(0, needs::MAX);
        }
        // Rain on the way: soaked on foot or by bike (an umbrella helps on foot).
        if weather::wet(self.weather.now) {
            let umbrella = p.inventory.has(item_kind::UMBRELLA);
            let storm = self.weather.now == weather::kind::STORM;
            let soaked = match p.commute_mode {
                commute::mode::BIKE => true,
                commute::mode::WALK => !umbrella,
                _ => false,
            };
            if soaked {
                let h = if storm { 20 } else { 12 };
                p.needs.hygiene = (p.needs.hygiene - h * needs::SCALE).max(0);
                p.needs.add_stress(if storm { 8 } else { 5 });
                self.pending_says.push((pid, weather::lines::SOAKED_ON_THE_WAY.to_string(), None));
            } else if p.commute_mode == commute::mode::WALK {
                self.pending_says.push((pid, weather::lines::UMBRELLA.to_string(), None));
            }
        }
        if late {
            p.needs.add_stress(10);
            self.pending_says.push((pid, commute::lines::LATE.to_string(), None));
        }
    }

    /// Move the vehicles; riders go along and get out at the stop.
    fn tick_vehicles(&mut self) {
        let mut arrived = Vec::new();
        let mut gone = Vec::new();
        for v in &mut self.vehicles {
            match v.tick() {
                Some(VehicleEvent::Arrived { rider, alight }) => arrived.push((rider, alight)),
                Some(VehicleEvent::Gone) => gone.push(v.handle),
                None => {}
            }
        }
        self.vehicles.retain(|v| !gone.contains(&v.handle));
        // Riders sit in their vehicle.
        let positions: Vec<(u16, Pos)> = self.vehicles.iter().map(|v| (v.handle, v.pos)).collect();
        for p in self.players.values_mut() {
            if let Some(h) = p.riding {
                match positions.iter().find(|(vh, _)| *vh == h) {
                    Some((_, pos)) => p.body.pos = *pos,
                    None => p.riding = None,
                }
                p.room = self.building.floor(0).map_or(0, |m| m.room_at(p.body.pos.x, p.body.pos.y));
            }
        }
        for (rider, alight) in arrived {
            if let Some(p) = self.players.get_mut(&rider) {
                p.riding = None;
                p.body = Body { pos: alight, ..p.body };
                p.room = self.building.floor(0).map_or(0, |m| m.room_at(alight.x, alight.y));
                refresh(p);
            }
            self.trip_done(rider);
        }
    }

    fn clock_packet(&self, p: &Player) -> Packet {
        let (place, arrive) = match p.stage {
            Stage::Portal(_) => (proto::place::PORTAL, proto::NO_TIME),
            Stage::Working => (proto::place::BUILDING, proto::NO_TIME),
            Stage::Home { arrive_at: None } if p.depart_at.is_some() => (proto::place::COMMUTING, proto::NO_TIME),
            Stage::Home { arrive_at: None } => (proto::place::HOME, proto::NO_TIME),
            Stage::Home { arrive_at: Some(t) } => (proto::place::COMMUTING, (t % clock::MIN_PER_DAY) as u16),
        };
        Packet::Clock {
            day: p.day.min(u16::MAX as u32) as u16,
            minute: self.clock.minute() as u16,
            night: self.clock.is_night(),
            place,
            arrive,
            pay: p.last_pay.0.clamp(0, u32::MAX as i64) as u32,
            pay_minutes: p.last_pay.1.min(u16::MAX as u32) as u16,
            today_minutes: (p.worked_ds / clock::DS_PER_MIN as u64).min(u16::MAX as u64) as u16,
            mode: p.commute_mode,
            depart: p.depart_at.map_or(proto::NO_TIME, |t| (t % clock::MIN_PER_DAY) as u16),
            money: p.money.clamp(0, u32::MAX as i64) as u32,
            weather: self.weather.now,
        }
    }

    // -------------------------------------------------------------- shop

    /// Take a product off a shelf (unpaid).
    fn handle_shop_take(&mut self, pid: u16, shelf: u8, kind: u8) {
        let Some(p) = self.players.get(&pid) else { return };
        let body = p.body;
        let Some(s) = shop::shelf_in_reach(&self.shelves, &body).filter(|s| s.id == shelf) else { return };
        if !s.goods.contains(&kind) {
            return;
        }
        let Some(prod) = shop::product(kind) else { return };
        let item = Item {
            id: self.next_item_id,
            kind,
            label: prod.name.into(),
            expires: None,
            owner: 0,
            count: prod.count,
            unpaid: true,
        };
        self.next_item_id += 1;
        let p = self.players.get_mut(&pid).unwrap();
        match p.inventory.add(item) {
            Ok(()) => refresh(p),
            Err(_) => self.pending_says.push((pid, shop::lines::NO_ROOM.into(), None)),
        }
    }

    /// Pay for everything unpaid; the cashier's answer.
    fn checkout(&mut self, pid: u16) -> Option<String> {
        let p = self.players.get_mut(&pid)?;
        let total: i64 = p.inventory.items().filter(|i| i.unpaid).filter_map(|i| shop::product(i.kind)).map(|pr| pr.price).sum();
        if total == 0 {
            return Some(shop::lines::NOTHING_TO_PAY.into());
        }
        if p.money < total {
            return Some(shop::lines::too_poor(total, p.money));
        }
        p.money -= total;
        p.inventory.mark_paid();
        refresh(p);
        let (nick, left) = (p.nick.clone(), p.money);
        self.log(format!("* shop: {nick} paid {}", shop::zl(total)));
        Some(shop::lines::paid(total, left))
    }

    // ------------------------------------------------------------- board

    /// The account whose calendar the player sees: the owner of the unlocked
    /// computer they sit at.
    fn calendar_account(&self, pid: u16) -> Option<u16> {
        let h = self.players.get(&pid)?.at_computer?;
        let c = self.computers.iter().find(|c| c.handle == h && !c.locked)?;
        self.players.get(&c.owner()).filter(|o| o.contract).map(|o| o.id)
    }

    fn calendar_packet(&self, pid: u16) -> Option<Packet> {
        let account = self.calendar_account(pid)?;
        let (day, now) = (self.clock.day, self.clock.minute());
        let mine = self
            .meetings
            .iter()
            .find(|m| m.day == day && m.owner == account && matches!(m.state, board::State::Booked | board::State::Talking(_)));
        let slots = board::slots()
            .map(|start| {
                let taken = self.meetings.iter().find(|m| {
                    m.day == day && m.start == start && matches!(m.state, board::State::Booked | board::State::Talking(_))
                });
                let state = match taken {
                    Some(m) if m.owner == account => proto::slot::MINE,
                    Some(_) => proto::slot::TAKEN,
                    None if start < now + board::BOOK_AHEAD => proto::slot::PAST,
                    None => proto::slot::FREE,
                };
                (start as u16, state)
            })
            .collect();
        Some(Packet::Calendar {
            mine_start: mine.map_or(proto::NO_TIME, |m| m.start as u16),
            mine_topic: mine.map_or(board::topic::NONE, |m| m.topic),
            slots,
        })
    }

    fn send_calendar(&mut self, pid: u16) {
        if let Some(pk) = self.calendar_packet(pid) {
            let addr = self.players[&pid].addr;
            self.send(addr, &pk);
        }
    }

    /// Book / change / cancel (topic 0) the account's meeting for today.
    fn handle_calendar_book(&mut self, pid: u16, start: u32, topic: u8) {
        let Some(account) = self.calendar_account(pid) else { return };
        let (day, now) = (self.clock.day, self.clock.minute());
        let is_mine = |m: &Meeting| m.day == day && m.owner == account && m.state == board::State::Booked;
        if topic == board::topic::NONE {
            self.meetings.retain(|m| !is_mine(m));
        } else {
            let valid_slot = board::slots().any(|s| s == start) && start >= now + board::BOOK_AHEAD;
            let free = !self.meetings.iter().any(|m| {
                m.day == day && m.start == start && m.owner != account && matches!(m.state, board::State::Booked | board::State::Talking(_))
            });
            if !valid_slot || !free || board::steps(topic).is_empty() {
                self.send_calendar(pid);
                return;
            }
            self.meetings.retain(|m| !is_mine(m));
            self.meetings.push(Meeting { day, start, owner: account, topic, state: board::State::Booked });
            let who = if pid == account { String::new() } else { format!(" (wpisane przez {})", self.players[&pid].nick) };
            self.log(format!("* calendar: {} books {} at {}{who}", self.players[&account].nick, board::topic_name(topic), clock::hhmm(start)));
        }
        self.send_calendar(pid);
    }

    /// Door, missed meetings, old days.
    fn tick_meetings(&mut self) {
        let (day, now) = (self.clock.day, self.clock.minute());
        self.meetings.retain(|m| m.day + 1 >= day);
        // The board-room door lets in whoever has a meeting now.
        for p in self.players.values_mut() {
            let open = self.meetings.iter().any(|m| m.owner == p.id && m.door_open(day, now));
            p.body.access = p.inventory.access() | if open { crate::map::access::BOARD } else { 0 };
        }
        let ceo = self.npcs.iter().find(|n| n.role == npc::Role::Ceo).map(|n| n.id);
        let mut missed = Vec::new();
        for m in &mut self.meetings {
            if m.day == day && m.state == board::State::Booked && now > m.start + board::GRACE {
                m.state = board::State::Missed;
                missed.push((m.owner, m.start));
            }
        }
        for (owner, start) in missed {
            if let Some(p) = self.players.get_mut(&owner) {
                p.needs.add_stress(5);
            }
            if let Some(c) = ceo {
                self.pending_says.push((c, board::lines::missed(start), Some(owner)));
            }
        }
        // Leaving the board room ends the conversation.
        let board_room = self.board_room;
        let left: Vec<u16> = self
            .players
            .values()
            .filter(|p| p.talk.is_some() && Some((p.body.floor, p.room)) != board_room)
            .map(|p| p.id)
            .collect();
        for pid in left {
            self.end_talk(pid, None);
        }
    }

    /// E at the CEO / co-founder.
    fn start_meeting(&mut self, npc_id: u16, pid: u16) -> Option<String> {
        let role = self.npcs.iter().find(|n| n.id == npc_id)?.role;
        let who = if role == npc::Role::CoFounder { board::Who::CoFounder } else { board::Who::Ceo };
        let (day, now) = (self.clock.day, self.clock.minute());
        if self.players.get(&pid)?.talk.is_some() {
            self.send_dialog(pid);
            return None;
        }
        let Some(i) = self.meetings.iter().position(|m| m.day == day && m.owner == pid && m.state == board::State::Booked) else {
            return Some(board::lines::NO_MEETING.into());
        };
        let m = &self.meetings[i];
        if board::who(m.topic) != who {
            let other = if who == board::Who::Ceo { "ze Wspólniczką" } else { "z Prezesem" };
            return Some(format!("Twoje spotkanie jest {other}."));
        }
        if !m.can_talk(day, now) {
            return Some(board::lines::NOT_YET.into());
        }
        self.meetings[i].state = board::State::Talking(0);
        let p = self.players.get_mut(&pid)?;
        p.next_dialog_id = p.next_dialog_id.wrapping_add(1).max(1);
        p.talk = Some(Talk { meeting: i, npc: npc_id, id: p.next_dialog_id, good: 0 });
        self.send_dialog(pid);
        None
    }

    fn dialog_packet(&self, pid: u16) -> Option<Packet> {
        let t = self.players.get(&pid)?.talk.as_ref()?;
        let m = self.meetings.get(t.meeting)?;
        let board::State::Talking(step) = m.state else { return None };
        let s = board::steps(m.topic).get(step)?;
        Some(Packet::Dialog { id: t.id, npc: t.npc, text: s.text.into(), options: s.options.iter().map(|o| o.to_string()).collect() })
    }

    fn send_dialog(&mut self, pid: u16) {
        if let Some(pk) = self.dialog_packet(pid) {
            let addr = self.players[&pid].addr;
            self.send(addr, &pk);
        }
    }

    fn handle_dialog_answer(&mut self, pid: u16, dialog: u8, choice: u8) {
        let Some(t) = self.players.get(&pid).and_then(|p| p.talk) else { return };
        if t.id != dialog {
            return; // stale (resend of an answered question)
        }
        let Some(m) = self.meetings.get(t.meeting).cloned() else { return };
        let board::State::Talking(step) = m.state else { return };
        let steps = board::steps(m.topic);
        let Some(s) = steps.get(step) else { return };
        let choice = (choice as usize).min(2);
        let good = t.good + (choice == s.good) as u32;
        self.pending_says.push((t.npc, s.replies[choice].to_string(), Some(pid)));
        if step + 1 < steps.len() {
            self.meetings[t.meeting].state = board::State::Talking(step + 1);
            let p = self.players.get_mut(&pid).unwrap();
            p.next_dialog_id = p.next_dialog_id.wrapping_add(1).max(1);
            p.talk = Some(Talk { id: p.next_dialog_id, good, ..t });
            self.send_dialog(pid);
            return;
        }
        let outcome = self.meeting_outcome(pid, &m, good, t.npc);
        self.end_talk(pid, Some(outcome));
    }

    /// What the meeting achieved.
    fn meeting_outcome(&mut self, pid: u16, m: &Meeting, good: u32, npc_id: u16) -> String {
        let day = self.clock.day;
        let chance_roll = self.rng.u32(0..100);
        let p = self.players.get_mut(&pid).unwrap();
        match m.topic {
            board::topic::RAISE => {
                if p.last_raise_day.is_some_and(|d| day < d + board::RAISE_COOLDOWN_DAYS) {
                    return board::lines::RAISE_TOO_SOON.into();
                }
                p.last_raise_day = Some(day);
                let days_worked = p.day.saturating_sub(1);
                if chance_roll < board::raise_chance(days_worked, good > 0) {
                    p.pay_rate += board::RAISE_STEP;
                    board::lines::RAISE_YES.into()
                } else {
                    board::lines::RAISE_NO.into()
                }
            }
            board::topic::IDEA => match good {
                2 => {
                    p.needs.add_stress(-10);
                    let nick = p.nick.clone();
                    let name = self.npcs.iter().find(|n| n.id == npc_id).map_or("Zarząd".into(), |n| n.name.clone());
                    let text = format!("Brawa dla {nick} za świetny pomysł na produkt! Wdrażamy.");
                    self.messenger.post_system(computer::conv::GENERAL, npc_id, &name, &text);
                    board::lines::IDEA_GREAT.into()
                }
                1 => {
                    p.needs.add_stress(-3);
                    board::lines::IDEA_OK.into()
                }
                _ => {
                    p.needs.add_stress(3);
                    board::lines::IDEA_BAD.into()
                }
            },
            board::topic::COMPLAINT => {
                p.needs.add_stress(-8);
                board::lines::THANKS.into()
            }
            _ => {
                p.needs.add_stress(-5);
                board::lines::THANKS.into()
            }
        }
    }

    /// Close the conversation (optionally with the last line).
    fn end_talk(&mut self, pid: u16, last: Option<String>) {
        let Some(p) = self.players.get_mut(&pid) else { return };
        let Some(t) = p.talk.take() else { return };
        if let Some(m) = self.meetings.get_mut(t.meeting) {
            m.state = board::State::Done;
        }
        if let Some(line) = last {
            self.pending_says.push((t.npc, line, Some(pid)));
        }
        let addr = p.addr;
        let close = Packet::Dialog { id: 0, npc: t.npc, text: String::new(), options: Vec::new() };
        self.send(addr, &close);
        self.send(addr, &close);
    }

    // ------------------------------------------------------------ stalls

    /// Lock / unlock the stall the player is in (from inside only).
    fn handle_door_action(&mut self, pid: u16) {
        let Some(p) = self.players.get(&pid) else { return };
        if !matches!(p.stage, Stage::Working) {
            return;
        }
        let (floor, room, pos) = (p.body.floor, p.room, p.body.pos);
        let say = |line: &str| (pid, line.to_string(), None);
        let Some(i) = self.stalls.iter().position(|s| s.floor == floor && s.room == room) else {
            self.pending_says.push(say(stalls::lines::NO_STALL));
            return;
        };
        let door = self.stalls[i].door;
        if self.stalls[i].locked_by.is_some() {
            self.set_stall_lock(i, None);
            self.pending_says.push(say(stalls::lines::UNLOCKED));
            return;
        }
        // Nobody may be standing in the doorway (they'd end up inside a wall).
        if pos.tile() == (door.x, door.y) {
            self.pending_says.push(say(stalls::lines::STEP_IN));
            return;
        }
        if self.players.values().any(|o| o.id != pid && o.body.floor == floor && stalls::touches(o.body.pos, door)) {
            self.pending_says.push(say(stalls::lines::IN_DOORWAY));
            return;
        }
        self.set_stall_lock(i, Some(pid));
        self.pending_says.push(say(stalls::lines::LOCKED));
    }

    fn set_stall_lock(&mut self, i: usize, by: Option<u16>) {
        let s = &mut self.stalls[i];
        s.locked_by = by;
        let (floor, door) = (s.floor, s.door);
        if let Some(m) = self.building.floor_mut(floor) {
            m.set_closed(door.x, door.y, by.is_some());
        }
        self.doors_dirty = true;
    }

    /// Whoever locked a stall and isn't in it any more (left, disconnected)
    /// unlocks it.
    fn check_stalls(&mut self) {
        let stale: Vec<usize> = self
            .stalls
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.locked_by.is_some_and(|pid| {
                    self.players.get(&pid).is_none_or(|p| p.body.floor != s.floor || p.room != s.room)
                })
            })
            .map(|(i, _)| i)
            .collect();
        for i in stale {
            self.set_stall_lock(i, None);
        }
    }

    fn doors_packet(&self, floor: u8) -> Packet {
        let tiles = self
            .building
            .floor(floor)
            .map_or_else(Vec::new, |m| m.closed_tiles().into_iter().map(|(x, y)| (x as u8, y as u8)).collect());
        let (lift_floor, lift_target, lift_moving) = self
            .elevators
            .first()
            .map_or((proto::NO_FLOOR, proto::NO_FLOOR, false), |e| (e.floor, e.heading().unwrap_or(proto::NO_FLOOR), e.moving.is_some()));
        Packet::Doors { floor, tiles, lift_floor, lift_target, lift_moving }
    }

    // --------------------------------------------------------- computers

    /// `--start-employed`: contract, department, card and laptop, and a spot
    /// in front of a desk of the department.
    fn employ(&mut self, id: u16) {
        let dept = if id % 2 == 1 { 1 } else { 2 };
        let Some(dept_name) = self.cfg.recruitment.department_name(dept).map(str::to_string) else { return };
        let seat = {
            let n = self.players.values().filter(|p| p.contract && p.department == dept).count();
            let desks: Vec<&Workstation> = self.workstations.iter().filter(|w| w.room_name == dept_name).collect();
            desks.get(n % desks.len().max(1)).and_then(|w| {
                let m = self.building.floor(w.floor)?;
                [1, -1].iter().map(|dy| (w.tile.x, w.tile.y + dy)).find(|&(x, y)| !m.is_blocked(x, y)).map(|(x, y)| (w.floor, x, y))
            })
        };
        let Some(p) = self.players.get_mut(&id) else { return };
        p.department = dept;
        p.contract = true;
        p.money += shop::ADVANCE;
        if let Some((floor, x, y)) = seat {
            p.body = Body::at(floor, Pos::tile_center(x, y));
            p.room = self.building.floor(floor).map_or(0, |m| m.room_at_tile(x, y));
        }
        self.give_new(id, item_kind::EMPLOYEE_CARD);
        self.give_new(id, item_kind::LAPTOP);
    }

    /// E at a sofa, toilet, ashtray or the fruit bowl. `None` = nothing in
    /// reach; `Some(line)` = handled. E while resting gets you up.
    fn use_spot(&mut self, pid: u16, body: &Body) -> Option<Option<String>> {
        let p = self.players.get_mut(&pid)?;
        if p.rest.take().is_some() {
            return Some(None);
        }
        let spot = needs::spot_in_reach(&self.spots, body)?.clone();
        let (floor, pos) = (p.body.floor, p.body.pos);
        let line = match spot.kind {
            SpotKind::FruitBowl => {
                let has_room = p.inventory.hands_free() || p.inventory.pockets.iter().any(|s| s.is_none());
                if !has_room {
                    return Some(Some(needs::lines::HANDS_FULL.into()));
                }
                let fruit = FRUITS[self.rng.usize(..FRUITS.len())];
                let item = Item {
                    id: self.next_item_id,
                    kind: item_kind::FRUIT,
                    label: fruit.into(),
                    expires: None,
                    owner: 0,
                    count: 1,
                    unpaid: false,
                };
                self.next_item_id += 1;
                self.give(pid, item);
                format!("{} {}", needs::lines::FRUIT, fruit)
            }
            SpotKind::Sofa => {
                p.rest = Some((Rest::Sofa, floor, pos));
                needs::lines::SOFA.into()
            }
            SpotKind::Ashtray => {
                // One cigarette from a paid pack.
                let slot = std::iter::once(&mut p.inventory.hands)
                    .chain(p.inventory.pockets.iter_mut())
                    .find(|s| s.as_ref().is_some_and(|i| i.kind == item_kind::CIGARETTES && !i.unpaid && i.count > 0));
                let Some(slot) = slot else {
                    return Some(Some(shop::lines::NO_CIGARETTES.into()));
                };
                let pack = slot.as_mut().unwrap();
                pack.count -= 1;
                if pack.count == 0 {
                    *slot = None;
                }
                refresh(p);
                p.rest = Some((Rest::Smoking { until: self.tick + needs::SMOKE_TICKS }, floor, pos));
                needs::lines::SMOKE.into()
            }
            SpotKind::Sink => {
                p.rest = Some((Rest::Washing { until: self.tick + needs::WASH_TICKS }, floor, pos));
                needs::lines::WASHING.into()
            }
            SpotKind::Sanitizer => {
                p.needs.sanitize();
                needs::lines::SANITIZED.into()
            }
            SpotKind::Toilet => {
                p.rest = Some((Rest::Toilet, floor, pos));
                p.needs.use_toilet();
                let mine = match p.profile.gender {
                    proto::gender::FEMALE => Some("female"),
                    proto::gender::MALE => Some("male"),
                    _ => None,
                };
                match (mine, spot.gender.as_deref()) {
                    (Some(m), Some(g)) if m != g => {
                        p.needs.add_stress(5);
                        needs::lines::WRONG_BATHROOM.into()
                    }
                    _ => needs::lines::TOILET.into(),
                }
            }
        };
        Some(Some(line))
    }

    /// Hired employees = messenger accounts.
    fn accounts(&self) -> Vec<Account> {
        self.players
            .values()
            .filter(|p| p.contract && matches!(p.stage, Stage::Working))
            .map(|p| Account { id: p.id, nick: p.nick.clone(), department: p.department })
            .collect()
    }

    /// E at a desk: open the computer on it, or put the laptop in hands down.
    /// `None` = no desk in reach (the E goes on to picking things up);
    /// `Some(line)` = handled, with an optional speech line.
    fn use_desk(&mut self, pid: u16, body: &Body) -> Option<Option<String>> {
        let ws = computer::workstation_in_reach(&self.workstations, body)?;
        if let Some(c) = self.computers.iter_mut().find(|c| c.station == ws) {
            if c.user.is_some_and(|u| u != pid) {
                return Some(Some(computer::lines::BUSY.into()));
            }
            c.user = Some(pid);
            let handle = c.handle;
            self.players.get_mut(&pid)?.at_computer = Some(handle);
            self.send_computer(pid);
            return Some(None);
        }
        let p = self.players.get(&pid)?;
        if p.inventory.held_kind() != item_kind::LAPTOP {
            return None;
        }
        let dept = self.cfg.recruitment.department_name(p.department).unwrap_or("");
        if !p.contract || dept.is_empty() {
            return Some(Some(computer::lines::NO_DEPARTMENT.into()));
        }
        if self.workstations[ws].room_name != dept {
            return Some(Some(computer::lines::NOT_MY_DEPARTMENT.into()));
        }
        let handle = self.alloc_handle();
        let p = self.players.get_mut(&pid)?;
        let item = p.inventory.take_hands()?;
        refresh(p);
        self.computers.push(Computer { handle, station: ws, item, locked: false, user: None });
        Some(Some(computer::lines::PLACED.into()))
    }

    /// Stop looking at the screen (the client closes it when the
    /// AT_COMPUTER status bit clears).
    fn end_session(&mut self, pid: u16) {
        let Some(p) = self.players.get_mut(&pid) else { return };
        if let Some(h) = p.at_computer.take() {
            if let Some(c) = self.computers.iter_mut().find(|c| c.handle == h && c.user == Some(pid)) {
                c.user = None;
            }
        }
    }

    /// Users who walked away (or whose computer is gone) leave the screen.
    fn check_computer_sessions(&mut self) {
        let mut gone = Vec::new();
        for p in self.players.values() {
            let Some(h) = p.at_computer else { continue };
            let ok = self.computers.iter().find(|c| c.handle == h).is_some_and(|c| {
                c.user == Some(p.id) && computer::in_leave_range(&self.workstations[c.station], &p.body)
            });
            if !ok {
                gone.push(p.id);
            }
        }
        for pid in gone {
            self.end_session(pid);
        }
    }

    fn computer_packet(&self, pid: u16) -> Option<Packet> {
        let h = self.players.get(&pid)?.at_computer?;
        let c = self.computers.iter().find(|c| c.handle == h)?;
        let accounts = self.accounts();
        let convs = match accounts.iter().find(|a| a.id == c.owner()) {
            Some(acc) if !c.locked => self
                .messenger
                .conversations(acc, &accounts, |d| self.cfg.recruitment.department_name(d).map(str::to_string))
                .into_iter()
                .map(|i| proto::ConvEntry { conv: i.conv, unread: i.unread, title: i.title })
                .collect(),
            _ => Vec::new(),
        };
        Some(Packet::Computer { handle: h, owner: c.owner(), locked: c.locked, convs })
    }

    fn send_computer(&mut self, pid: u16) {
        if let Some(pk) = self.computer_packet(pid) {
            let addr = self.players[&pid].addr;
            self.send(addr, &pk);
        }
    }

    /// Send messages, split so each datagram fits.
    fn send_chat(&mut self, addr: SocketAddr, conv: u16, msgs: &[computer::Msg]) {
        let mut chunk: Vec<proto::ChatEntry> = Vec::new();
        let mut size = proto::HEADER_LEN + 3;
        for m in msgs {
            let len = 4 + 2 + 2 + m.nick.len().min(proto::MAX_NICK_BYTES) + 2 + m.text.len().min(proto::MAX_CHAT_BYTES);
            if size + len > proto::MAX_PACKET && !chunk.is_empty() {
                self.send(addr, &Packet::Chat { conv, messages: std::mem::take(&mut chunk) });
                size = proto::HEADER_LEN + 3;
            }
            size += len;
            chunk.push(proto::ChatEntry { id: m.id, from: m.from, nick: m.nick.clone(), text: m.text.clone() });
        }
        self.send(addr, &Packet::Chat { conv, messages: chunk });
    }

    fn handle_computer_action(&mut self, pid: u16, action: u8, conv: u16, arg: u32, text: &str) {
        use proto::computer_action as a;
        let Some(h) = self.players.get(&pid).and_then(|p| p.at_computer) else { return };
        let Some(ci) = self.computers.iter().position(|c| c.handle == h && c.user == Some(pid)) else { return };
        let say = |line: &str| (pid, line.to_string(), None);
        let owner = self.computers[ci].owner();
        match action {
            a::CLOSE => self.end_session(pid),
            a::LOCK => {
                self.computers[ci].locked = true;
                self.end_session(pid);
            }
            a::UNLOCK => {
                if pid == owner {
                    self.computers[ci].locked = false;
                    self.send_computer(pid);
                } else {
                    self.pending_says.push(say(computer::lines::LOCKED));
                }
            }
            a::TAKE => {
                if !self.players[&pid].inventory.hands_free() {
                    self.pending_says.push(say(computer::lines::HANDS_FULL));
                    return;
                }
                self.end_session(pid);
                let c = self.computers.remove(ci);
                if owner != pid {
                    let (who, whose) = (self.players[&pid].nick.clone(), c.item.label.clone());
                    self.log(format!("* computer: {who} took {whose}"));
                }
                self.give(pid, c.item);
                self.pending_says.push(say(computer::lines::TAKEN));
            }
            a::SYNC | a::SEND => {
                if self.computers[ci].locked {
                    return;
                }
                let accounts = self.accounts();
                let Some(acc) = accounts.iter().find(|x| x.id == owner).cloned() else { return };
                let addr = self.players[&pid].addr;
                if action == a::SYNC {
                    if let Some(msgs) = self.messenger.sync(&acc, conv, arg, &accounts) {
                        self.send_chat(addr, conv, &msgs);
                    }
                    return;
                }
                let tick = self.tick;
                let p = self.players.get_mut(&pid).unwrap();
                if arg == p.last_chat_nonce || p.last_chat_tick.is_some_and(|t| tick.wrapping_sub(t) < computer::SEND_COOLDOWN_TICKS) {
                    return; // retry of a message already posted, or spam
                }
                let Some(msg) = self.messenger.post(&acc, conv, text, &accounts) else { return };
                let p = self.players.get_mut(&pid).unwrap();
                p.last_chat_nonce = arg;
                p.last_chat_tick = Some(tick);
                let typed_by = p.nick.clone();
                if pid == owner {
                    self.log(format!("* chat: {} -> conv {conv}", acc.nick));
                } else {
                    self.log(format!("* chat: {typed_by} as {} -> conv {conv}", acc.nick));
                }
                // Live push to everyone looking at a screen of an account in
                // the audience (the others get it from SYNC / unread counts).
                let mut pushes = Vec::new();
                for (account, seen_as) in self.messenger.audience(&acc, conv, &accounts) {
                    for viewer in self.players.values() {
                        let Some(vh) = viewer.at_computer else { continue };
                        if self.computers.iter().any(|c| c.handle == vh && c.owner() == account && !c.locked) {
                            pushes.push((viewer.addr, viewer.id, seen_as));
                        }
                    }
                }
                for (addr, vid, seen_as) in pushes {
                    self.send_chat(addr, seen_as, std::slice::from_ref(&msg));
                    self.send_computer(vid);
                }
            }
            _ => {}
        }
    }

    fn remove_player(&mut self, id: u16, why: &str) {
        if let Some(mut p) = self.players.remove(&id) {
            coffee::release(&mut self.machines, &p.cup);
            for c in &mut self.computers {
                if c.user == Some(id) {
                    c.user = None;
                }
            }
            // Other people's things they carried stay in the building...
            let carried: Vec<Item> = std::mem::take(&mut p.inventory).items().cloned().collect();
            for item in carried.into_iter().filter(|i| i.owner != 0 && i.owner != id) {
                self.drop_at(p.body.floor, p.body.pos, item);
            }
            // ...their own go with them (no persistent accounts yet): laptop on
            // a desk, card lent to someone, anything on the floor.
            self.computers.retain(|c| c.owner() != id);
            self.vehicles.retain(|v| v.owner != id);
            self.dropped.retain(|d| d.item.owner != id);
            for other in self.players.values_mut() {
                let before = other.inventory.clone();
                other.inventory.remove_owned_by(id);
                if other.inventory != before {
                    refresh(other);
                }
            }
            self.messenger.forget(id);
            self.by_token.remove(&p.token);
            if self.by_addr.get(&p.addr) == Some(&id) {
                self.by_addr.remove(&p.addr);
            }
            for other in self.players.values_mut() {
                other.known.remove(&id);
            }
            self.log(format!("- player {} '{}' {} ({} online)", id, p.nick, why, self.players.len()));
        }
    }

    fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        let now = Instant::now();
        self.tick_clock();

        // 1. Timeouts.
        let stale: Vec<u16> = self
            .players
            .values()
            .filter(|p| now.duration_since(p.last_heard) > self.cfg.client_timeout)
            .map(|p| p.id)
            .collect();
        for id in stale {
            let (addr, token) = (self.players[&id].addr, self.players[&id].token);
            self.send(addr, &Packet::Disconnect { token, reason: proto::disconnect::TIMEOUT });
            self.remove_player(id, "timed out");
        }

        // 2. Simulation: apply queued inputs in sequence order.
        let mut talks: Vec<(u16, Body)> = Vec::new();
        let mut coffee_says: Vec<(u16, String, Option<u16>)> = std::mem::take(&mut self.pending_says);
        let mut coffee_ready = Vec::new();
        let mut portal_resend = Vec::new();
        // Left a bathroom with dirty hands: (player, floor, bathroom room).
        let mut unwashed_exits: Vec<(u16, u8, u16)> = Vec::new();
        // Walked out of the shop with unpaid goods.
        let mut shoplifters: Vec<u16> = Vec::new();
        for p in self.players.values_mut() {
            if !matches!(p.stage, Stage::Working) {
                p.inputs.clear(); // not in the world yet
                portal_resend.push(p.id);
                continue;
            }
            if p.riding.is_some() {
                // In a vehicle: no walking (inputs are acknowledged, ignored).
                if let Some((seq, _)) = p.inputs.back() {
                    p.last_processed_seq = *seq;
                }
                p.inputs.clear();
                continue;
            }
            let mut moved = false;
            for _ in 0..MAX_INPUTS_PER_TICK {
                let Some((seq, bits)) = p.inputs.pop_front() else { break };
                let before = p.body;
                p.body = sim::step(&self.building, p.body, bits);
                p.last_processed_seq = seq;
                // E pressed (edge) without riding the elevator: talk to an NPC nearby.
                let pressed = bits & sim::IN_INTERACT != 0 && before.prev_input & sim::IN_INTERACT == 0;
                if pressed && p.body.floor == before.floor {
                    talks.push((p.id, p.body));
                }
                moved |= p.body.pos != before.pos || p.body.floor != before.floor;
                let (dx, dy) = sim::input_dir(bits);
                let facing = if dy > 0 { 0 } else if dy < 0 { 1 } else if dx < 0 { 2 } else if dx > 0 { 3 } else { p.flags & 3 };
                p.flags = facing;
            }
            if moved {
                p.flags |= 0b100;
            }
            let old_room = p.room;
            p.room = self.building.floor(p.body.floor).map_or(0, |m| m.room_at(p.body.pos.x, p.body.pos.y));
            if p.room != old_room
                && self.shop_rooms.contains(&(p.body.floor, old_room))
                && p.inventory.items().any(|i| i.unpaid)
            {
                shoplifters.push(p.id);
            }
            if p.needs.dirty_hands && p.room != old_room {
                let kind = |r: u16| {
                    self.building.floor(p.body.floor).and_then(|m| m.rooms.iter().find(|d| d.id == r)).map_or("", |d| d.kind.as_str())
                };
                if kind(old_room) == "bathroom" && !matches!(kind(p.room), "bathroom" | "stall") {
                    unwashed_exits.push((p.id, p.body.floor, old_room));
                }
            }
            if coffee::tick_cup(&mut p.cup, self.tick) {
                coffee_ready.push(p.id);
            }
            if !p.inventory.expire(self.tick).is_empty() {
                refresh(p);
                coffee_says.push((p.id, coffee::lines::COLD.to_string(), None));
            }
            // Needs: moving ends a rest; exhausted / desperate = slow.
            if let Some((_, floor, pos)) = p.rest {
                if (floor, pos) != (p.body.floor, p.body.pos) {
                    p.rest = None;
                }
            }
            let mut rest = p.rest.map(|r| r.0);
            let mut events = Vec::new();
            for _ in 0..self.cfg.needs_speed.max(1) {
                let (r, ev) = p.needs.tick(rest, self.tick);
                rest = r;
                events.extend(ev);
            }
            p.rest = rest.map(|r| (r, p.body.floor, p.body.pos));
            for e in events {
                let line = match e {
                    needs::Event::Warn(l) | needs::Event::RestDone(l) => l,
                    needs::Event::Accident => needs::lines::ACCIDENT,
                };
                coffee_says.push((p.id, line.to_string(), None));
            }
            // Weather under the open sky.
            let outdoors = self.outdoor_rooms.contains(&(p.body.floor, p.room));
            let umbrella = p.inventory.has(item_kind::UMBRELLA);
            let mut umbrella_open = false;
            if outdoors {
                let (h, s) = weather::outdoor_effect(self.weather.now, umbrella);
                p.needs.weather(h, s);
                umbrella_open = umbrella && weather::wet(self.weather.now);
                if weather::wet(self.weather.now) && !umbrella && !p.soaked_said {
                    p.soaked_said = true;
                    coffee_says.push((p.id, weather::lines::SOAKED.to_string(), None));
                }
            } else {
                p.soaked_said = false;
            }
            p.body.slow = p.needs.slow();
            p.flags = (p.flags & 0x37)
                | if umbrella_open { proto::FLAG_UMBRELLA } else { 0 }
                | if p.body.slow { proto::FLAG_SLOW } else { 0 }
                | if p.needs.smelly() { proto::FLAG_SMELLY } else { 0 };
        }
        // The security gate beeps; the goods stay in the shop.
        for pid in shoplifters {
            if let Some(p) = self.players.get_mut(&pid) {
                p.inventory.remove_unpaid();
                p.needs.add_stress(10);
                refresh(p);
            }
            let speaker = self.cashier.unwrap_or(pid);
            coffee_says.push((speaker, shop::lines::ALARM.to_string(), Some(pid)));
        }
        // Somebody in the bathroom saw it.
        for (pid, floor, bath) in unwashed_exits {
            let m = self.building.floor(floor);
            let witness = self.players.values().find(|o| {
                o.id != pid
                    && matches!(o.stage, Stage::Working)
                    && o.body.floor == floor
                    && (o.room == bath || m.is_some_and(|m| m.visible_from(o.room).contains(&bath)))
            });
            if let Some(w) = witness.map(|w| w.id) {
                let nick = self.players[&pid].nick.clone();
                coffee_says.push((w, format!("Ej, {nick}, a ręce?!"), Some(pid)));
                if let Some(p) = self.players.get_mut(&pid) {
                    p.needs.add_stress(3);
                }
            }
        }
        for pid in coffee_ready {
            let free = self.players[&pid].inventory.hands_free();
            coffee_says.push((pid, if free { coffee::lines::READY } else { coffee::lines::WAITING }.to_string(), None));
            self.give_new(pid, item_kind::COFFEE); // no free hands: it waits on the floor
        }

        for id in portal_resend {
            self.deliver_replies(id);
            if self.tick % PORTAL_RESEND_TICKS == 0 {
                self.send_portal(id, self.tick % (2 * PORTAL_RESEND_TICKS) == 0);
            }
        }

        // 3. NPCs: conversations, then their own behaviour.
        let mut events = Vec::new();
        for (pid, body) in talks {
            let p = &self.players[&pid];
            let dept = self.cfg.recruitment.department_name(p.department).map(str::to_string);
            let nearest = self
                .npcs
                .iter_mut()
                .filter(|n| n.in_talk_range(&body))
                // NPCs at their post first (a porter still standing next to
                // the guest he just brought mustn't shadow the receptionist).
                .min_by_key(|n| (!n.is_idle(), (n.body.pos.x - body.pos.x).abs() + (n.body.pos.y - body.pos.y).abs()));
            let hands_free = p.inventory.hands_free();
            if let Some(n) = nearest {
                events.extend(n.interact(&self.building, pid, body.access, dept.as_deref(), hands_free));
            } else if let Some(i) = coffee::machine_in_reach(&self.machines, &body) {
                let p = self.players.get_mut(&pid).unwrap();
                let line = match coffee::use_machine(&mut self.machines, i, &mut p.cup, hands_free, self.tick) {
                    coffee::Outcome::Started => coffee::lines::BREWING,
                    coffee::Outcome::Busy => coffee::lines::BUSY,
                    coffee::Outcome::HandsFull => coffee::lines::HANDS_FULL,
                };
                coffee_says.push((pid, line.to_string(), None));
            } else if let Some(said) = self.use_desk(pid, &body) {
                coffee_says.extend(said.map(|line| (pid, line, None)));
            } else if let Some(said) = self.use_spot(pid, &body) {
                coffee_says.extend(said.map(|line| (pid, line, None)));
            } else if let Some(line) = self.use_elevator(&body) {
                coffee_says.push((pid, line, None));
            } else if let Some(s) = shop::shelf_in_reach(&self.shelves, &body) {
                let goods = s
                    .goods
                    .iter()
                    .filter_map(|&k| shop::product(k))
                    .map(|p| proto::ShelfItem { kind: p.kind, price: p.price as u32, name: p.name.into() })
                    .collect();
                let packet = Packet::Shelf { shelf: s.id, title: s.title.into(), goods };
                let addr = self.players[&pid].addr;
                self.send(addr, &packet);
            } else if let Some(line) = self.try_pickup(pid, &body) {
                coffee_says.push((pid, line, None));
            }
        }
        self.check_computer_sessions();
        self.check_stalls();
        self.tick_elevators();
        self.tick_vehicles();
        self.tick_meetings();

        let bodies: HashMap<u16, Body> =
            self.players.values().filter(|p| matches!(p.stage, Stage::Working)).map(|p| (p.id, p.body)).collect();
        for n in &mut self.npcs {
            events.extend(n.tick(&self.building, &bodies));
        }
        let mut says: Vec<(u16, String, Option<u16>)> = coffee_says;
        for e in events {
            match e {
                npc::Event::Give { player, item } => self.give_new(player, item),
                npc::Event::Take { player, item } => {
                    if let Some(p) = self.players.get_mut(&player) {
                        if p.inventory.remove_kind(item).is_some() {
                            refresh(p);
                        }
                    }
                }
                npc::Event::Say { npc, text, to } => says.push((npc, text, to)),
                npc::Event::Meeting { npc, player } => {
                    if let Some(line) = self.start_meeting(npc, player) {
                        says.push((npc, line, Some(player)));
                    }
                }
                npc::Event::Checkout { npc, player } => {
                    if let Some(line) = self.checkout(player) {
                        says.push((npc, line, Some(player)));
                    }
                }
                npc::Event::Contract { player } => {
                    if let Some(p) = self.players.get_mut(&player) {
                        p.contract = true;
                        p.money += shop::ADVANCE;
                        let msg = format!(
                            "* player {player} '{}' signed a contract: {}",
                            p.nick,
                            self.cfg.recruitment.department_name(p.department).unwrap_or("-")
                        );
                        self.log(msg);
                    }
                    // Everyone gets the updated PlayerInfo (department) again.
                    for other in self.players.values_mut() {
                        other.known.remove(&player);
                    }
                }
            }
        }

        // 4. Interest management: group entities by (floor, room).
        let mut groups: HashMap<(u8, u16), Vec<EntityState>> = HashMap::new();
        for v in &self.vehicles {
            let room = self.building.floor(0).map_or(0, |m| m.room_at(v.pos.x, v.pos.y));
            groups.entry((0, room)).or_default().push(EntityState {
                id: v.handle,
                kind: proto::kind::VEHICLE,
                x: v.pos.x,
                y: v.pos.y,
                flags: v.facing | if v.moving { 0b100 } else { 0 },
                held: v.kind,
                activity: 0,
            });
        }
        // Riders are inside their vehicle: not shown.
        for p in self.players.values().filter(|p| matches!(p.stage, Stage::Working) && p.riding.is_none()) {
            groups.entry((p.body.floor, p.room)).or_default().push(EntityState {
                id: p.id,
                kind: proto::kind::PLAYER,
                x: p.body.pos.x,
                y: p.body.pos.y,
                flags: p.flags,
                held: p.inventory.held_kind(),
                activity: activity(p),
            });
        }
        for n in &self.npcs {
            groups.entry((n.body.floor, n.room)).or_default().push(EntityState {
                id: n.id,
                kind: proto::kind::NPC,
                x: n.body.pos.x,
                y: n.body.pos.y,
                flags: n.flags,
                held: 0,
                activity: 0,
            });
        }
        for c in &self.computers {
            let w = &self.workstations[c.station];
            let pos = Pos::tile_center(w.tile.x, w.tile.y);
            let room = self.building.floor(w.floor).map_or(0, |m| m.room_at(pos.x, pos.y));
            groups.entry((w.floor, room)).or_default().push(EntityState {
                id: c.handle,
                kind: proto::kind::COMPUTER,
                x: pos.x,
                y: pos.y,
                flags: computer::entity_flags(c),
                held: c.item.kind,
                activity: 0,
            });
        }
        for d in &self.dropped {
            let room = self.building.floor(d.floor).map_or(0, |m| m.room_at(d.pos.x, d.pos.y));
            groups.entry((d.floor, room)).or_default().push(EntityState {
                id: d.handle,
                kind: proto::kind::ITEM,
                x: d.pos.x,
                y: d.pos.y,
                flags: 0,
                held: d.item.kind,
                activity: 0,
            });
        }

        // 5. Snapshots + PlayerInfo for newly visible entities.
        let tick = self.tick;
        let ids: Vec<u16> =
            self.players.values().filter(|p| matches!(p.stage, Stage::Working)).map(|p| p.id).collect();
        let mut outgoing: Vec<(SocketAddr, u16, Packet)> = Vec::new();
        for id in ids {
            let p = &self.players[&id];
            let also: &[u16] = self.building.floor(p.body.floor).map_or(&[], |m| m.visible_from(p.room));
            let visible: Vec<EntityState> = std::iter::once(&p.room)
                .chain(also)
                .filter_map(|r| groups.get(&(p.body.floor, *r)))
                .flatten()
                .filter(|e| e.id != id)
                .copied()
                .collect();
            self.stats.max_visible = self.stats.max_visible.max(visible.len());
            let new_infos: Vec<PlayerInfoEntry> = visible
                .iter()
                .filter(|e| !p.known.contains(&e.id))
                .filter_map(|e| self.info_of(e.id))
                .collect();
            let me = SelfState {
                x: p.body.pos.x,
                y: p.body.pos.y,
                floor: p.body.floor,
                room: p.room,
                lock: p.body.lock,
                prev_input: p.body.prev_input,
                access: p.body.access,
                slow: p.body.slow,
                activity: activity(p),
            };
            for f in proto::snapshot_fragments(tick, p.last_processed_seq, me, &visible) {
                outgoing.push((p.addr, id, f));
            }
            if p.inv_dirty || tick % INVENTORY_RESEND_TICKS == 0 {
                outgoing.push((p.addr, id, inventory_packet(&p.inventory)));
            }
            if self.doors_dirty || tick % STATS_EVERY_TICKS == 0 {
                outgoing.push((p.addr, id, self.doors_packet(p.body.floor)));
            }
            if tick % STATS_EVERY_TICKS == 0 {
                let [hunger, energy, stress, bladder, hygiene] = p.needs.points();
                let flags = if p.needs.dirty_hands { proto::STATS_DIRTY_HANDS } else { 0 };
                let money = p.money.clamp(0, u32::MAX as i64) as u32;
                outgoing.push((p.addr, id, Packet::Stats { hunger, energy, stress, bladder, hygiene, flags, money }));
            }
            if tick % COMPUTER_RESEND_TICKS == 0 {
                if let Some(pk) = self.computer_packet(id) {
                    outgoing.push((p.addr, id, pk));
                }
                if let Some(pk) = self.calendar_packet(id) {
                    outgoing.push((p.addr, id, pk));
                }
                if let Some(pk) = self.dialog_packet(id) {
                    outgoing.push((p.addr, id, pk));
                }
            }
            for chunk in new_infos.chunks(INFO_PER_PACKET) {
                outgoing.push((p.addr, id, Packet::PlayerInfo { players: chunk.to_vec() }));
            }
            let p = self.players.get_mut(&id).unwrap();
            p.known.extend(new_infos.iter().map(|e| e.id));
            p.inv_dirty = false;
        }
        self.doors_dirty = false;
        // 6. Speech (NPCs, and players' own "thought" lines): to everyone in
        //    the speaker's room or seeing into it (e.g. from a toilet stall),
        //    plus the addressee.
        for (npc_id, text, to) in says {
            let speaker = match self.npcs.iter().find(|n| n.id == npc_id) {
                Some(n) => Some(((n.body.floor, n.room), n.name.clone())),
                None => self.players.get(&npc_id).map(|p| ((p.body.floor, p.room), p.nick.clone())),
            };
            let Some((place, name)) = speaker else { continue };
            let info = self.info_of(npc_id).unwrap_or(PlayerInfoEntry {
                id: npc_id,
                nick: name,
                department: 0,
                gender: proto::gender::OTHER,
                appearance: proto::Appearance::default(),
            });
            for p in self.players.values_mut() {
                let sees = p.body.floor == place.0
                    && self.building.floor(p.body.floor).is_some_and(|m| m.visible_from(p.room).contains(&place.1));
                if (p.body.floor, p.room) == place || sees || Some(p.id) == to || p.id == npc_id {
                    // Name first, so the line isn't shown as "?".
                    if p.id != npc_id && p.known.insert(npc_id) {
                        outgoing.push((p.addr, p.id, Packet::PlayerInfo { players: vec![info.clone()] }));
                    }
                    outgoing.push((p.addr, p.id, Packet::Say { id: npc_id, text: text.clone() }));
                }
            }
        }
        // Game time for everyone (also at home / on the portal).
        if self.clock_dirty || tick % CLOCK_RESEND_TICKS == 0 {
            for p in self.players.values() {
                outgoing.push((p.addr, p.id, self.clock_packet(p)));
            }
            self.clock_dirty = false;
        }
        for (addr, id, packet) in outgoing {
            let n = self.send(addr, &packet);
            if matches!(packet, Packet::Snapshot { .. }) {
                self.stats.snapshot_bytes += n as u64;
            }
            if let Some(p) = self.players.get_mut(&id) {
                p.bytes_out += n as u64;
            }
        }
    }

    fn print_stats(&mut self) {
        let secs = self.cfg.stats_every.as_secs_f64();
        let s = std::mem::take(&mut self.stats);
        let avg_us = if s.ticks > 0 { s.tick_total.as_micros() as f64 / s.ticks as f64 } else { 0.0 };
        let n = self.players.len();
        let (mut min_bps, mut max_bps, mut sum) = (u64::MAX, 0u64, 0u64);
        for p in self.players.values_mut() {
            let bps = (p.bytes_out as f64 / secs) as u64;
            min_bps = min_bps.min(bps);
            max_bps = max_bps.max(bps);
            sum += bps;
            p.bytes_out = 0;
        }
        let avg_bps = if n > 0 { sum / n as u64 } else { 0 };
        if n == 0 {
            min_bps = 0;
        }
        let msg = format!(
            "tick {} | ticks {} (missed {}) | tick avg {:.0} us max {} us | players {} max_visible {} | out/client avg {:.1} KB/s (min {:.1}, max {:.1}) | in {:.0} pkt/s {:.1} KB/s | out {:.0} pkt/s | sim-dropped {}",
            self.tick,
            s.ticks,
            s.missed,
            avg_us,
            s.tick_max.as_micros(),
            n,
            s.max_visible,
            avg_bps as f64 / 1024.0,
            min_bps as f64 / 1024.0,
            max_bps as f64 / 1024.0,
            self.net.packets_in as f64 / secs,
            self.net.bytes_in as f64 / secs / 1024.0,
            self.net.packets_out as f64 / secs,
            self.net.dropped,
        );
        self.net.packets_in = 0;
        self.net.bytes_in = 0;
        self.net.packets_out = 0;
        self.net.bytes_out = 0;
        self.net.dropped = 0;
        self.log(msg);
    }
}

/// What others see the player doing (one at a time, most visible first).
fn activity(p: &Player) -> u8 {
    use proto::activity as a;
    if p.riding.is_some() {
        return a::RIDING;
    }
    match (p.at_computer, p.rest.map(|r| r.0)) {
        (Some(_), _) => a::COMPUTER,
        (_, Some(Rest::Toilet)) => a::TOILET,
        (_, Some(Rest::Sofa)) => a::SOFA,
        (_, Some(Rest::Smoking { .. })) => a::SMOKING,
        (_, Some(Rest::Washing { .. })) => a::WASHING,
        _ if p.cup.brewing() => a::BREWING,
        _ => a::NONE,
    }
}

/// After an inventory change: access follows the carried items.
fn refresh(p: &mut Player) {
    p.body.access = p.inventory.access();
    p.inv_dirty = true;
}

fn dist2(a: Pos, b: Pos) -> i32 {
    let (dx, dy) = (a.x - b.x, a.y - b.y);
    dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
}

fn inventory_packet(inv: &Inventory) -> Packet {
    let slot = |it: &Option<Item>| match it {
        Some(i) => {
            let mut label = i.label.clone();
            if i.count > 1 {
                label += &format!(" ({} szt.)", i.count);
            }
            if i.unpaid {
                let price = shop::product(i.kind).map_or(0, |p| p.price);
                label += &format!(" — niezapłacone, {}", shop::zl(price));
            }
            proto::SlotInfo { kind: i.kind, id: i.id, label }
        }
        None => proto::SlotInfo::default(),
    };
    let mut slots = vec![slot(&inv.hands)];
    slots.extend(inv.pockets.iter().map(slot));
    Packet::Inventory { slots }
}

/// Check and normalise a character profile. `None` = reject.
pub fn validate_profile(mut p: Profile) -> Option<Profile> {
    let clean = |s: &str| s.chars().filter(|c| !c.is_control()).collect::<String>().trim().to_string();
    p.city = clean(&p.city);
    p.email = clean(&p.email).to_lowercase();
    let email_ok = {
        let e = &p.email;
        let parts: Vec<&str> = e.split('@').collect();
        parts.len() == 2
            && !parts[0].is_empty()
            && parts[1].contains('.')
            && !parts[1].starts_with('.')
            && !parts[1].ends_with('.')
            && !e.contains(char::is_whitespace)
    };
    let ok = p.gender <= proto::gender::OTHER
        && (18..=70).contains(&p.age)
        && !p.city.is_empty()
        && email_ok
        && p.appearance.is_valid();
    ok.then_some(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Appearance;

    fn good() -> Profile {
        Profile { gender: 0, age: 27, city: " Łódź ".into(), email: "Ola@Poczta.PL".into(), appearance: Appearance::default() }
    }

    #[test]
    fn profile_is_normalised() {
        let p = validate_profile(good()).unwrap();
        assert_eq!((p.city.as_str(), p.email.as_str()), ("Łódź", "ola@poczta.pl"));
    }

    #[test]
    fn bad_profiles_are_rejected() {
        let cases: Vec<Profile> = vec![
            Profile { age: 12, ..good() },
            Profile { age: 90, ..good() },
            Profile { gender: 7, ..good() },
            Profile { city: "   ".into(), ..good() },
            Profile { email: "ola".into(), ..good() },
            Profile { email: "ola@poczta".into(), ..good() },
            Profile { email: "o la@poczta.pl".into(), ..good() },
            Profile { email: "@poczta.pl".into(), ..good() },
            Profile { appearance: Appearance { shirt: 99, ..Appearance::default() }, ..good() },
        ];
        for c in cases {
            assert!(validate_profile(c.clone()).is_none(), "{c:?}");
        }
    }
}
