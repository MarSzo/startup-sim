# Architektura

Dedykowany, autorytatywny serwer w Rust + klient Godot 4 (GDScript) po UDP,
z własnym binarnym protokołem (`docs/PROTOCOL.md`).

```
┌───────────── client (Godot) ─────────────┐        UDP        ┌──────────── server (Rust) ────────────┐
│ main.gd        start screen <-> gra      │                   │ main.rs     CLI, start                │
│ net_client.gd  handshake, ping, timeout  │ ── Input 60 Hz ─▶ │ server.rs   pętla 20 Hz, gracze,      │
│ game.gd        predykcja + rekoncyliacja │                   │             interest mgmt, snapshoty  │
│                interpolacja innych       │ ◀─ Snapshot 20 Hz │ net.rs      socket + symulator laga    │
│ movement.gd ═══════ identyczny algorytm ══════════════════════ sim.rs      ruch i kolizje (int)      │
│ protocol.gd ═══════ identyczny format ════════════════════════ protocol.rs kodowanie pakietów        │
│ map_data.gd ═══╗                         │                   │ map.rs ═══╗ mapa, pokoje, BFS         │
└────────────────║─────────────────────────┘                   └───────────║───────────────────────────┘
                 ╚═════════════ client/maps/floor0.json (JEDEN plik) ══════╝
```

## Repozytorium

```
server/                 crate Rusta (lib `game` + binarki)
  src/lib.rs            moduły współdzielone przez serwer i boty
  src/map.rs            wczytywanie mapy, kolizje kafli, pokoje, BFS
  src/sim.rs            deterministyczny krok ruchu (liczby całkowite)
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
  maps/floor0.json      mapa parteru (czytana też przez serwer)
  main.gd / main.tscn   wejście: start screen <-> gra, argumenty dev
  net/protocol.gd       lustro protocol.rs
  net/net_client.gd     połączenie UDP (PacketPeerUDP)
  sim/movement.gd       lustro sim.rs
  map/map_data.gd       lustro map.rs (bez BFS)
  map/map_view.gd       render mapy do jednej tekstury
  game/game.gd          logika sieciowa gry po stronie klienta
  game/player_view.gd   placeholder postaci + nick
  game/remote_player.gd bufor snapshotów + interpolacja
  ui/start_screen.gd    ekran startowy
  ui/debug_overlay.gd   F3
  tests/run_tests.gd    testy headless (parytet z Rustem)
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
   (średnio 3 = 60 Hz / 20 Hz); aktualizacja pokoju i flag.
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

Ten sam algorytm jest w `sim.rs` i `movement.gd`; GDScript liczy na 64-bit
int, więc wyniki są bit w bit równe. `tests/golden/movement_vectors.json`
(8 przypadków × 400 kroków przy ścianach, meblach, drzwiach) jest generowany
przez Rust i odtwarzany w Godocie.

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

**Rekoncyliacja**: z każdym nowym tickiem klient bierze pozycję serwera,
usuwa inputy `≤ last_input_seq` i odtwarza pozostałe. Jeśli wynik różni się
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

**Render**: mapa rysowana raz do `ImageTexture` (jeden sprite), postacie to
`Node2D._draw()` z `Label`em (y-sort). Kamera `Camera2D` z zoomem 3×, bez
wygładzania, z limitami mapy. Etykiety mają skalę `1/zoom` i rozmiar czcionki
ekranowej, więc są ostre mimo zoomu.

**F3** (`debug_overlay.gd`): FPS, ping, tick serwera i czas renderu, pokój,
widoczni gracze, id/kafel, inputy w locie, liczba korekt, procent klatek z
pustym buforem interpolacji, transfer.

## Mapa

`client/maps/floor0.json` — jedyne źródło. Serwer czyta go ścieżką
`../client/maps/floor0.json` względem crate'a (lub `--map`), klient przez
`res://maps/floor0.json`. `Welcome` niesie CRC32 pliku; klient odrzuca
niezgodną wersję.

Format:
- `tiles`: wiersze znaków; `legend` mapuje znak → `{type, solid, color}`.
- `rooms`: druga warstwa znaków tej samej wielkości; `room_defs` mapuje znak →
  `{id, name, type}`, `-` = brak pokoju (ściany).
- `doors`: lista kafli drzwi z polem `access` (na przyszłe karty dostępu).
- `spawns`: kafle startowe.

Kafle drzwi należą do pokoju po stronie „publicznej” (korytarz / wejście), więc
stojąc w drzwiach widzisz korytarz.

## Gotowość na rozbudowę (nie zaimplementowane)

| funkcja | gdzie się wepnie |
|---------|------------------|
| **Piętra / winda** | `floor` jest w `Map`, stanie gracza i snapshocie; interest mgmt grupuje po `(floor, room)`. Kolejne piętra = kolejne pliki map + `MapSet` indeksowany piętrem; winda = strefa, która zmienia `floor` i pozycję. |
| **Karty dostępu** | `Map::is_blocked` to jedyne miejsce decyzji o kolizji; drzwi mają pole `access`. Dojdzie kontekst gracza (posiadane karty) i ta sama logika w `movement.gd`. |
| **NPC** | `kind` w encji snapshotu, BFS `Map::find_path` (używany już przez boty). NPC to encje serwera bez adresu sieciowego, symulowane w tym samym ticku. |
| **Akcje / interakcje** | bity 4–7 inputu (zarezerwowane) lub nowe typy pakietów; kolejność zapewnia `seq`. |
| **Więcej graczy w pokoju** | fragmentacja snapshotów już działa; następny krok to delta względem `ack_tick` i/lub priorytet po odległości. |

## Boty (`cargo run --release --bin bots`)

Jeden wątek, N gniazd nieblokujących, pętla 60 Hz. Każdy bot przechodzi pełny
handshake, predykuje ruch tym samym `sim::step` i robi rekoncyliację (log
pokazuje liczbę błędnych predykcji), chodzi po ścieżkach BFS do losowych
kafli — `--room-share` z nich wybiera cele tylko w `--room`. Log co 5 s:
połączeni, liczba w docelowym pokoju, RTT, odbierany transfer, widoczni.

## Testy

| polecenie | co sprawdza |
|-----------|-------------|
| `cd server && cargo test` | 19 testów jednostkowych (mapa, ruch/kolizje, protokół: round-trip, nagłówek, śmieciowe i ucięte pakiety, fragmentacja ≤ 1200 B), golden, e2e (handshake, widoczność po pokojach, timeout, odrzucenia) |
| `godot --headless --path client -s tests/run_tests.gd` | parytet protokołu (bajt w bajt) i ruchu (3200 kroków) z Rustem, zgodność CRC mapy |
