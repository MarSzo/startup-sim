#!/bin/zsh
# Records every shot of the trailer into out/<shot>/ (JPG frames).
# Needs: release build of the server and bots (cargo build --release), godot.
# shoot.sh <name> "<server flags>" "<client args>" <record start s> <length s> [bots] [bot room]
set -eu
S=${0:A:h}/shoot.sh
$S s1_title "" "" 2 4.5
# A rainy morning on the sidewalk.
$S s2_rain "--skip-recruitment --start-with-card --start-time 8:30 --weather rain" \
  "--nick=Mati --autoconnect --goto='wait:2;14,59;wait:2'" 2 5
# In through the draught lobby: Pani Wiesia says hello (and asks about the wedding).
$S s3_hall "--skip-recruitment --start-with-card --start-time 8:40 --weather sunny" \
  "--nick=Mati --autoconnect --goto='wait:1;31,58;31,55;31,51;33,50;wait:4'" 4.5 5
# Our team's room upstairs, full of colleagues (the bots join first, so the
# player gets an even id: the Biznes room).
$S s4_office "--start-employed --start-time 10:30 --weather sunny" \
  "--nick=Mati --autoconnect --goto='wait:8'" 3 5 7 "Biznes"
# The chill room: the match on the TV, disco polo from the boombox.
$S s5_chill "--start-employed --start-time 11:00 --weather sunny" \
  "--nick=Mati --autoconnect --goto='wait:0.5;item:drop;34,12;E;wait:0.6;item:take1;wait:0.4;item:use;wait:0.8;dlg:3;wait:0.6;item:put;wait:0.3;41,12;E;wait:0.6;item:use;wait:0.8;dlg:0;wait:8'" 16 5 5 "Chill room"
# The shop: a beer off the shelf, and the cashier's question at the counter.
$S s6_shop "--skip-recruitment --start-with-card --start-time 12:00 --weather sunny" \
  "--nick=Mati --autoconnect --goto='wait:0.5;22,58;22,55;21,50;20,48;shop:5:21;wait:0.4;20,53;wait:6'" 9 5
