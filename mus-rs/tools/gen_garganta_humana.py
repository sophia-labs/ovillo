#!/usr/bin/env python
"""La Garganta Humana — one throat, as human as rung 1 allows.

The opposite experiment from las vocales: not five mouths demonstrated,
one voice attempted. Every humanizing device available at GARGANTA
rung 1, spent on a single wordless vocalise:

  vibrato        orbit at 5.3 Hz with orbit_depth ramping 0 -> ~0.3
                 AFTER ~350 ms (delayed onset — the detail that reads
                 human); amplitude vibrato, honestly: FM vibrato waits
                 for the pitch-ratio ControlTarget.
  messa di voce  bow-pressure trajectories per note: swell to the
                 note's center, release toward air.
  phrasing       breath rises toward phrase ends; releases dissolve
                 tone into aspiration (bow down, breath up).
  breathing      audible inhales between phrases — breath=1 bow=0
                 notes shaped by the /a/ body, quiet, unpitched-ish.
  jitter         per-note bow_speed tremor, small gain drift, detune 5.
  the vowels     a journey per phrase (a-o-e-i-u), one vowel per note
                 (mid-note glides wait for rung 2).

All sound from the string network; assembly and room in python; no
foreign synthesis. Run:
  .venv/bin/python mus-rs/tools/gen_garganta_humana.py [--out DIR]
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import fftconvolve

ROOT = Path(__file__).resolve().parents[2]
MUS = ROOT / "mus-rs/target/release/mus"
SR = 48000

rng = np.random.default_rng(0x6A26A27A)
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


def sung_note(midi, dur, vowel, *, peak=0.55, vib=0.32, tail_air=0.35):
    """One sung note: swell, delayed vibrato, dissolve to air."""
    speed = float(np.clip(0.5 + rng.normal(0, 0.035), 0.35, 0.65))
    patch = {
        "synth": "weave", "bodyp": vowel, "bow": 0.16, "bow_speed": round(speed, 3),
        "breath": 0.12, "sus": 7, "damp": 0.2, "courses": 7, "couple": 0.13,
        "orbit": 5.3, "orbit_depth": 0.0, "curvature": 0.15, "detune": 5,
        "body": 1.0, "pick": 0.25,
    }
    mid = dur * 0.45
    release = dur * 0.82
    trajectory = [
        {"atSeconds": round(dur * 0.12, 3), "set": {"bow": round(peak * 0.75, 3)}},
        {"atSeconds": round(mid, 3), "set": {"bow": round(peak, 3)}},
        {"atSeconds": round(release, 3), "set": {"bow": 0.12, "breath": round(tail_air, 3)}},
    ]
    if dur > 0.9:
        trajectory.insert(1, {"atSeconds": 0.35, "set": {"orbit_depth": round(vib * 0.6, 3)}})
        trajectory.insert(2, {"atSeconds": round(min(0.7, mid), 3),
                              "set": {"orbit_depth": round(vib, 3)}})
    result = call("renderWeaveProbe", {
        "params": patch, "midis": [midi], "slotSeconds": round(dur + 0.9, 3),
        "trajectory": sorted(trajectory, key=lambda f: f["atSeconds"]),
    })
    return np.fromfile(result["pcm"]["path"], dtype=np.float32).astype(np.float64)


def inhale(dur=0.55):
    patch = {
        "synth": "weave", "bodyp": "a", "bow": 0.0, "breath": 1.0, "sus": 0.4,
        "damp": 0.5, "courses": 5, "couple": 0.0, "body": 0.6, "pick": 0.1,
    }
    result = call("renderWeaveProbe", {
        "params": patch, "midis": [69 + int(rng.integers(-2, 3))],
        "slotSeconds": dur, "trajectory": [],
    })
    return np.fromfile(result["pcm"]["path"], dtype=np.float32).astype(np.float64)


# the vocalise: (midi, seconds, vowel, peak-bow) — A aeolian, alto register
PHRASES = [
    [(57, 1.2, "a", 0.5), (60, 0.8, "o", 0.5), (64, 1.6, "a", 0.58),
     (62, 0.8, "e", 0.52), (60, 1.0, "o", 0.5), (59, 1.8, "u", 0.46)],
    [(60, 0.9, "a", 0.5), (64, 0.9, "e", 0.54), (67, 1.4, "i", 0.6),
     (65, 0.7, "e", 0.52), (64, 1.1, "a", 0.52), (62, 2.0, "o", 0.46)],
    [(64, 0.8, "a", 0.55), (67, 0.8, "e", 0.58), (69, 1.8, "i", 0.64),
     (67, 0.9, "e", 0.56), (65, 0.9, "o", 0.52), (64, 1.2, "a", 0.52),
     (67, 2.2, "i", 0.6)],
    [(65, 1.0, "e", 0.5), (64, 1.0, "o", 0.48), (62, 1.4, "u", 0.44),
     (60, 1.0, "o", 0.44), (59, 2.6, "u", 0.4), (57, 3.8, "a", 0.5)],
]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path,
                        default=ROOT / "paper" / "ariadne-protected-nonabelian" / "garganta-humana")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    timeline = []
    t = 0.8
    for pi, phrase in enumerate(PHRASES):
        if pi:
            breath_audio = inhale()
            timeline.append((t - 0.72, 0.16, breath_audio))
        for ni, (midi, dur, vowel, peak) in enumerate(phrase):
            last = ni == len(phrase) - 1
            audio = sung_note(midi, dur, vowel,
                              peak=peak,
                              vib=0.36 if last else 0.3,
                              tail_air=0.5 if last else 0.3)
            gain = 0.9 + 0.08 * np.sin(ni * 1.1) + float(rng.normal(0, 0.02))
            timeline.append((t, gain, audio))
            t += dur - 0.07  # legato: next onset just inside the release
        t += 1.15  # the breath between phrases

    total = t + 6.0
    mix = np.zeros(int(total * SR))
    for when, gain, audio in timeline:
        start = int(when * SR)
        end = min(start + len(audio), len(mix))
        mix[start:end] += gain * audio[: end - start]

    ir_rng = np.random.default_rng(1949)
    ir_n = int(1.6 * SR)
    ir = ir_rng.standard_normal(ir_n) * np.exp(-np.arange(ir_n) / (0.33 * SR))
    wet = fftconvolve(mix, ir / np.abs(ir).sum() * 9.0)[: len(mix)]
    mix = mix + 0.24 * wet
    peak = np.abs(mix).max()
    mix *= 0.85 / peak
    stereo = np.stack([mix, np.roll(mix, int(0.0004 * SR))], axis=1)
    sf.write(args.out / "la_garganta_humana.wav", stereo.astype(np.float32), SR,
             subtype="PCM_24")
    print(f"{args.out / 'la_garganta_humana.wav'}  {total:.1f}s  notes={sum(len(p) for p in PHRASES)}")
    service.stdin.close()


if __name__ == "__main__":
    main()
