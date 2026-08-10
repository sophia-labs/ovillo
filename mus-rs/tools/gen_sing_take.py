#!/usr/bin/env python3
"""A 16-second vowel take for the `sing` demo (cuerdas simpaticas).

A voice proxy until a human take exists: glottal-ish sawtooth source with
a slow pitch contour and vibrato, through two morphing formant resonators
(ah -> oh -> oo -> ah), breath-shaped amplitude. Deterministic.

Run:  .venv/bin/python mus-rs/tools/gen_sing_take.py
Writes aigua/sing_take.wav (gitignored, like every aigua wav).
"""
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import lfilter

SR = 48_000
DUR = 16.0
ROOT = Path(__file__).resolve().parents[2]

t = np.arange(int(DUR * SR)) / SR

# Pitch contour: G3 drifting through the take, with 5.2 Hz vibrato that
# deepens after each phrase onset.
f0 = 196.0 * 2 ** (
    (2.0 * np.sin(2 * np.pi * t / 16.0) + 1.0 * np.sin(2 * np.pi * t / 5.3)) / 12.0
)
vibrato_depth = 0.18 * np.clip(np.sin(np.pi * (t % 4.0) / 4.0) * 2.0, 0, 1)
f0 = f0 * 2 ** (vibrato_depth * np.sin(2 * np.pi * 5.2 * t) / 12.0)

phase = np.cumsum(2 * np.pi * f0 / SR)
source = ((phase / (2 * np.pi)) % 1.0) * 2.0 - 1.0  # saw
source = source - lfilter([1], [1, -0.995], source * 0.02)  # tame DC drift

# Formant morph: (F1, F2) ah(800,1150) -> oh(500,900) -> oo(350,700) -> ah
keys = np.array([[800, 1150], [500, 900], [350, 700], [800, 1150]], dtype=float)
key_t = np.array([0.0, 5.5, 10.5, 16.0])
f1 = np.interp(t, key_t, keys[:, 0])
f2 = np.interp(t, key_t, keys[:, 1])

def formant_pass(x: np.ndarray, centers: np.ndarray, bw: float) -> np.ndarray:
    """Block-wise two-pole resonator whose center tracks `centers`."""
    out = np.zeros_like(x)
    block = 480
    z1 = z2 = 0.0
    for start in range(0, len(x), block):
        end = min(start + block, len(x))
        fc = centers[start]
        r = np.exp(-np.pi * bw / SR)
        theta = 2 * np.pi * fc / SR
        a1, a2 = -2 * r * np.cos(theta), r * r
        b0 = (1 - r) * np.sqrt(1 + r * r - 2 * r * np.cos(2 * theta))
        for i in range(start, end):
            y = b0 * x[i] - a1 * z1 - a2 * z2
            z2, z1 = z1, y
            out[i] = y
    return out

voice = 0.9 * formant_pass(source, f1, 110.0) + 0.55 * formant_pass(source, f2, 150.0)

# Breath phrasing: four 4s phrases, soft onsets, real gaps between.
env = np.clip(np.sin(np.pi * (t % 4.0) / 4.0) ** 0.6, 0, 1)
env[(t % 4.0) > 3.65] = 0.0
attack = np.minimum(t / 0.4, 1.0)
voice = voice * env * attack

voice = voice / np.max(np.abs(voice)) * 0.7
out = ROOT / "aigua" / "sing_take.wav"
sf.write(out, voice.astype(np.float32), SR)
print(f"{out} ({DUR}s, peak {np.max(np.abs(voice)):.2f})")
