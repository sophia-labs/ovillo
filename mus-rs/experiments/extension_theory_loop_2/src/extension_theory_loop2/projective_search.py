from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import numpy as np
import scipy.optimize
import torch
from sklearn.decomposition import NMF

from .atlas import PROCESS_SPECS
from .core import ObservableHMM, PositiveProjectiveModel, TrainConfig, evaluate_model, train_predictive_model
from .metrics import analyze_predictive_state
from .projective_init import empirical_hankel, initialize_from_factorization


def factor_candidate(
    hankel: np.ndarray,
    shifted: list[np.ndarray],
    rank: int,
    *,
    solver: str,
    beta_loss: str,
    init: str,
    seed: int,
) -> dict[str, Any]:
    epsilon = max(1e-10, hankel.max() * 1e-10)
    nmf = NMF(
        n_components=rank,
        init=init,
        solver=solver,
        beta_loss=beta_loss,
        max_iter=2500,
        tol=2e-7,
        random_state=seed,
    )
    left = nmf.fit_transform(hankel + epsilon)
    right = nmf.components_
    design = np.kron(right.T, left)
    operators = []
    shift_residuals = []
    for matrix in shifted:
        solution, _ = scipy.optimize.nnls(
            design, matrix.reshape(-1, order="F"), maxiter=20_000
        )
        operator = solution.reshape(rank, rank, order="F")
        operators.append(operator)
        shift_residuals.append(
            float(np.linalg.norm(left @ operator @ right - matrix) / max(np.linalg.norm(matrix), 1e-12))
        )
    initial, initial_residual = scipy.optimize.nnls(right.T, hankel[0])
    terminal, terminal_residual = scipy.optimize.nnls(left, hankel[:, 0])
    return {
        "operators": np.stack(operators),
        "initial": initial,
        "terminal": terminal,
        "nmf_error": float(nmf.reconstruction_err_),
        "shift_residuals": shift_residuals,
        "initial_residual": float(initial_residual),
        "terminal_residual": float(terminal_residual),
        "iterations": int(nmf.n_iter_),
        "config": {
            "solver": solver,
            "beta_loss": beta_loss,
            "init": init,
            "seed": seed,
        },
    }


def run(output: Path, *, stream_length: int, seed: int, steps: int) -> dict[str, Any]:
    torch.set_num_threads(1)
    try:
        torch.set_num_interop_threads(1)
    except RuntimeError:
        pass
    process = ObservableHMM(PROCESS_SPECS["p6_baseline"])
    _, hankel, shifted, empirical = empirical_hankel(
        process, stream_length=stream_length, word_depth=2, seed=seed + 1000
    )
    candidates = []
    configs = []
    # Deterministic NNDSVD starts and randomized starts under both useful losses.
    configs.extend([("mu", "kullback-leibler", "nndsvda", seed)])
    configs.extend([("mu", "kullback-leibler", "nndsvdar", seed + i) for i in range(4)])
    configs.extend([("mu", "kullback-leibler", "random", seed + 10 + i) for i in range(5)])
    configs.extend([("cd", "frobenius", "nndsvda", seed)])
    configs.extend([("cd", "frobenius", "nndsvdar", seed + 20 + i) for i in range(3)])
    configs.extend([("cd", "frobenius", "random", seed + 30 + i) for i in range(4)])

    for index, (solver, beta, init, candidate_seed) in enumerate(configs):
        factor = factor_candidate(
            hankel,
            shifted,
            process.states,
            solver=solver,
            beta_loss=beta,
            init=init,
            seed=candidate_seed,
        )
        model = PositiveProjectiveModel(process.symbols, process.states)
        initialize_from_factorization(model, factor)
        validation = evaluate_model(
            model,
            process,
            seed=seed + 50_000,
            batches=5,
            batch=256,
            length=56,
        )
        candidates.append(
            {
                "index": index,
                "config": factor["config"],
                "validation": validation,
                "nmf_error": factor["nmf_error"],
                "mean_shift_residual": float(np.mean(factor["shift_residuals"])),
                "max_shift_residual": float(np.max(factor["shift_residuals"])),
                "factor": factor,
            }
        )

    best = min(candidates, key=lambda row: row["validation"]["bps"])
    model = PositiveProjectiveModel(process.symbols, process.states)
    initialize_from_factorization(model, best["factor"])
    initial = evaluate_model(
        model, process, seed=seed + 60_000, batches=16, batch=256, length=64
    )
    training = train_predictive_model(
        model,
        process,
        TrainConfig(steps=steps, batch=128, length=24, learning_rate=0.004, seed=seed),
    )
    final = evaluate_model(
        model, process, seed=seed + 70_000, batches=24, batch=256, length=64
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
    compact_candidates = [
        {key: value for key, value in row.items() if key != "factor"} for row in candidates
    ]
    payload = {
        "stream_length": stream_length,
        "seed": seed,
        "steps": steps,
        "empirical": empirical,
        "candidate_count": len(candidates),
        "candidates": compact_candidates,
        "selected_index": best["index"],
        "selected_config": best["config"],
        "selected_validation": best["validation"],
        "initial": initial,
        "final": final,
        "training": training["history"],
        "predictive_state": predictive,
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"DONE": payload}, sort_keys=True), flush=True)
    return payload


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--stream-length", type=int, default=100_000)
    parser.add_argument("--seed", type=int, default=7)
    parser.add_argument("--steps", type=int, default=300)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    run(args.output, stream_length=args.stream_length, seed=args.seed, steps=args.steps)


if __name__ == "__main__":
    main()
