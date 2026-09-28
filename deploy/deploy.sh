#!/bin/bash
# From your computer: send the server sources and the maps to the VPS, set it
# up (first time) and build + restart it.
#   deploy/deploy.sh [root@startup-sim]
set -euo pipefail
HOST=${1:-root@startup-sim}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
ssh "$HOST" 'command -v rsync >/dev/null || (apt-get update -q && apt-get install -y -q rsync); mkdir -p /opt/startup-sim/src/client /opt/startup-sim/deploy'
rsync -az --delete --exclude target --exclude saves "$ROOT/server/" "$HOST:/opt/startup-sim/src/server/"
rsync -az --delete "$ROOT/client/maps/" "$HOST:/opt/startup-sim/src/client/maps/"
rsync -az --delete "$ROOT/deploy/" "$HOST:/opt/startup-sim/deploy/"
ssh "$HOST" 'bash /opt/startup-sim/deploy/remote-setup.sh && bash /opt/startup-sim/deploy/remote-build.sh'
