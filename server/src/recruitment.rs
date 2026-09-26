//! Recruitment (GDD section 4): job portal offers and a short quiz per offer.
//!
//! Everything is decided on the server: the client only sees the question
//! texts and the shuffled options, and sends back the chosen index.
//! Data: `server/data/recruitment.json` (the first option of every question
//! is the correct one; the server shuffles them for each attempt).

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::protocol::{OfferInfo, MAX_MAIL_BYTES, MAX_OPTIONS, MAX_TEXT_BYTES};

#[derive(Debug, Deserialize, Clone)]
pub struct Department {
    pub id: u8,
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Question {
    #[serde(rename = "q")]
    pub text: String,
    /// `options[0]` is correct.
    pub options: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Offer {
    pub id: u8,
    pub company: String,
    /// Only our startup hires; other companies just reply (or don't).
    #[serde(default = "yes")]
    pub hiring: bool,
    #[serde(default)]
    pub department: u8,
    /// Open positions at the start (our startup; more appear every morning).
    #[serde(default)]
    pub vacancies: u8,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub questions: Vec<Question>,
    /// Non-hiring companies: the reply mail (None = they never answer).
    #[serde(default)]
    pub reply: Option<String>,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct Recruitment {
    version: u32,
    pub questions_per_attempt: usize,
    pub pass_score: usize,
    /// Delay between the application and the reply mail.
    pub invite_delay_secs: u32,
    pub departments: Vec<Department>,
    pub offers: Vec<Offer>,
}

pub fn default_recruitment_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data/recruitment.json")
}

impl Recruitment {
    pub fn load(path: &Path) -> Result<Recruitment, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let r: Recruitment = serde_json::from_slice(&bytes).map_err(|e| format!("recruitment json: {e}"))?;
        r.validate()?;
        Ok(r)
    }

    fn validate(&self) -> Result<(), String> {
        if self.version != 2 {
            return Err(format!("unsupported recruitment version {}", self.version));
        }
        if self.pass_score == 0 || self.pass_score > self.questions_per_attempt {
            return Err("pass_score must be 1..=questions_per_attempt".into());
        }
        if self.offers.is_empty() {
            return Err("no job offers".into());
        }
        for o in &self.offers {
            let too_long = |s: &str| s.len() > MAX_TEXT_BYTES;
            if too_long(&o.title) || too_long(&o.description) || too_long(&o.company) {
                return Err(format!("offer {}: text longer than {MAX_TEXT_BYTES} bytes", o.id));
            }
            if !o.hiring {
                if o.reply.as_deref().is_some_and(|r| r.len() > MAX_MAIL_BYTES) {
                    return Err(format!("offer {}: reply too long", o.id));
                }
                continue;
            }
            if self.department_name(o.department).is_none() {
                return Err(format!("offer {}: unknown department {}", o.id, o.department));
            }
            if o.questions.len() < self.questions_per_attempt {
                return Err(format!("offer {}: needs at least {} questions", o.id, self.questions_per_attempt));
            }
            for q in &o.questions {
                if !(2..=MAX_OPTIONS).contains(&q.options.len()) {
                    return Err(format!("offer {}: '{}' needs 2..={MAX_OPTIONS} options", o.id, q.text));
                }
                if too_long(&q.text) || q.options.iter().any(|s| s.len() > 120) {
                    return Err(format!("offer {}: '{}' text too long", o.id, q.text));
                }
            }
        }
        Ok(())
    }

    pub fn offer(&self, id: u8) -> Option<&Offer> {
        self.offers.iter().find(|o| o.id == id)
    }

    pub fn department_name(&self, id: u8) -> Option<&str> {
        self.departments.iter().find(|d| d.id == id).map(|d| d.name.as_str())
    }

    /// Offers as shown on the job portal; `applied(id)` marks the ones this
    /// player has already applied for.
    pub fn portal(&self, applied: impl Fn(u8) -> bool) -> Vec<OfferInfo> {
        self.offers
            .iter()
            .map(|o| OfferInfo {
                id: o.id,
                department: o.department,
                applied: applied(o.id),
                vacancies: 0,
                company: o.company.clone(),
                title: o.title.clone(),
                description: o.description.clone(),
            })
            .collect()
    }

    /// Start an attempt: random questions of the offer, options shuffled.
    pub fn start(&self, offer_id: u8, number: u8, rng: &mut fastrand::Rng) -> Option<Attempt> {
        let offer = self.offer(offer_id).filter(|o| o.hiring)?;
        let mut picks: Vec<usize> = (0..offer.questions.len()).collect();
        rng.shuffle(&mut picks);
        let items = picks
            .into_iter()
            .take(self.questions_per_attempt)
            .map(|q| {
                let mut order: Vec<usize> = (0..offer.questions[q].options.len()).collect();
                rng.shuffle(&mut order);
                Item { question: q, order }
            })
            .collect();
        Some(Attempt { offer: offer_id, number, items, index: 0, score: 0 })
    }
}

#[derive(Debug, Clone)]
struct Item {
    question: usize,
    /// Shown position -> original option index (0 = correct).
    order: Vec<usize>,
}

/// One pass through the quiz for one offer.
#[derive(Debug, Clone)]
pub struct Attempt {
    pub offer: u8,
    /// Attempt counter (per player), lets the client ignore stale packets.
    pub number: u8,
    items: Vec<Item>,
    index: usize,
    score: usize,
}

/// Question as sent to the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    pub index: u8,
    pub total: u8,
    pub text: String,
    pub options: Vec<String>,
}

impl Attempt {
    pub fn finished(&self) -> bool {
        self.index >= self.items.len()
    }

    pub fn score(&self) -> usize {
        self.score
    }

    pub fn total(&self) -> usize {
        self.items.len()
    }

    /// The current question (None once finished).
    pub fn current(&self, r: &Recruitment) -> Option<Shown> {
        let item = self.items.get(self.index)?;
        let q = &r.offer(self.offer)?.questions[item.question];
        Some(Shown {
            index: self.index as u8,
            total: self.items.len() as u8,
            text: q.text.clone(),
            options: item.order.iter().map(|&i| q.options[i].clone()).collect(),
        })
    }

    /// Answer question `index` with shown option `choice`. Stale or invalid
    /// answers (wrong index, choice out of range) are ignored: returns false.
    pub fn answer(&mut self, index: u8, choice: u8) -> bool {
        let Some(item) = self.items.get(self.index) else { return false };
        if index as usize != self.index || choice as usize >= item.order.len() {
            return false;
        }
        if item.order[choice as usize] == 0 {
            self.score += 1;
        }
        self.index += 1;
        true
    }

    /// Shown index of the correct option of the current question (tests only).
    #[cfg(test)]
    fn correct_choice(&self) -> u8 {
        self.items[self.index].order.iter().position(|&i| i == 0).unwrap() as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r() -> Recruitment {
        Recruitment::load(&default_recruitment_path()).unwrap()
    }

    #[test]
    fn our_startup_hires_four_positions_others_only_reply() {
        let r = r();
        let ours: Vec<_> = r
            .offers
            .iter()
            .filter(|o| o.hiring)
            .map(|o| (o.company.as_str(), o.title.as_str(), r.department_name(o.department).unwrap()))
            .collect();
        assert_eq!(
            ours,
            vec![
                ("Startup Sim sp. z o.o.", "Programista/ka", "IT / Produkt"),
                ("Startup Sim sp. z o.o.", "Designer/ka", "IT / Produkt"),
                ("Startup Sim sp. z o.o.", "Specjalista/ka ds. sprzedaży", "Biznes"),
                ("Startup Sim sp. z o.o.", "Specjalista/ka ds. marketingu", "Biznes"),
            ]
        );
        let others: Vec<_> = r.offers.iter().filter(|o| !o.hiring).collect();
        assert!(others.len() >= 3);
        assert!(others.iter().any(|o| o.reply.is_some()) && others.iter().any(|o| o.reply.is_none()), "some reply, some stay silent");
        assert!(others.iter().all(|o| o.company != "Startup Sim sp. z o.o."));
        assert_eq!((r.questions_per_attempt, r.pass_score), (3, 2));
        assert!(r.start(10, 1, &mut fastrand::Rng::new()).is_none(), "no interview at other companies");
    }

    #[test]
    fn all_correct_passes_all_wrong_fails() {
        let r = r();
        let mut rng = fastrand::Rng::with_seed(3);
        let mut a = r.start(1, 1, &mut rng).unwrap();
        while !a.finished() {
            let i = a.current(&r).unwrap().index;
            let c = a.correct_choice();
            assert!(a.answer(i, c));
        }
        assert_eq!(a.score(), 3);

        let mut a = r.start(2, 2, &mut rng).unwrap();
        while !a.finished() {
            let i = a.current(&r).unwrap().index;
            let wrong = (a.correct_choice() + 1) % 4;
            assert!(a.answer(i, wrong));
        }
        assert_eq!(a.score(), 0);
    }

    #[test]
    fn stale_and_invalid_answers_are_ignored() {
        let r = r();
        let mut a = r.start(1, 1, &mut fastrand::Rng::with_seed(1)).unwrap();
        assert!(!a.answer(1, 0), "not the current question");
        assert!(!a.answer(0, 9), "no such option");
        assert!(a.answer(0, 0));
        assert!(!a.answer(0, 0), "repeated answer (lost ack) doesn't count twice");
        assert_eq!(a.current(&r).unwrap().index, 1);
    }

    #[test]
    fn questions_differ_between_attempts_and_options_are_shuffled() {
        let r = r();
        let mut rng = fastrand::Rng::with_seed(9);
        let mut firsts = std::collections::HashSet::new();
        let mut correct_positions = std::collections::HashSet::new();
        for n in 0..30 {
            let a = r.start(1, n, &mut rng).unwrap();
            firsts.insert(a.current(&r).unwrap().text);
            correct_positions.insert(a.correct_choice());
        }
        assert!(firsts.len() > 3, "random questions");
        assert!(correct_positions.len() > 1, "correct answer isn't always first");
    }

    #[test]
    fn unknown_offer_is_rejected() {
        assert!(r().start(99, 1, &mut fastrand::Rng::new()).is_none());
    }
}
