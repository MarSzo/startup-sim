//! Shoplifting: the security gate, the guard, patrol cars and fines.

use crate::commute::Vehicle;
use crate::npc::{self, Npc};
use crate::security::{self, PoliceCall};
use crate::shop;

use super::player::refresh;
use super::{Say, Server};

impl Server {
    /// Somebody walked out of the shop with unpaid goods: the gate beeps and
    /// the guard runs after them (busy with somebody else: straight to the
    /// police).
    pub(super) fn shoplifted(&mut self, pid: u16) {
        if self.npcs.iter().any(|n| n.chasing() == Some(pid)) || self.police_calls.iter().any(|c| c.target == pid) {
            return; // already after them
        }
        if let Some(p) = self.players.get_mut(&pid) {
            p.thefts_today = p.thefts_today.saturating_add(1);
        }
        let speaker = self.cashier.unwrap_or(pid);
        self.says.push(Say::addressed(speaker, shop::lines::ALARM, pid));
        self.sound(crate::protocol::sound::GATE_ALARM, pid);
        match self.npcs.iter_mut().find(|n| n.role == npc::Role::Guard && n.chasing().is_none()) {
            Some(g) => {
                g.chase(pid);
                let at = (g.body.floor, g.body.pos);
                self.sounds.push((crate::protocol::sound::WHISTLE, at.0, at.1));
                self.says.push(Say::addressed(g.id, npc::lines::GUARD_STOP, pid));
            }
            None => {
                self.says.push(Say::addressed(speaker, security::lines::CALLED, pid));
                self.call_police(pid);
            }
        }
    }

    /// A patrol car for `pid` (one per person).
    pub(super) fn call_police(&mut self, pid: u16) {
        if self.police_calls.iter().any(|c| c.target == pid) {
            return;
        }
        let car = self.alloc_handle();
        self.vehicles.push(Vehicle::police(&self.building.outside, car));
        self.police_calls.push(PoliceCall { target: pid, car, officer: None });
        let nick = self.players.get(&pid).map_or("?", |p| p.nick.as_str());
        self.log(format!("* police called for {nick}"));
    }

    /// Patrol cars: the officer gets out once the car stands at the
    /// entrance, and when back by the car both leave.
    pub(super) fn tick_police(&mut self) {
        let mut done = Vec::new();
        for i in 0..self.police_calls.len() {
            let PoliceCall { target, car, officer } = self.police_calls[i].clone();
            match officer {
                None => {
                    if self.vehicles.iter().any(|v| v.handle == car && v.parked()) {
                        let id = npc::NPC_ID_BASE + 0x0F00 + self.next_officer_id % 0x100;
                        self.next_officer_id = self.next_officer_id.wrapping_add(1);
                        let mut n = Npc::police(&self.building, id, security::officer_spawn(&self.building.outside));
                        n.chase(target);
                        self.npcs.push(n);
                        self.police_calls[i].officer = Some(id);
                    }
                }
                Some(id) => {
                    if self.npcs.iter().any(|n| n.id == id && n.is_idle()) {
                        self.npcs.retain(|n| n.id != id);
                        if let Some(v) = self.vehicles.iter_mut().find(|v| v.handle == car) {
                            v.leave(security::car_exit(&self.building.outside));
                        }
                        done.push(car);
                    }
                }
            }
        }
        self.police_calls.retain(|c| !done.contains(&c.car));
    }

    /// The police fine: from the wallet (never below zero), goods confiscated.
    fn police_fine(&mut self, pid: u16) -> Option<i64> {
        let p = self.players.get_mut(&pid)?;
        let fine = security::FINE.min(p.money.max(0));
        p.money -= fine;
        p.inventory.remove_unpaid();
        p.needs.add_stress(security::POLICE_STRESS);
        refresh(p);
        Some(fine)
    }

    /// A chase ended next to the player: what the guard / officer does.
    pub(super) fn caught(&mut self, npc_id: u16, pid: u16) {
        let Some(role) = self.npcs.iter().find(|n| n.id == npc_id).map(|n| n.role) else { return };
        if role == npc::Role::Police {
            let Some(fine) = self.police_fine(pid) else { return };
            self.hold(pid, security::POLICE_HOLD_TICKS);
            self.says.push(Say::addressed(npc_id, security::lines::police_fine(fine), pid));
            self.says.push(Say::new(pid, security::lines::SHAME));
            let nick = self.players.get(&pid).map_or("?", |p| p.nick.as_str());
            self.log(format!("* police fined {nick} {}", shop::zl(fine)));
            return;
        }
        let Some(p) = self.players.get_mut(&pid) else { return };
        if !p.inventory.items().any(|i| i.unpaid) {
            self.says.push(Say::addressed(npc_id, security::lines::GUARD_PAID, pid));
            return;
        }
        p.inventory.remove_unpaid();
        p.held_until = self.tick + security::GUARD_HOLD_TICKS;
        p.needs.add_stress(security::GUARD_STRESS);
        refresh(p);
        let again = p.thefts_today >= security::THEFTS_FOR_POLICE;
        self.says.push(Say::addressed(npc_id, security::lines::GUARD_CAUGHT, pid));
        if again {
            self.says.push(Say::addressed(npc_id, security::lines::GUARD_POLICE_AGAIN, pid));
            self.call_police(pid);
        }
    }

    /// Caught: no walking for `ticks`.
    fn hold(&mut self, pid: u16, ticks: u32) {
        if let Some(p) = self.players.get_mut(&pid) {
            p.held_until = self.tick + ticks;
        }
    }

    /// The chase was given up: the guard calls the police; the police send
    /// the fine anyway.
    pub(super) fn escaped(&mut self, npc_id: u16, pid: u16) {
        let Some(role) = self.npcs.iter().find(|n| n.id == npc_id).map(|n| n.role) else { return };
        if role == npc::Role::Police {
            if let Some(fine) = self.police_fine(pid) {
                self.says.push(Say::new(pid, security::lines::fine_by_mail(fine)));
            }
            return;
        }
        if self.players.get(&pid).is_some_and(|p| p.inventory.items().any(|i| i.unpaid)) {
            self.call_police(pid);
            self.says.push(Say::addressed(npc_id, security::lines::GUARD_ESCAPED, pid));
        }
    }
}
