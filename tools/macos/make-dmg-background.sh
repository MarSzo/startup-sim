#!/bin/bash
# Renders dmg-background.svg plus the title (in the game's font) into
# dmg-background.tiff, 1x + 2x for Retina. Run after changing the SVG; the
# TIFF is committed so build-macos.sh doesn't need rsvg-convert or magick.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
FONT="$HERE/../../client/fonts/PatrickHand-Regular.ttf"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

for scale in 1 2; do
  rsvg-convert -w $((660 * scale)) -h $((400 * scale)) "$HERE/dmg-background.svg" -o "$TMP/sky.png"
  magick "$TMP/sky.png" -font "$FONT" -gravity north \
    -fill "#f4ead0" -pointsize $((40 * scale)) -annotate +0+$((20 * scale)) "Startup Sim" \
    -fill "#c2af8a" -pointsize $((18 * scale)) -annotate +0+$((74 * scale)) "Przeciągnij grę na folder Aplikacje" \
    -density $((72 * scale)) -units PixelsPerInch "$TMP/bg$scale.png"
done
tiffutil -cathidpicheck "$TMP/bg1.png" "$TMP/bg2.png" -out "$HERE/dmg-background.tiff"
