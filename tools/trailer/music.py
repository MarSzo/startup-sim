"""A small lo-fi loop for the trailer, synthesized from scratch (no samples):
soft electric-piano chords, a plucked melody, bass, a dusty kick/snare/hat.
Usage: python3 music.py out.wav seconds"""
import array, math, random, sys, wave

SR = 44100
BPM = 88
BEAT = 60 / BPM
out_path, seconds = sys.argv[1], float(sys.argv[2])
n = int(SR * seconds)
buf = [0.0] * n
rng = random.Random(7)


def midi(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def add(start, dur, fn, gain):
    s0 = int(start * SR)
    for i in range(int(dur * SR)):
        j = s0 + i
        if j >= n:
            break
        buf[j] += fn(i / SR) * gain


def epiano(f, dur):
    def fn(t):
        env = math.exp(-t * 1.6) * min(1.0, t * 80) * min(1.0, (dur - t) * 8)
        return env * (math.sin(2 * math.pi * f * t) + 0.25 * math.sin(4 * math.pi * f * t) * math.exp(-t * 4)
                      + 0.08 * math.sin(2 * math.pi * f * 3.01 * t) * math.exp(-t * 6))
    return fn


def pluck(f):
    def fn(t):
        env = math.exp(-t * 5) * min(1.0, t * 200)
        tri = 2 / math.pi * math.asin(math.sin(2 * math.pi * f * t))
        return env * (0.7 * tri + 0.3 * math.sin(2 * math.pi * 2 * f * t))
    return fn


def bass(f, dur):
    def fn(t):
        env = min(1.0, t * 60) * min(1.0, (dur - t) * 10) * (0.6 + 0.4 * math.exp(-t * 3))
        return env * (math.sin(2 * math.pi * f * t) + 0.2 * math.sin(4 * math.pi * f * t))
    return fn


def kick(t):
    f = 50 + 70 * math.exp(-t * 30)
    return math.sin(2 * math.pi * f * t) * math.exp(-t * 9)


def snare(t):
    return (rng.uniform(-1, 1) * 0.8 + 0.3 * math.sin(2 * math.pi * 190 * t)) * math.exp(-t * 16)


def hat(t):
    return rng.uniform(-1, 1) * math.exp(-t * 60)


# Cmaj9 - Am9 - Fmaj7 - G7sus -> back
chords = [[48, 55, 59, 62, 64], [45, 52, 55, 59, 60], [41, 48, 52, 57, 60], [43, 50, 53, 57, 62]]
roots = [36, 33, 29, 31]
melody = [  # (beat within 4 bars, midi, beats)
    (0, 76, 1), (1.5, 74, 0.5), (2, 72, 1), (3, 67, 1),
    (4, 72, 1), (5, 74, 0.5), (5.5, 76, 0.5), (6, 74, 2),
    (8, 69, 1), (9, 72, 1), (10, 74, 1), (11, 72, 0.5), (11.5, 69, 0.5),
    (12, 71, 1.5), (13.5, 72, 0.5), (14, 74, 2),
]
bar = 4 * BEAT
bars = int(seconds / bar) + 1
for b in range(bars):
    t0 = b * bar
    k = b % 4
    for m in chords[k]:
        add(t0, bar, epiano(midi(m), bar), 0.045)
    add(t0, bar * 0.9, bass(midi(roots[k]), bar * 0.9), 0.16)
    for beat in range(4):
        tb = t0 + beat * BEAT
        if beat in (0, 2) and b > 0:
            add(tb, 0.5, kick, 0.35)
        if beat in (1, 3) and b > 0:
            add(tb, 0.3, snare, 0.08)
        for h in (0, 0.5):
            swing = 0.06 * BEAT if h else 0
            add(tb + h * BEAT + swing, 0.08, hat, 0.03 if b > 0 else 0.0)
    if b >= 1:
        for (mb, m, ln) in melody:
            if int(mb // 4) == (b - 1) % 4:
                add(t0 + (mb % 4) * BEAT, ln * BEAT + 0.3, pluck(midi(m)), 0.09)

# vinyl crackle, a gentle low-pass, fades
prev = 0.0
for i in range(n):
    if rng.random() < 0.0004:
        buf[i] += rng.uniform(-0.05, 0.05)
    prev = prev + 0.35 * (buf[i] - prev)
    buf[i] = prev
fade_in, fade_out = int(0.8 * SR), int(2.5 * SR)
peak = max(abs(x) for x in buf) or 1.0
data = array.array("h")
for i, x in enumerate(buf):
    g = min(1.0, i / fade_in, (n - i) / fade_out)
    data.append(int(max(-1.0, min(1.0, x / peak * 0.8 * g)) * 32767))
with wave.open(out_path, "wb") as w:
    w.setnchannels(1)
    w.setsampwidth(2)
    w.setframerate(SR)
    w.writeframes(data.tobytes())
print("wrote", out_path)
