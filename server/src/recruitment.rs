//! Recruitment (GDD section 4): job portal offers and a short quiz per offer.
//!
//! Everything is decided on the server: the client only sees the question
//! texts and the shuffled options, and sends back the chosen index.
//! Data: `server/data/recruitment.json`: question sets (the first option of
//! every question is the correct one; the server shuffles them for each
//! attempt), the departments and the offers. Our startup's offers are only
//! the starting positions — the founder edits them in the game
//! (`server/positions.rs`); each position names the question set it uses.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::protocol::{OfferInfo, MAX_MAIL_BYTES, MAX_OPTIONS, MAX_TEXT_BYTES};

#[derive(Debug, Deserialize, Clone)]
pub struct Department {
    pub id: u8,
    pub name: String,
    /// Shown next to a nick ("Ola · IT"); the name if missing.
    #[serde(default)]
    pub short: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Question {
    #[serde(rename = "q")]
    pub text: String,
    /// `options[0]` is correct.
    pub options: Vec<String>,
}

/// A named pool of interview questions ("programming", "general"...).
#[derive(Debug, Deserialize, Clone)]
pub struct QuestionSet {
    pub id: String,
    /// Shown in the founder's panel.
    pub name: String,
    pub questions: Vec<Question>,
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
    /// Pay range: zł a month gross, [min, max].
    #[serde(default)]
    pub salary: [u32; 2],
    /// Our startup: the question set of the interview.
    #[serde(default)]
    pub set: String,
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
    pub question_sets: Vec<QuestionSet>,
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
        if self.version != 3 {
            return Err(format!("unsupported recruitment version {}", self.version));
        }
        if self.pass_score == 0 || self.pass_score > self.questions_per_attempt {
            return Err("pass_score must be 1..=questions_per_attempt".into());
        }
        if self.offers.is_empty() {
            return Err("no job offers".into());
        }
        let too_long = |s: &str| s.len() > MAX_TEXT_BYTES;
        for s in &self.question_sets {
            if s.questions.len() < self.questions_per_attempt {
                return Err(format!("question set {}: needs at least {} questions", s.id, self.questions_per_attempt));
            }
            for q in &s.questions {
                if !(2..=MAX_OPTIONS).contains(&q.options.len()) {
                    return Err(format!("question set {}: '{}' needs 2..={MAX_OPTIONS} options", s.id, q.text));
                }
                if too_long(&q.text) || q.options.iter().any(|o| o.len() > 120) {
                    return Err(format!("question set {}: '{}' text too long", s.id, q.text));
                }
            }
        }
        for o in &self.offers {
            if too_long(&o.title) || too_long(&o.description) || too_long(&o.company) {
                return Err(format!("offer {}: text longer than {MAX_TEXT_BYTES} bytes", o.id));
            }
            if !o.hiring {
                if o.reply.as_deref().is_some_and(|r| r.len() > MAX_MAIL_BYTES) {
                    return Err(format!("offer {}: reply too long", o.id));
                }
                continue;
            }
            if o.salary[0] == 0 || o.salary[0] > o.salary[1] {
                return Err(format!("offer {}: needs a pay range [min, max]", o.id));
            }
            if self.department_name(o.department).is_none() {
                return Err(format!("offer {}: unknown department {}", o.id, o.department));
            }
            if self.set(&o.set).is_none() {
                return Err(format!("offer {}: unknown question set '{}'", o.id, o.set));
            }
        }
        Ok(())
    }

    pub fn offer(&self, id: u8) -> Option<&Offer> {
        self.offers.iter().find(|o| o.id == id)
    }

    pub fn set(&self, id: &str) -> Option<&QuestionSet> {
        self.question_sets.iter().find(|s| s.id == id)
    }

    pub fn department_name(&self, id: u8) -> Option<&str> {
        self.departments.iter().find(|d| d.id == id).map(|d| d.name.as_str())
    }

    /// The departments for the clients (`Departments`).
    pub fn department_list(&self) -> Vec<crate::protocol::DepartmentInfo> {
        self.departments
            .iter()
            .map(|d| crate::protocol::DepartmentInfo {
                id: d.id,
                short: if d.short.is_empty() { d.name.clone() } else { d.short.clone() },
                name: d.name.clone(),
            })
            .collect()
    }

    /// Other companies' offers as shown on the job portal (our startup's
    /// positions are added by the server); `applied(id)` marks the ones this
    /// player has already applied for.
    pub fn portal(&self, applied: impl Fn(u8) -> bool) -> Vec<OfferInfo> {
        self.offers
            .iter()
            .filter(|o| !o.hiring)
            .map(|o| OfferInfo {
                id: o.id,
                department: o.department,
                applied: applied(o.id),
                vacancies: 0,
                salary_min: o.salary[0],
                salary_max: o.salary[1],
                company: o.company.clone(),
                title: o.title.clone(),
                description: o.description.clone(),
            })
            .collect()
    }

    /// Start an attempt: random questions of the offer, options shuffled.
    /// Questions this character hasn't had yet come first (`seen`: their
    /// ids, `question_id`, updated here); once the whole pool has been
    /// asked, a new round starts.
    pub fn start(&self, set: &str, offer_id: u8, number: u8, rng: &mut fastrand::Rng, seen: &mut Vec<u32>) -> Option<Attempt> {
        let qs = self.set(set)?;
        let id = |i: usize| question_id(&qs.questions[i].text);
        let (mut fresh, mut old): (Vec<usize>, Vec<usize>) = (0..qs.questions.len()).partition(|&i| !seen.contains(&id(i)));
        rng.shuffle(&mut fresh);
        rng.shuffle(&mut old);
        let picks: Vec<usize> = fresh.into_iter().chain(old).take(self.questions_per_attempt).collect();
        let all: Vec<u32> = (0..qs.questions.len()).map(id).collect();
        seen.retain(|s| all.contains(s)); // questions removed from the file
        seen.extend(picks.iter().map(|&i| id(i)).filter(|q| !seen.contains(q)).collect::<Vec<_>>());
        if seen.len() >= all.len() {
            // Everything asked: a new round, without the ones just asked.
            seen.clear();
            seen.extend(picks.iter().map(|&i| id(i)));
        }
        let items = picks
            .into_iter()
            .take(self.questions_per_attempt)
            .map(|q| {
                let mut order: Vec<usize> = (0..qs.questions[q].options.len()).collect();
                rng.shuffle(&mut order);
                Item { question: q, order }
            })
            .collect();
        Some(Attempt { offer: offer_id, set: set.to_string(), number, items, index: 0, score: 0 })
    }
}

/// A question's stable id (its text), so adding or reordering questions in
/// the file doesn't mix up who has seen what.
pub fn question_id(text: &str) -> u32 {
    crc32fast::hash(text.as_bytes())
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
    /// The question set the questions come from.
    pub set: String,
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
        let q = &r.set(&self.set)?.questions[item.question];
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
                ("Startup Sim sp. z o.o.", "Programista/ka", "Produkt / IT"),
                ("Startup Sim sp. z o.o.", "Designer/ka", "Produkt / IT"),
                ("Startup Sim sp. z o.o.", "Specjalista/ka ds. sprzedaży", "Sales"),
                ("Startup Sim sp. z o.o.", "Specjalista/ka ds. marketingu", "Marketing"),
            ]
        );
        let others: Vec<_> = r.offers.iter().filter(|o| !o.hiring).collect();
        assert!(others.len() >= 3);
        assert!(others.iter().any(|o| o.reply.is_some()) && others.iter().any(|o| o.reply.is_none()), "some reply, some stay silent");
        assert!(others.iter().all(|o| o.company != "Startup Sim sp. z o.o."));
        assert_eq!((r.questions_per_attempt, r.pass_score), (3, 2));
        assert!(r.start("nope", 10, 1, &mut fastrand::Rng::new(), &mut Vec::new()).is_none(), "no such question set");
        assert!(r.offers.iter().filter(|o| o.hiring).all(|o| r.set(&o.set).is_some()), "every position has its set");
    }

    #[test]
    fn all_correct_passes_all_wrong_fails() {
        let r = r();
        let mut rng = fastrand::Rng::with_seed(3);
        let mut a = r.start("programming", 1, 1, &mut rng, &mut Vec::new()).unwrap();
        while !a.finished() {
            let i = a.current(&r).unwrap().index;
            let c = a.correct_choice();
            assert!(a.answer(i, c));
        }
        assert_eq!(a.score(), 3);

        let mut a = r.start("design", 2, 2, &mut rng, &mut Vec::new()).unwrap();
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
        let mut a = r.start("programming", 1, 1, &mut fastrand::Rng::with_seed(1), &mut Vec::new()).unwrap();
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
            let a = r.start("programming", 1, n, &mut rng, &mut Vec::new()).unwrap();
            firsts.insert(a.current(&r).unwrap().text);
            correct_positions.insert(a.correct_choice());
        }
        assert!(firsts.len() > 3, "random questions");
        assert!(correct_positions.len() > 1, "correct answer isn't always first");
    }

    #[test]
    fn no_repeats_until_the_whole_pool_was_asked() {
        let r = r();
        let pool = r.set("programming").unwrap().questions.len();
        assert!(pool >= 20, "a big enough pool: {pool}");
        let mut rng = fastrand::Rng::with_seed(3);
        let mut seen = Vec::new();
        let mut asked = Vec::new();
        let texts = |a: &mut Attempt| {
            let mut t = Vec::new();
            while let Some(q) = a.current(&r) {
                t.push(q.text);
                let i = q.index;
                a.answer(i, 0);
            }
            t
        };
        // Enough attempts to ask every question once: all different.
        for n in 0..pool / r.questions_per_attempt {
            let mut a = r.start("programming", 1, n as u8, &mut rng, &mut seen).unwrap();
            asked.extend(texts(&mut a));
        }
        let unique: std::collections::HashSet<_> = asked.iter().collect();
        assert_eq!(unique.len(), asked.len(), "no question twice within a round");
        // Then a new round starts (it never runs dry).
        for n in 0..10 {
            let mut a = r.start("programming", 1, n, &mut rng, &mut seen).unwrap();
            assert_eq!(texts(&mut a).len(), r.questions_per_attempt);
        }
    }

    #[test]
    fn unknown_offer_is_rejected() {
        assert!(r().start("", 99, 1, &mut fastrand::Rng::new(), &mut Vec::new()).is_none());
    }
}
