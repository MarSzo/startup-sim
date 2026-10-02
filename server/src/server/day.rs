//! Game time: the working day, going home, commuting in the morning.

use crate::clock::{self, Transition};
use crate::commute::{self, Vehicle, VehicleEvent};
use crate::inventory::kind as item_kind;
use crate::needs;
use crate::protocol::{self as proto, Packet};
use crate::shop;
use crate::sim::{Body, Pos};
use crate::weather;

use super::player::{refresh, Player, Stage};
use super::{Say, Server};

impl Server {
    // ------------------------------------------------------------- clock

    /// Game time: salary accrues, 22:00 sends everybody home (payday), 6:00
    /// starts a new day with random arrivals, arrivals come in.
    pub(super) fn tick_clock(&mut self) {
        let rate = self.clock.rate() as u64;
        let transition = self.clock.tick();
        if self.weather.tick(self.clock.total_minutes(), &mut self.rng) {
            self.log(format!("* weather: {}", weather::name(self.weather.now)));
            self.clock_dirty = true;
        }
        if !self.clock.is_night() {
            for p in self.players.values_mut() {
                if p.contract && p.in_building() {
                    p.worked_ds += rate;
                }
            }
        }
        match transition {
            Some(Transition::Evening) => {
                self.lunch_orders.clear(); // uncollected boxes go in the bin
                self.lights.on.clear(); // the last one out turns off the lights
                self.puddles.clear(); // mopped up overnight
                let ids: Vec<u16> = self.players.values().filter(|p| p.in_building()).map(|p| p.id).collect();
                for pid in ids {
                    self.go_home(pid);
                }
                self.log(format!("* day {} ends: office closed", self.clock.day));
                self.clock_dirty = true;
            }
            Some(Transition::Morning) => {
                let now = self.clock.total_minutes();
                let mut off = Vec::new();
                for p in self.players.values_mut() {
                    p.day += 1;
                    p.thefts_today = 0;
                    // A day off: at home all day (paid on an employment contract).
                    if p.contract && p.hr.morning(p.day) {
                        let paid = matches!(p.employment, 0 | proto::employment::EMPLOYMENT);
                        if paid {
                            p.money += crate::hr::LEAVE_HOURS * p.pay_rate;
                        }
                        p.depart_at = None;
                        off.push((p.nick.clone(), paid));
                        continue;
                    }
                    if matches!(p.stage, Stage::Home { .. }) {
                        // Leaves home at a random time; how they travel is
                        // chosen until then (the last choice by default).
                        p.depart_at = Some(now + self.rng.u32(commute::DEPART_FROM..=commute::DEPART_TO));
                    }
                }
                self.schedule_treats();
                self.open_vacancy();
                if let Some(k) = self.kitchen.as_mut() {
                    k.restock();
                }
                for (nick, paid) in off {
                    self.log(format!("* {nick} is on leave today ({})", if paid { "paid" } else { "unpaid" }));
                }
                self.log(format!("* day {} starts", self.clock.day));
                self.clock_dirty = true;
            }
            None => {}
        }
        let now = self.clock.total_minutes();
        if self.treat_drops.first().is_some_and(|&t| now >= t) {
            self.treat_drops.remove(0);
            self.put_tray();
        }
        // Leaving home: pay the fare, the trip takes its time.
        let leaving: Vec<u16> = self
            .players
            .values()
            .filter(|p| matches!(p.stage, Stage::Home { arrive_at: None }) && p.depart_at.is_some_and(|t| now >= t))
            .map(|p| p.id)
            .collect();
        for pid in leaving {
            let traffic = self.rng.u32(0..=commute::MAX_TRAFFIC);
            let Some(p) = self.players.get_mut(&pid) else { continue };
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
        let arriving: Vec<u16> =
            self.players.values().filter(|p| matches!(p.stage, Stage::Home { arrive_at: Some(t) } if now >= t)).map(|p| p.id).collect();
        for pid in arriving {
            self.arrive(pid);
        }
    }

    /// 22:00: out of the building; salary for the hours worked today.
    pub(super) fn go_home(&mut self, pid: u16) {
        self.end_session(pid);
        self.vehicles.retain(|v| v.owner != pid); // the car / bike goes home too
        let Some(p) = self.players.get_mut(&pid) else { return };
        p.rest = None;
        p.riding = None;
        let minutes = (p.worked_ds / clock::DS_PER_MIN as u64) as u32;
        let pay = minutes as i64 * p.pay_rate / 60;
        p.money += pay;
        p.last_pay = (pay, minutes);
        if p.contract && minutes >= crate::hr::WORKED_DAY_MINUTES {
            p.hr.worked_a_day();
        }
        p.worked_ds = 0;
        p.stage = Stage::Home { arrive_at: None };
        let msg = format!("* {} goes home: worked {} min, paid {}", p.nick, minutes, shop::zl(pay));
        self.log(msg);
        self.save_soon = true;
    }

    /// Morning arrival: on foot along the sidewalk, or riding in a vehicle
    /// that drops them off (see `tick_vehicles`).
    pub(super) fn arrive(&mut self, pid: u16) {
        let Some(mode) = self.players.get(&pid).map(|p| p.commute_mode) else { return };
        if let Some(p) = self.players.get_mut(&pid) {
            p.skip_wait = false;
        }
        let handle = self.alloc_handle();
        let kind_of = |m: u8| Vehicle::for_mode(&self.building.outside, m, 0, 0, 0).map(|v| v.kind);
        let slot = self.vehicles.iter().filter(|v| v.parks && Some(v.kind) == kind_of(mode)).count();
        let vehicle = Vehicle::for_mode(&self.building.outside, mode, handle, pid, slot);
        let (pos, riding) = match &vehicle {
            Some(v) => (v.pos, Some(v.handle)),
            None => {
                let w = self.building.outside.walk_arrival;
                (Pos::tile_center(w.x, w.y), None) // walking in from the west
            }
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
    pub(super) fn trip_done(&mut self, pid: u16) {
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
                self.says.push(Say::new(pid, weather::lines::SOAKED_ON_THE_WAY));
            } else if p.commute_mode == commute::mode::WALK {
                self.says.push(Say::new(pid, weather::lines::UMBRELLA));
            }
        }
        if late {
            p.needs.add_stress(10);
            self.says.push(Say::new(pid, commute::lines::LATE));
        }
    }
}

impl Server {
    /// Move the vehicles; riders go along and get out at the stop.
    pub(super) fn tick_vehicles(&mut self) {
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

    pub(super) fn clock_packet(&self, p: &Player) -> Packet {
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
            company: self.company.name.clone(),
            founded: self.company.founder.is_some() || self.offline.founder.is_some(),
            alarm: self.alarm.is_some() as u8,
            skip: if self.clock.skip { 2 } else { p.skip_wait as u8 },
            leave: p.hr.on_leave,
        }
    }
}
