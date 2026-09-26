#!/bin/zsh
# Trailer assembly: captions (paper-and-ink boxes), clips, end card, music.
set -eu
T=${0:A:h}
R=${T:h:h}
cd $T/out
FONT=$R/client/fonts/PatrickHand-Regular.ttf
BG='#1f1a2e'
rm -rf build; mkdir -p build

caption() {  # caption <out.png> <text> [gravity]
  magick -background '#f2e7cb' -fill '#2a2118' -font $FONT -pointsize 58 label:"$2" \
    -bordercolor '#f2e7cb' -border 34x14 -bordercolor '#2a2118' -border 5 \
    \( +clone -background '#000000' -shadow 60x6+6+8 \) +swap -background none -layers merge +repage build/cap.png
  magick -size 1920x1080 xc:none build/cap.png -gravity ${3:-north} -geometry +0+150 -composite $1
}

clip() {  # clip <name> <dir> <start frame> <frames> [caption] [gravity]
  local name=$1 dir=$2 st=$3 fr=$4 cap=${5:-} grav=${6:-north}
  local d=$(( fr / 30.0 ))
  local base="scale=1720:1080:flags=lanczos,pad=1920:1080:(ow-iw)/2:0:color=$BG,fps=30,format=yuv420p"
  if [[ -n $cap ]]; then
    caption build/$name.cap.png "$cap" $grav
    ffmpeg -loglevel error -y -framerate 30 -start_number $st -i $dir/%05d.jpg -loop 1 -framerate 30 -i build/$name.cap.png \
      -filter_complex "[0:v]${base}[v];[1:v]format=rgba,fade=t=in:st=0.25:d=0.35:alpha=1[c];[v][c]overlay=0:0:shortest=1,fade=t=in:st=0:d=0.3,fade=t=out:st=$((d-0.3)):d=0.3,format=yuv420p" \
      -frames:v $fr -c:v libx264 -crf 17 -preset medium build/$name.mp4
  else
    ffmpeg -loglevel error -y -framerate 30 -start_number $st -i $dir/%05d.jpg \
      -vf "$base,fade=t=in:st=0:d=0.3,fade=t=out:st=$((d-0.3)):d=0.3" -frames:v $fr -c:v libx264 -crf 17 -preset medium build/$name.mp4
  fi
  echo "build/$name.mp4" >> build/list.txt.tmp
}

clip c1 s1_title 0 135
clip c2 s5_rain 0 150 "Dojedź do pracy — nawet w deszczu"
clip c3 s2_office 0 150 "Pracuj w startupie razem z innymi graczami"
clip c4 s3_coffee 30 150 "Kawa z ekspresu… jeśli znajdziesz czysty kubek"
clip c5 s4_smoke 0 150 "Jeden szybki papieros w łazience? Co może pójść nie tak…"
clip c6 s6_commute 0 105 "A jutro? Pieszo, rowerem, autem, taksówką czy tramwajem?" south

# End card: the splash (logo + name), tagline, "wkrótce".
magick $R/client/icons/splash.png -resize 1920x1080 \
  -font $FONT -fill '#c2af8a' -pointsize 54 -gravity center -annotate +0+400 'Symulator pracy w startupie IT  ·  wkrótce' build/end.png
ffmpeg -loglevel error -y -loop 1 -framerate 30 -i build/end.png -vf "fade=t=in:st=0:d=0.5,fade=t=out:st=3.5:d=0.5,format=yuv420p" \
  -frames:v 120 -c:v libx264 -crf 17 build/c7.mp4
echo build/c7.mp4 >> build/list.txt.tmp

sed "s/^/file '/; s/$/'/" build/list.txt.tmp | sed "s#file 'build/#file '#" > build/list.txt
ffmpeg -loglevel error -y -f concat -safe 0 -i build/list.txt -c copy build/video.mp4
DUR=$(ffprobe -v error -show_entries format=duration -of csv=p=0 build/video.mp4)
python3 $T/music.py build/music.wav $DUR
ffmpeg -loglevel error -y -i build/video.mp4 -i build/music.wav -c:v copy -c:a aac -b:a 192k -shortest -movflags +faststart startup_sim_zwiastun.mp4
ffprobe -v error -show_entries format=duration,size -of default=nw=1 startup_sim_zwiastun.mp4
