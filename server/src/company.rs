//! The company and its founder (backlog 9b, without payments for now).
//!
//! On a server without a founder the first player to press "Załóż firmę" on
//! the job portal founds the company: picks its name, joins the board and
//! gets the company panel on their computer - job openings (places and
//! descriptions), candidates who passed the interview (hire / reject) and
//! staff (fire). Without a founder (or when they don't decide in time) the
//! interview alone decides, as before.

use std::collections::HashMap;

/// Board department (the founder's).
pub const BOARD_DEPARTMENT: u8 = 3;
/// A candidate waits this long (game minutes) for the founder's decision;
/// then the interview result decides.
pub const DECISION_MINUTES: u32 = 30;
/// Places per opening the founder can set.
pub const MAX_PLACES: u8 = 5;
pub const NAME_MIN: usize = 3;
pub const NAME_MAX: usize = 40;
pub const DESCRIPTION_MAX: usize = 200;

/// `CompanyAction::action`.
pub mod action {
    /// From the job portal: found the company (`text` = name).
    pub const FOUND: u8 = 1;
    pub const RENAME: u8 = 2;
    /// Places for offer `target` = `value`.
    pub const SET_PLACES: u8 = 3;
    /// Description of offer `target` = `text`.
    pub const SET_DESCRIPTION: u8 = 4;
    /// Candidate `target` (player id).
    pub const HIRE: u8 = 5;
    pub const REJECT: u8 = 6;
    /// Employee `target`.
    pub const FIRE: u8 = 7;
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub player: u16,
    pub offer: u8,
    pub score: u8,
    pub total: u8,
    /// Game minute (`Clock::total_minutes`) they started waiting.
    pub since: u32,
}

#[derive(Debug, Clone)]
pub struct Company {
    pub name: String,
    pub founder: Option<u16>,
    /// Custom job descriptions (offer id -> text).
    pub descriptions: HashMap<u8, String>,
    /// Passed the interview, waiting for the founder.
    pub candidates: Vec<Candidate>,
    /// Employee -> world day they were hired.
    pub hired_on: HashMap<u16, u32>,
}

impl Company {
    pub fn new(name: &str) -> Company {
        Company { name: name.into(), founder: None, descriptions: HashMap::new(), candidates: Vec::new(), hired_on: HashMap::new() }
    }
}

/// Clean up a company name / description; None if it's not acceptable.
pub fn clean(text: &str, min: usize, max: usize) -> Option<String> {
    let t: String = text.chars().filter(|c| !c.is_control()).take(max).collect();
    let t = t.trim().to_string();
    (t.chars().count() >= min).then_some(t)
}

pub mod lines {
    pub fn awaiting(nick: &str, company: &str) -> String {
        format!(
            "Cześć {nick},\n\nświetny wynik rozmowy! O zatrudnieniu zdecyduje zarząd {company} — odezwiemy się \
             wkrótce (najpóźniej za pół godziny).\n\nZespół rekrutacji"
        )
    }
    pub fn rejected(nick: &str, company: &str) -> String {
        format!(
            "Cześć {nick},\n\nzarząd {company} zdecydował się tym razem na inną osobę. Dziękujemy i trzymamy kciuki \
             — zajrzyj na portal jeszcze raz.\n\nZespół rekrutacji"
        )
    }
    pub fn fired(nick: &str, company: &str) -> String {
        format!(
            "Cześć {nick},\n\nz przykrością informujemy, że {company} rozwiązuje z Tobą umowę. Kartę i laptop \
             oddajesz na recepcji (już to zrobiliśmy). Powodzenia w dalszych poszukiwaniach!\n\nZarząd"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_cleaned_and_checked() {
        assert_eq!(clean("  Pixel Pierogi sp. z o.o. ", NAME_MIN, NAME_MAX).as_deref(), Some("Pixel Pierogi sp. z o.o."));
        assert_eq!(clean("ab", NAME_MIN, NAME_MAX), None);
        assert_eq!(clean(&"x".repeat(100), NAME_MIN, NAME_MAX).unwrap().len(), NAME_MAX);
        assert_eq!(clean("Firma\n\u{7}X", NAME_MIN, NAME_MAX).as_deref(), Some("FirmaX"));
    }
}
