#!/bin/bash
# From your computer: send the server sources and the maps to the VPS, set it
# up (first time) and build + restart it.
#   deploy/deploy.sh [root@startup-sim]
#
# Each deploy is also recorded on GitHub (Deployments, environment
# "serwer-testowy": which commit, when, success or failure) - with `gh`
# logged in and the commit pushed; no server address goes there.
# NO_GITHUB_DEPLOY=1 skips that.
set -euo pipefail
HOST=${1:-root@startup-sim}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
ENVIRONMENT="serwer-testowy"

# --- GitHub Deployments (best effort: never stops the deploy) ---------------
GH_REPO=""
DEPLOY_ID=""
gh_status() {  # gh_status <state> <description>
  [ -n "$DEPLOY_ID" ] || return 0
  gh api --silent -X POST "repos/$GH_REPO/deployments/$DEPLOY_ID/statuses" \
    -f state="$1" -f description="$2" -f environment="$ENVIRONMENT" -F auto_inactive=true 2>/dev/null \
    || echo "github: nie udało się ustawić statusu wdrożenia ($1)" >&2
}
gh_start() {
  [ "${NO_GITHUB_DEPLOY:-0}" = "1" ] && return 0
  command -v gh >/dev/null && gh auth status >/dev/null 2>&1 || { echo "github: brak gh / logowania — wdrożenie bez wpisu na GitHubie" >&2; return 0; }
  local sha dirty=""
  sha=$(git -C "$ROOT" rev-parse HEAD)
  # The commit must be on GitHub (a deployment points at it).
  if [ -z "$(git -C "$ROOT" branch -r --contains "$sha" 2>/dev/null)" ]; then
    echo "github: commit $sha nie jest wypchnięty — wdrożenie bez wpisu na GitHubie" >&2
    return 0
  fi
  git -C "$ROOT" diff --quiet HEAD -- server client/maps deploy || dirty=" + lokalne zmiany"
  GH_REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null) || return 0
  # (No status checks required: the deploy is of what's on this machine.)
  DEPLOY_ID=$(printf '{"ref":"%s","environment":"%s","auto_merge":false,"production_environment":false,"required_contexts":[],"description":"deploy.sh: serwer %s%s"}' \
      "$sha" "$ENVIRONMENT" "${sha:0:7}" "$dirty" \
    | gh api -X POST "repos/$GH_REPO/deployments" --input - -q .id 2>/dev/null) || DEPLOY_ID=""
  if [ -n "$DEPLOY_ID" ]; then
    echo "github: wdrożenie #$DEPLOY_ID ($ENVIRONMENT, ${sha:0:7}$dirty)"
    gh_status in_progress "Budowanie i restart serwera"
  else
    echo "github: nie udało się zapisać wdrożenia — wdrażam dalej" >&2
  fi
}
trap 'gh_status failure "Wdrożenie nie powiodło się"' ERR
gh_start

ssh "$HOST" 'command -v rsync >/dev/null || (apt-get update -q && apt-get install -y -q rsync); mkdir -p /opt/startup-sim/src/client /opt/startup-sim/deploy'
rsync -az --delete --exclude target --exclude saves "$ROOT/server/" "$HOST:/opt/startup-sim/src/server/"
rsync -az --delete "$ROOT/client/maps/" "$HOST:/opt/startup-sim/src/client/maps/"
rsync -az --delete "$ROOT/deploy/" "$HOST:/opt/startup-sim/deploy/"
ssh "$HOST" 'bash /opt/startup-sim/deploy/remote-setup.sh && bash /opt/startup-sim/deploy/remote-build.sh'

trap - ERR
gh_status success "Serwer zbudowany i uruchomiony"
