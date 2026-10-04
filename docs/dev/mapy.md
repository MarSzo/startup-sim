# Mapy

*Dla deweloperów · [dokumentacja](../README.md) · [budynek w GDD](../gdd/swiat.md)*

Budynek jest w `client/maps/` (`building.json` + `floorN.json`) — to jedno
źródło dla serwera i klienta. Układ zmieniaj w generatorze
`tools/build_maps.py`, nie w JSON-ach:

```bash
python3 tools/build_maps.py --preview                  # podgląd ASCII
godot --path client -s tests/render_maps.gd -- /tmp    # grafika pięter do PNG (bez --headless)
python3 tools/build_maps.py                            # zapis JSON-ów
cd server && UPDATE_GOLDEN=1 cargo test --test golden  # nowe wektory testowe
```

CI sprawdza, że JSON-y w repozytorium zgadzają się z generatorem. Format
mapy i jak serwer z niej korzysta: [architektura](architektura.md#mapa).
