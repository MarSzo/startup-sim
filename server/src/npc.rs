//! Server-side NPCs. They move with the same `sim::step` as players (driven
//! by `nav::Walker`), appear in snapshots as `kind::NPC` entities and talk via
//! `Packet::Say`.
//!
//! Onboarding (GDD section 4, "dzień próbny"):
//! - the **porter** gives a newcomer a guest pass and escorts them to the
//!   1st floor reception;
//! - the **receptionist** escorts guests to HR;
//! - **HR** signs the contract and swaps the guest pass for an employee card.

use std::collections::HashMap;

use crate::building::{Building, Place};
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

/// Appearance, sent in entity flags bits 3..5 (see PROTOCOL.md).
pub mod look {
    pub const PLAYER: u8 = 0;
    pub const PORTER: u8 = 1;
    pub const OFFICE: u8 = 2;
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
    pub const HR_SIGNED: &str = "Umowa podpisana — witamy w firmie! Oto karta pracownika.";
    pub fn hr_signed_in(department: &str) -> String {
        format!("Umowa podpisana — witamy w dziale {department}! Oto karta pracownika.")
    }
    pub const HR_HAS_CARD: &str = "Umowa już podpisana, karta działa. Powodzenia!";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Porter,
    Receptionist,
    Hr,
}

impl Role {
    fn parse(kind: &str) -> Option<Role> {
        match kind {
            "porter" => Some(Role::Porter),
            "receptionist" => Some(Role::Receptionist),
            "hr" => Some(Role::Hr),
            _ => None,
        }
    }

    fn look(self) -> u8 {
        match self {
            Role::Porter => look::PORTER,
            Role::Receptionist | Role::Hr => look::OFFICE,
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
    Grant { player: u16, access: u8 },
    Revoke { player: u16, access: u8 },
    /// Employment contract signed (HR): the department becomes official.
    Contract { player: u16 },
}

enum State {
    Idle,
    Escorting { guest: u16, walker: Walker, waited: u32 },
    Returning { walker: Walker },
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
    pub fn interact(&mut self, b: &Building, player: u16, player_access: u8, department: Option<&str>) -> Vec<Event> {
        let say = |text: &str| Event::Say { npc: self.id, text: text.to_string(), to: Some(player) };
        let has_card = player_access & access::CARD != 0;
        let has_pass = player_access & access::GUEST != 0;
        if self.role == Role::Hr {
            return if has_card {
                vec![say(lines::HR_HAS_CARD)]
            } else if has_pass {
                // Contract signed: the card replaces the guest pass.
                let line = department.map_or_else(|| lines::HR_SIGNED.to_string(), lines::hr_signed_in);
                vec![
                    Event::Say { npc: self.id, text: line, to: Some(player) },
                    Event::Grant { player, access: access::CARD },
                    Event::Revoke { player, access: access::GUEST },
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
                    Role::Porter => (lines::WELCOME_ESCORT, Some(access::GUEST)),
                    Role::Receptionist if has_card => return vec![say(lines::RECEPTION_HAS_CARD)],
                    Role::Receptionist if !has_pass => return vec![say(lines::NO_PASS)],
                    _ => (lines::RECEPTION_WELCOME, None),
                };
                let Some(goal) = self.escort_to else { return vec![] };
                let Some(walker) = Walker::to(b, &self.body, goal) else { return vec![] };
                self.state = State::Escorting { guest: player, walker, waited: 0 };
                let mut ev = vec![say(welcome)];
                if let Some(a) = grant {
                    ev.push(Event::Grant { player, access: a });
                }
                ev
            }
            State::Escorting { guest, .. } if *guest == player => vec![say(l.on_the_way)],
            State::Escorting { .. } => vec![say(lines::BUSY)],
            State::Returning { .. } => vec![say(l.back_soon)],
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
                                    events.push(Event::Revoke { player: guest, access: access::GUEST });
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
        }
        if walk {
            self.walk_steps(b);
            let finished = match &self.state {
                State::Escorting { walker, .. } | State::Returning { walker } => walker.done(),
                State::Idle => false,
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
        let (State::Escorting { walker, .. } | State::Returning { walker }) = &mut self.state else { return };
        let mut moved = false;
        let mut facing = self.flags & 3;
        for _ in 0..STEPS_PER_TICK {
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
        let ev = porter.interact(&b, 7, 0, None);
        assert_eq!(says(&ev), vec![lines::WELCOME_ESCORT]);
        assert!(ev.contains(&Event::Grant { player: 7, access: access::GUEST }));
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
        porter.interact(&b, 7, 0, None);
        // Guest already waiting at the reception (in a corner, off his route).
        let ahead = Body::at(1, Pos::tile_center(21, 15));
        let players = HashMap::from([(7u16, ahead)]);
        let arrived = (0..3000).any(|_| says(&porter.tick(&b, &players)).contains(&lines::ARRIVED));
        assert!(arrived);
    }

    #[test]
    fn waits_for_a_lagging_guest_then_gives_up() {
        let (b, mut porter) = setup();
        porter.interact(&b, 7, 0, None);
        let far_away = Body::at(0, Pos::tile_center(33, 35)); // stays outside
        let players = HashMap::from([(7u16, far_away)]);
        let start = porter.body.pos;
        let mut nags = 0;
        let mut gave_up = false;
        for _ in 0..GIVE_UP_TICKS {
            let ev = porter.tick(&b, &players);
            nags += says(&ev).iter().filter(|t| **t == lines::FOLLOW_ME).count();
            if says(&ev).contains(&lines::GAVE_UP) {
                assert!(ev.contains(&Event::Revoke { player: 7, access: access::GUEST }));
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
        assert_eq!(says(&porter.interact(&b, 1, access::CARD, None)), vec![lines::HAS_PASS]);
        assert!(porter.is_idle());
        porter.interact(&b, 1, 0, None);
        assert_eq!(says(&porter.interact(&b, 2, 0, None)), vec![lines::BUSY]);
        assert_eq!(says(&porter.interact(&b, 1, access::GUEST, None)), vec![lines::ON_THE_WAY]);
    }

    #[test]
    fn guest_leaving_the_game_sends_porter_home() {
        let (b, mut porter) = setup();
        porter.interact(&b, 7, 0, None);
        porter.tick(&b, &HashMap::new()); // guest gone
        assert_eq!(porter.escorting(), None);
        assert_eq!(says(&porter.interact(&b, 8, 0, None)), vec![lines::BACK_SOON]);
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
    fn spawns_three_staff_with_looks() {
        let (_, npcs) = everyone();
        let roles: Vec<(Role, &str, u8)> = npcs.iter().map(|n| (n.role, n.name.as_str(), n.flags >> 3)).collect();
        assert_eq!(
            roles,
            vec![(Role::Porter, "Portier", look::PORTER), (Role::Receptionist, "Recepcja", look::OFFICE), (Role::Hr, "HR", look::OFFICE)]
        );
        let ids: Vec<u16> = npcs.iter().map(|n| n.id).collect();
        assert_eq!(ids, vec![NPC_ID_BASE, NPC_ID_BASE + 1, NPC_ID_BASE + 2]);
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
        assert_eq!(says(&r.interact(&b, 1, 0, None)), vec![lines::NO_PASS]);
        assert_eq!(says(&r.interact(&b, 1, access::CARD, None)), vec![lines::RECEPTION_HAS_CARD]);
        let ev = r.interact(&b, 1, access::GUEST, None);
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
        r.interact(&b, 1, access::GUEST, None);
        let players = HashMap::from([(1u16, Body::at(0, Pos::tile_center(33, 35)))]);
        let ev: Vec<Event> = (0..GIVE_UP_TICKS).flat_map(|_| r.tick(&b, &players)).collect();
        assert!(says(&ev).contains(&lines::RECEPTION_GAVE_UP));
        assert!(!ev.iter().any(|e| matches!(e, Event::Revoke { .. })));
    }

    #[test]
    fn hr_swaps_the_guest_pass_for_a_card() {
        let (b, mut npcs) = everyone();
        let hr = by_role(&mut npcs, Role::Hr);
        assert_eq!(says(&hr.interact(&b, 1, 0, None)), vec![lines::NO_PASS]);
        let ev = hr.interact(&b, 1, access::GUEST, Some("IT / Produkt"));
        assert_eq!(says(&ev), vec!["Umowa podpisana — witamy w dziale IT / Produkt! Oto karta pracownika."]);
        assert!(ev.contains(&Event::Contract { player: 1 }));
        assert!(ev.contains(&Event::Grant { player: 1, access: access::CARD }));
        assert!(ev.contains(&Event::Revoke { player: 1, access: access::GUEST }));
        assert_eq!(says(&hr.interact(&b, 1, access::CARD, None)), vec![lines::HR_HAS_CARD]);
        assert!(hr.is_idle(), "HR stays at the desk");
    }
}
