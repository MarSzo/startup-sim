#!/usr/bin/env python3
"""Load test: the release server with N bots walking around the building for
a while (a share of them crowded into one room). Passes when the server
missed no ticks, its slowest tick stayed under the budget, it didn't crash
and its memory didn't keep growing.

    python3 tests/load/soak.py                     # 50 bots, 3 minutes
    python3 tests/load/soak.py --bots 80 --minutes 10
"""
import argparse
import os
import re
import signal
import subprocess
import sys
import tempfile
import time

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
SERVER_DIR = os.path.join(ROOT, "server")
TICK_BUDGET_US = 50_000   # 20 Hz
SLOW_TICK_US = 25_000     # the slowest tick allowed (half the budget)
MEMORY_GROWTH = 1.5       # end / after warm-up


def rss_kb(pid):
    out = subprocess.run(["ps", "-o", "rss=", "-p", str(pid)], capture_output=True, text=True).stdout.strip()
    return int(out) if out else 0


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--bots", type=int, default=50)
    ap.add_argument("--minutes", type=float, default=3)
    ap.add_argument("--room", default="Chill room")
    ap.add_argument("--port", type=int, default=7990)
    a = ap.parse_args()

    print("building…", flush=True)
    subprocess.run(["cargo", "build", "--release", "--bins", "-q"], cwd=SERVER_DIR, check=True)
    server_bin = os.path.join(SERVER_DIR, "target", "release", "server")
    bots_bin = os.path.join(SERVER_DIR, "target", "release", "bots")
    log_path = tempfile.mktemp(prefix="soak-server-", suffix=".log")
    addr = "127.0.0.1:%d" % a.port
    seconds = int(a.minutes * 60)

    with open(log_path, "w") as log:
        server = subprocess.Popen(
            [server_bin, "--bind", addr, "--no-save", "--allow-guests", "--start-with-card", "--stats-secs", "5"],
            cwd=SERVER_DIR, stdout=log, stderr=subprocess.STDOUT)
        time.sleep(1.0)
        bots = subprocess.Popen(
            [bots_bin, "--server", addr, "--count", str(a.bots), "--room", a.room, "--room-share", "0.5", "--duration", str(seconds)],
            cwd=SERVER_DIR, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        print("%d bots for %d s (room '%s')…" % (a.bots, seconds, a.room), flush=True)
        warm = None
        start = time.time()
        while bots.poll() is None and time.time() - start < seconds + 30:
            time.sleep(5)
            if warm is None and time.time() - start > 20:
                warm = rss_kb(server.pid)
            if server.poll() is not None:
                break
        end_rss = rss_kb(server.pid)
        crashed = server.poll() is not None
        if bots.poll() is None:
            bots.kill()
        if not crashed:
            server.send_signal(signal.SIGINT)
            server.wait(15)

    text = open(log_path, errors="replace").read()
    stats = re.findall(r"missed (\d+)\) \| tick avg (\d+) us max (\d+) us \| players (\d+)", text)
    missed = sum(int(s[0]) for s in stats)
    worst = max((int(s[2]) for s in stats), default=0)
    avg = max((int(s[1]) for s in stats), default=0)
    players = max((int(s[3]) for s in stats), default=0)
    panics = [ln for ln in text.splitlines() if "panicked" in ln]
    grew = (end_rss / warm) if warm else 0

    print("players at once: %d / %d" % (players, a.bots))
    print("missed ticks: %d | tick avg (worst 5 s) %d us | slowest tick %d us (budget %d)" % (missed, avg, worst, TICK_BUDGET_US))
    print("memory: %d KB after warm-up -> %d KB at the end (x%.2f)" % (warm or 0, end_rss, grew))
    problems = []
    if crashed or panics:
        problems.append("the server crashed: " + (panics[0] if panics else "exited"))
    if players < a.bots * 0.9:
        problems.append("only %d of %d bots got in" % (players, a.bots))
    if missed:
        problems.append("%d missed ticks" % missed)
    if worst > SLOW_TICK_US:
        problems.append("a tick took %d us" % worst)
    if warm and grew > MEMORY_GROWTH:
        problems.append("memory grew x%.2f" % grew)
    print("server log:", log_path)
    if problems:
        print("FAIL: " + "; ".join(problems))
        return 1
    print("PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
