#!/usr/bin/env python
"""P1 — anatomy causality under paired search (TWO-MACHINE-PLATEAU.md §6).

Does the fitted anatomy give the optimizer a systematic advantage? The
identical control/source optimizer runs over a counterfactual anatomy
set with paired seeds and a common neutral control initialization,
through the P0-certified renderer only.

Preregistered design (recorded in the receipt):
  anatomies    MAP, first posterior member, neutral center,
               length-matched neutral, unit-cube-shuffled MAP,
               random valid
  seeds        paired across all anatomies
  init         common neutral control start for every run;
               PLUS one MAP-warm column (anatomy-specific start),
               answering the pipeline question separately
  budget       fixed generations x population per run, identical
  level policy render RMS matched to target RMS before loss
               (P0.5 declared trim; removes the trivial global-level
               term the physical gain policy reintroduces)

Anatomy earns causal credit only if fitted/posterior beat the controls
across seeds. This run is one word, N=3 seeds: evidence, not a verdict
(P6 replication owns verdicts).

Run: .venv/bin/python mus-rs/tools/p1_anatomy_counterfactuals.py --receipt R
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np
import soundfile as sf

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "mus-rs" / "tools"))
sys.path.insert(0, str(ROOT))

from mus_analysis.garganta_anatomy import AnatomyGenome  # noqa: E402

from garganta_render import (  # noqa: E402
    ANCHOR_DIM,
    ANCHORS,
    GestureRenderer,
    SR,
    load_fitted,
    neutral_start,
    warm_anchors,
)
from speak_mimic_unified import HOP, WIN, logmel  # noqa: E402

TARGET = ROOT / "paper/ariadne-protected-nonabelian/garganta-humana/apple_target.wav"


def build_anatomies(fitted, rng):
    map_anatomy = fitted["anatomy"]
    unit = map_anatomy.to_unit_cube()
    shuffled_unit = unit[rng.permutation(10)]
    neutral = AnatomyGenome.from_unit_cube([0.5] * 10)
    length_matched_unit = np.full(10, 0.5)
    length_matched_unit[0] = unit[0]
    return {
        "map": map_anatomy,
        "posterior_1": AnatomyGenome(**fitted["posterior"][0]["anatomy"]),
        "neutral": neutral,
        "length_matched_neutral": AnatomyGenome.from_unit_cube(length_matched_unit),
        "shuffled": AnatomyGenome.from_unit_cube(shuffled_unit),
        "random_valid": AnatomyGenome.from_unit_cube(rng.uniform(0.0, 1.0, 10)),
    }


def evolve(renderer, target_mel, n_frames, total_s, target_rms,
           seed, generations, population, start):
    rng = np.random.default_rng(seed)
    dim = ANCHORS * ANCHOR_DIM
    mean = start.reshape(-1).copy()
    sigma = np.full(dim, 0.25)
    elite = max(4, population // 4)

    def score(candidate):
        audio = renderer.render(candidate, total_s, np.random.default_rng(0xF00D),
                                peak_normalize=False)
        rms = float(np.sqrt(np.mean(audio ** 2)))
        if rms > 1e-9:
            audio = audio * (target_rms / rms)
        return float(np.mean((logmel(audio, n_frames) - target_mel) ** 2))

    best = (score(mean), mean.copy())
    curve = [best[0]]
    for _ in range(generations):
        pop = mean + sigma * rng.standard_normal((population, dim))
        losses = [score(candidate) for candidate in pop]
        order = np.argsort(losses)
        if losses[order[0]] < best[0]:
            best = (losses[order[0]], pop[order[0]].copy())
        selected = pop[order[:elite]]
        mean = 0.7 * mean + 0.3 * selected.mean(axis=0)
        sigma = np.maximum(0.03, 0.55 * sigma + 0.45 * selected.std(axis=0))
        curve.append(best[0])
    return best[0], curve


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--generations", type=int, default=60)
    parser.add_argument("--population", type=int, default=16)
    parser.add_argument("--seeds", type=int, nargs="+", default=[101, 202, 303])
    parser.add_argument("--out", type=Path,
                        default=ROOT / "mus-rs" / "SPECS" / "garganta" / "p1_anatomy_receipt.json")
    args = parser.parse_args()

    fitted = load_fitted(args.receipt)
    anatomies = build_anatomies(fitted, np.random.default_rng(0x51))
    target, sr = sf.read(TARGET)
    assert sr == SR
    total_s = len(target) / SR
    n_frames = 1 + int((len(target) - WIN) / HOP)
    target_mel = logmel(target, n_frames)
    target_rms = float(np.sqrt(np.mean(target ** 2)))
    common_start = neutral_start()

    results: dict = {}
    for name, anatomy in anatomies.items():
        renderer = GestureRenderer(
            anatomy, fitted["section_count"],
            median_f0=fitted["median_f0"], source_tilt=fitted["tilt"],
            normalize="neutral_reference",
        )
        rows = []
        for seed in args.seeds:
            best, curve = evolve(renderer, target_mel, n_frames, total_s,
                                 target_rms, seed, args.generations,
                                 args.population, common_start)
            rows.append({"seed": seed, "best": round(best, 5),
                         "curve_every10": [round(v, 5) for v in curve[::10]]})
            print(f"{name} seed {seed}: best {best:.4f}", flush=True)
        results[name] = rows

    # the MAP-warm column: anatomy-specific start, pipeline question
    renderer = GestureRenderer(
        anatomies["map"], fitted["section_count"],
        median_f0=fitted["median_f0"], source_tilt=fitted["tilt"],
        normalize="neutral_reference",
    )
    warm_rows = []
    warm = warm_anchors(fitted["timeline"], total_s)
    for seed in args.seeds:
        best, curve = evolve(renderer, target_mel, n_frames, total_s,
                             target_rms, seed, args.generations,
                             args.population, warm)
        warm_rows.append({"seed": seed, "best": round(best, 5),
                          "curve_every10": [round(v, 5) for v in curve[::10]]})
        print(f"map+warm seed {seed}: best {best:.4f}", flush=True)
    results["map_warm_start"] = warm_rows

    medians = {name: round(float(np.median([r["best"] for r in rows])), 5)
               for name, rows in results.items()}
    receipt = {
        "battery": "P1 anatomy counterfactuals",
        "renderer": "P0-certified (see p0_renderer_receipt.json)",
        "design": {
            "generations": args.generations,
            "population": args.population,
            "paired_seeds": args.seeds,
            "init": "common neutral start; plus map_warm_start column",
            "level_policy": "render RMS matched to target RMS before loss",
            "note": "one word, three seeds: evidence, not a verdict (P6)",
        },
        "per_run": results,
        "median_best_by_anatomy": medians,
    }
    args.out.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(medians, indent=2))


if __name__ == "__main__":
    main()
