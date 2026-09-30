#!/bin/bash
# From your computer: the VPS's own login certificate into the client, so
# players trust it from the very first connection (no "trust on first use").
#   deploy/pull-cert.sh <game address, e.g. 203.0.113.10:7777> [root@startup-sim]
# (The address as in client/net/servers.cfg; the file is not in the repo.)
set -euo pipefail
ADDR=${1:?usage: deploy/pull-cert.sh <host:port> [ssh host]}
HOST=${2:-root@startup-sim}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$ROOT/client/net/pins"
scp "$HOST:/var/lib/startup-sim/tls/cert.pem" "$ROOT/client/net/pins/${ADDR//:/_}.pem"
echo "pinned: client/net/pins/${ADDR//:/_}.pem"
