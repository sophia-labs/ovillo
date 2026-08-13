#!/usr/bin/env python
"""El Riff del Telar — metal tone + riff sketch.

Dry side: real string physics — palm mutes are short-sus high-damp
bridge-position plucks (not gating), stabs ring, the final dive is the
probe's gliss engine playing whammy. E phrygian with the tritone on
duty. Double-tracked: two performances with jittered pick/detune/
timing, hard-panned, like humans do.

Wet side: the diagnosis recipe pushed to metal — tight pre-highpass
BEFORE the gain (chug clarity), four cascaded asymmetric stages with a
sagging reservoir, then a 4x12-ish voicing: low thump, 480 Hz scoop,
presence at 3.2k, steep top. All numpy sketch, honestly labeled.

Output: 08_riff_del_telar.wav (3 s of naked chugs first, then the wall).
"""

import json
import subprocess
from pathlib import Path

import numpy as np
import soundfile as sf

ROOT = Path("/Users/vera/dev/sophia/mus")
MUS = ROOT / "mus-rs/target/release/mus"
OUT = Path.home() / "Desktop" / "mus-listening" / "bocetos-vst"
SR = 48000
SIXT = 60.0 / 150.0 / 4.0  # sixteenth at 150 bpm

rpc_id = 0
service = subprocess.Popen(
    [str(MUS), "service"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
)


def call(method, params):
    global rpc_id
    rpc_id += 1
    service.stdin.write(json.dumps({"id": rpc_id, "method": method, "params": params}) + "\n")
    service.stdin.flush()
    r = json.loads(service.stdout.readline())
    if "error" in r:
        raise RuntimeError(r["error"])
    return r["result"]


def probe(patch, midis, slot, gliss=1.0):
    result = call("renderWeaveProbe", {
        "params": patch, "midis": midis, "slotSeconds": round(slot, 3),
        "glissRatio": gliss,
    })
    return np.fromfile(result["pcm"]["path"], dtype=np.float32).astype(np.float64)


MUTE = {"synth": "pluck", "bodyp": "guitar", "stiff": 0.5, "sus": 0.35,
        "damp": 0.62, "pos": 0.09, "pick": 0.85, "tension": 45,
        "body": 0.18, "body_size": 0.9, "detune": 5, "symp": 0.0}
STAB = dict(MUTE, sus=2.6, damp=0.22, pos=0.12, tension=55, body=0.25, symp=0.3)
LEAD = dict(STAB, sus=1.6, pos=0.14, pick=0.8)

E5 = [40, 47, 52]      # E2 B2 E3
F5 = [41, 48, 53]      # F power — the phrygian punch
BB = [46, 53]          # the tritone, on duty
CHUG = [40]


def riff_events():
    """(sixteenth_index, midis, patch, dur_sixteenths, gain, gliss)"""
    ev = []

    def bar_chugs(base):
        # gallop: 8th + two 16ths, twice
        for beat in (0, 4):
            ev.append((base + beat, CHUG, MUTE, 2, 1.0, 1.0))
            ev.append((base + beat + 2, CHUG, MUTE, 1, 0.82, 1.0))
            ev.append((base + beat + 3, CHUG, MUTE, 1, 0.88, 1.0))

    # bar 1: gallops, E stab, F punch
    bar_chugs(0)
    ev.append((8, E5, STAB, 4, 1.0, 1.0))
    ev.append((12, F5, STAB, 2, 1.05, 1.0))
    ev.append((14, CHUG, MUTE, 1, 0.8, 1.0))
    ev.append((15, CHUG, MUTE, 1, 0.85, 1.0))
    # bar 2: gallops, tritone, chromatic tail
    bar_chugs(16)
    ev.append((24, BB, STAB, 4, 1.05, 1.0))
    for i, m in enumerate([43, 42, 41]):  # G F# F
        ev.append((28 + i, [m], LEAD, 1, 0.9, 1.0))
    ev.append((31, CHUG, MUTE, 1, 0.85, 1.0))
    # bar 3 = bar 1
    bar_chugs(32)
    ev.append((40, E5, STAB, 4, 1.0, 1.0))
    ev.append((44, F5, STAB, 2, 1.05, 1.0))
    ev.append((46, CHUG, MUTE, 1, 0.8, 1.0))
    ev.append((47, CHUG, MUTE, 1, 0.85, 1.0))
    # bar 4: run and dive
    for i, m in enumerate([52, 55, 54, 53]):  # E G F# F
        ev.append((48 + i, [m], LEAD, 1, 0.95, 1.0))
    ev.append((52, E5, STAB, 6, 1.1, 1.0))
    ev.append((58, [40, 47, 52, 55], dict(STAB, sus=4.0), 10, 1.15, 0.42))  # DIVE
    return ev


def render_take(rng):
    ev = riff_events()
    total = int((70 * SIXT + 4.0) * SR)
    take = np.zeros(total)
    for idx, midis, patch, dur, gain, gliss in ev:
        p = dict(patch)
        p["pick"] = float(np.clip(p["pick"] + rng.normal(0, 0.04), 0.1, 0.98))
        p["detune"] = patch.get("detune", 5) + int(rng.integers(0, 4))
        when = idx * SIXT + float(rng.normal(0, 0.006))
        slot = max(dur * SIXT + 1.2, 0.5)
        audio = probe(p, midis, slot, gliss)
        start = int(max(when, 0) * SR)
        end = min(start + len(audio), total)
        take[start:end] += gain * audio[: end - start]
    return take


def biquad(x, f0, q, kind="peak", gain_db=0.0):
    A = 10 ** (gain_db / 40)
    w = 2 * np.pi * f0 / SR
    alpha = np.sin(w) / (2 * q)
    cw = np.cos(w)
    if kind == "peak":
        b = [1 + alpha * A, -2 * cw, 1 - alpha * A]
        a = [1 + alpha / A, -2 * cw, 1 - alpha / A]
    elif kind == "lowshelf":
        s = np.sqrt(A)
        b = [A * ((A + 1) - (A - 1) * cw + 2 * s * alpha),
             2 * A * ((A - 1) - (A + 1) * cw),
             A * ((A + 1) - (A - 1) * cw - 2 * s * alpha)]
        a = [(A + 1) + (A - 1) * cw + 2 * s * alpha,
             -2 * ((A - 1) + (A + 1) * cw),
             (A + 1) + (A - 1) * cw - 2 * s * alpha]
    elif kind == "highpass":
        b = [(1 + cw) / 2, -(1 + cw), (1 + cw) / 2]
        a = [1 + alpha, -2 * cw, 1 - alpha]
    else:
        raise ValueError(kind)
    y = np.zeros_like(x)
    x1 = x2 = y1 = y2 = 0.0
    for i, v in enumerate(x):
        out = (b[0] * v + b[1] * x1 + b[2] * x2 - a[1] * y1 - a[2] * y2) / a[0]
        x2, x1, y2, y1 = x1, v, y1, out
        y[i] = out
    return y


def metal_chain(x):
    # tight: cut the flub BEFORE the gain
    signal = biquad(x, 240, 0.71, "highpass")
    signal = biquad(signal, 900, 0.8, "peak", 3.0)  # the screamer hump
    reservoir, envelope = 1.0, 0.0
    attack = 1 - np.exp(-1 / (0.003 * SR))
    recover = 1 - np.exp(-1 / (0.09 * SR))
    for stage, gain in enumerate([9.0, 6.5, 4.5, 3.0]):
        out = np.zeros_like(signal)
        for i, v in enumerate(signal):
            envelope += (abs(v) - envelope) * (attack if abs(v) > envelope else recover)
            reservoir = max(0.4, min(1.0, reservoir + (1 - reservoir) * recover
                                     - 0.10 * envelope * attack))
            bias = 0.26 * (1.0 - reservoir)
            u = gain * (0.6 + 0.4 * reservoir) * v + bias
            out[i] = np.tanh(1.4 * u) - np.tanh(bias)
        # inter-stage tilt keeps it tight
        a = np.exp(-2 * np.pi * (5200 - 700 * stage) / SR)
        state, tilted = 0.0, np.zeros_like(out)
        for i, v in enumerate(out):
            state = (1 - a) * v + a * state
            tilted[i] = state
        signal = tilted
    # the 4x12: thump, scoop, presence, steep top
    signal = biquad(signal, 110, 0.8, "lowshelf", 4.5)
    signal = biquad(signal, 480, 1.1, "peak", -7.0)
    signal = biquad(signal, 3200, 1.3, "peak", 5.0)
    for f in (6200, 6200):
        a = np.exp(-2 * np.pi * f / SR)
        state, low = 0.0, np.zeros_like(signal)
        for i, v in enumerate(signal):
            state = (1 - a) * v + a * state
            low[i] = state
        signal = low
    return signal


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    left = render_take(np.random.default_rng(0xE1))
    right = render_take(np.random.default_rng(0xE2))
    wet_l = metal_chain(left)
    wet_r = metal_chain(right)

    # 3 seconds of naked chugs first — the physics before the wall
    naked = left[: int(3.2 * SR)].copy()
    naked = naked / max(np.abs(naked).max(), 1e-9) * 0.7

    for ch in (wet_l, wet_r):
        ch /= max(np.abs(ch).max(), 1e-9)
    stereo_wall = np.stack([wet_l * 0.88 + wet_r * 0.12,
                            wet_r * 0.88 + wet_l * 0.12], axis=1) * 0.85
    gap = np.zeros((int(0.5 * SR), 2))
    naked_st = np.stack([naked, naked], axis=1)
    full = np.concatenate([naked_st, gap, stereo_wall])
    peak = np.abs(full).max()
    full = full / peak * 0.89
    sf.write(OUT / "08_riff_del_telar.wav", full.astype(np.float32), SR, subtype="PCM_24")
    print(f"08_riff_del_telar.wav: {len(full)/SR:.1f} s")
    service.stdin.close()


if __name__ == "__main__":
    main()
