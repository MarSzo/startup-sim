# Protokół sieciowy (wersja 15)

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
| version | u8  | `15` |
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
| nick  | u8 len + UTF-8 (imię postaci) |
| gender | u8 — 0 kobieta, 1 mężczyzna, 2 inna |
| age   | u8 — 18..70 |
| appearance | 5 × u8: skóra (0..3), fryzura (0..5), kolor włosów (0..6), koszula (0..9), spodnie (0..4) |
| city  | u16 len + UTF-8 (≤ 48 B) |
| email | u16 len + UTF-8 (≤ 64 B, format `x@y.z`) |

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
Wiek, miejscowość i e-mail to dane *postaci* (fikcyjne) — zostają na serwerze;
innym graczom idą tylko imię, płeć i wygląd (`PlayerInfo`).

### 3 `Reject` (S→C)
| pole   | typ |
|--------|-----|
| reason | u8 — 1 serwer pełny, 2 zła wersja protokołu, 3 złe imię, 4 złe dane postaci |

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
| self_slow      | u8 — `sim::Body::slow` odbiorcy (1 = wolny chód: wyczerpanie / pilna toaleta); część symulowanego stanu |
| self_activity  | u8 — czynność odbiorcy (nie symulowana), jak `activity` encji; 1 = przy komputerze (klient pokazuje jego ekran, dopóki trwa) |
| n              | u8 |
| entities       | n × 14 B |

Encja (14 B): `id u16 | kind u8 | x i32 | y i32 | flags u8 | held u8 | activity u8`.
- `kind`: 0 gracz, 1 NPC, 2 przedmiot na podłodze, 3 laptop na biurku.
  Id: gracze 1..0xDFFF, przedmioty na podłodze i laptopy na biurkach od
  `0xE000` (wspólna pula), NPC od `0xF000`. `PlayerInfo` laptopa niesie imię
  i dział jego właściciela.
- `held`: przedmiot w rękach (0 brak, 1 przepustka gościa, 2 karta
  pracownika, 3 laptop, 4 kawa, 5 owoc); dla `kind` 2 i 3 — sam przedmiot.
- `activity`: 0 nic, 1 przy komputerze, 2 parzy kawę, 3 odpoczywa na sofie,
  4 w toalecie, 5 pali (strefa palenia), 6 myje ręce.
- `flags`: bity 0–1 kierunek (0 dół, 1 góra, 2 lewo, 3 prawo), bit 2 „w ruchu”,
  bity 3–5 wygląd (0 gracz, 1 portier — mundur z czapką, 2 pracownik biurowy —
  koszula z krawatem), bit 6 wolny chód (zmęczenie / pilna toaleta), bit 7
  niska higiena (chmurka).
  Dla laptopa (`kind` 3): bit 0 zablokowany, bit 1 ktoś przy nim siedzi.

**Interest management**: lista zawiera tylko encje z tym samym `(floor, room)` co
odbiorca (bez niego samego). Snapshot jest pełny (nie delta) — zgubienie
któregokolwiek nie wymaga retransmisji.

`self_*` + `floor` to **pełny stan symulacji** odbiorcy, więc klient odtwarza
niepotwierdzone inputy dokładnie od tego stanu, także przez schody. Windą
przenosi serwer (zmiana `floor` w snapshocie = przeskok, jak korekta).

Uprawnienia zmienia tylko serwer (np. portier daje przepustkę); klient poznaje
je ze snapshotu i od razu uwzględnia w predykcji kolizji z bramkami.

**Fragmentacja**: stała część snapshotu ma 31 B, więc mieści się 83 encje
(31 + 83·14 = 1193 B). Więcej encji → kilka fragmentów z tym samym `tick`,
każdy z pełnymi polami `self_*`. Pusty pokój → 1 fragment z `n = 0`.

### 6 `PlayerInfo` (S→C)
| pole    | typ |
|---------|-----|
| n       | u8 |
| players | n × (`id u16`, nick `u8 len + UTF-8`, `department u8`, `gender u8`, `appearance 5 × u8`) |

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

### 12–18 Pulpit: portal z ofertami, poczta, rozmowa online

Nowy gracz po `Welcome` **nie jest jeszcze w świecie** (nie dostaje
snapshotów, jego inputy są ignorowane): siedzi w domu przy komputerze. Serwer
**co 1 s ponawia stan pulpitu** — listę ofert (z flagą „zaaplikowano”) i
bieżące pytanie rozmowy, a co 2 s całą skrzynkę odbiorczą — więc zgubiony
pakiet niczego nie blokuje; klient ponawia swoją ostatnią akcję, jeśli stan
serwera pokazuje, że do niego nie dotarła.

Przebieg: `Apply` (formularz) → po `invite_delay_secs` `Mail` z zaproszeniem
(`action` 1, `arg` = oferta) → `PortalAction(1)` → `Question`/`Answer` ×3 →
`RecruitResult` + `Mail` (sukces: zaproszenie na dzień próbny, `action` 2;
porażka: podziękowanie, można aplikować ponownie) → `PortalAction(2)` →
gracz pojawia się przed budynkiem. Inne firmy odpowiadają `Mail` z odmową
(bez akcji) albo milczą.

| typ | kierunek | treść |
|-----|----------|-------|
| 12 `JobOffers` | S→C | n u8, n × {`id u8`, `department u8` (0 = inna firma), `applied u8`, `company` str16, `title` str16, `description` str16} — lista może przyjść w kilku pakietach (≤ 1200 B każdy); klient scala po `id` |
| 13 `Apply` | C→S | token u32, offer u8, `motivation` str16 („Dlaczego chcesz u nas pracować?”) |
| 14 `Question` | S→C | attempt u8, index u8, total u8, `text` str16, n u8 (≤ 4), n × `option` str16 (kolejność potasowana) |
| 15 `Answer` | C→S | token u32, attempt u8, index u8, choice u8 — odpowiedzi nieaktualne (inna próba / pytanie) są ignorowane |
| 16 `RecruitResult` | S→C | attempt u8, passed u8 (0/1), score u8, total u8, department u8 — wysyłany 2× |
| 17 `Mail` | S→C | id u8, `from` str16, `subject` str16, `body` str16 (≤ 600 B), action u8 (0 brak, 1 dołącz do rozmowy, 2 idę do biura), arg u8 |
| 18 `PortalAction` | C→S | token u32, action u8, arg u8 — przycisk z maila |

Zasady (`server/data/recruitment.json`): 3 losowe pytania z puli stanowiska, 2
poprawne = przyjęcie. Poprawne odpowiedzi zna tylko serwer.

### 19 `Inventory` (S→C), 20 `ItemAction` (C→S)

`Inventory`: n u8 (= 4), n × {`kind u8`, `id u32`, `label` str16} — najpierw
ręce, potem 3 kieszenie; wysyłany po każdej zmianie i co 2 s.
`ItemAction`: token u32, action u8, slot u8 — 1 wyjmij kieszeń `slot` do rąk
(zamiana z małym przedmiotem w rękach), 2 schowaj z rąk do wolnej kieszeni,
3 upuść z rąk na podłogę, 4 podaj z rąk najbliższemu graczowi (≤ 2 kafle),
5 użyj (kawa: wypij; karta/przepustka: pokaż). Podniesienie przedmiotu z
podłogi to E (jak rozmowa). Odmowy wracają jako `Say` od samego gracza.

Uprawnienia (`self_access`) wynikają z noszonych przedmiotów: przepustka →
gość, karta → pracownik — niezależnie od tego, czyja jest.

### 21 `Computer` (S→C), 22 `ComputerAction` (C→S), 23 `Chat` (S→C)

Komputer to laptop położony na biurku; zawsze jest zalogowany na **właściciela**
— kto siedzi przy cudzym odblokowanym komputerze, pisze w jego imieniu.

`Computer` — ekran komputera, przy którym siedzi odbiorca (po E przy biurku,
po każdej zmianie i co 1 s): handle u16 (id encji laptopa), owner u16 (id
właściciela), locked u8, n u8 (≤ 40), n × {`conv u16`, `unread u8`, `title`
str16 (≤ 24 B)}. Zablokowany komputer nie pokazuje rozmów (n = 0).

Rozmowy (`conv`, z perspektywy konta właściciela): 1 = #ogólny, 16 + id działu
= kanał działu (tylko ten dział), `0x8000 | id gracza` = wiadomości prywatne.
`unread` = wiadomości innych nowsze niż ostatnio przeczytana.

`ComputerAction`: token u32, action u8, conv u16, arg u32, text str16 (≤ 400 B):
1 zamknij ekran, 2 zablokuj (i zamknij; może każdy), 3 odblokuj (tylko
właściciel — inaczej `Say` z odmową), 4 zabierz laptop (wymaga wolnych rąk; może
każdy, także zablokowany), 5 synchronizuj `conv` — odpowiedź `Chat` z
wiadomościami o id > `arg` (najnowsze 30; oznacza je jako przeczytane),
6 wyślij `text` do `conv`; `arg` = nonce klienta (ponowienia z tym samym
nonce są ignorowane; limit 1 wiadomość / 0,5 s, max 200 znaków).

`Chat`: conv u16, n u8, n × {`id u32`, `from u16`, `nick` str16, `text` str16}
— odpowiedź na synchronizację albo natychmiastowe powiadomienie wszystkich,
którzy właśnie patrzą na ekran konta z tej rozmowy. Dzielony na kilka pakietów,
żeby każdy mieścił się w 1200 B. Klient synchronizuje otwartą rozmowę co 1 s,
więc zgubiony pakiet nie gubi wiadomości.

Historia jest tylko w pamięci serwera (60 wiadomości na rozmowę); wiadomości
prywatne gracza, który wyszedł, są usuwane (jego id może dostać ktoś inny).

### 25 `Doors` (S→C), 26 `DoorAction` (C→S)

Kabiny toaletowe: drzwi (typ kafla `stall_door`) zamknięte od środka są
**nieprzechodnie dla wszystkich** — to część symulacji ruchu, więc klient musi
je znać do predykcji. `Doors`: floor u8, n u8, n × {`x u8`, `y u8`} — lista
zamkniętych drzwi na piętrze odbiorcy; wysyłana po każdej zmianie i co 0,5 s
(zastępuje poprzednią listę dla tego piętra). Lista obejmuje też **drzwi
windy** — zamknięte, dopóki winda nie stoi na danym piętrze z otwartymi
drzwiami. Na końcu pakietu: `lift_floor u8` (gdzie jest winda) i
`lift_target u8` (dokąd jedzie / najbliższe wezwanie; 255 = stoi) i
`lift_moving u8` (1 = w ruchu) — do wyświetlacza przy drzwiach, podpowiedzi i
widoku samej kabiny w czasie jazdy. Winda nie rusza z więcej niż 6 osobami w
kabinie (drzwi zostają otwarte, `Say` „Przeciążenie!” od kogoś w kabinie). `DoorAction`: token u32 —
zamknij / otwórz kabinę, w której stoi nadawca (odmowy jako `Say`: nie w
kabinie, ktoś stoi w drzwiach, sam stoi w drzwiach). Serwer otwiera kabinę
sam, gdy zamykający z niej wyjdzie albo wyjdzie z gry.

Każda kabina jest osobnym pokojem, który „widzi” łazienkę (ale nie odwrotnie):
nikt z łazienki nie widzi, kto jest w środku; kto wejdzie w otwarte drzwi
(pole drzwi należy do kabiny), ten widzi. `Say` trafia też do pokoi, które
widzą pokój mówiącego.

### 24 `Stats` (S→C)

Potrzeby postaci odbiorcy, co 0,5 s (tylko w budynku): `hunger u8`, `energy
u8`, `stress u8`, `bladder u8`, `hygiene u8` (każda 0..100), `flags u8` (bit 0
brudne ręce), `money u32` (portfel w groszach). Głód, stres i toaleta: 100 =
źle; energia i higiena: 0 = źle. Liczy je tylko serwer.

### 27 `Shelf` (S→C), 28 `ShopTake` (C→S)

`Shelf` — odpowiedź na E przy półce sklepowej: shelf u8, title str16, n u8 (≤ 16),
n × {`kind u8`, `price u32` (grosze), `name` str16}. `ShopTake`: token u32,
shelf u8, kind u8 — weź jedną sztukę (serwer sprawdza zasięg półki); towar
trafia do ekwipunku jako niezapłacony (etykieta w `Inventory` z dopiskiem i
ceną). Płacenie: E przy NPC „Kasa” (odpowiedź jako `Say`). Wyjście ze sklepu
z niezapłaconym towarem: `Say` z alarmem od kasy, towar znika.

Rodzaje przedmiotów sklepowych (`held`, `Inventory.kind`): 10 kanapka z serem,
11 z szynką, 12 wrap wege, 13 hamburger, 14 frytki, 15 drożdżówka, 16 batonik,
17 chipsy, 18 woda, 19 energetyk, 20 sok, 21 piwo, 22 wino, 23 papierosy.

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

- **15** — sklep: `Stats` + `money`, `Shelf`, `ShopTake`, przedmioty 10–23.
- **14** — `Doors` + `lift_moving`; limit 6 osób w windzie; mniejsza kabina (3×2).
- **13** — higiena: `Stats` + `hygiene`, `flags` (brudne ręce), flaga encji 7 = niska higiena, czynność 6 = mycie rąk.
- **12** — winda poza symulacją (wzywanie, jazda, drzwi w `Doors`), `Doors` + `lift_floor`, `lift_target`; mapa klatki schodowej (piętro 3).
- **11** — kabiny toaletowe: `Doors`, `DoorAction`; zamknięte drzwi blokują ruch (także w predykcji klienta).
- **10** — potrzeby: `activity` w encji (14 B) i `self_activity` w miejsce bitów `self_status`, `self_slow` (wolny chód w symulacji — też w wektorach golden ruchu), flaga 6 = wolny chód, pakiet `Stats`, przedmiot 5 = owoc.
- **9** — komputer i komunikator: encja laptopa (`kind` 3), bit „przy komputerze” (`self_status` 0 / flaga 6, w miejsce „trzyma kawę”), `Computer`, `ComputerAction`, `Chat`.
- **8** — ekwipunek: `held` w encji (13 B), encje przedmiotów na podłodze, `Inventory`, `ItemAction`; kubek kawy jako przedmiot.
- **7** — pulpit: firmy i flaga `applied` w `JobOffers` (dzielonych na pakiety), `motivation` w `Apply`, `Mail`, `PortalAction`.
- **6** — profil postaci w `Connect` (płeć, wiek, wygląd, miejscowość, e-mail); płeć i wygląd w `PlayerInfo`; `Reject(4)`.
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
