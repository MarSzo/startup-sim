#!/bin/bash
# On the VPS, as root: join the tailnet (the login link is printed — open it
# and approve), so SSH (deploys) can later be closed to everyone else.
#   bash remote-tailscale.sh
set -euo pipefail
command -v tailscale >/dev/null || curl -fsSL https://tailscale.com/install.sh | sh
systemctl enable --now tailscaled
tailscale up --hostname startup-sim
tailscale ip -4
