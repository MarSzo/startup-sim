//! Meetings with the board (backlog 9b): book a slot in the calendar app on
//! your computer, come to the board room on time (the door opens only then),
//! talk to the CEO or the co-founder.
//!
//! The calendar books for the computer's *owner* (like the messenger), so an
//! unlocked computer lets others book meetings in your name.

pub mod topic {
    pub const NONE: u8 = 0;
    /// Ask the CEO for a raise.
    pub const RAISE: u8 = 1;
    /// Pitch a product idea to the co-founder.
    pub const IDEA: u8 = 2;
    /// Complain to the CEO.
    pub const COMPLAINT: u8 = 3;
    /// Small talk with the CEO.
    pub const CHAT: u8 = 4;
}

pub fn topic_name(t: u8) -> &'static str {
    match t {
        topic::RAISE => "Prośba o podwyżkę",
        topic::IDEA => "Pomysł na produkt",
        topic::COMPLAINT => "Skarga / problem",
        topic::CHAT => "Luźna rozmowa",
        _ => "?",
    }
}

/// Who you talk to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Who {
    Ceo,
    CoFounder,
}

pub fn who(t: u8) -> Who {
    if t == topic::IDEA {
        Who::CoFounder
    } else {
        Who::Ceo
    }
}

/// Calendar: 30-minute slots, 10:00 - 17:30.
pub const SLOT_MIN: u32 = 30;
pub const FIRST_SLOT: u32 = 10 * 60;
pub const LAST_SLOT: u32 = 17 * 60 + 30;
/// The door opens this many minutes before / after the start.
pub const GRACE: u32 = 10;
/// Book at least this many minutes ahead.
pub const BOOK_AHEAD: u32 = 10;
/// A raise: +5 zł per game hour (grosze); ask at most every 3 days.
pub const RAISE_STEP: i64 = 5_00;
pub const RAISE_COOLDOWN_DAYS: u32 = 3;

pub fn slots() -> impl Iterator<Item = u32> {
    (FIRST_SLOT..=LAST_SLOT).step_by(SLOT_MIN as usize)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Booked,
    /// Talking: dialog step.
    Talking(usize),
    Done,
    Missed,
}

#[derive(Debug, Clone)]
pub struct Meeting {
    /// World day (`Clock::day`).
    pub day: u32,
    /// Minute of the day.
    pub start: u32,
    /// Account (player id) the meeting is for.
    pub owner: u16,
    pub topic: u8,
    pub state: State,
}

impl Meeting {
    /// The board-room door lets the owner in now.
    pub fn door_open(&self, day: u32, minute: u32) -> bool {
        self.day == day && self.state == State::Booked && minute + GRACE >= self.start && minute <= self.start + GRACE
    }

    /// Can start talking (a little late is fine, the whole slot).
    pub fn can_talk(&self, day: u32, minute: u32) -> bool {
        self.day == day && minute + GRACE >= self.start && minute < self.start + SLOT_MIN
    }
}

/// One question with three answers; `good` = the best one.
pub struct Step {
    pub text: &'static str,
    pub options: [&'static str; 3],
    pub good: usize,
    /// Replies to each answer (said right after it).
    pub replies: [&'static str; 3],
}

pub fn steps(t: u8) -> &'static [Step] {
    match t {
        topic::RAISE => &[Step {
            text: "Podwyżka? Proszę mnie przekonać.",
            options: ["Pracuję tu od początku i robię więcej, niż widać.", "Bo kawa w automacie podrożała.", "Konkurencja płaci więcej…"],
            good: 0,
            replies: ["Hm, to prawda, widać Cię często w biurze.", "Kawa jest u nas za darmo.", "To niech płaci. Ale rozumiem."],
        }],
        topic::IDEA => &[
            Step {
                text: "Słucham! Jaki masz pomysł?",
                options: [
                    "Tryb ciemny. Dla wszystkiego.",
                    "Blockchain w ekspresie do kawy.",
                    "Prostsze logowanie — klienci się w nim gubią.",
                ],
                good: 2,
                replies: ["Już jest, od zeszłego roku.", "…Nie.", "O, to realny problem. Mów dalej."],
            },
            Step {
                text: "A jak sprawdzimy, czy to działa?",
                options: ["Zapytamy kilku klientów i zrobimy prototyp.", "Wypuścimy w piątek o 17:00 i zobaczymy.", "Zaufajmy intuicji."],
                good: 0,
                replies: ["Dokładnie tak.", "Tylko nie w piątek…", "Intuicja to nie plan."],
            },
        ],
        topic::COMPLAINT => &[Step {
            text: "Co się stało?",
            options: ["Ktoś podjada moje jogurty z lodówki.", "W biurze jest za zimno.", "Ekspres ciągle zajęty."],
            good: 0,
            replies: [
                "Jogurtowy złodziej… Powieszę kartkę. Dużą.",
                "Klimatyzacja ma własne zdanie. Porozmawiam z nią.",
                "Drugi ekspres jest w planach. Od trzech lat.",
            ],
        }],
        topic::CHAT => &[Step {
            text: "O czym pogadamy?",
            options: ["Jak minął weekend?", "Co słychać u konkurencji?", "Jakie są plany dla firmy?"],
            good: 0,
            replies: [
                "Grill, deszcz, grill w deszczu. Klasyka.",
                "Ciii, ściany mają uszy. Ale radzimy sobie lepiej.",
                "Rośniemy! Kiedyś będzie tu drugie piętro.",
            ],
        }],
        _ => &[],
    }
}

pub mod lines {
    pub const NO_MEETING: &str = "Dzień dobry. Najpierw proszę umówić spotkanie w kalendarzu.";
    pub const NOT_YET: &str = "Mamy spotkanie trochę później — zapraszam o czasie.";
    pub const RAISE_YES: &str = "Dobrze. Od jutra +5 zł na godzinę. Proszę nie rozpowiadać.";
    pub const RAISE_NO: &str = "Wróć za kilka dni, zobaczymy.";
    pub const RAISE_TOO_SOON: &str = "Rozmawialiśmy o tym niedawno. Cierpliwości.";
    pub const IDEA_GREAT: &str = "Świetne! Wrzucę to na #ogólny.";
    pub const IDEA_OK: &str = "Ciekawe. Przemyśl to jeszcze.";
    pub const IDEA_BAD: &str = "Hmm… Może innym razem.";
    pub const THANKS: &str = "Dzięki za rozmowę. Wracamy do pracy!";
    pub fn missed(start: u32) -> String {
        format!("Nie było Cię na spotkaniu o {}. Szkoda.", crate::clock::hhmm(start))
    }
}

/// Chance (0..100) the CEO says yes to a raise: days worked and the answer.
pub fn raise_chance(days_worked: u32, good_answer: bool) -> u32 {
    20 + 10 * days_worked.min(5) + if good_answer { 30 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixteen_half_hour_slots_and_every_topic_has_a_dialog() {
        assert_eq!(slots().count(), 16);
        assert_eq!(slots().next(), Some(10 * 60));
        for t in [topic::RAISE, topic::IDEA, topic::COMPLAINT, topic::CHAT] {
            assert!(!steps(t).is_empty(), "{t}");
            for s in steps(t) {
                assert!(s.good < 3);
            }
        }
        assert_eq!(who(topic::IDEA), Who::CoFounder);
        assert_eq!(who(topic::RAISE), Who::Ceo);
    }

    #[test]
    fn the_door_opens_ten_minutes_around_the_start() {
        let m = Meeting { day: 2, start: 14 * 60, owner: 1, topic: topic::CHAT, state: State::Booked };
        assert!(!m.door_open(2, 13 * 60 + 49));
        assert!(m.door_open(2, 13 * 60 + 50));
        assert!(m.door_open(2, 14 * 60 + 10));
        assert!(!m.door_open(2, 14 * 60 + 11), "too late");
        assert!(!m.door_open(3, 14 * 60), "another day");
        assert!(m.can_talk(2, 14 * 60 + 25));
    }

    #[test]
    fn seniority_and_a_good_answer_help() {
        assert!(raise_chance(0, false) < raise_chance(5, false));
        assert!(raise_chance(5, true) <= 100);
    }
}
