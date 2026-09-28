//! Work e-mail: an inbox per employee (by nick) with a trash. Mails come
//! from colleagues and from the office itself (HR, the calendar, lunch, the
//! task board).

use std::collections::HashMap;

use crate::tasks::{clip, clip_text};

/// Mails kept per inbox (the oldest go first, trashed ones before others).
pub const MAX_MAILS: usize = 40;
pub const SUBJECT_MAX: usize = 80;
pub const BODY_MAX: usize = 400;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Mail {
    pub id: u16,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
    /// Game time sent: day and minute of the day.
    pub day: u16,
    pub minute: u16,
    pub trashed: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Inbox {
    pub mails: Vec<Mail>,
    next_id: u16,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PostOffice {
    inboxes: HashMap<String, Inbox>,
}

pub mod lines {
    pub const NO_SUCH: &str = "Nie ma takiego adresata w firmie.";
    pub const EMPTY: &str = "Mail musi mieć temat albo treść.";
    pub const SENT: &str = "Wysłane.";
}

impl Inbox {
    pub fn get(&self, id: u16) -> Option<&Mail> {
        self.mails.iter().find(|m| m.id == id)
    }

    fn push(&mut self, mut m: Mail) -> u16 {
        self.next_id = self.next_id.wrapping_add(1).max(1);
        m.id = self.next_id;
        self.mails.push(m);
        while self.mails.len() > MAX_MAILS {
            let i = self.mails.iter().position(|m| m.trashed).unwrap_or(0);
            self.mails.remove(i);
        }
        self.next_id
    }

    pub fn set_trashed(&mut self, id: u16, on: bool) -> bool {
        self.mails.iter_mut().find(|m| m.id == id).map(|m| m.trashed = on).is_some()
    }

    pub fn empty_trash(&mut self) {
        self.mails.retain(|m| !m.trashed);
    }

    pub fn max_id(&self) -> u16 {
        self.mails.iter().map(|m| m.id).max().unwrap_or(0)
    }
}

impl PostOffice {
    pub fn inbox(&self, nick: &str) -> Option<&Inbox> {
        self.inboxes.get(nick)
    }

    pub fn inbox_mut(&mut self, nick: &str) -> &mut Inbox {
        self.inboxes.entry(nick.to_string()).or_default()
    }

    /// Deliver one mail; returns its id in the recipient's inbox.
    pub fn send(&mut self, from: &str, to: &str, subject: &str, body: &str, day: u16, minute: u16) -> u16 {
        let m = Mail {
            id: 0,
            from: clip(from, 32),
            to: clip(to, 32),
            subject: clip(subject, SUBJECT_MAX),
            body: clip_text(body, BODY_MAX),
            day,
            minute,
            trashed: false,
        };
        self.inbox_mut(to).push(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deliver_trash_and_cap() {
        let mut po = PostOffice::default();
        let id = po.send("Ola", "Kuba", "Kawa?", "O 12 w kuchni.\nPrzynieś kubek.", 2, 600);
        let inbox = po.inbox("Kuba").unwrap();
        assert_eq!(inbox.get(id).unwrap().body, "O 12 w kuchni.\nPrzynieś kubek.");
        assert!(po.inbox("Ola").is_none(), "only the recipient has it");
        let inbox = po.inbox_mut("Kuba");
        assert!(inbox.set_trashed(id, true));
        inbox.empty_trash();
        assert!(inbox.get(id).is_none());
        for i in 0..MAX_MAILS + 3 {
            po.send("HR", "Kuba", &format!("Mail {i}"), "", 2, 600);
        }
        let inbox = po.inbox("Kuba").unwrap();
        assert_eq!(inbox.mails.len(), MAX_MAILS);
        assert_eq!(inbox.mails[0].subject, "Mail 3", "the oldest went");
        assert!(inbox.max_id() > MAX_MAILS as u16);
    }
}
