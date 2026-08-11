#!/usr/bin/env python3
"""Trajectory études — the within-note automation seam, audible.

Four stems through `mus service` renderWeaveProbe trajectories:
  1. lazo_ab / lazo_ba — the E15 loop pair: identical endpoints, opposite
     order; what differs between the files IS the holonomy.
  2. manivela — the crank: orbit 0 -> 8 Hz -> 0, phase-continuous
     (the accumulator; no zipper at rate changes).
  3. respiracion — the body breathing inside one note (body + damp swing).
  4. escalera — four balanced loops with amplitudes graded by the paper's
     inverse-design law t(phi) = 2*arcsin(sqrt(sin(phi/4))), phi = 15, 45,
     90, 160 degrees. The mapping of t onto (couple-delta, chirality) is
     illustrative — the law is exact for the three-mode commutator angles,
     and here it grades one audible ladder.

Stems are raw voice output (no master chain), peak-normalized to -1 dBFS.
Run:  .venv/bin/python mus-rs/tools/gen_trajectory_etudes.py <outdir>
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
OUT = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "etudes"
OUT.mkdir(parents=True, exist_ok=True)

EM = [40, 47, 52, 55, 59, 64]
BASE = {
    "synth": "weave", "sus": 3.0, "courses": 11, "dimension": 1.6,
    "couple": 0.12, "chirality": 0.0, "orbit": 0.4, "orbit_depth": 0.5,
    "curvature": 0.3, "body": 0.5, "damp": 0.3,
}

service = subprocess.Popen(
    [str(MUS), "service"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
)
rpc_id = 0


def call(method, params):
    global rpc_id
    rpc_id += 1
    service.stdin.write(json.dumps({"id": rpc_id, "method": method, "params": params}) + "\n")
    service.stdin.flush()
    response = json.loads(service.stdout.readline())
    if "error" in response:
        raise RuntimeError(response["error"])
    return response["result"]


def stem(name, params, midis, slot, trajectory):
    result = call("renderWeaveProbe", {
        "params": params, "midis": midis, "slotSeconds": slot, "trajectory": trajectory,
    })
    pcm = np.fromfile(result["pcm"]["path"], dtype=np.float32)
    peak = np.abs(pcm).max() or 1.0
    sf.write(OUT / f"{name}.wav", pcm / peak * 0.891, 48_000, subtype="PCM_16")
    print(f"{name}: {len(pcm)/48000:.1f}s  raw peak {peak:.3f}  renderMs {result['renderMs']}")


# 1. The E15 pair: A = couple up, B = chirality on; loop closes; order flips.
loop_ab = [
    {"atSeconds": 1.0, "set": {"couple": 0.34}},
    {"atSeconds": 2.0, "set": {"chirality": 0.85}},
    {"atSeconds": 3.0, "set": {"couple": 0.12}},
    {"atSeconds": 4.0, "set": {"chirality": 0.0}},
]
loop_ba = [
    {"atSeconds": 1.0, "set": {"chirality": 0.85}},
    {"atSeconds": 2.0, "set": {"couple": 0.34}},
    {"atSeconds": 3.0, "set": {"chirality": 0.0}},
    {"atSeconds": 4.0, "set": {"couple": 0.12}},
]
stem("etude1_lazo_ab", BASE, EM, 5.5, loop_ab)
stem("etude1_lazo_ba", BASE, EM, 5.5, loop_ba)

# 2. The crank: triangular orbit ramp, 30 frames, phase-continuous.
crank = [
    {"atSeconds": round(t, 3), "set": {"orbit": round(8.0 * (1 - abs(t / 3.0 - 1)), 3)}}
    for t in [i * 0.2 for i in range(31)]
]
stem("etude2_manivela", {**BASE, "orbit_depth": 0.7, "sus": 5.0}, EM, 6.5, crank)

# 3. Breathing: two slow body+damp cycles inside one held chord.
breath = []
for i in range(33):
    t = i * 0.25
    phase = math.sin(2 * math.pi * t / 4.0)
    breath.append({
        "atSeconds": round(t, 3),
        "set": {"body": round(0.5 + 0.35 * phase, 3), "damp": round(0.35 - 0.15 * phase, 3)},
    })
stem("etude3_respiracion", {**BASE, "sus": 6.0, "couple": 0.08}, [40, 52, 59], 8.5, breath)

# 4. The inverse-design ladder: one loop per target angle, 2s apart.
ladder = []
for k, phi_deg in enumerate([15, 45, 90, 160]):
    phi = math.radians(phi_deg)
    t_amp = 2 * math.asin(math.sqrt(math.sin(phi / 4)))
    base_t = k * 2.2
    ladder += [
        {"atSeconds": round(base_t + 0.2, 3), "set": {"couple": round(min(0.12 + t_amp * 0.25, 0.45), 3)}},
        {"atSeconds": round(base_t + 0.7, 3), "set": {"chirality": round(min(t_amp, 1.0), 3)}},
        {"atSeconds": round(base_t + 1.2, 3), "set": {"couple": 0.12}},
        {"atSeconds": round(base_t + 1.7, 3), "set": {"chirality": 0.0}},
    ]
    print(f"  ladder rung phi={phi_deg}deg -> t = {t_amp:.3f} rad")
stem("etude4_escalera", {**BASE, "sus": 4.0}, EM, 9.5, ladder)

service.stdin.close()
service.terminate()
print(f"-> {OUT}")
