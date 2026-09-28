#!/bin/bash
# On the VPS, as root (idempotent): packages, the service user, directories,
# the Rust toolchain, the firewall and the systemd unit.
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
apt-get update -q
apt-get install -y -q build-essential pkg-config cmake curl rsync ufw
id startup-sim >/dev/null 2>&1 || useradd --system --home-dir /var/lib/startup-sim --shell /usr/sbin/nologin startup-sim
install -d -m 755 /opt/startup-sim/src /opt/startup-sim/bin
install -d -m 750 -o startup-sim -g startup-sim /var/lib/startup-sim
if ! command -v cargo >/dev/null && [ ! -x /root/.cargo/bin/cargo ]; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi
# Firewall: SSH first (so we don't lock ourselves out) — only from the
# tailnet once the VPS is on it (deploy/ssh-tailnet-only.sh) — then the game.
if tailscale ip -4 >/dev/null 2>&1 && ufw status | grep -q tailscale0; then
  ufw delete allow OpenSSH >/dev/null 2>&1 || true
else
  ufw allow OpenSSH
fi
ufw allow 7777/udp comment 'Startup Sim: gra'
ufw allow 7778/tcp comment 'Startup Sim: logowanie HTTPS'
ufw --force enable
install -m 644 /opt/startup-sim/deploy/startup-sim.service /etc/systemd/system/startup-sim.service
systemctl daemon-reload
systemctl enable startup-sim >/dev/null
echo "setup ok"
