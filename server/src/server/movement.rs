//! Applying the players' inputs, and what walking around causes.

use crate::building::Building;
use crate::coffee;
use crate::inventory::kind as item_kind;
use crate::needs::{self, Rest};
use crate::protocol as proto;
use crate::sim::{self, Body, Pos};
use crate::weather;

use super::player::{refresh, Player};
use super::{Say, Server, MAX_INPUTS_PER_TICK};

/// Entity flag: walked this tick.
const FLAG_MOVING: u8 = 0b100;

/// What happened while applying this tick's inputs; handled afterwards,
/// when the players aren't borrowed any more.
#[derive(Default)]
pub(super) struct Steps {
    /// E pressed (edge) without changing floors: who, and where they stood.
    pub(super) presses: Vec<(u16, Body)>,
    /// Not in the building (portal, home, commuting).
    pub(super) offline: Vec<u16>,
    coffee_ready: Vec<u16>,
    cold_cups: Vec<u16>,
    /// Smoking: (floor, room), who, where.
    puffs: Vec<((u8, u16), u16, Pos)>,
    /// Left a bathroom with dirty hands: (player, floor, bathroom room).
    unwashed_exits: Vec<(u16, u8, u16)>,
    /// Walked out of the shop with unpaid goods.
    shoplifters: Vec<u16>,
    /// Didn't make it to the toilet: (floor, where they stood).
    accidents: Vec<(u8, Pos)>,
}

impl Server {
    /// Apply queued inputs in sequence order, then per-player upkeep: rooms,
    /// coffee, needs, weather, entity flags.
    pub(super) fn simulate_players(&mut self) -> Steps {
        let mut steps = Steps::default();
        let (tick, weather_now, needs_speed) = (self.tick, self.weather.now, self.cfg.needs_speed.max(1));
        for p in self.players.values_mut() {
            if !p.in_building() {
                p.inputs.clear(); // not in the world yet
                steps.offline.push(p.id);
                continue;
            }
            if p.riding.is_some() {
                // In a vehicle: no walking (inputs are acknowledged, ignored).
                if let Some(&(seq, _)) = p.inputs.back() {
                    p.last_processed_seq = seq;
                }
                p.inputs.clear();
                continue;
            }
            let old_room = p.room;
            if tick < p.held_until {
                // Stopped by the guard / the police: inputs acknowledged, ignored.
                if let Some(&(seq, _)) = p.inputs.back() {
                    p.last_processed_seq = seq;
                }
                p.inputs.clear();
                p.flags &= !FLAG_MOVING;
            } else {
                apply_inputs(p, &self.building, &mut steps.presses);
            }
            p.room = self.building.room_at(p.body.floor, p.body.pos);
            if p.room != old_room {
                if self.shop_rooms.contains(&(p.body.floor, old_room)) && p.inventory.items().any(|i| i.unpaid) {
                    steps.shoplifters.push(p.id);
                }
                if p.needs.dirty_hands && left_bathroom(&self.building, p.body.floor, old_room, p.room) {
                    steps.unwashed_exits.push((p.id, p.body.floor, old_room));
                }
            }
            if coffee::tick_cup(&mut p.cup, tick) {
                steps.coffee_ready.push(p.id);
            }
            if !p.inventory.expire(tick).is_empty() {
                refresh(p);
                self.says.push(Say::new(p.id, coffee::lines::COLD));
                steps.cold_cups.push(p.id);
            }
            tick_needs(p, needs_speed, tick, &mut self.says, &mut self.sounds, &mut steps.accidents);
            if matches!(p.rest, Some((Rest::Smoking { .. }, _, _))) {
                steps.puffs.push(((p.body.floor, p.room), p.id, p.body.pos));
            }
            let outdoors = self.outdoor_rooms.contains(&(p.body.floor, p.room));
            let umbrella_open = outdoor_weather(p, outdoors, weather_now, &mut self.says);
            p.body.slow = p.needs.slow();
            p.flags = (p.flags & 0x37)
                | if umbrella_open { proto::FLAG_UMBRELLA } else { 0 }
                | if p.body.slow { proto::FLAG_SLOW } else { 0 }
                | if p.needs.smelly() { proto::FLAG_SMELLY } else { 0 };
        }
        steps
    }

    /// What the steps set off: the shop gate, bathroom witnesses, coffee
    /// ready / gone cold, cigarette smoke, accident puddles.
    pub(super) fn react_to_steps(&mut self, steps: &mut Steps) {
        for pid in std::mem::take(&mut steps.shoplifters) {
            self.shoplifted(pid);
        }
        for (pid, floor, bath) in std::mem::take(&mut steps.unwashed_exits) {
            self.unwashed_hands_seen(pid, floor, bath);
        }
        for pid in std::mem::take(&mut steps.coffee_ready) {
            let free = self.players.get(&pid).is_some_and(|p| p.inventory.hands_free());
            self.says.push(Say::new(pid, if free { coffee::lines::READY } else { coffee::lines::WAITING }));
            self.give_new(pid, item_kind::COFFEE); // no free hands: it waits on the floor
        }
        for pid in std::mem::take(&mut steps.cold_cups) {
            self.give_new(pid, item_kind::EMPTY_CUP);
        }
        for (place, pid, pos) in std::mem::take(&mut steps.puffs) {
            self.smoke.puff(place, pid, pos);
        }
        for (floor, pos) in std::mem::take(&mut steps.accidents) {
            self.leave_puddle(floor, pos);
        }
    }

    /// Somebody in the bathroom saw it.
    fn unwashed_hands_seen(&mut self, pid: u16, floor: u8, bath: u16) {
        let m = self.building.floor(floor);
        let witness = self.players.values().find(|o| {
            o.id != pid
                && o.in_building()
                && o.body.floor == floor
                && (o.room == bath || m.is_some_and(|m| m.visible_from(o.room).contains(&bath)))
        });
        let Some(w) = witness.map(|w| w.id) else { return };
        let Some(p) = self.players.get_mut(&pid) else { return };
        p.needs.add_stress(3);
        let line = format!("Ej, {}, a ręce?!", p.nick);
        self.says.push(Say::addressed(w, line, pid));
    }
}

/// Up to `MAX_INPUTS_PER_TICK` queued inputs through `sim::step`; E presses
/// go to `presses`. Updates facing and the "moving" flag.
fn apply_inputs(p: &mut Player, building: &Building, presses: &mut Vec<(u16, Body)>) {
    let mut moved = false;
    for _ in 0..MAX_INPUTS_PER_TICK {
        let Some((seq, bits)) = p.inputs.pop_front() else { break };
        let before = p.body;
        p.body = sim::step(building, p.body, bits);
        p.last_processed_seq = seq;
        // E pressed (edge) without riding the elevator.
        let pressed = bits & sim::IN_INTERACT != 0 && before.prev_input & sim::IN_INTERACT == 0;
        if pressed && p.body.floor == before.floor {
            presses.push((p.id, p.body));
        }
        moved |= p.body.pos != before.pos || p.body.floor != before.floor;
        p.flags = facing(bits, p.flags & 3);
    }
    if moved {
        p.flags |= FLAG_MOVING;
    }
}

/// Facing from the input direction (0 down, 1 up, 2 left, 3 right);
/// `current` when not pressing any.
fn facing(bits: u8, current: u8) -> u8 {
    match sim::input_dir(bits) {
        (_, dy) if dy > 0 => 0,
        (_, dy) if dy < 0 => 1,
        (dx, _) if dx < 0 => 2,
        (dx, _) if dx > 0 => 3,
        _ => current,
    }
}

/// Walked from a bathroom straight out (not into a stall).
fn left_bathroom(building: &Building, floor: u8, from: u16, to: u16) -> bool {
    let kind = |r: u16| building.floor(floor).and_then(|m| m.rooms.iter().find(|d| d.id == r)).map_or("", |d| d.kind.as_str());
    kind(from) == "bathroom" && !matches!(kind(to), "bathroom" | "stall")
}

/// Needs: moving ends a rest; `speed` needs ticks at once (dev); the
/// warnings they raise are said aloud, accidents go to `accidents`.
fn tick_needs(p: &mut Player, speed: u32, tick: u32, says: &mut Vec<Say>, sounds: &mut Vec<(u8, u8, Pos)>, accidents: &mut Vec<(u8, Pos)>) {
    let was_toilet = matches!(p.rest, Some((Rest::Toilet, _, _)));
    if let Some((_, floor, pos)) = p.rest {
        if (floor, pos) != (p.body.floor, p.body.pos) {
            p.rest = None;
        }
    }
    let mut rest = p.rest.map(|r| r.0);
    for _ in 0..speed {
        let (r, events) = p.needs.tick(rest, tick);
        rest = r;
        for e in events {
            let line = match e {
                needs::Event::Warn(l) | needs::Event::RestDone(l) => l,
                needs::Event::Accident => {
                    accidents.push((p.body.floor, p.body.pos));
                    needs::lines::ACCIDENT
                }
            };
            says.push(Say::new(p.id, line));
        }
    }
    p.rest = rest.map(|r| (r, p.body.floor, p.body.pos));
    if was_toilet && !matches!(p.rest, Some((Rest::Toilet, _, _))) {
        sounds.push((proto::sound::FLUSH, p.body.floor, p.body.pos));
    }
}

/// Weather under the open sky; returns whether the umbrella is open.
fn outdoor_weather(p: &mut Player, outdoors: bool, now: u8, says: &mut Vec<Say>) -> bool {
    if !outdoors {
        p.soaked_said = false;
        return false;
    }
    let umbrella = p.inventory.has(item_kind::UMBRELLA);
    let (hygiene, stress) = weather::outdoor_effect(now, umbrella);
    p.needs.weather(hygiene, stress);
    let wet = weather::wet(now);
    if wet && !umbrella && !p.soaked_said {
        p.soaked_said = true;
        says.push(Say::new(p.id, weather::lines::SOAKED));
    }
    umbrella && wet
}
