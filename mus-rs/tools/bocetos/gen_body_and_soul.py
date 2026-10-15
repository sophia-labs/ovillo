#!/usr/bin/env python
"""Body and Soul (Johnny Green, 1930 — US public domain 2026-01-01),
solo chord-melody in the Ted Greene manner, on the crystalline Tele.

The tone: neck-position steel (pos toward the soundhole side of a
solid body), soft nails, long sustain, heavy sympathetic ring — then a
single whisper-gain tube stage (glow, not grit), a gentle top tilt,
and 10% of a small plate. Crystal is mostly TOUCH: every chord rolled
18-40 ms, melody note voiced 1-2 dB over its chord, inner voices
moving between phrases, harp-harmonic bell (pos=0.5) for punctuation.

Transcribed from memory of the 1930 sheet; the A sections are canon,
one or two bridge voicings may be the arranger's. Rubato by hand.
"""

import json
import subprocess
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import fftconvolve

ROOT = Path(__file__).resolve().parents[3]
MUS = ROOT / "mus-rs/target/release/mus"
OUT = Path.home() / "Desktop" / "mus-listening" / "bocetos-vst"
SR = 48000
Q = 1.05  # quarter note at ~57 bpm

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
    return np.fromfile(r["result"]["pcm"]["path"], dtype=np.float32).astype(np.float64) \
        if "result" in r else None


TELE = {"synth": "pluck", "bodyp": "guitar", "stiff": 0.32, "sus": 9.0,
        "damp": 0.10, "pos": 0.30, "pick": 0.38, "tension": 14,
        "body": 0.30, "body_size": 1.0, "detune": 3, "symp": 0.55}

rng = np.random.default_rng(0x7ED)
events = []  # (time_s, midis, dur_s, gain, patch_overrides)
clock = [0.0]


def push(beats, midis, *, gain=1.0, strum=26, melody_up=1.6, sus=None,
         pos=None, bell=False, hold=None):
    """One chord-melody event; melody = highest note, voiced up."""
    t = clock[0]
    over = {"strum": strum}
    if sus is not None:
        over["sus"] = sus
    if pos is not None:
        over["pos"] = pos
    if bell:
        over["pos"] = 0.5
        over["pick"] = 0.2
        over["sus"] = 12
    dur = (hold if hold is not None else beats) * Q
    events.append((t + rng.normal(0, 0.010), midis, dur + 2.5, gain, over, melody_up))
    clock[0] += beats * Q


def rest(beats):
    clock[0] += beats * Q


# ---- the tune (Db). Voicings low->high, melody on top. ---------------
Db, Eb, F, Gb, Ab, Bb, C, B = 61, 63, 65, 66, 68, 70, 72, 71


def a_section(final):
    # bar 1  Ebm7 — "my heart is sad and lonely"
    push(1.5, [51, 58, 61, F], gain=0.95, strum=34)            # Ebm7, mel F4
    push(0.5, [Gb], melody_up=1.0, strum=0)
    push(1.0, [46, 56, 61, F], strum=22)                       # Bb7 push, mel F
    push(1.0, [Eb + 0], strum=0, melody_up=1.0)
    # bar 2  Bb7b9 — "...lonely"  mel Db held
    push(3.0, [46, 56, 60, 66, Db + 12 - 12], gain=0.9, strum=38)  # Bb7b9, mel Db4... top
    push(1.0, [Eb], strum=0, melody_up=1.0)
    # bar 3  Ebm7 Ab7 — "for you I sigh"
    push(1.0, [51, 58, 63, F], strum=24)
    push(1.0, [Gb], strum=0, melody_up=1.0)
    push(1.5, [44, 54, 60, Ab], strum=26)                      # Ab7, mel Ab4
    push(0.5, [Gb], strum=0, melody_up=1.0)
    # bar 4  Db6 Gb7 — "for you dear only"
    push(1.0, [49, 56, 61, F], strum=22)                       # Db6
    push(1.0, [Eb], strum=0, melody_up=1.0)
    push(2.0, [42, 52, 58, 64, Db], gain=0.92, strum=30)       # Gb7#11 color, mel Db
    # bar 5  Fm7 E°7 — "why haven't you seen it"
    push(1.0, [41, 51, 56, Ab], strum=24)                      # Fm7
    push(1.0, [C + 0], strum=0, melody_up=1.0)
    push(1.0, [40, 50, 56, C], strum=24)                       # E°7, mel C5
    push(1.0, [B], strum=0, melody_up=1.0)                     # Cb5
    # bar 6  Ebm7 Ab7 — "I'm all for you"
    push(1.0, [51, 58, 63, Gb], strum=22)
    push(1.0, [F], strum=0, melody_up=1.0)
    push(1.0, [44, 54, 60, Eb + 12 - 12], strum=24)
    push(0.5, [F], strum=0, melody_up=1.0)
    push(0.5, [Gb], strum=0, melody_up=1.0)
    # bars 7-8  Db — "body and soul"
    push(1.0, [49, 56, 61, F], strum=26)
    push(1.0, [Eb], strum=0, melody_up=1.0)
    if final:
        return
    push(2.0, [49, 53, 58, 61, Db], gain=0.95, strum=36)       # Db69
    push(1.0, [51, 58, 61, F - 12 + 12], gain=0.7, strum=20)   # turn: Ebm7
    push(1.0, [44, 54, 60, 66], gain=0.7, strum=20)            # Ab7
    rest(0.4)                                                   # breath


def bridge():
    E, Fs, G, A, D5 = 64, 66, 67, 69, 74
    # "I can't believe it" — D major light
    push(0.5, [Fs], strum=0, melody_up=1.0, gain=0.85)
    push(0.5, [G], strum=0, melody_up=1.0)
    push(1.0, [40, 50, 55, A], strum=24)                       # Em7, mel A4
    push(1.0, [45, 55, 61, B - 12 + 12], strum=22)             # A7, mel B4
    push(1.0, [A], strum=0, melody_up=1.0)
    push(2.0, [38, 50, 54, Fs + 12 - 12], gain=0.9, strum=30)  # D6, mel F#4
    push(1.0, [D5 - 12], strum=0, melody_up=1.0)
    push(1.0, [E], strum=0, melody_up=1.0)
    # "it's hard to conceive it"
    push(0.5, [Fs], strum=0, melody_up=1.0)
    push(0.5, [G], strum=0, melody_up=1.0)
    push(1.0, [40, 50, 55, A], strum=22)
    push(1.0, [45, 55, 61, 71], strum=22)
    push(1.0, [A], strum=0, melody_up=1.0)
    push(3.0, [38, 45, 50, 54, 62, Fs], gain=0.92, strum=40)   # D6/9 spread
    rest(0.5)
    # "my life a wreck you're making" — down through C
    push(0.5, [E], strum=0, melody_up=1.0, gain=0.85)
    push(0.5, [F], strum=0, melody_up=1.0)
    push(1.0, [38, 48, 53, G], strum=24)                       # Dm7, mel G4
    push(1.0, [43, 53, 59, A - 12 + 12], strum=22)             # G7, mel A4
    push(1.0, [G], strum=0, melody_up=1.0)
    push(2.0, [36, 48, 52, E + 12 - 12], gain=0.9, strum=28)   # C6, mel E4
    push(1.0, [G], strum=0, melody_up=1.0)
    push(1.0, [A], strum=0, melody_up=1.0)
    # the climb home: B7 -> Bb7 -> (Ebm7 Ab7)
    push(2.0, [47, 57, 63, 66, Ab + 2 - 2], gain=0.95, strum=30)  # B7, mel Ab4
    push(2.0, [46, 56, 62, 66, Ab], gain=0.95, strum=30)          # Bb7#5-ish
    push(1.0, [51, 58, 63, Gb], strum=22)                          # Ebm7
    push(0.5, [F], strum=0, melody_up=1.0)
    push(0.5, [Gb], strum=0, melody_up=1.0)
    push(2.0, [44, 54, 60, 66, Ab], gain=0.9, strum=26)            # Ab7
    rest(0.6)


def render():
    a_section(final=False)
    a_section(final=False)
    bridge()
    a_section(final=True)
    # the last words: "...body and soul", Ted's ending
    push(2.5, [49, 56, 61, 68, Db + 12], gain=1.0, strum=44)   # Db69, mel Db5
    rest(0.3)
    push(4.0, [37, 49, 56, 61, 65, 70, 75], gain=1.05, strum=60, sus=14)  # final Dbmaj13
    clock[0] -= 3.0
    push(3.0, [Db + 12, Db + 24], gain=0.5, bell=True, strum=12)  # harp bell 8ve

    total = clock[0] + 8.0
    mix = np.zeros(int(total * SR))
    for t, midis, slot, gain, over, melody_up in events:
        patch = dict(TELE, **over)
        audio_full = None
        # voice the melody: render chord and melody-top separately when up
        if len(midis) > 1 and melody_up > 1.0:
            chord = call("renderWeaveProbe", {
                "params": patch, "midis": midis[:-1],
                "slotSeconds": round(min(slot, 8.0), 3)})
            mel = call("renderWeaveProbe", {
                "params": dict(patch, strum=0, pick=min(0.55, patch["pick"] + 0.12)),
                "midis": [midis[-1]], "slotSeconds": round(min(slot, 8.0), 3)})
            n = max(len(chord), len(mel))
            audio_full = np.zeros(n)
            audio_full[:len(chord)] += chord * 0.85
            audio_full[:len(mel)] += mel * 0.85 * melody_up
        else:
            audio_full = call("renderWeaveProbe", {
                "params": patch, "midis": midis,
                "slotSeconds": round(min(slot, 8.0), 3)})
        start = int(max(t, 0.0) * SR)
        end = min(start + len(audio_full), len(mix))
        mix[start:end] += gain * audio_full[: end - start]

    # the whisper stage: glow, never grit
    glow = np.tanh(1.15 * mix) / np.tanh(1.15)
    a = np.exp(-2 * np.pi * 7500 / SR)
    state, tilted = 0.0, np.zeros_like(glow)
    for i, v in enumerate(glow):
        state = (1 - a) * v + a * state
        tilted[i] = state
    # 10% small plate
    ir_rng = np.random.default_rng(1930)
    ir_n = int(0.9 * SR)
    ir = ir_rng.standard_normal(ir_n) * np.exp(-np.arange(ir_n) / (0.19 * SR))
    wet = fftconvolve(tilted, ir / np.abs(ir).sum() * 7.0)[: len(tilted)]
    final = tilted + 0.10 * wet
    final /= np.abs(final).max() / 0.84
    stereo = np.stack([final, np.roll(final, int(0.0004 * SR))], axis=1)
    OUT.mkdir(parents=True, exist_ok=True)
    sf.write(OUT / "09_body_and_soul_tele.wav", stereo.astype(np.float32), SR,
             subtype="PCM_24")
    print(f"09_body_and_soul_tele.wav: {total:.1f} s, {len(events)} events")
    service.stdin.close()


if __name__ == "__main__":
    render()
