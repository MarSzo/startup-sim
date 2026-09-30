## Co i dlaczego

<!-- Krótko: co zmienia ten PR i po co. Link do zgłoszenia: Closes #… -->

## Jak sprawdzone

<!-- Testy, ręczne sprawdzenie w grze (zrzut ekranu mile widziany). -->

## Lista kontrolna

- [ ] CI zielone (rustfmt, clippy, testy serwera i klienta, gdlint, ruff, cargo-deny)
- [ ] Zmiana protokołu → podbite `VERSION` i wpis w `docs/PROTOCOL.md`
- [ ] Zmiana mapy → w `tools/build_maps.py`, odświeżone golden
- [ ] Nowa mechanika → test i opis w `docs/GDD.md` / `docs/ARCHITECTURE.md`
- [ ] Bez adresów serwerów, kluczy i danych graczy w kodzie
