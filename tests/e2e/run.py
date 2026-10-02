#!/usr/bin/env python3
"""End-to-end gameplay tests: the real server and the real Godot client
(headless), driven by scenario scripts (client/tests/e2e/scenarios/*.gd).

    python3 tests/e2e/run.py                 # every scenario
    python3 tests/e2e/run.py workday founder # some of them
    python3 tests/e2e/run.py --list

Each scenario gets its own server (own port, temporary save directory) and
one or more clients; a phase may restart the server in between. A client
passes when it exits with 0 and its log has no "SCRIPT ERROR". Logs of a
failed scenario are printed; all of them stay in --logs (default: a temp dir).

Needs: `godot` (4.7) in PATH (or GODOT=...), cargo (builds the server).
"""
import argparse
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import time

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
SERVER_DIR = os.path.join(ROOT, "server")
CLIENT_DIR = os.path.join(ROOT, "client")
CLIENT_TIMEOUT = 300  # seconds per client process (scenarios stop themselves at 240)

EMPLOYED = ["--start-employed", "--allow-guests", "--no-save"]

# name -> phases; a phase: server args ("same" = keep / restart the save) and
# clients (run in parallel): [scenario script, client args].
SCENARIOS = {
    "workday": [
        {"server": EMPLOYED + ["--start-time", "10:00", "--time-scale", "20"],
         "clients": [["workday", ["--nick=Ola", "--autoconnect"]]]},
    ],
    "onboarding": [
        {"server": ["--save", "{tmp}/world.db"],
         "clients": [["onboarding", ["--login=Nowa:haslo-nowej-1", "--register", "--autocreate", "--auto-recruit=1"]]]},
    ],
    "resign": [
        {"server": ["--save", "{tmp}/world.db"],
         "clients": [["resign", ["--login=Wybredna:haslo-wybrednej-1", "--register", "--autocreate", "--auto-recruit=1"]]]},
    ],
    "drinking": [
        {"server": EMPLOYED + ["--start-time", "10:00"],
         "clients": [["drinking", ["--nick=Kuba", "--autoconnect"]]]},
    ],
    "fight": [
        {"server": EMPLOYED + ["--start-time", "10:00"],
         "clients": [["fight_ola", ["--nick=Ola", "--autoconnect"]],
                     ["fight_kuba", ["--nick=Kuba", "--autoconnect"], 1.5]]},
    ],
    "office_apps": [
        {"server": EMPLOYED + ["--start-time", "10:00"],
         "clients": [["office_apps", ["--nick=Ola", "--autoconnect"]]]},
    ],
    "together": [
        {"server": EMPLOYED + ["--start-time", "10:00"],
         "clients": [["together_ola", ["--nick=Ola", "--autoconnect"]],
                     ["together_kuba", ["--nick=Kuba", "--autoconnect"], 1.5]]},
    ],
    "founder": [
        {"server": ["--allow-guests", "--no-save"],
         "clients": [["founder", ["--nick=Szef", "--autoconnect", "--found=Kosmiczne Pierogi"]],
                     ["founder_hire", ["--nick=Mobilny", "--autoconnect"], 1.5]]},
    ],
    "persistence": [
        {"server": ["--save", "{tmp}/world.db", "--start-employed"],
         "clients": [["persist_day1", ["--login=Trwala:haslo-trwalej-1", "--register", "--autocreate"]]]},
        {"server": "restart",
         "clients": [["persist_day2", ["--login=Trwala:haslo-trwalej-1"]]]},
    ],
}


class Server:
    def __init__(self, binary, port, args, log):
        self.cmd = [binary, "--bind", "127.0.0.1:%d" % port, "--stats-secs", "3600"] + args
        self.log = log
        self.proc = None

    def start(self):
        self.out = open(self.log, "a")
        self.proc = subprocess.Popen(self.cmd, cwd=SERVER_DIR, stdout=self.out, stderr=subprocess.STDOUT)
        time.sleep(1.2)
        if self.proc.poll() is not None:
            raise RuntimeError("server exited at start, see " + self.log)

    def stop(self):
        if self.proc and self.proc.poll() is None:
            self.proc.send_signal(signal.SIGINT)  # saves the game
            try:
                self.proc.wait(15)
            except subprocess.TimeoutExpired:
                self.proc.kill()
        self.out.close()


def run_client(godot, port, script, args, sid, log):
    cmd = [godot, "--headless", "--path", CLIENT_DIR, "--", "--server=127.0.0.1:%d" % port,
           "--scenario=" + script, "--scenario-id=" + sid] + args
    with open(log, "w") as out:
        p = subprocess.Popen(cmd, stdout=out, stderr=subprocess.STDOUT)
    return p


def scenario(name, phases, godot, binary, port, logs):
    tmp = tempfile.mkdtemp(prefix="e2e-%s-" % name)
    server = None
    ok = True
    results = []
    try:
        for i, phase in enumerate(phases):
            if phase["server"] == "restart":
                server.stop()
                server.start()
            else:
                if server:
                    server.stop()
                args = [a.replace("{tmp}", tmp) for a in phase["server"]]
                server = Server(binary, port, args, os.path.join(logs, "%s-server.log" % name))
                server.start()
            procs = []
            for c in phase["clients"]:
                script, args = c[0], c[1]
                if len(c) > 2:
                    time.sleep(c[2])
                sid = "%s-%s" % (name, script)
                log = os.path.join(logs, "%s-%d-%s.log" % (name, i, script))
                procs.append((script, log, run_client(godot, port, script, args, sid, log), time.time()))
            for script, log, p, started in procs:
                try:
                    code = p.wait(max(1, CLIENT_TIMEOUT - (time.time() - started)))
                except subprocess.TimeoutExpired:
                    p.kill()
                    code = "timeout"
                text = open(log, errors="replace").read()
                errors = [ln for ln in text.splitlines() if "SCRIPT ERROR" in ln]
                passed = code == 0 and not errors and ("E2E PASS" in text)
                results.append((script, passed, code, log, errors))
                ok &= passed
            if not ok:
                break
    finally:
        if server:
            server.stop()
        shutil.rmtree(tmp, ignore_errors=True)
    return ok, results


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("names", nargs="*")
    ap.add_argument("--list", action="store_true")
    ap.add_argument("--logs", default=None)
    ap.add_argument("--port", type=int, default=7900)
    a = ap.parse_args()
    if a.list:
        print("\n".join(SCENARIOS))
        return 0
    names = a.names or list(SCENARIOS)
    unknown = [n for n in names if n not in SCENARIOS]
    if unknown:
        sys.exit("unknown scenario(s): %s" % ", ".join(unknown))
    godot = os.environ.get("GODOT", "godot")
    logs = a.logs or tempfile.mkdtemp(prefix="e2e-logs-")
    os.makedirs(logs, exist_ok=True)
    print("building the server…", flush=True)
    subprocess.run(["cargo", "build", "--release", "--bin", "server", "-q"], cwd=SERVER_DIR, check=True)
    binary = os.path.join(SERVER_DIR, "target", "release", "server")
    subprocess.run([godot, "--headless", "--path", CLIENT_DIR, "--import"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    failed = []
    for i, n in enumerate(names):
        t0 = time.time()
        ok, results = scenario(n, SCENARIOS[n], godot, binary, a.port + i * 2, logs)
        print("%s %s (%.0f s)" % ("PASS" if ok else "FAIL", n, time.time() - t0), flush=True)
        for script, passed, code, log, errors in results:
            if not passed:
                failed.append(n)
                print("  %s: exit %s%s" % (script, code, ", " + errors[0] if errors else ""))
                lines = open(log, errors="replace").read().splitlines()
                interesting = [ln for ln in lines if "E2E" in ln or "ERROR" in ln]
                for ln in (interesting or lines)[-15:]:
                    print("    " + ln)
    print("logs:", logs)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
