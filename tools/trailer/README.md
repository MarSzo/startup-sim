# Zwiastun

Nagrywanie ujęć i montaż krótkiego zwiastuna (~30 s, 1920×1080, MP4).

```
cd server && cargo build --release && cd ..
tools/trailer/record_all.sh     # ujęcia -> tools/trailer/out/<ujęcie>/*.jpg
tools/trailer/build.sh          # -> tools/trailer/out/startup_sim_zwiastun.mp4
```

- `shoot.sh` — jedno ujęcie: serwer na porcie 7790 (flagi dev), opcjonalnie
  boty z imionami (`--nicks`), klient z `--goto` i `--record` (klatki JPG w
  stałym tempie, `client/dev_recorder.gd`).
- `build.sh` — napisy (ramki „papier i tusz”, czcionka Patrick Hand),
  przejścia, plansza końcowa ze splasha, muzyka z `music.py`.
- `sfx.py` — dźwięki gry (client/sounds) ułożone na osi czasu zwiastuna;
  muzyka jest pod nimi ściszana.
- `music.py` — pętla lo-fi syntetyzowana od zera (bez próbek i cudzych praw).

Wymaga: `godot`, `ffmpeg`, `magick` (ImageMagick), `python3`.
Rozdzielczość klatek = okno gry (pełny ekran z ustawień też działa).
