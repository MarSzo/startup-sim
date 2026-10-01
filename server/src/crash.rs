//! Client crash reports: after a crash the client asks the player and sends
//! the end of its log (`POST /api/crash`, see `http`). Each report is a text
//! file in `<save dir>/crashes/` (the newest `KEPT`), pulled with
//! `deploy/pull-crashes.sh`. Anyone may send one (the client may crash
//! before logging in), so: small, rate limited, cleaned of control chars.

use std::collections::HashMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Deserialize;

/// Reports kept on disk (the oldest go first).
pub const KEPT: usize = 200;
/// Per address, per hour.
pub const PER_ADDRESS: usize = 5;
/// From everybody together, per hour.
pub const PER_HOUR: usize = 60;
/// The log part of a report (its end is kept).
pub const LOG_MAX: usize = 48 * 1024;
/// The request body (JSON) at most.
pub const BODY_MAX: usize = 64 * 1024;
const FIELD_MAX: usize = 128;
const WINDOW: Duration = Duration::from_secs(3600);

#[derive(Debug, Deserialize)]
pub struct Report {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub os: String,
    #[serde(default)]
    pub cpu: String,
    #[serde(default)]
    pub gpu: String,
    /// When the crashed session started (client's clock, free text).
    #[serde(default)]
    pub started: String,
    #[serde(default)]
    pub log: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CrashError {
    TooMany,
    Empty,
    Disk(String),
}

#[derive(Clone)]
pub struct Crashes {
    dir: PathBuf,
    sent: Arc<Mutex<Limits>>,
}

#[derive(Default)]
struct Limits {
    by_address: HashMap<IpAddr, Vec<Instant>>,
    all: Vec<Instant>,
}

impl Crashes {
    pub fn new(dir: PathBuf) -> Crashes {
        Crashes { dir, sent: Arc::default() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Save a report; returns its id (the file name without `.txt`).
    pub fn save(&self, r: &Report, from: IpAddr) -> Result<String, CrashError> {
        if r.log.trim().is_empty() {
            return Err(CrashError::Empty);
        }
        self.allow(from, Instant::now())?;
        std::fs::create_dir_all(&self.dir).map_err(|e| CrashError::Disk(e.to_string()))?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let id = format!("{}-{:04x}", stamp(now), fastrand::u16(..));
        std::fs::write(self.dir.join(format!("{id}.txt")), render(r, from, now)).map_err(|e| CrashError::Disk(e.to_string()))?;
        prune(&self.dir, KEPT);
        Ok(id)
    }

    fn allow(&self, from: IpAddr, now: Instant) -> Result<(), CrashError> {
        let mut l = self.sent.lock().unwrap_or_else(|e| e.into_inner());
        let fresh = |v: &mut Vec<Instant>| v.retain(|t| now.duration_since(*t) < WINDOW);
        fresh(&mut l.all);
        let mine = l.by_address.entry(from).or_default();
        fresh(mine);
        if mine.len() >= PER_ADDRESS {
            return Err(CrashError::TooMany);
        }
        mine.push(now);
        if l.all.len() >= PER_HOUR {
            return Err(CrashError::TooMany);
        }
        l.all.push(now);
        l.by_address.retain(|_, v| !v.is_empty());
        Ok(())
    }
}

/// Printable text: no control characters but newlines and tabs, at most
/// `max` bytes (from the end when `tail`).
fn clean(s: &str, max: usize, tail: bool) -> String {
    let s: String = s.chars().filter(|c| !c.is_control() || *c == '\n' || *c == '\t').collect();
    if s.len() <= max {
        return s;
    }
    let mut cut = if tail { s.len() - max } else { max };
    while !s.is_char_boundary(cut) {
        if tail {
            cut += 1;
        } else {
            cut -= 1;
        }
    }
    if tail {
        format!("[…]\n{}", &s[cut..])
    } else {
        format!("{}…", &s[..cut])
    }
}

fn render(r: &Report, from: IpAddr, now: u64) -> String {
    let line = |s: &str| clean(s, FIELD_MAX, false).replace('\n', " ");
    format!(
        "Raport awarii klienta\nOdebrany: {} UTC\nOd: {from}\nWersja: {}\nSystem: {}\nCPU: {}\nGPU: {}\nSesja od: {}\n\n--- koniec logu gry ---\n{}\n",
        human(now),
        line(&r.version),
        line(&r.os),
        line(&r.cpu),
        line(&r.gpu),
        line(&r.started),
        clean(&r.log, LOG_MAX, true)
    )
}

/// Keep the newest `keep` reports (names start with the time, so they sort).
fn prune(dir: &Path, keep: usize) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<PathBuf> =
        rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "txt")).collect();
    files.sort();
    let n = files.len().saturating_sub(keep);
    for f in &files[..n] {
        let _ = std::fs::remove_file(f);
    }
}

/// "2026-10-01 20:15:03" (UTC).
fn human(secs: u64) -> String {
    let s = stamp(secs);
    let (date, time) = s.split_once('T').unwrap_or((&s, ""));
    format!("{date} {}", time.replace('-', ":"))
}

/// "2026-10-01T20-15-03" (UTC) from Unix seconds (a file name).
fn stamp(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Civil date from days since 1970-01-01 (H. Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}-{:02}-{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(log: &str) -> Report {
        Report {
            version: "0.1.0".into(),
            os: "macOS 15".into(),
            cpu: "Apple M2".into(),
            gpu: "Apple M2\u{7}".into(),
            started: String::new(),
            log: log.into(),
        }
    }

    #[test]
    fn dates_for_file_names() {
        assert_eq!(stamp(0), "1970-01-01T00-00-00");
        assert_eq!(stamp(1_790_000_000), "2026-09-21T14-13-20");
        assert_eq!(stamp(951_782_400), "2000-02-29T00-00-00", "leap day");
        assert_eq!(human(1_790_000_000), "2026-09-21 14:13:20");
    }

    #[test]
    fn saved_as_clean_text_rate_limited_and_pruned() {
        let dir = std::env::temp_dir().join(format!("crash-test-{}", fastrand::u32(..)));
        let c = Crashes::new(dir.clone());
        let ip: IpAddr = "203.0.113.7".parse().unwrap();
        let long = format!("start\n{}\nCrashHandlerException: Program crashed with signal 11\n", "x".repeat(LOG_MAX));
        let id = c.save(&report(&long), ip).unwrap();
        let text = std::fs::read_to_string(dir.join(format!("{id}.txt"))).unwrap();
        assert!(text.contains("Wersja: 0.1.0") && text.contains("GPU: Apple M2\n"), "control chars dropped");
        assert!(text.contains("signal 11") && !text.contains("start\n"), "the end of the log is kept");
        assert!(text.len() < LOG_MAX + 1024);
        assert_eq!(c.save(&report("   "), ip), Err(CrashError::Empty));
        for _ in 1..PER_ADDRESS {
            c.save(&report("boom"), ip).unwrap();
        }
        assert_eq!(c.save(&report("boom"), ip), Err(CrashError::TooMany), "5 an hour per address");
        assert!(c.save(&report("boom"), "203.0.113.8".parse().unwrap()).is_ok(), "others still can");
        prune(&dir, 2);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }
}
