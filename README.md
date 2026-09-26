# Startup Sim

2D multiplayer „symulator pracy w startupie IT” — serwer Rust + klient Godot 4.
Etap 1: fundament sieci. Dokumentacja: [GDD](docs/GDD.md) ·
[Protokół](docs/PROTOCOL.md) · [Architektura](docs/ARCHITECTURE.md).

## Uruchomienie

Wymagania: Rust (rustup), Godot 4.7 (`godot` w PATH).

```bash
cd server && cargo run --release            # serwer na [::]:7777 (IPv4 + IPv6)
godot --path client                         # klient (można odpalić kilka razy)
cd server && cargo run --release -- --start-with-card   # wariant dla botów: wszyscy mają kartę
cd server && cargo run --release --bin bots -- --count 50 --room "Chill room" --all-in-room
```

Sterowanie: WASD / strzałki, **E** — rozmowa z NPC / winda (stojąc w kabinie) / ekspres do kawy / podniesienie przedmiotu, **1–3** — wyjmij / schowaj przedmiot z kieszeni, **Q** — upuść, **G** — podaj osobie obok, **F** — użyj (wypij kawę, pokaż kartę), **F3** — overlay debug.
Na starcie tworzysz postać (imię, płeć, wiek, miejscowość, e-mail postaci,
wygląd z podglądem; zapamiętywana lokalnie). Po połączeniu siedzisz w domu przy komputerze: w **przeglądarce** jest portal z
ogłoszeniami (kilka firm; zatrudnia tylko nasz startup), wypełniasz formularz,
po chwili w **Poczcie** czeka zaproszenie na **rozmowę online** (3 pytania,
2 poprawne = przyjęcie), a potem zaproszenie na dzień próbny — „Idę do biura”.
Na miejscu startujesz przed budynkiem bez przepustki: bramki w holu go nie wpuszczą,
więc trzeba podejść do portierni i porozmawiać z portierem (E) — da przepustkę
gościa i zaprowadzi na recepcję na piętrze 1 (schodami). Recepcja (E) zaprowadzi
do HR, a HR (E) podpisze umowę i wyda kartę pracownika.

### Serwer — opcje
`--bind`, `--map`, `--max-players`, `--stats-secs`, `--start-with-card` (każdy gracz z kartą — do testów z botami),
`--skip-recruitment` (bez portalu, od razu do świata), `--recruitment <plik>` (oferty i pytania,
domyślnie `server/data/recruitment.json`), oraz symulacja sieci:
`--lag-ms <ms>` (opóźnienie w jedną stronę, RTT rośnie 2×), `--jitter-ms <ms>`, `--loss <0..1>`.
Np. RTT ~100 ms i 2% strat: `cargo run --release -- --lag-ms 50 --jitter-ms 10 --loss 0.02`.

### Klient — argumenty deweloperskie (po `--`)
`--nick=Ala --server=127.0.0.1:7777 --autoconnect --debug`, `--auto-recruit=1 [--auto-recruit-delay=2]`
(sam aplikuje na ofertę 1 i zgaduje odpowiedzi do skutku), (adres może być też IPv6: `--server=[::1]:7777`) (F3 od startu),
`--autowalk` (losowy ruch), `--goto="27,29;E;wait:2;34,6;Recepcja"` (kolejne
kroki: kafel / pokój na bieżącym piętrze, `E` = wciśnij E, `wait:N` = czekaj —
tu: rozmowa z portierem, potem schodami do recepcji),
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
godot --headless --path client -s tests/render_maps.gd -- /tmp   # grafika pięter do PNG
python3 tools/build_maps.py             # zapis JSON-ów
cd server && UPDATE_GOLDEN=1 cargo test --test golden   # nowe wektory testowe
```

