use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use game::args::Args;
use game::auth::Auth;
use game::http::{self, Tls};
use game::building::{default_building_path, Building};
use game::net::LinkConditions;
use game::recruitment::{default_recruitment_path, Recruitment};
use game::server::{Config, Server, DEFAULT_CLIENT_TIMEOUT, TICK_HZ};

const HELP: &str = "\
Startup sim - dedicated game server

USAGE: cargo run [--release] -- [OPTIONS]

OPTIONS:
  --bind <addr>         listen address        [default: [::]:7777, dual-stack IPv6+IPv4;
                                               falls back to 0.0.0.0:7777 without IPv6]
  --map <path>          building JSON         [default: ../client/maps/building.json]
  --max-players <n>                           [default: 256]
  --stats-secs <n>      stats log interval    [default: 5]
  --lag-ms <ms>         simulated one-way delay, each direction (RTT += 2x)
  --jitter-ms <ms>      simulated extra random delay 0..=ms
  --loss <p>            simulated packet loss per direction, e.g. 0.02
  --start-with-card     every player starts with an employee card (load tests / bots)
  --skip-recruitment    spawn straight into the building, no job portal (dev)
  --start-employed      like --skip-recruitment, but already hired: contract, card and
                        laptop, spawned at a desk (departments alternate by player id)
  --start-time <hh:mm>  game time when the server starts (day 1)  [default: 8:00]
  --time-scale <n>      daytime passes n times faster (dev)       [default: 1 = 1 h / 5 min]
  --weather <kind>      fixed weather: sun, clouds, rain, storm, fog (dev) [default: changing]
  --treats              a tray of sweets in the chill room right away (dev)
  --stale-fruit <pct>   chance that fruit from the bowl is stale      [default: 15]
  --cleaning-at <hh:mm> when the cleaner starts her round (exactly)   [default: random 15:00-16:00]
  --start-cigarettes    with --start-employed: a pack of cigarettes in the pocket (dev)
  --needs-speed <n>     needs (hunger, energy...) change n times faster (dev)
  --recruitment <path>  recruitment JSON  [default: data/recruitment.json]
  --save <path>         save file (SQLite): characters, company, desks, boards, mail,
                        clock; a daily backup in <dir>/backups   [default: saves/world.db]
  --no-save             don't load or save anything (dev / tests; everyone plays as a guest)
  --allow-guests        players without an account may join (nothing of theirs is saved)
  --auth-bind <addr>    login API (HTTPS)                  [default: the game port + 1]
  --tls-cert <pem>      certificate for the login API (e.g. Let's Encrypt fullchain.pem)
  --tls-key <pem>       its private key       [default: a self-made one in <save dir>/tls,
                                               pinned by the clients on first contact]
  --list-accounts       admin: print the accounts (nick, created, last login) and exit
  --reset-password <nick>  admin: set a new one-time password for an account, print it
                        and exit (its remember-me logins stop working)
";

const PORT: u16 = 7777;
/// Player ids live below the NPC ids; keep the cap well inside that range.
const MAX_PLAYERS_LIMIT: usize = 4096;

/// Why the server didn't start.
enum StartError {
    /// Bad command line (exit status 2).
    Usage(String),
    /// Couldn't load data or bind the socket (exit status 1).
    Fatal(String),
}

impl From<game::args::InvalidArg> for StartError {
    fn from(e: game::args::InvalidArg) -> Self {
        StartError::Usage(e.to_string())
    }
}

fn main() -> ExitCode {
    let args = Args::from_env();
    if args.flag("help") {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    }
    // Ctrl+C / SIGTERM: save, then stop.
    if let Err(e) = ctrlc::set_handler(|| game::server::STOP.store(true, std::sync::atomic::Ordering::Relaxed)) {
        eprintln!("no Ctrl+C handler: {e}");
    }
    // Admin: list the accounts and quit.
    if args.flag("list-accounts") {
        let path = args.str("save").map_or_else(|| PathBuf::from("saves/world.db"), PathBuf::from);
        return match Auth::open(&path) {
            Ok(a) => {
                let day = |t: i64| format!("{} dni temu", (now_secs() - t).max(0) / 86_400);
                for (nick, created, last) in a.list() {
                    println!("{nick:<16}  konto od: {:<14}  ostatnio: {}", day(created), last.map_or("-".into(), day));
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        };
    }
    // Admin: reset an account's password and quit.
    if let Some(nick) = args.str("reset-password") {
        let path = args.str("save").map_or_else(|| PathBuf::from("saves/world.db"), PathBuf::from);
        return match Auth::open(&path).map_err(|e| e.to_string()).and_then(|a| a.reset_password(nick).map_err(|e| e.message())) {
            Ok(pw) => {
                println!("New password for '{nick}': {pw}\n(tell them to change it after logging in)");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        };
    }
    match start(&args) {
        Ok(mut server) => server.run(),
        Err(StartError::Usage(msg)) => {
            eprintln!("{msg}");
            ExitCode::from(2)
        }
        Err(StartError::Fatal(msg)) => {
            eprintln!("{msg}");
            ExitCode::FAILURE
        }
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// Load the data, parse the options, bind the socket.
fn start(args: &Args) -> Result<Server, StartError> {
    let map_path = args.str("map").map_or_else(default_building_path, PathBuf::from);
    let building = Building::load(&map_path).map_err(|e| StartError::Fatal(format!("failed to load map: {e}")))?;
    let rec_path = args.str("recruitment").map_or_else(default_recruitment_path, PathBuf::from);
    let recruitment = Recruitment::load(&rec_path).map_err(|e| StartError::Fatal(format!("failed to load recruitment: {e}")))?;
    let link = LinkConditions {
        lag: Duration::from_millis(args.try_get("lag-ms", 0)?),
        jitter: Duration::from_millis(args.try_get("jitter-ms", 0)?),
        loss: args.try_get("loss", 0.0)?,
    };
    if !(0.0..=1.0).contains(&link.loss) {
        return Err(StartError::Usage("--loss must be between 0 and 1".into()));
    }
    let max_players = args.try_get("max-players", 256)?;
    if !(1..=MAX_PLAYERS_LIMIT).contains(&max_players) {
        return Err(StartError::Usage(format!("--max-players must be 1..={MAX_PLAYERS_LIMIT}")));
    }
    let time = |key: &str, default: u32| match args.str(key) {
        None => Ok(default),
        Some(s) => parse_time(s).ok_or_else(|| StartError::Usage(format!("invalid --{key} {s} (expected hh:mm)"))),
    };
    let weather = match args.str("weather") {
        None => None,
        Some(w) => Some(
            game::weather::parse(w)
                .ok_or_else(|| StartError::Usage(format!("invalid --weather {w} (sun, clouds, rain, storm, fog)")))?,
        ),
    };
    let bind = match args.str("bind") {
        Some(_) => args.try_get("bind", dual_stack())?,
        // No IPv6 on this host: plain IPv4.
        None if game::net::bind_udp(dual_stack()).is_err() => SocketAddr::from((Ipv4Addr::UNSPECIFIED, PORT)),
        None => dual_stack(),
    };
    let cleaning_fixed = args.str("cleaning-at").is_some();
    let save_path = if args.flag("no-save") { None } else { Some(args.str("save").map_or_else(|| PathBuf::from("saves/world.db"), PathBuf::from)) };
    let cfg = Config {
        bind,
        link,
        max_players,
        stats_every: Duration::from_secs(args.try_get("stats-secs", 5)?.max(1)),
        client_timeout: DEFAULT_CLIENT_TIMEOUT,
        start_access: if args.flag("start-with-card") { game::map::access::CARD } else { 0 },
        recruitment,
        skip_recruitment: args.flag("skip-recruitment") || args.flag("start-employed"),
        start_employed: args.flag("start-employed"),
        start_cigarettes: args.flag("start-cigarettes"),
        needs_speed: args.try_get("needs-speed", 1)?,
        start_minute: time("start-time", 8 * 60)?,
        time_scale: args.try_get("time-scale", 1)?,
        treats_now: args.flag("treats"),
        stale_fruit_percent: args.try_get("stale-fruit", game::treats::STALE_FRUIT_PERCENT)?,
        cleaning_at: time("cleaning-at", game::cleaning::ROUND_AT)?,
        cleaning_spread: if cleaning_fixed { 0 } else { game::cleaning::ROUND_SPREAD },
        weather,
        save_path: save_path.clone(),
        allow_guests: args.flag("allow-guests") || save_path.is_none(),
    };
    let crc = building.crc;
    let floors = building.active_floors().count();
    let mut server = Server::new(building, cfg).map_err(|e| StartError::Fatal(format!("failed to start: {e}")))?;
    // Accounts: the login API over HTTPS next to the game port.
    if let Some(path) = &save_path {
        let auth = Auth::open(path).map_err(StartError::Fatal)?;
        let tls = match (args.str("tls-cert"), args.str("tls-key")) {
            (Some(c), Some(k)) => Tls { cert: c.into(), key: k.into(), self_signed: false },
            (None, None) => http::self_signed(&path.parent().unwrap_or(std::path::Path::new(".")).join("tls")).map_err(StartError::Fatal)?,
            _ => return Err(StartError::Usage("--tls-cert and --tls-key go together".into())),
        };
        let game = server.local_addr();
        let auth_bind = args.try_get("auth-bind", SocketAddr::new(bind.ip(), game.port().wrapping_add(1)))?;
        http::spawn(auth.clone(), auth_bind, tls).map_err(StartError::Fatal)?;
        server.set_auth(auth);
        println!("login API (HTTPS) on {auth_bind}");
    }
    println!(
        "server listening on {} | tick {} Hz | building crc {:08x}, {} active floors ({}) | link: lag {:?} jitter {:?} loss {:.1}%",
        server.local_addr(),
        TICK_HZ,
        crc,
        floors,
        map_path.display(),
        link.lag,
        link.jitter,
        link.loss * 100.0
    );
    Ok(server)
}

/// `[::]:7777`: IPv6 and (dual-stack) IPv4.
fn dual_stack() -> SocketAddr {
    SocketAddr::from((Ipv6Addr::UNSPECIFIED, PORT))
}

/// "8:30" -> minutes since midnight.
fn parse_time(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':').unwrap_or((s, "0"));
    let (h, m) = (h.trim().parse::<u32>().ok()?, m.trim().parse::<u32>().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}
