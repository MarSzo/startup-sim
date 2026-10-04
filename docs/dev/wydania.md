# Wydania

*Dla deweloperów · [dokumentacja](../README.md) · [uruchomienie](uruchomienie.md)*

## Budowanie klienta na macOS

```bash
tools/build-macos.sh              # → build/StartupSim-<wersja>.dmg
tools/build-macos.sh --app-only   # sama aplikacja, bez podpisu
```

Skrypt najpierw buduje przeglądarkę w grze (`tools/build_webview.sh`, godot_wry
ze źródeł) — Godot wkłada ją do `Contents/Frameworks`, a skrypt podpisuje ją
jako osobny framework przed aplikacją. Aplikacja uniwersalna (Intel + Apple Silicon), podpisana Developer ID z
hardened runtime i uprawnieniem do mikrofonu, a z profilem `notarytool` także
notaryzowana. Kto podpisuje: `IDENTITY` / `NOTARY_PROFILE` w środowisku albo w
`tools/macos/signing.env` (poza repozytorium). `--app-only` — do własnego
podpisania z `tools/macos/entitlements.plist`. Wymaga szablonów eksportu
Godota 4.7.2; wersja w `client/export_presets.cfg`.

## Aktualizacje u graczy

Gra przy starcie sprawdza najnowsze wydanie na GitHubie i, jeśli jest nowsze,
proponuje „Pobierz” na ekranie tytułowym. Serwer odrzucający starą wersję
(inny protokół) też kończy się tym przyciskiem. Szkiców i wydań
„pre-release” klienci nie proponują.

## Nowe wydanie

1. Podbij wersję w `client/project.godot` (`config/version`) i w
   `client/export_presets.cfg` (`application/short_version` i o 1 wyżej
   `application/version`) — po niej klienci poznają, że jest nowsza.
2. Zbuduj dmg: `tools/build-macos.sh`.
3. `git tag -a v<wersja> -m "Startup Sim <wersja>"` i `git push origin v<wersja>`.
4. `gh release create v<wersja> build/StartupSim-<wersja>.dmg --title "Startup Sim <wersja>" --notes "…"`
   (tag `v<wersja>` musi zgadzać się z `config/version`).
5. Przy zmianie protokołu wdróż też serwer (`deploy/deploy.sh`) — stare
   klienty dostaną „pobierz najnowszą”.

## Zwiastun

Nagranie i montaż: [tools/trailer](../../tools/trailer/README.md). GIF do
README robi się z gotowego mp4 (opis tamże).
