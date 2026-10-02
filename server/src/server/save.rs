//! The game saved in SQLite (`crate::persist`): loading the world at
//! start-up, restoring a character when its nick connects again, and
//! snapshots every `SAVE_TICKS` (sooner after money / hiring changes) and
//! at shutdown.
//!
//! Stage 1 of accounts: a character is found by its nick (no password yet).

use std::collections::HashMap;

use crate::computer::Computer;
use crate::persist::{self, Character, SavedComputer, SavedItem, SavedKitchen, SavedProfile, World};

use super::player::{refresh, Player, Stage};
use super::Server;

/// A snapshot every 10 s.
const SAVE_TICKS: u32 = 200;

/// What the world save knows by nick while those people are offline.
#[derive(Debug, Default)]
pub(super) struct Offline {
    /// Characters not online (and the last snapshot of those who are).
    pub(super) characters: HashMap<String, Character>,
    pub(super) founder: Option<String>,
    pub(super) hired_on: HashMap<String, u32>,
    /// Laptops on desks whose owner isn't here: computer handle -> nick.
    pub(super) laptop_owner: HashMap<u16, String>,
}

impl Server {
    /// Saving is on (a save file): the world is persistent, so people who
    /// leave keep their job, desk and company.
    pub(super) fn persistent(&self) -> bool {
        self.store.is_some()
    }

    /// At start-up: open the save file and put the saved world back.
    pub(super) fn load_save(&mut self) -> Result<(), String> {
        let Some(path) = self.cfg.save_path.clone() else { return Ok(()) };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let loaded = persist::load(&path)?;
        let n = loaded.characters.len();
        if let Some(w) = loaded.world {
            self.apply_world(w);
        }
        self.offline.characters = loaded.characters;
        self.store = Some(persist::Store::start(path.clone())?);
        self.log(format!("* save: {} ({n} characters)", path.display()));
        Ok(())
    }

    fn apply_world(&mut self, w: World) {
        self.clock.day = w.clock_day.max(1);
        self.clock.ds = w.clock_ds % (crate::clock::MIN_PER_DAY * crate::clock::DS_PER_MIN);
        if self.cfg.weather.is_none() && w.weather != 0 {
            self.weather.now = w.weather;
        }
        if !w.company_name.is_empty() {
            self.company.name = w.company_name;
        }
        match w.positions {
            Some(list) => self.positions = list,
            // Saved before positions could be edited: places and descriptions.
            None => {
                for pos in &mut self.positions {
                    if let Some(&n) = w.vacancies.get(&pos.id) {
                        pos.places = n;
                    }
                    if let Some(d) = w.descriptions.get(&pos.id) {
                        pos.description = d.clone();
                    }
                }
            }
        }
        self.offline.founder = w.founder;
        self.offline.hired_on = w.hired_on;

        for c in w.computers {
            if c.station >= self.workstations.len() || self.computers.iter().any(|x| x.station == c.station) {
                continue;
            }
            let handle = self.alloc_handle();
            let id = self.next_item_id();
            self.computers.push(Computer { handle, station: c.station, item: c.item.to_item(id, 0), locked: c.locked, user: None });
            if !c.item.owner.is_empty() {
                self.offline.laptop_owner.insert(handle, c.item.owner);
            }
        }
        if let (Some(k), Some(s)) = (self.kitchen.as_mut(), w.kitchen) {
            k.mugs = s.mugs;
            k.dirty = s.dirty;
            k.washed = s.washed;
            k.running_until = s.running_until;
            k.milk = s.milk;
            k.water = s.water;
            k.juice = s.juice;
            k.stored = Vec::new();
            for it in &s.stored {
                let id = self.next_item_id;
                self.next_item_id = self.next_item_id.wrapping_add(1).max(1);
                k.stored.push(it.to_item(id, 0));
            }
        }
        self.boards = w.boards;
        self.post = w.post;
        self.clock_dirty = true;
    }

    fn next_item_id(&mut self) -> u32 {
        let id = self.next_item_id;
        self.next_item_id = self.next_item_id.wrapping_add(1).max(1);
        id
    }

    /// Some other account character (online or saved) has this e-mail.
    pub(super) fn email_taken(&self, email: &str, nick: &str) -> bool {
        let email = email.to_lowercase();
        let other = |n: &str| n.to_lowercase() != nick.to_lowercase();
        self.players.values().any(|p| !p.guest && other(&p.nick) && p.profile.email.to_lowercase() == email)
            || self.offline.characters.values().any(|c| other(&c.nick) && c.profile.email.to_lowercase() == email)
    }

    /// A nick for a session id (online players, else "").
    fn nick_of(&self, id: u16) -> String {
        self.players.get(&id).map(|p| p.nick.clone()).unwrap_or_default()
    }

    fn capture(&self, p: &Player) -> Character {
        let nick_of = |id: u16| self.nick_of(id);
        let slot = |it: &Option<crate::inventory::Item>| it.as_ref().filter(|i| !i.unpaid).map(|i| SavedItem::from_item(i, nick_of));
        let mut inventory = vec![slot(&p.inventory.hands)];
        inventory.extend(p.inventory.pockets.iter().map(slot));
        Character {
            nick: p.nick.clone(),
            profile: SavedProfile::from_profile(&p.profile),
            contract: p.contract,
            department: p.department,
            position: p.position,
            attempts: p.attempts,
            money: p.money,
            day: p.day,
            worked_ds: p.worked_ds,
            last_pay: p.last_pay,
            commute_mode: p.commute_mode,
            pay_rate: p.pay_rate,
            last_raise_day: p.last_raise_day,
            needs: p.needs.clone(),
            inventory,
            seen_questions: p.seen_questions.clone(),
            reprimands: p.reprimands,
        }
    }

    fn world_snapshot(&self) -> World {
        let nick_of = |id: u16| self.nick_of(id);
        let mut hired_on = self.offline.hired_on.clone();
        for (&pid, &day) in &self.company.hired_on {
            if let Some(p) = self.players.get(&pid) {
                hired_on.insert(p.nick.clone(), day);
            }
        }
        let founder = match self.company.founder {
            Some(pid) => self.players.get(&pid).map(|p| p.nick.clone()).or_else(|| self.offline.founder.clone()),
            None => self.offline.founder.clone(),
        };
        World {
            clock_day: self.clock.day,
            clock_ds: self.clock.ds,
            weather: self.weather.now,
            company_name: self.company.name.clone(),
            founder,
            descriptions: Default::default(),
            hired_on,
            vacancies: Default::default(),
            positions: Some(self.positions.clone()),
            computers: self
                .computers
                .iter()
                .map(|c| {
                    let mut item = SavedItem::from_item(&c.item, nick_of);
                    if item.owner.is_empty() {
                        item.owner = self.offline.laptop_owner.get(&c.handle).cloned().unwrap_or_default();
                    }
                    SavedComputer { station: c.station, item, locked: c.locked }
                })
                .collect(),
            kitchen: self.kitchen.as_ref().map(|k| SavedKitchen {
                mugs: k.mugs,
                dirty: k.dirty,
                washed: k.washed,
                running_until: k.running_until,
                stored: k.stored.iter().map(|i| SavedItem::from_item(i, nick_of)).collect(),
                milk: k.milk,
                water: k.water,
                juice: k.juice,
            }),
            boards: self.boards.clone(),
            post: self.post.clone(),
        }
    }

    /// Every tick: a snapshot now and then (or soon after something
    /// important: money, hiring).
    pub(super) fn tick_save(&mut self) {
        if self.store.is_none() || !(self.save_soon || self.tick.is_multiple_of(SAVE_TICKS)) {
            return;
        }
        self.save_now();
    }

    pub(super) fn save_now(&mut self) {
        self.save_soon = false;
        let online: Vec<Character> = self
            .players
            .values()
            .filter(|p| !p.guest && (!matches!(p.stage, Stage::Portal(_)) || p.day > 1 || p.money != 0))
            .map(|p| self.capture(p))
            .collect();
        for c in online {
            self.offline.characters.insert(c.nick.clone(), c);
        }
        let world = self.world_snapshot();
        if let Some(store) = self.store.as_mut() {
            store.save(&world, &self.offline.characters);
        }
    }

    /// Shutdown: the last snapshot, written before returning.
    pub fn shutdown(&mut self) {
        if self.store.is_some() {
            self.save_now();
            if let Some(s) = self.store.as_ref() {
                s.flush();
            }
            self.log("* save: written");
        }
    }

    /// A nick connected: bring its character back (returns whether one was
    /// found). Called right after the player was created.
    pub(super) fn restore(&mut self, pid: u16) -> bool {
        let Some(nick) = self.players.get(&pid).map(|p| p.nick.clone()) else { return false };
        let Some(c) = self.offline.characters.get(&nick).cloned() else { return false };
        let night = self.clock.is_night();
        let ids: Vec<u32> = (0..c.inventory.len()).map(|_| self.next_item_id()).collect();
        let owner_id = |n: &str| if n == nick { pid } else { self.players.values().find(|o| o.nick == n).map_or(0, |o| o.id) };
        let items: Vec<Option<crate::inventory::Item>> =
            c.inventory.iter().zip(&ids).map(|(it, &id)| it.as_ref().map(|i| i.to_item(id, owner_id(&i.owner)))).collect();
        let Some(p) = self.players.get_mut(&pid) else { return false };
        p.profile = c.profile.to_profile();
        p.money = c.money;
        p.day = c.day.max(1);
        p.commute_mode = c.commute_mode;
        p.pay_rate = c.pay_rate;
        p.last_raise_day = c.last_raise_day;
        p.needs = c.needs.clone();
        p.last_pay = c.last_pay;
        p.worked_ds = c.worked_ds;
        p.attempts = c.attempts;
        p.seen_questions = c.seen_questions.clone();
        p.reprimands = c.reprimands;
        if c.contract {
            p.contract = true;
            p.department = c.department;
            p.position = c.position;
            p.stage = if night { Stage::Home { arrive_at: None } } else { Stage::Working };
            let mut slots = items.into_iter();
            p.inventory.hands = slots.next().flatten();
            for pocket in &mut p.inventory.pockets {
                *pocket = slots.next().flatten();
            }
            refresh(p);
        }
        // A board member saved before the breathalyser existed gets one.
        let board_without =
            c.contract && c.department == crate::company::BOARD_DEPARTMENT && !p.inventory.has(crate::inventory::kind::BREATHALYSER);
        // The company knows them by nick.
        if self.offline.founder.as_deref() == Some(nick.as_str()) && c.contract {
            self.company.founder = Some(pid);
        }
        if let Some(&day) = self.offline.hired_on.get(&nick) {
            self.company.hired_on.insert(pid, day);
        }
        let mine: Vec<u16> = self.offline.laptop_owner.iter().filter(|(_, n)| **n == nick).map(|(&h, _)| h).collect();
        for h in mine {
            self.offline.laptop_owner.remove(&h);
            if let Some(comp) = self.computers.iter_mut().find(|x| x.handle == h) {
                comp.item.owner = pid;
            }
        }
        if board_without {
            self.give_new(pid, crate::inventory::kind::BREATHALYSER);
        }
        self.clock_dirty = true;
        self.says.push(super::Say::new(pid, "Z powrotem — wszystko jest tam, gdzie było."));
        self.log(format!("* save: {nick} is back (day {}, {})", c.day, crate::shop::zl(c.money)));
        true
    }

    /// Persistent world: someone leaves — remember them, keep their job,
    /// desk (laptop) and company.
    pub(super) fn remember_leaving(&mut self, p: &Player) {
        if !self.persistent() || p.guest {
            return;
        }
        let c = self.capture(p);
        self.offline.characters.insert(p.nick.clone(), c);
        if self.company.founder == Some(p.id) {
            self.offline.founder = Some(p.nick.clone());
        }
        if let Some(&day) = self.company.hired_on.get(&p.id) {
            self.offline.hired_on.insert(p.nick.clone(), day);
        }
        for comp in &mut self.computers {
            if comp.item.owner == p.id {
                comp.item.owner = 0;
                self.offline.laptop_owner.insert(comp.handle, p.nick.clone());
            }
        }
        self.save_soon = true;
    }
}
