# Startup Sim

2D multiplayer „symulator pracy w startupie IT” — serwer Rust + klient Godot 4.
Etap 1: fundament sieci. Dokumentacja: [GDD](docs/GDD.md) ·
[Protokół](docs/PROTOCOL.md) · [Architektura](docs/ARCHITECTURE.md).

## Uruchomienie

Wymagania: Rust (rustup), Godot 4.7 (`godot` w PATH).

```bash
cd server && cargo run --release            # serwer na 0.0.0.0:7777
godot --path client                         # klient (można odpalić kilka razy)
cd server && cargo run --release --bin bots -- --count 50 --room "Open space" --all-in-room
```

Sterowanie: WASD / strzałki, **F3** — overlay debug.

### Serwer — opcje
`--bind`, `--map`, `--max-players`, `--stats-secs`, oraz symulacja sieci:
`--lag-ms <ms>` (opóźnienie w jedną stronę, RTT rośnie 2×), `--jitter-ms <ms>`, `--loss <0..1>`.
Np. RTT ~100 ms i 2% strat: `cargo run --release -- --lag-ms 50 --jitter-ms 10 --loss 0.02`.

### Klient — argumenty deweloperskie (po `--`)
`--nick=Ala --server=127.0.0.1:7777 --autoconnect --debug` (F3 od startu),
`--autowalk` (losowy ruch), `--goto="Open space"` (idzie do pokoju),
`--screenshot=/tmp/x.png --screenshot-delay=5` (zapis klatki i wyjście).

## Testy

```bash
cd server && cargo test
godot --headless --path client -s tests/run_tests.gd
```

Pliki golden (`server/tests/golden/`) pilnują, że protokół i ruch są identyczne
w Rust i GDScript. Po celowej zmianie: `UPDATE_GOLDEN=1 cargo test --test golden`.
