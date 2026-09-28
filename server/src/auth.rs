//! Player accounts: the character's nick is the login, the password is
//! kept only as an Argon2id hash. Logging in (over HTTPS, `http.rs`) gives
//! a short-lived ticket for the game's UDP `Connect` and a refresh token
//! ("remember me"; stored hashed, rotated on every use).
//!
//! Accounts live in the save file (tables `accounts`, `refresh_tokens`).

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::protocol::MAX_NICK_BYTES;

pub const PASSWORD_MIN: usize = 8;
pub const PASSWORD_MAX: usize = 128;
pub const NICK_MIN: usize = 2;
/// A game ticket works this long (and may be used again for reconnects).
pub const TICKET_TTL: Duration = Duration::from_secs(10 * 60);
/// "Remember me": a refresh token lasts 30 days.
pub const REFRESH_DAYS: u64 = 30;
/// Wrong passwords in a row (per nick and address) before a pause.
const MAX_FAILS: u32 = 5;
const LOCKOUT: Duration = Duration::from_secs(60);
/// New accounts per address per hour.
const REGISTER_PER_HOUR: u32 = 10;

/// What the player sees when something is wrong (Polish, for the UI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    BadNick,
    WeakPassword,
    NickTaken,
    WrongPassword,
    TooManyAttempts,
    Expired,
    Internal(String),
}

impl AuthError {
    pub fn message(&self) -> String {
        match self {
            AuthError::BadNick => format!("Nick: {NICK_MIN}–{MAX_NICK_BYTES} znaków, bez znaków specjalnych."),
            AuthError::WeakPassword => format!("Hasło musi mieć co najmniej {PASSWORD_MIN} znaków."),
            AuthError::NickTaken => "Ten nick ma już konto — zaloguj się.".into(),
            AuthError::WrongPassword => "Zły nick albo hasło.".into(),
            AuthError::TooManyAttempts => "Za dużo prób — spróbuj za minutę.".into(),
            AuthError::Expired => "Sesja wygasła — zaloguj się ponownie.".into(),
            AuthError::Internal(_) => "Błąd serwera — spróbuj później.".into(),
        }
    }

    /// HTTP status for the API.
    pub fn status(&self) -> u16 {
        match self {
            AuthError::BadNick | AuthError::WeakPassword => 400,
            AuthError::NickTaken => 409,
            AuthError::WrongPassword | AuthError::Expired => 401,
            AuthError::TooManyAttempts => 429,
            AuthError::Internal(_) => 500,
        }
    }
}

impl From<rusqlite::Error> for AuthError {
    fn from(e: rusqlite::Error) -> Self {
        AuthError::Internal(e.to_string())
    }
}

/// A successful login / registration / refresh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Granted {
    pub nick: String,
    pub ticket: String,
    pub refresh: String,
    /// The account already has a character (skip the creation screen).
    pub character: bool,
}

struct Ticket {
    nick: String,
    expires: Instant,
}

struct Inner {
    db: Connection,
    tickets: HashMap<String, Ticket>,
    /// (key, count, since) for failed logins and registrations.
    fails: HashMap<String, (u32, Instant)>,
    registrations: HashMap<String, (u32, Instant)>,
}

/// Shared by the HTTPS thread and the game loop.
#[derive(Clone)]
pub struct Auth {
    inner: Arc<Mutex<Inner>>,
}

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// 32 random bytes, hex.
pub fn random_token() -> String {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).expect("OS randomness");
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn sha256(s: &str) -> String {
    Sha256::digest(s.as_bytes()).iter().map(|x| format!("{x:02x}")).collect()
}

fn hash_password(password: &str) -> Result<String, AuthError> {
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).map_err(|e| AuthError::Internal(e.to_string()))?;
    let salt = SaltString::encode_b64(&salt).map_err(|e| AuthError::Internal(e.to_string()))?;
    Argon2::default().hash_password(password.as_bytes(), &salt).map(|h| h.to_string()).map_err(|e| AuthError::Internal(e.to_string()))
}

fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok())
}

/// A nick that can be an account: trimmed, 2..=16 bytes, letters, digits,
/// spaces, `_`, `-`, `.`.
pub fn clean_nick(nick: &str) -> Option<String> {
    let n = nick.trim();
    let ok = n.len() >= NICK_MIN
        && n.len() <= MAX_NICK_BYTES
        && n.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '_' | '-' | '.'));
    ok.then(|| n.to_string())
}

fn open_db(path: &Path) -> rusqlite::Result<Connection> {
    let db = Connection::open(path)?;
    db.pragma_update(None, "journal_mode", "WAL")?;
    db.busy_timeout(Duration::from_secs(5))?;
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS accounts (
            id INTEGER PRIMARY KEY,
            nick TEXT NOT NULL UNIQUE COLLATE NOCASE,
            pass TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            last_login INTEGER
         );
         CREATE TABLE IF NOT EXISTS refresh_tokens (
            hash TEXT PRIMARY KEY,
            account INTEGER NOT NULL,
            expires_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS characters (nick TEXT PRIMARY KEY, json TEXT NOT NULL, updated_at INTEGER NOT NULL);",
    )?;
    Ok(db)
}

impl Auth {
    pub fn open(path: &Path) -> Result<Auth, String> {
        let db = open_db(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Auth { inner: Arc::new(Mutex::new(Inner { db, tickets: HashMap::new(), fails: HashMap::new(), registrations: HashMap::new() })) })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A nick with an account (guests may not use it).
    pub fn is_registered(&self, nick: &str) -> bool {
        let g = self.lock();
        g.db.query_row("SELECT 1 FROM accounts WHERE nick = ?1", [nick], |_| Ok(())).optional().ok().flatten().is_some()
    }

    /// New account. A saved character with this nick (from before accounts)
    /// becomes this account's.
    pub fn register(&self, nick: &str, password: &str, from: &str) -> Result<Granted, AuthError> {
        let nick = clean_nick(nick).ok_or(AuthError::BadNick)?;
        if password.chars().count() < PASSWORD_MIN || password.len() > PASSWORD_MAX {
            return Err(AuthError::WeakPassword);
        }
        {
            let mut g = self.lock();
            let e = g.registrations.entry(from.to_string()).or_insert((0, Instant::now()));
            if e.1.elapsed() > Duration::from_secs(3600) {
                *e = (0, Instant::now());
            }
            if e.0 >= REGISTER_PER_HOUR {
                return Err(AuthError::TooManyAttempts);
            }
            e.0 += 1;
        }
        let hash = hash_password(password)?; // slow: outside the lock
        let g = self.lock();
        let taken = g.db.query_row("SELECT 1 FROM accounts WHERE nick = ?1", [&nick], |_| Ok(())).optional()?.is_some();
        if taken {
            return Err(AuthError::NickTaken);
        }
        g.db.execute("INSERT INTO accounts (nick, pass, created_at, last_login) VALUES (?1, ?2, ?3, ?3)", params![nick, hash, now_secs()])?;
        let id = g.db.last_insert_rowid();
        drop(g);
        self.grant(id, &nick)
    }

    pub fn login(&self, nick: &str, password: &str, from: &str) -> Result<Granted, AuthError> {
        let key = format!("{}|{from}", nick.trim().to_lowercase());
        {
            let mut g = self.lock();
            if let Some((n, since)) = g.fails.get(&key).copied() {
                if n >= MAX_FAILS && since.elapsed() < LOCKOUT {
                    return Err(AuthError::TooManyAttempts);
                }
                if since.elapsed() >= LOCKOUT {
                    g.fails.remove(&key);
                }
            }
        }
        let row: Option<(i64, String, String)> = {
            let g = self.lock();
            g.db.query_row("SELECT id, nick, pass FROM accounts WHERE nick = ?1", [nick.trim()], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).optional()?
        };
        // Verify even for unknown nicks (the same time either way).
        let fake = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzb21lc2FsdA$Zl7r9zDgwQn0DkI1eD5dmWmJb2Sv7kR5x3VtCmfkUNI";
        let ok = verify_password(password, row.as_ref().map_or(fake, |r| r.2.as_str())) && row.is_some();
        let Some((id, nick, _)) = row.filter(|_| ok) else {
            let mut g = self.lock();
            let e = g.fails.entry(key).or_insert((0, Instant::now()));
            e.0 += 1;
            e.1 = Instant::now();
            return Err(AuthError::WrongPassword);
        };
        {
            let mut g = self.lock();
            g.fails.remove(&key);
            g.db.execute("UPDATE accounts SET last_login = ?1 WHERE id = ?2", params![now_secs(), id])?;
        }
        self.grant(id, &nick)
    }

    /// A new password (knowing the old one); other "remember me" logins
    /// stop working.
    pub fn change_password(&self, nick: &str, old: &str, new: &str, from: &str) -> Result<Granted, AuthError> {
        if new.chars().count() < PASSWORD_MIN || new.len() > PASSWORD_MAX {
            return Err(AuthError::WeakPassword);
        }
        let g = self.login(nick, old, from)?;
        let hash = hash_password(new)?;
        let lock = self.lock();
        lock.db.execute("UPDATE accounts SET pass = ?1 WHERE nick = ?2", params![hash, g.nick])?;
        lock.db.execute(
            "DELETE FROM refresh_tokens WHERE account = (SELECT id FROM accounts WHERE nick = ?1) AND hash != ?2",
            params![g.nick, sha256(&g.refresh)],
        )?;
        Ok(g)
    }

    /// "Remember me": a new ticket (and a new refresh token) for an old one.
    pub fn refresh(&self, refresh: &str) -> Result<Granted, AuthError> {
        let hash = sha256(refresh);
        let row: Option<(i64, String)> = {
            let g = self.lock();
            let row = g
                .db
                .query_row(
                    "SELECT a.id, a.nick FROM refresh_tokens t JOIN accounts a ON a.id = t.account WHERE t.hash = ?1 AND t.expires_at > ?2",
                    params![hash, now_secs()],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            g.db.execute("DELETE FROM refresh_tokens WHERE hash = ?1 OR expires_at <= ?2", params![hash, now_secs()])?;
            row
        };
        let (id, nick) = row.ok_or(AuthError::Expired)?;
        self.grant(id, &nick)
    }

    pub fn logout(&self, refresh: &str) {
        let g = self.lock();
        let _ = g.db.execute("DELETE FROM refresh_tokens WHERE hash = ?1", [sha256(refresh)]);
    }

    fn grant(&self, account: i64, nick: &str) -> Result<Granted, AuthError> {
        let (ticket, refresh) = (random_token(), random_token());
        let mut g = self.lock();
        g.db.execute(
            "INSERT INTO refresh_tokens (hash, account, expires_at) VALUES (?1, ?2, ?3)",
            params![sha256(&refresh), account, now_secs() + (REFRESH_DAYS * 86_400) as i64],
        )?;
        let character = g.db.query_row("SELECT 1 FROM characters WHERE nick = ?1", [nick], |_| Ok(())).optional()?.is_some();
        g.tickets.retain(|_, t| t.expires > Instant::now());
        g.tickets.insert(ticket.clone(), Ticket { nick: nick.to_string(), expires: Instant::now() + TICKET_TTL });
        Ok(Granted { nick: nick.to_string(), ticket, refresh, character })
    }

    /// UDP Connect: whose ticket is it (the account's nick), if still valid.
    pub fn redeem(&self, ticket: &str) -> Option<String> {
        let g = self.lock();
        g.tickets.get(ticket).filter(|t| t.expires > Instant::now()).map(|t| t.nick.clone())
    }

    /// Admin (server console): a new one-time password for `nick`; its
    /// "remember me" tokens stop working.
    pub fn reset_password(&self, nick: &str) -> Result<String, AuthError> {
        let password: String = random_token().chars().take(12).collect();
        let hash = hash_password(&password)?;
        let g = self.lock();
        let id: Option<i64> = g.db.query_row("SELECT id FROM accounts WHERE nick = ?1", [nick], |r| r.get(0)).optional()?;
        let id = id.ok_or(AuthError::WrongPassword)?;
        g.db.execute("UPDATE accounts SET pass = ?1 WHERE id = ?2", params![hash, id])?;
        g.db.execute("DELETE FROM refresh_tokens WHERE account = ?1", [id])?;
        Ok(password)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth(name: &str) -> Auth {
        let dir = std::env::temp_dir().join(format!("startup-sim-auth-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Auth::open(&dir.join("world.db")).unwrap()
    }

    #[test]
    fn register_login_refresh_logout() {
        let a = auth("flow");
        assert_eq!(a.register("x", "haslo1234", "ip").unwrap_err(), AuthError::BadNick);
        assert_eq!(a.register("Ola", "krótkie", "ip").unwrap_err(), AuthError::WeakPassword);
        let g = a.register("Ola", "tajne-haslo", "ip").unwrap();
        assert_eq!((g.nick.as_str(), g.character), ("Ola", false));
        assert_eq!(a.redeem(&g.ticket).as_deref(), Some("Ola"));
        assert_eq!(a.redeem("nonsense"), None);
        assert_eq!(a.register("ola", "inne-haslo1", "ip").unwrap_err(), AuthError::NickTaken, "nicks ignore case");
        assert!(a.is_registered("OLA"));
        assert_eq!(a.login("Ola", "zle-haslo", "ip").unwrap_err(), AuthError::WrongPassword);
        let g2 = a.login("ola", "tajne-haslo", "ip").unwrap();
        assert_eq!(g2.nick, "Ola", "the account's own spelling");
        // Refresh tokens rotate: the old one is used up.
        let g3 = a.refresh(&g2.refresh).unwrap();
        assert_eq!(a.refresh(&g2.refresh).unwrap_err(), AuthError::Expired);
        a.logout(&g3.refresh);
        assert_eq!(a.refresh(&g3.refresh).unwrap_err(), AuthError::Expired);
        // The admin resets the password: the old one and "remember me" stop working.
        let g4 = a.login("Ola", "tajne-haslo", "ip").unwrap();
        let new = a.reset_password("Ola").unwrap();
        assert_eq!(a.login("Ola", "tajne-haslo", "ip").unwrap_err(), AuthError::WrongPassword);
        assert!(a.login("Ola", &new, "ip").is_ok());
        assert_eq!(a.refresh(&g4.refresh).unwrap_err(), AuthError::Expired);
        // ...and Ola sets her own again.
        assert_eq!(a.change_password("Ola", &new, "krotko", "ip").unwrap_err(), AuthError::WeakPassword);
        assert!(a.change_password("Ola", &new, "moje-nowe-haslo", "ip").is_ok());
        assert!(a.login("Ola", "moje-nowe-haslo", "ip").is_ok());
    }

    #[test]
    fn a_saved_character_goes_to_the_first_account_with_its_nick() {
        let a = auth("claim");
        {
            let g = a.lock();
            g.db.execute("INSERT INTO characters (nick, json, updated_at) VALUES ('Ewa', '{}', 0)", []).unwrap();
        }
        let g = a.register("Ewa", "haslo-ewy-1", "ip").unwrap();
        assert!(g.character, "the stage-1 character is hers now");
        assert_eq!(a.register("Ewa", "ktos-inny-1", "ip2").unwrap_err(), AuthError::NickTaken);
    }

    #[test]
    fn too_many_wrong_passwords_lock_for_a_while() {
        let a = auth("lockout");
        a.register("Kuba", "dobre-haslo", "ip").unwrap();
        for _ in 0..MAX_FAILS {
            assert_eq!(a.login("Kuba", "zle", "1.2.3.4").unwrap_err(), AuthError::WrongPassword);
        }
        assert_eq!(a.login("Kuba", "dobre-haslo", "1.2.3.4").unwrap_err(), AuthError::TooManyAttempts);
        assert!(a.login("Kuba", "dobre-haslo", "5.6.7.8").is_ok(), "per address");
    }
}
