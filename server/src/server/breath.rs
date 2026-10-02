//! The board's breathalyser: a board member holding it presses F next to
//! somebody - the reading, and above the limit the question whether to
//! reprimand. Three reprimands: fired.

use crate::company;
use crate::drunk::{self, lines};
use crate::protocol::Packet;

use super::{Say, Server};

/// Dialog ids of the "reprimand?" question (apart from the board's talks).
const ASK_ID_BASE: u8 = 200;

impl Server {
    /// F with the breathalyser in hand.
    pub(super) fn breath_test(&mut self, id: u16, department: u8) {
        if department != company::BOARD_DEPARTMENT {
            self.says.push(Say::new(id, lines::NOT_BOARD));
            return;
        }
        let Some(me) = self.players.get(&id) else { return };
        let (floor, pos) = (me.body.floor, me.body.pos);
        let reach2 = i64::from(drunk::BREATH_REACH).pow(2);
        let target = self
            .players
            .values()
            .filter(|o| o.id != id && o.in_building() && o.riding.is_none() && o.body.floor == floor)
            .map(|o| (o.id, i64::from(o.body.pos.x - pos.x).pow(2) + i64::from(o.body.pos.y - pos.y).pow(2)))
            .filter(|&(_, d)| d <= reach2)
            .min_by_key(|&(_, d)| d)
            .map(|(o, _)| o);
        let Some(target) = target else {
            self.says.push(Say::new(id, lines::NOBODY));
            return;
        };
        let Some(t) = self.players.get(&target) else { return };
        let (milli, nick) = (t.needs.promille_milli(), t.nick.clone());
        self.says.push(Say::new(id, lines::reading(&nick, milli)));
        self.log(format!("* breathalyser: {nick} {milli} milli-per-mille"));
        if milli <= drunk::LIMIT_MILLI {
            return;
        }
        let ask = ASK_ID_BASE + (self.tick % 50) as u8;
        let Some(me) = self.players.get_mut(&id) else { return };
        me.reprimand_ask = Some((ask, target));
        let addr = me.addr;
        let packet = Packet::Dialog {
            id: ask,
            npc: target,
            text: format!("{} ({nick})", lines::REPRIMAND_ASK),
            options: vec![lines::REPRIMAND_YES.into(), lines::REPRIMAND_NO.into()],
        };
        self.send(addr, &packet);
        self.send(addr, &packet); // a tiny packet: a duplicate makes loss unlikely
    }

    /// The answer to "reprimand?"; false if `dialog` isn't that question.
    pub(super) fn answer_reprimand(&mut self, pid: u16, dialog: u8, choice: u8) -> bool {
        let Some(p) = self.players.get_mut(&pid) else { return false };
        let Some((ask, target)) = p.reprimand_ask else { return false };
        if ask != dialog {
            return false;
        }
        p.reprimand_ask = None;
        let addr = p.addr;
        self.send(addr, &Packet::Dialog { id: 0, npc: target, text: String::new(), options: Vec::new() });
        if choice != 0 {
            self.says.push(Say::new(pid, lines::SPARED));
            return true;
        }
        let Some(nick) = self.players.get(&target).map(|t| t.nick.clone()) else { return true };
        let n = self.reprimand(target, |n, fired| {
            if fired {
                format!("{nick},\n\nalkomat nie kłamie: to już {n}. nagana za alkohol w pracy. Rozwiązujemy umowę.\n\nZarząd")
            } else {
                format!(
                    "{nick},\n\nkontrola trzeźwości wykazała alkohol powyżej normy. Udzielamy nagany ({n}/{}). Przy {} — zwolnienie.\n\nZarząd",
                    drunk::REPRIMANDS_TO_FIRE,
                    drunk::REPRIMANDS_TO_FIRE
                )
            }
        });
        self.says.push(Say::new(pid, lines::reprimanded(&nick, n)));
        true
    }

    /// A reprimand for `target` (the mail's text from `body(n, fired)`):
    /// counted, mailed, and the third one fires them. Returns how many now.
    pub(super) fn reprimand(&mut self, target: u16, body: impl Fn(u8, bool) -> String) -> u8 {
        let company = self.company.name.clone();
        let Some(t) = self.players.get_mut(&target) else { return 0 };
        t.reprimands = t.reprimands.saturating_add(1);
        let (n, nick) = (t.reprimands, t.nick.clone());
        let fired = n >= drunk::REPRIMANDS_TO_FIRE;
        self.office_mail(&nick, &format!("{company} — Zarząd"), "Nagana", &body(n, fired));
        self.log(format!("* reprimand: {nick} ({n})"));
        self.save_soon = true;
        if fired {
            self.fire(target);
        }
        n
    }
}
