# Uruchomienie i opcje

*Dla deweloperów · [dokumentacja](../README.md) · [testy](../../tests/README.md) · [wydania](wydania.md) · [mapy](mapy.md)*

Wymagania: Rust (rustup), Godot 4.7 (`godot` w PATH), Python 3 (narzędzia i
testy e2e).

```bash
cd server && cargo run --release            # gra na [::]:7777 (UDP) + logowanie HTTPS na :7778; zapis w server/saves/world.db
godot --path client                         # klient (można odpalić kilka razy): Graj → zaloguj się / załóż konto
cd server && cargo run --release -- --start-with-card --allow-guests   # wariant dla botów: goście z kartą
cd server && cargo run --release -- --reset-password Ola                # administrator: nowe jednorazowe hasło
cd server && cargo run --release -- --list-accounts                     # administrator: lista kont
cd server && cargo run --release --bin bots -- --count 50 --room "Chill room" --all-in-room
```

Serwery na liście w kliencie: `client/net/servers.cfg` (poza repozytorium,
wzór `servers.example.cfg`) — bez niego klient zna tylko serwer lokalny.

Serwer sam robi sobie certyfikat HTTPS do logowania (w `<katalog zapisu>/tls`),
klienty zapamiętują go przy pierwszym połączeniu. Własny certyfikat (np.
Let's Encrypt): `--tls-cert` / `--tls-key`.

## Serwer — opcje

Pełna lista: `cargo run --release -- --help`.

| Opcja | Co robi |
|---|---|
| `--bind <adres>` | adres gry, domyślnie `[::]:7777` (IPv6 + IPv4) |
| `--auth-bind <adres>` | logowanie (HTTPS), domyślnie port gry + 1 |
| `--map <plik>` | budynek, domyślnie `../client/maps/building.json` |
| `--max-players <n>`, `--stats-secs <n>` | limit graczy (256), co ile sekund log statystyk (5) |
| `--save <plik>` | zapis SQLite (domyślnie `saves/world.db`, dzienna kopia w `backups/`) |
| `--no-save` | nic nie wczytuje ani nie zapisuje — wszyscy grają jako goście |
| `--allow-guests` | wpuszcza graczy bez konta (nic im się nie zapisuje) |
| `--tls-cert <pem>`, `--tls-key <pem>` | własny certyfikat logowania |
| `--list-accounts` | administrator: lista kont i wyjście |
| `--reset-password <nick>` | administrator: nowe jednorazowe hasło i wyjście |
| `--recruitment <plik>` | oferty i pytania, domyślnie `data/recruitment.json` |

Do testów i nagrań:

| Opcja | Co robi |
|---|---|
| `--start-with-card` | każdy gracz z kartą pracownika (boty, obciążenie) |
| `--skip-recruitment` | bez portalu, od razu do świata |
| `--start-employed` | od razu zatrudniony: umowa, karta i laptop, start przy biurku (nieparzyste id IT, parzyste Biznes) |
| `--start-cigarettes` | z `--start-employed`: paczka papierosów w kieszeni |
| `--start-time <hh:mm>` | godzina gry na starcie (8:00) |
| `--time-scale <n>` | dzień płynie n razy szybciej (1 h gry = 5 min) |
| `--needs-speed <n>` | potrzeby zmieniają się n razy szybciej |
| `--weather sun\|clouds\|rain\|storm\|fog` | stała pogoda |
| `--treats` | od razu taca słodyczy w chill roomie |
| `--stale-fruit <proc>` | szansa na nieświeży owoc (15) |
| `--cleaning-at <hh:mm>` | dokładny start obchodu sprzątaczki (domyślnie losowo 15:00–16:00) |
| `--lag-ms <ms>`, `--jitter-ms <ms>`, `--loss <0..1>` | symulacja sieci (opóźnienie w jedną stronę — RTT rośnie 2×) |

Np. RTT ~100 ms i 2% strat: `cargo run --release -- --lag-ms 50 --jitter-ms 10 --loss 0.02`.

## Klient — argumenty deweloperskie

Po `--`, np. `godot --path client -- --nick=Ala --autoconnect`.

| Argument | Co robi |
|---|---|
| `--nick=Ala --server=127.0.0.1:7777 --autoconnect` | wejście jako gość (serwer z `--allow-guests` albo `--no-save`); adres może być IPv6: `--server=[::1]:7777` |
| `--login=Ola:haslo [--register] [--autocreate]` | logowanie / rejestracja przez ekran logowania |
| `--login-screen` | od razu ekran logowania |
| `--debug` | F3 od startu |
| `--commute=3` | co rano wybierz dojazd (1 pieszo … 5 tramwaj) |
| `--found="Nazwa firmy"` | załóż firmę z portalu |
| `--auto-recruit=1 [--auto-recruit-delay=2]` | sam aplikuje na ofertę 1 i zgaduje odpowiedzi do skutku |
| `--voice-tone` | czat głosowy nadaje ton testowy zamiast mikrofonu |
| `--autowalk` | losowy ruch |
| `--goto="…"` | scenariusz kroków (niżej) |
| `--record=/katalog --record-start=2 --record-length=6` | klatki JPG do zwiastuna ([tools/trailer](../../tools/trailer/README.md)) |
| `--screenshot=/tmp/x.png --screenshot-delay=5` | zapis klatki i wyjście; `--screenshot-delay=5,12,20` zapisuje `x_1.png`, `x_2.png`, … |
| `--perf` | co 2 s: FPS, wywołania rysowania, co się najczęściej przerysowuje |
| `--zoom=1.5` | przybliżenie kamery |
| `--no-mood` | bez efektu „tuszu i papieru” (tylko do testów — w grze jest zawsze) |

### `--goto` — kroki oddzielone `;`

- `34,49` — idź na kafel; `Sklep` — idź do pokoju (na bieżącym piętrze);
- `E` — wciśnij E; `L` — zamknij / otwórz kabinę; `wait:N` — czekaj N s;
- `item:take0|put|drop|give|use` — przedmioty; `dlg:<nr>` — odpowiedz w oknie rozmowy;
- `talk:N` / `whisper:N` — mów / szeptaj N s (z `--voice-tone`);
- komputer: `pc:say:general|dept|dm:<imię>:<tekst>`, `pc:open:…`,
  `pc:win:mail|trash|browser|tasks|lunch|calendar|company|chat`,
  `pc:task:<tytuł>`, `pc:card:<n>`, `pc:comment:<tekst>`, `pc:take-card`,
  `pc:mail:<nick>:<temat>`, `pc:read:<n>`, `pc:lock`, `pc:unlock`, `pc:take`,
  `pc:close`, kalendarz `pc:cal:<minuta>:<temat>`, obiad `pc:lunch:<danie>`,
  panel firmy `pc:company` / `pc:company:hire` /
  `pc:company:<akcja>:<cel>:<wartość>[:<tekst>]`.

Przykład: `--goto="34,49;E;wait:2;28,42;Sklep"` — rozmowa z portierem, pod
drzwi klatki schodowej, potem do sklepu.

## CI

GitHub Actions (`.github/workflows/ci.yml`) przy każdym pull requeście i
pushu na `main`: rustfmt, clippy i testy serwera, cargo-deny (podatności i
licencje zależności), gdlint i testy klienta w Godocie, ruff i zgodność map z
generatorem, gitleaks. Scenariusze e2e — tylko w pull requestach zmieniających
grę, w 3 równoległych częściach ([tests/README.md](../../tests/README.md)).

## Dźwięki i czcionka

Dźwięki generuje `python3 tools/sounds/gen_sounds.py`. Interfejs używa
odręcznej czcionki **Patrick Hand** (© Patrick Wagesreiter, SIL OFL 1.1) —
plik i licencja w `client/fonts/`.

## Wdrożenie na VPS

Serwer testowy działa na VPS-ie; SSH tylko z tailnetu (Tailscale):

```bash
deploy/deploy.sh          # kod → VPS, build, restart (gra zapisuje się przy restarcie)
deploy/pull-backups.sh    # dzienne kopie bazy do vps-backups/
deploy/pull-crashes.sh    # raporty awarii klientów do crash-reports/
```

Porty gry (7777/udp, 7778/tcp) są publiczne. Pierwsza konfiguracja, zapora i
Tailscale: [deploy/README.md](../../deploy/README.md).
