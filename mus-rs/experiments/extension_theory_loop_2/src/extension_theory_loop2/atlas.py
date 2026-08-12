from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import numpy as np
import scipy.stats
import torch

from .core import (
    ProcessSpec,
    ObservableHMM,
    TrainConfig,
    build_model,
    evaluate_model,
    filter_lyapunov_spectrum,
    spectrum,
    teacher_hankel,
    train_predictive_model,
)
from .metrics import analyze_predictive_state, spectral_overlap


PROCESS_SPECS: dict[str, ProcessSpec] = {
    "p4_fast_clear": ProcessSpec(
        "p4_fast_clear", states=4, persistence=0.80, emission_concentration=0.45, anchor=3.0, seed=11
    ),
    "p4_slow_clear": ProcessSpec(
        "p4_slow_clear", states=4, persistence=0.975, emission_concentration=0.45, anchor=3.0, seed=11
    ),
    "p6_baseline": ProcessSpec(
        "p6_baseline", states=6, persistence=0.94, emission_concentration=0.65, anchor=2.0, seed=41
    ),
    "p6_noisy": ProcessSpec(
        "p6_noisy", states=6, persistence=0.94, emission_concentration=2.5, anchor=0.55, seed=41
    ),
    "p6_cycle": ProcessSpec(
        "p6_cycle", states=6, persistence=0.82, emission_concentration=0.65, anchor=2.0, transition_kind="cycle", seed=19
    ),
    "p6_reversible": ProcessSpec(
        "p6_reversible", states=6, persistence=0.94, emission_concentration=0.65, anchor=2.0, transition_kind="reversible", seed=23
    ),
    "p8_baseline": ProcessSpec(
        "p8_baseline", states=8, persistence=0.94, emission_concentration=0.65, anchor=2.0, seed=31
    ),
    "p8_slow_weak": ProcessSpec(
        "p8_slow_weak", states=8, persistence=0.985, emission_concentration=1.5, anchor=0.9, seed=31
    ),
}

MODEL_SPECS: dict[str, tuple[str, int]] = {
    "clifford4": ("clifford", 4),
    "clifford8": ("clifford", 8),
    "projective4": ("projective", 4),
    "projective8": ("projective", 8),
    "linear8": ("linear", 8),
    "gru4": ("gru", 4),
    "gru8": ("gru", 8),
}


def process_payload(name: str) -> dict[str, Any]:
    process = ObservableHMM(PROCESS_SPECS[name])
    hankel = spectrum(teacher_hankel(process, 3))
    lyapunov = filter_lyapunov_spectrum(process, steps=8_000, burn_in=800, seed=913)
    return {"process": process.diagnostics(), "hankel": hankel, "filter": lyapunov}


def run_one(
    process_name: str,
    model_name: str,
    output: Path,
    *,
    seed: int = 7,
    steps: int = 200,
) -> dict[str, Any]:
    torch.set_num_threads(1)
    try:
        torch.set_num_interop_threads(1)
    except RuntimeError:
        pass
    torch.manual_seed(seed)
    process = ObservableHMM(PROCESS_SPECS[process_name])
    kind, state_size = MODEL_SPECS[model_name]
    model = build_model(kind, process, state_size)
    training = train_predictive_model(
        model,
        process,
        TrainConfig(steps=steps, batch=128, length=24, learning_rate=0.006, seed=seed),
    )
    evaluation = evaluate_model(
        model, process, seed=seed + 70_000, batches=18, batch=256, length=56
    )
    predictive = analyze_predictive_state(
        model,
        process,
        seed=seed + 80_000,
        train_samples=5000,
        test_samples=2500,
        history_length=28,
        future_horizon=5,
    )
    data_spectrum = predictive["future_prediction"]["future_mode_singular_values"]
    balanced = predictive["balanced"]["singular_values"]
    predictive["balanced_future_spectral_overlap"] = spectral_overlap(data_spectrum, balanced)
    diagnostics = model.diagnostics() if hasattr(model, "diagnostics") else {}
    payload = {
        "process_name": process_name,
        "model_name": model_name,
        "model_kind": kind,
        "declared_state_size": state_size,
        "actual_state_size": int(model.state_size),
        "seed": seed,
        "steps": steps,
        "parameters": sum(parameter.numel() for parameter in model.parameters()),
        "training": training["history"],
        "evaluation": evaluation,
        "predictive_state": predictive,
        "diagnostics": diagnostics,
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    torch.save({"state_dict": model.state_dict(), "payload": payload}, output.with_suffix(".pt"))
    print(json.dumps({"DONE": payload}, sort_keys=True), flush=True)
    return payload


def _metric_value(row: dict[str, Any], metric: str) -> float:
    pred = row["predictive_state"]
    table = {
        "state_size": float(row["actual_state_size"]),
        "state_participation_rank": float(pred["state_participation_rank"]),
        "balanced_participation_rank": float(pred["balanced"]["participation_rank"]),
        "belief_r2": float(pred["belief_probe"]["r2"]),
        "future_r2": float(pred["future_prediction"]["weighted_future_r2"]),
        "future_modes": float(pred["future_prediction"]["future_modes_r2_above_0_5"]),
        "spectral_overlap": float(pred["balanced_future_spectral_overlap"]),
    }
    return table[metric]


def _within_process_concordance(rows: list[dict[str, Any]], metric: str) -> float:
    by_process: dict[str, list[dict[str, Any]]] = {}
    for row in rows:
        by_process.setdefault(row["process_name"], []).append(row)
    correct = total = 0
    for group in by_process.values():
        for i in range(len(group)):
            for j in range(i + 1, len(group)):
                mi = _metric_value(group[i], metric)
                mj = _metric_value(group[j], metric)
                li = group[i]["evaluation"]["excess_bps"]
                lj = group[j]["evaluation"]["excess_bps"]
                if abs(mi - mj) < 1e-10 or abs(li - lj) < 1e-10:
                    continue
                correct += int((mi > mj) == (li < lj))
                total += 1
    return correct / total if total else float("nan")


def aggregate(directory: Path, output: Path) -> dict[str, Any]:
    rows = [json.loads(path.read_text()) for path in sorted(directory.glob("*.json"))]
    process_rows: dict[str, Any] = {}
    for name in PROCESS_SPECS:
        process_rows[name] = process_payload(name)

    metrics = [
        "state_size",
        "state_participation_rank",
        "balanced_participation_rank",
        "belief_r2",
        "future_r2",
        "future_modes",
        "spectral_overlap",
    ]
    correlations: dict[str, Any] = {}
    loss = np.array([row["evaluation"]["excess_bps"] for row in rows])
    for metric in metrics:
        values = np.array([_metric_value(row, metric) for row in rows])
        pearson = scipy.stats.pearsonr(values, loss)
        spearman = scipy.stats.spearmanr(values, loss)
        correlations[metric] = {
            "pearson_r_with_excess_bps": float(pearson.statistic),
            "pearson_p": float(pearson.pvalue),
            "spearman_r_with_excess_bps": float(spearman.statistic),
            "spearman_p": float(spearman.pvalue),
            "within_process_pairwise_concordance": _within_process_concordance(rows, metric),
        }

    # Rank the models within each process to make failures legible.
    winners = {}
    for process_name in PROCESS_SPECS:
        group = [row for row in rows if row["process_name"] == process_name]
        group.sort(key=lambda row: row["evaluation"]["excess_bps"])
        winners[process_name] = [
            {
                "model": row["model_name"],
                "excess_bps": row["evaluation"]["excess_bps"],
                "future_r2": row["predictive_state"]["future_prediction"]["weighted_future_r2"],
                "balanced_rank": row["predictive_state"]["balanced"]["participation_rank"],
            }
            for row in group
        ]

    payload = {
        "processes": process_rows,
        "models": rows,
        "correlations": correlations,
        "rankings": winners,
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    print(json.dumps(payload, indent=2)[:100_000])
    return payload


def main() -> None:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run")
    run.add_argument("--process", choices=PROCESS_SPECS, required=True)
    run.add_argument("--model", choices=MODEL_SPECS, required=True)
    run.add_argument("--seed", type=int, default=7)
    run.add_argument("--steps", type=int, default=200)
    run.add_argument("--output", type=Path, required=True)
    analyze = sub.add_parser("aggregate")
    analyze.add_argument("--directory", type=Path, required=True)
    analyze.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "run":
        run_one(args.process, args.model, args.output, seed=args.seed, steps=args.steps)
    else:
        aggregate(args.directory, args.output)


if __name__ == "__main__":
    main()
