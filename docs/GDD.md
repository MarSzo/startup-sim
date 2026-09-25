# GDD — „symulator pracy w startupie IT”

2D multiplayer, widok z góry, pikselowa grafika w stylu The Escapists.

> **Uwaga:** pełne ustalenia projektowe (rozgrywka, rekrutacja, sklep, palenie,
> NPC, zadania, piętra, winda, karty dostępu) nie zostały jeszcze spisane w tym
> pliku — do uzupełnienia. Poniżej jest to, co wiemy na etapie 1.

## Stack

- Klient: Godot 4 (4.7), GDScript, aplikacja desktopowa.
- Serwer: dedykowany, autorytatywny, Rust (`std::net::UdpSocket`, jeden wątek).
- Transport: UDP, własny binarny protokół (`docs/PROTOCOL.md`).

## Etap 1 — pionowy wycinek sieci

Fundament multiplayera: wielu graczy chodzi po wspólnej mapie parteru.
Bez rekrutacji, sklepu, palenia, NPC i zadań.

Zakres:
1. Serwer: stały tick 20 Hz; handshake connect → player_id → spawn; timeout
   5 s; klient wysyła tylko inputy, serwer liczy pozycje i kolizje; snapshoty
   tylko z graczami w tym samym pomieszczeniu; pakiety binarne ≤ ~1200 B.
2. Mapa parteru w siatce kafelków, jeden format danych dla serwera i klienta.
3. Klient: ekran startowy (nick, adres, „Połącz”), placeholderowe kafelki,
   predykcja + rekoncyliacja, interpolacja innych (~100 ms), nick nad
   postacią, kamera, overlay F3.
4. Boty: N wirtualnych klientów chodzących losowo, część w jednym pokoju.

Kryteria akceptacji: `cargo run` startuje serwer, kilka klientów łączy się
lokalnie; własny ruch natychmiastowy, inni płynni; 50 botów w jednym pokoju →
klient 60 FPS, serwer bez zgubionych ticków (logi czasu ticka i transferu);
lag ~100 ms + 2% strat → nadal płynnie; zmiana pokoju zmienia widocznych
graczy; testy jednostkowe serializacji i ruchu/kolizji.

## Mapa parteru (floor0)

Siatka 64×40 kafli po 16 px (`client/maps/floor0.json`).

```
################################################################
#==================#..........................#,,,,,,,,#TTTTTTT#
#==================#..........................#,,,,,,,,#:::::::#
#==XXX========XXX==#...TTTTTT...TTTTTT..TTTT..#,,,,,,,,#:::::::#
#==XXX========XXX==#....................TTTT..#,,,,,,,,#:::::::#
#==================#....................TTTT..#,,TTTT,,#::::TT:#
#==================#..........................#,,TTTT,,#::::TT:#
#==================#...TTTTTT...TTTTTT........#,,TTTT,,#::::TT:#
#==================#..........................#,,TTTT,,#::::TT:#
#==XXX========XXX==#..........................#,,TTTT,,#:::::::#
#==XXX========XXX==#..........................#,,TTTT,,#:::::::#
#==================#...TTTTTT...TTTTTT........#,,TTTT,,#:::::::#
#==================#..........................#,,TTTT,,#:::::::#
#==================#..........................#,,TTTT,,#:::::::#
#==================#..........................#,,,,,,,,#:::::::#
#==XXX========XXX==#...TTTTTT...TTTTTT........#,,,,,,,,#:::::::#
#==XXX========XXX==#..........................#,,,,,,,,#:::::::#
#==================#..........................#,,,,,,,,#:::::::#
#==================#..........................#,,,,,,,,#:::::::#
#==================#############DD######DD########DD######DD####
#==================#...........................................#
#==================D...........................................#
#==================D...........................................#
#==================#...........................................#
#==================#####################DDD#####################
#==================#...........#___________________#...........#
#==================#...........#___________________#...TTTTTTT.#
#==XXX========XXX==#...........#___________________#...........#
#==XXX========XXX==#...........D___________________#...........#
#==================#...........D___________________#...TTTTTTT.#
#==================#...........#___________________D...........#
#==================#...........#___________________D...........#
#==================#TTTTTTT....#___________________#...TTTTTTT.#
#==XXX========XXX==#...........#___________________#...........#
#==XXX========XXX==#...........#___________________#...........#
#==================#...........#___________________#...........#
#==================#...........#___________________#.TTT.......#
#==================#...........#___________________#...........#
#==================#...........#___________________#...........#
########################################GGG#####################
```

Układ: lewo — Parking wewnętrzny; góra — Open space, Sala konferencyjna,
Kuchnia; środek — Korytarz; dół — Portiernia, Wejście (hol), Sklep.

Legenda: `#` ściana, `.` podłoga, `,` wykładzina, `:` płytki, `_` posadzka
holu, `=` parking, `D` drzwi, `G` szklane drzwi wejściowe (na razie zamknięte —
świat zewnętrzny nie istnieje), `T` meble (biurka, lady, regały), `X` auta.

| id | pokój | uwagi |
|----|-------|-------|
| 1 | Wejście | hol z punktem spawnu, drzwi do korytarza, portierni i sklepu |
| 2 | Portiernia | lada recepcji |
| 3 | Korytarz | łączy wszystko; kafle drzwi należą do korytarza |
| 4 | Sklep | regały, lada |
| 5 | Parking wewnętrzny | auta; drzwi do korytarza |
| 6 | Open space | rzędy biurek; największe pomieszczenie |
| 7 | Sala konferencyjna | stół konferencyjny |
| 8 | Kuchnia | blat, stolik |

## Stan implementacji

*Stan na 2026-09-25 — etap 1 (sieć) ukończony.*

### Zrobione
- **Serwer Rust** (`server/`): tick 20 Hz bez dryfu z liczeniem zgubionych
  ticków; handshake z `nonce`/`token`, odrzucenia (pełny serwer, wersja,
  nick), timeout 5 s; kolejka inputów z limitem 6/tick; kolizje ze ścianami i
  meblami; snapshoty z interest management po `(piętro, pokój)`,
  fragmentowane ≤ 1200 B; nicki przez `PlayerInfo`/`InfoRequest`; ping;
  statystyki co 5 s; symulator `--lag-ms/--jitter-ms/--loss`.
- **Mapa**: parter 64×40 z 8 pomieszczeniami i drzwiami w jednym pliku JSON
  czytanym przez obie strony, weryfikowanym CRC32 przy połączeniu.
- **Klient Godot** (`client/`): ekran startowy, mapa z kolorowych kafli i
  podpisów pomieszczeń, predykcja + rekoncyliacja z wygładzaniem korekt,
  interpolacja innych graczy (100 ms), nicki, kamera, overlay F3.
- **Boty** (`cargo run --release --bin bots`): 50 domyślnie, chodzą po BFS,
  część zbiera się w wybranym pokoju.
- **Testy**: 19 jednostkowych w Rust (mapa, ruch/kolizje, protokół), golden +
  e2e serwera (łącznie 23), 79 sprawdzeń w Godocie (parytet protokołu i ruchu).

### Pomiary (MacBook, wszystko lokalnie)
| scenariusz | wynik |
|------------|-------|
| 50 botów w jednym pokoju | serwer: 0 zgubionych ticków, tick śr. ~1,2 ms (max ~2,6 ms), ~12 KB/s na klienta |
| klient + 50 botów w tym samym pokoju | 60 FPS (vsync), 50 widocznych, 0 korekt predykcji |
| RTT ~117 ms, jitter 10 ms, 2% strat | 60 FPS, 0 korekt, bufor interpolacji pusty w 0,40% klatek |
| RTT ~226 ms, jitter 20 ms, 2% strat | 60 FPS, 0 korekt, bufor pusty w 0,25% klatek |
| przejście z Wejścia do Korytarza | widoczni: 42 → 5 |

### Znane ograniczenia
- Brak kolizji między graczami (celowo — to biuro, nie bijatyka).
- Ping w F3 ma rozdzielczość klatki (~16 ms), bo `Pong` jest czytany w `_process`.
- Szklane drzwi wejściowe są zamknięte, bo mapa nie ma świata zewnętrznego.
- Eksport klienta: przy eksporcie trzeba dodać `*.json` do filtra zasobów
  nie-Godotowych, inaczej `maps/floor0.json` nie trafi do paczki.

### Następne kroki (propozycja)
Piętra i winda, karty dostępu do drzwi, NPC, interakcje — punkty wpięcia
opisane w `docs/ARCHITECTURE.md` („Gotowość na rozbudowę”).
