#!/bin/bash
# On the VPS, as root: SSH only from the tailnet (the game ports stay open).
# Run it only after `ssh root@startup-sim` works over Tailscale.
set -euo pipefail
tailscale ip -4 >/dev/null || { echo "Tailscale is not up — not closing SSH." >&2; exit 1; }
ufw allow in on tailscale0 to any port 22 proto tcp comment 'SSH tylko z tailnetu'
ufw delete allow OpenSSH || true
ufw delete allow 22/tcp 2>/dev/null || true
ufw status verbose
