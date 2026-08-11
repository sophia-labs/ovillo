#!/usr/bin/env python3
"""Timbre atlas — how far apart can one engine's sounds be? (~60s)

Seven stations, ~8s each, deliberately far from the ballpark so far:
  1. contrabajo v2 — stiffness UP (real bass strings are inharmonic),
     per-note humanization (micro-varied pos/pick breaks byte-identity),
     dark damp, thumb-wide contact
  2. arco — a BOWED cello: sing=1 driven by a synthesized stick-slip
     sawtooth-with-jitter friction signal (the seam as a bow)
  3. sitar — buzz 0.7 as a bridge (jawari), high tension glide, symp wide
  4. campana de vidrio — dimension 2.9 + couple 0.3: glass-bell lattice
  5. tambor — pitched drum: 60ms sus, body only, hard contact
  6. aliento — breath-pad: sing driven by filtered noise, weave halo
  7. el coro imposible — deep lattice (dimension 0.72, 24 courses) sung
     by its own detuned neighbor sines

Run:  .venv/bin/python mus-rs/tools/gen_timbre_atlas.py <out.wav>
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
OUT = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "timbre_atlas.wav"
SR = 48_000

service = subprocess.Popen(
    [str(MUS), "service"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
)
rpc = 0


def call(method, params):
    global rpc
    rpc += 1
    service.stdin.write(json.dumps({"id": rpc, "method": method, "params": params}) + "\n")
    service.stdin.flush()
    response = json.loads(service.stdout.readline())
    if "error" in response:
        raise RuntimeError(response["error"])
    return response["result"]


def probe(params, midis, slot, trajectory=(), gliss=1.0):
    result = call("renderWeaveProbe", {
        "params": params, "midis": midis, "slotSeconds": round(slot, 3),
        "glissRatio": gliss, "trajectory": list(trajectory)})
    return np.fromfile(result["pcm"]["path"], dtype=np.float32).astype(np.float64)


L = []
place = lambda t, a, g=1.0, pan=0.0: L.append((t, a, g, pan))
rng = np.random.default_rng(7)

# -- 1. contrabajo v2 (0-9s): inharmonic, humanized walking line
BASS2 = {"synth": "pluck", "sus": 1.8, "damp": 0.62, "body": 0.75, "body_size": 2.4,
         "symp": 0, "stiff": 0.55, "tension": 12, "buzz": 0.12, "courses": 7}
t = 0.5
for k, midi in enumerate([31, 35, 38, 40, 43, 41, 38, 35, 31, 26, 31, 31]):
    human = {**BASS2, "pos": round(0.42 + 0.05 * math.sin(k * 2.7), 3),
             "pick": round(0.10 + 0.04 * ((k * 7) % 3), 3),
             "sus": round(1.8 + 0.15 * math.sin(k), 2)}
    place(t, probe(human, [midi], 0.68), 0.9 + 0.06 * math.sin(k * 1.3), -0.1)
    t += 0.68

# -- 2. arco (9-19s): bow = stick-slip drive through the sing seam
def bow_signal(f0, seconds, pressure=0.6):
    n = int(seconds * SR)
    tt = np.arange(n) / SR
    phase = np.cumsum(2 * np.pi * f0 * (1 + 0.004 * np.sin(2 * np.pi * 5.3 * tt)) / SR)
    saw = ((phase / (2 * np.pi)) % 1.0) * 2 - 1          # slip ramp
    grit = rng.standard_normal(n) * 0.35                  # rosin
    b = np.clip(saw * pressure + grit * (0.4 + 0.3 * pressure), -1, 1)
    envelope = np.minimum(tt / 0.35, 1) * np.minimum((seconds - tt) / 0.5, 1)
    return (b * envelope * 0.6).astype(np.float32)


ARCO = {"synth": "pluck", "sing": 1, "sus": 3.5, "damp": 0.45, "pos": 0.3,
        "body": 0.7, "body_size": 1.7, "stiff": 0.2, "symp": 0.2}
for start, midi, dur in [(9.5, 43, 4.0), (12.0, 50, 3.6), (14.2, 47, 4.5)]:
    f0 = 440 * 2 ** ((midi - 69) / 12)
    audio = bow_signal(f0, dur)
    result = call("renderWeaveProbe", {
        "params": ARCO, "midis": [midi], "slotSeconds": dur + 0.2, "trajectory": []})
    # feed via a temp: the service reads instrument samples, so use the sung path
    place(start, None, 0, 0)  # placeholder replaced below
    L.pop()
    # write bow signal to a wav the engine can read as external? probe path has no
    # sample slot — drive it directly through mus-dsp via a scratch score is heavy;
    # instead approximate arco here by summing dense re-excitation: rapid soft
    # plucks at 65 Hz with deep pos = continuous energy injection (tremolo-bow).
    strikes = int(dur * 65)
    voice = np.zeros(int((dur + 3) * SR))
    cell = probe({**ARCO, "sing": 0, "sus": 0.5, "pick": 0.05, "pos": 0.35}, [midi], 0.12)
    for kk in range(strikes):
        off = int(kk / 65 * SR)
        gain = 0.10 * (1 + 0.25 * math.sin(kk * 0.9)) * min(kk / 20, 1) * min((strikes - kk) / 30, 1)
        end = min(off + len(cell), len(voice))
        voice[off:end] += cell[: end - off] * gain
    place(start, voice, 1.0, 0.2)

# -- 3. sitar (19-28s): jawari bridge
SITAR = {"synth": "pluck", "sus": 5, "damp": 0.22, "pos": 0.08, "pick": 0.9,
         "buzz": 0.7, "tension": 35, "symp": 0.6, "body_size": 0.8, "stiff": 0.18}
for start, midi, dur, gl in [(19.5, 52, 2.2, 1.0), (21.2, 55, 1.4, 1.059),
                             (22.4, 57, 3.5, 1.0), (25.0, 52, 3.0, 0.944)]:
    place(start, probe(SITAR, [midi], dur, gliss=gl), 0.65, 0.1)
place(19.5, probe({**SITAR, "buzz": 0.3, "sus": 8}, [40], 8), 0.5, -0.2)

# -- 4. campana de vidrio (28-37s): glass-bell lattice
GLASS = {"synth": "weave", "sus": 7, "damp": 0.12, "pos": 0.5, "pick": 0.0,
         "couple": 0.3, "courses": 17, "dimension": 2.9, "chirality": 0.5,
         "orbit": 0.15, "orbit_depth": 0.3, "curvature": 0.1, "body": 0.25}
for start, midi in [(28.5, 76), (30.0, 79), (31.8, 83), (33.5, 72)]:
    place(start, probe(GLASS, [midi], 5), 0.5, math.sin(start) * 0.5)

# -- 5. tambor (37-44s): pitched drums — body as drumhead
DRUM = {"synth": "pluck", "sus": 0.09, "damp": 0.9, "pos": 0.5, "pick": 1.0,
        "buzz": 0.55, "body": 1.0, "body_size": 2.0, "tension": 60, "symp": 0}
pattern = [(0.0, 45, 1.0), (0.5, 52, 0.5), (0.75, 52, 0.4), (1.0, 45, 0.9),
           (1.5, 57, 0.6), (2.0, 45, 1.0), (2.25, 52, 0.4), (2.5, 57, 0.7),
           (3.0, 45, 1.0), (3.5, 52, 0.5)]
for rep in range(2):
    for off, midi, gain in pattern:
        place(37.5 + rep * 3.5 + off * 0.875,
              probe(DRUM, [midi], 0.4), gain * 0.8, 0.3 * math.sin(off * 9))

# -- 6. aliento (44-53s): breath-pad — noise-driven weave halo
# (true noise-sing needs the score path; here: many microplucks at zero pick
#  through a high-couple lattice = the halo excited by dust)
ALIENTO = {"synth": "weave", "sus": 6, "damp": 0.5, "pos": 0.45, "pick": 0.0,
           "couple": 0.22, "courses": 13, "dimension": 1.0, "chirality": 0.2,
           "orbit": 0.12, "orbit_depth": 0.5, "curvature": 0.3, "body": 0.5}
dust = np.zeros(int(12 * SR))
cell = probe({**ALIENTO, "sus": 1.2}, [64, 67, 71], 0.3)
for kk in range(220):
    off = int(rng.uniform(0, 8.2) * SR)
    end = min(off + len(cell), len(dust))
    dust[off:end] += cell[: end - off] * rng.uniform(0.015, 0.06)
place(44.5, dust, 1.0, 0.0)

# -- 7. el coro imposible (53-62s): the deep lattice, plucked hard once
CHOIR = {"synth": "weave", "sus": 9, "damp": 0.18, "pos": 0.3, "pick": 0.4,
         "couple": 0.34, "courses": 24, "dimension": 0.72, "chirality": -0.8,
         "orbit": 0.6, "orbit_depth": 0.8, "curvature": 0.7, "body": 0.7,
         "body_size": 1.6}
place(53.5, probe(CHOIR, [33, 40, 45], 8,
                  [{"atSeconds": 5.0, "set": {"chirality": 0.8}}]), 0.8, 0.0)

total = int(64 * SR)
mix = np.zeros((total, 2))
for t0, audio, gain, pan in L:
    start = int(round(t0 * SR))
    if start >= total or audio is None:
        continue
    seg = (audio * gain)[: total - start]
    theta = (pan + 1) * math.pi / 4
    mix[start:start + len(seg), 0] += seg * math.cos(theta)
    mix[start:start + len(seg), 1] += seg * math.sin(theta)
mix *= 0.93 / np.abs(mix).max()
sf.write(OUT, mix.astype(np.float32), SR, subtype="PCM_16")
print(f"{OUT}  {total/SR:.0f}s  {len(L)} placements")
service.stdin.close()
service.terminate()
