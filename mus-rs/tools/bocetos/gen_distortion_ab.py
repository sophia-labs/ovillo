#!/usr/bin/env python
"""Distortion topology A/B — why the powered radiator won't crunch.

Pure-numpy sketches of two TOPOLOGIES (not the engine, said plainly),
driven by the same dry guitar phrase:

  A) the convergence team's current shape:
     one symmetric soft_sign knee -> eight high-Q modal resonators.
     Prediction: polite warmth, harmonics strained through a wind chime.

  B) the lawful recipe:
     three cascaded receipted-gain stages with ASYMMETRIC soft knees
     (bias moving with a sag reservoir), passive tilt filters between
     stages, then a LOW-Q broadband radiator tilt with the modal color
     in parallel at -14 dB.
     Prediction: clean -> crunch -> singing, touch-responsive.

Both use identical input gain so the difference is topology, not level.
Structure per file: input -> low drive -> medium -> high.
"""

import numpy as np
import soundfile as sf
from pathlib import Path

SR = 48000
BASE = Path(__file__).resolve().parent
OUT = Path.home() / "Desktop" / "mus-listening" / "bocetos-vst"

dry, sr = sf.read(BASE / "dry.wav")
assert sr == SR
if dry.ndim > 1:
    dry = dry.mean(axis=1)


def biquad(x, f0, q, gain=1.0):
    w = 2 * np.pi * f0 / SR
    alpha = np.sin(w) / (2 * q)
    b0, b1, b2 = alpha, 0.0, -alpha
    a0, a1, a2 = 1 + alpha, -2 * np.cos(w), 1 - alpha
    y = np.zeros_like(x)
    x1 = x2 = y1 = y2 = 0.0
    for i, v in enumerate(x):
        out = (b0 * v + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2) / a0
        x2, x1, y2, y1 = x1, v, y1, out
        y[i] = out
    return y * gain


def one_pole_tilt(x, f0):
    a = np.exp(-2 * np.pi * f0 / SR)
    y = np.zeros_like(x)
    state = 0.0
    for i, v in enumerate(x):
        state = (1 - a) * v + a * state
        y[i] = state
    return y


def soft_sign(x):
    return x / (1.0 + np.abs(x))


def topology_a(x, drive):
    """One symmetric soft knee -> high-Q modal comb (their current shape)."""
    clipped = soft_sign(x * (0.7 + 2.3 * drive) * 3.0)
    modes = [(88, 18), (176, 16), (330, 14), (580, 12),
             (950, 10), (1600, 9), (2700, 8), (4300, 7)]
    out = sum(biquad(clipped, f, q) for f, q in modes) / len(modes) * 6.0
    return out


def topology_b(x, drive):
    """Cascaded asymmetric receipted-gain stages -> broadband tilt +
    parallel low-level modal color."""
    signal = x.copy()
    reservoir = 1.0
    stage_gains = [2.0 + 6.0 * drive, 1.6 + 4.0 * drive, 1.2 + 2.5 * drive]
    for stage, gain in enumerate(stage_gains):
        out = np.zeros_like(signal)
        envelope = 0.0
        attack = 1 - np.exp(-1 / (0.004 * SR))
        recover = 1 - np.exp(-1 / (0.10 * SR))
        for i, v in enumerate(signal):
            # reservoir sags with rectified level; bias rides the sag —
            # asymmetry (even harmonics + touch) emerges from the supply
            envelope += (abs(v) - envelope) * (attack if abs(v) > envelope else recover)
            reservoir += (1.0 - reservoir) * recover - 0.12 * envelope * attack
            reservoir = max(0.35, min(1.0, reservoir))
            bias = 0.22 * (1.0 - reservoir)
            g = gain * (0.55 + 0.45 * reservoir)
            u = g * v + bias
            out[i] = np.tanh(u) - np.tanh(bias)
        signal = one_pole_tilt(out, 5200 - 900 * stage)
    direct = signal * 0.9
    color_modes = [(110, 2.2, 0.8), (420, 2.8, 0.7), (1400, 3.2, 0.9), (3100, 3.5, 0.6)]
    color = sum(biquad(signal, f, q, g) for f, q, g in color_modes) / len(color_modes)
    return direct + 0.2 * color


def montage(topology):
    gap = np.zeros(int(0.6 * SR))
    parts = [dry * 0.8, gap]
    for drive in (0.15, 0.55, 0.95):
        wet = topology(dry, drive)
        wet = wet / max(np.abs(wet).max(), 1e-9) * 0.82
        parts += [wet, gap]
    out = np.concatenate(parts)
    return out / max(np.abs(out).max(), 1e-9) * 0.85


for name, topo in (("07a_distorsion_actual.wav", topology_a),
                   ("07b_distorsion_recipe.wav", topology_b)):
    audio = montage(topo)
    stereo = np.stack([audio, np.roll(audio, int(0.0003 * SR))], axis=1)
    sf.write(OUT / name, stereo.astype(np.float32), SR, subtype="PCM_24")
    print(name, f"{len(audio)/SR:.1f} s")
