//! Computers on desks and the company messenger (GDD 9a, step 4).
//!
//! A laptop from HR is put on a free desk in the owner's department (E with
//! the laptop in hands). Whoever presses E at the desk opens its screen; the
//! computer is always logged in as its *owner*, so someone using an unlocked
//! computer writes in the owner's name. Locking is manual (a button on the
//! screen, anyone may press it); only the owner unlocks. Anyone may take a
//! laptop off a desk, locked or not.

use std::collections::{HashMap, VecDeque};

use crate::building::Building;
use crate::inventory::Item;
use crate::map::Tile;
use crate::sim::{Body, Pos, TILE_UNITS};

/// Reach for using / putting a laptop on a desk (the tile in front of you).
pub const REACH: i32 = TILE_UNITS * 5 / 4;
/// Walking further than this from the desk ends the session.
pub const LEAVE_RADIUS: i32 = TILE_UNITS * 2;
/// Messages kept per conversation.
pub const HISTORY: usize = 60;
/// Messages sent per sync reply (older ones need `after` = older id).
pub const SYNC_BATCH: usize = 30;
/// Minimum ticks between two messages of one user (spam guard).
pub const SEND_COOLDOWN_TICKS: u32 = 10;
pub const MAX_MESSAGE_CHARS: usize = 200;

/// A desk tile in a department room where a laptop can stand.
#[derive(Debug, Clone)]
pub struct Workstation {
    pub floor: u8,
    pub tile: Tile,
    /// Name of the room the desk is in.
    pub room_name: String,
    /// Whose desk it is: the room's department (0 = nobody's).
    pub department: u8,
}

pub fn find_workstations(b: &Building) -> Vec<Workstation> {
    let mut out = Vec::new();
    for (f, m) in b.active_floors() {
        for y in 0..m.height {
            for x in 0..m.width {
                // Desks in the departments; the board works at its meeting table.
                let rid = m.room_at_tile(x, y);
                let Some(r) = m.rooms.iter().find(|r| r.id == rid) else { continue };
                let ok = match m.tile_type(x, y) {
                    Some("desk") => r.kind == "department",
                    Some("table") => r.kind == "management",
                    _ => false,
                };
                if ok {
                    out.push(Workstation { floor: f, tile: Tile { x, y }, room_name: r.name.clone(), department: r.department });
                }
            }
        }
    }
    out
}

/// Index of the workstation nearest to `body` within reach.
pub fn workstation_in_reach(ws: &[Workstation], body: &Body) -> Option<usize> {
    ws.iter()
        .enumerate()
        .filter(|(_, w)| w.floor == body.floor)
        .map(|(i, w)| (i, dist2(Pos::tile_center(w.tile.x, w.tile.y), body.pos)))
        .filter(|&(_, d)| d <= REACH * REACH)
        .min_by_key(|&(_, d)| d)
        .map(|(i, _)| i)
}

/// A laptop standing on a desk; `handle` is its entity id in snapshots.
#[derive(Debug, Clone)]
pub struct Computer {
    pub handle: u16,
    pub station: usize,
    pub item: Item,
    pub locked: bool,
    /// Player currently looking at the screen.
    pub user: Option<u16>,
}

impl Computer {
    pub fn owner(&self) -> u16 {
        self.item.owner
    }
}

/// Entity flags of a computer (see PROTOCOL.md).
pub mod flags {
    pub const LOCKED: u8 = 1;
    pub const IN_USE: u8 = 2;
}

pub fn entity_flags(c: &Computer) -> u8 {
    (if c.locked { flags::LOCKED } else { 0 }) | (if c.user.is_some() { flags::IN_USE } else { 0 })
}

pub fn in_leave_range(ws: &Workstation, body: &Body) -> bool {
    ws.floor == body.floor && dist2(Pos::tile_center(ws.tile.x, ws.tile.y), body.pos) <= LEAVE_RADIUS * LEAVE_RADIUS
}

pub mod lines {
    pub const PLACED: &str = "Laptop na biurku. Do roboty!";
    pub const NOT_MY_DEPARTMENT: &str = "To biurko innego działu.";
    pub const NO_DEPARTMENT: &str = "Najpierw muszę podpisać umowę w HR.";
    pub const TAKEN: &str = "Zabieram laptop.";
    pub const HANDS_FULL: &str = "Mam zajęte ręce — nie wezmę laptopa.";
    pub const BUSY: &str = "Ktoś już przy nim siedzi.";
    pub const LOCKED: &str = "Zablokowany. Tylko właściciel go odblokuje.";
}

// ---------------------------------------------------------------- messenger

/// Conversation ids (as seen from one account).
pub mod conv {
    pub const GENERAL: u16 = 1;
    /// Department channel: `DEPARTMENT_BASE + department id`.
    pub const DEPARTMENT_BASE: u16 = 16;
    /// Private conversation: `DM | other player id`.
    pub const DM: u16 = 0x8000;
}

/// Account = owner of the computer (id, name, department once hired).
#[derive(Debug, Clone)]
pub struct Account {
    pub id: u16,
    pub nick: String,
    /// Official department (0 before the contract).
    pub department: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Key {
    Channel(u16),
    Dm(u16, u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Msg {
    pub id: u32,
    pub from: u16,
    pub nick: String,
    pub text: String,
}

/// A conversation in an account's sidebar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvInfo {
    pub conv: u16,
    pub title: String,
    pub unread: u8,
}

#[derive(Default)]
pub struct Messenger {
    next_id: u32,
    convs: HashMap<Key, VecDeque<Msg>>,
    /// Last message id read: (account, conv) -> id.
    read: HashMap<(u16, u16), u32>,
}

/// "#produkt-it" from "Produkt / IT".
pub fn channel_title(department_name: &str) -> String {
    let mut s = String::from("#");
    let mut dash = false;
    for c in department_name.to_lowercase().chars() {
        if c.is_alphanumeric() {
            if dash && s.len() > 1 {
                s.push('-');
            }
            s.push(c);
            dash = false;
        } else {
            dash = true;
        }
    }
    s
}

impl Messenger {
    /// Storage key of `conv` for `account`, if the account may use it.
    /// `accounts` = everyone who has a messenger (hired employees).
    fn key(&self, account: &Account, conv: u16, accounts: &[Account]) -> Option<Key> {
        if conv == conv::GENERAL {
            return Some(Key::Channel(conv::GENERAL));
        }
        if conv & conv::DM != 0 {
            let other = conv & !conv::DM;
            if other == account.id || !accounts.iter().any(|a| a.id == other) {
                return None;
            }
            return Some(Key::Dm(account.id.min(other), account.id.max(other)));
        }
        let dept = conv.checked_sub(conv::DEPARTMENT_BASE)?;
        (account.department != 0 && dept == account.department as u16).then_some(Key::Channel(conv))
    }

    /// Sidebar of `account`: #ogólny, own department channel, then DMs.
    pub fn conversations(&self, account: &Account, accounts: &[Account], dept_name: impl Fn(u8) -> Option<String>) -> Vec<ConvInfo> {
        let mut out = vec![self.info(account, conv::GENERAL, "#ogólny".into(), accounts)];
        if account.department != 0 {
            let title = dept_name(account.department).map_or_else(|| "#dział".into(), |n| channel_title(&n));
            out.push(self.info(account, conv::DEPARTMENT_BASE + account.department as u16, title, accounts));
        }
        let mut others: Vec<&Account> = accounts.iter().filter(|a| a.id != account.id).collect();
        others.sort_by_key(|a| a.nick.to_lowercase());
        for o in others {
            out.push(self.info(account, conv::DM | o.id, o.nick.clone(), accounts));
        }
        out
    }

    fn info(&self, account: &Account, conv: u16, title: String, accounts: &[Account]) -> ConvInfo {
        let read = self.read.get(&(account.id, conv)).copied().unwrap_or(0);
        let unread = self
            .key(account, conv, accounts)
            .and_then(|k| self.convs.get(&k))
            .map_or(0, |q| q.iter().filter(|m| m.id > read && m.from != account.id).count());
        ConvInfo { conv, title, unread: unread.min(99) as u8 }
    }

    /// Post as `account`. Returns the stored message, or None if the text is
    /// empty or the conversation isn't available to the account.
    pub fn post(&mut self, account: &Account, conv: u16, text: &str, accounts: &[Account]) -> Option<Msg> {
        let text: String = text.chars().filter(|c| !c.is_control()).take(MAX_MESSAGE_CHARS).collect();
        let text = text.trim().to_string();
        if text.is_empty() {
            return None;
        }
        let key = self.key(account, conv, accounts)?;
        self.next_id += 1;
        let msg = Msg { id: self.next_id, from: account.id, nick: account.nick.clone(), text };
        let q = self.convs.entry(key).or_default();
        q.push_back(msg.clone());
        while q.len() > HISTORY {
            q.pop_front();
        }
        self.read.insert((account.id, conv), msg.id);
        Some(msg)
    }

    /// Messages newer than `after` (at most `SYNC_BATCH`, the newest ones);
    /// marks them read for the account.
    pub fn sync(&mut self, account: &Account, conv: u16, after: u32, accounts: &[Account]) -> Option<Vec<Msg>> {
        let key = self.key(account, conv, accounts)?;
        let msgs: Vec<Msg> = match self.convs.get(&key) {
            Some(q) => {
                let newer: Vec<&Msg> = q.iter().filter(|m| m.id > after).collect();
                newer[newer.len().saturating_sub(SYNC_BATCH)..].iter().map(|m| (*m).clone()).collect()
            }
            None => Vec::new(),
        };
        if let Some(last) = msgs.last() {
            let r = self.read.entry((account.id, conv)).or_default();
            *r = (*r).max(last.id);
        }
        Some(msgs)
    }

    /// Accounts that can see a conversation `key` (for live pushes): the
    /// conv id as seen by each of them.
    pub fn audience(&self, account: &Account, conv: u16, accounts: &[Account]) -> Vec<(u16, u16)> {
        match self.key(account, conv, accounts) {
            Some(Key::Channel(c)) => accounts
                .iter()
                .filter(|a| c == conv::GENERAL || c == conv::DEPARTMENT_BASE + a.department as u16)
                .map(|a| (a.id, c))
                .collect(),
            Some(Key::Dm(a, b)) => vec![(a, conv::DM | b), (b, conv::DM | a)],
            None => Vec::new(),
        }
    }

    /// A message from the company itself (an NPC) to a channel.
    pub fn post_system(&mut self, conv: u16, from: u16, nick: &str, text: &str) -> Msg {
        self.next_id += 1;
        let msg = Msg { id: self.next_id, from, nick: nick.into(), text: text.into() };
        let q = self.convs.entry(Key::Channel(conv)).or_default();
        q.push_back(msg.clone());
        while q.len() > HISTORY {
            q.pop_front();
        }
        msg
    }

    /// A player left: their private conversations go (ids get reused).
    pub fn forget(&mut self, id: u16) {
        self.convs.retain(|k, _| !matches!(k, Key::Dm(a, b) if *a == id || *b == id));
        self.read.retain(|(a, c), _| *a != id && *c != (conv::DM | id));
    }
}

fn dist2(a: Pos, b: Pos) -> i32 {
    let (dx, dy) = (a.x - b.x, a.y - b.y);
    dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(id: u16, nick: &str, dept: u8) -> Account {
        Account { id, nick: nick.into(), department: dept }
    }

    #[test]
    fn every_department_has_desks_and_the_map_knows_only_real_ones() {
        use crate::building::{default_building_path, Building};
        use crate::recruitment::{default_recruitment_path, Recruitment};
        let b = Building::load(&default_building_path()).unwrap();
        let r = Recruitment::load(&default_recruitment_path()).unwrap();
        let ws = find_workstations(&b);
        for w in &ws {
            assert!(w.department == 0 || r.department_name(w.department).is_some(), "{}: unknown department {}", w.room_name, w.department);
        }
        for d in &r.departments {
            assert!(ws.iter().any(|w| w.department == d.id), "{} has no desks", d.name);
        }
        assert!(r.departments.len() >= 10);
        // Positions: any department but the board.
        assert!(crate::company::position_department(&r, 4), "Mobile");
        assert!(crate::company::position_department(&r, 10), "customer service");
        assert!(!crate::company::position_department(&r, crate::company::BOARD_DEPARTMENT));
        assert!(!crate::company::position_department(&r, 99));
        let list = r.department_list();
        assert_eq!(list.iter().find(|d| d.id == 10).map(|d| d.short.as_str()), Some("Obsługa"));
    }

    #[test]
    fn channel_titles() {
        assert_eq!(channel_title("IT / Produkt"), "#it-produkt");
        assert_eq!(channel_title("Biznes"), "#biznes");
        assert_eq!(channel_title("Produkt / IT"), "#produkt-it");
        assert_eq!(channel_title("Obsługa klienta"), "#obsługa-klienta");
    }

    #[test]
    fn department_channel_is_private_to_the_department() {
        let all = vec![acc(1, "Ola", 1), acc(2, "Kuba", 2)];
        let mut m = Messenger::default();
        assert!(m.post(&all[0], conv::DEPARTMENT_BASE + 1, "deploy w piątek?", &all).is_some());
        assert!(m.post(&all[1], conv::DEPARTMENT_BASE + 1, "wbijam", &all).is_none(), "Kuba is in Biznes");
        assert!(m.sync(&all[1], conv::DEPARTMENT_BASE + 1, 0, &all).is_none());
        let aud = m.audience(&all[0], conv::DEPARTMENT_BASE + 1, &all);
        assert_eq!(aud, vec![(1, conv::DEPARTMENT_BASE + 1)]);
        assert_eq!(m.audience(&all[1], conv::GENERAL, &all).len(), 2);
    }

    #[test]
    fn direct_messages_and_unread_counts() {
        let all = vec![acc(1, "Ola", 1), acc(2, "Kuba", 2), acc(3, "Ewa", 1)];
        let mut m = Messenger::default();
        m.post(&all[0], conv::DM | 2, "cześć!", &all).unwrap();
        m.post(&all[0], conv::DM | 2, "masz chwilę?", &all).unwrap();
        let unread =
            |m: &Messenger, a: &Account, c: u16| m.conversations(a, &all, |_| None).into_iter().find(|i| i.conv == c).unwrap().unread;
        assert_eq!(unread(&m, &all[1], conv::DM | 1), 2);
        assert_eq!(unread(&m, &all[0], conv::DM | 2), 0, "own messages aren't unread");
        let got = m.sync(&all[1], conv::DM | 1, 0, &all).unwrap();
        assert_eq!(got.iter().map(|x| x.text.as_str()).collect::<Vec<_>>(), ["cześć!", "masz chwilę?"]);
        assert_eq!(unread(&m, &all[1], conv::DM | 1), 0);
        assert!(m.sync(&all[2], conv::DM | 1, 0, &all).unwrap().is_empty(), "Ewa has her own DM with Ola");
        assert!(m.post(&all[0], conv::DM | 1, "do siebie", &all).is_none());
        assert!(m.post(&all[0], conv::DM | 9, "nie ma kogoś takiego", &all).is_none());
        m.forget(2);
        assert!(m.sync(&all[0], conv::DM | 2, 0, &all).unwrap().is_empty(), "DMs of a player who left are gone");
    }

    #[test]
    fn messages_are_cleaned_and_history_is_capped() {
        let all = vec![acc(1, "Ola", 1)];
        let mut m = Messenger::default();
        assert!(m.post(&all[0], conv::GENERAL, "  \n ", &all).is_none());
        let msg = m.post(&all[0], conv::GENERAL, &"x".repeat(500), &all).unwrap();
        assert_eq!(msg.text.len(), MAX_MESSAGE_CHARS);
        for i in 0..100 {
            m.post(&all[0], conv::GENERAL, &format!("{i}"), &all);
        }
        let got = m.sync(&all[0], conv::GENERAL, 0, &all).unwrap();
        assert_eq!(got.len(), SYNC_BATCH);
        assert_eq!(got.last().unwrap().text, "99");
    }
}
