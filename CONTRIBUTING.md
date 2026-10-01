# Jak pomóc

*English: contributions are welcome — issues and pull requests may be in
English or Polish. The game and its docs are in Polish.*

Dzięki, że chcesz coś dodać albo poprawić! Poniżej wszystko, co trzeba wiedzieć.

## Zanim zaczniesz

- **Błąd** — zgłoś go w [Issues](../../issues) (szablon „Błąd”): co robiłeś,
  co się stało, czego oczekiwałeś, wersja klienta i system.
- **Nowa funkcja** — najpierw opisz pomysł w Issues (szablon „Pomysł”), żeby
  ustalić, czy pasuje do gry, zanim włożysz w nią pracę. Opis gry i jej
  założenia: [docs/GDD.md](docs/GDD.md).
- **Luka bezpieczeństwa** — nie w Issues, patrz [SECURITY.md](SECURITY.md).

## Uruchomienie u siebie

Wymagania: Rust (rustup), Godot 4.7.2 (`godot` w PATH), Python 3.

```bash
cd server && cargo run --release        # serwer: gra :7777 (UDP), logowanie :7778 (HTTPS)
godot --path client                     # klient: z edytora widać „Serwer lokalny (dev)”
```

Więcej opcji (boty, szybszy czas, start od razu w biurze): [README](README.md).
Jak to działa: [architektura](docs/ARCHITECTURE.md), [protokół](docs/PROTOCOL.md).

## Zasady zmian

- Serwer jest **autorytatywny**: logika gry po stronie Rusta, klient tylko
  pokazuje i przewiduje ruch. Ruch (`server/src/sim.rs` ↔
  `client/sim/movement.gd`) i protokół muszą zgadzać się bit w bit — pilnują
  tego pliki golden w `server/tests/golden/`.
- **Zmiana formatu pakietu** = podbicie `VERSION` (Rust i GDScript) i wpis w
  „Historii wersji” w `docs/PROTOCOL.md`; po celowej zmianie:
  `cd server && UPDATE_GOLDEN=1 cargo test --test golden`.
- **Mapy** zmieniaj w `tools/build_maps.py`, nie w JSON-ach
  (`python3 tools/build_maps.py`), potem odśwież wektory golden jak wyżej.
- Styl: taki jak w otaczającym kodzie — komentarze po angielsku, teksty w grze
  po polsku; nazwy i gęstość komentarzy jak obok.
- Nowa mechanika = test (jednostkowy w module, e2e w `server/tests/`, parytet
  w `client/tests/run_tests.gd`, a większa rzecz w grze — scenariusz w
  `client/tests/e2e/scenarios/`, patrz `tests/README.md`) i akapit w GDD /
  ARCHITECTURE.
- **Nie commituj** adresów serwerów, certyfikatów, kluczy ani danych graczy:
  `client/net/servers.cfg`, `client/net/pins/*.pem` i
  `tools/macos/signing.env` są w `.gitignore` z powodu.

## Sprawdź przed pull requestem

To samo sprawdza CI (musi być zielone, żeby scalić):

```bash
cd server && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd server && cargo deny check                       # cargo install cargo-deny
godot --headless --path client --import && godot --headless --path client -s tests/run_tests.gd
cd client && gdlint .                               # pipx install "gdtoolkit==4.*"
ruff check tools tests && python3 tools/build_maps.py && git diff --exit-code client/maps
python3 tests/e2e/run.py                            # scenariusze rozgrywki (ok. 3 min)
```

## Pull request

1. Fork, gałąź od `main` (np. `fix/winda-drzwi`, `feat/automat-z-kawa`).
2. Małe, spójne zmiany; opis co i dlaczego (szablon podpowie).
3. PR do `main` — wymaga zielonego CI i akceptacji opiekuna; `main` nie
   przyjmuje bezpośrednich pushy.

## Licencja

Wysyłając zmiany, zgadzasz się, że będą na licencji projektu:
[AGPL-3.0-or-later](LICENSE).
