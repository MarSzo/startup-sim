//! Task boards (a small kanban, one per department): cards in three columns
//! with a priority, a description, an assignee and comments. The server
//! keeps them in memory; clients act as the computer's owner.

use std::collections::HashMap;

/// Cards on one board at most.
pub const MAX_TASKS: usize = 40;
/// Byte limits (UTF-8).
pub const TITLE_MAX: usize = 80;
pub const DESC_MAX: usize = 300;
pub const COMMENT_MAX: usize = 120;
/// Comments kept per card (the oldest go).
pub const MAX_COMMENTS: usize = 20;

pub mod column {
    pub const TODO: u8 = 0;
    pub const DOING: u8 = 1;
    pub const DONE: u8 = 2;
}

pub mod priority {
    pub const LOW: u8 = 0;
    pub const MID: u8 = 1;
    pub const URGENT: u8 = 2;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: u16,
    pub column: u8,
    pub priority: u8,
    pub title: String,
    pub desc: String,
    pub author: String,
    /// Nick, "" = nobody.
    pub assignee: String,
    /// (nick, text), oldest first.
    pub comments: Vec<(String, String)>,
}

/// Who should hear about a change (by e-mail).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    Assigned { to: String, by: String, title: String },
    Commented { to: Vec<String>, by: String, title: String, text: String },
}

#[derive(Debug, Default)]
pub struct Boards {
    next_id: u16,
    boards: HashMap<u8, Vec<Task>>,
}

/// `s` cut to at most `max` bytes on a character boundary, trimmed, one line.
pub fn clip(s: &str, max: usize) -> String {
    let s: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let s = s.trim();
    let mut end = s.len().min(max);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].trim_end().to_string()
}

/// Like `clip` but keeps line breaks (descriptions).
pub fn clip_text(s: &str, max: usize) -> String {
    let s: String = s.chars().filter(|c| *c == '\n' || !c.is_control()).collect();
    let s = s.trim();
    let mut end = s.len().min(max);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].trim_end().to_string()
}

pub mod lines {
    pub const FULL: &str = "Tablica pełna — najpierw posprzątaj zrobione zadania.";
    pub const NO_TITLE: &str = "Zadanie musi mieć tytuł.";
    pub const NO_DEPARTMENT: &str = "Tablica zadań jest dla pracowników działów.";
}

impl Boards {
    pub fn board(&self, dept: u8) -> &[Task] {
        self.boards.get(&dept).map_or(&[], |b| b.as_slice())
    }

    pub fn task(&self, dept: u8, id: u16) -> Option<&Task> {
        self.board(dept).iter().find(|t| t.id == id)
    }

    fn task_mut(&mut self, dept: u8, id: u16) -> Option<&mut Task> {
        self.boards.get_mut(&dept)?.iter_mut().find(|t| t.id == id)
    }

    /// A new card in "to do"; `text` = title, then the description after a
    /// line break.
    pub fn create(&mut self, dept: u8, author: &str, text: &str, prio: u8) -> Result<u16, &'static str> {
        let (title, desc) = text.split_once('\n').unwrap_or((text, ""));
        let title = clip(title, TITLE_MAX);
        if title.is_empty() {
            return Err(lines::NO_TITLE);
        }
        let board = self.boards.entry(dept).or_default();
        if board.len() >= MAX_TASKS {
            return Err(lines::FULL);
        }
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let id = self.next_id;
        board.push(Task {
            id,
            column: column::TODO,
            priority: prio.min(priority::URGENT),
            title,
            desc: clip_text(desc, DESC_MAX),
            author: author.to_string(),
            assignee: String::new(),
            comments: Vec::new(),
        });
        Ok(id)
    }

    pub fn move_to(&mut self, dept: u8, id: u16, col: u8) -> bool {
        match self.task_mut(dept, id) {
            Some(t) if col <= column::DONE => {
                t.column = col;
                true
            }
            _ => false,
        }
    }

    pub fn set_priority(&mut self, dept: u8, id: u16, prio: u8) -> bool {
        self.task_mut(dept, id).map(|t| t.priority = prio.min(priority::URGENT)).is_some()
    }

    /// `to` = "" unassigns. Tells the new assignee (unless they did it).
    pub fn assign(&mut self, dept: u8, id: u16, to: &str, by: &str) -> Option<Notice> {
        let t = self.task_mut(dept, id)?;
        let to = clip(to, 32);
        if t.assignee == to {
            return None;
        }
        t.assignee = to.clone();
        (!to.is_empty() && to != by).then(|| Notice::Assigned { to, by: by.to_string(), title: t.title.clone() })
    }

    /// Tells the author and the assignee (not the one commenting).
    pub fn comment(&mut self, dept: u8, id: u16, by: &str, text: &str) -> Option<Notice> {
        let text = clip(text, COMMENT_MAX);
        if text.is_empty() {
            return None;
        }
        let t = self.task_mut(dept, id)?;
        t.comments.push((by.to_string(), text.clone()));
        if t.comments.len() > MAX_COMMENTS {
            t.comments.remove(0);
        }
        let mut to: Vec<String> = [&t.author, &t.assignee].into_iter().filter(|n| !n.is_empty() && *n != by).cloned().collect();
        to.dedup();
        (!to.is_empty()).then(|| Notice::Commented { to, by: by.to_string(), title: t.title.clone(), text })
    }

    pub fn edit(&mut self, dept: u8, id: u16, text: &str) -> bool {
        let (title, desc) = text.split_once('\n').unwrap_or((text, ""));
        let title = clip(title, TITLE_MAX);
        let Some(t) = self.task_mut(dept, id) else { return false };
        if !title.is_empty() {
            t.title = title;
        }
        t.desc = clip_text(desc, DESC_MAX);
        true
    }

    pub fn delete(&mut self, dept: u8, id: u16) -> bool {
        let Some(b) = self.boards.get_mut(&dept) else { return false };
        let n = b.len();
        b.retain(|t| t.id != id);
        b.len() != n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_move_get_assigned_and_commented() {
        let mut b = Boards::default();
        let id = b.create(1, "Ola", "Naprawić logowanie\nPo zmianie hasła nie działa.", priority::URGENT).unwrap();
        let t = b.task(1, id).unwrap();
        assert_eq!((t.title.as_str(), t.desc.as_str(), t.column), ("Naprawić logowanie", "Po zmianie hasła nie działa.", column::TODO));
        assert!(b.board(2).is_empty(), "boards are per department");
        assert!(b.move_to(1, id, column::DOING));
        assert!(!b.move_to(1, id, 7), "no such column");
        // Kuba takes it himself: nobody to tell; Ola assigns Kuba: Kuba hears.
        assert_eq!(b.assign(1, id, "Kuba", "Kuba"), None);
        assert_eq!(b.assign(1, id, "", "Kuba"), None);
        assert_eq!(b.assign(1, id, "Kuba", "Ola"), Some(Notice::Assigned { to: "Kuba".into(), by: "Ola".into(), title: "Naprawić logowanie".into() }));
        match b.comment(1, id, "Kuba", "Zrobione, do sprawdzenia") {
            Some(Notice::Commented { to, .. }) => assert_eq!(to, vec!["Ola".to_string()]),
            n => panic!("{n:?}"),
        }
        assert_eq!(b.task(1, id).unwrap().comments.len(), 1);
        assert!(b.delete(1, id));
        assert!(b.board(1).is_empty());
    }

    #[test]
    fn limits_and_clipping() {
        let mut b = Boards::default();
        assert_eq!(b.create(1, "Ola", "   \nopis", 0), Err(lines::NO_TITLE));
        let long = "ż".repeat(100); // 200 bytes
        let id = b.create(1, "Ola", &long, 9).unwrap();
        let t = b.task(1, id).unwrap();
        assert!(t.title.len() <= TITLE_MAX && t.title.chars().all(|c| c == 'ż'));
        assert_eq!(t.priority, priority::URGENT, "clamped");
        for i in 1..MAX_TASKS {
            b.create(1, "Ola", &format!("Zadanie {i}"), 0).unwrap();
        }
        assert_eq!(b.create(1, "Ola", "Jeszcze jedno", 0), Err(lines::FULL));
        for _ in 0..MAX_COMMENTS + 5 {
            b.comment(1, id, "Kuba", "hej");
        }
        assert_eq!(b.task(1, id).unwrap().comments.len(), MAX_COMMENTS);
    }
}
