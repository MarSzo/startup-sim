//! Cigarette smoke, the fire alarm and the fire brigade.

use crate::clock;
use crate::commute::Vehicle;
use crate::computer;
use crate::fire::{self, Alarm};
use crate::map::Tile;
use crate::needs::Rest;
use crate::npc::{self, Npc};
use crate::sim::Pos;

use super::{Say, Server};

impl Server {
    /// Smoke: complaints in smoky rooms, detectors, the fire alarm and the
    /// fire brigade (engine at the entrance, a firefighter checks the room,
    /// airs it, fines the smoker; back at the engine = the alarm is over).
    pub(super) fn tick_smoke_and_alarm(&mut self) {
        self.smoke_complaints();
        match self.alarm.take() {
            None => self.detect_fire(),
            Some(alarm) => {
                self.nag_to_get_out();
                self.alarm = self.tick_fire_brigade(alarm);
            }
        }
    }

    /// Coughing in smoky rooms (not about your own smoke).
    fn smoke_complaints(&mut self) {
        for p in self.players.values_mut().filter(|p| p.in_building()) {
            let place = (p.body.floor, p.room);
            let level = self.smoke.get(place);
            let smoking =
                matches!(p.rest, Some((Rest::Smoking { .. }, _, _))) || self.smoke.smoker.get(&place).is_some_and(|s| s.0 == p.id);
            if level >= fire::NOTICEABLE && !smoking && !p.smoke_said {
                p.smoke_said = true;
                p.needs.add_stress(fire::SMOKY_STRESS);
                self.says.push(Say::new(p.id, fire::lines::SMOKY));
            } else if level < fire::NOTICEABLE / 2 {
                p.smoke_said = false;
            }
        }
    }

    /// A detector goes off? The fire engine is called.
    fn detect_fire(&mut self) {
        let hit = self
            .smoke
            .levels()
            .into_iter()
            .filter(|(_, l)| *l >= fire::ALARM)
            .map(|(k, _)| k)
            .find(|&(f, r)| self.building.floor(f).is_some_and(|m| m.rooms.iter().any(|d| d.id == r && d.detector)));
        let Some(place) = hit else { return };
        let (smoker, spot) = match self.smoke.smoker.get(&place) {
            Some(&(pid, pos)) => (Some(pid), pos),
            None => {
                let t = self.building.floor(place.0).and_then(|m| m.room_tiles(place.1).first().copied());
                (None, t.map_or(Pos { x: 0, y: 0 }, |t| Pos::tile_center(t.x, t.y)))
            }
        };
        let truck = self.alloc_handle();
        self.vehicles.push(Vehicle::fire_engine(&self.building.outside, truck));
        self.alarm = Some(Alarm { place, smoker, spot, truck, firefighter: None, checking_until: None, done: false });
        self.clock_dirty = true;
        let room = self.room_name(place);
        self.log(format!("* FIRE ALARM: smoke in {room} (floor {})", place.0));
    }

    /// Everybody out! Reminders (and stress) for those still inside.
    fn nag_to_get_out(&mut self) {
        let tick = self.tick;
        for p in self.players.values_mut().filter(|p| p.in_building()) {
            if !self.smoke.is_open_air((p.body.floor, p.room)) && tick.wrapping_sub(p.alarm_nag) >= fire::NAG_TICKS {
                p.alarm_nag = tick;
                p.needs.add_stress(fire::STAYING_IN_STRESS);
                self.says.push(Say::new(p.id, fire::lines::GET_OUT));
            }
        }
    }

    /// The firefighter's job, one step; `None` once the alarm is over.
    fn tick_fire_brigade(&mut self, mut alarm: Alarm) -> Option<Alarm> {
        let tick = self.tick;
        let Some(id) = alarm.firefighter else {
            if self.vehicles.iter().any(|v| v.handle == alarm.truck && v.parked()) {
                let id = npc::NPC_ID_BASE + 0x0E00 + self.next_crew_id % 0x100;
                self.next_crew_id = self.next_crew_id.wrapping_add(1);
                let mut n = Npc::firefighter(&self.building, id, fire::crew_spawn(&self.building.outside));
                let (x, y) = alarm.spot.tile();
                let sent = n.go_to(&self.building, (alarm.place.0, Tile { x, y }));
                self.npcs.push(n);
                self.says.push(Say::new(id, fire::lines::ARRIVED));
                alarm.firefighter = Some(id);
                alarm.done = !sent; // can't get there: nothing to do
            }
            return Some(alarm);
        };
        let i = self.npcs.iter().position(|n| n.id == id)?;
        if alarm.done {
            if self.npcs[i].at_home() {
                self.npcs.remove(i);
                if let Some(v) = self.vehicles.iter_mut().find(|v| v.handle == alarm.truck) {
                    v.leave(fire::truck_exit(&self.building.outside));
                }
                self.clock_dirty = true;
                self.log("* fire alarm over");
                return None;
            }
            if self.npcs[i].is_idle() {
                self.npcs[i].return_home(&self.building);
            }
            return Some(alarm);
        }
        if !self.npcs[i].is_idle() {
            return Some(alarm); // on the way
        }
        match alarm.checking_until {
            None => {
                alarm.checking_until = Some(tick + fire::CHECK_TICKS);
                self.says.push(Say::new(id, fire::lines::CHECKING));
            }
            Some(t) if tick >= t => {
                self.smoke.clear(alarm.place);
                let room = self.room_name(alarm.place);
                self.says.push(Say::new(id, fire::lines::verdict(&room)));
                let mut nick = None;
                if let Some(p) = alarm.smoker.and_then(|pid| self.players.get_mut(&pid)) {
                    let fine = fire::FINE.min(p.money.max(0));
                    p.money -= fine;
                    nick = Some(p.nick.clone());
                    self.says.push(Say::addressed(id, fire::lines::fined(fine), p.id));
                    self.says.push(Say::new(p.id, fire::lines::SHAME));
                }
                let text = fire::lines::post(&clock::hhmm(self.clock.minute()), &room, nick.as_deref());
                self.messenger.post_system(computer::conv::GENERAL, id, "Administracja budynku", &text);
                alarm.done = true;
                self.npcs[i].return_home(&self.building);
            }
            Some(_) => {}
        }
        Some(alarm)
    }

    fn room_name(&self, (floor, room): (u8, u16)) -> String {
        self.building.floor(floor).map_or("-", |m| m.room_name(room)).to_string()
    }
}
