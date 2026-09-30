"""The trailer's sound effects: the game's own sounds (client/sounds) laid
out on the trailer's timeline, to go under the music.

    python3 sfx.py out.wav seconds

Times match the clips in build.sh (title 0-4.5, rain 4.5-9.5, office
9.5-14.5, coffee 14.5-19.5, smoke 19.5-24.5, commute 24.5-28, end 28-32).
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
# 3. The office: the hum, carpet steps, keyboards, somebody talking.
bed("office_loop", 9.5, 14.5, -3)
steps("carpet", 9.6, 14.3, 0)
put("typing", 10.4, -10)
put("typing", 12.9, -12)
put("blip", 11.3, -12, 0.95)
put("blip", 11.45, -12, 1.1)
# 4. Coffee: corridor steps, the mug from the cupboard, the machine.
bed("office_loop", 14.5, 19.5, -9)
steps("floor", 14.6, 16.6, -5)
put("cupboard", 16.75, -2)
put("blip", 16.8, -10, 1.0)
steps("floor", 17.1, 17.6, -6)
put("coffee", 18.2, -1)
put("blip", 18.25, -10, 1.0)
# 5. Smoke in the bathroom: muffled office, a cough of the smoke detector.
bed("office_loop", 19.5, 24.5, -14)
put("lighter", 19.7, -6)
for k in range(3):
    put("detector_beep", 22.9 + k * 0.45, -8)
# 6. Tomorrow: a paper swish, a click on "Tramwaj".
put("ui_open", 24.7, -6)
put("ui_click", 26.6, -4)
# 7. End card: the elevator's ding.
put("ding", 28.25, -4)

peak = max(abs(v) for v in buf) or 1.0
scale = min(1.0, 0.9 / peak)
data = array.array("h", (int(max(-1.0, min(1.0, v * scale)) * 32767) for v in buf))
with wave.open(out_path, "wb") as w:
    w.setnchannels(1)
    w.setsampwidth(2)
    w.setframerate(SR)
    w.writeframes(data.tobytes())
print("wrote", out_path)
