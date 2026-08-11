#!/usr/bin/env python
"""Los Fantasmas del Telar — a passacaglia on a braid word.

The first piece composed *inside* the protected non-Abelian bundle
(paper/ariadne-protected-nonabelian): the C2=1 Wilson--Dirac family's
doubly degenerate acoustic pair, played through the reference package's
own Kato transport and finite-time Schrodinger evolution, staged in the
mus spatial scene renderer.

Form (one timeline, ~4:10):
  I    El fundamento    the braid ground B∘A cycles; pure Kato (geometric)
                        transport re-voices a ringing chord at exactly
                        constant energy — the ostinato that never repeats.
  II   Las variaciones  the same word at five Schrodinger durations, fast
                        and sloppy to slow and sure; leakage out of the
                        protected pair is audible as the overhead sombra,
                        dying away as the gesture is learned.
  III  El fantasma      A·B, then B·A, then the closed commutator; the
                        residue between beginning and end sustained as an
                        interference ghost.
  IV   El trinquete     the exact Z3 pump: [C3,C3,C3',C3] walks one cell
                        per bar, [C3',C3,C3,C3] stays trapped — rhythm as
                        winding number, plucked across a spatial arc.
  V    El borde         mass -3.0 -> -3.95: the gap closes, adiabaticity
                        fails, the same word stops landing; the sombra
                        swallows the chord.
  Coda El otro lado     mass -5 (C2=0, trivial phase): the ground barely
                        moves; one reveal pluck; silence.

Laws asserted in-script (fail loud): holonomy unitarity, constant total
energy under Kato transport, variation-endpoint convergence with
duration, Z3 cell shifts -1 vs 0.

Audio mapping: the 8 realified coordinates [Re z; Im z] are 8 voices on
a D-minor ninth (the chaconne's world — this piece is its other side).
Voice i and i+4 are the same complex mode's real and imaginary parts,
seated on opposite sides of the room: dynamical phase swaps sound across
the hall, geometric transport migrates it around the ring.

Run: .venv/bin/python mus-rs/tools/gen_fantasmas.py [--out DIR]
Then: mus scene-render <out>/fantasmas_scene.json --root <out>/stems
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

import numpy as np

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "paper" / "ariadne-gesture-compiler" / "reference"))

from ariadne_gesture.protected import (  # noqa: E402
    Dirac4DParameters,
    dirac4d_hamiltonian,
    kato_direct_rotation,
    occupied_projector,
    torus_smooth_loop,
    wrap_torus,
    z3_pump_operators,
)

try:
    from scipy.linalg import expm
except ImportError:  # pragma: no cover
    print("scipy required", file=sys.stderr)
    raise

SR = 48000
BASE = (-0.4, 0.0, 0.9, -0.7)
PATH_POINTS = 193
# The chaconne's D-minor world, one voice per realified coordinate.
VOICE_HZ = [146.832, 174.614, 220.0, 261.626, 329.628, 391.995, 466.164, 587.330]
PARTIAL_W = [1.0, 0.35, 0.15]

rng = np.random.default_rng(0xFA27A57A)


def loops_for(params: Dirac4DParameters):
    a = torus_smooth_loop(BASE, 2, 3, 1.50, 1.20, points=PATH_POINTS)
    b = torus_smooth_loop(BASE, 0, 3, 1.30, 1.00, points=PATH_POINTS)
    return a, b


def occupied_frame_at(point, params) -> np.ndarray:
    _, frame = np.linalg.eigh(dirac4d_hamiltonian(wrap_torus(np.asarray(point)), params))
    return frame[:, :2]


def kato_walk(path, params, z0) -> tuple[list[np.ndarray], np.ndarray]:
    """Per-step geometric transport of state z0 along a closed path."""
    old = occupied_projector(wrap_torus(np.asarray(path[0])), params)
    total = np.eye(4, dtype=np.complex128)
    states = [z0.copy()]
    for point in path[1:]:
        new = occupied_projector(wrap_torus(np.asarray(point)), params)
        step, _ = kato_direct_rotation(old, new)
        total = step @ total
        states.append(total @ z0)
        old = new
    return states, total


def schrodinger_walk(path, params, z0, duration) -> tuple[list[np.ndarray], list[float]]:
    """Finite-time evolution; returns states and per-step leakage magnitude."""
    evolution = np.eye(4, dtype=np.complex128)
    step_duration = duration / (len(path) - 1)
    states = [z0.copy()]
    leaks = [0.0]
    for left, right in zip(path, path[1:]):
        lv = wrap_torus(np.asarray(left))
        rv = wrap_torus(np.asarray(right))
        disp = (rv - lv + math.pi) % (2.0 * math.pi) - math.pi
        mid = wrap_torus(lv + 0.5 * disp)
        h = dirac4d_hamiltonian(mid, params)
        evolution = expm(-1.0j * h * step_duration) @ evolution
        z = evolution @ z0
        proj = occupied_projector(rv, params)
        occ = proj @ z
        states.append(occ)
        leaks.append(float(np.linalg.norm(z - occ)))
    return states, leaks


def realify(z: np.ndarray) -> np.ndarray:
    return np.concatenate([np.real(z), np.imag(z)])


class Timeline:
    """Per-voice amplitude breakpoint envelopes on one global clock."""

    def __init__(self) -> None:
        self.tracks: dict[str, list[tuple[float, float]]] = {}
        self.plucks: dict[str, list[tuple[float, float, float]]] = {}
        self.end = 0.0

    def seg(self, name: str, t0: float, t1: float, a0: float, a1: float) -> None:
        self.tracks.setdefault(name, []).append((t0, a0))
        self.tracks[name].append((t1, a1))
        self.end = max(self.end, t1)

    def states_span(self, states, t0: float, t1: float, gain: float = 1.0) -> None:
        """Lay a sequence of 4-complex states across [t0, t1] as 8 voices."""
        n = len(states)
        times = np.linspace(t0, t1, n)
        for k, z in enumerate(states):
            s = np.abs(realify(np.asarray(z)))
            for v in range(8):
                self.tracks.setdefault(f"telar_{v}", []).append(
                    (float(times[k]), float(s[v]) * gain)
                )
        self.end = max(self.end, t1)

    def leak_span(self, leaks, t0: float, t1: float, gain: float = 1.0) -> None:
        n = len(leaks)
        times = np.linspace(t0, t1, n)
        for k, magnitude in enumerate(leaks):
            self.tracks.setdefault("sombra", []).append(
                (float(times[k]), float(magnitude) * gain)
            )
        self.end = max(self.end, t1)

    def pluck(self, name: str, t: float, hz: float, amp: float) -> None:
        self.plucks.setdefault(name, []).append((t, hz, amp))
        self.end = max(self.end, t + 4.0)


def render_envelope(points: list[tuple[float, float]], n_total: int) -> np.ndarray:
    """Piecewise-linear envelope, 30 ms one-pole smoothed."""
    env = np.zeros(n_total, dtype=np.float64)
    points = sorted(points)
    times = np.array([p[0] for p in points]) * SR
    values = np.array([p[1] for p in points])
    env = np.interp(np.arange(n_total), times, values, left=0.0, right=0.0)
    pole = math.exp(-1.0 / (0.03 * SR))
    out = np.empty_like(env)
    acc = 0.0
    for i in range(n_total):
        acc = pole * acc + (1.0 - pole) * env[i]
        out[i] = acc
    return out


def voice_signal(hz: float, n_total: int, vib_hz: float, vib_cents: float) -> np.ndarray:
    t = np.arange(n_total) / SR
    vib = 2.0 ** (vib_cents * np.sin(2 * math.pi * vib_hz * t) / 1200.0)
    signal = np.zeros(n_total)
    for p, w in enumerate(PARTIAL_W, start=1):
        phase = 2 * math.pi * np.cumsum(hz * p * vib) / SR
        signal += w * np.sin(phase + rng.uniform(0, 2 * math.pi))
    return signal / sum(PARTIAL_W)


def sombra_signal(n_total: int) -> np.ndarray:
    noise = rng.standard_normal(n_total)
    # crude band: lowpass 4 kHz minus lowpass 900 Hz
    def onepole(x, cutoff):
        pole = math.exp(-2 * math.pi * cutoff / SR)
        y = np.empty_like(x)
        acc = 0.0
        for i in range(len(x)):
            acc = pole * acc + (1 - pole) * x[i]
            y[i] = acc
        return y

    return onepole(noise, 4000.0) - onepole(noise, 900.0)


def pluck_wave(hz: float, seconds: float = 3.5) -> np.ndarray:
    n = int(seconds * SR)
    t = np.arange(n) / SR
    sig = np.zeros(n)
    for p, w in enumerate(PARTIAL_W, start=1):
        sig += w * np.sin(2 * math.pi * hz * p * t) * np.exp(-t * (2.2 + 1.4 * p))
    attack = np.minimum(t / 0.004, 1.0)
    return sig * attack / sum(PARTIAL_W)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, default=REPO / "paper" / "ariadne-protected-nonabelian" / "fantasmas")
    args = parser.parse_args()
    stems_dir = args.out / "stems"
    stems_dir.mkdir(parents=True, exist_ok=True)

    base_params = Dirac4DParameters()
    path_a, path_b = loops_for(base_params)
    path_a_inv = tuple(reversed(path_a))
    path_b_inv = tuple(reversed(path_b))
    frame0 = occupied_frame_at(BASE, base_params)
    z0 = frame0 @ np.array([0.82, 0.42 + 0.39j], dtype=np.complex128)
    z0 /= np.linalg.norm(z0)

    tl = Timeline()
    report: dict[str, object] = {}

    # ---- I. El fundamento: 4 Kato cycles of the ground word B∘A ---------
    t = 2.0
    z = z0.copy()
    energy_drift = 0.0
    for _cycle in range(4):
        for path in (path_a, path_b):
            states, _ = kato_walk(path, base_params, z)
            tl.states_span(states, t, t + 7.0)
            energies = [float(np.vdot(s, s).real) for s in states]
            energy_drift = max(energy_drift, max(abs(e - 1.0) for e in energies))
            z = states[-1]
            t += 7.0
    assert energy_drift < 1e-9, f"Kato transport leaked energy: {energy_drift}"
    report["kato_energy_drift"] = energy_drift
    fundamento_end_state = z.copy()

    # ---- II. Las variaciones: one word, five durations ------------------
    t += 2.0
    word = tuple(path_a) + tuple(path_b)[1:]
    durations = [3.0, 8.0, 20.0, 48.0, 120.0]
    endpoints = []
    for duration in durations:
        states, leaks = schrodinger_walk(word, base_params, z0, duration)
        tl.states_span(states, t, t + 9.0)
        tl.leak_span(leaks, t, t + 9.0, gain=1.6)
        endpoints.append(states[-1] / max(np.linalg.norm(states[-1]), 1e-12))
        t += 10.0
    kato_states, _ = kato_walk(word, base_params, z0)
    kato_end = kato_states[-1]
    conv = [
        float(np.linalg.norm(np.abs(e) - np.abs(kato_end)))
        for e in endpoints
    ]
    assert conv[-1] < 0.05, f"longest variation failed to converge: {conv}"
    assert conv[0] > conv[-1], f"leakage did not decrease with duration: {conv}"
    report["variation_convergence"] = conv

    # ---- III. El fantasma ------------------------------------------------
    t += 2.0
    for first, second in ((path_a, path_b), (path_b, path_a)):
        z = z0.copy()
        for path in (first, second):
            states, _ = kato_walk(path, base_params, z)
            tl.states_span(states, t, t + 6.0)
            z = states[-1]
            t += 6.0
        tl.states_span([z] * 2, t, t + 4.0)
        t += 4.5
    z = z0.copy()
    for path in (path_a, path_b, path_a_inv, path_b_inv):
        states, _ = kato_walk(path, base_params, z)
        tl.states_span(states, t, t + 4.5)
        z = states[-1]
        t += 4.5
    phantom = z.copy()
    phantom_distance = float(np.linalg.norm(phantom - z0))
    assert phantom_distance > 0.3, "the commutator ghost vanished"
    report["phantom_distance"] = phantom_distance
    # the ghost: sustain the arrival, and the origin voicing beneath it
    tl.states_span([phantom] * 2, t, t + 7.0)
    origin = np.abs(realify(z0))
    times = (t, t + 7.0)
    for v in range(8):
        tl.seg(f"telar_{v}", times[0], times[1], float(origin[v]) * 0.4, 0.0)
    t += 8.0

    # ---- IV. El trinquete ------------------------------------------------
    pump = z3_pump_operators(cells=7)
    assert pump.certificate.path_one_cell_shift == -1
    assert pump.certificate.path_two_cell_shift == 0
    report["z3_shifts"] = (
        pump.certificate.path_one_cell_shift,
        pump.certificate.path_two_cell_shift,
    )
    state_hz = {0: 293.665, 1: 349.228, 2: 220.0}  # +, -, link
    words = [
        (pump.c3, pump.c3, pump.c3_prime, pump.c3),
        (pump.c3, pump.c3, pump.c3_prime, pump.c3),
        (pump.c3, pump.c3, pump.c3_prime, pump.c3),
        (pump.c3, pump.c3, pump.c3_prime, pump.c3),
        (pump.c3_prime, pump.c3, pump.c3, pump.c3),
        (pump.c3_prime, pump.c3, pump.c3, pump.c3),
        (pump.c3, pump.c3, pump.c3_prime, pump.c3),
        (pump.c3, pump.c3, pump.c3_prime, pump.c3),
    ]
    state = np.zeros(21)
    state[3 * 3 + 0] = 1.0  # center cell, + state
    for bar in words:
        for op in bar:
            state = op @ state
            idx = int(np.argmax(np.abs(state)))
            cell, internal = idx // 3, idx % 3
            tl.pluck(f"cell_{cell}", t, state_hz[internal], 0.5)
            t += 0.9
        t += 0.4
    t += 2.0

    # ---- V. El borde -----------------------------------------------------
    masses = [-3.0, -3.3, -3.6, -3.8, -3.95]
    gaps = []
    leak_peaks = []
    for mass in masses:
        params = Dirac4DParameters(mass=mass)
        states, leaks = schrodinger_walk(path_a, params, z0, 24.0)
        gap = min(
            2.0 * float(np.linalg.norm(
                dirac4d_hamiltonian(wrap_torus(np.asarray(p)), params) @ occupied_frame_at(p, params)[:, :1]
                - occupied_frame_at(p, params)[:, :1]
                * float(np.linalg.eigvalsh(dirac4d_hamiltonian(wrap_torus(np.asarray(p)), params))[0])
            ))
            for p in path_a[:: len(path_a) // 8]
        )
        gaps.append(gap)
        leak_peaks.append(max(leaks))
        tl.states_span(states, t, t + 7.0)
        tl.leak_span(leaks, t, t + 7.0, gain=2.2)
        t += 8.0
    assert leak_peaks[-1] > leak_peaks[0], f"gap closure did not grow leakage: {leak_peaks}"
    report["borde_leak_peaks"] = leak_peaks

    # ---- Coda: el otro lado ---------------------------------------------
    t += 2.0
    trivial = Dirac4DParameters(mass=-5.0)
    frame_trivial = occupied_frame_at(BASE, trivial)
    z_trivial = frame_trivial @ np.array([0.82, 0.42 + 0.39j], dtype=np.complex128)
    z_trivial /= np.linalg.norm(z_trivial)
    z = z_trivial.copy()
    movement = 0.0
    for path in (path_a, path_b, path_a, path_b):
        states, _ = kato_walk(path, trivial, z)
        tl.states_span(states, t, t + 5.0, gain=0.8)
        movement = max(
            movement,
            float(np.linalg.norm(np.abs(realify(states[-1])) - np.abs(realify(states[0])))),
        )
        z = states[-1]
        t += 5.0
    report["coda_movement"] = movement
    # the reveal pluck: the final voicing, struck once
    final = np.abs(realify(z))
    for v in range(8):
        if final[v] > 0.05:
            tl.pluck(f"telar_{v}_pluck", t + 1.0, VOICE_HZ[v], float(final[v]))
    total_seconds = t + 8.0

    # ---- render stems ----------------------------------------------------
    n_total = int(total_seconds * SR)
    stems: dict[str, np.ndarray] = {}
    for v in range(8):
        name = f"telar_{v}"
        env = render_envelope(tl.tracks.get(name, [(0, 0)]), n_total)
        stems[name] = (voice_signal(VOICE_HZ[v], n_total, 0.13 + 0.02 * v, 3.5) * env).astype(np.float32)
    env = render_envelope(tl.tracks.get("sombra", [(0, 0)]), n_total)
    stems["sombra"] = (sombra_signal(n_total) * env * 0.5).astype(np.float32)
    for name, events in tl.plucks.items():
        buf = np.zeros(n_total, dtype=np.float64)
        for when, hz, amp in events:
            wave = pluck_wave(hz)
            start = int(when * SR)
            end = min(start + len(wave), n_total)
            buf[start:end] += amp * wave[: end - start]
        key = name if name.startswith("cell_") else "revelacion"
        stems[key] = stems.get(key, np.zeros(n_total, dtype=np.float32)) + buf.astype(np.float32)

    import soundfile as sf

    for name, audio in stems.items():
        peak = float(np.abs(audio).max()) or 1.0
        sf.write(stems_dir / f"{name}.wav", (audio / max(peak, 1.0) * 0.9).astype(np.float32), SR, subtype="FLOAT")

    # ---- scene -----------------------------------------------------------
    objects = []
    for v in range(8):
        azimuth = math.radians(v * 45.0)
        objects.append({
            "objectId": f"telar_{v}", "label": f"voz {v}", "audio": f"telar_{v}.wav",
            "position": {"x": 2.6 * math.sin(azimuth), "y": 0.3 if v % 2 == 0 else 1.5,
                         "z": -2.6 * math.cos(azimuth)},
            "gainDb": 0, "reverbSend": 0.3,
        })
    objects.append({"objectId": "sombra", "label": "la sombra", "audio": "sombra.wav",
                    "position": {"x": 0.0, "y": 3.2, "z": -0.5}, "gainDb": -2, "reverbSend": 0.5})
    for cell in range(7):
        azimuth = math.radians(-75 + cell * 25)
        objects.append({
            "objectId": f"cell_{cell}", "label": f"celda {cell}", "audio": f"cell_{cell}.wav",
            "position": {"x": 3.2 * math.sin(azimuth), "y": 0.0, "z": -3.2 * math.cos(azimuth)},
            "gainDb": -1, "reverbSend": 0.22,
        }) if f"cell_{cell}" in stems else None
    if "revelacion" in stems:
        objects.append({"objectId": "revelacion", "label": "la revelacion", "audio": "revelacion.wav",
                        "position": {"x": 0.0, "y": 0.8, "z": -1.8}, "gainDb": 1, "reverbSend": 0.4})
    objects = [o for o in objects if o]

    scene = {
        "sceneId": "fantasmas-del-telar",
        "title": "Los Fantasmas del Telar",
        "sampleRate": SR,
        "durationSeconds": float(total_seconds + 4.0),
        "coordinateSystem": "+X right, +Y up, -Z front",
        "listener": {},
        "room": {"enabled": True, "decaySeconds": 2.6, "wet": 0.35, "damping": 0.45, "seed": 1949},
        "metadata": {"form": "passacaglia on a braid word", "bundle": "Wilson-Dirac C2=1",
                     "report": report},
        "objects": objects,
    }
    (args.out / "fantasmas_scene.json").write_text(json.dumps(scene, indent=2))
    print(json.dumps(report, indent=2, default=str))
    print(f"stems: {stems_dir}  scene: {args.out / 'fantasmas_scene.json'}  duration: {total_seconds:.1f}s")


if __name__ == "__main__":
    main()
