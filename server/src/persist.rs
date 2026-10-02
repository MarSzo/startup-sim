//! Saving the game: characters (by nick) and the world (company, desks,
//! task boards, mail, kitchen, clock) in a SQLite file, so progress
//! survives a server restart.
//!
//! The game loop never waits for the disk: it serialises a snapshot (JSON)
//! and hands the changed rows to a writer thread, which stores them in one
//! transaction. Once a day the writer also makes a backup copy
//! (`VACUUM INTO`, the last `BACKUPS_KEPT` are kept).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::inventory::{kind as item_kind, Item};
use crate::needs::Needs;
use crate::protocol::{Appearance, Profile};
use crate::tasks::Boards;
use crate::workmail::PostOffice;

/// Schema version (`PRAGMA user_version`).
const SCHEMA: i32 = 1;
const BACKUPS_KEPT: usize = 7;
const DAY_SECS: u64 = 24 * 3600;

/// An item as saved: owned by a nick (ids are per session).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedItem {
    pub kind: u8,
    pub label: String,
    /// Nick of the owner ("" = nobody).
    pub owner: String,
    pub count: u8,
    pub unpaid: bool,
    pub stale: bool,
}

impl SavedItem {
    /// `owner_nick` maps the session id of the owner to a nick. A coffee
    /// gets cold over a restart: an empty (dirty) mug is saved.
    pub fn from_item(i: &Item, owner_nick: impl Fn(u16) -> String) -> SavedItem {
        let cold = matches!(i.kind, item_kind::COFFEE | item_kind::LATTE);
        SavedItem {
            kind: if cold { item_kind::EMPTY_CUP } else { i.kind },
            label: if cold { "Po kawie".into() } else { i.label.clone() },
            owner: if i.owner == 0 { String::new() } else { owner_nick(i.owner) },
            count: i.count,
            unpaid: i.unpaid,
            stale: i.stale,
        }
    }

    /// Back into an item (`id` fresh, `owner` = the owner's session id or 0).
    pub fn to_item(&self, id: u32, owner: u16) -> Item {
        Item {
            id,
            kind: self.kind,
            label: self.label.clone(),
            expires: None,
            owner,
            count: self.count.max(1),
            unpaid: self.unpaid,
            stale: self.stale,
            tainted: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedProfile {
    pub gender: u8,
    pub age: u8,
    pub city: String,
    pub email: String,
    pub appearance: [u8; 5],
}

impl SavedProfile {
    pub fn from_profile(p: &Profile) -> SavedProfile {
        let a = &p.appearance;
        SavedProfile {
            gender: p.gender,
            age: p.age,
            city: p.city.clone(),
            email: p.email.clone(),
            appearance: [a.skin, a.hair_style, a.hair_color, a.shirt, a.pants],
        }
    }

    pub fn to_profile(&self) -> Profile {
        let [skin, hair_style, hair_color, shirt, pants] = self.appearance;
        Profile {
            gender: self.gender,
            age: self.age,
            city: self.city.clone(),
            email: self.email.clone(),
            appearance: Appearance { skin, hair_style, hair_color, shirt, pants },
        }
    }
}

/// One character (by nick).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Character {
    pub nick: String,
    pub profile: SavedProfile,
    /// Hired (contract signed): comes back to work; otherwise the job
    /// hunt starts again on the portal.
    pub contract: bool,
    pub department: u8,
    pub position: Option<u8>,
    pub attempts: u8,
    pub money: i64,
    pub day: u32,
    pub worked_ds: u64,
    pub last_pay: (i64, u32),
    pub commute_mode: u8,
    pub pay_rate: i64,
    pub last_raise_day: Option<u32>,
    pub needs: Needs,
    /// Hands, then the pockets.
    pub inventory: Vec<Option<SavedItem>>,
    /// Interview questions already asked (question set -> question ids).
    #[serde(default)]
    pub seen_questions: HashMap<String, Vec<u32>>,
    /// Reprimands from the board (alcohol at work).
    #[serde(default)]
    pub reprimands: u8,
}

/// A laptop standing on a desk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedComputer {
    pub station: usize,
    pub item: SavedItem,
    pub locked: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedKitchen {
    pub mugs: u8,
    pub dirty: u8,
    pub washed: u8,
    pub running_until: Option<u32>,
    pub stored: Vec<SavedItem>,
    pub milk: u8,
    pub water: u8,
    pub juice: u8,
}

/// Everything besides the characters.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct World {
    pub clock_day: u32,
    pub clock_ds: u32,
    pub weather: u8,
    pub company_name: String,
    pub founder: Option<String>,
    #[serde(default)]
    pub descriptions: HashMap<u8, String>,
    /// Employee nick -> world day hired.
    pub hired_on: HashMap<String, u32>,
    /// Older saves (before positions): places and descriptions per offer.
    #[serde(default)]
    pub vacancies: HashMap<u8, u8>,
    /// Our startup's positions (None in older saves).
    #[serde(default)]
    pub positions: Option<Vec<crate::company::Position>>,
    pub computers: Vec<SavedComputer>,
    pub kitchen: Option<SavedKitchen>,
    pub boards: Boards,
    pub post: PostOffice,
}

/// What was in the file at start-up.
#[derive(Debug, Default)]
pub struct Loaded {
    pub world: Option<World>,
    pub characters: HashMap<String, Character>,
}

enum Job {
    /// Rows to write: the world (JSON) and characters (nick, JSON).
    Save { world: Option<String>, characters: Vec<(String, String)> },
    /// Everything written: answer when done.
    Flush(mpsc::Sender<()>),
}

/// The writer thread's handle.
pub struct Store {
    tx: mpsc::Sender<Job>,
    thread: Option<thread::JoinHandle<()>>,
    /// Last JSON sent per row (only changes are written).
    last_world: String,
    last_chars: HashMap<String, String>,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn open(path: &Path) -> rusqlite::Result<Connection> {
    let db = Connection::open(path)?;
    db.pragma_update(None, "journal_mode", "WAL")?;
    db.pragma_update(None, "synchronous", "NORMAL")?;
    let version: i32 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version < 1 {
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS world (key TEXT PRIMARY KEY, json TEXT NOT NULL, updated_at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS characters (nick TEXT PRIMARY KEY, json TEXT NOT NULL, updated_at INTEGER NOT NULL);",
        )?;
    }
    db.pragma_update(None, "user_version", SCHEMA)?;
    Ok(db)
}

/// Read the saved game (a missing file = a new world).
pub fn load(path: &Path) -> Result<Loaded, String> {
    let db = open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut loaded = Loaded::default();
    let world: Option<String> = db.query_row("SELECT json FROM world WHERE key = 'world'", [], |r| r.get(0)).ok();
    if let Some(json) = world {
        loaded.world = Some(serde_json::from_str(&json).map_err(|e| format!("world: {e}"))?);
    }
    let mut stmt = db.prepare("SELECT nick, json FROM characters").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).map_err(|e| e.to_string())?;
    for row in rows {
        let (nick, json) = row.map_err(|e| e.to_string())?;
        match serde_json::from_str::<Character>(&json) {
            Ok(c) => {
                loaded.characters.insert(nick, c);
            }
            Err(e) => eprintln!("save: skipping character '{nick}': {e}"),
        }
    }
    Ok(loaded)
}

impl Store {
    /// Open (create) the file and start the writer thread.
    pub fn start(path: PathBuf) -> Result<Store, String> {
        let mut db = open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let (tx, rx) = mpsc::channel::<Job>();
        let thread = thread::Builder::new()
            .name("save".into())
            .spawn(move || {
                let mut last_backup = 0;
                for job in rx {
                    match job {
                        Job::Save { world, characters } => {
                            if let Err(e) = write(&mut db, world.as_deref(), &characters) {
                                eprintln!("save: {e}");
                            }
                            if now_secs() >= last_backup + DAY_SECS {
                                last_backup = now_secs();
                                if let Err(e) = backup(&db, &path) {
                                    eprintln!("save: backup: {e}");
                                }
                            }
                        }
                        Job::Flush(done) => {
                            let _ = done.send(());
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Store { tx, thread: Some(thread), last_world: String::new(), last_chars: HashMap::new() })
    }

    /// Queue what changed since the last call.
    pub fn save(&mut self, world: &World, characters: &HashMap<String, Character>) {
        let world_json = serde_json::to_string(world).unwrap_or_default();
        let world = (world_json != self.last_world).then(|| {
            self.last_world = world_json.clone();
            world_json
        });
        let mut changed = Vec::new();
        for (nick, c) in characters {
            let json = serde_json::to_string(c).unwrap_or_default();
            if self.last_chars.get(nick) != Some(&json) {
                self.last_chars.insert(nick.clone(), json.clone());
                changed.push((nick.clone(), json));
            }
        }
        if world.is_some() || !changed.is_empty() {
            let _ = self.tx.send(Job::Save { world, characters: changed });
        }
    }

    /// Wait until everything queued is on disk.
    pub fn flush(&self) {
        let (tx, rx) = mpsc::channel();
        if self.tx.send(Job::Flush(tx)).is_ok() {
            let _ = rx.recv();
        }
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        self.flush();
        // Closing the channel ends the writer loop.
        let (tx, _) = mpsc::channel();
        drop(std::mem::replace(&mut self.tx, tx));
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn write(db: &mut Connection, world: Option<&str>, characters: &[(String, String)]) -> rusqlite::Result<()> {
    let now = now_secs() as i64;
    let tx = db.transaction()?;
    if let Some(json) = world {
        tx.execute(
            "INSERT INTO world (key, json, updated_at) VALUES ('world', ?1, ?2)
             ON CONFLICT(key) DO UPDATE SET json = excluded.json, updated_at = excluded.updated_at",
            params![json, now],
        )?;
    }
    for (nick, json) in characters {
        tx.execute(
            "INSERT INTO characters (nick, json, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(nick) DO UPDATE SET json = excluded.json, updated_at = excluded.updated_at",
            params![nick, json, now],
        )?;
    }
    tx.commit()
}

/// `<dir>/backups/<name>-<unix day>.db`, the newest `BACKUPS_KEPT` kept.
fn backup(db: &Connection, path: &Path) -> Result<(), String> {
    let dir = path.parent().unwrap_or(Path::new(".")).join("backups");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("world");
    let file = dir.join(format!("{stem}-{}.db", now_secs() / DAY_SECS));
    if !file.exists() {
        db.execute("VACUUM INTO ?1", params![file.to_string_lossy()]).map_err(|e| e.to_string())?;
        let size = std::fs::metadata(&file).map_or(0, |m| m.len());
        println!("* save: backup {} ({} KB)", file.display(), size / 1024);
    }
    let mut old: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(stem) && n.ends_with(".db")))
        .collect();
    old.sort();
    while old.len() > BACKUPS_KEPT {
        let _ = std::fs::remove_file(old.remove(0));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("startup-sim-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("world.db")
    }

    fn character(nick: &str, money: i64) -> Character {
        Character {
            nick: nick.into(),
            profile: SavedProfile {
                gender: 1,
                age: 28,
                city: "Kraków".into(),
                email: "ola@example.com".into(),
                appearance: [1, 2, 3, 4, 5],
            },
            contract: true,
            department: 1,
            position: Some(2),
            attempts: 1,
            money,
            day: 4,
            worked_ds: 0,
            last_pay: (0, 0),
            commute_mode: 5,
            pay_rate: 3000,
            last_raise_day: None,
            needs: Needs::default(),
            inventory: vec![
                None,
                Some(SavedItem { kind: 2, label: "Ola · IT".into(), owner: "Ola".into(), count: 1, unpaid: false, stale: false }),
                None,
                None,
            ],
            seen_questions: HashMap::from([("programming".to_string(), vec![11, 22])]),
            reprimands: 0,
        }
    }

    #[test]
    fn save_restart_load() {
        let path = tmp("roundtrip");
        {
            let mut store = Store::start(path.clone()).unwrap();
            let mut world = World { clock_day: 3, company_name: "Pixel Pierogi".into(), founder: Some("Ola".into()), ..Default::default() };
            world.boards.create(1, "Ola", "Naprawić logowanie", 2).unwrap();
            world.post.send("HR", "Ola", "Witamy!", "Cześć", 3, 540);
            let chars = HashMap::from([("Ola".to_string(), character("Ola", 250_00))]);
            store.save(&world, &chars);
            // Unchanged: nothing new is queued; changed: only that row.
            store.save(&world, &chars);
            let chars = HashMap::from([("Ola".to_string(), character("Ola", 300_00))]);
            store.save(&world, &chars);
        } // dropped = flushed
        let loaded = load(&path).unwrap();
        let w = loaded.world.expect("world saved");
        assert_eq!((w.clock_day, w.company_name.as_str(), w.founder.as_deref()), (3, "Pixel Pierogi", Some("Ola")));
        assert_eq!(w.boards.board(1).len(), 1);
        assert_eq!(w.post.inbox("Ola").map(|i| i.mails.len()), Some(1));
        let ola = &loaded.characters["Ola"];
        assert_eq!((ola.money, ola.day, ola.inventory[1].as_ref().map(|i| i.kind)), (300_00, 4, Some(2)));
        assert!(path.parent().unwrap().join("backups").read_dir().unwrap().count() >= 1, "a backup was made");
    }

    #[test]
    fn coffee_goes_cold_over_a_restart() {
        let item = Item {
            id: 7,
            kind: item_kind::COFFEE,
            label: "Kawa".into(),
            expires: Some(100),
            owner: 3,
            count: 1,
            unpaid: false,
            stale: false,
            tainted: false,
        };
        let s = SavedItem::from_item(&item, |_| "Ola".into());
        assert_eq!((s.kind, s.owner.as_str()), (item_kind::EMPTY_CUP, "Ola"));
        assert_eq!(s.to_item(9, 3).expires, None);
    }
}
