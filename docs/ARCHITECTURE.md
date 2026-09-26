# Architektura

Dedykowany, autorytatywny serwer w Rust + klient Godot 4 (GDScript) po UDP,
z własnym binarnym protokołem (`docs/PROTOCOL.md`).

```
┌───────────── client (Godot) ─────────────┐        UDP        ┌──────────── server (Rust) ────────────┐
│ main.gd        start screen <-> gra      │                   │ main.rs     CLI, start                │
│ net_client.gd  handshake, ping, timeout  │ ── Input 60 Hz ─▶ │ server.rs   pętla 20 Hz, gracze,      │
│ game.gd        predykcja + rekoncyliacja │                   │             interest mgmt, snapshoty  │
│                interpolacja innych       │ ◀─ Snapshot 20 Hz │ net.rs      socket + symulator laga    │
│ movement.gd ═══════ identyczny algorytm ══════════════════════ sim.rs      ruch, kolizje, piętra     │
│ protocol.gd ═══════ identyczny format ════════════════════════ protocol.rs kodowanie pakietów        │
│ building.gd ═══╗                         │                   │ building.rs ═╗ piętra, BFS po budynku │
│ map_data.gd    ║                         │                   │ map.rs       ║ piętro, pokoje, linki   │
└────────────────║─────────────────────────┘                   └──────────────║────────────────────────┘
                 ╚══════ client/maps/building.json + floorN.json (JEDNO źródło) ╝
```

## Repozytorium

```
server/                 crate Rusta (lib `game` + binarki)
  src/lib.rs            moduły współdzielone przez serwer i boty
  src/building.rs       budynek: lista pięter, CRC, winda, BFS między piętrami
  src/map.rs            jedno piętro: kafle, kolizje, pokoje, linki (schody/winda)
  src/sim.rs            deterministyczny krok: ruch, kolizje, schody, winda
  src/nav.rs            podążanie ścieżką (boty; później NPC)
  src/protocol.rs       pakiety: encode/decode, fragmentacja snapshotów
  src/net.rs            UdpSocket + symulator opóźnienia/jittera/strat
  src/server.rs         pętla ticka, handshake, inputy, interest mgmt, statystyki
  src/args.rs           minimalny parser argumentów CLI
  src/main.rs           binarka `server` (domyślna dla `cargo run`)
  src/bin/bots.rs       binarka `bots` — test obciążeniowy
  tests/golden.rs       generuje/sprawdza pliki golden (parytet z GDScriptem)
  tests/server_e2e.rs   prawdziwy serwer na losowym porcie + surowe klienty UDP
  tests/golden/         packets.json, movement_vectors.json
client/                 projekt Godota 4.7
  maps/building.json    lista pięter (piętro 2 zablokowane)
  maps/floor0.json      parter + teren zewnętrzny
  maps/floor1.json      piętro 1
  main.gd / main.tscn   wejście: start screen <-> gra, argumenty dev
  net/protocol.gd       lustro protocol.rs
  net/net_client.gd     połączenie UDP (PacketPeerUDP)
  sim/movement.gd       lustro sim.rs
  map/building.gd       lustro building.rs (bez BFS)
  map/map_data.gd       lustro map.rs (bez BFS)
  map/map_view.gd       render mapy do jednej tekstury
  game/game.gd          logika sieciowa gry po stronie klienta
  game/player_view.gd   placeholder postaci + nick
  game/remote_player.gd bufor snapshotów + interpolacja
  ui/start_screen.gd    ekran startowy
  ui/debug_overlay.gd   F3
  tests/run_tests.gd    testy headless (parytet z Rustem)
tools/build_maps.py     generator map z czytelnego opisu (wynik = JSON-y wyżej)
docs/                   GDD, PROTOCOL, ARCHITECTURE
```

## Serwer

**Jeden wątek, bez async.** `std::net::UdpSocket` z timeoutem odczytu, domyślnie
na `[::]:7777` w trybie dual-stack (IPv4 + IPv6; gniazdo tworzone przez `socket2`,
bo std nie pozwala wyłączyć `IPV6_V6ONLY`). Przy
dziesiątkach–setkach graczy koszt ticka to ~0,2–1,5 ms (głównie `sendto`), więc
tokio nic by nie dało, a pętla jest deterministyczna i łatwa w debugowaniu.

**Pętla** (`Server::run`):
1. Jeśli minął termin ticka → `tick()`; termin liczony jako `t0 + n·50 ms`
   (bez dryfu). Jeśli spóźnienie > 1 tick, pominięte ticki są liczone jako
   `missed` i przeskakiwane (brak spirali nadrabiania).
2. Wysłanie pakietów, którym minął symulowany lag; obsługa odebranych.
3. `recv` z timeoutem do najbliższego terminu (tick / statystyki / zwolnienie
   pakietu z symulatora).

Pakiety są obsługiwane od razu po odebraniu: `Input` trafia do kolejki gracza,
`Ping` dostaje `Pong` natychmiast (dokładny RTT), `Connect` tworzy gracza.

**Tick** (`Server::tick`):
1. Timeout: gracze bez pakietów > 5 s → `Disconnect(timeout)` i usunięcie.
2. Symulacja: z kolejki inputów każdego gracza max 6 kroków `sim::step`
   (średnio 3 = 60 Hz / 20 Hz) na jego `Body` (piętro, pozycja, poprzedni
   input, blokada schodów); aktualizacja pokoju i flag.
3. Grupowanie encji po `(floor, room)`.
4. Dla każdego gracza snapshot z encji jego grupy (fragmentowany ≤ 1200 B)
   + `PlayerInfo` dla encji, których nicku jeszcze nie dostał.

**Statystyki** co 5 s (`--stats-secs`): liczba ticków i `missed`, średni i
maksymalny czas ticka, gracze, max widocznych, transfer na klienta
(średnia/min/max), pakiety/s wejście/wyjście, pakiety wycięte przez symulator.

**Sesje** są indeksowane tokenem (`by_token`), nie adresem: pakiet z ważnym
tokenem z nowego adresu przenosi sesję (`migrate`), jeśli dowodzi „świeżości”
(`Ping` albo nowe inputy). Nieznany token dostaje `Disconnect(4)`. `by_addr`
służy już tylko do deduplikacji powtórzonych `Connect`. Szczegóły:
`docs/PROTOCOL.md` („Sesja, zmiana adresu i ponowne łączenie”).

**Symulator sieci** (`net.rs`): `--lag-ms` (opóźnienie w jedną stronę, dla
obu kierunków — RTT rośnie o 2×), `--jitter-ms` (losowe 0..=j, może zmienić
kolejność jak prawdziwe UDP), `--loss` (prawdopodobieństwo zgubienia, osobno
w każdą stronę). Działa na pakietach przychodzących i wychodzących.

## Deterministyczny ruch

Pozycje w liczbach całkowitych (1 px = 16 j.). Krok wejścia 1/60 s:
prosto 24 j. (90 px/s), po skosie 17 j. na oś. Kolizja: pudełko 10×8 px vs
siatka kafli, najpierw oś X, potem Y (ślizganie po ścianach), docinanie do
krawędzi kafla. Kafle poza mapą są blokujące. Krok (24) < kafel (256), więc
nie ma tunelowania.

**Stan postaci** (`sim::Body`, w GDScript słownik z `Movement.body()`):
piętro, pozycja, poprzedni input i blokada schodów. Wszystko, od czego zależy
krok, jest w tym stanie i idzie w snapshocie do właściciela — dzięki temu
rekoncyliacja odtwarza inputy od dokładnie tego samego stanu.

**Przejścia między piętrami** (część kroku, więc przewidywane przez klienta):
- **Schody**: wejście środkiem postaci na kafel schodów (`links` typu `stairs`)
  przenosi na kafel przyjścia na drugim piętrze. Zaraz potem działa blokada:
  schody nie zadziałają, dopóki nie zmienisz klawiszy ruchu *i* nie zejdziesz
  z obszaru schodów — przytrzymanie „w górę” po przyjściu nie cofa na dół.
- **Winda**: w kabinie (`links` typu `elevator`, ta sama pozycja na każdym
  piętrze) wciśnięcie E (zbocze: nie wciśnięte w poprzednim kroku) przenosi
  na następne aktywne piętro z kabiną o tym samym `id`. Zablokowane piętro 2
  jest pomijane.

Ten sam algorytm jest w `sim.rs` i `movement.gd`; GDScript liczy na 64-bit
int, więc wyniki są bit w bit równe. `tests/golden/movement_vectors.json`
(10 losowych przejść × 400 kroków przy ścianach, meblach, drzwiach, bramkach,
schodach i w windzie + scenariusz: spawn → schody → Chill room → winda w dół
i w górę) jest generowany przez Rust i odtwarzany w Godocie.

## Klient

**Połączenie** (`net_client.gd`): parsowanie adresów z IPv6, rozwiązywanie
nazw `TYPE_ANY`, nowe gniazdo po 1,5 s ciszy lub powrocie z tła (ta sama
sesja), automatyczne ponowne łączenie przez 30 s po utracie sesji — `main.gd`
zostawia wtedy scenę gry, a `game.gd` zamraża się (`on_reconnecting`) i
czyści stan po nowym `Welcome` (`reset_session`).

**Predykcja własnej postaci** (`game.gd`): w `_physics_process` (60 Hz)
klient próbkuje klawisze, nadaje inputowi `seq`, od razu liczy nową pozycję
(`movement.gd`) i wysyła `Input` z 4 ostatnimi inputami (redundancja).
Render interpoluje między dwoma ostatnimi krokami fizyki
(`Engine.get_physics_interpolation_fraction()`), więc ruch jest płynny przy
dowolnym odświeżaniu monitora.

**Rekoncyliacja**: z każdym nowym tickiem klient bierze stan serwera
(`floor`, pozycja, `self_lock`, `self_prev_input`), usuwa inputy
`≤ last_input_seq` i odtwarza pozostałe. Zmiana piętra w wyniku korekty
przełącza widok bez wygładzania. Jeśli wynik różni się
od predykcji (np. zgubione 4+ pakiety inputu z rzędu), różnica trafia do
`error_offset`, który wygasa wykładniczo (~70 ms) — bez teleportów. Przy
jednakowej symulacji po obu stronach korekt praktycznie nie ma (0 w testach).

**Interpolacja innych graczy** (`remote_player.gd`): próbki `(tick, pozycja)`
w buforze; render w czasie `est_tick − 2 ticki` (100 ms). `est_tick` rośnie z
zegarem lokalnym i jest łagodnie (10%/snapshot) korygowany do ticku ostatnio
odebranego snapshotu; przy dużym rozjeździe (>5 ticków) jest przestawiany.
Gdy brakuje danych, krótka ekstrapolacja (max 2 ticki), potem stop.

**Widoczność**: przy zmianie `(floor, room)` klient usuwa wszystkich
zdalnych graczy — nowy zestaw przychodzi w tym samym snapshocie. Gracz
nieobecny w snapshotach przez 5 ticków znika.

**Render**: każde piętro rysowane raz do `ImageTexture` (jeden sprite na
piętro, widoczne tylko bieżące), postacie to
`Node2D._draw()` z `Label`em (y-sort). Kamera `Camera2D` z zoomem 3×, bez
wygładzania, z limitami mapy. Etykiety mają skalę `1/zoom` i rozmiar czcionki
ekranowej, więc są ostre mimo zoomu.

**Podpowiedź** „[E] Winda: jedź na …” pojawia się w kabinie windy.

**F3** (`debug_overlay.gd`): FPS, ping, tick serwera i czas renderu, piętro i pokój,
widoczni gracze, id/kafel, inputy w locie, liczba korekt, procent klatek z
pustym buforem interpolacji, transfer.

## Mapa

`client/maps/building.json` wymienia piętra (`floor`, `file`, `name`,
`locked`); każde piętro ma swój `floorN.json`. To jedyne źródło: serwer czyta
je ścieżką `../client/maps/building.json` względem crate'a (lub `--map`),
klient przez `res://maps/`. `Welcome` niesie CRC32 całego budynku; klient
odrzuca niezgodną wersję. Pliki generuje `tools/build_maps.py` (edytuj
generator, nie JSON-y ręcznie), który też sprawdza, czy drzwi gdzieś prowadzą.

Format piętra:
- `tiles`: wiersze znaków; `legend` mapuje znak → `{type, solid, color, access?}`.
  `access` („card” dla bramek, „service” dla zaplecza) jest na razie tylko
  informacją — o kolizji decyduje `solid` (bramki otwarte, zaplecze zamknięte).
- `rooms`: druga warstwa znaków tej samej wielkości; `room_defs` mapuje znak →
  `{id, name, type}`, `-` = brak pokoju (ściany). Id są unikalne w obrębie piętra.
- `links`: `{kind: "stairs", area: [x,y,w,h], to_floor, to: [x,y]}` albo
  `{kind: "elevator", id, area}`.
- `spawns`: kafle startowe (tylko parter: chodnik przed wejściem).

Kafle drzwi należą do pokoju po stronie „publicznej” (korytarz / hol), więc
stojąc w drzwiach widzisz korytarz.

## Gotowość na rozbudowę (nie zaimplementowane)

| funkcja | gdzie się wepnie |
|---------|------------------|
| **Piętro 2** | wpis w `building.json` z `locked: true`; odblokowanie = plik mapy + `locked: false` (winda i schody same go obsłużą; do ustalenia: odblokowanie w trakcie gry wymaga zmiany CRC albo osobnego komunikatu). |
| **Karty dostępu** | `Map::is_blocked` to jedyne miejsce decyzji o kolizji, a kafle mają już `access`. Dojdzie kontekst gracza (posiadane karty) w `Body`, ta sama logika w `movement.gd`. |
| **NPC** | `kind` w encji snapshotu, `nav::Walker` + `Building::find_path` (używane już przez boty, także między piętrami). NPC to encje serwera bez adresu sieciowego, symulowane w tym samym ticku. |
| **Akcje / interakcje** | bit 16 (E) działa jak w windzie: kontekst = link/kafel, na którym stoisz; bity 5–7 wolne. |
| **Więcej graczy w pokoju** | fragmentacja snapshotów już działa; następny krok to delta względem `ack_tick` i/lub priorytet po odległości. |

## Boty (`cargo run --release --bin bots`)

Jeden wątek, N gniazd nieblokujących, pętla 60 Hz. Każdy bot przechodzi pełny
handshake, predykuje ruch tym samym `sim::step` i robi rekoncyliację (log
pokazuje liczbę błędnych predykcji), chodzi (`nav::Walker`) po ścieżkach BFS
przez cały budynek, schodami między piętrami — `--room-share` z nich wybiera
cele tylko w `--room` (domyślnie „Chill room” na piętrze 1). Log co 5 s:
połączeni, liczba w docelowym pokoju, RTT, odbierany transfer, widoczni.

## Testy

| polecenie | co sprawdza |
|-----------|-------------|
| `cd server && cargo test` | 28 testów jednostkowych (budynek i mapy wg GDD, osiągalność pokoi, ruch/kolizje, schody bez odbijania, winda na wciśnięcie, nawigacja, protokół), 2 golden, 6 e2e (handshake, widoczność po pokojach i piętrach, stan serwera = predykcja po schodach, timeout, odrzucenia, IPv4+IPv6, migracja adresu, nieznany token) |
| `godot --headless --path client -s tests/run_tests.gd` | parytet protokołu (bajt w bajt) i ruchu z przejściami między piętrami z Rustem, zgodność CRC budynku, parsowanie adresów |
