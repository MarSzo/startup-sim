use std::path::PathBuf;
use std::time::Duration;

use game::args::Args;
use game::building::{default_building_path, Building};
use game::recruitment::{default_recruitment_path, Recruitment};
use game::net::LinkConditions;
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
  --recruitment <path>  recruitment JSON  [default: data/recruitment.json]
";

fn main() {
    let args = Args::from_env();
    if args.flag("help") {
        print!("{HELP}");
        return;
    }
    let map_path = args.str("map").map(PathBuf::from).unwrap_or_else(default_building_path);
    let building = Building::load(&map_path).unwrap_or_else(|e| {
        eprintln!("failed to load map: {e}");
        std::process::exit(1);
    });
    let rec_path = args.str("recruitment").map(PathBuf::from).unwrap_or_else(default_recruitment_path);
    let recruitment = Recruitment::load(&rec_path).unwrap_or_else(|e| {
        eprintln!("failed to load recruitment: {e}");
        std::process::exit(1);
    });
    let link = LinkConditions {
        lag: Duration::from_millis(args.get("lag-ms", 0)),
        jitter: Duration::from_millis(args.get("jitter-ms", 0)),
        loss: args.get("loss", 0.0),
    };
    let cfg = Config {
        bind: args.get("bind", "[::]:7777".parse().unwrap()),
        link,
        max_players: args.get("max-players", 256),
        stats_every: Duration::from_secs(args.get("stats-secs", 5)),
        client_timeout: DEFAULT_CLIENT_TIMEOUT,
        start_access: if args.flag("start-with-card") { game::map::access::CARD } else { 0 },
        recruitment,
        skip_recruitment: args.flag("skip-recruitment"),
    };
    let mut cfg = cfg;
    if args.str("bind").is_none() && game::net::bind_udp(cfg.bind).is_err() {
        // No IPv6 on this host: plain IPv4.
        cfg.bind = "0.0.0.0:7777".parse().unwrap();
    }
    let crc = building.crc;
    let floors = building.active_floors().count();
    let mut server = match Server::new(building, cfg) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to bind: {e}");
            std::process::exit(1);
        }
    };
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
    server.run();
}
