#!/bin/bash
# From your computer: the VPS's own login certificate into the client, so
# players trust it from the very first connection (no "trust on first use").
#   deploy/pull-cert.sh [root@178.105.233.184] [178.105.233.184:7777]
set -euo pipefail
HOST=${1:-root@178.105.233.184}
ADDR=${2:-178.105.233.184:7777}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$ROOT/client/net/pins"
scp "$HOST:/var/lib/startup-sim/tls/cert.pem" "$ROOT/client/net/pins/${ADDR//:/_}.pem"
echo "pinned: client/net/pins/${ADDR//:/_}.pem"
