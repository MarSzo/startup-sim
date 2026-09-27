//! The startup: founder, candidates, hiring and firing.

use crate::company;
use crate::computer::Workstation;
use crate::inventory::kind as item_kind;
use crate::protocol::{self as proto, Packet};
use crate::sim::{Body, Pos};
use crate::shop;

use super::player::{refresh, Desk, Stage};
use super::portal::MAX_VACANCIES;
use super::Server;

impl Server {
    pub(super) fn founder_online(&self) -> bool {
        self.company.founder.is_some_and(|f| self.players.contains_key(&f))
    }

    /// Hired: invitation to the trial day, one place fewer.
    pub(super) fn hire(&mut self, pid: u16, offer: u8) {
        self.company.candidates.retain(|c| c.player != pid);
        if self.vacancies.get(&offer).copied().unwrap_or(0) == 0 {
            self.position_filled_mail(pid, offer);
            return;
        }
        let from = format!("{} — Rekrutacja", self.company.name);
        let Some(o) = self.cfg.recruitment.offer(offer).cloned() else { return };
        let Some(p) = self.players.get_mut(&pid) else { return };
        let nick = p.nick.clone();
        p.position = Some(offer);
        let Stage::Portal(desk) = &mut p.stage else { return };
        desk.awaiting = None;
        // Days as the employee sees them (their own day count): they start
        // at the office on their next day.
        self.company.hired_on.insert(pid, p.day + 1);
        desk.hired = Some(o.department);
        desk.mail(
            &from,
            "Zaproszenie na dzień próbny".into(),
            format!(
                "Gratulacje, {nick}!\n\nZapraszamy na dzień próbny na stanowisko {}: zgłoś się na portierni — portier \
                 zaprowadzi Cię na recepcję, a w HR podpiszesz umowę i odbierzesz kartę.\n\nDo zobaczenia!",
                o.title
            ),
            proto::portal_action::GO_TO_OFFICE,
            0,
        );
        self.take_vacancy(offer);
        self.send_portal(pid, true);
    }

    /// The founder says no (or the place is gone).
    pub(super) fn reject_candidate(&mut self, pid: u16) {
        let Some(i) = self.company.candidates.iter().position(|c| c.player == pid) else { return };
        let c = self.company.candidates.remove(i);
        let from = format!("{} — Rekrutacja", self.company.name);
        let company = self.company.name.clone();
        let Some(p) = self.players.get_mut(&pid) else { return };
        let nick = p.nick.clone();
        let Stage::Portal(desk) = &mut p.stage else { return };
        desk.awaiting = None;
        desk.applied.retain(|o| *o != c.offer);
        desk.mail(&from, "Decyzja zarządu".into(), company::lines::rejected(&nick, &company), proto::portal_action::NONE, 0);
        self.send_portal(pid, true);
    }

    /// Candidates the founder didn't decide on in time (or with no founder
    /// around): the interview result stands - hired.
    pub(super) fn tick_company(&mut self) {
        let now = self.clock.total_minutes();
        let founder_here = self.founder_online();
        let due: Vec<(u16, u8)> = self
            .company
            .candidates
            .iter()
            .filter(|c| !founder_here || now >= c.since + company::DECISION_MINUTES)
            .map(|c| (c.player, c.offer))
            .collect();
        for (pid, offer) in due {
            self.hire(pid, offer);
        }
        self.company.candidates.retain(|c| self.players.contains_key(&c.player));
    }

    /// Found the company from the job portal: into the board, with a card and
    /// a laptop, in the board room.
    pub(super) fn found_company(&mut self, pid: u16, name: &str) {
        if self.company.founder.is_some() {
            return;
        }
        let Some(name) = company::clean(name, company::NAME_MIN, company::NAME_MAX) else { return };
        let Some(p) = self.players.get_mut(&pid) else { return };
        if !matches!(p.stage, Stage::Portal(_)) {
            return;
        }
        p.stage = Stage::Working;
        p.department = company::BOARD_DEPARTMENT;
        p.contract = true;
        p.day = p.day.max(2);
        p.money += shop::ADVANCE;
        let pos = self.board_room.and_then(|(f, r)| {
            let m = self.building.floor(f)?;
            // By the meeting table (where the laptop goes), else anywhere free.
            let tiles = m.room_tiles(r);
            let by_table = crate::map::Tile { x: 50, y: 18 };
            let t = if tiles.contains(&by_table) { Some(by_table) } else { tiles.into_iter().find(|t| !m.is_blocked(t.x, t.y)) };
            t.map(|t| (f, t))
        });
        if let Some((f, t)) = pos {
            p.body = Body::at(f, Pos::tile_center(t.x, t.y));
            p.room = self.board_room.map_or(0, |b| b.1);
        }
        self.company.founder = Some(pid);
        self.company.name = name.clone();
        self.company.hired_on.insert(pid, p.day);
        let nick = p.nick.clone();
        self.give_new(pid, item_kind::EMPLOYEE_CARD);
        self.give_new(pid, item_kind::LAPTOP);
        self.clock_dirty = true;
        for other in self.players.values_mut() {
            other.known.remove(&pid); // new department on the name tag
        }
        self.log(format!("* company founded: '{name}' by {nick}"));
    }

    /// The panel's account: the founder's computer (whoever sits at it).
    pub(super) fn is_founder_screen(&self, pid: u16) -> bool {
        self.calendar_account(pid).is_some_and(|a| Some(a) == self.company.founder)
    }

    pub(super) fn company_packets(&self, pid: u16) -> Vec<Packet> {
        if !self.is_founder_screen(pid) {
            return Vec::new();
        }
        let offers = self
            .cfg
            .recruitment
            .offers
            .iter()
            .filter(|o| o.hiring)
            .map(|o| {
                let desc = self.company.descriptions.get(&o.id).cloned().unwrap_or_else(|| o.description.clone());
                (o.id, self.vacancies.get(&o.id).copied().unwrap_or(0), o.title.clone(), desc)
            })
            .collect();
        let candidates = self
            .company
            .candidates
            .iter()
            .filter_map(|c| self.players.get(&c.player).map(|p| (c.player, c.offer, c.score, c.total, p.nick.clone())))
            .collect();
        let staff = self
            .players
            .values()
            // Signed, or hired and on the way (no contract yet).
            .filter(|p| p.contract || p.position.is_some())
            .map(|p| {
                let dept = match p.department {
                    0 => p.position.and_then(|o| self.cfg.recruitment.offer(o)).map_or(0, |o| o.department),
                    d => d,
                };
                (p.id, dept, self.company.hired_on.get(&p.id).copied().unwrap_or(1) as u16, p.nick.clone())
            })
            .collect();
        vec![Packet::CompanyOffers { name: self.company.name.clone(), offers }, Packet::CompanyPeople { candidates, staff }]
    }

    pub(super) fn send_company(&mut self, pid: u16) {
        for pk in self.company_packets(pid) {
            self.send_to(pid, &pk);
        }
    }

    pub(super) fn handle_company_action(&mut self, pid: u16, action: u8, target: u16, value: u8, text: &str) {
        use company::action as a;
        if action == a::FOUND {
            self.found_company(pid, text);
            return;
        }
        if !self.is_founder_screen(pid) {
            return;
        }
        match action {
            a::RENAME => {
                if let Some(name) = company::clean(text, company::NAME_MIN, company::NAME_MAX) {
                    self.log(format!("* company renamed: '{name}'"));
                    self.company.name = name;
                    self.clock_dirty = true;
                }
            }
            // Offer ids are u8; a larger target must not wrap onto another offer.
            a::SET_PLACES => self.company_set_places(target, value),
            a::SET_DESCRIPTION => {
                if let Some(offer) = u8::try_from(target).ok().filter(|o| self.vacancies.contains_key(o)) {
                    match company::clean(text, 1, company::DESCRIPTION_MAX) {
                        Some(d) => self.company.descriptions.insert(offer, d),
                        None => self.company.descriptions.remove(&offer),
                    };
                }
            }
            a::HIRE => {
                if let Some(offer) = self.company.candidates.iter().find(|c| c.player == target).map(|c| c.offer) {
                    self.hire(target, offer);
                }
            }
            a::REJECT => self.reject_candidate(target),
            a::FIRE => self.fire(target),
            _ => {}
        }
        self.send_company(pid);
    }

    /// Open places for an offer (the founder's panel).
    pub(super) fn company_set_places(&mut self, target: u16, value: u8) {
        if let Some(v) = u8::try_from(target).ok().and_then(|o| self.vacancies.get_mut(&o)) {
            *v = value.min(company::MAX_PLACES);
        }
    }

    /// Fired: card and laptop back, out of the building, job hunting again.
    pub(super) fn fire(&mut self, pid: u16) {
        if Some(pid) == self.company.founder {
            return;
        }
        let Some(p) = self.players.get(&pid) else { return };
        if !p.contract && p.position.is_none() {
            return;
        }
        self.end_session(pid);
        self.computers.retain(|c| c.owner() != pid);
        self.vehicles.retain(|v| v.owner != pid);
        let company = self.company.name.clone();
        let from = format!("{company} — Zarząd");
        let Some(p) = self.players.get_mut(&pid) else { return };
        let nick = p.nick.clone();
        p.inventory.remove_owned_by(pid);
        refresh(p);
        p.contract = false;
        p.department = 0;
        p.rest = None;
        p.riding = None;
        let position = p.position.take();
        let mut desk = Box::<Desk>::default();
        // The client clears its inbox when it sees the portal again; new ids
        // anyway, in case that Clock is lost.
        desk.next_mail = 100;
        desk.mail(&from, "Rozwiązanie umowy".into(), company::lines::fired(&nick, &company), proto::portal_action::NONE, 0);
        p.stage = Stage::Portal(desk);
        if let Some(v) = position.and_then(|o| self.vacancies.get_mut(&o)) {
            *v = (*v + 1).min(MAX_VACANCIES);
        }
        self.company.hired_on.remove(&pid);
        for other in self.players.values_mut() {
            other.known.remove(&pid);
        }
        self.clock_dirty = true;
        self.log(format!("* {nick} fired"));
        // Clock first (back on the portal), then the new inbox.
        if let Some(clock) = self.players.get(&pid).map(|p| self.clock_packet(p)) {
            self.send_to(pid, &clock);
        }
        self.send_portal(pid, true);
    }

    /// HR signed the contract: the department is official, the advance paid.
    pub(super) fn sign_contract(&mut self, pid: u16) {
        let Some(p) = self.players.get_mut(&pid) else { return };
        p.contract = true;
        p.money += shop::ADVANCE;
        self.company.hired_on.entry(pid).or_insert(p.day);
        let dept = self.cfg.recruitment.department_name(p.department).unwrap_or("-");
        let msg = format!("* player {pid} '{}' signed a contract: {dept}", p.nick);
        let (nick, dept) = (p.nick.clone(), dept.to_string());
        self.log(msg);
        let company = self.company.name.clone();
        let body = format!(
            "Cześć {nick}!\nWitamy w {company}, dział: {dept}.\n\nNa pulpicie masz komunikator, pocztę i przeglądarkę — w ulubionych jest tablica zadań działu i zamawianie obiadów. Kubki są w szafce w aneksie kuchennym.\n\nPowodzenia!\nHR"
        );
        self.office_mail(&nick, "HR", &format!("Witamy w {company}!"), &body);
        // Everyone gets the updated PlayerInfo (department) again.
        for other in self.players.values_mut() {
            other.known.remove(&pid);
        }
    }

    /// `--start-employed`: contract, department, card and laptop, and a spot
    /// in front of a desk of the department.
    pub(super) fn employ(&mut self, id: u16) {
        let dept = if id % 2 == 1 { 1 } else { 2 };
        let Some(dept_name) = self.cfg.recruitment.department_name(dept).map(str::to_string) else { return };
        let seat = {
            let n = self.players.values().filter(|p| p.contract && p.department == dept).count();
            let desks: Vec<&Workstation> = self.workstations.iter().filter(|w| w.room_name == dept_name).collect();
            desks.get(n % desks.len().max(1)).and_then(|w| {
                let m = self.building.floor(w.floor)?;
                [1, -1].iter().map(|dy| (w.tile.x, w.tile.y + dy)).find(|&(x, y)| !m.is_blocked(x, y)).map(|(x, y)| (w.floor, x, y))
            })
        };
        let Some(p) = self.players.get_mut(&id) else { return };
        p.department = dept;
        p.contract = true;
        p.money += shop::ADVANCE;
        self.company.hired_on.insert(id, p.day.max(2)); // --start-employed: day 2
        if let Some((floor, x, y)) = seat {
            p.body = Body::at(floor, Pos::tile_center(x, y));
            p.room = self.building.floor(floor).map_or(0, |m| m.room_at_tile(x, y));
        }
        self.give_new(id, item_kind::EMPLOYEE_CARD);
        self.give_new(id, item_kind::LAPTOP);
    }
}
