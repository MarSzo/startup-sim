#!/bin/bash
# From your computer: copy the VPS's daily backups here (vps-backups/).
#   deploy/pull-backups.sh [root@startup-sim]
set -euo pipefail
HOST=${1:-root@startup-sim}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$ROOT/vps-backups"
rsync -az "$HOST:/var/lib/startup-sim/backups/" "$ROOT/vps-backups/"
ls -lh "$ROOT/vps-backups" | tail -8
