# Protokół sieciowy (wersja 44)

Własny binarny protokół na UDP. Implementacje:
- serwer: `server/src/protocol/` (źródło prawdy),
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
| version | u8  | `44` |
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
| self_drunk     | u8 — `sim::Body::drunk` odbiorcy (0 trzeźwy, 1 zataczanie od 50% upojenia, 2 mocniejsze i wolny chód od 75%); część symulowanego stanu |
| self_activity  | u8 — czynność odbiorcy (nie symulowana), jak `activity` encji; 1 = przy komputerze (klient pokazuje jego ekran, dopóki trwa) |
| n              | u8 |
| entities       | n × 14 B |

Encja (14 B): `id u16 | kind u8 | x i32 | y i32 | flags u8 | held u8 | activity u8`.
- `kind`: 0 gracz, 1 NPC, 2 przedmiot na podłodze, 3 laptop na biurku,
  4 pojazd, 5 taca słodyczy, 6 kałuża po wpadce (`held` 0), wymiociny
  (`held` 1) albo kupa (`held` 2); sprzątaczka ją ściera, inaczej znika o 22:00.
  Id: gracze 1..0xDFFF, przedmioty na podłodze i laptopy na biurkach od
  `0xE000` (wspólna pula), NPC od `0xF000`. `PlayerInfo` laptopa niesie imię
  i dział jego właściciela.
- `held`: przedmiot w rękach (0 brak, 1 przepustka gościa, 2 karta
  pracownika, 3 laptop, 4 kawa, 5 owoc); dla `kind` 2 i 3 — sam przedmiot.
- `activity`: 0 nic, 1 przy komputerze, 2 parzy kawę, 3 odpoczywa na sofie,
  4 w toalecie, 5 pali (strefa palenia), 6 myje ręce, 7 w pojeździe,
  8 zatrzymany (ochrona / policja), 9 wymiotuje, 10 śpi pijany (odsypia),
  11 znokautowany, 12 zadaje cios, 13 sika (pisuar, podłoga, ekspres, kubek),
  14 kuca (kupa na podłodze). NPC: Paulina (fotel) ma 3.
- `flags`: bity 0–1 kierunek (0 dół, 1 góra, 2 lewo, 3 prawo), bit 2 „w ruchu”,
  bity 3–5 wygląd (0 gracz, 1 portier — mundur z czapką, 2 pracownik biurowy —
  koszula z krawatem, 3 ochroniarz — czarny strój z żółtą opaską, 4 policjant —
  granatowy mundur z czapką, 5 sprzątaczka — turkusowy fartuch i mop, 6 strażak — czerwony hełm, odblaski, 7 kasjer — zielona koszulka i czapka; portier 1 to pani Wiesia — siwy kok, okulary, sweter), bit 6 wolny chód (zmęczenie / pilna toaleta), bit 7
  niska higiena (chmurka). U graczy (nie NPC) bit 3 = rozłożony parasol, bity 4–5
  = upojenie (0 trzeźwy, 1 od 25%, 2 od 50%, 3 od 75%).
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
`department` — dział gracza po podpisaniu umowy w HR (id z pakietu
`Departments`, np. 1 Produkt / IT, 2 Biznes, 3 Zarząd, 4 Mobile … 10 Obsługa
klienta; 0 brak / NPC). Po podpisaniu umowy serwer rozsyła `PlayerInfo` ponownie.

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
| 12 `JobOffers` | S→C | n u8, n × {`id u8`, `department u8` (0 = inna firma), `applied u8`, `vacancies u8` (wolne miejsca w naszym startupie; 0 = obsadzone), `salary_min u32`, `salary_max u32` (widełki, zł brutto / mies.), `company` str16, `title` str16, `description` str16} — lista może przyjść w kilku pakietach (≤ 1200 B każdy); klient scala po `id` |
| 13 `Apply` | C→S | token u32, offer u8, `motivation` str16 („Dlaczego chcesz u nas pracować?”), `salary u32` (oczekiwania, zł brutto / mies.; powyżej widełek — odmowa mailem), `form u8` (1 umowa o pracę, 2 B2B, 3 umowa zlecenie — tylko `student` < 26 lat, inaczej ignorowane), `student u8` |
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
drzwiami. Na końcu pakietu lista wind: `n u8` (≤ 16), n × {`floor u8` (gdzie
jest winda), `target u8` (dokąd jedzie / najbliższe wezwanie; 255 = stoi),
`moving u8` (1 = w ruchu)} — do wyświetlaczy przy drzwiach, podpowiedzi i
widoku samej kabiny w czasie jazdy. Kolejność wind: jak w mapie (piętra
rosnąco, linki w kolejności z pliku, pierwsze wystąpienie danego `id`) — klient
liczy ją tak samo (`Building.lift_ids`) i przypisuje drzwi do windy, której
kabiny dotykają. Winda nie rusza z więcej niż 6 osobami w
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
u8`, `stress u8`, `bladder u8`, `hygiene u8`, `alcohol u8`, `bowels u8`, `health u8` (każda 0..100),
`flags u8` (bit 0 brudne ręce, bit 1 rozstrój żołądka), `money u32` (portfel w groszach). Głód,
stres, toaleta, upojenie i jelita: 100 = źle; energia, higiena i zdrowie: 0 = źle. Liczy je tylko serwer.

### 29 `Clock` (S→C)

Czas gry dla każdego gracza (także w domu i na portalu), co 1 s i po każdej
zmianie (koniec dnia, poranek, przyjazd): `day u16` (dzień gracza: 1 = szukanie
pracy), `minute u16` (minuta doby 0..1439, zegar wspólny), `night u8` (biuro
zamknięte, 22:00–6:00), `place u8` (0 w budynku, 1 w domu na noc, 2 w drodze do
pracy, 3 na portalu), `arrive u16` (minuta przyjazdu albo 0xFFFF), `pay u32` i
`pay_minutes u16` (ostatnia wypłata: grosze, minuty gry w pracy),
`today_minutes u16` (minuty gry przepracowane dziś), `mode u8` (dojazd: 1
pieszo, 2 rower, 3 samochód, 4 taksówka, 5 tramwaj), `depart u16` (minuta
wyjazdu albo 0xFFFF), `money u32` (portfel — także w domu), `weather u8`
(1 słonecznie, 2 pochmurno, 3 deszcz, 4 burza, 5 mgła). Rano `place` = 2 i
`arrive` = 0xFFFF oznacza „jeszcze w domu, wybierz dojazd”; po wyjeździe
`arrive` = minuta przyjazdu. Od v23 na końcu: `company` str16 (nazwa firmy,
≤ 64 B) i `founded u8` (1 = firma ma założyciela; 0 = portal pokazuje „Załóż
firmę”). Od v26: `alarm u8` (1 = alarm pożarowy w budynku — ewakuacja). Od v42 na końcu: `leave u8` (1 = dziś dzień urlopu — w domu).

### 30 `CommuteChoice` (C→S)

token u32, mode u8 — wybór dojazdu (przed wyjazdem). Przyjazd pojazdem: gracz
jest w budynku z czynnością 7 (jedzie — niewidoczny, bez sterowania, pozycja =
pojazd), pojazd to encja `kind` 4 (`held`: 1 auto, 2 rower, 3 taksówka, 4
tramwaj, 5 radiowóz, 6 wóz strażacki — oba z niczyim dojazdem niezwiązane; `flags` kierunek + ruch). Poza budynkiem (w domu /
w drodze) serwer nie wysyła snapshotów; przyjazd = znowu snapshoty, postać
przed budynkiem.

### 31 `Calendar` (S→C), 32 `CalendarBook` (C→S)

Kalendarz zarządu na dziś dla konta komputera, przy którym siedzi odbiorca
(odblokowanego; co 1 s): `mine_start u16` (minuta albo 0xFFFF), `mine_topic
u8`, n u8, n × {`start u16`, `state u8`: 0 wolne, 1 zajęte, 2 moje, 3 minione}.
`CalendarBook`: token u32, start u16, topic u8 (1 podwyżka, 2 pomysł, 3 skarga,
4 luźna rozmowa; 0 = odwołaj). Rezerwacja zastępuje poprzednią tego konta.

Uprawnienie **8 (zarząd)** w `self_access`: dostaje je osoba z umówionym
spotkaniem od 10 min przed do 10 min po jego początku; drzwi zarządu (kafel
`board_door`, `access: "board"`, wyjście w dół wolne).

### 33 `Dialog` (S→C), 34 `DialogAnswer` (C→S)

Rozmowa z NPC (spotkanie z zarządem): `id u8` (0 = zamknij okno), `npc u16`,
`text` str16, n u8 (≤ 4) × `option` str16; ponawiane co 1 s, dopóki trwa.
`DialogAnswer`: token u32, id u8, choice u8 — odpowiedzi na nieaktualne `id`
są ignorowane. Odpowiedzi NPC idą jako `Say`.

### 35 `LunchMenu` (S→C), 36 `LunchOrder` (C→S)

Aplikacja obiadowa dla konta komputera, przy którym siedzi odbiorca (co 1 s):
`state u8` (0 można zamawiać, 1 zamówione, 2 czeka na recepcji, 3 poza
godzinami 10–15), `dish u8`, `arrives u16` (minuta przyjazdu albo 0xFFFF), n u8
× {`kind u8`, `price u32` (grosze), `eta u8` (min), `name` str16, `restaurant`
str16}. `LunchOrder`: token u32, dish u8. Dania to przedmioty 28 pierogi, 29
pizza, 30 sushi, 31 schabowy, 32 sałatka, 33 kebab (tylko w rękach). Odbiór:
E przy NPC „Recepcja”.

### 37 `CompanyOffers`, 38 `CompanyPeople` (S→C), 39 `CompanyAction` (C→S)

Panel założyciela — tylko dla założyciela przy jego własnym (odblokowanym)
komputerze, co 1 s i po każdej akcji. `CompanyOffers` (w częściach): `name`
str16, `part u8`, `parts u8`, zestawy pytań (tylko część 0) n u8 × {`id`
str8, `name` str16}, stanowiska n u8 (≤ 16) × {`id u8`, `places u8`,
`department u8`, `set` str8, `title` str16, `description` str16}. `CompanyPeople`: n
u8 × kandydat {`player u16`, `offer u8`, `score u8`, `total u8`, `nick` str16},
m u8 × pracownik {`player u16`, `department u8`, `day u16` (dzień zatrudnienia),
`reprimands u8` (nagany za alkohol, 3 = zwolnienie), `nick` str16}.

`CompanyAction`: token u32, `action u8`, `target u16`, `value u8`, `text` str16.
Akcje: 1 załóż firmę (z portalu, `text` = nazwa 3–40 znaków), 2 zmień nazwę,
3 miejsca oferty `target` = `value` (0–5), 4 opis oferty `target` = `text`
(≤ 200 znaków), 5 zatrudnij kandydata `target`, 6 odrzuć, 7 zwolnij pracownika,
8 nowe stanowisko (`value` = dział 1–2, `text` = „nazwa\nzestaw\nopis”), 9 nazwa
stanowiska `target` = `text`, 10 dział = `value`, 11 zestaw pytań = `text`,
12 usuń stanowisko `target`
`target`. Akcje 2–7 tylko od założyciela przy jego komputerze; inne są
ignorowane. Dział 3 = Zarząd.

### 40 `Smoke` (S→C)

Dym papierosowy na piętrze odbiorcy (co 1 s, tylko w budynku): `floor u8`, n
u8 (≤ 64) × {`room u16`, `level u8` 1–255}. Pokoi spoza listy nie ma dymu.
Czujki dymu są w danych mapy (`room_defs.*.detector`), klient rysuje je sam.

### Szyfrowanie (zalogowani gracze)

Klucz sesji K (32 B, hex w polu `key` odpowiedzi logowania — tylko przez
HTTPS). Klucze: `enc = HMAC-SHA256(K, "startup-sim enc")`,
`mac = HMAC-SHA256(K, "startup-sim mac")`. Datagram:

    magic u16 | version u8 | 0xF0 | token u32              (sesja)
    magic u16 | version u8 | 0xF1 | bilet 32 B (surowy)    (Connect)
    | licznik u64 | AES-256-CBC(pakiet gry, PKCS#7) | MAC 16 B

IV = AES-256-ECB(enc, [kierunek u8 (1 do serwera, 2 do klienta), licznik u64
LE, 7 × 0]). MAC = pierwsze 16 B z HMAC-SHA256(mac, kierunek u8 ‖ wszystko
przed MAC). Liczniki rosną (od 1), każda strona odrzuca licznik widziany już
w oknie 64 ostatnich; licznik zaszyfrowanego Connect musi być większy niż
ostatni dla tego biletu. Bilet w środku Connect musi się zgadzać z tym w
nagłówku. Sesja z kluczem przyjmuje tylko pakiety `0xF0`; klient przyjmuje
jawnie tylko `Reject` i `Disconnect` (np. serwer po restarcie nie zna już
sesji). Wektory testowe: `server/tests/golden/sealed.json`.

### Logowanie (HTTPS, port gry + 1)

Poza UDP, JSON: `POST /api/register` i `/api/login` `{nick, password}`,
`/api/password` `{nick, password, new_password}`, `/api/refresh` `{refresh}`,
`/api/logout` `{refresh}` → `{ok, nick, ticket, refresh, character, key, error}`
(`character` = postać już istnieje). `GET /api/cert` — PEM własnego
certyfikatu serwera (404 przy prawdziwym certyfikacie); klient przypina go
przy pierwszym kontakcie i weryfikuje z nazwą `startup-sim`. `ticket` idzie
potem w `Connect`.

Raport awarii klienta: `POST /api/crash` `{version, os, cpu, gpu, started,
log}` (bez logowania; ciało ≤ 64 KB, z logu zostaje koniec ≤ 48 KB; 5 na
godzinę z jednego adresu, 60 na godzinę łącznie) → `{ok, id}` albo
`{ok: false, error}` (429 za dużo, 400 pusty). Serwer zapisuje go jako
`<katalog zapisu>/crashes/<data>-<id>.txt` (najnowsze 200).

### 54 `Departments` (S→C)

Działy firmy (z `server/data/recruitment.json`): n u8 (≤ 32) × {`id u8`,
`short` str8 (przy nicku, np. „IT”), `name` str8 (pełna nazwa)}. Serwer wysyła
go zaraz po `Welcome` i co 5 s (zgubiony wraca); klient trzyma listę w
`net/departments.gd` i z niej bierze nazwy działów wszędzie (etykiety,
tablica zadań, panel założyciela, portal). Id działu są te same we wszystkich
pakietach (`PlayerInfo`, `JobOffers`, `TaskBoard`, `CompanyOffers` …).

### 52 `Voice` (C→S), 53 `VoiceFrom` (S→C)

`Voice`: token u32, seq u16, whisper u8 (0 pokój, 1 szept), n u16 (≤ 800) ×
bajt ramki. `VoiceFrom`: speaker u16, seq u16, whisper u8, n u16 + ramka.
Ramka = 40 ms mowy, 16 kHz mono, IMA ADPCM: predyktor i16 LE, indeks kroku
u8, potem po 2 próbki na bajt (najpierw młodszy półbajt) — 640 próbek = 323
B. Serwer nie zagląda do środka: przekazuje ramkę wszystkim w tym samym
(piętro, pokój) albo — szept — tylko najbliższej osobie w promieniu 1,5
kafla. Limit: ~30 ramek/s na gracza (nadmiar przepada).

### 46 `TaskAction` (C→S), 47 `TaskBoard`, 48 `TaskDetail` (S→C)

Tablica zadań działu właściciela komputera (tylko przy odblokowanym
komputerze, jako właściciel). `TaskAction`: token u32, `nonce u16` (0 = bez
skutków, tylko odpowiedź), `action u8` (0 SYNC, 1 CREATE, 2 MOVE, 3 ASSIGN,
4 PRIORITY, 5 COMMENT, 6 DELETE, 7 EDIT), `task u16`, `arg u8` (kolumna 0–2 /
priorytet 0–2), `text` str16 (≤ 400 B: tytuł + "\n" + opis, nick albo
komentarz). Każda akcja (także SYNC) dostaje w odpowiedzi tablicę:
`TaskBoard` dept u8, `done u16` (ostatni zastosowany nonce — klient ponawia
akcję, dopóki go nie zobaczy), part u8, parts u8, members (u8 × str8, tylko
część 0), n u8 × {id u16, column u8, priority u8, comments u8, title str16
(≤ 80 B), author str8, assignee str8}. SYNC z `task` ≠ 0 (albo akcja na
karcie) dodaje `TaskDetail`: id u16, desc str16, n u8 (≤ 6, najnowsze) ×
{nick str8, text str16 (≤ 120 B)}.

### 49 `MailAction` (C→S), 50 `WorkMail`, 51 `MailState` (S→C)

Poczta służbowa właściciela komputera. `MailAction`: token u32, nonce u16,
`action u8` (0 SYNC, 1 SEND, 2 TRASH, 3 RESTORE, 4 EMPTY_TRASH), `id u16`
(SYNC: najnowszy znany; TRASH / RESTORE: który), `to` str8, `subject` str16
(≤ 80 B), `body` str16 (≤ 400 B). SYNC odsyła do 6 nowszych maili jako
`WorkMail` (id u16, from str8, to str8, subject, body, day u16, minute u16);
każda akcja kończy się `MailState` (done u16, ids: u8 × u16, trashed: u8 ×
u16). Skrzynki są w pamięci serwera (po nicku, do 40 maili).

### 45 `Sound` (S→C)

n u8 (≤ 64) × {`kind u8`, `x i32`, `y i32`} (sub-piksele). Dźwięki zdarzeń z
tego ticku na piętrze odbiorcy w promieniu 28 kafli; klient gra je w miejscu
zdarzenia (`SOUND_FILES` w `net/protocol.gd`). Nie są potwierdzane — zgubiony
dźwięk po prostu przepada.
Rodzaje: 1 ekspres, 2 kasa, 3 bramka sklepu, 4 winda, 5 zamek kabiny, 6
włącznik, 7 spłuczka, 8 kran, 9 zapalniczka, 10 zmywarka, 11 lodówka, 12
szafka, 13 podniesienie, 14 upuszczenie, 15 jedzenie, 16 picie, 17 gwizdek,
18 beknięcie (po alkoholu; klient gra je ~1 s później, po łyku), 19 wymioty,
20 cios, 21 dźgnięcie, 22 sikanie, 23 kupa.

### 56 `HrAction` (C→S), 57 `HrInfo` (S→C)

Aplikacja Kadry. `HrAction`: token u32, `action u8` (1 pokaż, 2 wniosek
urlopowy na dzień `arg`, 3 anuluj wniosek `arg`), `arg u16`; serwer zawsze
odpowiada `HrInfo` (tylko z umową). `HrInfo`: `title` str16, `department u8`,
`form u8` (`employment`, 0 = sprzed widełek), `salary u32` (zł brutto /
mies.), `pay_rate u32` (gr / h), `start_day u16`, `today u16` (dni gracza),
`reprimands u8`, `leave_days u8`, `worked u8` (przepracowane dni do kolejnego
dnia urlopu, z 5), aneksy n u8 (≤ 10, najnowsze na końcu) × {`day u16`, `text`
str16}, wnioski n u8 (≤ 10) × {`id u8`, `day u16`, `status u8` (1
zaakceptowany, 2 odrzucony, 3 anulowany, 4 wykorzystany)}.

### 59 `Roll` (C→S)

token u32, `quality u8` (0..100) — wynik mini-gry skręcania; serwer zabiera
porcję z zapłaconego tytoniu (43) w rękach i daje skręt (44) z jakością w
nazwie (< 30 rozsypie się przy paleniu). Przedmioty: 45 klucz do magazynku
(daje uprawnienie 16 — drzwi „storeroom_door”), 46 Coca-Cola, 47 ciastka z
magazynu, 48 Apap, 49 węgiel aktywny, 50 witamina C, 51 plaster. Apteczka i
regały magazynku: `Dialog` 255.

### 58 `Media` (S→C)

Telewizory i boomboxy, do wszystkich w budynku co 1 s i po każdej zmianie:
ekrany n u8 (≤ 8) × {`floor u8`, `x u8`, `y u8` (lewy kafel ekranu), `channel
u8` (0 wyłączony, 1 kreskówki, 2 wiadomości, 3 pogoda, 4 mecz, 5 przyroda),
`started u32` (tick serwera, od którego leci)}, muzyka n u8 (≤ 8) × {`track
u8` (1 disco polo, 2 lo-fi, 3 techno, 4 szanty — pliki `boombox_<n>`),
`started u32`, `floor u8`, `x i32`, `y i32` (sub-piksele), `holder u16` (gracz
z boomboxem; 0 = stoi na podłodze)}. Klient liczy, ile minęło od `started`,
więc wszyscy widzą i słyszą ten sam moment. Wybór kanału / utworu: F z pilotem
(41) / boomboxem (42) w rękach → `Dialog` 253 / 254.

### 55 `Action` (C→S)

token u32, `action u8`: 1 menu psot (R) — serwer odpowiada `Dialog` o id 250
z tym, co da się tu zrobić (nasikać na podłogę, zesrać się na podłogę,
nasikać do ekspresu w zasięgu, nasikać do kubka osoby obok; ostatnia opcja =
nic); 2 atak (X) — cios pięścią albo, z nożem w rękach, dźgnięcie najbliższej
osoby w zasięgu 1,5 kafla. Odpowiedź na menu to zwykły `DialogAnswer`. Szafka
w kuchni (E z wolnymi rękami) to `Dialog` o id 251 (kubek, nóż, zamknij);
id 200–249 to pytanie o naganę po alkomacie, 252 — umowa w HR (kwota niższa niż uzgodniona; 0 podpisuję, 1 rezygnuję — HR odprowadza na portiernię, przepustka wraca, potem portal).

### 44 `SkipWait` (C→S)

token u32. „Pomiń czekanie” — tylko w domu / w drodze. Gdy poprosili wszyscy
gracze (i wszyscy są w domu lub w drodze), zegar pędzi aż do przyjazdu;
stan w `Clock::skip` (0 nie, 1 czekam na innych, 2 czas pędzi).

### 42 `Fridge` (S→C), 43 `FridgeAction` (C→S)

Lodówka w aneksie (E przy niej, i po każdej zmianie): n u8 (≤ 16) × {`kind
u8`, `label` str16 — np. „Kanapka z szynką (Ola)”}, `milk u8` (porcje),
`water u8`, `juice u8` (darmowe). `FridgeAction`: token u32, `action u8` (1
weź `arg`-tą rzecz, 2 włóż to, co w rękach — karton mleka = +10 porcji, 3 weź
wodę, 4 weź sok, 5 dolej mleka do kawy w rękach), `arg u8`. Tylko stojąc przy
lodówce. Szafka z kubkami i zmywarka działają przez E (odpowiedzi jako `Say`).

### 41 `Lights` (S→C)

Lampy zapalone na piętrze odbiorcy (co 1 s, tylko w budynku): `floor u8`, n u8
(≤ 64) × `room u16`. Włącza się je E przy włączniku (`room_defs.*.switch`, kafel
przy drzwiach, zasięg 1 kafel) — bez osobnego pakietu. Jasność pomieszczeń
liczy klient: pora dnia (`Clock.minute`), pogoda, `windows`, `light`
("switch" / "always"), `lit_by` (kabina → łazienka) i lampy.

### 27 `Shelf` (S→C), 28 `ShopTake` (C→S)

`Shelf` — odpowiedź na E przy półce sklepowej: shelf u8, title str16, n u8 (≤ 16),
n × {`kind u8`, `price u32` (grosze), `name` str16}. `ShopTake`: token u32,
shelf u8, kind u8 — weź jedną sztukę (serwer sprawdza zasięg półki); towar
trafia do ekwipunku jako niezapłacony (etykieta w `Inventory` z dopiskiem i
ceną). Płacenie: E przy NPC „Kasa” (odpowiedź jako `Say`). Wyjście ze sklepu
z niezapłaconym towarem: `Say` z alarmem od kasy, towar znika.

Rodzaje przedmiotów sklepowych (`held`, `Inventory.kind`): 10 kanapka z serem,
11 z szynką, 12 wrap wege, 13 hamburger, 14 frytki, 15 drożdżówka, 16 batonik,
17 chipsy, 18 woda, 19 energetyk, 20 sok, 21 piwo, 22 wino, 23 papierosy,
38 małpka (setka wódki; nazwa z serwera, więc starszy klient pokaże ją bez ikony).
39 alkomat (nie ze sklepu — dostaje go Zarząd przy założeniu firmy), 40 nóż
kuchenny (z szafki w kuchni).

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

- **44** — skręty i zapasy: `Roll` (59, C→S), przedmioty 43–51, uprawnienie 16 (klucz do magazynku), `Dialog` 255 (apteczka / magazynek).
- **43** — telewizor i boombox: `Media` (58, S→C), przedmioty 41 pilot, 42 boombox, `Dialog` 253 (kanały), 254 (utwory).
- **42** — Kadry: `HrAction` (56, C→S), `HrInfo` (57, S→C); `Clock` + `leave u8` na końcu (dzień urlopu: w domu).
- **41** — widełki i umowa: `JobOffers` oferta + `salary_min u32`, `salary_max u32` (zł brutto / mies., po `vacancies`); `Apply` + `salary u32`, `form u8` (1 umowa o pracę, 2 B2B, 3 umowa zlecenie — tylko student < 26 lat), `student u8` (po `motivation`); `Dialog` 252 = umowa w HR (0 podpisuję, 1 rezygnuję).
- **40** — psoty i bójki: `Stats` + `bowels`, `health` (po `alcohol`); czynności 11 nokaut, 12 cios, 13 sikanie, 14 kucanie; dźwięki 20 cios, 21 dźgnięcie, 22 sikanie, 23 kupa; kałuża `held` 2 = kupa; przedmiot 40 nóż; wygląd NPC 7 = kasjer; `Action` (55, C→S); `Dialog` 250 (menu R) i 251 (szafka).
- **39** — upojenie alkoholem: `Stats` + `alcohol` (po `hygiene`); `Snapshot` + `self_drunk` (po `self_slow`, zataczanie w symulacji — też w wektorach golden ruchu); flagi encji gracza bity 4–5 = upojenie; czynności 9 wymioty, 10 odsypianie; dźwięki 18 beknięcie, 19 wymioty; kałuża `held` 1 = wymiociny; `CompanyPeople` pracownik + `reprimands u8` (przed `nick`); przedmiot 39 alkomat (pytanie o naganę to zwykły `Dialog`).
- **38** — kałuża po wpadce: encja `kind` 6 (`flags`, `held`, `activity` = 0), id z puli od `0xE000`; widoczna jak przedmioty w pokoju; ściera ją sprzątaczka, inaczej znika o 22:00.
- **37** — osobne działy: pakiet `Departments` (54, S→C) z listą działów; działy 4–10 (Mobile, DevOps, AI, Finanse, Sales, Marketing, Obsługa klienta), dział 1 nazywa się „Produkt / IT”.
- **36** — dwie windy: `Doors` kończy się listą wind `n u8` (≤ 16) × {`floor u8`, `target u8` (255 = stoi), `moving u8`} zamiast jednej trójki `lift_*`.
- **35** — stanowiska firmy: `CompanyOffers` w częściach, z działem, zestawem pytań i listą zestawów; `CompanyAction` 8–12 (dodaj / nazwa / dział / zestaw / usuń stanowisko).
- **34** (uzup.) — `Reject` 7 = nick zajęty (konto, zapisana postać albo ktoś w grze), 8 = e-mail postaci zajęty.
- **34** — szyfrowanie: pakiety `0xF0` (sesja) i `0xF1` (Connect) dla zalogowanych; pakiet gry ≤ 1152 B (datagram ≤ 1200 B); klucz sesji w odpowiedzi logowania (`key`).
- **33** — konta: `Connect` + `ticket` str16 (≤ 64 B, z logowania HTTPS; pusty = gość); `Reject` 5 (bilet wygasł), 6 (serwer wymaga konta), 7 (nick ma konto).
- **32** — czat głosowy: `Voice` (52, C→S), `VoiceFrom` (53, S→C).
- **31** — firmowy komputer: `TaskAction` (46, C→S), `TaskBoard` (47), `TaskDetail` (48), `MailAction` (49, C→S), `WorkMail` (50), `MailState` (51).
- **30** — dźwięki: `Sound` (45, S→C): n u8 (≤ 64) × {kind u8, x i32, y i32} — zdarzenia słyszalne na piętrze odbiorcy w promieniu 28 kafli (1 ekspres, 2 kasa, 3 bramka sklepu, 4 winda, 5 zamek kabiny, 6 włącznik, 7 spłuczka, 8 kran, 9 zapalniczka, 10 zmywarka, 11 lodówka, 12 szafka, 13 podniesienie, 14 upuszczenie, 15 jedzenie, 16 picie, 17 gwizdek).
- **29** — `SkipWait` (44, C→S: token u32) — „Pomiń czekanie” w domu; `Clock` + `skip` u8 (0 nie, 1 poproszono, 2 czas pędzi); aktywność 8 = zatrzymany (ochrona / policja).
- **28** — aneks kuchenny: `Fridge` (42), `FridgeAction` (43); przedmioty 35 kubek (czysty), 36 mleko (karton), 37 kawa z mlekiem; 34 = brudny kubek.
- **27** — światło: pakiet `Lights` (41); w mapie `room_defs.*.light` / `switch` / `lit_by` / `windows`.
- **26** — dym i straż: `Clock` + `alarm`, pakiet `Smoke` (40), wygląd NPC 6 (strażak), pojazd 6 (wóz strażacki).
- **25** — kubki i sprzątaczka: przedmiot 34 = pusty kubek, wygląd NPC 5 (sprzątaczka); bez nowych pakietów.
- **24** — ochrona i policja: wygląd NPC 3 (ochroniarz) i 4 (policjant), pojazd 5 (radiowóz); bez nowych pakietów.
- **23** — panel założyciela: `Clock` + `company`, `founded`; `CompanyOffers`, `CompanyPeople`, `CompanyAction`; dział 3 (Zarząd).
- **22** — wakaty: `JobOffers` + `vacancies u8` po `applied` (wolne miejsca; stanowiska z 0 portal ukrywa, chyba że gracz już aplikował).
- **21** — obiady: `LunchMenu`, `LunchOrder`, przedmioty 28–33.
- **20** — słodycze: encja tacy (`kind` 5: `held` = słodycz, `activity` = liczba sztuk), przedmioty 25 pączek, 26 ciastko, 27 sernik, `Stats.flags` bit 1 = rozstrój żołądka.
- **19** — zarząd: `Calendar`, `CalendarBook`, `Dialog`, `DialogAnswer`, uprawnienie 8 (drzwi zarządu).
- **18** — pogoda: `Clock` + `weather`, flaga parasola (bit 3 u graczy), przedmiot 24 = parasol.
- **17** — dojazd: `Clock` + `mode`, `depart`, `money`; `CommuteChoice`; encja pojazdu; czynność 7 (jedzie).
- **16** — zegar i dni: `Clock`.
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
