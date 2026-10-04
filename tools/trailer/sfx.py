"""The trailer's sound effects: the game's own sounds (client/sounds) laid
out on the trailer's timeline, to go under the music.

    python3 sfx.py out.wav seconds

Times match the clips in build.sh (title 0-4.5, rain 4.5-9.5, hall
9.5-14.5, office 14.5-19.5, chill room 19.5-24.5, shop 24.5-29.5, end
29.5-33.5).
"""
import array
import os
import random
import sys
import wave

SOUNDS = os.path.join(os.path.dirname(__file__), "..", "..", "client", "sounds")
SR = 22050
rng = random.Random(3)
_cache = {}


def load(name):
    if name not in _cache:
        with wave.open(os.path.join(SOUNDS, name + ".wav")) as w:
            assert w.getframerate() == SR and w.getnchannels() == 1
            data = array.array("h", w.readframes(w.getnframes()))
        _cache[name] = [v / 32768 for v in data]
    return _cache[name]


def db(x):
    return 10 ** (x / 20)


out_path, seconds = sys.argv[1], float(sys.argv[2])
buf = [0.0] * int(seconds * SR)


def put(name, t, vol_db=0.0, pitch=1.0):
    x = load(name)
    g = db(vol_db)
    s0 = int(t * SR)
    n = int(len(x) / pitch)
    for i in range(n):
        j = s0 + i
        if 0 <= j < len(buf):
            buf[j] += x[min(len(x) - 1, int(i * pitch))] * g


def bed(name, t0, t1, vol_db, fade=0.35):
    """A loop from t0 to t1 with short fades (clip cuts)."""
    x = load(name)
    g = db(vol_db)
    a, b = int(t0 * SR), int(t1 * SR)
    f = int(fade * SR)
    for j in range(a, min(b, len(buf))):
        k = j - a
        e = min(1.0, k / f, (b - j) / f)
        buf[j] += x[k % len(x)] * g * e


def steps(surface, t0, t1, vol_db, every=0.24):
    t = t0
    n = 0
    while t < t1:
        put(f"step_{surface}_{n % 3 + 1}", t, vol_db + rng.uniform(-1.5, 1.0), rng.uniform(0.93, 1.07))
        t += every + rng.uniform(-0.015, 0.015)
        n += 1


# 1. Title: the menu, "Graj" clicked at the end.
put("ui_click", 3.9, -4)
# 2. Rainy morning outside: rain, the street, a car, footsteps, thunder.
bed("rain_loop", 4.5, 9.5, -4)
bed("street_loop", 4.5, 9.5, -8)
steps("out", 4.7, 9.3, -6)
put("thunder", 6.2, -3)
# 3. The hall: tiles underfoot, the glass door, Pani Wiesia's hello.
bed("office_loop", 9.5, 14.5, -12)
steps("tiles", 9.6, 11.8, -4)
put("blip", 12.0, -9, 0.85)
put("blip", 12.2, -9, 0.9)
put("blip", 12.45, -9, 0.85)
# 4. The office: the hum, carpet steps, keyboards, somebody talking.
bed("office_loop", 14.5, 19.5, -3)
steps("carpet", 14.6, 16.6, 0)
put("typing", 15.4, -10)
put("typing", 17.9, -12)
put("blip", 16.3, -12, 0.95)
put("blip", 16.45, -12, 1.1)
# 5. The chill room: the boombox's disco polo (the match on the TV).
bed("boombox_1", 19.5, 24.5, -6)
put("pickup", 19.7, -6)
# 6. The shop: steps, a bottle off the shelf, the till.
bed("street_loop", 24.5, 29.5, -16)
steps("floor", 24.6, 26.4, -5)
put("pickup", 26.5, -4)
steps("floor", 26.8, 27.6, -6)
put("blip", 28.0, -9, 1.15)
put("blip", 28.2, -9, 1.2)
# 7. End card: the elevator's ding.
put("ding", 29.75, -4)

peak = max(abs(v) for v in buf) or 1.0
scale = min(1.0, 0.9 / peak)
data = array.array("h", (int(max(-1.0, min(1.0, v * scale)) * 32767) for v in buf))
with wave.open(out_path, "wb") as w:
    w.setnchannels(1)
    w.setsampwidth(2)
    w.setframerate(SR)
    w.writeframes(data.tobytes())
print("wrote", out_path)
