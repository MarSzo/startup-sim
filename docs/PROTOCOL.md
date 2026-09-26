# Protokół sieciowy (wersja 5)

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
| version | u8  | `5` |
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
| map_crc     | u32 — CRC32 (IEEE) budynku: bajty `building.json`, a po nich kolejno plików wszystkich pięter |
| server_tick | u32 |

Klient liczy to samo CRC ze swoich plików `maps/` i przy różnicy się rozłącza.
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

Bity inputu: `1` góra, `2` dół, `4` lewo, `8` prawo, `16` interakcja (klawisz E —
na razie winda); bity 5–7 zarezerwowane (bieg…). Klient powtarza w każdym pakiecie 4 ostatnie inputy,
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
| floor          | u8 — piętro odbiorcy |
| room           | u16 — id pokoju odbiorcy na jego piętrze (0 = brak) |
| self_lock      | u8 — blokada schodów odbiorcy (`sim::Body::lock`: 0 brak, 1 trzymane, 2 zwolnione) |
| self_prev_input| u8 — poprzedni input odbiorcy (`sim::Body::prev_input`, do akcji „na wciśnięcie”) |
| self_access    | u8 — uprawnienia odbiorcy (`map::access`: 1 przepustka gościa, 2 karta pracownika, 4 obsługa) |
| self_status    | u8 — czynność odbiorcy (nie symulowana): bit 0 trzyma kawę, bit 1 parzy kawę |
| n              | u8 |
| entities       | n × 12 B |

Encja (12 B): `id u16 | kind u8 | x i32 | y i32 | flags u8`.
- `kind`: 0 gracz, 1 NPC. Id NPC zaczynają się od `0xF000` (61440); gracze mają 1..61439.
- `flags`: bity 0–1 kierunek (0 dół, 1 góra, 2 lewo, 3 prawo), bit 2 „w ruchu”,
  bity 3–5 wygląd (0 gracz, 1 portier — mundur z czapką, 2 pracownik biurowy —
  koszula z krawatem), bity 6–7 czynność (jak `self_status`: 6 trzyma kawę, 7 parzy).

**Interest management**: lista zawiera tylko encje z tym samym `(floor, room)` co
odbiorca (bez niego samego). Snapshot jest pełny (nie delta) — zgubienie
któregokolwiek nie wymaga retransmisji.

`self_*` + `floor` to **pełny stan symulacji** odbiorcy, więc klient odtwarza
niepotwierdzone inputy dokładnie od tego stanu, także przez schody i windę.

Uprawnienia zmienia tylko serwer (np. portier daje przepustkę); klient poznaje
je ze snapshotu i od razu uwzględnia w predykcji kolizji z bramkami.

**Fragmentacja**: stała część snapshotu ma 30 B, więc mieści się 97 encji
(30 + 97·12 = 1194 B). Więcej encji → kilka fragmentów z tym samym `tick`,
każdy z pełnymi polami `self_*`. Pusty pokój → 1 fragment z `n = 0`.

### 6 `PlayerInfo` (S→C)
| pole    | typ |
|---------|-----|
| n       | u8 |
| players | n × (`id u16`, nick `u8 len + UTF-8`, `department u8`) |

Wysyłany, gdy encja (gracz lub NPC — wtedy `nick` to jego imię, np. „Portier”)
pierwszy raz staje się widoczna dla odbiorcy, przed pierwszą skierowaną do niego
wypowiedzią NPC oraz w odpowiedzi na `InfoRequest`. Max 55 wpisów na pakiet.
`department` — dział gracza po podpisaniu umowy w HR (1 IT / Produkt, 2 Biznes,
0 brak / NPC). Po podpisaniu umowy serwer rozsyła `PlayerInfo` ponownie.

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
`token u32 | reason u8` — 0 wyjście klienta, 1 timeout, 2 wyrzucenie, 3 wyłączenie serwera,
4 nieznana sesja (odpowiedź serwera na `Input`/`Ping` z tokenem, którego nie zna —
sesja wygasła albo serwer był restartowany).

### 11 `Say` (S→C)
| pole | typ |
|------|-----|
| id   | u16 — kto mówi (NPC albo gracz — np. „Parzę kawę…” nad własną głową) |
| text | u16 len + UTF-8 (max 240 B, ucinane na granicy znaku) |

Wypowiedź pokazywana w dymku nad postacią i w logu na dole ekranu. Trafia do
wszystkich w tym samym `(piętro, pokój)` co mówiący oraz do gracza, do którego
jest skierowana. Bez retransmisji (zgubiona linia przepada — to tylko dialog).

### 12–16 Portal z ofertami i rekrutacja

Nowy gracz po `Welcome` **nie jest jeszcze w świecie** (nie dostaje
snapshotów, jego inputy są ignorowane): jest na portalu z ofertami. Serwer
**co 1 s ponawia bieżący ekran** (oferty albo aktualne pytanie), więc zgubiony
pakiet nie blokuje rekrutacji; klient, widząc ponownie ekran, na który już
odpowiedział, wysyła swoją odpowiedź jeszcze raz.

| typ | kierunek | treść |
|-----|----------|-------|
| 12 `JobOffers` | S→C | n u8, n × {`id u8`, `department u8`, `title` u16 len + UTF-8, `description` u16 len + UTF-8} |
| 13 `Apply` | C→S | token u32, offer u8 — zaczyna nową próbę (losowe pytania oferty) |
| 14 `Question` | S→C | attempt u8, index u8, total u8, `text` str16, n u8 (≤ 4), n × `option` str16 (kolejność potasowana) |
| 15 `Answer` | C→S | token u32, attempt u8, index u8, choice u8 — odpowiedzi nieaktualne (inna próba / pytanie) są ignorowane |
| 16 `RecruitResult` | S→C | attempt u8, passed u8 (0/1), score u8, total u8, department u8 — wysyłany 2× |

Zasady (`server/data/recruitment.json`): 3 losowe pytania z puli oferty, 2
poprawne = przyjęcie. Poprawne odpowiedzi zna tylko serwer. Po przyjęciu
gracz pojawia się przed budynkiem (pierwszy `Snapshot` = potwierdzenie, nawet
gdy `RecruitResult` zginie); po porażce wraca na listę ofert.

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

## Sesja, zmiana adresu i ponowne łączenie

**Gracza identyfikuje `token`, nie adres.** Pakiety C→S (poza `Connect`) są
przypisywane do sesji po tokenie, z dowolnego adresu i rodziny (IPv4/IPv6).

- **Migracja adresu**: jeśli `Ping` albo `Input` z *nowymi* inputami
  (`last_seq` > ostatnio odebrany) przyjdzie z innego adresu, serwer od razu
  przenosi sesję na ten adres — snapshoty idą tam od następnego ticku. Stare,
  spóźnione pakiety z poprzedniego adresu nie mogą przenieść sesji z powrotem.
  Pokrywa to zmianę Wi-Fi ↔ LTE, nowy port NAT, wybudzenie laptopa.
- **Nieznany token** → serwer odpowiada `Disconnect(4)`; klient zaczyna nowy
  `Connect` (nowe `player_id` i `token`).
- **Klient** (`net_client.gd`):
  - po **1,5 s** ciszy otwiera nowe gniazdo (nowy port, ponowne rozwiązanie
    nazwy hosta) i wysyła `Ping` z tym samym tokenem; powtarza co 1,5 s;
  - po powrocie aplikacji z tła (mobile) robi to od razu;
  - po **5 s** ciszy, `Disconnect(1)` lub `Disconnect(4)` łączy się od nowa
    (`Connect`), próbując przez **30 s**, zanim wróci do ekranu startowego.
    Gra w tym czasie jest zamrożona z komunikatem „Łączenie ponownie…”.

Bezpieczeństwo: `token` to 32-bitowa losowa wartość wysyłana otwartym tekstem,
więc chroni przed przypadkowym i „ślepym” podszyciem się, ale nie przed kimś,
kto podsłuchuje ruch. Docelowo (konta, konsole) zastąpi go uwierzytelnienie z
szyfrowaniem.

## IPv6

- Serwer domyślnie nasłuchuje na `[::]:7777` w trybie **dual-stack**
  (`IPV6_V6ONLY = 0` ustawiane jawnie), więc obsługuje klientów IPv4 i IPv6 na
  jednym gnieździe; bez IPv6 na hoście spada na `0.0.0.0:7777`.
- Klient rozwiązuje nazwę hosta z `IP.TYPE_ANY` (działa w sieciach tylko-IPv6,
  wymaganych przez App Store) i akceptuje adresy `host`, `host:port`,
  `1.2.3.4:port`, `[2001:db8::1]:port`, `[::1]` i gołe `::1` (port domyślny 7777).

## Rozmiary i transfer (zmierzone)

| sytuacja | S→C na klienta |
|----------|----------------|
| sam w pokoju | ~0,7 KB/s |
| 49 innych graczy w pokoju | ~12 KB/s (snapshot 614 B × 20/s) |
| C→S (input 60 Hz + ping) | ~1,2 KB/s |

## Historia wersji

- **5** — `self_status` w snapshocie, bity czynności 6–7 we `flags` encji; `Say` także od graczy.
- **4** — portal i rekrutacja (typy 12–16); `department` w `PlayerInfo`.
- **3** — snapshot: `self_access`; pakiet `Say`; encje NPC (`kind` 1) z imionami w
  `PlayerInfo`; wygląd w bitach 3–5 `flags` (dodany bez zmiany formatu).
- **2** — snapshot: pola `self_lock`, `self_prev_input`; bit inputu 16 (interakcja);
  `map_crc` liczone z całego budynku (wiele pięter).
- **1** — wersja początkowa.

## Rozszerzenia (zaplanowane, nie zaimplementowane)

- Akcje (drzwi z kartą dostępu, sklep, rozmowy): bit interakcji 16 + kontekst
  miejsca (jak winda) lub nowe typy pakietów; bity 5–7 wolne.
- Kompresja delta: `ack_tick` już jest w `Input`.
- Zmiana formatu = podbicie `VERSION`; stary klient dostaje `Reject(2)`.
