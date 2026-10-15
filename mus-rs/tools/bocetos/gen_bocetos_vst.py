#!/usr/bin/env python
"""Bocetos VST — audio sketches of the preset ideas, before formalizing.

Six sounds for Vera, current engine only, honestly labeled sketches:

  01 costa clara      clean-electric TERRITORY sketch (the real Jazzmaster
                      graph is design-only; this is steel-ish pluck physics
                      demoing touch response: same lick, three touches)
  02 telarana plate   dry phrase -> string-lattice plate (dense cluster,
                      dimension high, couple high) -> dry/wet/mix
  03 telarana spring  the dispersive 3-string tank (stiff at the ceiling)
  04 sympathique      lattice tuned to the phrase's key; sympathetic bloom
  05 shimmer          the tail that turns (orbit + curvature + chirality
                      flip per re-fret; braid-word version awaits U3 score
                      integration — this is shimmer-lite, said plainly)
  06 campana imposible  midpoint bells with tension going sharp — one
                      Imposibles-shelf teaser

Every reverb file has the same structure: dry alone · wet alone · mixed.
Score reverb is set to 0 — the tails you hear are strings, nothing else.
"""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

import numpy as np
import soundfile as sf

ROOT = Path(__file__).resolve().parents[3]
MUS = ROOT / "mus-rs/target/release/mus"
BASE = Path(__file__).resolve().parent
OUT = Path.home() / "Desktop" / "mus-listening" / "bocetos-vst"
SR = 48000

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


def probe(patch, midis, slot):
    result = call("renderWeaveProbe", {
        "params": patch, "midis": midis, "slotSeconds": round(slot, 3),
    })
    return np.fromfile(result["pcm"]["path"], dtype=np.float32).astype(np.float64)


STEEL = {
    "synth": "pluck", "bodyp": "guitar", "stiff": 0.45, "tension": 30,
    "sus": 7.0, "damp": 0.14, "body": 0.30, "body_size": 0.95,
    "detune": 4, "symp": 0.5, "pos": 0.16, "pick": 0.55,
}


def place(timeline, when, audio, gain=1.0):
    start = int(when * SR)
    end = min(start + len(audio), len(timeline))
    timeline[start:end] += gain * audio[: end - start]


def costa_clara():
    """Same lick three times: caress, speak, dig. Touch is the preset."""
    lick = [(52, 0.0, 0.45), (59, 0.45, 0.4), (64, 0.85, 0.4), (68, 1.25, 0.5),
            (66, 1.8, 0.45), (64, 2.25, 0.7)]
    chord = [40, 47, 52, 56, 59, 64]
    timeline = np.zeros(int(26 * SR))
    touches = [
        (0.5, 0.28, 0.32, 0.30, "caress"),
        (8.5, 0.55, 0.16, 0.55, "speak"),
        (16.5, 0.86, 0.11, 1.0, "dig"),
    ]
    for base_t, pick, pos, gain, _name in touches:
        for midi, offset, dur in lick:
            patch = dict(STEEL, pick=pick, pos=pos,
                         tension=10 + 55 * gain)
            place(timeline, base_t + offset, probe(patch, [midi], dur + 2.2), gain)
        patch = dict(STEEL, pick=pick * 0.9, pos=pos + 0.06, sus=9.0)
        place(timeline, base_t + 3.1, probe(patch, chord, 4.0), gain * 0.9)
    peak = np.abs(timeline).max()
    return timeline / peak * 0.85


def dry_phrase():
    """Four notes and a chord in E — the reverb test signal."""
    timeline = np.zeros(int(6 * SR))
    for midi, when, dur, gain in [(52, 0.0, 0.5, 0.8), (59, 0.5, 0.45, 0.7),
                                  (64, 0.95, 0.5, 0.85), (63, 1.5, 0.8, 0.75)]:
        place(timeline, when, probe(dict(STEEL, pick=0.6, pos=0.14), [midi], dur + 1.8), gain)
    place(timeline, 2.6, probe(dict(STEEL, pick=0.5, pos=0.2, sus=6), [52, 59, 64, 68], 3.2), 0.85)
    peak = np.abs(timeline).max()
    return timeline / peak * 0.8


REVERB_SCORES = {
    "plate": {
        "decl": "synth=weave, sample=dry.wav, sing=1, courses=24, couple=0.42, "
                "dimension=3.0, sus=9, damp=0.08, detune=14, orbit=0.12, "
                "orbit_depth=0.3, pos=0.5, pick=0.3, body=0.4, strum=0",
        "bars": ["<E2 A2 D3 G3 C4 F4 A#4 D#5>w[sing_at=0]",
                 "<E2 A2 D3 G3 C4 F4 A#4 D#5>w[sing_at=4]",
                 "<D2 G2 C3 F3 A#3 D#4 G#4 C#5>w[sing_at=8]",
                 "<E2 A2 D3 G3 C4 F4 A#4 D#5>w[sing_at=12]"],
    },
    "spring": {
        "decl": "synth=weave, sample=dry.wav, sing=1, courses=3, couple=0.2, "
                "stiff=0.9, sus=8, damp=0.10, detune=2, curvature=0.6, "
                "chirality=-0.7, pos=0.35, pick=0.3, body=0.5, strum=0",
        "bars": ["<E2 B2 E3>w[sing_at=0]", "<E2 B2 E3>w[sing_at=4]",
                 "<E2 B2 E3>w[sing_at=8]", "<E2 B2 E3>w[sing_at=12]"],
    },
    "sympathique": {
        "decl": "synth=weave, sample=dry.wav, sing=1, courses=8, couple=0.16, "
                "sus=12, damp=0.08, detune=3, orbit=0.08, pos=0.3, pick=0.35, "
                "body=0.55, strum=0",
        "bars": ["<E2 B2 E3 G#3 B3 E4 G#4 B4>w[sing_at=0]",
                 "<E2 B2 E3 G#3 B3 E4 G#4 B4>w[sing_at=4]",
                 "<E2 B2 E3 G#3 B3 E4 G#4 B4>w[sing_at=8]",
                 "<E2 B2 E3 G#3 B3 E4 G#4 B4>w[sing_at=12]"],
    },
    "shimmer": {
        "decl": "synth=weave, sample=dry.wav, sing=1, courses=15, couple=0.34, "
                "dimension=2.6, sus=10, damp=0.09, detune=9, orbit=2.6, "
                "orbit_depth=0.9, curvature=0.5, chirality=0.8, pos=0.5, "
                "pick=0.3, body=0.4, strum=0",
        "bars": ["<E3 B3 E4 G#4 B4 E5>w[sing_at=0,chirality=0.8]",
                 "<E3 B3 E4 G#4 B4 E5>w[sing_at=4,chirality=-0.8]",
                 "<F#3 C#4 F#4 A#4 C#5 F#5>w[sing_at=8,chirality=0.8]",
                 "<G#3 D#4 G#4 B4 D#5 G#5>w[sing_at=12,chirality=-0.8]"],
    },
}


def render_reverb(name, spec, dry):
    score = BASE / f"telarana_{name}.mus"
    lines = [
        f"# score: Telarana {name}",
        "# summary: string-reverb sketch — the tail is a lattice",
        "# tempo: 60", "# time: 4/4", "# key: Em", "# bars: 4",
        "# reverb: 0", "# master: rms=-18", "",
        "# instruments:",
        f"#   rv = telarana ({'treble, ' + spec['decl']})",
        "",
    ]
    for index, token in enumerate(spec["bars"], 1):
        lines.append(f"bar {index}: rv={token}")
    score.write_text("\n".join(lines) + "\n")
    wav = BASE / f"wet_{name}.wav"
    receipt = json.loads(subprocess.run(
        [str(MUS), "render", str(score), "--base", str(BASE), "-o", str(wav), "--json"],
        capture_output=True, text=True).stdout)
    assert not receipt["badTokens"], (name, receipt["badTokens"])
    wet, sr = sf.read(wav)
    assert sr == SR
    if wet.ndim > 1:
        wet = wet.mean(axis=1)
    # dry alone / wet alone / mix, one shared normalization
    gap = np.zeros(int(0.7 * SR))
    wet_gain = 0.9 * np.abs(dry).max() / max(np.abs(wet).max(), 1e-9)
    mix = np.zeros(max(len(dry), len(wet)))
    mix[:len(dry)] += dry
    mix[:len(wet)] += 0.55 * wet_gain * wet
    montage = np.concatenate([dry, gap, wet * wet_gain, gap, mix])
    peak = np.abs(montage).max()
    return montage / peak * 0.85


def campana_imposible():
    timeline = np.zeros(int(16 * SR))
    bell = {
        "synth": "weave", "bodyp": "mandolin", "pos": 0.5, "stiff": 0.6,
        "sus": 8, "damp": 0.10, "courses": 7, "couple": 0.12, "detune": 6,
        "pick": 0.8, "body": 0.45, "tension": 48,
    }
    strikes = [([64, 71], 0.5, 0.7), ([69, 76], 3.2, 0.8), ([73, 80], 5.9, 0.9)]
    for midis, when, gain in strikes:
        place(timeline, when, probe(bell, midis, 3.2), gain)
    final = dict(bell, orbit=1.8, orbit_depth=0.85, curvature=0.4, chirality=-0.8,
                 sus=12, tension=56)
    place(timeline, 8.8, probe(final, [64, 71, 76, 83], 6.5), 1.0)
    peak = np.abs(timeline).max()
    return timeline / peak * 0.85


def deliver(name, audio):
    stereo = np.stack([audio, np.roll(audio, int(0.00035 * SR))], axis=1)
    sf.write(OUT / name, stereo.astype(np.float32), SR, subtype="PCM_24")
    print(f"{name}: {len(audio)/SR:.1f} s")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    deliver("01_costa_clara.wav", costa_clara())
    dry = dry_phrase()
    sf.write(BASE / "dry.wav", dry.astype(np.float32), SR, subtype="FLOAT")
    for name, spec in REVERB_SCORES.items():
        deliver(f"0{list(REVERB_SCORES).index(name)+2}_telarana_{name}.wav",
                render_reverb(name, spec, dry))
    deliver("06_campana_imposible.wav", campana_imposible())
    service.stdin.close()
    print(f"delivered: {OUT}")


if __name__ == "__main__":
    main()
