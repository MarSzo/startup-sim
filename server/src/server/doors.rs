//! Doors that change: elevators and toilet stalls.

use crate::elevator;
use crate::protocol::{self as proto, Packet};
use crate::sim::{Body, Pos};
use crate::stalls;

use super::player::LiftPanel;
use super::{Say, Server};

/// What the panel in the cabin asks.
pub const PANEL_TEXT: &str = "Które piętro?";
/// The last button: close the panel, stay in the cabin.
pub const PANEL_STAY: &str = "Zostań";
/// Most floor buttons on the panel (a `Dialog` has up to 4 options).
const PANEL_BUTTONS: usize = proto::MAX_OPTIONS - 1;

impl Server {
    /// Elevator doors: closed (solid) unless the car stands there open.
    pub(super) fn sync_elevator_doors(&mut self) {
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

    /// E at the elevator: in the cabin the panel of floor buttons opens (a
    /// `Dialog`); at the doors it is the call button. `false` = no elevator
    /// here.
    pub(super) fn use_elevator(&mut self, pid: u16, body: &Body) -> bool {
        let tick = self.tick;
        if let Some(i) = self.elevators.iter().position(|e| e.in_cabin(body.floor, body.pos)) {
            let floors: Vec<u8> = self.elevators[i].buttons(body.floor).into_iter().take(PANEL_BUTTONS).collect();
            if floors.is_empty() {
                self.says.push(Say::new(pid, elevator::lines::RIDING));
                return true;
            }
            let Some(p) = self.players.get_mut(&pid) else { return true };
            let id = p.next_dialog();
            p.lift_panel = Some(LiftPanel { lift: i, floor: body.floor, id, floors });
            self.send_dialog(pid);
            return true;
        }
        // Between two lifts: the call button of the nearer one.
        let Some(i) = (0..self.elevators.len())
            .filter_map(|i| self.elevators[i].door_distance(body).map(|d| (i, d)))
            .min_by_key(|&(_, d)| d)
            .map(|(i, _)| i)
        else {
            return false;
        };
        // The call button needs the card, like the doors.
        if body.access & crate::map::access::required("card").unwrap_or(0) == 0 {
            self.says.push(Say::new(pid, elevator::lines::NO_CARD));
            return true;
        }
        self.doors_dirty = true; // show where the car is heading at once
        let line = self.elevators[i].call(body.floor, tick);
        self.says.push(Say::new(pid, line));
        true
    }

    /// The open panel as a `Dialog` (npc 0): a button per floor, then "stay".
    pub(super) fn lift_panel_packet(&self, panel: &LiftPanel) -> Packet {
        let mut options: Vec<String> = panel.floors.iter().map(|&f| self.building.floor_name(f).to_string()).collect();
        options.push(PANEL_STAY.to_string());
        Packet::Dialog { id: panel.id, npc: 0, text: PANEL_TEXT.to_string(), options, items: Vec::new() }
    }

    /// A button on the panel pressed (`DialogAnswer` to the panel's id).
    pub(super) fn press_lift_panel(&mut self, pid: u16, panel: &LiftPanel, choice: u8) {
        self.close_lift_panel(pid);
        let Some(&target) = panel.floors.get(usize::from(choice)) else { return }; // "stay"
        let Some(e) = self.elevators.get_mut(panel.lift) else { return };
        let line = match e.press_floor(panel.floor, target, self.tick) {
            Some(t) => format!("Jedziemy na: {}.", self.building.floor_name(t)),
            None => elevator::lines::RIDING.to_string(),
        };
        self.doors_dirty = true;
        self.says.push(Say::new(pid, line));
    }

    /// Close the panel (the client hides it on id 0).
    pub(super) fn close_lift_panel(&mut self, pid: u16) {
        if self.players.get_mut(&pid).and_then(|p| p.lift_panel.take()).is_none() {
            return;
        }
        let close = Packet::Dialog { id: 0, npc: 0, text: String::new(), options: Vec::new(), items: Vec::new() };
        self.send_to(pid, &close);
        self.send_to(pid, &close); // tiny packet; a duplicate makes loss unlikely
    }

    /// Panels of people who left the cabin, or whose car set off (somebody
    /// else pressed a button), close by themselves.
    fn close_stale_lift_panels(&mut self) {
        let stale: Vec<u16> = self
            .players
            .values()
            .filter(|p| {
                p.lift_panel.as_ref().is_some_and(|l| {
                    !self
                        .elevators
                        .get(l.lift)
                        .is_some_and(|e| !e.buttons(l.floor).is_empty() && p.body.floor == l.floor && e.in_cabin(l.floor, p.body.pos))
                })
            })
            .map(|p| p.id)
            .collect();
        for pid in stale {
            self.close_lift_panel(pid);
        }
    }

    /// Move the elevators; carry the people in a cabin that arrived.
    pub(super) fn tick_elevators(&mut self) {
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
                    self.says.push(Say::new(s, elevator::lines::OVERLOAD));
                }
            }
            if let Some((from, to)) = up.arrived {
                let e = &self.elevators[i];
                let mut riders = Vec::new();
                for p in self.players.values_mut() {
                    if p.in_building() && e.in_cabin(from, p.body.pos) && p.body.floor == from {
                        p.body.floor = to;
                        p.room = self.building.floor(to).map_or(0, |m| m.room_at(p.body.pos.x, p.body.pos.y));
                        riders.push(p.body.pos);
                    }
                }
                if let Some(&pos) = riders.first() {
                    self.sounds.push((crate::protocol::sound::DING, to, pos));
                }
            }
        }
        if changed || self.elevators.iter().any(|e| e.moving.is_some()) != self.lift_was_moving {
            self.lift_was_moving = self.elevators.iter().any(|e| e.moving.is_some());
            self.sync_elevator_doors();
            self.doors_dirty = true;
        }
        self.close_stale_lift_panels();
    }
}

impl Server {
    // ------------------------------------------------------------ stalls

    /// Lock / unlock the stall the player is in (from inside only).
    pub(super) fn handle_door_action(&mut self, pid: u16) {
        let Some(p) = self.players.get(&pid) else { return };
        if !p.in_building() {
            return;
        }
        let (floor, room, pos) = (p.body.floor, p.room, p.body.pos);
        let say = |line: &str| Say::new(pid, line);
        let Some(i) = self.stalls.iter().position(|s| s.floor == floor && s.room == room) else {
            self.says.push(say(stalls::lines::NO_STALL));
            return;
        };
        let door = self.stalls[i].door;
        if self.stalls[i].locked_by.is_some() {
            self.set_stall_lock(i, None);
            self.sound(crate::protocol::sound::LOCK, pid);
            self.says.push(say(stalls::lines::UNLOCKED));
            return;
        }
        // Nobody may be standing in the doorway (they'd end up inside a wall).
        if pos.tile() == (door.x, door.y) {
            self.says.push(say(stalls::lines::STEP_IN));
            return;
        }
        if self.players.values().any(|o| o.id != pid && o.body.floor == floor && stalls::touches(o.body.pos, door)) {
            self.says.push(say(stalls::lines::IN_DOORWAY));
            return;
        }
        self.set_stall_lock(i, Some(pid));
        self.sound(crate::protocol::sound::LOCK, pid);
        self.says.push(say(stalls::lines::LOCKED));
    }

    pub(super) fn set_stall_lock(&mut self, i: usize, by: Option<u16>) {
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
    pub(super) fn check_stalls(&mut self) {
        let stale: Vec<usize> = self
            .stalls
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.locked_by.is_some_and(|pid| self.players.get(&pid).is_none_or(|p| p.body.floor != s.floor || p.room != s.room))
            })
            .map(|(i, _)| i)
            .collect();
        for i in stale {
            self.set_stall_lock(i, None);
        }
    }

    pub(super) fn doors_packet(&self, floor: u8) -> Packet {
        let tiles =
            self.building.floor(floor).map_or_else(Vec::new, |m| m.closed_tiles().into_iter().map(|(x, y)| (x as u8, y as u8)).collect());
        let lifts = self
            .elevators
            .iter()
            .map(|e| proto::Lift { floor: e.floor, target: e.heading().unwrap_or(proto::NO_FLOOR), moving: e.moving.is_some() })
            .collect();
        Packet::Doors { floor, tiles, lifts }
    }
}
