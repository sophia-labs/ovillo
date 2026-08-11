#!/usr/bin/env python3
"""La vuelta entera — a piece for the within-note seam (~2:15).

A tape composition assembled from trajectory-carrying weave events
(the score language holds no trajectory syntax yet, by ruling; the
service does). Four sections: la respiracion (breathing drones), el
lazo (a chord cycle where every harmony carries a closed control loop,
orders alternating AB/BA — path memory as rhyme), la manivela (the
crank under a held ninth, with high answers), la vuelta (return,
carrying what the loops left behind).

Deterministic; dry by design — the lattice and body ARE the space.
Run:  .venv/bin/python mus-rs/tools/gen_la_vuelta_entera.py <out.wav>
"""
import json
import subprocess
import sys
from pathlib import Path

import numpy as np
import soundfile as sf

ROOT = Path(__file__).resolve().parents[2]
MUS = ROOT / "mus-rs/target/release/mus"
OUT = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "la_vuelta_entera.wav"
SR = 48_000

service = subprocess.Popen(
    [str(MUS), "service"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
)
rpc = 0


def probe(params, midis, slot, trajectory):
    global rpc
    rpc += 1
    service.stdin.write(json.dumps({
        "id": rpc, "method": "renderWeaveProbe",
        "params": {"params": params, "midis": midis, "slotSeconds": slot,
                   "trajectory": trajectory},
    }) + "\n")
    service.stdin.flush()
    result = json.loads(service.stdout.readline())["result"]
    return np.fromfile(result["pcm"]["path"], dtype=np.float32).astype(np.float64)


BASE = {"synth": "weave", "courses": 11, "dimension": 1.6, "couple": 0.10,
        "chirality": 0.0, "orbit": 0.3, "orbit_depth": 0.45, "curvature": 0.25,
        "body": 0.55, "damp": 0.32, "sus": 5.0}


def breath(t0, seconds, period, lo=0.25, hi=0.8):
    frames = []
    t = 0.0
    while t <= seconds:
        phase = np.sin(2 * np.pi * t / period)
        frames.append({"atSeconds": round(t0 + t, 3), "set": {
            "body": round(0.5 + (hi - lo) / 2 * phase, 3),
            "damp": round(0.34 - 0.12 * phase, 3)}})
        t += period / 12
    return frames


def lazo(order, t0, depth=0.30, swing=0.8):
    a = {"couple": depth}
    b = {"chirality": swing}
    a0 = {"couple": BASE["couple"]}
    b0 = {"chirality": 0.0}
    seq = [a, b, a0, b0] if order == "ab" else [b, a, b0, a0]
    return [{"atSeconds": round(t0 + i * 2.0, 3), "set": s} for i, s in enumerate(seq)]


events = []
place = lambda t, audio, gain=1.0: events.append((t, audio * gain))

# I. la respiracion (0:00-0:38)
place(0.0, probe({**BASE, "sus": 8, "couple": 0.06}, [40, 47, 52], 20, breath(0.5, 18, 8)), 0.9)
place(12.0, probe({**BASE, "sus": 8, "couple": 0.06}, [45, 52, 57], 18, breath(0.0, 16, 10)), 0.75)
place(26.0, probe({**BASE, "sus": 8, "couple": 0.07}, [40, 47, 52, 55], 14, breath(0.0, 12, 7)), 0.8)

# II. el lazo (0:38-1:26) — the cycle rhymes in path-order: AB, BA, AB, BA
CHORDS = [
    (38.0, [40, 47, 52, 55, 59, 64], "ab", 0.30, 0.80, 28),
    (50.0, [36, 48, 52, 55, 60], "ba", 0.30, 0.80, 24),
    (62.0, [45, 52, 57, 60, 64], "ab", 0.34, 0.90, 33),
    (74.0, [47, 51, 54, 57, 62], "ba", 0.34, 0.90, 35),
]
for t, midis, order, depth, swing, bass in CHORDS:
    place(t, probe({**BASE, "sus": 5}, midis, 11, lazo(order, 1.5, depth, swing)), 0.85)
    place(t, probe({**BASE, "sus": 7, "couple": 0.05, "courses": 7}, [bass], 11, []), 0.7)

# III. la manivela (1:26-1:56)
crank = [{"atSeconds": round(0.5 + i * 0.5, 3),
          "set": {"orbit": round(6.0 * (1 - abs(i / 16 - 1)), 3)}} for i in range(33)]
crank += [{"atSeconds": 18.0, "set": {"chirality": 0.7}},
          {"atSeconds": 21.0, "set": {"chirality": -0.7}},
          {"atSeconds": 23.5, "set": {"chirality": 0.0}}]
place(86.0, probe({**BASE, "sus": 8, "orbit_depth": 0.7}, [40, 47, 52, 55, 59, 66], 24, crank), 0.85)
place(86.0, probe({**BASE, "sus": 8, "couple": 0.05, "courses": 7}, [28], 22, []), 0.7)
place(92.0, probe({**BASE, "sus": 4}, [64, 71], 6, lazo("ab", 1.0, 0.26, 0.7)), 0.5)
place(100.0, probe({**BASE, "sus": 4}, [62, 67], 6, lazo("ba", 1.0, 0.26, 0.7)), 0.5)
place(106.0, probe({**BASE, "sus": 4}, [59, 64], 6, lazo("ab", 1.0, 0.22, 0.6)), 0.45)

# IV. la vuelta (1:52-end) — the gentlest rung of the ladder, then breath, then silence
gentle = lazo("ab", 2.0, 0.12 + 0.517 * 0.25, min(0.517, 1.0))
fade = [{"atSeconds": round(8 + i, 3), "set": {"body": round(0.5 - i * 0.04, 3)}} for i in range(7)]
place(112.0, probe({**BASE, "sus": 10}, [40, 47, 52, 59, 64], 16, gentle + fade), 0.85)
place(114.0, probe({**BASE, "sus": 10, "couple": 0.05, "courses": 7}, [28], 14, breath(0.0, 12, 9, 0.3, 0.6)), 0.7)

total = int((max(t + len(a) / SR for t, a in events) + 0.5) * SR)
mix = np.zeros(total)
for t, audio in events:
    start = int(t * SR)
    mix[start:start + len(audio)] += audio

mix *= 0.93 / np.abs(mix).max()
sf.write(OUT, mix.astype(np.float32), SR, subtype="PCM_16")
print(f"{OUT}  {total/SR:.1f}s  peak {np.abs(mix).max():.3f}  events {len(events)}")
service.stdin.close()
service.terminate()
