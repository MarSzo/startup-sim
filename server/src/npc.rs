//! Server-side NPCs. They move with the same `sim::step` as players (driven
//! by `nav::Walker`), appear in snapshots as `kind::NPC` entities and talk via
//! `Packet::Say`.
//!
//! Onboarding (GDD section 4, "dzień próbny"):
//! - the **porter** gives a newcomer a guest pass and escorts them to the
//!   1st floor reception;
//! - the **receptionist** escorts guests to HR;
//! - **HR** signs the contract and swaps the guest pass for an employee card.
//!
//! The shop's **security guard** chases anybody leaving with unpaid goods;
//! a **police officer** (spawned by the server with a patrol car) chases
//! whoever the guard couldn't stop.

use std::collections::HashMap;

use crate::building::{Building, Place};
use crate::inventory::kind as item;
use crate::map::{access, NpcDef};
use crate::nav::Walker;
use crate::sim::{self, Body, Pos, SUBPIXELS};

/// NPC entity ids live above player ids.
pub const NPC_ID_BASE: u16 = 0xF000;
/// How close (sub-pixel units) a player must be to talk to an NPC: 3.5 tiles.
pub const TALK_RADIUS: i32 = 56 * SUBPIXELS;
/// The escorted guest may be this far from the NPC (or from the rest of its
/// route, if the guest runs ahead) before it stops to wait: 4 tiles.
const FOLLOW_RADIUS: i32 = 64 * SUBPIXELS;
/// Input steps per server tick (60 Hz input / 20 Hz tick).
const STEPS_PER_TICK: usize = 3;
/// An escort gives up waiting after this many ticks (30 s).
const GIVE_UP_TICKS: u32 = 600;
/// An escort reminds a lagging guest every this many ticks (6 s).
const NAG_TICKS: u32 = 120;
/// Chasing: a bit faster than a player (4 steps per tick instead of 3).
const CHASE_STEPS_PER_TICK: usize = 4;
/// Caught when this close (same floor): 1.5 tiles.
pub const CATCH_RADIUS: i32 = 24 * SUBPIXELS;
/// The chase path is re-planned this often (1 s).
const REPATH_TICKS: u32 = 20;
/// The guard gives up after 20 s, the police after 3 min.
const GUARD_GIVE_UP_TICKS: u32 = 400;
const POLICE_GIVE_UP_TICKS: u32 = 3600;

/// Appearance, sent in entity flags bits 3..5 (see PROTOCOL.md).
pub mod look {
    pub const PLAYER: u8 = 0;
    pub const PORTER: u8 = 1;
    pub const OFFICE: u8 = 2;
    pub const GUARD: u8 = 3;
    pub const POLICE: u8 = 4;
}

pub mod lines {
    // Porter
    pub const WELCOME_ESCORT: &str = "Dzień dobry! Pierwszy dzień? Zaprowadzę na recepcję — proszę za mną.";
    pub const HAS_PASS: &str = "Dzień dobry! Przepustka działa, zapraszam przez bramki.";
    pub const FOLLOW_ME: &str = "Proszę za mną!";
    pub const ON_THE_WAY: &str = "Idziemy, idziemy — to niedaleko.";
    pub const BUSY: &str = "Chwileczkę, właśnie kogoś prowadzę.";
    pub const BACK_SOON: &str = "Chwileczkę, zaraz będę na miejscu.";
    pub const ARRIVED: &str = "To recepcja — tutaj proszę się zgłosić. Przepustka gościa jest ważna do końca dnia.";
    pub const GAVE_UP: &str = "Nie mogę dłużej czekać — wracam na portiernię.";
    // Reception
    pub const RECEPTION_WELCOME: &str = "Witamy! Zaprowadzę do HR — tam podpisuje się umowę.";
    pub const RECEPTION_ON_THE_WAY: &str = "To tuż obok, proszę za mną.";
    pub const RECEPTION_BACK_SOON: &str = "Chwileczkę, zaraz wracam za ladę.";
    pub const RECEPTION_ARRIVED: &str = "To dział HR — tutaj podpisuje się umowę i odbiera kartę.";
    pub const RECEPTION_GAVE_UP: &str = "Wracam na recepcję — HR jest w pokoju obok.";
    pub const RECEPTION_HAS_CARD: &str = "Dzień dobry! Miłego dnia w pracy.";
    pub const NO_PASS: &str = "Najpierw proszę zgłosić się na portierni.";
    // HR
    pub const HR_SIGNED: &str = "Umowa podpisana — witamy w firmie! Oto karta pracownika, Twój laptop i 200 zł zaliczki na start.";
    pub fn hr_signed_in(department: &str) -> String {
        format!("Umowa podpisana — witamy w dziale {department}! Oto karta pracownika, Twój laptop i 200 zł zaliczki na start.")
    }
    pub const HR_HAS_CARD: &str = "Umowa już podpisana, karta działa. Powodzenia!";
    pub const HR_HANDS_FULL: &str = "Proszę odłożyć to, co masz w rękach — zaraz dostaniesz laptopa.";
    // Security / police
    pub const GUARD_HELLO: &str = "Dzień dobry. Płacimy przy kasie, prawda?";
    pub const GUARD_STOP: &str = "Stać! Ochrona! Proszę wrócić z towarem!";
    pub const GUARD_BUSY: &str = "Nie teraz — jestem w pościgu!";
    pub const POLICE_BUSY: &str = "Proszę się odsunąć, trwa interwencja.";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Porter,
    Receptionist,
    Hr,
    /// Shop till: E = pay for what you took off the shelves.
    Cashier,
    /// The board: meetings booked in the calendar.
    Ceo,
    CoFounder,
    /// Shop security: chases shoplifters.
    Guard,
    /// Comes by patrol car when called (not placed in the building).
    Police,
}

impl Role {
    fn parse(kind: &str) -> Option<Role> {
        match kind {
            "porter" => Some(Role::Porter),
            "receptionist" => Some(Role::Receptionist),
            "hr" => Some(Role::Hr),
            "cashier" => Some(Role::Cashier),
            "ceo" => Some(Role::Ceo),
            "cofounder" => Some(Role::CoFounder),
            "guard" => Some(Role::Guard),
            _ => None,
        }
    }

    fn look(self) -> u8 {
        match self {
            Role::Porter => look::PORTER,
            Role::Receptionist | Role::Hr | Role::Cashier | Role::Ceo | Role::CoFounder => look::OFFICE,
            Role::Guard => look::GUARD,
            Role::Police => look::POLICE,
        }
    }
}

/// Lines used by the shared escort behaviour.
struct EscortLines {
    on_the_way: &'static str,
    back_soon: &'static str,
    arrived: &'static str,
    gave_up: &'static str,
}

const PORTER_ESCORT: EscortLines =
    EscortLines { on_the_way: lines::ON_THE_WAY, back_soon: lines::BACK_SOON, arrived: lines::ARRIVED, gave_up: lines::GAVE_UP };
const RECEPTION_ESCORT: EscortLines = EscortLines {
    on_the_way: lines::RECEPTION_ON_THE_WAY,
    back_soon: lines::RECEPTION_BACK_SOON,
    arrived: lines::RECEPTION_ARRIVED,
    gave_up: lines::RECEPTION_GAVE_UP,
};

/// What an NPC wants the server to do this tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Speech bubble; `to` is the player addressed (also gets it if in another room).
    Say { npc: u16, text: String, to: Option<u16> },
    /// Hand an item (`inventory::kind`) to a player; the server labels it.
    Give { player: u16, item: u8 },
    /// Take back an item of this kind from a player (if they still have it).
    Take { player: u16, item: u8 },
    /// Employment contract signed (HR): the department becomes official.
    Contract { player: u16 },
    /// At the till: the server charges the unpaid goods and answers as `npc`.
    Checkout { npc: u16, player: u16 },
    /// Board member: the server runs the meeting (calendar) as `npc`.
    Meeting { npc: u16, player: u16 },
    /// A chase ended next to the player.
    Caught { npc: u16, player: u16 },
    /// A chase was given up (too long, or the player left the building).
    Escaped { npc: u16, player: u16 },
}

enum State {
    Idle,
    Escorting { guest: u16, walker: Walker, waited: u32 },
    Returning { walker: Walker },
    Chasing { target: u16, walker: Option<Walker>, ticks: u32 },
}

pub struct Npc {
    pub id: u16,
    pub name: String,
    pub role: Role,
    pub body: Body,
    pub room: u16,
    /// Facing / moving bits (as for players) + appearance in bits 3..5.
    pub flags: u8,
    home: Place,
    escort_to: Option<Place>,
    /// (floor, room) of `escort_to`: a guest already there counts as "with me".
    escort_room: Option<(u8, u16)>,
    state: State,
}

impl Npc {
    pub fn spawn_all(b: &Building) -> Vec<Npc> {
        b.npcs()
            .into_iter()
            .filter_map(|(floor, def)| Role::parse(&def.kind).map(|r| (floor, def, r)))
            .enumerate()
            .map(|(i, (floor, def, role))| Npc::new(b, NPC_ID_BASE + i as u16, floor, &def, role))
            .collect()
    }

    fn new(b: &Building, id: u16, floor: u8, def: &NpcDef, role: Role) -> Npc {
        let pos = Pos::tile_center(def.home.x, def.home.y);
        let mut body = Body::at(floor, pos);
        body.access = access::CARD; // staff: walks through the gates
        Npc {
            id,
            name: def.name.clone(),
            role,
            body,
            room: b.floor(floor).map_or(0, |m| m.room_at(pos.x, pos.y)),
            flags: role.look() << 3,
            home: (floor, def.home),
            escort_to: def.escort_to,
            escort_room: def.escort_to.and_then(|(f, t)| b.floor(f).map(|m| (f, m.room_at_tile(t.x, t.y)))),
            state: State::Idle,
        }
    }

    /// A police officer next to the patrol car at `pos` (floor 0); its
    /// "home" is the car.
    pub fn police(b: &Building, id: u16, pos: Pos) -> Npc {
        let def = NpcDef { kind: "police".into(), name: "Policja".into(), home: { let (x, y) = pos.tile(); crate::map::Tile { x, y } }, escort_to: None };
        let mut n = Npc::new(b, id, 0, &def, Role::Police);
        n.body.pos = pos;
        n.body.access = access::GUEST | access::CARD | access::SERVICE | access::BOARD;
        n
    }

    /// Run after `target` (shoplifter).
    pub fn chase(&mut self, target: u16) {
        self.state = State::Chasing { target, walker: None, ticks: 0 };
    }

    pub fn chasing(&self) -> Option<u16> {
        match self.state {
            State::Chasing { target, .. } => Some(target),
            _ => None,
        }
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.state, State::Idle)
    }

    pub fn escorting(&self) -> Option<u16> {
        match self.state {
            State::Escorting { guest, .. } => Some(guest),
            _ => None,
        }
    }

    /// Distance check for talking (same floor, within `TALK_RADIUS`).
    pub fn in_talk_range(&self, body: &Body) -> bool {
        body.floor == self.body.floor && dist2(body.pos, self.body.pos) <= TALK_RADIUS * TALK_RADIUS
    }

    fn escort_lines(&self) -> &'static EscortLines {
        match self.role {
            Role::Receptionist => &RECEPTION_ESCORT,
            _ => &PORTER_ESCORT,
        }
    }

    /// A player pressed E next to this NPC. `department`: the position the
    /// player was recruited for (HR puts it on the contract).
    pub fn interact(
        &mut self,
        b: &Building,
        player: u16,
        player_access: u8,
        department: Option<&str>,
        hands_free: bool,
    ) -> Vec<Event> {
        let say = |text: &str| Event::Say { npc: self.id, text: text.to_string(), to: Some(player) };
        let has_card = player_access & access::CARD != 0;
        let has_pass = player_access & access::GUEST != 0;
        if self.role == Role::Cashier {
            return vec![Event::Checkout { npc: self.id, player }];
        }
        if matches!(self.role, Role::Ceo | Role::CoFounder) {
            return vec![Event::Meeting { npc: self.id, player }];
        }
        if matches!(self.role, Role::Guard | Role::Police) {
            let line = match (self.role, self.chasing().is_some()) {
                (Role::Police, _) => lines::POLICE_BUSY,
                (_, true) => lines::GUARD_BUSY,
                _ => lines::GUARD_HELLO,
            };
            return vec![say(line)];
        }
        if self.role == Role::Hr {
            return if has_card {
                vec![say(lines::HR_HAS_CARD)]
            } else if has_pass && !hands_free {
                vec![say(lines::HR_HANDS_FULL)]
            } else if has_pass {
                // Contract signed: card + laptop; the card replaces the guest pass.
                let line = department.map_or_else(|| lines::HR_SIGNED.to_string(), lines::hr_signed_in);
                vec![
                    Event::Say { npc: self.id, text: line, to: Some(player) },
                    Event::Take { player, item: item::GUEST_PASS },
                    Event::Give { player, item: item::EMPLOYEE_CARD },
                    Event::Give { player, item: item::LAPTOP },
                    Event::Contract { player },
                ]
            } else {
                vec![say(lines::NO_PASS)]
            };
        }
        let l = self.escort_lines();
        match &self.state {
            State::Idle => {
                let (welcome, grant) = match self.role {
                    Role::Porter if has_card || has_pass => return vec![say(lines::HAS_PASS)],
                    Role::Porter => (lines::WELCOME_ESCORT, true),
                    Role::Receptionist if has_card => return vec![say(lines::RECEPTION_HAS_CARD)],
                    Role::Receptionist if !has_pass => return vec![say(lines::NO_PASS)],
                    _ => (lines::RECEPTION_WELCOME, false),
                };
                let Some(goal) = self.escort_to else { return vec![] };
                let Some(walker) = Walker::to(b, &self.body, goal) else { return vec![] };
                self.state = State::Escorting { guest: player, walker, waited: 0 };
                let mut ev = vec![say(welcome)];
                if grant {
                    ev.push(Event::Give { player, item: item::GUEST_PASS });
                }
                ev
            }
            State::Escorting { guest, .. } if *guest == player => vec![say(l.on_the_way)],
            State::Escorting { .. } => vec![say(lines::BUSY)],
            State::Returning { .. } => vec![say(l.back_soon)],
            State::Chasing { .. } => vec![],
        }
    }

    /// One server tick. `players` holds every connected player's body.
    pub fn tick(&mut self, b: &Building, players: &HashMap<u16, Body>) -> Vec<Event> {
        let mut events = Vec::new();
        let mut walk = false;
        let l = self.escort_lines();
        let role = self.role;
        match &mut self.state {
            State::Idle => {}
            State::Escorting { guest, waited, walker } => {
                let guest = *guest;
                match players.get(&guest) {
                    None => self.go_home(b), // guest left the game
                    Some(g) => {
                        let r2 = FOLLOW_RADIUS * FOLLOW_RADIUS;
                        let close = |f: u8, p: Pos| g.floor == f && dist2(g.pos, p) <= r2;
                        let in_goal_room = self.escort_room.is_some_and(|(f, r)| {
                            g.floor == f && b.floor(f).is_some_and(|m| m.room_at(g.pos.x, g.pos.y) == r)
                        });
                        let near = in_goal_room
                            || close(self.body.floor, self.body.pos)
                            || walker.remaining().iter().any(|&(f, t)| close(f, Pos::tile_center(t.x, t.y)));
                        if near {
                            *waited = 0;
                            walk = true;
                        } else {
                            *waited += 1;
                            if *waited >= GIVE_UP_TICKS {
                                events.push(Event::Say { npc: self.id, text: l.gave_up.into(), to: Some(guest) });
                                if role == Role::Porter {
                                    events.push(Event::Take { player: guest, item: item::GUEST_PASS });
                                }
                                self.go_home(b);
                            } else if *waited % NAG_TICKS == 0 {
                                events.push(Event::Say { npc: self.id, text: lines::FOLLOW_ME.into(), to: Some(guest) });
                            }
                        }
                    }
                }
            }
            State::Returning { .. } => walk = true,
            State::Chasing { target, walker, ticks } => {
                let target = *target;
                *ticks += 1;
                let give_up = if role == Role::Police { POLICE_GIVE_UP_TICKS } else { GUARD_GIVE_UP_TICKS };
                match players.get(&target) {
                    Some(t) if t.floor == self.body.floor && dist2(t.pos, self.body.pos) <= CATCH_RADIUS * CATCH_RADIUS => {
                        events.push(Event::Caught { npc: self.id, player: target });
                        self.go_home(b);
                    }
                    Some(t) if *ticks < give_up => {
                        if walker.as_ref().map_or(true, |w| w.done()) || *ticks % REPATH_TICKS == 1 {
                            let (tx, ty) = t.pos.tile();
                            if let Some(w) = Walker::to(b, &self.body, (t.floor, crate::map::Tile { x: tx, y: ty })) {
                                *walker = Some(w);
                            }
                        }
                        walk = true;
                    }
                    _ => {
                        events.push(Event::Escaped { npc: self.id, player: target });
                        self.go_home(b);
                    }
                }
            }
        }
        if walk {
            self.walk_steps(b);
            let finished = match &self.state {
                State::Escorting { walker, .. } | State::Returning { walker } => walker.done(),
                State::Idle | State::Chasing { .. } => false,
            };
            if finished {
                if let State::Escorting { guest, .. } = self.state {
                    events.push(Event::Say { npc: self.id, text: l.arrived.into(), to: Some(guest) });
                    self.go_home(b);
                } else {
                    self.state = State::Idle;
                    self.body.pos = Pos::tile_center(self.home.1.x, self.home.1.y);
                    self.set_flags(0, false); // face down, towards visitors
                }
            }
        } else {
            let facing = self.flags & 3;
            self.set_flags(facing, false);
        }
        self.room = b.floor(self.body.floor).map_or(0, |m| m.room_at(self.body.pos.x, self.body.pos.y));
        events
    }

    fn set_flags(&mut self, facing: u8, moving: bool) {
        self.flags = facing | (moving as u8) << 2 | self.role.look() << 3;
    }

    fn walk_steps(&mut self, b: &Building) {
        let (walker, steps) = match &mut self.state {
            State::Escorting { walker, .. } | State::Returning { walker } => (walker, STEPS_PER_TICK),
            State::Chasing { walker: Some(walker), .. } => (walker, CHASE_STEPS_PER_TICK),
            _ => return,
        };
        let mut moved = false;
        let mut facing = self.flags & 3;
        for _ in 0..steps {
            let input = walker.next_input(&self.body);
            if input == 0 {
                break;
            }
            let before = self.body.pos;
            self.body = sim::step(b, self.body, input);
            let (dx, dy) = sim::input_dir(input);
            facing = if dy > 0 { 0 } else if dy < 0 { 1 } else if dx < 0 { 2 } else { 3 };
            moved |= self.body.pos != before;
        }
        self.set_flags(facing, moved);
    }

    fn go_home(&mut self, b: &Building) {
        self.state = match Walker::to(b, &self.body, self.home) {
            Some(walker) => State::Returning { walker },
            None => State::Idle,
        };
    }
}

fn dist2(a: Pos, b: Pos) -> i32 {
    let (dx, dy) = (a.x - b.x, a.y - b.y);
    dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::default_building_path;

    fn setup() -> (Building, Npc) {
        let (b, mut all) = everyone();
        (b, all.remove(0))
    }

    fn everyone() -> (Building, Vec<Npc>) {
        let b = Building::load(&default_building_path()).unwrap();
        let npcs = Npc::spawn_all(&b);
        (b, npcs)
    }

    fn by_role(npcs: &mut [Npc], role: Role) -> &mut Npc {
        npcs.iter_mut().find(|n| n.role == role).unwrap()
    }

    fn says(events: &[Event]) -> Vec<&str> {
        events.iter().filter_map(|e| if let Event::Say { text, .. } = e { Some(text.as_str()) } else { None }).collect()
    }

    #[test]
    fn escorts_a_newcomer_to_reception_and_returns() {
        let (b, mut porter) = setup();
        let home = porter.body;
        let ev = porter.interact(&b, 7, 0, None, true);
        assert_eq!(says(&ev), vec![lines::WELCOME_ESCORT]);
        assert!(ev.contains(&Event::Give { player: 7, item: item::GUEST_PASS }));
        assert_eq!(porter.escorting(), Some(7));

        // The guest sticks to the porter (same floor, same spot).
        let mut arrived = None;
        for t in 0..3000 {
            let players = HashMap::from([(7u16, porter.body)]);
            let ev = porter.tick(&b, &players);
            if says(&ev).contains(&lines::ARRIVED) {
                arrived = Some(t);
                let m = b.floor(porter.body.floor).unwrap();
                assert_eq!(porter.body.floor, 1);
                assert_eq!(m.room_name(porter.room), "Recepcja");
                break;
            }
        }
        assert!(arrived.is_some(), "porter reached the reception");
        let mut back = false;
        for _ in 0..3000 {
            porter.tick(&b, &HashMap::new());
            if porter.is_idle() {
                back = true;
                break;
            }
        }
        assert!(back, "porter walked back");
        assert_eq!((porter.body.floor, porter.body.pos), (home.floor, home.pos));
    }

    #[test]
    fn keeps_walking_when_the_guest_runs_ahead() {
        let (b, mut porter) = setup();
        porter.interact(&b, 7, 0, None, true);
        // Guest already waiting at the reception (in a corner, off his route).
        let ahead = Body::at(1, Pos::tile_center(21, 15));
        let players = HashMap::from([(7u16, ahead)]);
        let arrived = (0..3000).any(|_| says(&porter.tick(&b, &players)).contains(&lines::ARRIVED));
        assert!(arrived);
    }

    #[test]
    fn waits_for_a_lagging_guest_then_gives_up() {
        let (b, mut porter) = setup();
        porter.interact(&b, 7, 0, None, true);
        let far_away = Body::at(0, Pos::tile_center(33, 35)); // stays outside
        let players = HashMap::from([(7u16, far_away)]);
        let start = porter.body.pos;
        let mut nags = 0;
        let mut gave_up = false;
        for _ in 0..GIVE_UP_TICKS {
            let ev = porter.tick(&b, &players);
            nags += says(&ev).iter().filter(|t| **t == lines::FOLLOW_ME).count();
            if says(&ev).contains(&lines::GAVE_UP) {
                assert!(ev.contains(&Event::Take { player: 7, item: item::GUEST_PASS }));
                gave_up = true;
            }
        }
        assert!(gave_up);
        assert_eq!(nags, (GIVE_UP_TICKS / NAG_TICKS - 1) as usize, "reminders every 6 s");
        assert!((porter.body.pos.x - start.x).abs() < 2 * 256, "didn't walk off without the guest");
    }

    #[test]
    fn busy_porter_and_visitors_with_a_pass() {
        let (b, mut porter) = setup();
        assert_eq!(says(&porter.interact(&b, 1, access::CARD, None, true)), vec![lines::HAS_PASS]);
        assert!(porter.is_idle());
        porter.interact(&b, 1, 0, None, true);
        assert_eq!(says(&porter.interact(&b, 2, 0, None, true)), vec![lines::BUSY]);
        assert_eq!(says(&porter.interact(&b, 1, access::GUEST, None, true)), vec![lines::ON_THE_WAY]);
    }

    #[test]
    fn guest_leaving_the_game_sends_porter_home() {
        let (b, mut porter) = setup();
        porter.interact(&b, 7, 0, None, true);
        porter.tick(&b, &HashMap::new()); // guest gone
        assert_eq!(porter.escorting(), None);
        assert_eq!(says(&porter.interact(&b, 8, 0, None, true)), vec![lines::BACK_SOON]);
    }

    #[test]
    fn talking_needs_to_be_close() {
        let (_, porter) = setup();
        let at_door = Body::at(0, Pos::tile_center(27, 29)); // lobby, next to the lodge door
        assert!(porter.in_talk_range(&at_door));
        assert!(!porter.in_talk_range(&Body::at(0, Pos::tile_center(33, 29))));
        assert!(!porter.in_talk_range(&Body::at(1, porter.body.pos)), "other floor");
    }

    #[test]
    fn the_guard_catches_a_runner_or_gives_up() {
        let (b, mut npcs) = everyone();
        let guard = by_role(&mut npcs, Role::Guard);
        let home = guard.body;
        // A thief standing in the lobby: caught.
        let thief = Body::at(0, Pos::tile_center(34, 29));
        guard.chase(9);
        let mut caught = false;
        for _ in 0..400 {
            let ev = guard.tick(&b, &HashMap::from([(9u16, thief)]));
            if ev.contains(&Event::Caught { npc: guard.id, player: 9 }) {
                caught = true;
                break;
            }
        }
        assert!(caught, "the guard reached the thief");
        assert!(guard.chasing().is_none());
        for _ in 0..2000 {
            guard.tick(&b, &HashMap::new());
        }
        assert!(guard.is_idle() && guard.body.pos == home.pos, "back at the shop door");
        // Out of the game (not in the world): gives up at once.
        guard.chase(9);
        let ev = guard.tick(&b, &HashMap::new());
        assert!(ev.contains(&Event::Escaped { npc: guard.id, player: 9 }));
        // Somewhere the guard can't follow (the server room): gives up.
        let far = Body::at(0, Pos::tile_center(44, 6));
        guard.chase(9);
        let mut escaped = false;
        for _ in 0..GUARD_GIVE_UP_TICKS + 5 {
            if guard.tick(&b, &HashMap::from([(9u16, far)])).contains(&Event::Escaped { npc: guard.id, player: 9 }) {
                escaped = true;
                break;
            }
        }
        assert!(escaped);
    }

    #[test]
    fn spawns_the_staff_with_looks() {
        let (_, npcs) = everyone();
        let roles: Vec<(Role, &str, u8)> = npcs.iter().map(|n| (n.role, n.name.as_str(), n.flags >> 3)).collect();
        assert_eq!(
            roles,
            vec![
                (Role::Porter, "Portier", look::PORTER),
                (Role::Cashier, "Kasa", look::OFFICE),
                (Role::Guard, "Ochrona", look::GUARD),
                (Role::Receptionist, "Recepcja", look::OFFICE),
                (Role::Hr, "HR", look::OFFICE),
                (Role::Ceo, "Prezes", look::OFFICE),
                (Role::CoFounder, "Wspólniczka", look::OFFICE)
            ]
        );
        let ids: Vec<u16> = npcs.iter().map(|n| n.id).collect();
        assert_eq!(ids, (0..7).map(|i| NPC_ID_BASE + i).collect::<Vec<_>>());
    }

    #[test]
    fn guest_arriving_with_the_porter_can_talk_to_reception_and_hr_from_the_drop_off_spots() {
        let (b, mut npcs) = everyone();
        let porter_drop = Body::at(1, Pos::tile_center(32, 18));
        assert!(by_role(&mut npcs, Role::Receptionist).in_talk_range(&porter_drop));
        let reception_drop = Body::at(1, Pos::tile_center(43, 8));
        assert!(by_role(&mut npcs, Role::Hr).in_talk_range(&reception_drop));
        let m = b.floor(1).unwrap();
        assert_eq!(m.room_name(m.room_at_tile(43, 8)), "HR");
    }

    #[test]
    fn receptionist_escorts_guests_to_hr() {
        let (b, mut npcs) = everyone();
        let r = by_role(&mut npcs, Role::Receptionist);
        assert_eq!(says(&r.interact(&b, 1, 0, None, true)), vec![lines::NO_PASS]);
        assert_eq!(says(&r.interact(&b, 1, access::CARD, None, true)), vec![lines::RECEPTION_HAS_CARD]);
        let ev = r.interact(&b, 1, access::GUEST, None, true);
        assert_eq!(ev, vec![Event::Say { npc: r.id, text: lines::RECEPTION_WELCOME.into(), to: Some(1) }], "no pass changes");
        let arrived = (0..2000).any(|_| {
            let players = HashMap::from([(1u16, r.body)]);
            says(&r.tick(&b, &players)).contains(&lines::RECEPTION_ARRIVED)
        });
        assert!(arrived);
        let m = b.floor(1).unwrap();
        assert_eq!(m.room_name(r.room), "HR");
    }

    #[test]
    fn receptionist_giving_up_keeps_the_pass() {
        let (b, mut npcs) = everyone();
        let r = by_role(&mut npcs, Role::Receptionist);
        r.interact(&b, 1, access::GUEST, None, true);
        let players = HashMap::from([(1u16, Body::at(0, Pos::tile_center(33, 35)))]);
        let ev: Vec<Event> = (0..GIVE_UP_TICKS).flat_map(|_| r.tick(&b, &players)).collect();
        assert!(says(&ev).contains(&lines::RECEPTION_GAVE_UP));
        assert!(!ev.iter().any(|e| matches!(e, Event::Take { .. })));
    }

    #[test]
    fn hr_swaps_the_guest_pass_for_a_card() {
        let (b, mut npcs) = everyone();
        let hr = by_role(&mut npcs, Role::Hr);
        assert_eq!(says(&hr.interact(&b, 1, 0, None, true)), vec![lines::NO_PASS]);
        assert_eq!(says(&hr.interact(&b, 1, access::GUEST, None, false)), vec![lines::HR_HANDS_FULL], "laptop needs free hands");
        let ev = hr.interact(&b, 1, access::GUEST, Some("IT / Produkt"), true);
        assert_eq!(says(&ev), vec!["Umowa podpisana — witamy w dziale IT / Produkt! Oto karta pracownika, Twój laptop i 200 zł zaliczki na start."]);
        assert!(ev.contains(&Event::Contract { player: 1 }));
        assert!(ev.contains(&Event::Give { player: 1, item: item::EMPLOYEE_CARD }));
        assert!(ev.contains(&Event::Give { player: 1, item: item::LAPTOP }));
        assert!(ev.contains(&Event::Take { player: 1, item: item::GUEST_PASS }));
        assert_eq!(says(&hr.interact(&b, 1, access::CARD, None, true)), vec![lines::HR_HAS_CARD]);
        assert!(hr.is_idle(), "HR stays at the desk");
    }
}
