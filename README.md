# Startup Sim

2D multiplayer „symulator pracy w startupie IT” — serwer Rust + klient Godot 4.
Etap 1: fundament sieci. Dokumentacja: [GDD](docs/GDD.md) ·
[Protokół](docs/PROTOCOL.md) · [Architektura](docs/ARCHITECTURE.md).

## Uruchomienie

Wymagania: Rust (rustup), Godot 4.7 (`godot` w PATH).

```bash
cd server && cargo run --release            # serwer na [::]:7777 (IPv4 + IPv6)
godot --path client                         # klient (można odpalić kilka razy)
cd server && cargo run --release --bin bots -- --count 50 --room "Chill room" --all-in-room
```

Sterowanie: WASD / strzałki, **E** — winda (stojąc w kabinie), **F3** — overlay debug.
Gracz startuje przed budynkiem; na piętro 1 wchodzi się schodami lub windą.

### Serwer — opcje
`--bind`, `--map`, `--max-players`, `--stats-secs`, oraz symulacja sieci:
`--lag-ms <ms>` (opóźnienie w jedną stronę, RTT rośnie 2×), `--jitter-ms <ms>`, `--loss <0..1>`.
Np. RTT ~100 ms i 2% strat: `cargo run --release -- --lag-ms 50 --jitter-ms 10 --loss 0.02`.

### Klient — argumenty deweloperskie (po `--`)
`--nick=Ala --server=127.0.0.1:7777 --autoconnect --debug` (adres może być też IPv6: `--server=[::1]:7777`) (F3 od startu),
`--autowalk` (losowy ruch), `--goto="34,6;Recepcja"` (idzie po kolei do kafla /
pokoju na bieżącym piętrze — tu: schodami na górę i do recepcji),
`--screenshot=/tmp/x.png --screenshot-delay=5` (zapis klatki i wyjście).

## Testy

```bash
cd server && cargo test
godot --headless --path client -s tests/run_tests.gd
```

Pliki golden (`server/tests/golden/`) pilnują, że protokół i ruch są identyczne
w Rust i GDScript. Po celowej zmianie: `UPDATE_GOLDEN=1 cargo test --test golden`.

## Mapy

Budynek jest w `client/maps/` (`building.json` + `floorN.json`) — to jedno
źródło dla serwera i klienta. Układ zmieniaj w generatorze, nie w JSON-ach:

```bash
python3 tools/build_maps.py --preview   # podgląd ASCII
python3 tools/build_maps.py             # zapis JSON-ów
cd server && UPDATE_GOLDEN=1 cargo test --test golden   # nowe wektory testowe
```

