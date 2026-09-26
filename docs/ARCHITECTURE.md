# Architektura

Dedykowany, autorytatywny serwer w Rust + klient Godot 4 (GDScript) po UDP,
z własnym binarnym protokołem (`docs/PROTOCOL.md`).

```
┌───────────── client (Godot) ─────────────┐        UDP        ┌──────────── server (Rust) ────────────┐
│ main.gd        start screen <-> gra      │                   │ main.rs     CLI, start                │
│ net_client.gd  handshake, ping, timeout  │ ── Input 60 Hz ─▶ │ server/     pętla 20 Hz, gracze,      │
│ game.gd        predykcja + rekoncyliacja │                   │             interest mgmt, snapshoty  │
│                interpolacja innych       │ ◀─ Snapshot 20 Hz │ net.rs      socket + symulator laga    │
│ movement.gd ═══════ identyczny algorytm ══════════════════════ sim.rs      ruch, kolizje, piętra     │
│ protocol.gd ═══════ identyczny format ════════════════════════ protocol/   kodowanie pakietów        │
│ building.gd ═══╗                         │                   │ building.rs ═╗ piętra, BFS po budynku │
│ map_data.gd    ║                         │                   │ map.rs       ║ piętro, pokoje, linki   │
└────────────────║─────────────────────────┘                   └──────────────║────────────────────────┘
                 ╚══════ client/maps/building.json + floorN.json (JEDNO źródło) ╝
```

## Repozytorium

```
server/                 crate Rusta (lib `game` + binarki)
  src/lib.rs            moduły współdzielone przez serwer i boty
  src/building.rs       budynek: lista pięter (+ mapa klatki schodowej), CRC, BFS między piętrami
  src/map.rs            jedno piętro: kafle, kolizje, pokoje, linki (schody/winda)
  src/sim.rs            deterministyczny krok: ruch, kolizje, schody
  src/elevator.rs       winda: przywołanie, jazda, drzwi (serwer, poza symulacją)
  src/nav.rs            podążanie ścieżką (boty, NPC)
  src/npc.rs            NPC po stronie serwera (portier, recepcja, HR)
  src/recruitment.rs    portal z ofertami i quiz rekrutacyjny
  src/coffee.rs         ekspresy do kawy (parzenie → kawa jako przedmiot)
  src/inventory.rs      przedmioty, kieszenie i ręce, uprawnienia z przedmiotów
  src/computer.rs       stanowiska (biurka działów), laptopy na biurkach, komunikator
  src/fire.rs           dym (ilość na pokój, przenikanie drzwiami), czujki, alarm i straż
  src/cleaning.rs       kubki po kawie i wieczorny obchód sprzątaczki (stałe, teksty)
  src/security.rs       kradzież w sklepie: mandat, radiowóz (trasa), wezwania policji
  src/company.rs        firma i założyciel: nazwa, opisy ofert, kandydaci, zespół
  src/lunch.rs          zamawianie obiadów: menu, zamówienia, dostawa na recepcję
  src/treats.rs         słodycze w chill roomie (taca, ogłoszenie), nieświeże owoce
  src/board.rs          zarząd: sloty kalendarza, spotkania, dialogi i ich skutki
  src/weather.rs        pogoda: stany, przejścia, wpływ na zewnątrz
  src/commute.rs        dojazd: sposoby (czas, koszt, efekty), pojazdy jadące po trasach
  src/clock.rs          zegar gry: doba, biuro 6–22, przewijana noc, pensja za minuty
  src/shop.rs           sklep: towary i ceny, półki, kasa, złotówki
  src/stalls.rs         kabiny toaletowe: znajdowanie drzwi, zamykanie od środka
  src/needs.rs          potrzeby postaci (głód, energia, stres, toaleta), sofa / toaleta / papieros / owoce
  data/recruitment.json oferty i pule pytań (pierwsza odpowiedź = poprawna)
  src/protocol/         pakiety: mod.rs (typy, stałe), codec.rs (bajty), encode.rs / decode.rs,
                        snapshot.rs (fragmentacja), golden.rs (wektory parytetu z GDScriptem)
  src/net.rs            UdpSocket + symulator opóźnienia/jittera/strat
  src/server/           autorytatywny serwer: `Server` + jeden plik na funkcję (każdy to `impl Server`)
    mod.rs              stan świata, Config, pętla `run`, fazy `tick`, typ `Say`
    session.rs          datagramy, handshake, migracja adresu, timeouty, wyjście gracza
    movement.rs         inputy → `sim::step`, skutki chodzenia (sklep, łazienka, kawa, pogoda)
    interact.rs         klawisz E (biurko, NPC, ekspres, miejsca, winda…) i zdarzenia NPC
    snapshot.rs         interest mgmt, snapshoty, mowa, okresowe stany ekranów
    player.rs           gracz, etap (portal / praca / dom), walidacja profilu
    leave.rs            wcześniejszy powrót do domu (E przy pojeździe / przystanku), przyspieszony zegar
    portal.rs, company.rs, items.rs, day.rs, doors.rs, alarm.rs, cleaning.rs,
    police.rs, shop.rs, lunch.rs, treats.rs, board.rs, spots.rs, computers.rs, stats.rs
    tests.rs            testy wnętrza serwera (limity, id, spotkania)
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
  net/protocol.gd       lustro protocol/
  net/net_client.gd     połączenie UDP (PacketPeerUDP)
  sim/movement.gd       lustro sim.rs
  map/building.gd       lustro building.rs (bez BFS)
  map/map_data.gd       lustro map.rs (bez BFS)
  map/map_art.gd        proceduralny pixel art piętra (podłogi, ściany 3/4, meble)
  map/map_view.gd       tekstura piętra + podpisy pomieszczeń
  game/game.gd          logika sieciowa gry po stronie klienta
  game/player_view.gd   pixel-artowa postać z animacją chodu, przedmiotem w rękach, nickiem, dymkiem
  game/item_art.gd      ikony przedmiotów (rysowane prostokątami)
  game/item_view.gd     przedmiot leżący na podłodze
  ui/inventory_hud.gd   pasek ekwipunku (ręce + 3 kieszenie)
  ui/computer_screen.gd ekran komputera: komunikator i ekran blokady
  ui/stats_hud.gd       portfel i paski potrzeb (prawy górny róg)
  game/vehicle_view.gd  pojazd (auto, taksówka, tramwaj, rower)
  game/tray_view.gd     taca ze słodyczami
  ui/dialog_window.gd   okno rozmowy (spotkania z zarządem)
  ui/weather_fx.gd      deszcz, burza (błyski), mgła na ekranie
  ui/day_screen.gd      plansze dnia: koniec dnia, noc, poranny wybór dojazdu, w drodze
  ui/shelf_window.gd    okno półki sklepowej (towary, ceny, „Weź”)
  game/elevator_door_view.gd drzwi windy (rozsuwane) i wyświetlacz piętra
  game/stall_door_view.gd drzwi kabiny (zielone wolne / czerwone zajęte, otwarte, gdy ktoś w nich stoi)
  game/computer_view.gd laptop na biurku (ekran: niebieski / czat / zablokowany)
  game/remote_player.gd bufor snapshotów + interpolacja
  ui/character_screen.gd tworzenie postaci (dane + wygląd z podglądem)
  ui/desktop.gd         pulpit komputera: przeglądarka (portal, formularz), poczta, rozmowa online
  ui/debug_overlay.gd   F3
  tests/run_tests.gd    testy headless (parytet z Rustem)
  tests/render_maps.gd  narzędzie: zapis grafiki pięter do PNG (headless)
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
prowadzi go na recepcję piętra 1 (bramki, schody). Czeka, jeśli gościa nie ma
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
Powrót przed 22:00 (`server/leave.rs`): E przy własnym pojeździe
(`Vehicle::depart` — odjazd ulicą) lub w `commute::home_spot` dla trybu,
dwa naciśnięcia w `CONFIRM_TICKS` → `go_home` (wypłata). Gdy wszyscy gracze
są w domu i nikt nie jedzie, `Clock::fast` = tempo nocne.

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
piętro, pozycja, poprzedni input, blokada schodów i uprawnienia (`access`). Wszystko, od czego zależy
krok, jest w tym stanie i idzie w snapshocie do właściciela — dzięki temu
rekoncyliacja odtwarza inputy od dokładnie tego samego stanu.

**Uprawnienia i bramki**: jedyna reguła kolizji to `Map::blocks(kafel,
uprawnienia, kierunek)`: kafel blokuje, jeśli jest pełny albo wymaga
uprawnień, których postać nie ma — chyba że porusza się w jego „wolnym
kierunku” (`free_dir`: wyjście przez bramki i bramę garażową w dół jest wolne).
Bramki i brama garażowa wymagają przepustki lub karty, drzwi zaplecza —
uprawnień obsługi. BFS (`Building::find_path`) stosuje te same reguły.

**Przejścia między piętrami**:
- **Schody** (część kroku, więc przewidywane przez klienta): wejście środkiem
  postaci na kafel schodów (`links` typu `stairs`) przenosi na kafel przyjścia
  na innej mapie. Między parterem a piętrem 1 jest osobna mapa **klatki
  schodowej** (w `building.json` jako „piętro” 3 z `stairwell: true`): bieg w
  górę, półpiętro, drugi bieg — widać tylko klatkę i osoby na niej. Zaraz po
  przejściu działa blokada: schody nie zadziałają, dopóki nie zmienisz
  klawiszy ruchu *i* nie zejdziesz z obszaru schodów.
- **Winda** (`elevator.rs`, poza symulacją): E przy drzwiach przywołuje ją
  (kolejka pięter), jazda trwa 3 s na piętro, drzwi są otwarte 4 s i nie
  zamkną się na kimś w drzwiach; z więcej niż 6 osobami w kabinie (3×2 pola)
  nie rusza; w czasie jazdy klient wygasza wszystko poza kabiną (`ride_mask.gd`,
  między mapą a postaciami); E w kabinie wybiera następne aktywne piętro
  (drzwi zamykają się po 1 s). Drzwi windy są w nakładce `closed` mapy (jak
  kabiny toaletowe), więc zamknięte blokują ruch także w predykcji. Po
  przyjeździe serwer przenosi wszystkich z kabiny piętra startowego na tę samą
  pozycję docelowego piętra; klient dostaje to jak korektę (zmiana piętra =
  przeskok).

Ten sam algorytm jest w `sim.rs` i `movement.gd`; GDScript liczy na 64-bit
int, więc wyniki są bit w bit równe. `tests/golden/movement_vectors.json`
(losowe przejścia × 400 kroków przy ścianach, meblach, drzwiach, bramkach,
schodach, też z wolnym chodem + scenariusz: spawn → schody przez klatkę →
Chill room → z powrotem do holu) jest generowany przez Rust i odtwarzany w Godocie.

## Klient

**Pulpit** (`ui/desktop.gd`, osobna warstwa nad grą): „StartOS” z ikonami
Przeglądarka / Poczta / Kosz, paskiem zadań i zegarem; okna (przeciągane):
przeglądarka z portalem i formularzem zgłoszeniowym (dane postaci + „Dlaczego
chcesz u nas pracować?” + zgoda), poczta (lista, podgląd, przyciski akcji),
rozmowa online (kafelki wideo: rekruterka i Twoja postać, pytania). Pokazuje
stan z serwera, ponawia swoje akcje, gdy serwer ich nie odnotował, i znika,
gdy przyjdą pierwsze snapshoty. Dopóki jest widoczny, postać nie
dostaje inputu. Dział gracza widać przy nicku („Ala · IT”, po umowie) i w F3.

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

**Widoczność**: przy zmianie piętra klient usuwa wszystkich zdalnych graczy;
przy zmianie pokoju — tych, których nie ma w nowym snapshocie (osoby widoczne
z obu pokoi, np. portier, zostają bez mrugnięcia). Gracz
nieobecny w snapshotach przez 5 ticków znika.

**Grafika** — proceduralny pixel art (16 px), bez zewnętrznych plików:
- `map_art.gd` rysuje piętro raz, przy wczytaniu (~20–40 ms): podłogi z
  wariantami tekstur (bez powtarzalnego wzoru), ściany w rzucie 3/4 (ciemny
  wierzch, jasny front z listwą i obrazkami tam, gdzie widać pokój poniżej),
  cienie ścian, drzwi / bramki / szklane drzwi / schody / winda, a meble jako
  całe obiekty (spójne grupy tego samego znaku: biurka z monitorami i
  krzesłami, lady, regały z towarem, sofa, stoły z krzesłami, rośliny, szafy
  serwerowe, ławka, popielniczka, toalety, umywalki, samochody z liniami
  miejsc). Typ mebla bierze się z legendy mapy; kolizje zależą tylko od
  `solid`, więc zmiana wyglądu nie rusza symulacji.
- `player_view.gd` rysuje postać prostokątami w `_draw()`: głowa, fryzura
  (6 stylów), koszula, ręce, spodnie, buty; 4 kierunki; cykl chodu liczony z
  przebytej drogi (ten sam dla własnej postaci i interpolowanych innych).
  Wygląd gracza wybiera on sam na ekranie tworzenia postaci (indeksy palet w
  `PlayerInfo`); przed nadejściem `PlayerInfo` — zastępczy wygląd z id; NPC mają stroje
  (portier: mundur i czapka, personel: koszula z krawatem). Własna postać ma
  jasny obrys.

**Render**: każde piętro to jeden sprite (widoczne tylko bieżące), postacie to
`Node2D._draw()` z `Label`em (y-sort). Kamera `Camera2D` z zoomem 3×, bez
wygładzania, z limitami mapy. Etykiety mają skalę `1/zoom` i rozmiar czcionki
ekranowej, więc są ostre mimo zoomu.

**Podpowiedzi** na dole ekranu: „[E] Wezwij windę” / „Winda jedzie…” przy
drzwiach windy, „[E] Jedź na: …” w kabinie,
„[E] Porozmawiaj: Portier” przy NPC, informacja o wymaganej przepustce przed
bramką. **NPC** rysowane są w mundurze z czapką; wypowiedzi (`Say`) pokazują się
w dymku nad postacią (także gdy mówiący dopiero wejdzie w pole widzenia) i w
logu w lewym dolnym rogu.

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
- `tiles`: wiersze znaków; `legend` mapuje znak →
  `{type, solid, color, access?, free_dir?}`. `access`: „card” (przepustka gościa
  lub karta) dla bramek i bramy garażowej, „service” dla zaplecza;
  `free_dir`: kierunek, w którym kafel zawsze przepuszcza.
- `rooms`: druga warstwa znaków tej samej wielkości; `room_defs` mapuje znak →
  `{id, name, type, see?}`, `-` = brak pokoju (ściany). Id są unikalne w obrębie
  piętra; `see` — klucze pokoi, których ludzi też widać (interest management).
- `links`: `{kind: "stairs", area: [x,y,w,h], to_floor, to: [x,y]}` albo
  `{kind: "elevator", id, area}`.
- `spawns`: kafle startowe (tylko parter: chodnik przed wejściem).
- `npcs`: `{kind, name, home: [x,y], escort_to?: [piętro,x,y]}` — na razie portier.

Kafle drzwi należą do pokoju po stronie „publicznej” (korytarz / hol), więc
stojąc w drzwiach widzisz korytarz.

## Gotowość na rozbudowę (nie zaimplementowane)

| funkcja | gdzie się wepnie |
|---------|------------------|
| **Piętro 2** | wpis w `building.json` z `locked: true`; odblokowanie = plik mapy + `locked: false` (winda i schody same go obsłużą; do ustalenia: odblokowanie w trakcie gry wymaga zmiany CRC albo osobnego komunikatu). |
| **Trwałość karty** | karta żyje tyle, co sesja; zapis między sesjami wymaga kont (backend). |
| **Kolejne NPC** | np. Zarząd: nowy `kind` w `npcs` mapy + `Role` w `npc.rs`; rozmowa, odprowadzanie, dymki i widoczność są wspólne. |
| **Skutki działu** | dział jest w stanie gracza i w `PlayerInfo`; ograniczenia (np. drzwi działów, zadania) dojdą z zadaniami. |
| **Rekrutacja z AI** | `Attempt` to jedyne miejsce oceniania — rozmowę z NPC napędzaną AI można podpiąć zamiast quizu bez zmian w protokole świata. |
| **Akcje / interakcje** | bit 16 (E) działa jak w windzie: kontekst = link/kafel, na którym stoisz; bity 5–7 wolne. |
| **Więcej graczy w pokoju** | fragmentacja snapshotów już działa; następny krok to delta względem `ack_tick` i/lub priorytet po odległości. |

## Boty (`cargo run --release --bin bots`)

Jeden wątek, N gniazd nieblokujących, pętla 60 Hz. Każdy bot przechodzi pełny
handshake, predykuje ruch tym samym `sim::step` i robi rekoncyliację (log
pokazuje liczbę błędnych predykcji), chodzi (`nav::Walker`) po ścieżkach BFS
przez cały budynek, schodami między piętrami — `--room-share` z nich wybiera
cele tylko w `--room` (domyślnie „Chill room” na piętrze 1). Za bramki boty
przejdą tylko, gdy serwer działa z `--start-with-card`. Log co 5 s:
połączeni, liczba w docelowym pokoju, RTT, odbierany transfer, widoczni.

## Testy

| polecenie | co sprawdza |
|-----------|-------------|
| `cd server && cargo test` | 51 testów jednostkowych (budynek i mapy wg GDD, osiągalność zależna od uprawnień, bramki, ruch/kolizje, schody, winda, nawigacja, portier, recepcja, HR, rekrutacja: zaliczenie/oblanie, ignorowanie nieaktualnych odpowiedzi, losowanie i tasowanie; ekspres; protokół), 2 golden, 9 e2e (m.in. ekspres: parzenie, zajętość, kubek widoczny dla innych; portal: odrzucenie → przyjęcie → spawn; całe wdrożenie aż do karty; widoczność między piętrami; stan serwera = predykcja) |
| `godot --headless --path client -s tests/run_tests.gd` | parytet protokołu (bajt w bajt) i ruchu — z bramkami, uprawnieniami i przejściami między piętrami — z Rustem, zgodność CRC budynku, parsowanie adresów |
