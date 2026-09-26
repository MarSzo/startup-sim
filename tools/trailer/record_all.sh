#!/bin/zsh
# Records every shot of the trailer into out/<shot>/ (JPG frames).
# Needs: release build of the server and bots (cargo build --release), godot.
set -eu
S=${0:A:h}/shoot.sh
$S s1_title "" "" 2 4.5
$S s5_rain "--skip-recruitment --start-with-card --start-time 8:30 --weather rain" \
  "--nick=Mati --autoconnect --goto='wait:3.5;20,35;wait:0.5;8,35'" 5 5
$S s2_office "--start-employed --start-time 10:30 --weather sunny" \
  "--nick=Mati --autoconnect --goto='wait:3;4,12;wait:0.3;17,17'" 6 5 12 "IT / Produkt"
$S s3_coffee "--start-employed --start-time 11:00 --weather sunny" \
  "--nick=Mati --autoconnect --goto='wait:0.5;E;wait:0.8;35,27;E;wait:1;36,27;E;wait:8'" 10 6 10 "Chill room"
$S s4_smoke "--start-employed --start-cigarettes --start-time 13:00" \
  "--nick=Mati --autoconnect --goto='wait:0.5;E;wait:0.8;53,27;E;wait:0.3;52,30;item:take1;wait:0.5;item:use;wait:30'" 26 6
$S s6_commute "--start-employed --start-time 5:00" "--nick=Mati --autoconnect" 9 4
