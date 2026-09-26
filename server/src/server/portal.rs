//! The job portal at home: offers, applications, mail, online interview.

use crate::company;
use crate::inventory::kind as item_kind;
use crate::protocol::{self as proto, Packet};

use super::player::{Desk, Stage};
use super::{Server, TICK_HZ};

/// At most this many open positions per job offer.
pub(super) const MAX_VACANCIES: u8 = 3;

/// Resend the current portal screen this often (ticks) - UDP may drop it.
const PORTAL_RESEND_TICKS: u32 = 20;

impl Server {
    /// Players at home on the desktop: replies that came in, and a periodic
    /// resend of the screen.
    pub(super) fn tick_portals(&mut self, ids: &[u16]) {
        for &id in ids {
            self.deliver_replies(id);
            if self.tick.is_multiple_of(PORTAL_RESEND_TICKS) {
                self.send_portal(id, self.tick.is_multiple_of(2 * PORTAL_RESEND_TICKS));
            }
        }
    }

    /// (Re)send the desktop state: portal offers, current interview
    /// question and (with `mails`) the whole inbox. Clients dedupe.
    pub(super) fn send_portal(&mut self, id: u16, mails: bool) {
        let Some(p) = self.players.get(&id) else { return };
        let Stage::Portal(desk) = &p.stage else { return };
        let r = &self.cfg.recruitment;
        // Offers may not fit one datagram: split them (the client merges by id).
        let mut packets = Vec::new();
        let mut chunk = Vec::new();
        let mut size = proto::HEADER_LEN + 1;
        for mut offer in r.portal(|o| desk.applied.contains(&o)) {
            // Our startup's positions carry the number of free places; the
            // portal hides the ones with none (unless you already applied).
            if let Some(&free) = self.vacancies.get(&offer.id) {
                offer.vacancies = free;
                offer.company = self.company.name.clone();
                if let Some(d) = self.company.descriptions.get(&offer.id) {
                    offer.description = d.clone();
                }
            }
            let len = 4 + 6 + offer.company.len() + offer.title.len() + offer.description.len();
            if size + len > proto::MAX_PACKET && !chunk.is_empty() {
                packets.push(Packet::JobOffers { offers: std::mem::take(&mut chunk) });
                size = proto::HEADER_LEN + 1;
            }
            size += len;
            chunk.push(offer);
        }
        packets.push(Packet::JobOffers { offers: chunk });
        if let Some(q) = desk.attempt.as_ref().and_then(|a| a.current(r).map(|q| (a.number, q))) {
            let (attempt, q) = q;
            packets.push(Packet::Question { attempt, index: q.index, total: q.total, text: q.text, options: q.options });
        }
        if mails {
            for m in &desk.inbox {
                packets.push(Packet::Mail {
                    id: m.id,
                    from: m.from.clone(),
                    subject: m.subject.clone(),
                    body: m.body.clone(),
                    action: m.action,
                    arg: m.arg,
                });
            }
        }
        let addr = p.addr;
        for packet in packets {
            self.send(addr, &packet);
        }
    }

    pub(super) fn desk(&mut self, id: u16) -> Option<&mut Desk> {
        match &mut self.players.get_mut(&id)?.stage {
            Stage::Portal(d) => Some(d),
            Stage::Working | Stage::Home { .. } => None,
        }
    }

    pub(super) fn handle_apply(&mut self, id: u16, offer: u8) {
        let due = self.tick + self.cfg.recruitment.invite_delay_secs * TICK_HZ;
        let Some(o) = self.cfg.recruitment.offer(offer).map(|o| (o.hiring, o.reply.is_some())) else { return };
        if o.0 && self.vacancies.get(&offer).copied().unwrap_or(0) == 0 {
            self.send_portal(id, false); // filled in the meantime
            return;
        }
        let Some(desk) = self.desk(id) else { return };
        if desk.applied.contains(&offer) || desk.hired.is_some() || desk.awaiting.is_some() {
            return; // duplicate (resent) application
        }
        desk.applied.push(offer);
        if o.0 || o.1 {
            desk.pending.push((offer, due)); // other companies without a reply: silence
        }
        self.send_portal(id, false);
    }

    /// Replies that are due: interview invitations / other companies' answers.
    pub(super) fn deliver_replies(&mut self, id: u16) {
        let from = format!("{} — Rekrutacja", self.company.name);
        let tick = self.tick;
        let Some(p) = self.players.get_mut(&id) else { return };
        let nick = p.nick.clone();
        let Stage::Portal(desk) = &mut p.stage else { return };
        let due: Vec<u8> = desk.pending.iter().filter(|(_, t)| *t <= tick).map(|(o, _)| *o).collect();
        if due.is_empty() {
            return;
        }
        desk.pending.retain(|(_, t)| *t > tick);
        for offer in due {
            let Some(o) = self.cfg.recruitment.offer(offer) else { continue };
            if o.hiring {
                desk.invited.push(offer);
                desk.mail(
                    &from,
                    format!("Zaproszenie na rozmowę: {}", o.title),
                    format!(
                        "Cześć {nick}!\n\nDziękujemy za zgłoszenie na stanowisko {}. Zapraszamy na krótką rozmowę online — \
                         kilka pytań, zero stresu (prawie). Kliknij „Dołącz do rozmowy”, kiedy tylko możesz.\n\nZespół rekrutacji",
                        o.title
                    ),
                    proto::portal_action::JOIN_INTERVIEW,
                    offer,
                );
            } else if let Some(reply) = &o.reply {
                desk.mail(&o.company, format!("Re: {}", o.title), reply.clone(), proto::portal_action::NONE, 0);
            }
        }
        self.send_portal(id, true);
    }

    pub(super) fn handle_portal_action(&mut self, id: u16, action: u8, arg: u8) {
        match action {
            proto::portal_action::JOIN_INTERVIEW => {
                let Some(p) = self.players.get_mut(&id) else { return };
                let Stage::Portal(desk) = &mut p.stage else { return };
                if !desk.invited.contains(&arg) || desk.attempt.is_some() || desk.hired.is_some() || desk.awaiting.is_some() {
                    return;
                }
                if self.vacancies.get(&arg).copied().unwrap_or(0) == 0 {
                    self.position_filled_mail(id, arg);
                    return;
                }
                p.attempts = p.attempts.wrapping_add(1);
                desk.attempt = self.cfg.recruitment.start(arg, p.attempts, &mut self.rng);
                self.send_portal(id, false);
            }
            proto::portal_action::GO_TO_OFFICE => {
                let Some(p) = self.players.get_mut(&id) else { return };
                let Stage::Portal(desk) = &p.stage else { return };
                let Some(dept) = desk.hired else { return };
                // Hired: a new day - the first one at work. At night you come
                // in the morning (random arrival, like everybody).
                p.day += 1;
                p.stage = if self.clock.is_night() { Stage::Home { arrive_at: None } } else { Stage::Working };
                p.department = dept;
                self.clock_dirty = true;
                let msg = format!(
                    "* player {id} '{}' goes to the office: {}",
                    p.nick,
                    self.cfg.recruitment.department_name(dept).unwrap_or("?")
                );
                self.log(msg);
                if self.cfg.start_access & crate::map::access::CARD != 0 {
                    self.give_new(id, item_kind::EMPLOYEE_CARD); // load tests: straight in with a card
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_answer(&mut self, id: u16, attempt_no: u8, index: u8, choice: u8) {
        let from = format!("{} — Rekrutacja", self.company.name);
        let Some(p) = self.players.get_mut(&id) else { return };
        let nick = p.nick.clone();
        let addr = p.addr;
        let Stage::Portal(desk) = &mut p.stage else { return };
        let Some(a) = desk.attempt.as_mut() else { return };
        if a.number != attempt_no || !a.answer(index, choice) {
            return; // stale / duplicate: the resend loop shows the current state
        }
        if !a.finished() {
            self.send_portal(id, false);
            return;
        }
        let r = &self.cfg.recruitment;
        let (score, total, offer) = (a.score(), a.total(), a.offer);
        // First come, first served: the place may have gone meanwhile.
        let free = self.vacancies.get(&offer).copied().unwrap_or(0);
        let filled_meanwhile = score >= r.pass_score && free == 0;
        let passed = score >= r.pass_score && free > 0;
        let Some(o) = r.offer(offer) else { return }; // attempts are only started for known offers
        let result = Packet::RecruitResult {
            attempt: attempt_no,
            passed,
            score: small(score),
            total: small(total),
            department: if passed { o.department } else { 0 },
        };
        desk.attempt = None;
        desk.invited.retain(|x| *x != offer);
        if filled_meanwhile {
            desk.applied.retain(|x| *x != offer);
            desk.mail(
                &from,
                format!("Stanowisko obsadzone: {}", o.title),
                format!(
                    "Cześć {nick},\n\nrozmowa poszła dobrze ({score}/{total}), ale ktoś był szybszy — to stanowisko \
                     zostało już obsadzone. Zajrzyj na portal: nowe miejsca pojawiają się co rano.\n\nZespół rekrutacji"
                ),
                proto::portal_action::NONE,
                0,
            );
        } else if passed {
            // Hired right away, or the founder decides (see below).
        } else {
            desk.applied.retain(|x| *x != offer); // may apply again
            desk.mail(
                &from,
                format!("Dziękujemy za rozmowę: {}", o.title),
                format!(
                    "Cześć {nick},\n\ndziękujemy za rozmowę ({score}/{total}). Tym razem szukamy kogoś innego, ale nie \
                     przejmuj się — zapraszamy do ponownej aplikacji. Pytania będą inne!\n\nZespół rekrutacji"
                ),
                proto::portal_action::NONE,
                0,
            );
        }
        self.send(addr, &result);
        self.send(addr, &result); // tiny packet; a duplicate makes loss unlikely
        if passed {
            if self.founder_online() {
                let since = self.clock.total_minutes();
                let company = self.company.name.clone();
                self.company.candidates.push(company::Candidate { player: id, offer, score: small(score), total: small(total), since });
                if let Some(Stage::Portal(desk)) = self.players.get_mut(&id).map(|p| &mut p.stage) {
                    desk.awaiting = Some(offer);
                    desk.mail(&from, "Decyzja zarządu wkrótce".into(), company::lines::awaiting(&nick, &company), proto::portal_action::NONE, 0);
                }
            } else {
                self.hire(id, offer);
            }
        }
        self.send_portal(id, true);
    }
}

impl Server {
    /// Somebody got the job: one place fewer; if none is left, everybody
    /// else still in that recruitment hears it's filled.
    pub(super) fn take_vacancy(&mut self, offer: u8) {
        let Some(free) = self.vacancies.get_mut(&offer) else { return };
        *free = free.saturating_sub(1);
        let left = *free;
        if let Some(title) = self.cfg.recruitment.offer(offer).map(|o| o.title.clone()) {
            self.log(format!("* recruitment: {title} filled ({left} left)"));
        }
        if left > 0 {
            return;
        }
        let waiting: Vec<u16> = self
            .players
            .values()
            .filter(|p| match &p.stage {
                Stage::Portal(d) => {
                    d.pending.iter().any(|(o, _)| *o == offer)
                        || d.invited.contains(&offer)
                        || d.attempt.as_ref().is_some_and(|a| a.offer == offer)
                }
                _ => false,
            })
            .map(|p| p.id)
            .collect();
        for pid in waiting {
            self.position_filled_mail(pid, offer);
        }
    }

    /// "Sorry, the position has been filled" - and the recruitment for it ends.
    pub(super) fn position_filled_mail(&mut self, pid: u16, offer: u8) {
        let from = format!("{} — Rekrutacja", self.company.name);
        let Some(title) = self.cfg.recruitment.offer(offer).map(|o| o.title.clone()) else { return };
        let Some(p) = self.players.get_mut(&pid) else { return };
        let nick = p.nick.clone();
        let Stage::Portal(desk) = &mut p.stage else { return };
        desk.pending.retain(|(o, _)| *o != offer);
        desk.invited.retain(|o| *o != offer);
        desk.applied.retain(|o| *o != offer);
        if desk.attempt.as_ref().is_some_and(|a| a.offer == offer) {
            desk.attempt = None;
        }
        desk.mail(
            &from,
            format!("Stanowisko obsadzone: {title}"),
            format!(
                "Cześć {nick},\n\ndziękujemy za zainteresowanie — niestety stanowisko {title} zostało już obsadzone. \
                 Nowe miejsca pojawiają się na portalu co rano, zajrzyj jutro!\n\nZespół rekrutacji"
            ),
            proto::portal_action::NONE,
            0,
        );
        self.send_portal(pid, true);
    }

    /// A new day: the startup opens one more position (max 3 per offer).
    pub(super) fn open_vacancy(&mut self) {
        let open: Vec<u8> = self.vacancies.iter().filter(|(_, &n)| n < MAX_VACANCIES).map(|(&o, _)| o).collect();
        if open.is_empty() {
            return;
        }
        let offer = open[self.rng.usize(..open.len())];
        let Some(free) = self.vacancies.get_mut(&offer) else { return };
        *free += 1;
        let free = *free;
        if let Some(o) = self.cfg.recruitment.offer(offer) {
            self.log(format!("* recruitment: new opening - {} ({free} free)", o.title));
        }
    }
}

/// A score / question count for the wire.
fn small<T: TryInto<u8>>(n: T) -> u8 {
    n.try_into().unwrap_or(u8::MAX)
}
