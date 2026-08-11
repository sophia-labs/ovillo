#!/usr/bin/env python
"""Mambo del Telar (hilo puro) — render + the 4D braid solo.

The score (aigua/mambo_hilo.mus) is the band: strictly the string
network, no samples, no oscillators. This tool renders it, then adds
the one voice a score cannot carry: el hilo cuatridimensional — glossy
midpoint-bell weave plucks whose rings are re-voiced mid-decay by
compiled closed SO(3) words through the Phase 2 gesture seam, with
dynamics set by the protected bundle's interference law (the same
Kato orbit as Pizzicato Fantasma). The solo answers the saxes in the
mambo section, rides the coro, and ties the inverse word into the
final chord — the last ring in the piece is a braid coming home.

Run: .venv/bin/python mus-rs/tools/gen_mambo_hilo.py [--out DIR]
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import fftconvolve
from scipy.spatial.transform import Rotation

ROOT = Path(__file__).resolve().parents[2]
MUS = ROOT / "mus-rs/target/release/mus"
sys.path.insert(0, str(ROOT / "paper" / "ariadne-gesture-compiler" / "reference"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

from ariadne_gesture.closed_loop import compile_exact_closed_loop_so3  # noqa: E402
from ariadne_gesture.core import Graph, compile_so_on_tree  # noqa: E402
from gen_fantasmas import BASE, loops_for, kato_walk, occupied_frame_at  # noqa: E402
from ariadne_gesture.protected import Dirac4DParameters  # noqa: E402

SR = 48000
BAR = 240.0 / 182.0
QL = 60.0 / 182.0
PATH3 = Graph(n=3, edges=((0, 1), (1, 2)))
AXIS_A = np.array([1.0, -2.0, 3.0]) / np.linalg.norm([1.0, -2.0, 3.0])
AXIS_B = np.array([2.0, 1.0, -1.0]) / np.linalg.norm([2.0, 1.0, -1.0])

SOLO_PATCH = {
    "synth": "weave", "bodyp": "mandolin", "pick": 0.3, "pos": 0.5,
    "sus": 5.0, "damp": 0.14, "stiff": 0.3, "detune": 8, "courses": 9,
    "dimension": 2.0, "couple": 0.14, "orbit": 0.3, "orbit_depth": 0.4,
    "body": 0.5,
}

rpc_id = 0
service = subprocess.Popen(
    [str(MUS), "service"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
)


def call(method, params):
    global rpc_id
    rpc_id += 1
    service.stdin.write(json.dumps({"id": rpc_id, "method": method, "params": params}) + "\n")
    service.stdin.flush()
    response = json.loads(service.stdout.readline())
    if "error" in response:
        raise RuntimeError(response["error"])
    return response["result"]


def so3_word(kind, angle, speed_s, start_s):
    if kind == "none":
        return []
    if kind in ("A", "B"):
        axis = AXIS_A if kind == "A" else AXIS_B
        cert = compile_so_on_tree(Rotation.from_rotvec(axis * angle).as_matrix(), PATH3)
        gates = [(g.i, g.j, g.theta) for g in cert.schedule]
    elif kind == "AB":
        ca = compile_so_on_tree(Rotation.from_rotvec(AXIS_A * angle).as_matrix(), PATH3)
        cb = compile_so_on_tree(Rotation.from_rotvec(AXIS_B * angle).as_matrix(), PATH3)
        gates = [(g.i, g.j, g.theta) for g in ca.schedule + cb.schedule]
    elif kind == "C":
        cert = compile_exact_closed_loop_so3(Rotation.from_rotvec(AXIS_A * angle).as_matrix())
        gates = [(g.i, g.j, g.theta) for g in cert.schedule]
    elif kind == "inverse":
        cert = compile_exact_closed_loop_so3(Rotation.from_rotvec(AXIS_A * angle).as_matrix())
        forward = [(g.i, g.j, g.theta) for g in cert.schedule]
        gates = forward + [(i, j, -t) for (i, j, t) in reversed(forward)]
    else:
        raise ValueError(kind)
    times = np.linspace(start_s, start_s + speed_s, len(gates))
    return [{"atSeconds": float(t), "courses": [int(i), int(j)], "theta": float(th)}
            for (i, j, th), t in zip(gates, times)]


class BundleOrbit:
    def __init__(self):
        self.params = Dirac4DParameters()
        self.path_a, self.path_b = loops_for(self.params)
        frame = occupied_frame_at(BASE, self.params)
        z = frame @ np.array([0.82, 0.42 + 0.39j], dtype=np.complex128)
        self.z0 = z / np.linalg.norm(z)
        self.z = self.z0.copy()

    def advance(self, kind):
        loops = {"none": [], "A": [self.path_a], "B": [self.path_b],
                 "AB": [self.path_a, self.path_b],
                 "C": [self.path_a, self.path_b,
                       tuple(reversed(self.path_a)), tuple(reversed(self.path_b))],
                 "inverse": []}[kind]
        for path in loops:
            states, _ = kato_walk(path, self.params, self.z)
            self.z = states[-1]
        return float(np.linalg.norm(self.z0 + self.z) / 2.0)


def at(bar, ql):
    return (bar - 1) * BAR + ql * QL


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path,
                        default=ROOT / "paper" / "ariadne-protected-nonabelian" / "mambo-hilo")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    # 1. the band
    receipt = json.loads(subprocess.run(
        [str(MUS), "render", str(ROOT / "aigua" / "mambo_hilo.mus"),
         "--base", str(ROOT / "aigua"), "-o", str(args.out / "band.wav"), "--json"],
        capture_output=True, text=True).stdout)
    assert not receipt["badTokens"], receipt["badTokens"]
    band, sr = sf.read(args.out / "band.wav")
    assert sr == SR

    # 2. the braid solo
    orbit = BundleOrbit()
    events = []
    answers = [([72], "A"), ([70], "B"), ([67], "A"), ([75], "B"),
               ([72], "AB"), ([67], "A"), ([70], "B")]
    for k, bar in enumerate(range(36, 49, 2)):
        midis, kind = answers[k]
        events.append((at(bar, 2.5), midis, kind, 1.6, 0.9))
    for k, bar in enumerate(range(53, 68, 2)):
        chord = [60, 67, 75] if k % 2 == 0 else [65, 72, 74]
        events.append((at(bar, 1.5), chord, "AB" if k % 2 == 0 else "C", 2.2, 1.4))
        events.append((at(bar + 1, 3.0), [72 if k % 2 == 0 else 70], "A", 1.4, 0.7))
    events.append((at(69, 0.0), [60, 67, 72], "inverse", 3.5, 2.8))
    events.append((at(76, 0.0), [60, 67, 75, 84], "C", 5.0, 3.0))

    solo = np.zeros(len(band) + 8 * SR)
    gains = []
    for when, midis, kind, dur, speed in events:
        word = so3_word(kind, 1.1, speed_s=speed, start_s=0.1)
        gain = orbit.advance(kind)
        gains.append(round(gain, 3))
        result = call("renderWeaveProbe", {
            "params": SOLO_PATCH, "midis": midis, "slotSeconds": dur, "gesture": word,
        })
        drift = (result.get("gesture") or {}).get("worstPairEnergyDrift", 0.0)
        assert drift < 1e-9, drift
        pcm = np.fromfile(result["pcm"]["path"], dtype=np.float32).astype(np.float64)
        start = int(when * SR)
        solo[start:start + len(pcm)] += (0.4 + 0.5 * gain) * pcm

    # light room for the solo (deterministic IR, same seed family as the scenes)
    rng = np.random.default_rng(1949)
    ir_n = int(1.1 * SR)
    ir = rng.standard_normal(ir_n) * np.exp(-np.arange(ir_n) / (0.22 * SR))
    wet = fftconvolve(solo, ir / np.abs(ir).sum() * 8.0)[: len(solo)]
    solo = solo + 0.22 * wet
    solo = solo[: len(band)]
    peak = np.abs(solo).max()
    if peak > 0:
        solo *= (np.abs(band).max() * 0.55) / peak

    # 3. the mix (solo seated slightly right)
    mix = band.copy()
    mix[:, 0] += solo * 0.66
    mix[:, 1] += solo * 0.75
    final_peak = np.abs(mix).max()
    if final_peak > 0.98:
        mix *= 0.98 / final_peak
    sf.write(args.out / "mambo_hilo.wav", mix.astype(np.float32), SR, subtype="PCM_24")
    print("bundle gains:", gains)
    print("final:", args.out / "mambo_hilo.wav", f"peak {np.abs(mix).max():.3f}")
    service.stdin.close()


if __name__ == "__main__":
    main()
