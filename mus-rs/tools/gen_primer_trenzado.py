#!/usr/bin/env python
"""El Primer Trenzado — the first braid INSIDE the instrument (U3).

Until today every braid piece was a sidecar (their N2): the Kato orbit
ran in Python and modulated gains on ordinary plucks. This renders the
same experiment through `renderBraidProbe`: one persistent sixteen-mode
protected backbone lives across the events, bridged into the Weave
ring's own wave variables at the gesture seam, radiating its room port
into the timeline — ringing between the notes.

Two renders: pluck·A then pluck·B, versus pluck·B then pluck·A. Same
committed gate words (the mathematicians' A and B, 136 gates each),
same calibrated preparation, same energies. The only difference is the
order — and the difference in the audio is holonomy radiated through a
room port, no gain law anywhere.

Run: .venv/bin/python mus-rs/tools/gen_primer_trenzado.py [--out DIR]
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

import numpy as np
import soundfile as sf

ROOT = Path(__file__).resolve().parents[2]
MUS = ROOT / "mus-rs/target/release/mus"
VECTORS = (
    ROOT
    / "paper/ariadne-protected-nonabelian/unified_artifacts/unified_rust_test_vectors.json"
)
SR = 48000

PATCH = {
    "synth": "weave", "bodyp": "mandolin", "pick": 0.28, "pos": 0.5,
    "sus": 6.0, "damp": 0.12, "stiff": 0.25, "detune": 6, "courses": 7,
    "couple": 0.13, "body": 0.55,
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


def braid(order, words, prepared):
    events = []
    chords = [[57, 64, 69], [59, 62, 67]]
    for index, name in enumerate(order):
        events.append({
            "midis": chords[index],
            "atSeconds": 0.4 + index * 3.2,
            "slotSeconds": 2.6,
            "word": words[name],
            "wordSpanSeconds": 1.8,
        })
    result = call("renderBraidProbe", {
        "params": PATCH,
        "events": events,
        "couple": 0.05,
        "eta": 0.004,
        "prepare": prepared,
    })
    audio = np.fromfile(result["pcm"]["path"], dtype=np.float32).astype(np.float64)
    ledger = result["backbone"]["ledger"]
    assert abs(ledger["closureError"]) < 1e-9, ledger
    final = np.array([complex(re, im) for re, im in result["backbone"]["finalState"]])
    return audio, final, ledger


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path,
                        default=Path.home() / "Desktop" / "mus-listening" / "primer_trenzado")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    vectors = json.loads(VECTORS.read_text())
    words = {name: vectors["words"][name]["schedule"] for name in ("A", "B")}
    prepared = vectors["persistent"]["prepared"]

    ab_audio, ab_state, ab_ledger = braid(["A", "B"], words, prepared)
    ba_audio, ba_state, ba_ledger = braid(["B", "A"], words, prepared)

    state_distance = float(np.linalg.norm(ab_state - ba_state))
    length = min(len(ab_audio), len(ba_audio))
    ab_cut, ba_cut = ab_audio[:length], ba_audio[:length]
    denominator = float(np.linalg.norm(ab_cut))
    audio_distance = float(np.linalg.norm(ab_cut - ba_cut)) / max(denominator, 1e-12)

    peak = max(np.abs(ab_audio).max(), np.abs(ba_audio).max())
    gain = 0.85 / peak
    for name, audio in (("primer_trenzado_ab", ab_audio), ("primer_trenzado_ba", ba_audio)):
        stereo = np.stack([audio, np.roll(audio, int(0.0003 * SR))], axis=1) * gain
        sf.write(args.out / f"{name}.wav", stereo.astype(np.float32), SR, subtype="PCM_24")

    gap = np.zeros(int(0.8 * SR))
    contrast = np.concatenate([ab_audio, gap, ba_audio]) * gain
    stereo = np.stack([contrast, np.roll(contrast, int(0.0003 * SR))], axis=1)
    sf.write(args.out / "primer_trenzado_contraste.wav", stereo.astype(np.float32), SR,
             subtype="PCM_24")

    report = {
        "ab_ledger": ab_ledger,
        "ba_ledger": ba_ledger,
        "final_state_distance_ab_vs_ba": state_distance,
        "relative_audio_distance": audio_distance,
        "shared_normalization_gain": gain,
        "note": "one shared gain across all files; the AB/BA difference is physical",
    }
    (args.out / "receipt.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    print(f"delivered: {args.out}")
    service.stdin.close()


if __name__ == "__main__":
    main()
