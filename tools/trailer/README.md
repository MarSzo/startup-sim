# Zwiastun

Nagrywanie ujęć i montaż krótkiego zwiastuna (~30 s, 1920×1080, MP4).

```
cd server && cargo build --release && cd ..
tools/trailer/record_all.sh     # ujęcia -> tools/trailer/out/<ujęcie>/*.jpg
tools/trailer/build.sh          # -> tools/trailer/out/startup_sim_zwiastun.mp4
```

Ujęcia (`record_all.sh`): plansza tytułowa, dojazd w deszczu, hol z panią
Wiesią, biuro z botami, chill room (mecz w telewizorze, boombox), sklep na
parterze. Każde to osobny serwer z flagami dev i klient z `--goto`; start
nagrywania jest przesunięty o czas łączenia klienta.

GIF do README (`docs/media/zwiastun.gif`, 640 px, 8 kl./s — GitHub nie
odtwarza mp4 z repozytorium w README):

```
F=$(mktemp -d)
ffmpeg -i tools/trailer/out/startup_sim_zwiastun.mp4 -vf "fps=8,scale=640:-1:flags=lanczos" $F/f%04d.png
ffmpeg -framerate 8 -i $F/f%04d.png -vf "palettegen=max_colors=96:stats_mode=diff" $F/pal.png
ffmpeg -framerate 8 -i $F/f%04d.png -i $F/pal.png -lavfi "paletteuse=dither=none:diff_mode=rectangle" docs/media/zwiastun.gif
```

Zrzuty ekranu w `docs/media/` to klatki z `tools/trailer/out/<ujęcie>/`
przycięte do 1280×720 (`magick … -resize 1280x720^ -gravity center -extent 1280x720`).

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
