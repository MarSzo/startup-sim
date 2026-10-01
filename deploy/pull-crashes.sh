#!/bin/bash
# From your computer: copy the client crash reports from the VPS here
# (crash-reports/, not in the repository) and list the newest.
#   deploy/pull-crashes.sh [root@startup-sim]
set -euo pipefail
HOST=${1:-root@startup-sim}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$ROOT/crash-reports"
rsync -az "$HOST:/var/lib/startup-sim/crashes/" "$ROOT/crash-reports/" 2>/dev/null || { echo "brak raportów na serwerze"; exit 0; }
ls -t "$ROOT/crash-reports" | head -10
