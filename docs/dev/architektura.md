# Architektura — przegląd

*Architektura: [przegląd](architektura.md) · [serwer](architektura-serwer.md) · [klient](architektura-klient.md) · [protokół](protokol.md)*

Dedykowany, autorytatywny serwer w Rust + klient Godot 4 (GDScript) po UDP,
z własnym binarnym protokołem ([protokol.md](protokol.md)).

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
  src/needs.rs          potrzeby postaci (głód, energia, stres, toaleta, higiena, upojenie), sofa / toaleta / papieros / owoce
  src/drunk.rs          alkohol: punkty za napoje, bełkot (`slur`), odczyt alkomatu, kwestie
  src/mischief.rs       psoty i bójki: zasięgi, obrażenia, czasy, kwestie
  src/pay.rs            widełki, oczekiwania, formy zatrudnienia, kwota na umowie, stawka godzinowa
  src/hr.rs             teczka pracownika: aneksy, pula urlopu, wnioski, poranek urlopu
  src/media.rs          telewizor i boombox: kanały, utwory, kwestie
  src/supplies.rs       apteczka, magazynek, zapasy na dzień, jakość skrętów, kwestie
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
  audio/audio.gd        dźwięk: szyny SFX / Ambient / Music, efekty płaskie i w świecie, pętle otoczenia, muzyka
  audio/voice.gd        czat głosowy: push-to-talk (V / B), mikrofon → 16 kHz → ADPCM, odtwarzanie przy postaci (AudioStreamGenerator), znaczek mówienia
  audio/adpcm.gd        kodek IMA ADPCM (4 bity / próbkę)
  audio/game_sounds.gd  kroki wg podłoża, pakiety Sound, blipy mowy, otoczenie, syreny, grzmoty
  sounds/               pliki WAV z tools/sounds/gen_sounds.py (syntetyzowane)
  ui/office/            aplikacje firmowego komputera: okno (os_window), tablica kanban, poczta (mail_box = dane, mail_view = skrzynka / kosz)
  icons/                ikona gry (icon.svg — źródło; icon.icns / icon.ico do eksportu) i ekran startowy splash.png
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
deploy/                 wdrożenie na VPS (deploy.sh, remote-*.sh, Tailscale, unit systemd)
.github/workflows/ci.yml testy (Rust + Godot) na każdy push
docs/                   gra/ (dla graczy), gdd/ (design), dev/ (architektura, protokół), media/
```

## Serwer i klient

Szczegóły: [serwer](architektura-serwer.md) (pętla, gracze, NPC, zapis, logowanie…) i [klient](architektura-klient.md) (sieć, predykcja, interfejs, dźwięk…).

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

**Uprawnienia i drzwi na kartę**: jedyna reguła kolizji to `Map::blocks(kafel,
uprawnienia, kierunek)`: kafel blokuje, jeśli jest pełny albo wymaga
uprawnień, których postać nie ma — chyba że porusza się w jego „wolnym
kierunku” (`free_dir`: wyjście z windy, z parkingu i z klatki schodowej do
holu oraz przez bramę garażową jest wolne). Drzwi na kartę (`card_door`),
drzwi wind i brama garażowa wymagają przepustki lub karty — przycisk windy
też (`NO_CARD`) — drzwi zaplecza —
uprawnień obsługi. BFS (`Building::find_path`) stosuje te same reguły.

**Przejścia między piętrami**:
- **Schody** (część kroku, więc przewidywane przez klienta): wejście środkiem
  postaci na kafel schodów (`links` typu `stairs`) przenosi na kafel przyjścia
  na innej mapie. Między parterem a piętrem 1 jest osobna mapa **klatki
  schodowej** (w `building.json` jako „piętro” 3 z `stairwell: true`): bieg w
  górę, półpiętro, drugi bieg — widać tylko klatkę i osoby na niej. Zaraz po
  przejściu działa blokada: schody nie zadziałają, dopóki nie zmienisz
  klawiszy ruchu *i* nie zejdziesz z obszaru schodów.
- **Windy** (`elevator.rs`, poza symulacją): każdy `id` linku `elevator` to
  osobna winda (teraz „A” i „B” obok siebie); E przy drzwiach przywołuje
  najbliższą w zasięgu (`Elevator::door_distance`), `Doors` niesie stan każdej
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
schodach, też z wolnym chodem i zataczaniem po alkoholu + scenariusz: spawn → schody przez klatkę →
Chill room → z powrotem do holu) jest generowany przez Rust i odtwarzany w Godocie.

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
  lub karta) dla bramek i bramy garażowej, „service” dla składzika sprzątaczki;
  `free_dir`: kierunek, w którym kafel zawsze przepuszcza. Drzwi zablokowane
  (`locked_door`) są po prostu stałe (`solid`).
- `rooms`: druga warstwa znaków tej samej wielkości; `room_defs` mapuje znak →
  `{id, name, type, see?, gender?, outdoor?, detector?, below?, light?, switch?,
  lit_by?, windows?, department?}`, `-` = brak pokoju (ściany). Id są unikalne w
  obrębie piętra; `see` — klucze pokoi, których ludzi też widać (interest
  management); `department` — czyje są biurka w pokoju (`computer::Workstation`
  bierze dział z pokoju, nie z nazwy). Działy (id, nazwa, skrót) są w
  `recruitment.json`; serwer rozsyła je pakietem `Departments`, klient trzyma
  je w `net/departments.gd`.
- `links`: `{kind: "stairs", area: [x,y,w,h], to_floor, to: [x,y]}` albo
  `{kind: "elevator", id, area}`.
- `spawns`: kafle startowe (tylko parter: chodnik przed wejściem).
- `npcs`: `{kind, name, home: [x,y], escort_to?: [piętro,x,y]}` — portier,
  kasjer, ochrona, sprzątaczka, recepcja, HR, prezes, wspólniczka.
- `places`: nazwane punkty (`map::Places`). Na parterze otoczenie
  (`crate::outside::Outside`, wczytywane z budynkiem): `street_y`, `tram_y`,
  `walk_home`, `walk_arrival`, `taxi`, `tram_stop`, `car_bays`, `bike_rack`,
  `police`, `fire` — z nich dojazdy (`commute`), radiowóz (`security`) i wóz
  strażacki (`fire`); `shelves` (id półki → prostokąt, towary w `shop`). Na
  piętrze `tray` (słodycze) i `founder` (miejsce nowego założyciela). Dzięki
  temu przebudowa mapy nie wymaga zmian w kodzie; klient bierze z `places`
  miejsca powrotu do domu.

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

## Wdrożenie (VPS)

Serwer produkcyjny: VPS Hetzner (Ubuntu, x86_64), usługa systemd
`startup-sim` jako osobny użytkownik, dane w `/var/lib/startup-sim`
(`world.db`, `tls/`, `backups/`). Kod trafia tam z komputera dewelopera
(`deploy/deploy.sh`: rsync źródeł i map → build na VPS → restart; SIGTERM
zapisuje grę).

| port | dostęp | po co |
|------|--------|-------|
| 7777/udp | publiczny | gra |
| 7778/tcp | publiczny | logowanie HTTPS |
| 22/tcp | **tylko tailnet** (`tailscale0`) | SSH: wdrożenia, kopie, administracja |

SSH jest zamknięte dla internetu (ufw); maszyna jest w tailnecie Tailscale
jako `startup-sim`, skrypty łączą się z `root@startup-sim`. Klucz węzła musi
mieć wyłączone wygasanie (panel Tailscale); awaryjnie — konsola Hetznera. Szczegóły:
`deploy/README.md`.

## Testy

| polecenie | co sprawdza |
|-----------|-------------|
| `cd server && cargo test` | 51 testów jednostkowych (budynek i mapy wg GDD, osiągalność zależna od uprawnień, bramki, ruch/kolizje, schody, winda, nawigacja, portier, recepcja, HR, rekrutacja: zaliczenie/oblanie, ignorowanie nieaktualnych odpowiedzi, losowanie i tasowanie; ekspres; protokół), 2 golden, 9 e2e (m.in. ekspres: parzenie, zajętość, kubek widoczny dla innych; portal: odrzucenie → przyjęcie → spawn; całe wdrożenie aż do karty; widoczność między piętrami; stan serwera = predykcja) |
| `godot --headless --path client -s tests/run_tests.gd` | parytet protokołu (bajt w bajt) i ruchu — z bramkami, uprawnieniami i przejściami między piętrami — z Rustem, zgodność CRC budynku, parsowanie adresów |
| `python3 tests/e2e/run.py` | scenariusze rozgrywki: prawdziwy klient Godot (headless) na prawdziwym serwerze — `workday` (laptop, obiad z aplikacji, kawa, odbiór obiadu, powrót tramwajem z wypłatą), `onboarding` (rejestracja, postać, portal i rozmowa, portier, recepcja, HR, laptop na biurku działu), `together` (dwóch graczy: komunikator, spotkanie w chill roomie, winda), `founder` (firma z portalu, stanowisko w dziale Mobile, zatrudnienie kandydata), `persistence` (laptop na biurku, restart serwera, powrót postaci), `drinking` (wino i dwie małpki ze sklepu, wymioty z plamą, zataczanie), `fight` (dwóch graczy: nóż z szafki, nokaut, ochrona łapie napastnika, menu R), `resign` (jak onboarding, ale w HR „Rezygnuję”: odprowadzenie na portiernię, oddanie przepustki, powrót na portal), `office_apps` (Kadry: umowa i urlop na jutro, terminal, Internet w przeglądarce), `chill` (dwóch graczy: pilot — mecz w telewizorze, boombox — disco polo, drugi widzi i słyszy to samo), `storeroom` (witamina z apteczki, klucz: odmowa przy recepcjonistce, wzięty w jej przerwie, cola z magazynku), `chat` (dwóch graczy: powiadomienie o mailu, czat w chill roomie — pokój, szept, krzyk). Scenariusz (`client/tests/e2e/scenario.gd`) steruje postacią przez `game.script_driver`, ma własny folder w `user://e2e/` i kończy się `E2E PASS` / `E2E FAIL` z kodem wyjścia. |
| `python3 tests/load/soak.py` | obciążenie: N botów (`src/bin/bots.rs`) przez kilka minut — zero zgubionych ticków, najdłuższy tick < 25 ms, brak awarii, pamięć bez wzrostu; w CI co noc (`.github/workflows/soak.yml`). |
