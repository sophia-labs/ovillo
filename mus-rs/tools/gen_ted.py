#!/usr/bin/env python3
"""Para Ted — a chord-melody study after Ted Greene (~100s, stereo).

The deepened guitar in its warm corner: rolled extended voicings with
chromatic inner motion (Gmaj9 -> G#dim7 -> Am9 -> D13 ... the Chord
Chemistry moves), the melody floating 100-160ms behind each roll on a
slightly brighter attack, real rubato (free timing, breaths at phrase
ends), and his signature harp-harmonic cascades — alternating fretted
notes with TRUE midpoint plucks (pos=0.5, pick=0: even harmonics vanish
by nodal geometry). A small warm room is convolved at mix time; the
probe path is dry by design.

Run:  .venv/bin/python mus-rs/tools/gen_ted.py <out.wav>
"""
import json
import math
import subprocess
import sys
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import fftconvolve, lfilter

ROOT = Path(__file__).resolve().parents[2]
MUS = ROOT / "mus-rs/target/release/mus"
OUT = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "para_ted.wav"
SR = 48_000

service = subprocess.Popen(
    [str(MUS), "service"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
)
rpc = 0


def probe(params, midis, slot):
    global rpc
    rpc += 1
    service.stdin.write(json.dumps({
        "id": rpc, "method": "renderWeaveProbe",
        "params": {"params": params, "midis": midis, "slotSeconds": round(slot, 3)},
    }) + "\n")
    service.stdin.flush()
    return np.fromfile(
        json.loads(service.stdout.readline())["result"]["pcm"]["path"], dtype=np.float32
    ).astype(np.float64)


TELE = {"synth": "pluck", "sus": 4.5, "damp": 0.42, "pos": 0.34, "pick": 0.26,
        "body": 0.55, "symp": 0.25, "stiff": 0.05, "tension": 2, "buzz": 0}
MELODY_HAND = {**TELE, "pos": 0.3, "pick": 0.44, "sus": 5.0, "strum": 0}
HARMONIC = {**TELE, "pos": 0.5, "pick": 0.0, "sus": 6.5, "symp": 0.15, "strum": 0}

L = []
place = lambda t, audio, gain=1.0, pan=0.0: L.append((t, audio, gain, pan))


def chord(t, voicing, melody, hold, roll_ms=42, gain=0.8):
    place(t, probe({**TELE, "strum": roll_ms}, voicing, hold), gain, -0.12)
    place(t + 0.06 + roll_ms / 1000 * len(voicing) / 2,
          probe(MELODY_HAND, [melody], hold), gain * 1.12, 0.14)


# ---- Phrase 1: the chorale (Chord Chemistry voice leading, rubato)
PHRASE_1 = [
    ([43, 54, 59, 62], 69, 3.4, 46),   # Gmaj9, mel A4
    ([44, 56, 62, 65], 71, 2.7, 38),   # G#dim7 (bass walks G->G#), mel B4
    ([45, 55, 60, 64], 72, 3.1, 44),   # Am9, mel C5
    ([50, 54, 60, 64], 71, 2.5, 36),   # D13, mel B4
    ([47, 55, 62, 66], 74, 2.9, 44),   # Gmaj7/B, mel D5 (the lift)
    ([46, 58, 64, 67], 73, 2.4, 34),   # Bbdim7 slide, mel Db5
    ([45, 55, 60, 65], 72, 3.2, 46),   # Am11, mel C5
    ([50, 54, 60, 63], 69, 3.0, 40),   # D7b9, mel A4 (leans home)
]
t = 0.8
for voicing, melody, hold, roll in PHRASE_1:
    chord(t, voicing, melody, hold, roll)
    t += hold * 0.82
    if melody in (74, 69):
        t += 0.5  # breathe at the lift and at the half cadence

# ---- Chorus 2: same melody skeleton, re-harmonized (the second-pass
# game: tritone subs and minor-third slides), melody lifted where it sings.
PHRASE_2 = [
    ([43, 54, 59, 64], 74, 3.3, 48),   # Gmaj13, mel D5
    ([41, 53, 59, 63], 74, 2.6, 36),   # F9#11 slide-down sub, mel D5
    ([40, 55, 60, 64], 76, 3.0, 44),   # Em11 (relative), mel E5
    ([44, 56, 62, 66], 75, 2.5, 36),   # Ab13 (tritone sub for D), mel Eb5
    ([47, 55, 62, 66], 74, 2.8, 42),   # Gmaj7/B, mel D5
    ([46, 58, 64, 67], 73, 2.4, 34),   # Bbdim7, mel Db5
    ([45, 55, 60, 65], 72, 3.2, 46),   # Am11, mel C5
    ([49, 55, 61, 65], 71, 3.0, 40),   # D13b5/C# lean, mel B4
]
t += 0.6
for voicing, melody, hold, roll in PHRASE_2:
    chord(t, voicing, melody, hold, roll)
    t += hold * 0.82
    if melody in (76, 71):
        t += 0.5

# ---- The harp cascades (fretted / harmonic alternation, let ring)
def cascade(t0, pairs, step=0.185):
    t = t0
    for midi, is_harmonic in pairs:
        hand = HARMONIC if is_harmonic else {**MELODY_HAND, "pick": 0.3, "sus": 5.5}
        place(t, probe(hand, [midi], 3.5), 0.5 if is_harmonic else 0.58,
              0.35 if is_harmonic else -0.2)
        t += step * (1.0 + 0.05 * math.sin(t))
    return t

t += 0.4
t = cascade(t, [(79, True), (71, False), (74, True), (66, False), (71, True),
                (62, False), (67, True), (59, False), (62, True), (55, False),
                (59, True), (50, False)])
t += 0.55
t = cascade(t, [(76, True), (67, False), (72, True), (64, False), (67, True),
                (60, False), (64, True), (57, False), (60, True), (52, False)],
            step=0.21)

t += 0.5
t = cascade(t, [(83, True), (74, False), (79, True), (71, False), (74, True),
                (67, False), (71, True), (64, False), (67, True), (62, False),
                (64, True), (59, False), (62, True), (55, False)], step=0.17)

# ---- Phrase 3: the cadence
t += 0.7
chord(t, [45, 55, 60, 65], 72, 3.4, 48)          # Am11
t += 2.9
chord(t, [50, 54, 60, 63], 71, 3.0, 38)          # D7b9, mel B4 suspended over
t += 2.7
place(t, probe({**TELE, "strum": 58, "sus": 8}, [43, 50, 57, 62, 64], 9.0), 0.85, -0.1)
place(t + 0.24, probe({**MELODY_HAND, "sus": 8}, [69], 8.5), 0.9, 0.12)   # G6/9, mel A4
t += 1.7
place(t, probe(HARMONIC, [83], 6.5), 0.42, 0.4)   # B5 harmonic shimmer
t += 0.9
place(t, probe(HARMONIC, [86], 6.0), 0.36, -0.35)  # D6 harmonic, floating over

# ---- stereo assembly + a small warm room
total = int((max(t + len(a) / SR for t, a, _, _ in L) + 1.0) * SR)
mix = np.zeros((total, 2))
for t0, audio, gain, pan in L:
    start = int(round(t0 * SR))
    seg = audio * gain
    theta = (pan + 1) * math.pi / 4
    mix[start:start + len(seg), 0] += seg * math.cos(theta)
    mix[start:start + len(seg), 1] += seg * math.sin(theta)

rng = np.random.default_rng(1946)  # Ted's birth year
impulse_t = np.arange(int(1.1 * SR)) / SR
for channel in range(2):
    impulse = rng.standard_normal(len(impulse_t)) * np.exp(-impulse_t * 4.2)
    impulse = lfilter([1 - 0.72], [1, -0.72], impulse)  # warm the tail
    impulse[0] = 0.0
    wet = fftconvolve(mix[:, channel], impulse * 0.055)[:total]
    mix[:, channel] = mix[:, channel] * 0.88 + wet

mix *= 0.93 / np.abs(mix).max()
sf.write(OUT, mix.astype(np.float32), SR, subtype="PCM_16")
print(f"{OUT}  {total/SR:.1f}s  {len(L)} events")
service.stdin.close()
service.terminate()
