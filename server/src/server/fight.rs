//! Fights: X punches (with a knife in hands it stabs) the nearest person;
//! at 0 health they're knocked out for a minute. The guard runs after the
//! attacker; a knife also calls the police and earns a reprimand.

use crate::inventory::kind as item_kind;
use crate::mischief::{self, lines};
use crate::npc;
use crate::protocol as proto;

use super::{dist2, Say, Server};

impl Server {
    /// X (or F with a knife): a swing at the nearest person in reach.
    pub(super) fn attack(&mut self, id: u16) {
        let tick = self.tick;
        let Some(p) = self.players.get(&id) else { return };
        if !p.in_building() || tick < p.next_attack || tick < p.held_until {
            return;
        }
        let knife = p.inventory.held_kind() == item_kind::KNIFE;
        let body = p.body;
        let reach = mischief::REACH;
        let target = self
            .players
            .values()
            .filter(|o| o.id != id && o.in_building() && o.riding.is_none() && o.body.floor == body.floor)
            .filter(|o| !o.knocked_out && !o.passed_out) // nobody kicks a man when he's down
            .map(|o| (o.id, dist2(o.body.pos, body.pos)))
            .filter(|&(_, d)| d <= reach * reach)
            .min_by_key(|&(_, d)| d)
            .map(|(o, _)| o);
        let Some(p) = self.players.get_mut(&id) else { return };
        p.next_attack = tick + if knife { mischief::KNIFE_COOLDOWN } else { mischief::PUNCH_COOLDOWN };
        p.swing_until = tick + mischief::SWING_TICKS;
        p.rest = None;
        let Some(target) = target else { return }; // a swing at thin air
        let ouch = lines::OUCH[self.rng.usize(..lines::OUCH.len())];
        let Some(t) = self.players.get_mut(&target) else { return };
        let (floor, pos) = (t.body.floor, t.body.pos);
        let out = t.needs.hurt(if knife { mischief::KNIFE_DAMAGE } else { mischief::PUNCH_DAMAGE });
        t.rest = None;
        if out {
            t.knocked_out = true;
            t.held_until = tick + mischief::KNOCKOUT_TICKS;
            t.held_activity = proto::activity::KNOCKED_OUT;
        }
        self.sounds.push((if knife { proto::sound::STAB } else { proto::sound::PUNCH }, floor, pos));
        let line = if out {
            lines::KNOCKED_OUT
        } else if knife {
            lines::STABBED
        } else {
            ouch
        };
        self.says.push(Say::new(target, line));
        if out {
            let text = format!("{} leży znokautowany!", self.nick_of_player(target));
            self.notify_room_of(target, proto::notice::ALERT, &text);
        }
        let what = if knife { "stabbed" } else { "punched" };
        self.log(format!("* fight: {} {what} {}{}", self.nick_of_player(id), self.nick_of_player(target), if out { " (KO)" } else { "" }));
        self.assaulted(id, knife);
    }

    /// After hitting somebody: the guard comes running; a knife calls the
    /// police too and earns a reprimand (once per incident).
    fn assaulted(&mut self, id: u16, knife: bool) {
        let Some(p) = self.players.get_mut(&id) else { return };
        let before = p.assault;
        p.assault = Some(knife || before == Some(true));
        let contract = p.contract;
        let nick = p.nick.clone();
        if before.is_none() {
            let guard = self.npcs.iter_mut().find(|n| n.role == npc::Role::Guard && n.chasing().is_none());
            if let Some(g) = guard {
                g.chase(id);
                let at = (g.body.floor, g.body.pos);
                self.sounds.push((proto::sound::WHISTLE, at.0, at.1));
                self.says.push(Say::addressed(g.id, npc::lines::GUARD_FIGHT, id));
            }
        }
        if knife && before != Some(true) {
            self.call_police(id);
            if contract {
                self.reprimand(id, |n, fired| lines::knife_reprimand(&nick, n, fired));
            }
        }
    }

    /// Caught by the guard after a fight: a moment in place. False if they
    /// weren't wanted for that.
    pub(super) fn caught_fighting(&mut self, npc_id: u16, pid: u16) -> bool {
        let Some(p) = self.players.get_mut(&pid) else { return false };
        if p.assault.is_none() {
            return false;
        }
        p.assault = None;
        p.held_until = self.tick + crate::security::GUARD_HOLD_TICKS;
        p.held_activity = proto::activity::HELD;
        p.needs.add_stress(crate::security::GUARD_STRESS);
        self.says.push(Say::addressed(npc_id, lines::GUARD_CAUGHT, pid));
        true
    }
}
