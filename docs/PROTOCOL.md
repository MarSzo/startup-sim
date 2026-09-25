# Protokół sieciowy (wersja 1)

Własny binarny protokół na UDP. Implementacje:
- serwer: `server/src/protocol.rs` (źródło prawdy),
- klient: `client/net/protocol.gd`.

Parytet sprawdzają pliki golden w `server/tests/golden/packets.json`, generowane
przez `cargo test` i czytane przez `client/tests/run_tests.gd`.

## Zasady ogólne

- Kolejność bajtów: **little-endian**.
- Każdy datagram zawiera dokładnie jeden pakiet. Żaden pakiet nie przekracza **1200 B**.
- Brak JSON-a i tekstu poza nickami (UTF-8, długość w bajtach `u8`, max **16 B**,
  ucinane na granicy znaku).
- Pakiet z nadmiarowymi bajtami, uciętym polem, złym `magic`, nieznanym typem lub
  złą wersją jest odrzucany w całości.
- Protokół nie ma warstwy niezawodności: stan jest wysyłany w pełnych snapshotach,
  a nieliczne zdarzenia (nick) są samonaprawiające się (`InfoRequest`).

### Nagłówek (4 B)

| pole    | typ | wartość |
|---------|-----|---------|
| magic   | u16 | `0x5354` (bajty `54 53`, „TS”) |
| version | u8  | `1` |
| type    | u8  | typ pakietu (niżej) |

## Jednostki

- Pozycja: `i32` w **subpikselach** — 1 px = 16 j., 1 kafel (16 px) = 256 j.
  Pozycja to środek pudełka kolizji (10×8 px) — „stopy” postaci.
- Tick serwera: `u32`, 20 Hz (50 ms).
- Sekwencja inputu: `u32`, rośnie o 1 na każdy krok wejścia 60 Hz, zaczyna od 1.

## Typy pakietów

### 1 `Connect` (C→S)
| pole  | typ |
|-------|-----|
| nonce | u32 — losowy, identyfikuje próbę połączenia |
| nick  | u8 len + UTF-8 |

Klient powtarza co 500 ms, aż dostanie `Welcome`/`Reject`; po 5 s się poddaje.
Serwer na powtórzony `Connect` z tym samym `nonce` z tego samego adresu
odsyła ponownie `Welcome` (poprzedni mógł zginąć). Inny `nonce` z tego samego
adresu = ponowne połączenie (stary gracz jest usuwany).

### 2 `Welcome` (S→C)
| pole        | typ |
|-------------|-----|
| nonce       | u32 — echo z `Connect` |
| player_id   | u16 — id encji gracza (≠ 0) |
| token       | u32 — sekret sesji, wymagany w każdym dalszym pakiecie C→S |
| tick_hz     | u8 — 20 |
| input_hz    | u8 — 60 |
| map_crc     | u32 — CRC32 (IEEE) surowego pliku mapy |
| server_tick | u32 |

Klient porównuje `map_crc` z CRC swojego `maps/floor0.json`; przy różnicy się rozłącza.
Pozycja startowa przychodzi w pierwszym `Snapshot`.

### 3 `Reject` (S→C)
| pole   | typ |
|--------|-----|
| reason | u8 — 1 serwer pełny, 2 zła wersja protokołu, 3 zły nick |

### 4 `Input` (C→S) — wysyłany co krok wejścia (60 Hz)
| pole     | typ |
|----------|-----|
| token    | u32 |
| ack_tick | u32 — ostatni odebrany tick (zarezerwowane pod kompresję delta) |
| last_seq | u32 — sekwencja ostatniego inputu w pakiecie |
| count    | u8 — 1..8 |
| inputs   | count × u8, najstarszy pierwszy; input `i` ma seq `last_seq - (count-1-i)` |

Bity inputu: `1` góra, `2` dół, `4` lewo, `8` prawo; bity 4–7 zarezerwowane
(akcje: interakcja, bieg…). Klient powtarza w każdym pakiecie 4 ostatnie inputy,
więc zgubienie do 3 kolejnych pakietów nie traci ruchu.

Serwer przyjmuje inputy o `seq > ostatnio odebrany`, kolejkuje je i w każdym ticku
aplikuje max 6 (średnio 3 = 60/20). Kolejka ponad 30 jest przycinana od najstarszych.

### 5 `Snapshot` (S→C) — co tick, do każdego klienta
| pole           | typ |
|----------------|-----|
| tick           | u32 |
| last_input_seq | u32 — seq ostatniego zaaplikowanego inputu odbiorcy |
| frag_idx       | u8 |
| frag_cnt       | u8 |
| self_x, self_y | i32, i32 — autorytatywna pozycja odbiorcy |
| floor          | u8 — piętro (na razie zawsze 0) |
| room           | u16 — id pokoju odbiorcy (0 = brak) |
| n              | u8 |
| entities       | n × 12 B |

Encja (12 B): `id u16 | kind u8 | x i32 | y i32 | flags u8`.
- `kind`: 0 gracz, 1 NPC (zarezerwowane).
- `flags`: bity 0–1 kierunek (0 dół, 1 góra, 2 lewo, 3 prawo), bit 2 „w ruchu”, reszta zarezerwowana.

**Interest management**: lista zawiera tylko encje z tym samym `(floor, room)` co
odbiorca (bez niego samego). Snapshot jest pełny (nie delta) — zgubienie
któregokolwiek nie wymaga retransmisji.

**Fragmentacja**: stała część snapshotu ma 26 B, więc mieści się 97 encji
(26 + 97·12 = 1190 B). Więcej encji → kilka fragmentów z tym samym `tick`,
każdy z pełnymi polami `self_*`. Pusty pokój → 1 fragment z `n = 0`.

### 6 `PlayerInfo` (S→C)
| pole    | typ |
|---------|-----|
| n       | u8 |
| players | n × (`id u16`, nick `u8 len + UTF-8`) |

Wysyłany, gdy encja pierwszy raz staje się widoczna dla odbiorcy, oraz w
odpowiedzi na `InfoRequest`. Max 55 wpisów na pakiet.

### 7 `InfoRequest` (C→S)
| pole  | typ |
|-------|-----|
| token | u32 |
| n     | u8 |
| ids   | n × u16 |

Klient wysyła, gdy w snapshocie widzi id bez nicku (np. zgubiony `PlayerInfo`);
ponawia dla danego id nie częściej niż co 500 ms.

### 8 `Ping` (C→S) / 9 `Pong` (S→C)
`Ping`: `token u32 | client_time u32` (ms zegara klienta).
`Pong`: `client_time u32` (echo) `| server_tick u32`.
Klient pinguje co 1 s; RTT = teraz − `client_time`. Serwer odpowiada natychmiast
(poza tickiem).

### 10 `Disconnect` (obie strony)
`token u32 | reason u8` — 0 wyjście klienta, 1 timeout, 2 wyrzucenie, 3 wyłączenie serwera.

## Połączenie i timeouty

```
klient                         serwer
  | -- Connect(nonce, nick) -->  |   (co 500 ms, max 5 s)
  | <-- Welcome(id, token) ----  |
  | -- Input ... (60 Hz) ------> |
  | <-- Snapshot (20 Hz) ------  |
  | <-- PlayerInfo (gdy trzeba)  |
  | -- Ping (1 Hz) ------------> |
```

- Serwer usuwa klienta po **5 s** bez żadnego poprawnego pakietu (i wysyła mu `Disconnect(1)`).
- Klient uznaje połączenie za zerwane po 5 s bez pakietów od serwera.
- Pakiety C→S (poza `Connect`) są akceptowane tylko z adresu przypisanego do
  gracza i z poprawnym `token`.

## Rozmiary i transfer (zmierzone)

| sytuacja | S→C na klienta |
|----------|----------------|
| sam w pokoju | ~0,7 KB/s |
| 49 innych graczy w pokoju | ~12 KB/s (snapshot 614 B × 20/s) |
| C→S (input 60 Hz + ping) | ~1,2 KB/s |

## Rozszerzenia (zaplanowane, nie zaimplementowane)

- Piętra: pole `floor` już jest w snapshotach; interest management grupuje po `(floor, room)`.
- NPC: `kind = 1` w tym samym formacie encji.
- Akcje (drzwi z kartą dostępu, interakcje): bity 4–7 inputu lub nowe typy pakietów.
- Kompresja delta: `ack_tick` już jest w `Input`.
- Zmiana formatu = podbicie `VERSION`; stary klient dostaje `Reject(2)`.
