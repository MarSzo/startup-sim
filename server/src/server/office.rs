//! The office computer's apps besides the messenger: the department's task
//! board and work mail. Both act as the computer's owner (like the
//! messenger) and answer every action with the fresh state, so clients just
//! resend until their nonce comes back as `done`.

use std::net::SocketAddr;

use crate::protocol::{self as proto, Packet, TaskCard};
use crate::tasks::{self, Notice};
use crate::workmail;

use super::{Say, Server};

/// Bytes of board cards per TaskBoard part (under `MAX_PACKET`).
const BOARD_PART_BYTES: usize = 1000;
/// New mails sent per SYNC (the client asks again for more).
const MAILS_PER_SYNC: usize = 6;

/// Who a player acts as at an unlocked computer.
struct Seat {
    addr: SocketAddr,
    nick: String,
    dept: u8,
}

impl Server {
    fn seat(&self, pid: u16) -> Option<Seat> {
        let p = self.players.get(&pid)?;
        let h = p.at_computer?;
        let c = self.computers.iter().find(|c| c.handle == h && c.user == Some(pid) && !c.locked)?;
        let owner = self.players.get(&c.owner())?;
        owner.contract.then(|| Seat { addr: p.addr, nick: owner.nick.clone(), dept: owner.department })
    }

    /// A mail from the office (HR, the calendar, lunch, the board).
    pub(super) fn office_mail(&mut self, to: &str, from: &str, subject: &str, body: &str) {
        let (day, minute) = (self.clock.day.min(u16::MAX as u32) as u16, self.clock.minute() as u16);
        self.post.send(from, to, subject, body, day, minute);
        self.notify_nick(to, crate::protocol::notice::MAIL, format!("Nowa poczta od: {from} — {subject}"));
    }

    // ------------------------------------------------------------- tasks

    pub(super) fn handle_task_action(&mut self, pid: u16, nonce: u16, action: u8, task: u16, arg: u8, text: &str) {
        use proto::task_action as a;
        let Some(desk) = self.seat(pid) else { return };
        if desk.dept == 0 {
            self.says.push(Say::new(pid, tasks::lines::NO_DEPARTMENT));
            return;
        }
        let fresh = nonce != 0 && self.players.get(&pid).is_some_and(|p| p.task_nonce != nonce);
        if fresh {
            if let Some(p) = self.players.get_mut(&pid) {
                p.task_nonce = nonce;
            }
            let (d, me) = (desk.dept, desk.nick.as_str());
            let b = &mut self.boards;
            let notice = match action {
                a::CREATE => match b.create(d, me, text, arg) {
                    Ok(_) => None,
                    Err(line) => {
                        self.says.push(Say::new(pid, line));
                        None
                    }
                },
                a::MOVE => {
                    b.move_to(d, task, arg);
                    None
                }
                a::ASSIGN => b.assign(d, task, text, me),
                a::PRIORITY => {
                    b.set_priority(d, task, arg);
                    None
                }
                a::COMMENT => b.comment(d, task, me, text),
                a::DELETE => {
                    b.delete(d, task);
                    None
                }
                a::EDIT => {
                    b.edit(d, task, text);
                    None
                }
                _ => None,
            };
            match notice {
                Some(Notice::Assigned { to, by, title }) => {
                    let body = format!("{by} przypisał(a) Ci zadanie „{title}”.\nZajrzyj na tablicę zadań (Przeglądarka → Ulubione).");
                    self.office_mail(&to, "Tablica zadań", &format!("Nowe zadanie: {title}"), &body);
                }
                Some(Notice::Commented { to, by, title, text }) => {
                    for nick in to {
                        let body = format!("{by} skomentował(a) zadanie „{title}”:\n„{text}”");
                        self.office_mail(&nick, "Tablica zadań", &format!("Komentarz: {title}"), &body);
                    }
                }
                None => {}
            }
        }
        self.send_board(pid, &desk, if action == a::SYNC || fresh { task } else { 0 });
    }

    fn send_board(&mut self, pid: u16, desk: &Seat, detail: u16) {
        let done = self.players.get(&pid).map_or(0, |p| p.task_nonce);
        let mut members: Vec<String> =
            self.players.values().filter(|p| p.contract && p.department == desk.dept).map(|p| p.nick.clone()).collect();
        members.sort();
        members.truncate(proto::MAX_MEMBERS);
        let mut cards: Vec<&tasks::Task> = self.boards.board(desk.dept).iter().collect();
        cards.sort_by_key(|t| (t.column, std::cmp::Reverse(t.priority), t.id));
        // Parts by size: the first one also carries the members.
        let mut parts: Vec<Vec<TaskCard>> = vec![Vec::new()];
        let mut used: usize = members.iter().map(|m| m.len() + 1).sum();
        for t in cards {
            let size = 12 + t.title.len() + t.author.len() + t.assignee.len();
            if used + size > BOARD_PART_BYTES {
                parts.push(Vec::new());
                used = 0;
            }
            used += size;
            if let Some(p) = parts.last_mut() {
                p.push(TaskCard {
                    id: t.id,
                    column: t.column,
                    priority: t.priority,
                    comments: t.comments.len().min(255) as u8,
                    title: t.title.clone(),
                    author: t.author.clone(),
                    assignee: t.assignee.clone(),
                });
            }
        }
        let n = parts.len().min(255) as u8;
        for (i, tasks) in parts.into_iter().enumerate() {
            let members = if i == 0 { members.clone() } else { Vec::new() };
            self.send(desk.addr, &Packet::TaskBoard { dept: desk.dept, done, part: i as u8, parts: n, members, tasks });
        }
        if let Some(t) = self.boards.task(desk.dept, detail) {
            let skip = t.comments.len().saturating_sub(proto::DETAIL_COMMENTS);
            let pk = Packet::TaskDetail { id: t.id, desc: t.desc.clone(), comments: t.comments[skip..].to_vec() };
            self.send(desk.addr, &pk);
        }
    }

    // -------------------------------------------------------------- mail

    #[allow(clippy::too_many_arguments)]
    pub(super) fn handle_mail_action(&mut self, pid: u16, nonce: u16, action: u8, id: u16, to: &str, subject: &str, body: &str) {
        use proto::mail_action as a;
        let Some(desk) = self.seat(pid) else { return };
        let fresh = nonce != 0 && self.players.get(&pid).is_some_and(|p| p.mail_nonce != nonce);
        if fresh {
            if let Some(p) = self.players.get_mut(&pid) {
                p.mail_nonce = nonce;
            }
            match action {
                a::SEND => {
                    let known = self.players.values().any(|p| p.contract && p.nick == to);
                    if !known {
                        self.says.push(Say::new(pid, workmail::lines::NO_SUCH));
                    } else if subject.trim().is_empty() && body.trim().is_empty() {
                        self.says.push(Say::new(pid, workmail::lines::EMPTY));
                    } else {
                        let subject = if subject.trim().is_empty() { "(bez tematu)" } else { subject };
                        let from = desk.nick.clone();
                        self.office_mail(to, &from, subject, body);
                        self.says.push(Say::new(pid, workmail::lines::SENT));
                    }
                }
                a::TRASH | a::RESTORE => {
                    self.post.inbox_mut(&desk.nick).set_trashed(id, action == a::TRASH);
                }
                a::EMPTY_TRASH => self.post.inbox_mut(&desk.nick).empty_trash(),
                _ => {}
            }
        }
        // SYNC: the mails newer than the client's newest; always the state.
        let (mails, ids, trashed): (Vec<workmail::Mail>, Vec<u16>, Vec<u16>) = match self.post.inbox(&desk.nick) {
            Some(inbox) => (
                inbox.mails.iter().filter(|m| action == a::SYNC && m.id > id).take(MAILS_PER_SYNC).cloned().collect(),
                inbox.mails.iter().map(|m| m.id).collect(),
                inbox.mails.iter().filter(|m| m.trashed).map(|m| m.id).collect(),
            ),
            None => (Vec::new(), Vec::new(), Vec::new()),
        };
        for m in mails {
            let pk = Packet::WorkMail { id: m.id, from: m.from, to: m.to, subject: m.subject, body: m.body, day: m.day, minute: m.minute };
            self.send(desk.addr, &pk);
        }
        let done = self.players.get(&pid).map_or(0, |p| p.mail_nonce);
        self.send(desk.addr, &Packet::MailState { done, ids, trashed });
    }
}
