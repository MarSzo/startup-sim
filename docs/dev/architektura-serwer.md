# Architektura — serwer

*Architektura: [przegląd](architektura.md) · [serwer](architektura-serwer.md) · [klient](architektura-klient.md) · [protokół](protokol.md)*

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

**Tick** (`Server::tick`, `server/mod.rs` — każda faza to osobna metoda):
1. Zegar gry (`tick_clock`), potem timeouty (`drop_timed_out`): gracze bez
   pakietów > 5 s → `Disconnect(timeout)` i usunięcie.
2. Ruch (`simulate_players`, `movement.rs`): z kolejki inputów każdego gracza
   max 6 kroków `sim::step` (średnio 3 = 60 Hz / 20 Hz) na jego `Body`;
   aktualizacja pokoju, potrzeb, pogody i flag. Skutki zbierane w `Steps`
   i obsługiwane po pętli (`react_to_steps`: bramka sklepu, świadek w
   łazience, gotowa / zimna kawa, dym).
3. Klawisz E (`handle_interactions`, `interact.rs`), potem zachowanie świata:
   windy, pojazdy, policja, sprzątaczka, spotkania, obiady, firma, NPC;
   zdarzenia NPC (`apply_npc_events`).
4. Wysyłka (`send_updates`, `snapshot.rs`): grupowanie encji po
   `(floor, room)`; odbiorca dostaje encje swojego pokoju i pokoi z `see`
   (np. portiernia ↔ hol). Dla każdego gracza snapshot (fragmentowany
   ≤ 1200 B) + `PlayerInfo` dla nowych encji + okresowe ekrany; potem mowa
   (`Say` z kolejki `says`) i zegar.

Nowa funkcja gry = nowy moduł w `server/` (`impl Server`): stan jako pole
`Server`, `tick_*` wołane z `Server::tick`, obsługa pakietu w
`session::handle_datagram`; czyste reguły i dane w module domenowym
(`src/<funkcja>.rs`), testowalne bez sieci.

**Przestrzeń id encji** (`u16`): gracze `1..0xE000`, przedmioty na podłodze /
laptopy / pojazdy / taca `0xE000..0xF000` (max 1024 przedmiotów na podłodze —
nadmiar najstarszych niczyich znika), NPC od `0xF000`.

**Statystyki** co 5 s (`--stats-secs`): liczba ticków i `missed`, średni i
maksymalny czas ticka, gracze, max widocznych, transfer na klienta
(średnia/min/max), pakiety/s wejście/wyjście, pakiety wycięte przez symulator.

**NPC** (`npc.rs`) to encje serwera bez adresu sieciowego: w każdym ticku
robią 3 kroki tym samym `sim::step` (sterowane `nav::Walker`), są w
snapshotach jako `kind` NPC i mówią pakietem `Say`. Wciśnięcie E (zbocze) przez
gracza, który nie jedzie windą, trafia do NPC w promieniu 3,5 kafla —
najpierw do stojących na swoim stanowisku, potem do najbliższego
(`Npc::interact`). NPC zwracają zdarzenia (`Say`, `Grant`, `Revoke`),
które serwer wykonuje.

**Portier** (definicja w `floor0.json` → `npcs`): w spoczynku stoi w portierni.
Gość bez przepustki rozmawia z nim (E) → dostaje przepustkę gościa, a portier
prowadzi go na recepcję piętra 1 (schodami). Czeka, jeśli gościa nie ma
ani obok niego (4 kafle), ani dalej na trasie, ani w recepcji; przypomina co
6 s, po 30 s rezygnuje i odbiera przepustkę. Po dojściu mówi, że przepustka jest
ważna do końca dnia, i wraca. Prowadzi jedną osobę naraz.

**Recepcja** (piętro 1, za ladą) używa tej samej logiki odprowadzania: gościa
z przepustką prowadzi do HR (bez zmiany uprawnień, rezygnując nie odbiera
przepustki); osobę z kartą tylko wita. **HR** stoi za biurkiem: gościowi
„podpisuje umowę” — `Grant CARD` + `Revoke GUEST`; bez przepustki odsyła na
portiernię. Role (`npc::Role`) i ich kwestie są w `npc.rs`; wygląd idzie w
bitach 3–5 flag encji.

**Ekwipunek** (`inventory.rs`): 3 kieszenie na małe przedmioty (przepustka,
karta) i ręce na jeden dowolny (laptop i kawa tylko w rękach). `Body::access`
jest przeliczany z noszonych przedmiotów po każdej zmianie (`refresh`), więc
karta przekazana innemu graczowi przenosi dostęp. Portier daje przepustkę
(`npc::Event::Give`), przy rezygnacji ją zabiera (`Take`); HR zabiera
przepustkę i daje kartę (podpisaną imieniem i działem) oraz laptop — wymaga
wolnych rąk. Przedmioty na podłodze (`Dropped`) są encjami `kind` 2 w
snapshotach; E podnosi najbliższy w zasięgu 1,25 kafla (po NPC i ekspresie).
Gdy przedmiot się nie mieści, ląduje na podłodze pod nogami.

**Kabiny** (`stalls.rs`): kabina to mały pokój (typ `stall`, `see` = łazienka),
pole drzwi należy do kabiny — interest management sam ukrywa osobę w środku.
Zamknięcie ustawia w mapie nakładkę `closed` (`Map::set_closed`), którą
sprawdza `Map::blocks`, czyli wspólna reguła kolizji; klient trzyma tę samą
nakładkę w `MapData.closed` (pakiet `Doors`). Mapy są przez to zmienne po
stronie serwera (`Building::floor_mut`). Zamek zwalnia się, gdy zamykający
opuści pokój kabiny albo grę; nie można zamknąć, gdy ktoś stoi w drzwiach.

**Zegar** (`clock.rs`): czas w decysekundach gry od północy; w dzień 6 ds na
tick (1 h gry = 5 min), w nocy 240 ds na tick (8 h w 1 min). `Clock::tick`
zatrzymuje się dokładnie na 22:00 i 6:00 i zgłasza `Transition`. Serwer:
wieczór → każdy `Stage::Working` idzie do `Stage::Home { arrive_at: None }`
(koniec sesji komputera, odpoczynku; wypłata `PAY_PER_MIN` za `worked_ds`);
poranek → `day += 1` wszystkim, a domownicy dostają losowy `arrive_at`
(7:00–10:00); o tej minucie `arrive()` stawia postać przed budynkiem. Wszystko
poza `Working` (portal, dom) jest poza światem: bez snapshotów i bez udziału w
symulacji. `--start-time hh:mm`, `--time-scale N` do testów.

**Wakaty**: `Server::vacancies` (oferta → wolne miejsca, start z
`recruitment.json`: `vacancies`), +1 losowo co rano (maks. 3). Zdana rozmowa
przy braku miejsca = mail „obsadzone”; zajęcie ostatniego miejsca
(`take_vacancy`) kończy rekrutację pozostałym (`position_filled_mail`).
`Player::position` zwalnia miejsce przy wyjściu z gry.

**Ochrona i policja** (`security.rs`, `npc.rs`): NPC ma stan `Chasing` —
co 1 s nowa ścieżka (`Walker::to`) do bieżącego kafla celu, 4 kroki na tick,
`Event::Caught` w promieniu 1,5 kafla, `Event::Escaped` po czasie (ochrona
20 s, policja 3 min) albo gdy cel zniknie ze świata. Wyjście ze sklepu z
towarem `unpaid` → `Player::thefts_today` + pościg ochroniarza. `call_police`
dodaje `Vehicle::police` (parkuje przy wejściu) i `PoliceCall`; `tick_police`
wypuszcza policjanta (`Npc::police`, id od `NPC_ID_BASE + 0xF00`), a gdy ten
wróci do auta — usuwa NPC i odsyła radiowóz (`Vehicle::leave`).

**Wygląd** (klient): `ui/ink_ui.gd` — czcionka (FontFile z
`fonts/PatrickHand-Regular.ttf`), `fs()` powiększa rozmiary (min. 18), `box(kind)`
buduje `StyleBoxTexture` 9-slice z obrazka 64×64 rysowanego w kodzie (ziarno,
falujący kontur tuszu, postrzępione krawędzie, cień; krawędzie kafelkowane),
`button()`, `label()`, `draw_bar()`, `draw_disc()`. `main.gd` ustawia czcionkę i
motyw globalnie. Świat: `game/mood.gdshader` na `CanvasLayer` 4 (Sobel na
jasności = tusz, gradacja kolorów, ziarno, winieta); teksty świata na
`CanvasLayer` 6 z `follow_viewport_enabled` (`PlayerView.label_root`,
`MapView.labels`). Okno skaluje się w trybie `canvas_items`. Warstwy: świat
(0), efekt tuszu (4), dym `SmokeView` (5, `follow_viewport`), pogoda (6),
teksty świata (7), HUD (11+). Postać (`game/player_view.gd`) rysowana wektorowo
(`_limb`, `_blob`, `_shape` z konturem `OL`).

**Światło** (`lights.rs`, klient `game/light_view.gd`): serwer trzyma zbiór
zapalonych lamp (`Lights`), włączniki z mapy (`switches`), E w zasięgu
przełącza; wieczorem gasi wszystko; `Lights` co 1 s. `LightView` (warstwa
świata, pod dymem) rysuje ciemność na kaflach każdego pomieszczenia z
`light_of` (pora dnia × pogoda × okna / lampa) i tabliczki włączników;
`CanvasModulate` zostawia tylko lekki odcień nieba.

**Mapa** (`map/map_painter.gd`): rysunek wektorowy (podłogi wg typu kafla,
ściany 3/4 z tynkiem i konturem, okna tam, gdzie ściana dzieli pomieszczenie
z `windows` od zewnątrz, drzwi, meble jako spójne grupy kafli), wykonany raz
w `SubViewport` (`UPDATE_ONCE`, 3×, MSAA) i pokazany jako tekstura.

**Aneks kuchenny** (`kitchen.rs`, `server/kitchen.rs`): `Kitchen` (pozycje
szafki / zlewu / zmywarki / lodówki z mapy, czyste kubki w szafce, zmywarka:
brudne, umyte, koniec cyklu; lodówka: rzeczy, mleko, darmowe napoje).
`use_kitchen` bierze najbliższą rzecz aneksu, ustępując ekspresowi i
bliższym miejscom (`spots`); ekspres wymaga `CUP` w rękach; zlew zamienia
`EMPTY_CUP` → `CUP`. `FridgeAction` → `handle_fridge_action`. Rano `restock`,
sprzątaczka `cleaner_load`, wyjście gracza `return_mugs_of`.

**Menu** (klient): `ui/title_screen.gd` (ekran tytułowy), `ui/pause_menu.gd`
(Esc — `main.gd::_input`, jeśli `game.window_open()` jest fałszem),
`ui/settings_panel.gd` + `ui/settings.gd` (`user://settings.cfg`).

**Balkon**: `RoomDef::below` (nazwy pomieszczeń piętra niżej) →
`Building::below(floor, room)`; snapshot odbiorcy na balkonie zawiera też
encje z tych pomieszczeń, a mowa stamtąd też do niego dociera. Encje nie mają
piętra, ale siatki pięter się pokrywają, więc klient rysuje je na miejscu; pod
widokiem piętra pokazuje przyciemniony widok piętra niżej (`room_below`,
`_update_below_view`), a „~” nad parterem jest przezroczyste.

**Dym i straż** (`fire.rs`): `Smoke` trzyma ilość dymu na (piętro, pokój);
stężenie = ilość / liczba kafli pokoju. `puff` (każdy tick palenia w środku),
`tick` co `DRIFT_EVERY` przelewa przez każde przejście między pokojami
(`Map::room_adjacency`) `różnica × DOORWAY` (nie więcej niż wyrównanie;
pokoje otwarte pochłaniają), co `DECAY_EVERY` zanika. `tick_smoke_and_alarm`:
skargi, czujka (`RoomDef::detector`) ≥ `ALARM` → `Alarm` + `Vehicle::fire_engine`;
po zaparkowaniu `Npc::firefighter` idzie (`go_to`) na miejsce palacza, sprawdza
`CHECK_TICKS`, `clear`, kara i wpis na #ogólny, wraca; przy wozie — koniec.
`Clock.alarm` i pakiet `Smoke` (co 1 s) dla klienta; `SmokeView` rysuje mgłę i
czujki.

**Sprzątaczka** (`cleaning.rs`): pusty kubek (`EMPTY_CUP`) powstaje po
wypiciu / wystygnięciu kawy; umywalka i ekspres go zabierają. Od
`Config::cleaning_at` + losowo do `cleaning_spread` minut (co dzień nowa pora; domyślnie 15:00–16:00, `--cleaning-at` = dokładnie) `tick_cleaning` prowadzi
`Round`: gdy sprzątaczka (`Role::Cleaner`) jest bezczynna, zbiera kubki
(`dropped`) w zasięgu, po `WIPE_TICKS` wysyła ją (`Npc::go_to` → stan `Errand`)
do najbliższego następnego (najpierw jej piętro); nieosiągalne pomija. Koniec:
podsumowanie, przy `DAY_COMPLAINT` wpis na #ogólny (`post_system`), powrót do
zaplecza. Raz na dzień świata (`round_day`).

**Firma** (`company.rs`): `Server::company` (nazwa z pierwszej oferty
`recruitment.json`, `founder`, własne opisy, `candidates`, `hired_on`).
`found_company` (akcja z portalu) robi z gracza pracownika działu 3 przy stole
zarządu. Zdana rozmowa przy założycielu w grze → `Candidate` (+ `Desk::awaiting`
blokuje dalsze aplikacje), inaczej od razu `hire`; `tick_company` zatrudnia po
`DECISION_MINUTES`. Panel = `company_packets` wysyłane założycielowi, gdy
`calendar_account` jego sesji to on sam (razem z pakietami komputera).
`fire` kończy sesję komputera, zabiera przedmioty i pojazdy gracza i wraca go
na portal (`Stage::Portal`, nowe id maili od 100); najpierw idzie `Clock`, po
którym klient czyści skrzynkę.

**Obiady** (`lunch.rs`): menu = produkty sklepu (rodzaje 28–33) spoza półek;
`Order { owner, dish, arrives, delivered }` w `Server::lunch_orders`. Zamawia
się dla konta komputera (jak kalendarz), płatność od razu; `tick_lunch`
oznacza dostarczone i recepcja mówi zamawiającemu. E przy NPC recepcji z
czekającym obiadem → `pick_up_lunch` (zamiast zwykłej rozmowy).

**Słodycze** (`treats.rs`): rano `schedule_treats` losuje 1–2 godziny tac
(9–16); `put_tray` stawia `Tray` (encja `kind` 5) i ogłasza na #ogólny przez
`Messenger::post_system` od HR. E w zasięgu stołu → `take_treat`. Słodycze to
produkty sklepu z ceną 0 (poza półkami) — jedzenie działa jak dla towarów.
Owoce z misy mają `Item::stale` z szansą `stale_fruit_percent`; zjedzenie →
`Needs::upset_stomach` (pęcherz min. 70, +1 pkt/s aż do toalety).

**Kałuże** (`server/puddles.rs`): `needs::Event::Accident` trafia do
`Steps::accidents` (piętro, pozycja), `react_to_steps` → `leave_puddle`
kładzie `Puddle` (encja `kind` 6, uchwyt z `alloc_handle`, najwyżej 256 —
nadmiar wysycha od najstarszej). Sprzątaczka w obchodzie (`tick_cleaning`)
chodzi do kubków i kałuż, kałużę w zasięgu ściera z `lines::PUDDLE`;
wieczorne przejście zegara (22:00) czyści resztę `Server::puddles`; bez zapisu w bazie. Klient: `game/puddle_view.gd` na
własnej warstwie między mapą a `world` (pod ludźmi i przedmiotami).

**Zarząd** (`board.rs`): `Meeting { day, start, owner, topic, state }` w
`Server::meetings`. Kalendarz (pakiet `Calendar`) i rezerwacje idą przez konto
właściciela komputera, przy którym siedzi gracz. Co tick `tick_meetings`
ustawia `Body::access` = przedmioty + `BOARD`, jeśli trwa okno wejścia na jego
spotkanie (reguła kolizji jak bramki, więc klient to przewiduje), oznacza
przepadłe spotkania i kończy rozmowę po wyjściu z sali. NPC `Ceo` / `CoFounder`
zwracają `npc::Event::Meeting`; serwer prowadzi `Talk` (kroki
`board::steps`, pakiety `Dialog`) i liczy wynik (`pay_rate` przy podwyżce,
`Messenger::post_system` z pochwałą na #ogólny, stres).

**Pogoda** (`weather.rs`): łańcuch Markowa (wagi przejść), zmiana co 1–3 h
gry. Pokoje mają w mapie `outdoor: true` (na zewnątrz, parkingi, strefa
palenia); gracz w takim pokoju co tick dostaje `outdoor_effect` (deszcz i burza
moczą, chyba że nosi parasol; słońce odpręża). Dojazd: korki dla auta,
przemoczenie pieszo / rowerem. Klient: `weather_fx.gd` (efekty ekranowe tylko
na zewnątrz), odcień świata = pora dnia × pogoda (w środku słabiej).
`--weather rain` itd. ustala stałą pogodę do testów.

**Dojazd** (`commute.rs`): rano gracz w domu dostaje `depart_at`; do tej
minuty wybiera `commute_mode` (`CommuteChoice`). Wyjazd: opłata (brak
pieniędzy → pieszo), `arrive_at` = wyjazd + czas (+ korki dla auta). Przyjazd:
pieszo — od razu na chodnik; inaczej `Vehicle` z listą punktów trasy (ulica →
parking / stojak / krawężnik / przystanek), gracz jest `Working` z `riding`
(pozycja = pojazd, inputy potwierdzane, ale ignorowane, ukryty przed innymi).
Na przystanku `VehicleEvent::Arrived` → wysiada, efekty na potrzeby,
spóźnienie po 9:00. Auto i rower zostają zaparkowane do wieczora (znikają
przy `go_home`), taksówka i tramwaj odjeżdżają (`Gone`).
Stanowiska (`company::Position`, `server/positions.rs`): lista stanowisk
naszej firmy w `Server::positions` (start = oferty z `hiring` z pliku; potem
zmienia je założyciel; zapis w `World::positions`). Pytania są w zestawach
(`recruitment.json` → `question_sets`), stanowisko wskazuje zestaw;
`Recruitment::start(set, …)`. Oferty innych firm zostają w pliku.
Konta (`auth.rs`, `http.rs`): tabele `accounts` (nick bez rozróżniania
wielkości liter, skrót Argon2id) i `refresh_tokens` (SHA-256 tokenu) w tym
samym pliku SQLite; bilety w pamięci (`Auth` = `Arc<Mutex>` współdzielony
przez wątek HTTPS i pętlę gry, która tylko `redeem`-uje bilet z `Connect`).
API: `axum` + `axum-server` (rustls) na własnym wątku z runtime `tokio`
(current_thread), Argon2 w `spawn_blocking`. Certyfikat: `--tls-cert/--tls-key`
albo własny z `rcgen` w `<save>/tls/`. Klient: `net/auth_client.gd`
(HTTPRequest, przypinanie certyfikatu w `user://known_servers.cfg`, token
„zapamiętaj mnie” w `user://auth.cfg`), `ui/login_screen.gd`; `main.gd`
odświeża wygasły bilet przy ponownym łączeniu.
Szyfrowanie (`crypto.rs`, klient `net/seal.gd`): AES-256-CBC + HMAC-SHA256
(RustCrypto `aes`/`cbc`/`hmac`; w Godocie natywne `AESContext` /
`Crypto.hmac_digest`), `Keys::seal/open`, `ReplayWindow`. Serwer:
`handle_datagram` otwiera `0xF0` (klucz z `Player::crypto` po tokenie) i
`0xF1` (klucz z biletu w `Auth`, licznik przez `accept_connect`), a
`Server::send` szyfruje wszystko dla gracza z kluczem (po `by_addr`). Klient:
`net_client.gd` szyfruje w `send()` i otwiera w `_poll()`. Parytet bajtów:
golden `sealed.json`.
Zapis gry (`persist.rs`, `server/save.rs`): SQLite (`rusqlite`, WAL), tabele
`world` (jeden JSON świata) i `characters` (nick → JSON). `Store` ma wątek
zapisu: pętla gry co `SAVE_TICKS` (albo po `save_soon`) serializuje zrzut i
wysyła tylko zmienione wiersze kanałem; wątek zapisuje je w jednej
transakcji i raz na dobę robi `VACUUM INTO` do `backups/`. Ctrl+C / SIGTERM
(`ctrlc`) ustawia `server::STOP` → `shutdown()` = ostatni zrzut + `flush`.
Identyfikacja po nicku (`restore` przy Connect, `remember_leaving` przy
wyjściu); przedmioty, laptopy, założyciel i zatrudnieni są w zapisie po
nicku, bo id sesji się zmieniają. Z zapisem świat jest trwały (wyjście nie
zwalnia etatu, biurka ani firmy); bez niego (`--no-save`, testy) — jak dawniej.
Firmowy komputer (`tasks.rs`, `workmail.rs`, `server/office.rs`): tablice
zadań per dział (`Boards`) i skrzynki po nicku (`PostOffice`); obsługa działa
jako właściciel odblokowanego komputera, przy którym siedzi gracz. Akcje mają
nonce (ostatni zastosowany w `Player::task_nonce` / `mail_nonce`), a każda
odpowiedź niesie pełny stan — klient ponawia, dopóki nie zobaczy swojego
nonce. `office_mail` wysyła maile systemowe (HR, kalendarz, obiady, tablica).
Głos (`server/voice.rs`): `Voice` jest przekazywany od razu jako `VoiceFrom`
— do graczy w tym samym (piętro, pokój) albo, szeptem, do jednej
najbliższej osoby w `WHISPER_RADIUS`; limit ramek per gracz (wiaderko
tokenów w tickach). Serwer nie dekoduje ani nie zapisuje głosu.
Dźwięki (`Server::sounds`): zdarzenia dopisują `(rodzaj, piętro, pozycja)`
(`sound()` dla miejsca gracza); `queue_sounds` wysyła `Sound` każdemu na tym
piętrze w promieniu `HEAR_RADIUS`. Resztę (kroki, otoczenie, muzykę) klient
wylicza sam.
Powrót przed 22:00 (`server/leave.rs`): E przy własnym pojeździe
(`Vehicle::depart` — odjazd ulicą) lub w `commute::home_spot` dla trybu,
dwa naciśnięcia w `CONFIRM_TICKS` → `go_home` (wypłata). Gdy wszyscy gracze
są w domu i nikt nie jedzie, `Clock::fast` = tempo nocne; gdy wszyscy w domu
/ w drodze wysłali `SkipWait`, `Clock::skip` = `SKIP_DS_PER_TICK` (zerowane
przy przyjeździe). Złapany gracz ma `held_until` (inputy potwierdzane,
ignorowane; aktywność `HELD`). Eskorta NPC po dojściu: `State::Lingering`.

**Sklep** (`shop.rs`): lista towarów (`PRODUCTS`: rodzaj przedmiotu, nazwa,
cena w groszach, efekt na potrzeby, liczba sztuk) i półek (prostokąty kafli
`shelf` na parterze + co na nich leży; `shop::check` pilnuje zgodności z mapą).
E przy półce → pakiet `Shelf`; `ShopTake` dodaje `Item { unpaid: true }`.
NPC `Cashier` zwraca `npc::Event::Checkout`, a serwer pobiera z `Player::money`
sumę niezapłaconych rzeczy (`Inventory::mark_paid`). Zmiana pokoju ze sklepu z
niezapłaconym towarem → `Inventory::remove_unpaid`, stres i alarm od kasy.
Zaliczka `shop::ADVANCE` przy `npc::Event::Contract`. Paczka papierosów to
przedmiot z `count` (20), zużywany przy popielniczce.

**Potrzeby** (`needs.rs`): `Needs` w stałym przecinku (10 000 jednostek na
punkt), zmiana co tick: głód 0→100 w 25 min, energia 100→0 w 35 min, toaleta
0→100 w 20 min; stres rośnie za każdą zaniedbaną potrzebę, a bez zaniedbań
powoli spada. Odpoczynek (`Rest`: sofa, toaleta, papieros) włącza się E przy
miejscu znalezionym w mapie po typie kafla (`find_spots`, zasięg 1,5 kafla) i
kończy ruchem, ponownym E albo sam (pusta toaleta, koniec papierosa). Kawa i
owoc działają przy użyciu (F). Progi dają jednorazowe ostrzeżenia w dymku;
toaleta na 100 = „wpadka” (stres +30, reset). `Needs::slow()` ustawia
`sim::Body::slow` — wolniejszy chód jest częścią wspólnej symulacji (Rust i
GDScript, wektory golden), a klient poznaje go ze snapshotu. Łazienki mają w
mapie `gender`; użycie „nie tej” to komentarz i trochę stresu.
`--needs-speed N` przyspiesza potrzeby do testów.

**Upojenie** (`needs.rs` + `drunk.rs` + `server/breath.rs`): `Needs::alcohol`
(trzeźwienie ~20 pkt / godz. gry). `drink_alcohol` przy użyciu napoju
(`items.rs` `use_held` → `after_drink`) zwraca `Event::Vomit` (≥ 75, raz do
spadku poniżej 40) albo `Event::PassOut` (≥ 100 po wymiotach): gracz stoi
(`held_until` + `held_activity` = 9 / 10), wymioty kładą kałużę `vomit` (encja
6, `held` 1), po zaśnięciu `sleep_it_off` w `simulate_players`. Bez zdarzenia:
dźwięk beknięcia. `Needs::stagger()` → `sim::Body::drunk` (zataczanie i wolny
chód w `sim::move_at`, przewidywane przez klienta), `drunk_tier()` → bity 4–5
flag encji. Bełkot: `drunk::slur(tekst, poziom, ziarno)` — deterministyczny
(`fastrand` z ziarnem z ticku i mówiącego) w `queue_says` (dymki graczy) i
`post_chat` (komunikator); głos zniekształca dopiero słuchacz
(`audio/voice.gd`, falujący `pitch_scale`). Alkomat (`breath.rs`): F z
przedmiotem 39 tylko dla działu Zarządu; najbliższy gracz w 2 kaflach, odczyt
`promille_milli`, powyżej `LIMIT_MILLI` zwykły `Dialog` o id 200–249
(`answer_reprimand` przed rozmowami z zarządem): nagana = `reprimands` (w
zapisie postaci), mail od Zarządu, przy 3 `fire` (`breath.rs::reprimand` —
też za nóż).

**Psoty i bójki** (`mischief.rs` + `server/actions.rs`, `fight.rs`,
`greetings.rs`): `Action` (R/X). R buduje listę `Deed` (podłoga: siku / kupa,
`PeeMachine` w zasięgu, `PeeCup` osoby z kawą w rękach) i wysyła ją jako
`Dialog` 250; `answer_mischief` (przed rozmowami z zarządem) wykonuje wybór:
`Needs::pee_now` / `poop_now` (trzeba mieć ≥ 15), chwila w miejscu
(`held_activity` PEEING / POOPING), kałuża `puddle::*`, `Machine::tainted` (3
kawy, płucze sprzątaczka na początku obchodu) albo `Item::tainted` w kubku;
wypicie skażonego: `drank_pee` — stres, 50% `throw_up`. Świadek w tym samym
pokoju reaguje. Szafka w kuchni: `Dialog` 251 (kubek / nóż, `Kitchen::knives`
2, rano znów). Atak: najbliższy gracz w 1,5 kafla, nie leżący; `Needs::hurt`
(pięść 10, nóż 35, odstęp 1 / 1,5 s), przy 0 nokaut na minutę (`knocked_out`,
budzi `simulate_players`, 30 zdrowia), `Player::assault` = ścigany: ochroniarz
(`caught_fighting`), za nóż `call_police` i nagana. Pięć papierosów pod rząd
(zapalony ≤ 30 s po poprzednim) — `throw_up`. `Needs::bowels` rośnie powoli i po
jedzeniu (połowa zjedzonego głodu); `Rest::Toilet` opróżnia pęcherz i jelita,
`Rest::Urinal` (pisuar) tylko pęcherz; 100 = kupa na podłodze. NPC:
`Role::Idler` (Paulina, zawsze siedzi), patrol ochroniarza (`NpcDef::patrol`,
`State::Patrolling`, 6 s w każdym punkcie; przy półce E łapie tylko ktoś tuż
obok), pani Wiesia wita wchodzących z wiatrołapu / parkingu (`greet_entering`,
raz na 10 min), kasjer pyta o parówkę, gdy podejdzie się do lady z towarem
(`tick_cashier`).

**Widełki i umowa** (`pay.rs` + `server/contract.rs`): oferty mają `salary`
[min, max] (`recruitment.json`, `Position::salary`, nowe stanowiska
`DEFAULT_RANGE`). `Apply` niesie oczekiwania i formę (`form_allowed`: zlecenie
tylko student < 26 lat), zapisane w `Desk::terms`; `deliver_replies` odmawia,
gdy oczekiwania > max. `hire` przenosi je do `Player::terms` (zapisywane z
postacią). HR (`Npc::interact`) zgłasza `ShowContract` → `show_contract`
losuje raz obniżkę (`CUT_PERCENT`, B2B +20%) i wysyła `Dialog` 252;
`answer_contract`: podpis = dawne zdarzenia HR (karta, laptop, `Contract`) —
`sign_contract` ustawia `salary`, `employment`, `pay_rate = hourly(...)`, zaliczka
tylko na umowie o pracę; rezygnacja = `Npc::see_out` (eskorta do `escort_to`
HR, czyli portierni), `SawOut` → `saw_out` (przepustka wraca, kwestia pani
Wiesi) i po 5 s `tick_to_portal` → `back_to_portal` (wspólne ze zwolnieniem).
Recepcja: `tick_reception` (9–13, raz dziennie, bez zamówionego obiadu); pani
Maria: `tick_maria` (każdemu w 3,5 kafla co minutę, między historiami 12 s,
`cleaning::lines::STORIES`).

**Kadry** (`hr.rs` + `server/hr.rs`): `Player::hr: HrFile` (zapisywana z
postacią) — `signed` otwiera ją przy umowie (`open_hr_file`: też
`--start-employed` i stare zapisy), `annex` przy podwyżce (`board.rs`, razem z
`salary`), `request`/`cancel` z aplikacji (`HrAction` → zawsze `HrInfo`; mail
od HR), `worked_a_day` w `go_home` (≥ 60 min), `morning` w porannym przejściu
zegara: dzień urlopu = brak `depart_at` (zostaje w domu), wypłata 8 h na
umowie o pracę, `Clock::leave`. Klient: `ui/office/hr_view.gd` (zakładki,
odpytuje co 3 s, gdy okno otwarte), `terminal_view.gd` + `shell.gd` (fikcyjna
powłoka, czysty tekst, testowana w `run_tests.gd`), `web_browser.gd`
(prawdziwe strony: `WebView` z GDExtension godot_wry, jeśli jest — patrz
[uruchomienie](uruchomienie.md#przeglądarka-w-grze); `computer_screen.gd`
pokazuje go tylko, gdy okno przeglądarki jest na wierzchu, bo natywny widok
leży nad całą grą; bez wtyczki — `OS.shell_open`, nigdy w trybie headless).

**Telewizor i boombox** (`media.rs` + `server/media.rs`): ekrany z kafli
„tv” (`find_screens`, pokój, na który patrzą), kanał i tick startu; boombox to
jeden utwór + tick startu, a jego miejsce liczy `boombox_at` (gracz z nim w
ekwipunku albo przedmiot na podłodze; zgubiony = cisza). F z pilotem /
boomboxem → `Dialog` 253 / 254 → `answer_media`. `tick_media` wysyła `Media`
co sekundę i po zmianie. `ensure_media_items` (start, poranek) kładzie
brakujący pilot / boombox w `places` mapy. Klient: `game/tv_view.gd` (kanały
rysowane z czasu od startu), muzyka w `game.gd` (`AudioStreamPlayer2D` na
szynie Music, za trzymającym, poprawka dryfu > 0,6 s), pętle z
`gen_sounds.py` (`boombox_1..4`).

**Zapasy i skręty** (`supplies.rs` + `server/supplies.rs`): `Supplies::find`
szuka kafli „key_hook”, „medicine_cabinet” i regałów pokoju „storage”;
`use_supplies` (E, przed rozmową z NPC) — haczyk (klucz tylko gdy
recepcjonistka nie `at_home`), apteczka / regały → `Dialog` 255 →
`answer_supplies` (`Stock` na dzień, `restock_supplies` rano, klucz wraca).
`access::KEY` z klucza w ekwipunku otwiera drzwi z `access: "key"` (też w
`map_data.gd` — parytet symulacji). `tick_lunch_break` wysyła recepcjonistkę do
aneksu 12:00–12:30. Skręty: klient (`ui/roll_game.gd`, punktacja w funkcjach
statycznych, testowana) wysyła `Roll` → `handle_roll` (porcja z tytoniu,
`Item::quality`); `smoke_roll` przy F.

**Czat, powiadomienia, barek, przechodzień**: `Say` ma `reach` (pokój /
szept tylko do `to` / całe piętro) — `queue_says` wybiera odbiorców;
`server/chat.rs` (`ChatSay`, `/s`, `/k`, limit) i pomocnicze `notify*`
(`Notice`), wołane m.in. z `office_mail` (każda poczta), tacy ze słodyczami,
nokautu, wymiotów, zaśnięcia, telewizji i boomboxa. Klient: `ui/notices.gd`,
`ui/log_history.gd` (H), `ui/chat_box.gd` (Enter; blokuje chodzenie, głos
ignoruje klawisze przy aktywnym polu). Barek: `Supplies::liquor`, kryjówki z
kafli „plant” / „bin” / „wardrobe” w dostępnych pomieszczeniach
(`hiding`), `bar_key_at` losowane rano (`hide_bar_key`), `search_hideout` na
końcu łańcucha E. `server/lost.rs`: co 30 s szansa na przechodnia
(`Npc::passerby`, `Role::Passerby`) idącego do gracza na zewnątrz; po
`Event::Arrived` pytanie (`Dialog` z `next_dialog`, `answer_lost`), potem
odejście w stronę „numeru 50” albo do drzwi i z powrotem.

**Komputery** (`computer.rs`): stanowiska to kafle typu `desk` w pokojach typu
`department` (nazwa pokoju = nazwa działu). E z laptopem w rękach przy
najbliższym wolnym stanowisku swojego działu (≤ 1,25 kafla) kładzie go
(`Computer` z `station`, `handle` z puli przedmiotów); E przy stanowisku z
laptopem otwiera sesję (`Player::at_computer`, `Computer::user` — jedna osoba
naraz), która kończy się po oddaleniu > 2 kafle, zamknięciu, blokadzie albo
zabraniu laptopa. Konto komputera = `Item::owner` laptopa; `Messenger` trzyma
kanały i rozmowy prywatne w pamięci, liczy nieprzeczytane per (konto, rozmowa)
i wskazuje odbiorców powiadomień (`audience`). Serwer loguje, kto faktycznie
pisał z cudzego komputera (bez treści). Gdy właściciel wychodzi z gry, jego
przedmioty (laptop, karta u kogoś, rzeczy na podłodze) znikają, a cudze
przedmioty, które niósł, zostają na podłodze. Klient pokazuje ekran, dopóki
w snapshocie jest bit „przy komputerze”, i blokuje wtedy ruch.

**Ekspresy** (`coffee.rs`, znalezione w mapie po typie `coffee_machine`): E
w zasięgu 1,5 kafla (gdy nie ma NPC w zasięgu rozmowy, wolne ręce) → parzenie
3 s (ekspres zajęty dla innych; bit „parzy” w `self_status` / fladze 7) →
kawa jako przedmiot w rękach (stygnie po 90 s, F = wypij); komunikaty to `Say` od samego gracza (dymek nad jego
głową, widoczny dla innych w pokoju).

**Profil postaci** (`Connect`): imię, płeć, wiek, miejscowość, e-mail i
wygląd, sprawdzane w `validate_profile` (wiek 18–70, format e-maila, wygląd w
zakresie palet) — inaczej `Reject(4)`. Wiek, miejscowość i e-mail zostają na
serwerze; `PlayerInfo` niesie imię, płeć i wygląd.

**Etapy gracza** (`Stage`): `Portal(Desk)` (po połączeniu — w domu przy
komputerze; poza światem: brak snapshotów, inputy ignorowane; stan pulpitu
ponawiany co 20 ticków) → `Working` (po „Idę do biura”: spawn przed
budynkiem). `Desk` trzyma: zgłoszenia, zaplanowane odpowiedzi (mail po
`invite_delay_secs`), zaproszenia, trwającą rozmowę, wynik i skrzynkę. Rekrutacja
(`recruitment.rs`) losuje 3 pytania oferty i tasuje odpowiedzi; odpowiedź
jest sprawdzana na serwerze. Dział z oferty trafia do umowy: HR emituje
`Event::Contract`, serwer ustawia `contract` i rozsyła `PlayerInfo` z działem
na nowo (usuwa gracza z `known` wszystkich). `--skip-recruitment` wpuszcza od
razu do świata (dev/testy); boty przechodzą quiz, zgadując do skutku.

**Sesje** są indeksowane tokenem (`by_token`), nie adresem: pakiet z ważnym
tokenem z nowego adresu przenosi sesję (`migrate`), jeśli dowodzi „świeżości”
(`Ping` albo nowe inputy). Nieznany token dostaje `Disconnect(4)`. `by_addr`
służy już tylko do deduplikacji powtórzonych `Connect`. Szczegóły:
[protokole](protokol.md) („Sesja, zmiana adresu i ponowne łączenie”).

**Symulator sieci** (`net.rs`): `--lag-ms` (opóźnienie w jedną stronę, dla
obu kierunków — RTT rośnie o 2×), `--jitter-ms` (losowe 0..=j, może zmienić
kolejność jak prawdziwe UDP), `--loss` (prawdopodobieństwo zgubienia, osobno
w każdą stronę). Działa na pakietach przychodzących i wychodzących.
