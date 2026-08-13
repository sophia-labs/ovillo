#!/usr/bin/env python
"""Body and Soul A/B: Ted Greene rig vs Julian Lage 'Love Hurts' rig.

Same 109-event performance — the rig is the only variable.

Ted (file 09):  neck-ish pos 0.30, soft nails, symp 0.55, one whisper
                stage, 7.5k tilt, 10% plate.
Lage (this):    brighter sensing (pos 0.20), a little more nail
                (pick 0.50), snappier steel (stiff 0.38, tension 20),
                symp 0.40, TWO gentle asymmetric stages at the edge of
                bloom (tweed champ, not clean), open top (9.5k tilt +
                small 4k presence), and SPRING reverb at 18% with 12 ms
                pre-delay — brighter, boingier IR than the plate.

Outputs:
  10_body_and_soul_lage.wav      full chorus through the Lage rig
  11_ted_vs_lage_cambio.wav      A1=Ted · A2=Lage · bridge=Ted ·
                                 A3+coda=Lage (cuts at section seams)
"""

import sys
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import fftconvolve

sys.path.insert(0, str(Path(__file__).resolve().parent))
import gen_body_and_soul as g  # noqa: E402  (shares its live service)

SR = g.SR
OUT = Path.home() / "Desktop" / "mus-listening" / "bocetos-vst"

LAGE = dict(g.TELE, pos=0.20, pick=0.50, stiff=0.38, tension=20,
            symp=0.40, sus=7.5)

# ---- build the identical performance, recording section seams --------
marks = {}
g.a_section(final=False); marks["A2"] = g.clock[0]
g.a_section(final=False); marks["bridge"] = g.clock[0]
g.bridge(); marks["A3"] = g.clock[0]
g.a_section(final=True)
g.push(2.5, [49, 56, 61, 68, 73], gain=1.0, strum=44)
g.rest(0.3)
g.push(4.0, [37, 49, 56, 61, 65, 70, 75], gain=1.05, strum=60, sus=14)
g.clock[0] -= 3.0
g.push(3.0, [73, 85], gain=0.5, bell=True, strum=12)
total = g.clock[0] + 8.0


def render_dry(base_patch):
    mix = np.zeros(int(total * SR))
    for t, midis, slot, gain, over, melody_up in g.events:
        patch = dict(base_patch, **over)
        if len(midis) > 1 and melody_up > 1.0:
            chord = g.call("renderWeaveProbe", {
                "params": patch, "midis": midis[:-1],
                "slotSeconds": round(min(slot, 8.0), 3)})
            mel = g.call("renderWeaveProbe", {
                "params": dict(patch, strum=0, pick=min(0.62, patch["pick"] + 0.12)),
                "midis": [midis[-1]], "slotSeconds": round(min(slot, 8.0), 3)})
            n = max(len(chord), len(mel))
            audio = np.zeros(n)
            audio[:len(chord)] += chord * 0.85
            audio[:len(mel)] += mel * 0.85 * melody_up
        else:
            audio = g.call("renderWeaveProbe", {
                "params": patch, "midis": midis,
                "slotSeconds": round(min(slot, 8.0), 3)})
        start = int(max(t, 0.0) * SR)
        end = min(start + len(audio), len(mix))
        mix[start:end] += gain * audio[: end - start]
    return mix


def one_pole(x, f0):
    a = np.exp(-2 * np.pi * f0 / SR)
    y = np.zeros_like(x)
    s = 0.0
    for i, v in enumerate(x):
        s = (1 - a) * v + a * s
        y[i] = s
    return y


def presence(x, f0, q, gain_db):
    A = 10 ** (gain_db / 40)
    w = 2 * np.pi * f0 / SR
    alpha = np.sin(w) / (2 * q)
    cw = np.cos(w)
    b = [1 + alpha * A, -2 * cw, 1 - alpha * A]
    a = [1 + alpha / A, -2 * cw, 1 - alpha / A]
    y = np.zeros_like(x)
    x1 = x2 = y1 = y2 = 0.0
    for i, v in enumerate(x):
        out = (b[0] * v + b[1] * x1 + b[2] * x2 - a[1] * y1 - a[2] * y2) / a[0]
        x2, x1, y2, y1 = x1, v, y1, out
        y[i] = out
    return y


def lage_chain(dry):
    # two gentle asymmetric stages: the champ at the edge of bloom
    signal = dry
    for gain, bias in ((1.8, 0.05), (1.3, 0.03)):
        signal = np.tanh(gain * signal + bias) - np.tanh(bias)
    signal = one_pole(signal, 9500)
    signal = presence(signal, 4000, 1.1, 2.0)
    # spring: brighter, boingier IR — band-limited noise with pre-delay
    ir_rng = np.random.default_rng(2019)
    ir_n = int(1.2 * SR)
    ir = ir_rng.standard_normal(ir_n) * np.exp(-np.arange(ir_n) / (0.26 * SR))
    ir = ir - one_pole(ir, 420)          # thin the lows: spring, not plate
    ir = np.concatenate([np.zeros(int(0.012 * SR)), ir])
    wet = fftconvolve(signal, ir / np.abs(ir).sum() * 7.0)[: len(signal)]
    out = signal + 0.18 * wet
    return out / np.abs(out).max() * 0.84


def main():
    lage = lage_chain(render_dry(LAGE))
    stereo = np.stack([lage, np.roll(lage, int(0.0004 * SR))], axis=1)
    sf.write(OUT / "10_body_and_soul_lage.wav", stereo.astype(np.float32), SR,
             subtype="PCM_24")
    print(f"10_body_and_soul_lage.wav: {len(lage)/SR:.1f} s")

    ted, _ = sf.read(OUT / "09_body_and_soul_tele.wav")
    n = min(len(ted), len(lage))
    seams = [0.0, marks["A2"], marks["bridge"], marks["A3"], n / SR]
    rigs = [ted[:, 0], lage, ted[:, 0], lage]  # A1 A2 B A3+coda
    cambio = np.zeros(n)
    fade = int(0.06 * SR)
    for (t0, t1), source in zip(zip(seams, seams[1:]), rigs):
        s0, s1 = int(t0 * SR), min(int(t1 * SR), n)
        seg = source[s0:s1].copy()
        seg[:fade] *= np.linspace(0, 1, fade)
        seg[-fade:] *= np.linspace(1, 0, fade)
        cambio[s0:s1] += seg
    cambio /= np.abs(cambio).max() / 0.84
    stereo = np.stack([cambio, np.roll(cambio, int(0.0004 * SR))], axis=1)
    sf.write(OUT / "11_ted_vs_lage_cambio.wav", stereo.astype(np.float32), SR,
             subtype="PCM_24")
    print(f"11_ted_vs_lage_cambio.wav: seams at "
          f"{[round(s,1) for s in seams[1:4]]} s")
    g.service.stdin.close()


if __name__ == "__main__":
    main()
