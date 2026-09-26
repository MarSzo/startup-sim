//! Server-side NPCs. They move with the same `sim::step` as players (driven
//! by `nav::Walker`), appear in snapshots as `kind::NPC` entities and talk via
//! `Packet::Say`.
//!
//! Only the porter exists so far: a newcomer without a pass talks to him (E)
//! and he escorts them to the 1st floor reception, handing out a guest pass
//! (GDD section 4: "dzień próbny").

use std::collections::HashMap;

use crate::building::{Building, Place};
use crate::map::{access, NpcDef};
use crate::nav::Walker;
use crate::sim::{self, Body, Pos, SUBPIXELS};

/// NPC entity ids live above player ids.
pub const NPC_ID_BASE: u16 = 0xF000;
/// How close (sub-pixel units) a player must be to talk to an NPC: 3.5 tiles.
pub const TALK_RADIUS: i32 = 56 * SUBPIXELS;
/// The escorted guest may be this far from the porter (or from the rest of
/// his route, if the guest runs ahead) before he stops to wait: 4 tiles.
const FOLLOW_RADIUS: i32 = 64 * SUBPIXELS;
/// Input steps per server tick (60 Hz input / 20 Hz tick).
const STEPS_PER_TICK: usize = 3;
/// Porter gives up waiting after this many ticks (30 s).
const GIVE_UP_TICKS: u32 = 600;
/// Porter reminds a lagging guest every this many ticks (6 s).
const NAG_TICKS: u32 = 120;

pub mod lines {
    pub const WELCOME_ESCORT: &str = "Dzień dobry! Pierwszy dzień? Zaprowadzę na recepcję — proszę za mną.";
    pub const HAS_PASS: &str = "Dzień dobry! Przepustka działa, zapraszam przez bramki.";
    pub const FOLLOW_ME: &str = "Proszę za mną!";
    pub const ON_THE_WAY: &str = "Idziemy, idziemy — to niedaleko.";
    pub const BUSY: &str = "Chwileczkę, właśnie kogoś prowadzę.";
    pub const BACK_SOON: &str = "Chwileczkę, zaraz będę na miejscu.";
    pub const ARRIVED: &str = "To recepcja — tutaj proszę się zgłosić. Przepustka gościa jest ważna do końca dnia.";
    pub const GAVE_UP: &str = "Nie mogę dłużej czekać — wracam na portiernię.";
}

/// What an NPC wants the server to do this tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Speech bubble; `to` is the player addressed (also gets it if in another room).
    Say { npc: u16, text: &'static str, to: Option<u16> },
    Grant { player: u16, access: u8 },
    Revoke { player: u16, access: u8 },
}

enum State {
    Idle,
    Escorting { guest: u16, walker: Walker, waited: u32 },
    Returning { walker: Walker },
}

pub struct Npc {
    pub id: u16,
    pub name: String,
    pub body: Body,
    pub room: u16,
    /// Facing / moving bits, same meaning as for players.
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
            .enumerate()
            .map(|(i, (floor, def))| Npc::new(b, NPC_ID_BASE + i as u16, floor, &def))
            .collect()
    }

    fn new(b: &Building, id: u16, floor: u8, def: &NpcDef) -> Npc {
        let pos = Pos::tile_center(def.home.x, def.home.y);
        let mut body = Body::at(floor, pos);
        body.access = access::CARD; // staff: the porter walks through the gates
        Npc {
            id,
            name: def.name.clone(),
            body,
            room: b.floor(floor).map_or(0, |m| m.room_at(pos.x, pos.y)),
            flags: 0,
            home: (floor, def.home),
            escort_to: def.escort_to,
            escort_room: def
                .escort_to
                .and_then(|(f, t)| b.floor(f).map(|m| (f, m.room_at_tile(t.x, t.y)))),
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

    /// A player pressed E next to this NPC.
    pub fn interact(&mut self, b: &Building, player: u16, player_access: u8) -> Vec<Event> {
        let say = |text| Event::Say { npc: self.id, text, to: Some(player) };
        match &self.state {
            State::Idle => {
                if player_access & (access::GUEST | access::CARD) != 0 {
                    return vec![say(lines::HAS_PASS)];
                }
                let Some(goal) = self.escort_to else { return vec![] };
                let Some(walker) = Walker::to(b, &self.body, goal) else { return vec![] };
                self.state = State::Escorting { guest: player, walker, waited: 0 };
                vec![say(lines::WELCOME_ESCORT), Event::Grant { player, access: access::GUEST }]
            }
            State::Escorting { guest, .. } if *guest == player => vec![say(lines::ON_THE_WAY)],
            State::Escorting { .. } => vec![say(lines::BUSY)],
            State::Returning { .. } => vec![say(lines::BACK_SOON)],
        }
    }

    /// One server tick. `players` holds every connected player's body.
    pub fn tick(&mut self, b: &Building, players: &HashMap<u16, Body>) -> Vec<Event> {
        let mut events = Vec::new();
        let mut walk = None;
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
                            walk = Some(());
                        } else {
                            *waited += 1;
                            if *waited >= GIVE_UP_TICKS {
                                events.push(Event::Say { npc: self.id, text: lines::GAVE_UP, to: Some(guest) });
                                events.push(Event::Revoke { player: guest, access: access::GUEST });
                                self.go_home(b);
                            } else if *waited % NAG_TICKS == 0 {
                                events.push(Event::Say { npc: self.id, text: lines::FOLLOW_ME, to: Some(guest) });
                            }
                        }
                    }
                }
            }
            State::Returning { .. } => walk = Some(()),
        }
        if walk.is_some() {
            self.walk_steps(b);
            let finished = match &self.state {
                State::Escorting { walker, .. } | State::Returning { walker } => walker.done(),
                State::Idle => false,
            };
            if finished {
                if let State::Escorting { guest, .. } = self.state {
                    events.push(Event::Say { npc: self.id, text: lines::ARRIVED, to: Some(guest) });
                    self.go_home(b);
                } else {
                    self.state = State::Idle;
                    self.body.pos = Pos::tile_center(self.home.1.x, self.home.1.y);
                    self.flags = 0; // face down at the desk
                }
            }
        } else {
            self.flags &= 3; // standing
        }
        self.room = b.floor(self.body.floor).map_or(0, |m| m.room_at(self.body.pos.x, self.body.pos.y));
        events
    }

    fn walk_steps(&mut self, b: &Building) {
        let (State::Escorting { walker, .. } | State::Returning { walker }) = &mut self.state else { return };
        let mut moved = false;
        for _ in 0..STEPS_PER_TICK {
            let input = walker.next_input(&self.body);
            if input == 0 {
                break;
            }
            let before = self.body.pos;
            self.body = sim::step(b, self.body, input);
            let (dx, dy) = sim::input_dir(input);
            self.flags = if dy > 0 { 0 } else if dy < 0 { 1 } else if dx < 0 { 2 } else { 3 };
            moved |= self.body.pos != before;
        }
        if moved {
            self.flags |= 0b100;
        }
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
        let b = Building::load(&default_building_path()).unwrap();
        let porter = Npc::spawn_all(&b).remove(0);
        (b, porter)
    }

    fn says(events: &[Event]) -> Vec<&'static str> {
        events.iter().filter_map(|e| if let Event::Say { text, .. } = e { Some(*text) } else { None }).collect()
    }

    #[test]
    fn escorts_a_newcomer_to_reception_and_returns() {
        let (b, mut porter) = setup();
        let home = porter.body;
        let ev = porter.interact(&b, 7, 0);
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
        porter.interact(&b, 7, 0);
        // Guest already waiting at the reception (in a corner, off his route).
        let ahead = Body::at(1, Pos::tile_center(21, 15));
        let players = HashMap::from([(7u16, ahead)]);
        let arrived = (0..3000).any(|_| says(&porter.tick(&b, &players)).contains(&lines::ARRIVED));
        assert!(arrived);
    }

    #[test]
    fn waits_for_a_lagging_guest_then_gives_up() {
        let (b, mut porter) = setup();
        porter.interact(&b, 7, 0);
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
        assert_eq!(says(&porter.interact(&b, 1, access::CARD)), vec![lines::HAS_PASS]);
        assert!(porter.is_idle());
        porter.interact(&b, 1, 0);
        assert_eq!(says(&porter.interact(&b, 2, 0)), vec![lines::BUSY]);
        assert_eq!(says(&porter.interact(&b, 1, access::GUEST)), vec![lines::ON_THE_WAY]);
    }

    #[test]
    fn guest_leaving_the_game_sends_porter_home() {
        let (b, mut porter) = setup();
        porter.interact(&b, 7, 0);
        porter.tick(&b, &HashMap::new()); // guest gone
        assert_eq!(porter.escorting(), None);
        assert_eq!(says(&porter.interact(&b, 8, 0)), vec![lines::BACK_SOON]);
    }

    #[test]
    fn talking_needs_to_be_close() {
        let (_, porter) = setup();
        let at_door = Body::at(0, Pos::tile_center(27, 29)); // lobby, next to the lodge door
        assert!(porter.in_talk_range(&at_door));
        assert!(!porter.in_talk_range(&Body::at(0, Pos::tile_center(33, 29))));
        assert!(!porter.in_talk_range(&Body::at(1, porter.body.pos)), "other floor");
    }
}
