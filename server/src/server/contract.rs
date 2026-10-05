//! Signing at HR: the contract shows the pay - a bit lower than agreed in
//! the interview. Sign it (card, laptop, the job), or turn it down: HR walks
//! you to the porter's desk, the pass goes back, and it's the job portal
//! again.

use crate::inventory::kind as item_kind;
use crate::npc::{self, Role};
use crate::pay::{self, lines, Terms};
use crate::protocol::{employment, Packet};

use super::{Say, Server};

/// After giving the pass back, this long (5 s) before the portal.
const TO_PORTAL_TICKS: u32 = 5 * 20;

impl Server {
    /// HR (`npc`) shows `pid` the contract: the agreed pay with HR's "small
    /// correction" (drawn once, then the same every time it's shown).
    pub(super) fn show_contract(&mut self, npc: u16, pid: u16) {
        let fallback =
            self.players.get(&pid).and_then(|p| p.position).and_then(|o| self.position(o)).map_or(pay::DEFAULT_RANGE[0], |o| o.salary[0]);
        let cut = self.rng.u32(pay::CUT_PERCENT);
        let Some(p) = self.players.get_mut(&pid) else { return };
        let t = p.terms.get_or_insert(Terms { agreed: fallback, form: employment::EMPLOYMENT, offered: 0 });
        if t.offered == 0 {
            t.offered = pay::offered(t.agreed, t.form, cut);
        }
        let t = *t;
        p.contract_shown = Some(npc);
        let (addr, position, dept) = (p.addr, p.position, p.department);
        let title = position
            .and_then(|o| self.job_title(o))
            .or_else(|| self.cfg.recruitment.department_name(dept).map(str::to_string))
            .unwrap_or_default();
        let packet = Packet::Dialog {
            id: pay::CONTRACT_ID,
            npc,
            text: lines::contract(&title, t.form, t.offered, t.agreed),
            options: vec![lines::SIGN.into(), lines::RESIGN.into()],
            items: Vec::new(),
        };
        self.send(addr, &packet);
    }

    /// The answer to the contract; false if `dialog` isn't it.
    pub(super) fn answer_contract(&mut self, pid: u16, dialog: u8, choice: u8) -> bool {
        if dialog != pay::CONTRACT_ID {
            return false;
        }
        let Some(p) = self.players.get_mut(&pid) else { return true };
        let Some(hr) = p.contract_shown.take() else { return true };
        let addr = p.addr;
        self.send(addr, &Packet::Dialog { id: 0, npc: hr, text: String::new(), options: Vec::new(), items: Vec::new() });
        let Some(p) = self.players.get(&pid) else { return true };
        if p.contract || !p.inventory.has(item_kind::GUEST_PASS) {
            return true; // signed already, or the pass is gone
        }
        if choice == 0 {
            let dept = self.cfg.recruitment.department_name(p.department).map(str::to_string);
            let advance = p.terms.is_none_or(|t| t.form == employment::EMPLOYMENT);
            // Signed: card + laptop; the card replaces the guest pass.
            self.apply_npc_events(vec![
                npc::Event::Say { npc: hr, text: lines::signed(dept.as_deref(), advance), to: Some(pid) },
                npc::Event::Take { player: pid, item: item_kind::GUEST_PASS },
                npc::Event::Give { player: pid, item: item_kind::EMPLOYEE_CARD },
                npc::Event::Give { player: pid, item: item_kind::LAPTOP },
                npc::Event::Contract { player: pid },
            ]);
            return true;
        }
        self.says.push(Say::addressed(hr, lines::RESIGNED, pid));
        self.log(format!("* {} turned the contract down", self.nick_of_player(pid)));
        let walked = self.npcs.iter_mut().find(|n| n.id == hr).is_some_and(|n| n.see_out(&self.building, pid));
        if !walked {
            self.saw_out(pid);
        }
        true
    }

    /// Seen out to the porter's desk: Pani Wiesia takes the pass back; a
    /// moment later, the job portal.
    pub(super) fn saw_out(&mut self, pid: u16) {
        let porter = self.npcs.iter().find(|n| n.role == Role::Porter).map(|n| n.id);
        let tick = self.tick;
        let Some(p) = self.players.get_mut(&pid) else { return };
        if p.inventory.remove_kind(item_kind::GUEST_PASS).is_some() {
            super::player::refresh(p);
        }
        p.to_portal_at = Some(tick + TO_PORTAL_TICKS);
        if let Some(porter) = porter {
            self.says.push(Say::addressed(porter, lines::PASS_BACK, pid));
        }
    }

    /// Players whose time in the building is up after turning the contract down.
    pub(super) fn tick_to_portal(&mut self) {
        let tick = self.tick;
        let due: Vec<u16> = self.players.values().filter(|p| p.to_portal_at.is_some_and(|t| tick >= t)).map(|p| p.id).collect();
        for pid in due {
            let body = lines::resigned_mail(&self.nick_of_player(pid));
            self.back_to_portal(pid, "HR", "Rezygnacja z umowy", body, "turned the contract down");
        }
    }
}
