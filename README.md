# Startup Sim

2D multiplayer „symulator pracy w startupie IT” — serwer Rust + klient Godot 4.
Dokumentacja: [GDD](docs/GDD.md) · [Protokół](docs/PROTOCOL.md) ·
[Architektura](docs/ARCHITECTURE.md) · [Jak pomóc](CONTRIBUTING.md).

*English: a 2D multiplayer "working at an IT startup" simulator — an
authoritative Rust server (UDP, 20 Hz) and a Godot 4 client. The game and the
docs are in Polish; contributions in English are welcome too
([CONTRIBUTING](CONTRIBUTING.md)). License: AGPL-3.0-or-later.*

## Pobierz

Gotowy klient na macOS (podpisany i notaryzowany dmg, Intel + Apple Silicon):
[Startup Sim 0.1.0](https://github.com/mateuszpalak/startup-sim/releases/tag/v0.1.0) —
wszystkie wydania: [Releases](https://github.com/mateuszpalak/startup-sim/releases).
Gra przy starcie sprawdza najnowsze wydanie na GitHubie i, jeśli jest nowsze,
proponuje „Pobierz” na ekranie tytułowym; serwer odrzucający starą wersję
(inny protokół) też kończy się tym przyciskiem.

Nowe wydanie:
1. podbij wersję w `client/project.godot` (`config/version`) i w
   `client/export_presets.cfg` (`application/short_version` i o 1 wyżej
   `application/version`) — po niej klienci poznają, że jest nowsza;
2. zbuduj dmg: `tools/build-macos.sh`;
3. `git tag -a v<wersja> -m "Startup Sim <wersja>"` i `git push origin v<wersja>`;
4. `gh release create v<wersja> build/StartupSim-<wersja>.dmg --title "Startup Sim <wersja>" --notes "…"`
   (tag `v<wersja>` musi zgadzać się z `config/version`; szkiców i wersji
   „pre-release” klienci nie proponują);
5. przy zmianie protokołu wdróż też serwer (`deploy/deploy.sh`) — stare
   klienty dostaną „pobierz najnowszą”.

## Uruchomienie

Wymagania: Rust (rustup), Godot 4.7 (`godot` w PATH).
Klient na macOS jako jeden plik: `tools/build-macos.sh` → `build/StartupSim-<wersja>.dmg`
(aplikacja uniwersalna Intel + Apple Silicon, podpisana Developer ID z hardened runtime i
uprawnieniem do mikrofonu, a z profilem `notarytool` także notaryzowana). Kto podpisuje:
`IDENTITY` / `NOTARY_PROFILE` w środowisku albo w `tools/macos/signing.env` (poza repozytorium).
`--app-only` robi samą aplikację bez podpisu (do własnego podpisania z
`tools/macos/entitlements.plist`). Wymaga szablonów eksportu Godota 4.7.2; wersja w `client/export_presets.cfg`.
Serwery w kliencie: `client/net/servers.cfg` (poza repozytorium, wzór `servers.example.cfg`) —
bez niego klient zna tylko serwer lokalny.
CI (GitHub Actions, `.github/workflows/ci.yml`) przy każdym pull requeście i pushu na `main`: rustfmt, clippy i testy serwera,
cargo-deny (podatności i licencje zależności), gdlint i testy klienta w Godocie, ruff i zgodność map z generatorem, gitleaks.

```bash
cd server && cargo run --release            # gra na [::]:7777 (UDP) + logowanie HTTPS na :7778; zapis w server/saves/world.db
godot --path client                         # klient (można odpalić kilka razy): Graj → zaloguj się / załóż konto
cd server && cargo run --release -- --start-with-card --allow-guests   # wariant dla botów: goście z kartą
cd server && cargo run --release -- --reset-password Ola                # administrator: nowe jednorazowe hasło
cd server && cargo run --release -- --list-accounts                     # administrator: lista kont
cd server && cargo run --release --bin bots -- --count 50 --room "Chill room" --all-in-room
```

Sterowanie: WASD / strzałki, **E** — rozmowa z NPC / winda (przy drzwiach — wezwij, w kabinie — wybierz piętro) / umywalka / płyn antybakteryjny / ekspres do kawy / podniesienie przedmiotu / biurko (połóż laptop, usiądź do komputera — pulpit z pocztą, przeglądarką z tablicą zadań działu i obiadami, komunikatorem, kalendarzem i koszem; Esc — wstań) / misa z owocami / sofa / toaleta / popielniczka (E ponownie — wstań), **1–3** — wyjmij / schowaj przedmiot z kieszeni, **Q** — upuść, **G** — podaj osobie obok, **F** — użyj (pilot w chill roomie: kanał w telewizorze, boombox: muzyka; wypij kawę, zjedz / wypij coś ze sklepu, pokaż kartę; Zarząd: alkomat przy kimś — powyżej 0,2 ‰ można wystawić naganę, 3 nagany = zwolnienie); w sklepie na parterze **E** przy półce — lista towarów (1–9 weź), **E** przy kasie — zapłać, **L** — zamknij / otwórz kabinę toaletową (od środka), **R** — menu psot (nasikaj / zesraj się na podłogę, nasikaj do ekspresu albo komuś do kubka), **X** — uderz osobę obok (z nożem z kuchennej szafki w rękach: dźgnij; **F** z nożem też), **E** przy włączniku przy drzwiach — zapal / zgaś światło, **kółko myszy** albo **+ / -** — przybliż / oddal kamerę, **Esc** — menu gry (ustawienia, wyjście do menu / z gry), w aneksie kuchennym **E** przy szafce (kubek), ekspresie (kawa do kubka), zlewie (umyj kubek), zmywarce (włóż / włącz / rozładuj) i lodówce (okno), **E** dwa razy przy swoim aucie / rowerze, na zachodnim końcu chodnika, przystanku tramwajowym lub postoju taksówek — powrót do domu przed końcem dnia (wypłata za przepracowany czas), **V** (trzymaj) — mów do osób w tym samym pomieszczeniu, **B** (trzymaj) — szept do osoby obok, **Enter** — czat (do pokoju, `/s` szept, `/k` krzyk), **H** — dziennik dnia, **F3** — overlay debug. Głośność efektów, otoczenia i muzyki, „Oszczędzanie baterii” (30 klatek/s zamiast 60; w tle gra rysuje 20) i „Wysyłaj raporty awarii bez pytania”: Ustawienia (menu / Esc). Po awarii gra przy następnym starcie proponuje wysłanie raportu (koniec dziennika gry) na serwer. Dźwięki generuje `python3 tools/sounds/gen_sounds.py`.
Na starcie logujesz się (nick = login i imię postaci, hasło; „Zapamiętaj mnie”
trzyma na komputerze tylko token, nie hasło). Nowe konto tworzy postać (imię,
płeć, wiek, miejscowość, e-mail postaci, wygląd z podglądem); kolejne
logowania wchodzą prosto do gry. Serwer sam robi sobie certyfikat HTTPS —
klient zapamiętuje go przy pierwszym połączeniu i ostrzega, jeśli się zmieni
(własny certyfikat, np. Let's Encrypt: `--tls-cert` / `--tls-key`).
Po zalogowaniu cały ruch gry jest szyfrowany kluczem sesji z logowania. Po połączeniu siedzisz w domu przy komputerze: w **przeglądarce** jest portal z
ogłoszeniami (kilka firm; zatrudnia tylko nasz startup), wypełniasz formularz,
po chwili w **Poczcie** czeka zaproszenie na **rozmowę online** (3 pytania
z puli 25 na stanowisko, bez powtórek dopóki nie przejdziesz całej puli;
2 poprawne = przyjęcie), a potem zaproszenie na dzień próbny — „Idę do biura”.
Na miejscu startujesz przed budynkiem bez przepustki: bramki w holu go nie wpuszczą,
więc trzeba podejść do portierni i porozmawiać z portierem (E) — da przepustkę
gościa i zaprowadzi na recepcję na piętrze 1 (schodami). Recepcja (E) zaprowadzi
do HR, a HR (E) podpisze umowę i wyda kartę pracownika.

### Serwer — opcje
`--bind`, `--map`, `--max-players`, `--stats-secs`, `--start-with-card` (każdy gracz z kartą — do testów z botami),
`--skip-recruitment` (bez portalu, od razu do świata), `--start-employed` (od razu zatrudniony:
umowa, karta i laptop, start przy biurku działu — nieparzyste id IT, parzyste Biznes),
`--needs-speed <n>` (głód, energia itd. zmieniają się n razy szybciej),
`--start-time <hh:mm>` (godzina gry na starcie, domyślnie 8:00), `--weather sun|clouds|rain|storm|fog` (stała pogoda), `--treats` (od razu taca słodyczy w chill roomie), `--stale-fruit <proc>` (szansa na nieświeży owoc, domyślnie 15), `--cleaning-at <hh:mm>` (dokładny start obchodu sprzątaczki; domyślnie losowo 15:00–16:00), `--start-cigarettes` (z `--start-employed`: paczka papierosów w kieszeni), `--time-scale <n>` (dzień w grze płynie n razy szybciej), `--recruitment <plik>` (oferty i pytania,
domyślnie `server/data/recruitment.json`), oraz symulacja sieci:
`--lag-ms <ms>` (opóźnienie w jedną stronę, RTT rośnie 2×), `--jitter-ms <ms>`, `--loss <0..1>`.
Np. RTT ~100 ms i 2% strat: `cargo run --release -- --lag-ms 50 --jitter-ms 10 --loss 0.02`.

### Klient — argumenty deweloperskie (po `--`)
`--nick=Ala --server=127.0.0.1:7777 --autoconnect --debug` (gość — serwer z `--allow-guests` albo `--no-save`), `--login=Ola:haslo [--register] [--autocreate]` (logowanie / rejestracja przez ekran logowania), `--login-screen`, `--commute=3` (co rano wybierz dojazd: 1 pieszo … 5 tramwaj), `--found="Nazwa firmy"` (załóż firmę z portalu), `--voice-tone` (czat głosowy nadaje ton testowy zamiast mikrofonu; goto `talk:N` / `whisper:N`), `--record=/katalog --record-start=2 --record-length=6` (klatki JPG do zwiastuna — patrz `tools/trailer/`), `--auto-recruit=1 [--auto-recruit-delay=2]`
(sam aplikuje na ofertę 1 i zgaduje odpowiedzi do skutku), (adres może być też IPv6: `--server=[::1]:7777`) (F3 od startu),
`--autowalk` (losowy ruch), `--goto="34,49;E;wait:2;31,44;Sklep"` (kolejne
kroki: kafel / pokój na bieżącym piętrze, `E` = wciśnij E, `wait:N` = czekaj —
tu: rozmowa z portierem, za bramki, potem do sklepu; też `item:take0|put|drop|give|use` i
`L` = zamknij/otwórz kabinę, ekran komputera: `pc:say:general|dept|dm:<imię>:<tekst>`, `pc:open:…`, `pc:win:mail|trash|browser|tasks|lunch|calendar|company|chat`, `pc:task:<tytuł>`, `pc:card:<n>`, `pc:comment:<tekst>`, `pc:take-card`, `pc:mail:<nick>:<temat>`, `pc:read:<n>`, `pc:lock`, `pc:unlock`,
`pc:take`, `pc:close`, kalendarz `pc:cal:<minuta>:<temat>`, obiad `pc:lunch:<danie>`, panel firmy `pc:company` / `pc:company:hire` (pierwszy kandydat) / `pc:company:<akcja>:<cel>:<wartość>[:<tekst>]`; `dlg:<nr>` — odpowiedz w oknie rozmowy),
`--perf` (co 2 s: FPS, wywołania rysowania, elementy, co się najczęściej przerysowuje — pomiar wydajności),
`--screenshot=/tmp/x.png --screenshot-delay=5` (zapis klatki i wyjście; kilka czasów
`--screenshot-delay=5,12,20` zapisuje `x_1.png`, `x_2.png`, … i wychodzi po ostatnim).

## Testy

```bash
cd server && cargo test
godot --headless --path client -s tests/run_tests.gd
```

Pliki golden (`server/tests/golden/`) pilnują, że protokół i ruch są identyczne
w Rust i GDScript. Po celowej zmianie: `UPDATE_GOLDEN=1 cargo test --test golden`.

Rozgrywka od początku do końca — prawdziwy klient (bez okna) na prawdziwym
serwerze, sterowany scenariuszami z `client/tests/e2e/scenarios/`:

```bash
python3 tests/e2e/run.py              # wszystkie (ok. 3 min): workday, onboarding, resign, office_apps, chill, storeroom, chat, together, founder, persistence, drinking, fight
python3 tests/e2e/run.py together     # wybrane; --list wypisze nazwy
python3 tests/load/soak.py            # obciążenie: 50 botów przez 3 min (--bots, --minutes)
```

Szczegóły i jak dopisać scenariusz: [tests/README.md](tests/README.md).

## Wdrożenie na VPS

Serwer testowy działa na VPS-ie (Hetzner). SSH jest otwarte **tylko z tailnetu**
(Tailscale, maszyna `startup-sim`) — wdrażać można tylko z komputera w tailnecie:

```bash
deploy/deploy.sh          # kod → VPS, build, restart (gra zapisuje się przy restarcie)
deploy/pull-backups.sh    # dzienne kopie bazy do vps-backups/
deploy/pull-crashes.sh    # raporty awarii klientów do crash-reports/
```

Porty gry (7777/udp, 7778/tcp) są publiczne. Pierwsza konfiguracja, zapora i
Tailscale: [deploy/README.md](deploy/README.md).

## Czcionka

Interfejs używa odręcznej czcionki **Patrick Hand** (© Patrick Wagesreiter), na
licencji SIL Open Font License 1.1 — plik i licencja w `client/fonts/`
(`PatrickHand-Regular.ttf`, `OFL-PatrickHand.txt`). Klient: `--no-mood` wyłącza
efekt „tuszu i papieru” na świecie, `--zoom=1.5` ustawia przybliżenie kamery.

## Mapy

Budynek jest w `client/maps/` (`building.json` + `floorN.json`) — to jedno
źródło dla serwera i klienta. Układ zmieniaj w generatorze, nie w JSON-ach:

```bash
python3 tools/build_maps.py --preview   # podgląd ASCII
godot --path client -s tests/render_maps.gd -- /tmp   # grafika pięter do PNG (bez --headless)
python3 tools/build_maps.py             # zapis JSON-ów
cd server && UPDATE_GOLDEN=1 cargo test --test golden   # nowe wektory testowe
```


## Licencja

Kod: [GNU AGPL-3.0-or-later](LICENSE) — możesz go używać, zmieniać i
udostępniać, ale zmienioną wersję (także uruchomioną jako serwer w sieci)
trzeba udostępnić na tej samej licencji. Czcionka Patrick Hand: SIL OFL 1.1
(`client/fonts/OFL-PatrickHand.txt`). Dźwięki i grafika są generowane kodem z
tego repozytorium.
