#!/usr/bin/env python
"""Pizzicato Fantasma — identical plucks, braided rings.

The first piece through the Phase 2 gesture seam: every note is a real
Weave pluck (renderWeaveProbe), and between attack and decay a compiled
braid word — exact 12-gate closed control loops from the reference
compiler, per-edge closure certified by the service receipt — rotates
three of the ring's courses WHILE THE NOTE RINGS. The attacks are
identical; the rings are not. On top, the protected bundle's own
interference law sets each pluck's dynamics: the accumulated braid
state decides how much each pluck is allowed to ring (|z0 + Wz0|/2 —
the "ghost eats 2 dB" law, measured earlier tonight).

Honesty note: within each note the braid acts in the strings themselves
(Phase 2 seam, gesture receipts asserted per probe). Across notes the
interference gains are computed from the bundle algebra and applied as
composition — successive plucks do not yet share live string state
(re-excitation into a ringing network is future work; the sing seam is
the likely door).

Sections:
  I   la fila           pluck train, words cycling none/A/B/AB/C,
                        gains breathing on the accumulated bundle orbit
  II  el eco            pairs down the lamento: control pair rings full
                        (+6.02 dB law), ghost pair loses what the
                        commutator eats (+4.11 dB law)
  III la trenza acelerada  fast words — 12 gates in a quarter second
  IV  la vuelta         one pluck, the inverse 24-gate word across a
                        long ring (state returns to 1e-16), one final
                        full-voiced pluck: the only forte

Run: .venv/bin/python mus-rs/tools/gen_pizzicato_fantasma.py [--out DIR]
Then: mus scene-render <out>/pizzicato_scene.json --root <out>/stems
"""

from __future__ import annotations

import argparse
import json
import math
import subprocess
import sys
from pathlib import Path

import numpy as np
import soundfile as sf

ROOT = Path(__file__).resolve().parents[2]
MUS = ROOT / "mus-rs/target/release/mus"
sys.path.insert(0, str(ROOT / "paper" / "ariadne-gesture-compiler" / "reference"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

from ariadne_gesture.closed_loop import compile_exact_closed_loop_so3  # noqa: E402
from ariadne_gesture.core import Graph, compile_so_on_tree  # noqa: E402
from scipy.spatial.transform import Rotation  # noqa: E402

from gen_fantasmas import BASE, loops_for, kato_walk, occupied_frame_at  # noqa: E402
from ariadne_gesture.protected import Dirac4DParameters  # noqa: E402

SR = 48000
PATH3 = Graph(n=3, edges=((0, 1), (1, 2)))

# The audio-side braid targets (SO(3) on courses 0,1,2 of the ring).
AXIS_A = np.array([1.0, -2.0, 3.0]) / np.linalg.norm([1.0, -2.0, 3.0])
AXIS_B = np.array([2.0, 1.0, -1.0]) / np.linalg.norm([2.0, 1.0, -1.0])

PATCH = {
    "synth": "weave", "sus": 4.5, "damp": 0.24, "pick": 0.62, "pos": 0.17,
    "courses": 9, "dimension": 1.7, "couple": 0.16, "chirality": 0.5,
    "orbit": 0.6, "orbit_depth": 0.45, "curvature": 0.3, "body": 0.55,
    "bodyp": "guitar",
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


def so3_word(kind: str, angle: float, speed_s: float, start_s: float) -> list[dict]:
    """Compile a braid word to service gesture frames on courses (0,1,2)."""
    if kind == "none":
        return []
    if kind in ("A", "B"):
        axis = AXIS_A if kind == "A" else AXIS_B
        target = Rotation.from_rotvec(axis * angle).as_matrix()
        cert = compile_so_on_tree(target, PATH3)
        gates = [(g.i, g.j, g.theta) for g in cert.schedule]
    elif kind == "AB":
        a = Rotation.from_rotvec(AXIS_A * angle).as_matrix()
        b = Rotation.from_rotvec(AXIS_B * angle).as_matrix()
        ca = compile_so_on_tree(a, PATH3)
        cb = compile_so_on_tree(b, PATH3)
        gates = [(g.i, g.j, g.theta) for g in ca.schedule + cb.schedule]
    elif kind == "C":
        target = Rotation.from_rotvec(AXIS_A * angle).as_matrix()
        cert = compile_exact_closed_loop_so3(target)
        gates = [(g.i, g.j, g.theta) for g in cert.schedule]
    elif kind == "inverse":
        target = Rotation.from_rotvec(AXIS_A * angle).as_matrix()
        cert = compile_exact_closed_loop_so3(target)
        forward = [(g.i, g.j, g.theta) for g in cert.schedule]
        gates = forward + [(i, j, -t) for (i, j, t) in reversed(forward)]
    else:
        raise ValueError(kind)
    times = np.linspace(start_s, start_s + speed_s, len(gates)) if gates else []
    return [
        {"atSeconds": float(t), "courses": [int(i), int(j)], "theta": float(th)}
        for (i, j, th), t in zip(gates, times)
    ]


def pluck(midis, slot_s, word) -> np.ndarray:
    result = call("renderWeaveProbe", {
        "params": PATCH, "midis": midis, "slotSeconds": slot_s, "gesture": word,
    })
    receipt = result.get("gesture") or {}
    drift = receipt.get("worstPairEnergyDrift", 0.0)
    assert drift < 1e-9, f"gesture energy drift {drift}"
    return np.fromfile(result["pcm"]["path"], dtype=np.float32).astype(np.float64)


# ---- the bundle layer: accumulated braid state sets the dynamics ----------
class BundleOrbit:
    def __init__(self) -> None:
        self.params = Dirac4DParameters()
        self.path_a, self.path_b = loops_for(self.params)
        frame = occupied_frame_at(BASE, self.params)
        z = frame @ np.array([0.82, 0.42 + 0.39j], dtype=np.complex128)
        self.z0 = z / np.linalg.norm(z)
        self.z = self.z0.copy()

    def advance(self, kind: str) -> float:
        """Apply the word's bundle loops; return the interference gain."""
        loops = {"none": [], "A": [self.path_a], "B": [self.path_b],
                 "AB": [self.path_a, self.path_b],
                 "C": [self.path_a, self.path_b,
                       tuple(reversed(self.path_a)), tuple(reversed(self.path_b))],
                 "inverse": []}[kind]
        for path in loops:
            states, _ = kato_walk(path, self.params, self.z)
            self.z = states[-1]
        return float(np.linalg.norm(self.z0 + self.z) / 2.0)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path,
                        default=ROOT / "paper" / "ariadne-protected-nonabelian" / "pizzicato")
    args = parser.parse_args()
    stems_dir = args.out / "stems"
    stems_dir.mkdir(parents=True, exist_ok=True)

    orbit = BundleOrbit()
    buses = 6
    events: list[tuple[float, int, np.ndarray, float]] = []  # (t, bus, audio, gain)
    report = {"plucks": 0, "words": {}, "gains": []}

    def add(t, bus, audio, gain):
        events.append((t, bus, audio, gain))
        report["plucks"] += 1
        report["gains"].append(round(gain, 4))

    # ---- I. la fila (0-60 s) --------------------------------------------
    chord = [50, 57, 65]  # D3 A3 F4
    cycle = ["none", "A", "B", "AB", "C"]
    t = 1.0
    k = 0
    while t < 58.0:
        kind = cycle[k % len(cycle)]
        word = so3_word(kind, 0.9, speed_s=1.1, start_s=0.12)
        gain = orbit.advance(kind)
        audio = pluck(chord, 1.9, word)
        add(t, k % buses, audio, 0.35 + 0.65 * gain)
        report["words"][kind] = report["words"].get(kind, 0) + 1
        t += 1.5
        k += 1

    # ---- II. el eco (60-112 s): the +6.02 / +4.11 dB law as pairs -------
    lamento = [[50, 57, 65], [48, 55, 64], [46, 53, 62], [45, 52, 61]]
    t = 61.0
    for i, notes in enumerate(lamento * 2):
        ghost = i % 2 == 1
        first_word = so3_word("C", 0.9, speed_s=1.8, start_s=0.15) if ghost else []
        a1 = pluck(notes, 2.6, first_word)
        add(t, (2 * i) % buses, a1, 0.85)
        pair_gain = 10 ** (-1.91 / 20) if ghost else 1.0  # measured phantom loss
        a2 = pluck(notes, 2.6, [])
        add(t + 2.9, (2 * i + 1) % buses, a2, 0.85 * pair_gain)
        t += 6.4

    # ---- III. la trenza acelerada (112-140 s) ---------------------------
    t = 113.0
    arps = [[50, 57, 65], [53, 60, 69], [57, 65, 72], [62, 69, 77],
            [57, 65, 72], [53, 60, 69]]
    k = 0
    while t < 138.0:
        kind = ["C", "A", "B"][k % 3]
        word = so3_word(kind, 1.4, speed_s=0.25, start_s=0.06)
        gain = orbit.advance(kind)
        audio = pluck(arps[k % len(arps)], 1.1, word)
        add(t, k % buses, audio, 0.3 + 0.6 * gain)
        t += 0.8
        k += 1

    # ---- IV. la vuelta (140-165 s) --------------------------------------
    word = so3_word("inverse", 0.9, speed_s=11.0, start_s=0.4)
    audio = pluck([38, 50, 57, 65, 69], 13.0, word)
    add(140.5, 0, audio, 0.9)
    final = pluck([38, 50, 57, 65, 69], 8.0, [])
    add(154.5, 0, final, 1.0)  # the state returned; the only forte

    # ---- mix to bus stems ------------------------------------------------
    total_s = max(t for t, *_ in events) + 12.0
    n_total = int(total_s * SR)
    bus_audio = [np.zeros(n_total) for _ in range(buses)]
    for t, bus, audio, gain in events:
        start = int(t * SR)
        end = min(start + len(audio), n_total)
        bus_audio[bus][start:end] += gain * audio[: end - start]
    for b in range(buses):
        peak = np.abs(bus_audio[b]).max()
        if peak > 0.9:
            bus_audio[b] *= 0.9 / peak
        sf.write(stems_dir / f"bus_{b}.wav", bus_audio[b].astype(np.float32), SR,
                 subtype="FLOAT")

    objects = []
    for b in range(buses):
        azimuth = math.radians(-125 + b * 50)
        objects.append({
            "objectId": f"bus_{b}", "label": f"atril {b}", "audio": f"bus_{b}.wav",
            "position": {"x": 2.8 * math.sin(azimuth), "y": 0.4 + 0.5 * (b % 3),
                         "z": -2.8 * math.cos(azimuth)},
            "gainDb": 0, "reverbSend": 0.3,
        })
    scene = {
        "sceneId": "pizzicato-fantasma",
        "title": "Pizzicato Fantasma",
        "sampleRate": SR,
        "durationSeconds": float(total_s + 3.0),
        "coordinateSystem": "+X right, +Y up, -Z front",
        "listener": {},
        "room": {"enabled": True, "decaySeconds": 2.2, "wet": 0.3, "damping": 0.45,
                 "seed": 1949},
        "metadata": {"form": "pluck train with compiled braid words in the rings",
                     "seam": "Ariadne Phase 2 GestureFrame via renderWeaveProbe",
                     "report": report},
        "objects": objects,
    }
    (args.out / "pizzicato_scene.json").write_text(json.dumps(scene, indent=2))
    print(json.dumps({k: v for k, v in report.items() if k != "gains"}, indent=2))
    print("gain range:", min(report["gains"]), "-", max(report["gains"]))
    print(f"stems: {stems_dir}  scene: {args.out / 'pizzicato_scene.json'}  dur: {total_s:.1f}s")
    service.stdin.close()
    service.wait(timeout=10)


if __name__ == "__main__":
    main()
