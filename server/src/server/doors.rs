//! Doors that change: elevators and toilet stalls.

use crate::elevator;
use crate::protocol::{self as proto, Packet};
use crate::sim::{Body, Pos};
use crate::stalls;

use super::{Say, Server};

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

    /// E at the elevator: the button in the cabin, or the call button at
    /// the doors. `None` = no elevator here.
    pub(super) fn use_elevator(&mut self, body: &Body) -> Option<String> {
        let tick = self.tick;
        if let Some(i) = self.elevators.iter().position(|e| e.in_cabin(body.floor, body.pos)) {
            let e = &mut self.elevators[i];
            self.doors_dirty = true;
            return Some(match e.press_inside(&self.building, body.floor, tick) {
                Some(t) => format!("Jedziemy na: {}.", self.building.floor_name(t)),
                None => elevator::lines::RIDING.to_string(),
            });
        }
        // Between two lifts: the call button of the nearer one.
        let i = (0..self.elevators.len()).filter_map(|i| self.elevators[i].door_distance(body).map(|d| (i, d))).min_by_key(|&(_, d)| d)?.0;
        self.doors_dirty = true; // show where the car is heading at once
        Some(self.elevators[i].call(body.floor, tick).to_string())
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
