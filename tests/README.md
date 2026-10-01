# Testy rozgrywki i obciążenia

Testy jednostkowe, golden i e2e serwera są w `server/` (`cargo test`), parytet
klienta z serwerem w `client/tests/run_tests.gd`. Tutaj: testy całej gry.

## Scenariusze rozgrywki (`tests/e2e/`)

Prawdziwy serwer (wydanie release) i prawdziwy klient Godot uruchomiony bez
okna (`--headless`), sterowany skryptem scenariusza — tak, jakby grał człowiek:
chodzi (także po schodach między piętrami), naciska E, siada do komputera,
pisze na komunikatorze, zamawia obiad.

```bash
python3 tests/e2e/run.py                 # wszystkie (ok. 3 min)
python3 tests/e2e/run.py founder         # jeden
python3 tests/e2e/run.py --list
GODOT=/ścieżka/do/godot python3 tests/e2e/run.py
```

| scenariusz | co sprawdza |
|------------|-------------|
| `workday` | pracownik: laptop na biurku, obiad z aplikacji, kawa z aneksu, obiad z recepcji, powrót tramwajem z wypłatą |
| `onboarding` | nowe konto: rejestracja, postać, portal i rozmowa, portier, recepcja, HR (karta + laptop), laptop na biurku działu |
| `together` | dwóch graczy: wiadomości w komunikatorze w obie strony, spotkanie w chill roomie, zjazd windą |
| `founder` | założyciel: firma z portalu, stanowisko w dziale Mobile, drugi gracz aplikuje, zatrudnienie |
| `persistence` | laptop na biurku, restart serwera, ponowne logowanie: postać, karta i laptop na miejscu |

Każdy scenariusz dostaje własny serwer (osobny port, katalog tymczasowy z
zapisem) i własny folder klienta `user://e2e/<scenariusz>/` — nie rusza
Twojego zapamiętanego logowania ani ustawień. Wynik: kod wyjścia klienta i
brak `SCRIPT ERROR` w jego logu; przy błędzie `run.py` wypisuje końcówkę logu
(wszystkie logi zostają w katalogu z `--logs`).

### Nowy scenariusz

1. Plik `client/tests/e2e/scenarios/<nazwa>.gd`:

   ```gdscript
   extends "res://tests/e2e/scenario.gd"

   func run() -> void:
       if not await until(in_world, 30.0, "w biurze"):
           return
       if not await walk(1, 25, 8):        # piętro, x, y (drogę liczy sam)
           return
       await press_e()
       await hear("Kawa gotowa", 8.0)       # czeka na wypowiedź
   ```

   Pomocnicze: `walk`, `press_e`, `until(warunek, sekundy, opis)`, `hear`,
   `wait`, `pc("polecenie komputera")`, `item(akcja)`, `sit_at_desk`,
   `desk_of(dział)`, stan: `room_name()`, `floor_now()`, `tile_now()`,
   `holding()`, `carrying()`, `money()`, `access()`, `sees(nick)`,
   `got_chat(tekst)`, `last[typ pakietu]`; `check(warunek, opis)` kończy
   scenariusz błędem. Zwykle kończy się sam po `run()`.
2. Wpis w `SCENARIOS` w `tests/e2e/run.py`: argumenty serwera (np.
   `--start-employed`, `--time-scale`) i klientów (`--nick=… --autoconnect`
   albo `--login=nick:hasło --register --autocreate`); kilku klientów naraz =
   gra wieloosobowa, kolejna faza z `"server": "restart"` = restart serwera.

## Obciążenie (`tests/load/soak.py`)

Serwer release i N botów (`server/src/bin/bots.rs`) chodzących po budynku
(połowa w jednym pokoju) przez kilka minut. Zielony, gdy: wszystkie boty
weszły, zero zgubionych ticków, najdłuższy tick < 25 ms (budżet 50 ms), serwer
się nie wysypał i pamięć nie rosła.

```bash
python3 tests/load/soak.py                       # 50 botów, 3 min
python3 tests/load/soak.py --bots 100 --minutes 10
```

W CI: scenariusze przy każdym pull requeście (zadanie „Rozgrywka”),
obciążenie co noc i ręcznie (workflow „Obciążenie”).
