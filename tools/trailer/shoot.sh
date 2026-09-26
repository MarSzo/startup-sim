#!/bin/zsh
# Records one trailer shot (server + optional bots + client with --record):
# shoot.sh <name> "<server flags>" "<client args>" <start> <len> [bots] [bot room]
set -u
R=${0:A:h:h:h}            # repo root
V=${0:A:h}/out
mkdir -p $V
name=$1; sflags=$2; cargs=$3; start=$4; len=$5; bots=${6:-0}; room=${7:-Chill room}
PORT=7790
rm -rf $V/$name
pkill -f "server --bind 127.0.0.1:$PORT" 2>/dev/null
pkill -f "bots --server 127.0.0.1:$PORT" 2>/dev/null
sleep 0.3
eval "$R/server/target/release/server --bind 127.0.0.1:$PORT $sflags" > $V/$name.srv.log 2>&1 &
sleep 0.8
if [ $bots -gt 0 ]; then
  $R/server/target/release/bots --server 127.0.0.1:$PORT --count $bots --room "$room" --room-share 0.6 \
    --nicks "Ola,Kuba,Zosia,Bartek,Ania,Tomek,Kasia,Piotr,Magda,Wojtek,Ewa,Marek,Iga,Filip" > $V/$name.bots.log 2>&1 &
  sleep 1.5
fi
eval "godot --path $R/client --windowed -- --server=127.0.0.1:$PORT --record=$V/$name --record-start=$start --record-length=$len $cargs" > $V/$name.cli.log 2>&1
pkill -f "bots --server 127.0.0.1:$PORT" 2>/dev/null
pkill -f "server --bind 127.0.0.1:$PORT" 2>/dev/null
echo "$name: $(ls $V/$name | wc -l) frames"
