//! The HR app on the office computer: the receiver's contract, annexes and
//! leave; leave requests are decided at once (mail from HR).

use crate::hr::{self, lines};
use crate::protocol::{HrInfo, Packet};

use super::{Say, Server};

impl Server {
    /// `HrAction`: show the file, ask for leave, cancel a request.
    pub(super) fn handle_hr_action(&mut self, pid: u16, action: u8, arg: u16) {
        let Some(p) = self.players.get_mut(&pid) else { return };
        if !p.contract {
            return;
        }
        let today = p.day;
        let nick = p.nick.clone();
        match action {
            hr::action::REQUEST => match p.hr.request(today, u32::from(arg)) {
                Ok(r) => {
                    let approved = r.status == hr::status::APPROVED;
                    let body = if approved { lines::approved(r.day) } else { lines::rejected(r.day) };
                    self.office_mail(&nick, "HR", "Wniosek urlopowy", &body);
                    self.log(format!("* leave: {nick} asked for day {} ({})", r.day, if approved { "approved" } else { "rejected" }));
                    self.save_soon = true;
                }
                Err(line) => self.says.push(Say::new(pid, line)),
            },
            hr::action::CANCEL if p.hr.cancel(today, arg.min(u16::from(u8::MAX)) as u8) => self.save_soon = true,
            _ => {}
        }
        if let Some(info) = self.hr_info(pid) {
            self.send_to(pid, &Packet::HrInfo(Box::new(info)));
        }
    }

    fn hr_info(&self, pid: u16) -> Option<HrInfo> {
        let p = self.players.get(&pid)?;
        let title = p
            .position
            .and_then(|o| self.job_title(o))
            .or_else(|| self.cfg.recruitment.department_name(p.department).map(str::to_string))
            .unwrap_or_default();
        let short = |d: u32| u16::try_from(d).unwrap_or(u16::MAX);
        let skip = p.hr.annexes.len().saturating_sub(hr::MAX_ANNEXES_SENT);
        Some(HrInfo {
            title,
            department: p.department,
            form: p.employment,
            salary: p.salary,
            pay_rate: u32::try_from(p.pay_rate.max(0)).unwrap_or(u32::MAX),
            start_day: short(self.company.hired_on.get(&pid).copied().unwrap_or(p.day)),
            today: short(p.day),
            reprimands: p.reprimands,
            leave_days: p.hr.leave_days,
            worked: p.hr.worked,
            annexes: p.hr.annexes.iter().skip(skip).map(|a| (short(a.day), a.text.clone())).collect(),
            requests: p.hr.requests.iter().map(|r| (r.id, short(r.day), r.status)).collect(),
        })
    }
}
