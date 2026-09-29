//! Laptops on desks and the messenger.

use std::net::SocketAddr;

use crate::computer::{self, Account, Computer};
use crate::inventory::kind as item_kind;
use crate::protocol::{self as proto, Packet};
use crate::sim::Body;

use super::player::refresh;
use super::{Say, Server};

impl Server {
    /// Hired employees = messenger accounts.
    pub(super) fn accounts(&self) -> Vec<Account> {
        self.players
            .values()
            .filter(|p| p.contract && p.in_building())
            .map(|p| Account { id: p.id, nick: p.nick.clone(), department: p.department })
            .collect()
    }

    /// E at a desk: open the computer on it, or put the laptop in hands down.
    /// `None` = no desk in reach (the E goes on to picking things up);
    /// `Some(line)` = handled, with an optional speech line.
    pub(super) fn use_desk(&mut self, pid: u16, body: &Body) -> Option<Option<String>> {
        let ws = computer::workstation_in_reach(&self.workstations, body)?;
        if let Some(c) = self.computers.iter_mut().find(|c| c.station == ws) {
            if c.user.is_some_and(|u| u != pid) {
                return Some(Some(computer::lines::BUSY.into()));
            }
            c.user = Some(pid);
            let handle = c.handle;
            self.players.get_mut(&pid)?.at_computer = Some(handle);
            self.send_computer(pid);
            return Some(None);
        }
        let p = self.players.get(&pid)?;
        if p.inventory.held_kind() != item_kind::LAPTOP {
            return None;
        }
        let dept = self.cfg.recruitment.department_name(p.department).unwrap_or("");
        if !p.contract || dept.is_empty() {
            return Some(Some(computer::lines::NO_DEPARTMENT.into()));
        }
        if self.workstations[ws].department != p.department {
            return Some(Some(computer::lines::NOT_MY_DEPARTMENT.into()));
        }
        let handle = self.alloc_handle();
        let p = self.players.get_mut(&pid)?;
        let item = p.inventory.take_hands()?;
        refresh(p);
        self.computers.push(Computer { handle, station: ws, item, locked: false, user: None });
        Some(Some(computer::lines::PLACED.into()))
    }

    /// Stop looking at the screen (the client closes it when the
    /// AT_COMPUTER status bit clears).
    pub(super) fn end_session(&mut self, pid: u16) {
        let Some(p) = self.players.get_mut(&pid) else { return };
        if let Some(h) = p.at_computer.take() {
            if let Some(c) = self.computers.iter_mut().find(|c| c.handle == h && c.user == Some(pid)) {
                c.user = None;
            }
        }
    }

    /// Users who walked away (or whose computer is gone) leave the screen.
    pub(super) fn check_computer_sessions(&mut self) {
        let mut gone = Vec::new();
        for p in self.players.values() {
            let Some(h) = p.at_computer else { continue };
            let ok = self.computers.iter().find(|c| c.handle == h).is_some_and(|c| {
                c.user == Some(p.id) && computer::in_leave_range(&self.workstations[c.station], &p.body)
            });
            if !ok {
                gone.push(p.id);
            }
        }
        for pid in gone {
            self.end_session(pid);
        }
    }

    pub(super) fn computer_packet(&self, pid: u16) -> Option<Packet> {
        let h = self.players.get(&pid)?.at_computer?;
        let c = self.computers.iter().find(|c| c.handle == h)?;
        let accounts = self.accounts();
        let convs = match accounts.iter().find(|a| a.id == c.owner()) {
            Some(acc) if !c.locked => self
                .messenger
                .conversations(acc, &accounts, |d| self.cfg.recruitment.department_name(d).map(str::to_string))
                .into_iter()
                .map(|i| proto::ConvEntry { conv: i.conv, unread: i.unread, title: i.title })
                .collect(),
            _ => Vec::new(),
        };
        Some(Packet::Computer { handle: h, owner: c.owner(), locked: c.locked, convs })
    }

    pub(super) fn send_computer(&mut self, pid: u16) {
        if let Some(pk) = self.computer_packet(pid) {
            self.send_to(pid, &pk);
        }
    }

    /// Send messages, split so each datagram fits.
    pub(super) fn send_chat(&mut self, addr: SocketAddr, conv: u16, msgs: &[computer::Msg]) {
        let mut chunk: Vec<proto::ChatEntry> = Vec::new();
        let mut size = proto::HEADER_LEN + 3;
        for m in msgs {
            let len = 4 + 2 + 2 + m.nick.len().min(proto::MAX_NICK_BYTES) + 2 + m.text.len().min(proto::MAX_CHAT_BYTES);
            if size + len > proto::MAX_PACKET && !chunk.is_empty() {
                self.send(addr, &Packet::Chat { conv, messages: std::mem::take(&mut chunk) });
                size = proto::HEADER_LEN + 3;
            }
            size += len;
            chunk.push(proto::ChatEntry { id: m.id, from: m.from, nick: m.nick.clone(), text: m.text.clone() });
        }
        self.send(addr, &Packet::Chat { conv, messages: chunk });
    }

    pub(super) fn handle_computer_action(&mut self, pid: u16, action: u8, conv: u16, arg: u32, text: &str) {
        use proto::computer_action as a;
        let Some(p) = self.players.get(&pid) else { return };
        let (addr, hands_free) = (p.addr, p.inventory.hands_free());
        let Some(h) = p.at_computer else { return };
        let Some(ci) = self.computers.iter().position(|c| c.handle == h && c.user == Some(pid)) else { return };
        let owner = self.computers[ci].owner();
        match action {
            a::CLOSE => self.end_session(pid),
            a::LOCK => {
                self.computers[ci].locked = true;
                self.end_session(pid);
            }
            a::UNLOCK if pid == owner => {
                self.computers[ci].locked = false;
                self.send_computer(pid);
            }
            a::UNLOCK => self.says.push(Say::new(pid, computer::lines::LOCKED)),
            a::TAKE if !hands_free => self.says.push(Say::new(pid, computer::lines::HANDS_FULL)),
            a::TAKE => {
                self.end_session(pid);
                let c = self.computers.remove(ci);
                if owner != pid {
                    let line = format!("* computer: {} took {}", self.nick(pid), c.item.label);
                    self.log(line);
                }
                self.give(pid, c.item);
                self.says.push(Say::new(pid, computer::lines::TAKEN));
            }
            a::SYNC | a::SEND if !self.computers[ci].locked => {
                let accounts = self.accounts();
                let Some(acc) = accounts.iter().find(|x| x.id == owner).cloned() else { return };
                if action == a::SEND {
                    self.post_chat(pid, &acc, &accounts, conv, arg, text);
                } else if let Some(msgs) = self.messenger.sync(&acc, conv, arg, &accounts) {
                    self.send_chat(addr, conv, &msgs);
                }
            }
            _ => {}
        }
    }

    /// Post as `acc` (typed by `pid`); `nonce` dedupes retries. Pushed live
    /// to everyone looking at a screen of an account in the audience (the
    /// others get it from SYNC / unread counts).
    fn post_chat(&mut self, pid: u16, acc: &Account, accounts: &[Account], conv: u16, nonce: u32, text: &str) {
        let tick = self.tick;
        let Some(p) = self.players.get(&pid) else { return };
        let spam = p.last_chat_tick.is_some_and(|t| tick.wrapping_sub(t) < computer::SEND_COOLDOWN_TICKS);
        if nonce == p.last_chat_nonce || spam {
            return; // retry of a message already posted, or spam
        }
        let Some(msg) = self.messenger.post(acc, conv, text, accounts) else { return };
        let Some(p) = self.players.get_mut(&pid) else { return };
        p.last_chat_nonce = nonce;
        p.last_chat_tick = Some(tick);
        let line = if pid == acc.id {
            format!("* chat: {} -> conv {conv}", acc.nick)
        } else {
            format!("* chat: {} as {} -> conv {conv}", p.nick, acc.nick)
        };
        self.log(line);
        let mut pushes = Vec::new();
        for (account, seen_as) in self.messenger.audience(acc, conv, accounts) {
            for viewer in self.players.values() {
                let Some(vh) = viewer.at_computer else { continue };
                if self.computers.iter().any(|c| c.handle == vh && c.owner() == account && !c.locked) {
                    pushes.push((viewer.addr, viewer.id, seen_as));
                }
            }
        }
        for (addr, vid, seen_as) in pushes {
            self.send_chat(addr, seen_as, std::slice::from_ref(&msg));
            self.send_computer(vid);
        }
    }
}
