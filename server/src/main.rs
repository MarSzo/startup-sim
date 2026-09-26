use std::path::PathBuf;
use std::time::Duration;

use game::args::Args;
use game::map::{default_map_path, Map};
use game::net::LinkConditions;
use game::server::{Config, Server, DEFAULT_CLIENT_TIMEOUT, TICK_HZ};

const HELP: &str = "\
Startup sim - dedicated game server

USAGE: cargo run [--release] -- [OPTIONS]

OPTIONS:
  --bind <addr>         listen address        [default: [::]:7777, dual-stack IPv6+IPv4;
                                               falls back to 0.0.0.0:7777 without IPv6]
  --map <path>          map JSON              [default: ../client/maps/floor0.json]
  --max-players <n>                           [default: 256]
  --stats-secs <n>      stats log interval    [default: 5]
  --lag-ms <ms>         simulated one-way delay, each direction (RTT += 2x)
  --jitter-ms <ms>      simulated extra random delay 0..=ms
  --loss <p>            simulated packet loss per direction, e.g. 0.02
";

fn main() {
    let args = Args::from_env();
    if args.flag("help") {
        print!("{HELP}");
        return;
    }
    let map_path = args.str("map").map(PathBuf::from).unwrap_or_else(default_map_path);
    let map = Map::load(&map_path).unwrap_or_else(|e| {
        eprintln!("failed to load map: {e}");
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
    };
    let mut cfg = cfg;
    if args.str("bind").is_none() && game::net::bind_udp(cfg.bind).is_err() {
        // No IPv6 on this host: plain IPv4.
        cfg.bind = "0.0.0.0:7777".parse().unwrap();
    }
    let mut server = match Server::new(map, cfg) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to bind: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "server listening on {} | tick {} Hz | map '{}' crc {:08x} ({}) | link: lag {:?} jitter {:?} loss {:.1}%",
        server.local_addr(),
        TICK_HZ,
        map_path.file_name().unwrap().to_string_lossy(),
        crc(&map_path),
        map_path.display(),
        link.lag,
        link.jitter,
        link.loss * 100.0
    );
    server.run();
}

fn crc(path: &std::path::Path) -> u32 {
    crc32fast::hash(&std::fs::read(path).unwrap_or_default())
}
