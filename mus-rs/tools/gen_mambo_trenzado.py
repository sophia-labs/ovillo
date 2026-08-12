#!/usr/bin/env python
"""Mambo del Telar (trenzado) — the greatest hit, re-braided for real.

Last night's mambo_hilo solo was their N2 sidecar: a Python Kato orbit
modulating gains on ordinary weave plucks. Tonight the braid is state:
ONE persistent sixteen-mode protected backbone lives across the entire
solo via `renderBraidProbe`, bridged into the Weave ring at the gesture
seam, executing the mathematicians' committed words (A, B, and words
composed from them) while it rings — dynamics emerge from radiation
physics, no gain law anywhere in the braid layer.

Also renders the band from the unchanged score and byte-compares it
against last night's committed band render — the U3 absence law heard
at corpus scale: no braid keys, no difference, to the byte.

Run: .venv/bin/python mus-rs/tools/gen_mambo_trenzado.py [--out DIR]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import fftconvolve

ROOT = Path(__file__).resolve().parents[2]
MUS = ROOT / "mus-rs/target/release/mus"
VECTORS = (
    ROOT
    / "paper/ariadne-protected-nonabelian/unified_artifacts/unified_rust_test_vectors.json"
)
OLD_BAND = ROOT / "paper/ariadne-protected-nonabelian/mambo-hilo/band.wav"
SR = 48000
BAR = 240.0 / 182.0
QL = 60.0 / 182.0

SOLO_PATCH = {
    "synth": "weave", "bodyp": "mandolin", "pick": 0.3, "pos": 0.5,
    "sus": 5.0, "damp": 0.14, "stiff": 0.3, "detune": 8, "courses": 9,
    "couple": 0.14, "body": 0.5,
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


def invert_schedule(schedule):
    """Exact inverse word: reversed order, inverted gates."""
    inverted = []
    for gate in reversed(schedule):
        if gate["kind"] == "phase_shift":
            inverted.append({"kind": "phase_shift", "i": gate["i"],
                             "phase": -gate["phase"]})
        else:
            (a, b), (c, d) = gate["block"]
            adjoint = [[[a[0], -a[1]], [c[0], -c[1]]],
                       [[b[0], -b[1]], [d[0], -d[1]]]]
            inverted.append({"kind": "two_mode_unitary", "i": gate["i"],
                             "j": gate["j"], "block": adjoint})
    return inverted


def build_words(vectors):
    a = vectors["words"]["A"]["schedule"]
    b = vectors["words"]["B"]["schedule"]
    return {
        "A": a,
        "B": b,
        "AB": a + b,
        "C": a + b + invert_schedule(a) + invert_schedule(b),
        "inverse": a + invert_schedule(a),
    }


def at(bar, ql):
    return (bar - 1) * BAR + ql * QL


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path,
                        default=ROOT / "paper" / "ariadne-protected-nonabelian" / "mambo-trenzado")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    # 1. the band, unchanged score, tonight's engine — the absence law
    receipt = json.loads(subprocess.run(
        [str(MUS), "render", str(ROOT / "aigua" / "mambo_hilo.mus"),
         "--base", str(ROOT / "aigua"), "-o", str(args.out / "band.wav"), "--json"],
        capture_output=True, text=True).stdout)
    assert not receipt["badTokens"], receipt["badTokens"]
    new_digest = hashlib.sha256((args.out / "band.wav").read_bytes()).hexdigest()
    old_digest = hashlib.sha256(OLD_BAND.read_bytes()).hexdigest()
    byte_identical = new_digest == old_digest
    band, sr = sf.read(args.out / "band.wav")
    assert sr == SR

    # 2. the solo — one persistent backbone across the whole piece
    vectors = json.loads(VECTORS.read_text())
    words = build_words(vectors)
    prepared = vectors["persistent"]["prepared"]

    plan = []  # (when, midis, kind, dur)
    answers = [([72], "A"), ([70], "B"), ([67], "A"), ([75], "B"),
               ([72], "AB"), ([67], "A"), ([70], "B")]
    for k, bar in enumerate(range(36, 49, 2)):
        midis, kind = answers[k]
        plan.append((at(bar, 2.5), midis, kind, 1.6))
    for k, bar in enumerate(range(53, 68, 2)):
        chord = [60, 67, 75] if k % 2 == 0 else [65, 72, 74]
        plan.append((at(bar, 1.5), chord, "AB" if k % 2 == 0 else "C", 2.2))
        plan.append((at(bar + 1, 3.0), [72 if k % 2 == 0 else 70], "A", 1.4))
    plan.append((at(69, 0.0), [60, 67, 72], "inverse", 3.5))
    plan.append((at(76, 0.0), [60, 67, 75, 84], "C", 5.0))
    plan.sort(key=lambda event: event[0])

    events = []
    for index, (when, midis, kind, dur) in enumerate(plan):
        # the overlap refusal is law: clamp each slot to its gap so the
        # backbone's schedule is unambiguous (the ring-out carries on
        # through the room port regardless)
        if index + 1 < len(plan):
            gap = plan[index + 1][0] - when
            dur = min(dur, max(0.3, gap - 0.02))
        events.append({
            "midis": midis,
            "atSeconds": round(when, 4),
            "slotSeconds": round(dur, 3),
            "word": words[kind],
            "wordSpanSeconds": round(min(dur * 0.7, 1.6), 3),
        })

    result = call("renderBraidProbe", {
        "params": SOLO_PATCH,
        "events": events,
        "couple": 0.045,
        "eta": 0.0035,
        "prepare": prepared,
    })
    ledger = result["backbone"]["ledger"]
    assert abs(ledger["closureError"]) < 1e-9, ledger
    solo = np.fromfile(result["pcm"]["path"], dtype=np.float32).astype(np.float64)

    # light room for the solo — same deterministic IR family as before
    rng = np.random.default_rng(1949)
    ir_n = int(1.1 * SR)
    ir = rng.standard_normal(ir_n) * np.exp(-np.arange(ir_n) / (0.22 * SR))
    wet = fftconvolve(solo, ir / np.abs(ir).sum() * 8.0)[: len(solo)]
    solo = solo + 0.22 * wet
    solo = solo[: len(band)]
    peak = np.abs(solo).max()
    if peak > 0:
        solo *= (np.abs(band).max() * 0.6) / peak

    mix = band.copy()
    mix[:, 0] += solo * 0.66
    mix[:, 1] += solo * 0.75
    final_peak = np.abs(mix).max()
    if final_peak > 0.98:
        mix *= 0.98 / final_peak
    sf.write(args.out / "mambo_trenzado.wav", mix.astype(np.float32), SR, subtype="PCM_24")

    report = {
        "band_byte_identical_to_last_night": byte_identical,
        "band_digest": new_digest[:16],
        "solo_events": len(events),
        "backbone_ledger": ledger,
        "note": "solo dynamics are radiation physics on one persistent "
                "backbone; the old version's bundle-gain law is gone",
    }
    (args.out / "receipt.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    service.stdin.close()


if __name__ == "__main__":
    main()
