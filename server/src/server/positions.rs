//! Our startup's positions (job openings): they start as the file's hiring
//! offers; the founder adds, edits and removes them in the company panel.
//! Other companies' offers stay in the file (they only reply to mail).

use crate::company::{self, Position};
use crate::protocol::{self as proto, CompanyOffer, Packet};
use crate::recruitment::Recruitment;

use super::player::Stage;
use super::Server;

/// Bytes of positions per CompanyOffers part.
const OFFERS_PART_BYTES: usize = 900;

/// The starting positions: the file's offers of our startup.
pub(super) fn from_file(r: &Recruitment) -> Vec<Position> {
    r.offers
        .iter()
        .filter(|o| o.hiring)
        .map(|o| Position {
            id: o.id,
            title: o.title.clone(),
            department: o.department,
            set: o.set.clone(),
            description: o.description.clone(),
            places: o.vacancies,
            salary: o.salary,
        })
        .collect()
}

pub mod lines {
    pub const LIMIT: &str = "Firma ma już 10 stanowisk — usuń któreś, zanim dodasz nowe.";
    pub const BAD_TITLE: &str = "Nazwa stanowiska: 3–40 znaków.";
    pub const BAD_DEPARTMENT: &str = "Nie ma takiego działu (Zarząd nie rekrutuje).";
    pub const BAD_SET: &str = "Nie ma takiego zestawu pytań.";
}

impl Server {
    pub(super) fn position(&self, id: u8) -> Option<&Position> {
        self.positions.iter().find(|p| p.id == id)
    }

    pub(super) fn position_mut(&mut self, id: u8) -> Option<&mut Position> {
        self.positions.iter_mut().find(|p| p.id == id)
    }

    /// Open places of a position (0 if there's no such position).
    pub(super) fn places(&self, id: u8) -> u8 {
        self.position(id).map_or(0, |p| p.places)
    }

    /// Title of a position of ours or of another company's offer.
    pub(super) fn job_title(&self, id: u8) -> Option<String> {
        self.position(id).map(|p| p.title.clone()).or_else(|| self.cfg.recruitment.offer(id).map(|o| o.title.clone()))
    }

    /// The founder's panel: `text` = "title\nset\ndescription".
    pub(super) fn add_position(&mut self, department: u8, text: &str) -> Result<u8, &'static str> {
        if self.positions.len() >= company::MAX_POSITIONS {
            return Err(lines::LIMIT);
        }
        let mut parts = text.splitn(3, '\n');
        let title = company::clean(parts.next().unwrap_or(""), company::TITLE_MIN, company::TITLE_MAX).ok_or(lines::BAD_TITLE)?;
        let set = parts.next().unwrap_or("").trim().to_string();
        let description = parts.next().and_then(|d| company::clean(d, 1, company::DESCRIPTION_MAX)).unwrap_or_default();
        if !company::position_department(&self.cfg.recruitment, department) {
            return Err(lines::BAD_DEPARTMENT);
        }
        if self.cfg.recruitment.set(&set).is_none() {
            return Err(lines::BAD_SET);
        }
        // A fresh id, never one of another company's offers.
        let id = (company::FIRST_CUSTOM_ID..=u8::MAX)
            .find(|&i| self.position(i).is_none() && self.cfg.recruitment.offer(i).is_none())
            .ok_or(lines::LIMIT)?;
        self.log(format!("* company: new position '{title}'"));
        self.positions.push(Position { id, title, department, set, description, places: 1, salary: crate::pay::DEFAULT_RANGE });
        self.save_soon = true;
        Ok(id)
    }

    /// Remove a position: people still in its recruitment are told it's
    /// closed; those hired for it stay.
    pub(super) fn remove_position(&mut self, id: u8) {
        let Some(i) = self.positions.iter().position(|p| p.id == id) else { return };
        let title = self.positions[i].title.clone();
        let waiting: Vec<u16> = self
            .players
            .values()
            .filter(|p| match &p.stage {
                Stage::Portal(d) => {
                    d.pending.iter().any(|(o, _)| *o == id)
                        || d.invited.contains(&id)
                        || d.applied.contains(&id)
                        || d.attempt.as_ref().is_some_and(|a| a.offer == id)
                }
                _ => false,
            })
            .map(|p| p.id)
            .collect();
        let candidates: Vec<u16> = self.company.candidates.iter().filter(|c| c.offer == id).map(|c| c.player).collect();
        self.company.candidates.retain(|c| c.offer != id);
        for pid in waiting.into_iter().chain(candidates) {
            self.position_closed_mail(pid, id, &title);
        }
        self.positions.remove(i);
        self.log(format!("* company: position '{title}' removed"));
        self.save_soon = true;
    }

    /// "The position is closed" — and its recruitment ends for this player.
    fn position_closed_mail(&mut self, pid: u16, offer: u8, title: &str) {
        let from = format!("{} — Rekrutacja", self.company.name);
        let Some(p) = self.players.get_mut(&pid) else { return };
        let nick = p.nick.clone();
        let Stage::Portal(desk) = &mut p.stage else { return };
        desk.pending.retain(|(o, _)| *o != offer);
        desk.invited.retain(|o| *o != offer);
        desk.applied.retain(|o| *o != offer);
        if desk.attempt.as_ref().is_some_and(|a| a.offer == offer) {
            desk.attempt = None;
        }
        if desk.awaiting == Some(offer) {
            desk.awaiting = None;
        }
        desk.mail(
            &from,
            format!("Rekrutacja zakończona: {title}"),
            format!("Cześć {nick},\n\nzamknęliśmy rekrutację na stanowisko {title}. Dziękujemy za zainteresowanie — zajrzyj na portal, może inne stanowisko będzie dla Ciebie!\n\nZespół rekrutacji"),
            proto::portal_action::NONE,
            0,
        );
        self.send_portal(pid, true);
    }

    /// The founder's panel: the positions (and the question sets to choose
    /// from), in parts that fit a datagram.
    pub(super) fn company_offer_packets(&self) -> Vec<Packet> {
        let sets: Vec<(String, String)> = self.cfg.recruitment.question_sets.iter().map(|s| (s.id.clone(), s.name.clone())).collect();
        let mut parts: Vec<Vec<CompanyOffer>> = vec![Vec::new()];
        let mut used: usize = sets.iter().map(|(a, b)| a.len() + b.len() + 3).sum();
        for p in &self.positions {
            let size = 8 + p.set.len() + p.title.len() + p.description.len();
            if used + size > OFFERS_PART_BYTES && !parts.last().is_some_and(|x| x.is_empty()) {
                parts.push(Vec::new());
                used = 0;
            }
            used += size;
            if let Some(last) = parts.last_mut() {
                last.push(CompanyOffer {
                    id: p.id,
                    places: p.places,
                    department: p.department,
                    set: p.set.clone(),
                    title: p.title.clone(),
                    description: p.description.clone(),
                });
            }
        }
        let n = parts.len().min(255) as u8;
        parts
            .into_iter()
            .enumerate()
            .map(|(i, offers)| Packet::CompanyOffers {
                name: self.company.name.clone(),
                part: i as u8,
                parts: n,
                sets: if i == 0 { sets.clone() } else { Vec::new() },
                offers,
            })
            .collect()
    }
}
