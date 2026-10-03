//! Authoritative game server: fixed 20 Hz tick, handshake, input processing,
//! room-based interest management and per-client snapshots.
//!
//! [`Server`] owns the whole world. Its methods are split by feature into the
//! submodules below, each one an `impl Server` block:
//!
//! | module      | what                                                     |
//! |-------------|----------------------------------------------------------|
//! | `session`   | datagrams, handshake, address migration, timeouts        |
//! | `movement`  | applying inputs, what walking around causes              |
//! | `interact`  | the E key (NPCs, desks, machines, spots...) and NPC events |
//! | `snapshot`  | interest management, snapshots, speech, periodic resends |
//! | `portal`    | the job portal at home, mail, online interview           |
//! | `company`   | founder, candidates, hiring, contracts, firing           |
//! | `items`     | items on the floor and in hands / pockets                |
//! | `day`       | game clock, going home, commuting, vehicles              |
//! | `doors`     | elevators and toilet stalls                              |
//! | `alarm`, `cleaning`, `police`, `shop`, `lunch`, `treats`, `board`, `spots`, `computers` | one feature each |
//!
//! Entity ids share one `u16` space: players `1..0xE000`, items on the
//! floor / laptops / vehicles / the tray `0xE000..0xF000`, NPCs from
//! `npc::NPC_ID_BASE` (0xF000).
//!
//! A new feature gets its own module: state as a field here (initialised in
//! [`Server::new`]), a `tick_*` method called from [`Server::tick`], packet
//! handlers dispatched from `session::handle_datagram`.

mod actions;
mod alarm;
mod board;
mod breath;
mod cleaning;
mod company;
mod computers;
mod contract;
mod day;
mod doors;
mod fight;
mod greetings;
mod hr;
mod interact;
mod items;
mod kitchen;
mod leave;
mod office;
mod positions;
pub use positions::lines as position_lines;
mod lunch;
mod media;
mod movement;
mod player;
mod police;
mod portal;
mod puddles;
mod save;
mod session;
mod shop;
mod snapshot;
mod spots;
mod stats;
mod supplies;
mod treats;
mod voice;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use crate::board::Meeting;
use crate::building::Building;
use crate::clock::Clock;
use crate::coffee::{self, Machine};
use crate::commute::Vehicle;
use crate::company::Company;
use crate::computer::{self, Computer, Messenger, Workstation};
use crate::elevator::{self, Elevator};
use crate::fire::{Alarm, Smoke};
use crate::inventory::Item;
use crate::kitchen::Kitchen;
use crate::lights::{self, Lights, Switch};
use crate::needs::{self, Spot};
use crate::net::{LinkConditions, Net};
use crate::npc::{self, Npc};
use crate::protocol::{self as proto, Packet};
use crate::recruitment::Recruitment;
use crate::security::PoliceCall;
use crate::shop::Shelf;
use crate::sim::Pos;
use crate::stalls::{self, Stall};
use crate::treats::Tray;
use crate::weather::Weather;

use cleaning::Cleaning;
use items::{Dropped, DROP_HANDLE_BASE};
use player::Player;
use puddles::Puddle;
use snapshot::Outgoing;
use stats::Stats;

pub use player::validate_profile;

pub const TICK_HZ: u32 = 20;
pub const TICK: Duration = Duration::from_millis(1000 / TICK_HZ as u64);
pub const DEFAULT_CLIENT_TIMEOUT: Duration = Duration::from_secs(5);
/// Max input steps applied per client per tick (3 expected at 60/20 Hz; the
/// slack absorbs jitter, the cap stops speed hacks).
pub const MAX_INPUTS_PER_TICK: usize = 6;
/// Inputs buffered beyond this are dropped (client running ahead / flooding).
pub const MAX_INPUT_QUEUE: usize = 30;

/// Set (Ctrl+C / SIGTERM) to save and stop at the next loop turn.
pub static STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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
    /// Put a tray of sweets in the chill room right away (dev / tests).
    pub treats_now: bool,
    /// Chance (%) that fruit from the bowl is stale.
    pub stale_fruit_percent: u32,
    /// Minute of the day the cleaner starts her round.
    pub cleaning_at: u32,
    /// ... plus a random 0..this many minutes, drawn each day (0 = exactly).
    pub cleaning_spread: u32,
    /// Everybody starts with a (paid) pack of cigarettes (dev).
    pub start_cigarettes: bool,
    /// SQLite save file (None = nothing is saved).
    pub save_path: Option<std::path::PathBuf>,
    /// Players without an account may join (dev, tests, bots).
    pub allow_guests: bool,
}

impl Config {
    /// Normal play on `bind`: the defaults of the `server` binary.
    pub fn new(bind: SocketAddr, recruitment: Recruitment) -> Config {
        Config {
            bind,
            link: LinkConditions::default(),
            max_players: 256,
            stats_every: Duration::from_secs(5),
            client_timeout: DEFAULT_CLIENT_TIMEOUT,
            start_access: 0,
            recruitment,
            skip_recruitment: false,
            start_employed: false,
            needs_speed: 1,
            start_minute: 8 * 60,
            time_scale: 1,
            weather: None,
            treats_now: false,
            stale_fruit_percent: crate::treats::STALE_FRUIT_PERCENT,
            cleaning_at: crate::cleaning::ROUND_AT,
            cleaning_spread: crate::cleaning::ROUND_SPREAD,
            start_cigarettes: false,
            save_path: None,
            allow_guests: true,
        }
    }
}

/// A speech bubble: `speaker` (a player or an NPC) says `text` to everyone
/// who can see them, and to `to` wherever they are.
#[derive(Debug, Clone)]
struct Say {
    speaker: u16,
    text: String,
    to: Option<u16>,
}

impl Say {
    fn new(speaker: u16, text: impl Into<String>) -> Say {
        Say { speaker, text: text.into(), to: None }
    }

    fn addressed(speaker: u16, text: impl Into<String>, to: u16) -> Say {
        Say { speaker, text: text.into(), to: Some(to) }
    }
}

pub struct Server {
    building: Building,
    npcs: Vec<Npc>,
    machines: Vec<Machine>,
    /// Items lying on the floor.
    dropped: Vec<Dropped>,
    /// Accident puddles, until 22:00.
    puddles: Vec<Puddle>,
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
    /// The TVs (channels) and the boombox's track (track, started on tick).
    screens: Vec<media::Screen>,
    music: Option<(u8, u32)>,
    media_dirty: bool,
    /// The first-aid cabinet, the storeroom, the key hook.
    supplies: supplies::Supplies,
    /// Players the cashier already asked about the hot dog (until they step away).
    cashier_asked: HashSet<u16>,
    /// Pani Wiesia: when she last greeted each player, and the next joke.
    porter_greeted: HashMap<u16, u32>,
    porter_joke: usize,
    /// The receptionist asked about lunch: player -> world day.
    lunch_asked: HashMap<u16, u32>,
    /// Pani Maria: when she last told each player something, when she may
    /// talk again at all, and the next story.
    maria_told: HashMap<u16, u32>,
    maria_next: u32,
    maria_story: usize,
    /// Some elevator was moving last tick (resend `Doors` when it starts/stops).
    lift_was_moving: bool,
    clock: Clock,
    /// Vehicles bringing people to work (and parked cars / bikes).
    vehicles: Vec<Vehicle>,
    weather: Weather,
    /// Board meetings (calendar).
    meetings: Vec<Meeting>,
    /// Open positions per job offer (our startup).
    /// Our startup's positions (job openings; the founder edits them).
    positions: Vec<crate::company::Position>,
    company: Company,
    /// Lunch orders (the app on the computer).
    lunch_orders: Vec<crate::lunch::Order>,
    /// Sweets on the chill-room table, and when the next trays come today.
    tray: Option<Tray>,
    treat_drops: Vec<u32>,
    /// Patrol cars called for shoplifters.
    police_calls: Vec<PoliceCall>,
    /// Cigarette smoke in the rooms, and the fire alarm it may set off.
    smoke: Smoke,
    alarm: Option<Alarm>,
    /// Lamps switched on, and where the switches are.
    lights: Lights,
    /// The chill-room kitchenette (mugs, dishwasher, fridge).
    kitchen: Option<Kitchen>,
    switches: Vec<Switch>,
    /// The cleaner's afternoon round.
    cleaning: Cleaning,
    next_crew_id: u16,
    next_officer_id: u16,
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
    /// Speech waiting to be sent at the end of the tick.
    says: Vec<Say>,
    /// Sounds this tick: (kind, floor, position), sent with the updates.
    sounds: Vec<(u8, u8, Pos)>,
    /// Task boards (per department) and work mail.
    boards: crate::tasks::Boards,
    post: crate::workmail::PostOffice,
    /// Saving (None = off) and what is known by nick of people not online.
    store: Option<crate::persist::Store>,
    offline: save::Offline,
    /// Save at the next tick (money / hiring changed).
    save_soon: bool,
    /// Accounts (tickets from the HTTPS login); None = guests only.
    auth: Option<crate::auth::Auth>,
    /// Packets queued this tick (kept to reuse the allocation).
    outbox: Vec<Outgoing>,
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
    /// Bind the socket and set up the world.
    ///
    /// # Errors
    /// When the socket can't be bound to `cfg.bind`.
    pub fn new(building: Building, cfg: Config) -> std::io::Result<Server> {
        let net = Net::bind(cfg.bind, cfg.link)?;
        let rooms_where = |pred: fn(&crate::map::RoomDef) -> bool| -> Vec<(u8, u16)> {
            building.active_floors().flat_map(|(f, m)| m.rooms.iter().filter(move |r| pred(r)).map(move |r| (f, r.id))).collect()
        };
        let board_room = rooms_where(|r| r.kind == "management").first().copied();
        let shop_rooms = rooms_where(|r| r.kind == "shop");
        let outdoor_rooms = rooms_where(|r| r.outdoor);
        let hiring = cfg.recruitment.offers.iter().filter(|o| o.hiring);
        let positions = positions::from_file(&cfg.recruitment);
        let company_name = hiring.clone().next().map_or("Startup Sim sp. z o.o.", |o| o.company.as_str()).to_string();
        let npcs = Npc::spawn_all(&building);
        let mut server = Server {
            cashier: npcs.iter().find(|n| n.role == npc::Role::Cashier).map(|n| n.id),
            cashier_asked: HashSet::new(),
            screens: media::find_screens(&building),
            music: None,
            media_dirty: false,
            supplies: supplies::Supplies::find(&building),
            porter_greeted: HashMap::new(),
            porter_joke: 0,
            lunch_asked: HashMap::new(),
            maria_told: HashMap::new(),
            maria_next: 0,
            maria_story: 0,
            npcs,
            machines: coffee::find_machines(&building),
            dropped: Vec::new(),
            puddles: Vec::new(),
            workstations: computer::find_workstations(&building),
            spots: needs::find_spots(&building),
            stalls: stalls::find_stalls(&building),
            elevators: elevator::find_elevators(&building),
            shelves: crate::shop::shelves(&building),
            shop_rooms,
            lift_was_moving: false,
            clock: Clock::new(cfg.start_minute, cfg.time_scale),
            vehicles: Vec::new(),
            weather: Weather::new(0),
            meetings: Vec::new(),
            positions,
            company: Company::new(&company_name),
            lunch_orders: Vec::new(),
            tray: None,
            treat_drops: Vec::new(),
            police_calls: Vec::new(),
            smoke: Smoke::new(&building),
            alarm: None,
            lights: Lights::default(),
            kitchen: Kitchen::find(&building),
            switches: lights::switches(&building),
            cleaning: Cleaning::default(),
            next_crew_id: 0,
            next_officer_id: 0,
            board_room,
            outdoor_rooms,
            clock_dirty: true,
            doors_dirty: false,
            computers: Vec::new(),
            messenger: Messenger::default(),
            next_item_id: 1,
            next_drop_handle: DROP_HANDLE_BASE,
            says: Vec::new(),
            sounds: Vec::new(),
            boards: Default::default(),
            post: Default::default(),
            store: None,
            offline: Default::default(),
            save_soon: false,
            auth: None,
            outbox: Vec::new(),
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
        server.load_save().map_err(std::io::Error::other)?;
        server.schedule_treats();
        server.ensure_media_items();
        if server.cfg.treats_now {
            server.put_tray();
        }
        Ok(server)
    }

    /// The address the socket is bound to.
    ///
    /// # Panics
    /// Never in practice: the socket is bound in [`Server::new`].
    pub fn local_addr(&self) -> SocketAddr {
        self.net.local_addr().expect("bound socket")
    }

    fn log(&self, msg: impl AsRef<str>) {
        println!("[{:>8.2}s] {}", self.started.elapsed().as_secs_f64(), msg.as_ref());
    }

    /// Run forever.
    pub fn run(&mut self) -> ! {
        let mut next_tick = Instant::now() + TICK;
        let mut next_stats = Instant::now() + self.cfg.stats_every;
        loop {
            let now = Instant::now();
            if now >= next_tick {
                let t0 = Instant::now();
                self.tick();
                self.stats.record_tick(t0.elapsed());
                next_tick += TICK;
                // Fell behind by more than a whole tick: skip, don't spiral.
                let late = Instant::now().saturating_duration_since(next_tick);
                if late >= TICK {
                    let skipped = u32::try_from(late.as_nanos() / TICK.as_nanos()).unwrap_or(u32::MAX);
                    self.stats.missed += u64::from(skipped);
                    self.tick = self.tick.wrapping_add(skipped);
                    next_tick += TICK.saturating_mul(skipped);
                }
            }
            if STOP.load(std::sync::atomic::Ordering::Relaxed) {
                self.shutdown();
                std::process::exit(0);
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

    /// Accounts: tickets from the HTTPS login let players in.
    pub fn set_auth(&mut self, auth: crate::auth::Auth) {
        self.auth = Some(auth);
    }

    /// One simulation step, in phases.
    fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        self.update_fast_forward();
        self.tick_clock();
        self.drop_timed_out(Instant::now());

        // Players walk; what they run into reacts.
        let mut steps = self.simulate_players();
        self.react_to_steps(&mut steps);
        self.smoke.tick(self.tick);
        self.tick_smoke_and_alarm();
        self.tick_portals(&steps.offline);

        // The E key, then the world's own behaviour.
        let mut events = self.handle_interactions(&steps.presses);
        self.check_computer_sessions();
        self.check_stalls();
        self.tick_elevators();
        self.tick_vehicles();
        self.tick_police();
        self.tick_cleaning();
        self.tick_cashier();
        self.tick_reception();
        self.tick_lunch_break();
        self.tick_maria();
        self.tick_to_portal();
        self.tick_media();
        self.tick_kitchen();
        self.tick_meetings();
        self.tick_lunch();
        self.tick_company();
        events.extend(self.tick_npcs());
        self.apply_npc_events(events);

        // Everything the clients need to know.
        self.send_updates();
        self.tick_save();
    }

    /// Encode and send one packet; returns its size.
    fn send(&mut self, addr: SocketAddr, p: &Packet) -> usize {
        let mut b = p.encode();
        debug_assert!(b.len() <= proto::MAX_PACKET);
        // A logged-in session: sealed with its key.
        if let Some(pl) = self.by_addr.get(&addr).and_then(|id| self.players.get_mut(id)) {
            if let Some(c) = pl.crypto.as_mut() {
                c.send_counter += 1;
                b = c.keys.seal(crate::crypto::Dir::ToClient, &crate::crypto::session_prefix(pl.token), c.send_counter, &b);
            }
        }
        let n = b.len();
        self.net.send(addr, b);
        n
    }

    /// Send a packet to a connected player (no-op if they are gone).
    fn send_to(&mut self, pid: u16, p: &Packet) {
        if let Some(addr) = self.players.get(&pid).map(|p| p.addr) {
            self.send(addr, p);
        }
    }

    /// A sound where the player stands (heard by those nearby).
    fn sound(&mut self, kind: u8, pid: u16) {
        if let Some(p) = self.players.get(&pid) {
            self.sounds.push((kind, p.body.floor, p.body.pos));
        }
    }

    /// A player's nick for logs and lines ("?" if they're gone).
    fn nick(&self, pid: u16) -> &str {
        self.players.get(&pid).map_or("?", |p| p.nick.as_str())
    }

    fn room_of(&self, floor: u8, pos: Pos) -> u16 {
        self.building.room_at(floor, pos)
    }

    /// A fresh item (nobody's, one piece, paid) with the next unique id.
    fn mint_item(&mut self, kind: u8, label: impl Into<String>) -> Item {
        let id = self.next_item_id;
        self.next_item_id = self.next_item_id.wrapping_add(1).max(1);
        Item { id, kind, label: label.into(), expires: None, owner: 0, count: 1, unpaid: false, stale: false, tainted: false, quality: 0 }
    }
}

/// Squared distance, saturating (positions are in sub-tile units).
fn dist2(a: Pos, b: Pos) -> i32 {
    let (dx, dy) = (a.x - b.x, a.y - b.y);
    dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
}
