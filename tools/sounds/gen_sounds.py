"""Synthesizes every sound of the game (no samples, no licences):
footsteps, devices, alarms, ambience loops and two music loops.

    python3 tools/sounds/gen_sounds.py        # -> client/sounds/*.wav

Mono 16-bit WAV at 22 050 Hz. Loops (names ending in _loop / music_*) are
seamless: the tail is folded back onto the start.
"""
import array
import math
import os
import random
import wave

SR = 22050
OUT = os.path.join(os.path.dirname(__file__), "..", "..", "client", "sounds")
rng = random.Random(1234)
TAU = 2 * math.pi


# ------------------------------------------------------------------ basics

def silence(sec):
    return [0.0] * int(sec * SR)


def noise(sec):
    return [rng.uniform(-1, 1) for _ in range(int(sec * SR))]


def tone(freq, sec, shape="sine", phase=0.0):
    out = []
    ph = phase
    f = freq if callable(freq) else (lambda t, fr=freq: fr)
    for i in range(int(sec * SR)):
        t = i / SR
        ph += TAU * f(t) / SR
        if shape == "sine":
            out.append(math.sin(ph))
        elif shape == "square":
            out.append(1.0 if math.sin(ph) >= 0 else -1.0)
        elif shape == "tri":
            out.append(2 / math.pi * math.asin(math.sin(ph)))
        else:  # saw
            out.append(((ph / TAU) % 1.0) * 2 - 1)
    return out


def env(x, a=0.005, d=None, r=0.02, hold=None):
    """Attack, optional exponential decay rate d (1/s), release at the end."""
    n = len(x)
    out = []
    for i, v in enumerate(x):
        t = i / SR
        g = min(1.0, t / a) if a > 0 else 1.0
        if d:
            g *= math.exp(-t * d)
        rem = (n - i) / SR
        if r > 0:
            g *= min(1.0, rem / r)
        out.append(v * g)
    return out


def lowpass(x, cutoff):
    a = 1 - math.exp(-TAU * cutoff / SR)
    out, y = [], 0.0
    for v in x:
        y += a * (v - y)
        out.append(y)
    return out


def highpass(x, cutoff):
    lp = lowpass(x, cutoff)
    return [v - low for v, low in zip(x, lp)]


def bandpass(x, lo, hi):
    return lowpass(highpass(x, lo), hi)


def sweep_lowpass(x, c0, c1):
    out, y = [], 0.0
    n = len(x)
    for i, v in enumerate(x):
        c = c0 + (c1 - c0) * i / max(1, n - 1)
        a = 1 - math.exp(-TAU * c / SR)
        y += a * (v - y)
        out.append(y)
    return out


def gain(x, g):
    return [v * g for v in x]


def mix(*parts):
    n = max(len(p) for p in parts)
    out = [0.0] * n
    for p in parts:
        for i, v in enumerate(p):
            out[i] += v
    return out


def at(x, sec, total=None):
    """x delayed by sec (padded to total seconds)."""
    pre = [0.0] * int(sec * SR)
    y = pre + x
    if total:
        y = (y + [0.0] * int(total * SR))[: int(total * SR)]
    return y


def cat(*parts):
    out = []
    for p in parts:
        out += p
    return out


def bell(freq, sec, decay=4.0, partials=((1, 1.0), (2.76, 0.4), (5.4, 0.2), (8.9, 0.08))):
    out = [0.0] * int(sec * SR)
    for mul, amp in partials:
        t = tone(freq * mul, sec)
        for i in range(len(out)):
            out[i] += t[i] * amp * math.exp(-i / SR * decay * (1 + mul * 0.3))
    return env(out, a=0.002, r=0.05)


def reverb(x, amount=0.25, delays=(0.029, 0.037, 0.041, 0.053), fb=0.5):
    """A cheap room: a few feedback combs."""
    out = list(x) + [0.0] * int(0.4 * SR)
    wet = [0.0] * len(out)
    for d in delays:
        k = int(d * SR)
        buf = [0.0] * len(out)
        for i in range(len(out)):
            v = (out[i] if i < len(x) else 0.0) + (buf[i - k] * fb if i >= k else 0.0)
            buf[i] = v
            wet[i] += v
    return [o + w * amount / len(delays) for o, w in zip(out, wet)]


def fold_loop(x, sec):
    """Seamless loop of `sec`: whatever runs past the end is added to the start."""
    n = int(sec * SR)
    out = x[:n] + [0.0] * max(0, n - len(x))
    for i in range(n, len(x)):
        out[i % n] += x[i]
    return out


def crossfade_loop(x, sec, fade=0.5):
    """Seamless loop for noise beds: the extra tail fades into the head."""
    n, f = int(sec * SR), int(fade * SR)
    x = x[: n + f] + [0.0] * max(0, n + f - len(x))
    out = x[:n]
    for i in range(f):
        g = i / f
        out[i] = x[i] * g + x[n + i] * (1 - g)
    return out


def write(name, x, peak=0.85):
    m = max((abs(v) for v in x), default=1.0) or 1.0
    data = array.array("h", (int(max(-1.0, min(1.0, v / m * peak)) * 32767) for v in x))
    with wave.open(os.path.join(OUT, name + ".wav"), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(data.tobytes())
    print(f"{name:22s} {len(x) / SR:5.2f} s")


# ------------------------------------------------------------------ sounds

def steps():
    for k in range(3):
        # Wood-ish floor: a dull knock.
        body = env(tone(85 + k * 9, 0.12), a=0.002, d=35)
        tap = env(lowpass(noise(0.08), 1400 + k * 200), a=0.001, d=60)
        write(f"step_floor_{k + 1}", mix(gain(body, 0.7), tap), 0.6)
        # Carpet: soft, muffled.
        write(f"step_carpet_{k + 1}", env(lowpass(noise(0.12), 500 + k * 80), a=0.01, d=28), 0.35)
        # Tiles: a hard heel click.
        click = env(highpass(noise(0.05), 2500), a=0.0005, d=120)
        ring = env(tone(1700 + k * 130, 0.06), a=0.0005, d=90)
        write(f"step_tiles_{k + 1}", mix(click, gain(ring, 0.25), gain(body, 0.4)), 0.6)
        # Outside: grit and gravel.
        grit = []
        for g in range(7):
            grit = mix(grit, at(env(bandpass(noise(0.02), 1800, 5000), a=0.0005, d=150), g * 0.012 + rng.uniform(0, 0.01)))
        write(f"step_out_{k + 1}", mix(gain(grit, 0.8), gain(env(lowpass(noise(0.1), 600), a=0.003, d=40), 0.5)), 0.5)


def ui():
    write("ui_click", mix(env(tone(2100, 0.03), a=0.0005, d=160), gain(env(highpass(noise(0.02), 3000), a=0.0005, d=250), 0.5)), 0.5)
    swish = sweep_lowpass(noise(0.22), 800, 5000)
    write("ui_open", env(swish, a=0.03, d=10), 0.4)
    write("blip", env(mix(tone(420, 0.07, "tri"), gain(tone(1260, 0.07), 0.2)), a=0.004, d=25, r=0.02), 0.5)
    write("notify", reverb(cat(env(tone(1046.5, 0.12), a=0.002, d=18), env(tone(1318.5, 0.3), a=0.002, d=9)), 0.3), 0.6)


def coffee():
    # Grinder (0.9 s): buzzing band noise, then the pour with bubbles (1.9 s).
    g = bandpass(noise(0.9), 600, 3500)
    buzz = [v * (0.6 + 0.4 * math.sin(TAU * 38 * i / SR)) for i, v in enumerate(g)]
    grind = env(buzz, a=0.03, r=0.08)
    pour = lowpass(noise(1.9), 900)
    for b in range(22):
        t0 = rng.uniform(0.1, 1.8)
        f = rng.uniform(300, 700)
        bub = env(tone(lambda t, f=f: f + 900 * t, 0.05), a=0.002, d=60)
        pour = mix(pour, at(gain(bub, 0.35), t0))
    pour = env(pour, a=0.15, r=0.3)
    hiss = env(highpass(noise(0.5), 4000), a=0.05, r=0.2)
    write("coffee", cat(gain(grind, 0.8), silence(0.1), mix(pour, at(gain(hiss, 0.15), 1.3))), 0.7)


def till():
    hit = env(bandpass(noise(0.06), 1000, 6000), a=0.0005, d=70)
    bells = mix(bell(2093, 0.9, 5), at(bell(2637, 0.8, 5), 0.07), at(bell(3136, 0.7, 5), 0.12))
    write("till", reverb(mix(hit, gain(bells, 0.6)), 0.2), 0.7)
    coin = mix(bell(3520, 0.35, 12), at(bell(4186, 0.3, 14), 0.05))
    write("coin", coin, 0.5)


def alarms():
    beep = env(tone(2800, 0.15, "square"), a=0.002, r=0.01)
    write("gate_alarm", lowpass(cat(beep, silence(0.1), beep, silence(0.1), beep, silence(0.1)), 6000), 0.55)
    write("detector_beep", lowpass(env(tone(3200, 0.09, "square"), a=0.001, r=0.01), 6000), 0.5)
    # Fire bell: a hammer at 22 Hz on a 1 kHz bell, one second loop.
    b = []
    for i in range(22):
        b = mix(b, at(bell(980, 0.12, 25, ((1, 1), (2.7, 0.5), (5.1, 0.3))), i / 22))
    write("fire_bell_loop", fold_loop(b, 1.0), 0.55)
    # Police wail (up and down, 2 s) and the fire engine hi-lo (Polish "na-no").
    wail = tone(lambda t: 750 + 550 * (0.5 - 0.5 * math.cos(TAU * t / 2.0)), 2.0, "saw")
    write("siren_police_loop", lowpass(wail, 2500), 0.45)
    hilo = tone(lambda t: 950 if (t % 1.2) < 0.6 else 700, 2.4, "saw")
    write("siren_fire_loop", lowpass(hilo, 2200), 0.45)
    # Guard's whistle: a trill.
    wh = tone(lambda t: 2900 + 180 * math.sin(TAU * 26 * t), 0.8)
    wh = mix(wh, gain(bandpass(noise(0.8), 2500, 4000), 0.25))
    write("whistle", env(wh, a=0.02, r=0.12), 0.55)


def devices():
    write("ding", reverb(mix(bell(880, 1.4, 2.5), at(bell(698.5, 1.4, 2.5), 0.35)), 0.3), 0.6)
    clk = env(highpass(noise(0.02), 1800), a=0.0005, d=200)
    clunk = env(mix(lowpass(noise(0.08), 900), gain(tone(140, 0.08), 0.6)), a=0.001, d=50)
    write("lock", cat(clk, silence(0.04), clunk), 0.6)
    write("switch", mix(clk, gain(env(tone(3000, 0.015), a=0.0005, d=300), 0.4)), 0.55)
    # Flush: a rising whoosh, gurgles, the tank refilling.
    wh = sweep_lowpass(noise(2.2), 300, 1800)
    wh = [v * math.sin(math.pi * min(1, i / (2.2 * SR))) for i, v in enumerate(wh)]
    gur = []
    for b in range(14):
        f = rng.uniform(150, 350)
        gur = mix(gur, at(env(tone(lambda t, f=f: f + 400 * t, 0.07), a=0.003, d=40), rng.uniform(0.6, 1.9)))
    write("flush", mix(wh, gain(gur, 0.3)), 0.7)
    tap = lowpass(highpass(noise(1.6), 400), 3000)
    for b in range(30):
        f = rng.uniform(900, 2000)
        tap = mix(tap, at(gain(env(tone(lambda t, f=f: f + 3000 * t, 0.03), a=0.001, d=90), 0.25), rng.uniform(0, 1.5)))
    write("tap", env(tap, a=0.08, r=0.25), 0.5)
    flick = env(highpass(noise(0.03), 2500), a=0.0005, d=150)
    spark = env(bandpass(noise(0.12), 3000, 8000), a=0.001, d=40)
    flame = env(lowpass(noise(0.6), 700), a=0.05, r=0.3)
    write("lighter", mix(flick, at(gain(spark, 0.6), 0.02), at(gain(flame, 0.5), 0.1)), 0.6)
    beeps = cat(env(tone(1600, 0.1), r=0.01), silence(0.05), env(tone(2000, 0.12), r=0.01))
    whirr = env(bandpass(noise(1.4), 200, 900), a=0.3, r=0.3)
    whirr = [v * (0.7 + 0.3 * math.sin(TAU * 6 * i / SR)) for i, v in enumerate(whirr)]
    write("dishwasher", mix(gain(beeps, 0.5), at(whirr, 0.35)), 0.6)
    thunk = env(mix(lowpass(noise(0.1), 500), gain(tone(95, 0.12), 0.8)), a=0.002, d=30)
    suck = env(bandpass(noise(0.2), 800, 3000), a=0.02, d=15)
    hum = env(mix(tone(100, 0.9), gain(tone(200, 0.9), 0.4)), a=0.2, r=0.3)
    write("fridge", mix(thunk, at(gain(suck, 0.4), 0.03), at(gain(hum, 0.25), 0.15)), 0.6)
    door = env(mix(lowpass(noise(0.07), 1200), gain(tone(180, 0.07), 0.5)), a=0.002, d=40)
    clink = mix(bell(3300, 0.4, 10), at(bell(4100, 0.35, 12), 0.06), at(bell(2800, 0.3, 12), 0.13))
    write("cupboard", mix(door, at(gain(clink, 0.5), 0.1)), 0.6)
    write("pickup", env(tone(lambda t: 500 + 900 * t / 0.09, 0.09, "tri"), a=0.003, d=20), 0.45)
    write("drop", env(mix(lowpass(noise(0.12), 700), gain(tone(110, 0.12), 0.8)), a=0.001, d=30), 0.55)
    crunch = []
    for b in range(9):
        crunch = mix(crunch, at(env(bandpass(noise(0.04), 1000, 5000), a=0.001, d=80), b * 0.065 + rng.uniform(0, 0.02)))
    write("eat", crunch, 0.5)
    gulp = []
    for b in range(3):
        gulp = mix(gulp, at(env(lowpass(tone(lambda t: 260 - 500 * t, 0.12, "tri"), 900), a=0.01, d=18), b * 0.22))
    write("drink", gulp, 0.5)
    typing = []
    for b in range(10):
        typing = mix(typing, at(env(bandpass(noise(0.02), 1500, 6000), a=0.0005, d=200), b * 0.075 + rng.uniform(0, 0.03)))
    write("typing", typing, 0.4)
    write("card_reader", env(tone(1760, 0.14), a=0.002, r=0.02), 0.4)


def weather():
    # Thunder: a crack and a long rolling rumble.
    crack = env(highpass(noise(0.25), 1200), a=0.001, d=12)
    rum = lowpass(lowpass(noise(4.0), 180), 120)
    rum = [v * (0.5 + 0.5 * math.sin(TAU * 1.3 * i / SR + 1) * math.sin(TAU * 0.4 * i / SR)) for i, v in enumerate(rum)]
    write("thunder", mix(gain(crack, 0.5), env(rum, a=0.05, d=0.9, r=0.8)), 0.9)
    # Rain bed: steady hiss + droplets; 8 s seamless.
    rain = bandpass(noise(8.6), 700, 6000)
    for b in range(900):
        f = rng.uniform(1500, 5000)
        rain = mix(rain, at(gain(env(tone(f, 0.02), a=0.0005, d=250), rng.uniform(0.05, 0.3)), rng.uniform(0, 8.5)))
    write("rain_loop", crossfade_loop(rain, 8.0, 0.6), 0.5)
    # Street: low traffic rumble, a few cars passing; 12 s.
    street = lowpass(noise(12.8), 250)
    for c in range(4):
        t0 = rng.uniform(0, 10.5)
        car = lowpass(noise(2.4), 600)
        car = [v * math.sin(math.pi * i / len(car)) ** 2 for i, v in enumerate(car)]
        street = mix(street, at(gain(car, 1.6), t0))
    write("street_loop", crossfade_loop(street, 12.0, 0.8), 0.45)
    # Office: an air-con hum, far-off keyboards and a chair now and then; 12 s.
    office = mix(gain(lowpass(noise(12.8), 300), 0.5), gain(tone(120, 12.8), 0.04), gain(tone(240, 12.8), 0.02))
    for b in range(70):
        office = mix(office, at(gain(env(bandpass(noise(0.02), 1500, 5000), a=0.0005, d=200), 0.25), rng.uniform(0, 12.5)))
    write("office_loop", crossfade_loop(office, 12.0, 0.8), 0.35)


def music(name, bpm, chords, roots, melody, drums, bars_n, pad_gain):
    beat = 60 / bpm
    bar = 4 * beat
    total = bars_n * bar
    buf = [0.0] * int((total + 3) * SR)

    def add(t0, x, g):
        s = int(t0 * SR)
        for i, v in enumerate(x):
            if s + i < len(buf):
                buf[s + i] += v * g

    def midi(m):
        return 440.0 * 2 ** ((m - 69) / 12)

    def ep(f, dur):
        x = tone(f, dur)
        x2 = tone(2 * f, dur)
        return env([a + 0.25 * b * math.exp(-i / SR * 4) for i, (a, b) in enumerate(zip(x, x2))], a=0.01, d=1.4, r=0.1)

    for b in range(bars_n):
        t0 = b * bar
        k = b % len(chords)
        for m in chords[k]:
            add(t0, ep(midi(m), bar), pad_gain)
        add(t0, env(tone(midi(roots[k]), bar * 0.9), a=0.02, r=0.1), 0.16)
        if drums:
            for bt in range(4):
                tb = t0 + bt * beat
                if bt in (0, 2):
                    add(tb, env(tone(lambda t: 50 + 70 * math.exp(-t * 30), 0.4), a=0.001, d=9), 0.33)
                else:
                    add(tb, env(mix(gain(noise(0.25), 0.8), gain(tone(190, 0.25), 0.3)), a=0.001, d=16), 0.07)
                for h in (0, 0.5):
                    add(tb + h * beat + (0.06 * beat if h else 0), env(noise(0.06), a=0.0005, d=60), 0.025)
    for (mb, m, ln) in melody:
        for rep in range(bars_n // 4):
            t = (rep * 4) * bar + mb * beat
            add(t, env(tone(midi(m), ln * beat + 0.3, "tri"), a=0.004, d=4.5, r=0.05), 0.08)
    buf = lowpass(buf, 5000)
    for i in range(len(buf)):
        if rng.random() < 0.0003:
            buf[i] += rng.uniform(-0.04, 0.04)
    write(name, fold_loop(buf, total), 0.7)


def main():
    os.makedirs(OUT, exist_ok=True)
    steps()
    ui()
    coffee()
    till()
    alarms()
    devices()
    weather()
    lofi_chords = [[48, 55, 59, 62, 64], [45, 52, 55, 59, 60], [41, 48, 52, 57, 60], [43, 50, 53, 57, 62]]
    melody = [(0, 76, 1), (1.5, 74, 0.5), (2, 72, 1), (3, 67, 1), (4, 72, 1), (5, 74, 0.5), (5.5, 76, 0.5), (6, 74, 2),
              (8, 69, 1), (9, 72, 1), (10, 74, 1), (11, 72, 0.5), (11.5, 69, 0.5), (12, 71, 1.5), (13.5, 72, 0.5), (14, 74, 2)]
    music("music_menu", 86, lofi_chords, [36, 33, 29, 31], melody, True, 16, 0.045)
    night_chords = [[45, 52, 55, 60, 64], [41, 48, 52, 55, 60], [43, 50, 55, 59, 62], [40, 47, 52, 55, 59]]
    night_mel = [(0, 72, 2), (2, 71, 1), (3, 67, 1), (4, 69, 3), (8, 72, 1), (9, 74, 1), (10, 76, 2), (12, 74, 2), (14, 71, 2)]
    music("music_home", 70, night_chords, [33, 29, 31, 28], night_mel, False, 12, 0.05)


if __name__ == "__main__":
    main()
