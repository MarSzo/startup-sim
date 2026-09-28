#!/bin/bash
# On the VPS, as root: build the server, swap the binary (the running server
# saves the game on SIGTERM), start it again.
set -euo pipefail
export PATH=/root/.cargo/bin:$PATH
cd /opt/startup-sim/src/server
cargo build --release --bin server
systemctl stop startup-sim || true
install -m 755 target/release/server /opt/startup-sim/bin/server
systemctl start startup-sim
sleep 2
systemctl --no-pager --lines=8 status startup-sim
