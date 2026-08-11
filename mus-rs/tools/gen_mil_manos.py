#!/usr/bin/env python3
"""Mil manos — a virtuosity study no hand can play (~2:30, stereo).

Thesis piece: the within-note seam + sample-accurate assembly push the
instrument past human motor limits while every gesture stays a
declared, deterministic control path.

  I.   El tremolo imposible — one string, 8 -> 90 strikes/second
       (human tremolo tops out near 13 Hz), pick position sweeping
       bridge->neck continuously across the accelerando.
  II.  La espiral — five voices on one figure at tempo ratios
       1 : phi : phi^2 : 2 : sqrt2 — never realigning — each in its own
       spectral-dimension physics (1.35/1.6/2.0/2.6/1.0), spread across
       the stereo field.
  III. El canon del lazo — a four-voice canon whose entrances differ in
       traversal order (AB/BA/AB/BA) and holonomy depth: the four rungs
       of the exact inverse-design ladder (phi = 15/45/90/160 deg) as
       the four voices. Path memory as counterpoint.
  IV.  Las mil manos — machine-precise strum rolls (+3 / -15 / +40 /
       -60 / +80 ms) under one continuous orbit crank 0 -> 12 Hz -> 0,
       curvature rising, ending on the 24-course deep lattice and a
       single midpoint-pluck bell.

Run:  .venv/bin/python mus-rs/tools/gen_mil_manos.py <out.wav>
"""
import json
import math
import subprocess
import sys
from pathlib import Path

import numpy as np
import soundfile as sf

ROOT = Path(__file__).resolve().parents[2]
MUS = ROOT / "mus-rs/target/release/mus"
OUT = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "mil_manos.wav"
SR = 48_000
PHI = (1 + 5 ** 0.5) / 2

service = subprocess.Popen(
    [str(MUS), "service"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
)
rpc = 0
probe_count = 0


def probe(params, midis, slot, trajectory=()):
    global rpc, probe_count
    rpc += 1
    probe_count += 1
    service.stdin.write(json.dumps({
        "id": rpc, "method": "renderWeaveProbe",
        "params": {"params": params, "midis": midis,
                   "slotSeconds": round(slot, 3), "trajectory": list(trajectory)},
    }) + "\n")
    service.stdin.flush()
    result = json.loads(service.stdout.readline())["result"]
    return np.fromfile(result["pcm"]["path"], dtype=np.float32).astype(np.float64)


L = []  # (t, audio, gain, pan)
place = lambda t, audio, gain=1.0, pan=0.0: L.append((t, audio, gain, pan))

W = {"synth": "weave", "courses": 9, "dimension": 1.35, "couple": 0.08,
     "chirality": 0.0, "orbit": 0.3, "orbit_depth": 0.4, "curvature": 0.2,
     "body": 0.5, "damp": 0.3, "sus": 1.2}

# ------------------------------------------------ I. el tremolo imposible
t = 1.0
rate = 8.0
strike = 0
while t < 29.0:
    progress = (t - 1.0) / 28.0
    rate = 8.0 * (90.0 / 8.0) ** progress
    pos = round(0.5 - 0.44 * progress + 0.03 * math.sin(t * 2.1), 2)
    pick = round(0.3 + 0.6 * progress, 1)
    cell = {**W, "pos": max(pos, 0.06), "pick": pick, "sus": 0.9, "couple": 0.06}
    place(t, probe(cell, [64], 0.4), 0.35 + 0.4 * progress, math.sin(strike) * 0.15)
    t += 1.0 / rate
    strike += 1
place(0.0, probe({**W, "sus": 9, "couple": 0.05, "courses": 7}, [40], 26,
                 [{"atSeconds": s, "set": {"body": round(0.4 + 0.3 * math.sin(s / 3), 2)}}
                  for s in range(1, 24, 2)]), 0.8, 0.0)

# ------------------------------------------------ II. la espiral
FIGURE = [52, 55, 59, 64, 59, 55]
VOICES = [
    (0, 0.55, 1.35, -0.8, 32.0), (12, 0.55 / PHI, 1.6, -0.4, 35.0),
    (19, 0.55 / PHI ** 2, 2.0, 0.1, 38.0), (24, 0.275, 2.6, 0.5, 41.0),
    (-12, 0.55 * 2 ** 0.5, 1.0, 0.8, 44.0),
]
for transpose, period, dim, pan, start in VOICES:
    t = start
    note = 0
    while t < 68.0:
        midi = FIGURE[note % 6] + transpose
        chir = 0.4 if note % 2 == 0 else -0.4
        cell = {**W, "dimension": dim, "chirality": chir, "sus": round(period * 2.5, 2),
                "couple": 0.09}
        envelope = min((t - start) / 6, 1.0) * min((70.0 - t) / 5, 1.0)
        place(t, probe(cell, [midi], round(period * 2.2, 3)), 0.5 * max(envelope, 0), pan)
        t += period
        note += 1

# ------------------------------------------------ III. el canon del lazo
BEAT = 0.75
MELODY = [(59, 2), (57, 1), (55, 1), (54, 2), (52, 2), (55, 1), (59, 1), (64, 3)]
RUNGS = [15, 45, 90, 160]
ORDERS = ["ab", "ba", "ab", "ba"]
for k, (entrance, transpose, pan) in enumerate([(72.0, 0, -0.7), (76.5, 7, -0.25),
                                                (81.0, 12, 0.25), (85.5, 19, 0.7)]):
    phi_target = math.radians(RUNGS[k])
    amp = 2 * math.asin(math.sqrt(math.sin(phi_target / 4)))
    depth = round(min(0.08 + amp * 0.22, 0.45), 3)
    swing = round(min(amp, 1.0), 3)
    t = entrance
    for midi, beats in MELODY:
        slot = beats * BEAT
        a = {"couple": depth}; b = {"chirality": swing}
        a0 = {"couple": W["couple"]}; b0 = {"chirality": 0.0}
        seq = [a, b, a0, b0] if ORDERS[k] == "ab" else [b, a, b0, a0]
        loop = [{"atSeconds": round(slot * f, 3), "set": s}
                for f, s in zip((0.15, 0.4, 0.65, 0.9), seq)]
        place(t, probe({**W, "sus": 2.5}, [midi + transpose], round(slot * 1.15, 3), loop),
              0.62, pan)
        t += slot
for t0, midi in [(72.0, 28), (81.0, 33), (90.0, 35), (94.0, 28)]:
    place(t0, probe({**W, "sus": 8, "couple": 0.05, "courses": 7}, [midi], 9), 0.62, 0.0)

# ------------------------------------------------ IV. las mil manos
CHORDS = [
    ([40, 47, 52, 55, 59, 64], 3, 0.9, 0.25), ([36, 48, 52, 55, 60, 64], -15, -0.9, 0.35),
    ([45, 52, 57, 60, 64, 69], 40, 0.9, 0.5), ([47, 51, 54, 57, 62, 66], -60, -0.9, 0.65),
    ([40, 47, 52, 55, 59, 64], 80, 0.9, 0.7),
]
for k, (midis, strum, chir, curve) in enumerate(CHORDS):
    t0 = 100.0 + k * 6.0
    crank_profile = 12.0 * math.sin(math.pi * (k * 6.0 + 3.0) / 30.0)
    crank = [{"atSeconds": round(0.5 + i * 0.7, 2),
              "set": {"orbit": round(max(12.0 * math.sin(math.pi * (k * 6 + 0.5 + i * 0.7 + 3) / 30.0), 0), 2)}}
             for i in range(8)]
    deep = k == 4
    cell = {**W, "strum": strum, "chirality": chir, "curvature": curve,
            "sus": 5 if not deep else 7, "couple": 0.14 if not deep else 0.3,
            "orbit_depth": 0.7, "courses": 11 if not deep else 24,
            "dimension": 1.35 if not deep else 0.72, "body": 0.6}
    place(t0, probe(cell, midis, 7.5, crank), 0.8, -0.3 if k % 2 else 0.3)
place(132.0, probe({**W, "sus": 10, "pos": 0.5, "pick": 0.0, "couple": 0.05}, [76], 12), 0.5, 0.0)
place(132.0, probe({**W, "sus": 10, "couple": 0.04, "courses": 7}, [28], 12), 0.6, 0.0)

# ------------------------------------------------ stereo assembly
total = int((max(t + len(a) / SR for t, a, _, _ in L) + 0.5) * SR)
mix = np.zeros((total, 2))
for t, audio, gain, pan in L:
    start = int(round(t * SR))
    seg = audio * gain
    theta = (pan + 1) * math.pi / 4
    mix[start:start + len(seg), 0] += seg * math.cos(theta)
    mix[start:start + len(seg), 1] += seg * math.sin(theta)

mix *= 0.95 / np.abs(mix).max()
sf.write(OUT, mix.astype(np.float32), SR, subtype="PCM_16")
print(f"{OUT}  {total/SR:.1f}s  {len(L)} events  {probe_count} probes (memoized by the service)")
service.stdin.close()
service.terminate()
