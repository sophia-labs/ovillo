from __future__ import annotations

import argparse
import itertools
import json
from pathlib import Path
from typing import Any

import numpy as np
import scipy.optimize
import torch
from sklearn.decomposition import NMF

from .atlas import PROCESS_SPECS
from .core import (
    ObservableHMM,
    PositiveProjectiveModel,
    TrainConfig,
    evaluate_model,
    train_predictive_model,
)
from .metrics import analyze_predictive_state


def inverse_softplus(value: torch.Tensor) -> torch.Tensor:
    return torch.log(torch.expm1(value.clamp_min(1e-7)))


def initialize_exact(model: PositiveProjectiveModel, process: ObservableHMM) -> None:
    if model.rank != process.states:
        raise ValueError("exact initialization requires rank equal to hidden state count")
    operators = torch.tensor(process.ops, dtype=torch.float32)
    operators = operators / operators.mean()
    initial = torch.tensor(process.pi, dtype=torch.float32)
    terminal = torch.ones(process.states, dtype=torch.float32)
    with torch.no_grad():
        model.raw_ops.copy_(inverse_softplus(operators - 1e-6))
        model.raw_initial.copy_(inverse_softplus(initial - 1e-5))
        model.raw_terminal.copy_(inverse_softplus(terminal - 1e-5))


def words(symbols: int, max_length: int) -> list[tuple[int, ...]]:
    result: list[tuple[int, ...]] = [()]
    for length in range(1, max_length + 1):
        result.extend(itertools.product(range(symbols), repeat=length))
    return result


def stream_word_probabilities(
    stream: np.ndarray, symbols: int, max_length: int
) -> dict[tuple[int, ...], float]:
    probabilities: dict[tuple[int, ...], float] = {(): 1.0}
    for length in range(1, max_length + 1):
        count = np.zeros(symbols**length, dtype=np.int64)
        if len(stream) < length:
            continue
        code = np.zeros(len(stream) - length + 1, dtype=np.int64)
        for offset in range(length):
            code = code * symbols + stream[offset : offset + len(code)]
        count += np.bincount(code, minlength=symbols**length)
        denominator = count.sum()
        for index, value in enumerate(count):
            digits = []
            remaining = index
            for _ in range(length):
                digits.append(remaining % symbols)
                remaining //= symbols
            probabilities[tuple(reversed(digits))] = float(value / denominator)
    return probabilities


def empirical_hankel(
    process: ObservableHMM,
    *,
    stream_length: int,
    word_depth: int,
    seed: int,
) -> tuple[list[tuple[int, ...]], np.ndarray, list[np.ndarray], dict[str, Any]]:
    stream = process.sample_stream(stream_length, seed)
    probability = stream_word_probabilities(stream, process.symbols, 2 * word_depth + 1)
    basis = words(process.symbols, word_depth)
    hankel = np.array([[probability[u + v] for v in basis] for u in basis])
    shifted = [
        np.array(
            [[probability[u + (symbol,) + v] for v in basis] for u in basis]
        )
        for symbol in range(process.symbols)
    ]
    return basis, hankel, shifted, {
        "stream_length": stream_length,
        "word_depth": word_depth,
        "basis_size": len(basis),
        "minimum_nonzero_probability": float(
            min(value for value in probability.values() if value > 0)
        ),
    }


def positive_hankel_factorization(
    hankel: np.ndarray,
    shifted: list[np.ndarray],
    rank: int,
    *,
    seed: int,
    max_iter: int = 4_000,
) -> dict[str, Any]:
    # KL-NMF emphasizes relative accuracy in weak probability entries and keeps
    # the factors positive. A tiny pseudocount avoids exact zeros from finite data.
    epsilon = max(1e-10, hankel.max() * 1e-10)
    model = NMF(
        n_components=rank,
        init="nndsvda",
        solver="mu",
        beta_loss="kullback-leibler",
        max_iter=max_iter,
        tol=1e-7,
        random_state=seed,
    )
    left = model.fit_transform(hankel + epsilon)
    right = model.components_

    design = np.kron(right.T, left)
    operators = []
    residuals = []
    for matrix in shifted:
        solution, residual = scipy.optimize.nnls(
            design, matrix.reshape(-1, order="F"), maxiter=20_000
        )
        operator = solution.reshape(rank, rank, order="F")
        operators.append(operator)
        reconstructed = left @ operator @ right
        residuals.append(
            float(np.linalg.norm(reconstructed - matrix) / max(np.linalg.norm(matrix), 1e-12))
        )

    initial, initial_residual = scipy.optimize.nnls(right.T, hankel[0])
    terminal, terminal_residual = scipy.optimize.nnls(left, hankel[:, 0])
    operators_array = np.stack(operators)
    return {
        "left": left,
        "right": right,
        "operators": operators_array,
        "initial": initial,
        "terminal": terminal,
        "nmf_reconstruction_error": float(model.reconstruction_err_),
        "shift_relative_residuals": residuals,
        "initial_residual": float(initial_residual),
        "terminal_residual": float(terminal_residual),
        "iterations": int(model.n_iter_),
    }


def initialize_from_factorization(
    model: PositiveProjectiveModel, factorization: dict[str, Any]
) -> None:
    operators = torch.tensor(factorization["operators"], dtype=torch.float32).clamp_min(1e-7)
    operators = operators / operators.mean()
    initial = torch.tensor(factorization["initial"], dtype=torch.float32).clamp_min(1e-7)
    terminal = torch.tensor(factorization["terminal"], dtype=torch.float32).clamp_min(1e-7)
    # Overall state/terminal gauge does not matter; keep values in a conditioned range.
    initial = initial / initial.mean()
    terminal = terminal / terminal.mean()
    with torch.no_grad():
        model.raw_ops.copy_(inverse_softplus(operators - 1e-6))
        model.raw_initial.copy_(inverse_softplus(initial - 1e-5))
        model.raw_terminal.copy_(inverse_softplus(terminal - 1e-5))


def split_rank(
    source: PositiveProjectiveModel,
    target_rank: int,
    *,
    noise: float,
    seed: int,
) -> PositiveProjectiveModel:
    if target_rank < source.rank:
        raise ValueError("target rank must be at least source rank")
    target = PositiveProjectiveModel(source.symbols, target_rank)
    rng = np.random.default_rng(seed)
    assignment = np.arange(target_rank) % source.rank
    old_ops = source.operators().detach().cpu().numpy()
    old_initial = source.initial_state(1)[0].detach().cpu().numpy()
    old_terminal = source.terminal().detach().cpu().numpy()
    new_ops = np.empty((source.symbols, target_rank, target_rank), dtype=np.float64)
    for symbol in range(source.symbols):
        for i in range(target_rank):
            rows = np.where(assignment == assignment[i])[0]
            for j in range(target_rank):
                cols = np.where(assignment == assignment[j])[0]
                new_ops[symbol, i, j] = old_ops[symbol, assignment[i], assignment[j]] / max(
                    len(cols), 1
                )
        new_ops[symbol] *= np.exp(noise * rng.normal(size=(target_rank, target_rank)))
    new_initial = np.array([old_initial[a] for a in assignment])
    for source_index in range(source.rank):
        new_initial[assignment == source_index] /= max((assignment == source_index).sum(), 1)
    new_terminal = np.array([old_terminal[a] for a in assignment])
    factor = {
        "operators": new_ops,
        "initial": new_initial,
        "terminal": new_terminal,
    }
    initialize_from_factorization(target, factor)
    return target


def run_condition(
    condition: str,
    process_name: str,
    output: Path,
    *,
    seed: int,
    steps: int,
    stream_length: int,
) -> dict[str, Any]:
    torch.set_num_threads(1)
    try:
        torch.set_num_interop_threads(1)
    except RuntimeError:
        pass
    torch.manual_seed(seed)
    process = ObservableHMM(PROCESS_SPECS[process_name])
    factor_diagnostics = None

    if condition == "exact":
        model = PositiveProjectiveModel(process.symbols, process.states)
        initialize_exact(model, process)
        initial = evaluate_model(model, process, seed=seed + 500, batches=12, batch=256, length=56)
        training = {"history": []}
    elif condition.startswith("nmf"):
        model = PositiveProjectiveModel(process.symbols, process.states)
        _, hankel, shifted, empirical = empirical_hankel(
            process,
            stream_length=stream_length,
            word_depth=2,
            seed=seed + 900,
        )
        factor = positive_hankel_factorization(
            hankel, shifted, process.states, seed=seed + 901
        )
        initialize_from_factorization(model, factor)
        factor_diagnostics = {
            "empirical": empirical,
            "nmf_reconstruction_error": factor["nmf_reconstruction_error"],
            "shift_relative_residuals": factor["shift_relative_residuals"],
            "initial_residual": factor["initial_residual"],
            "terminal_residual": factor["terminal_residual"],
            "iterations": factor["iterations"],
        }
        initial = evaluate_model(model, process, seed=seed + 500, batches=12, batch=256, length=56)
        horizon = 4 if condition == "nmf_horizon" else 0
        weight = 0.25 if condition == "nmf_horizon" else 0.0
        training = train_predictive_model(
            model,
            process,
            TrainConfig(
                steps=steps,
                batch=128,
                length=24,
                learning_rate=0.004,
                seed=seed,
                horizon_aux=horizon,
                horizon_weight=weight,
            ),
        )
    elif condition == "random":
        model = PositiveProjectiveModel(process.symbols, process.states)
        initial = evaluate_model(model, process, seed=seed + 500, batches=12, batch=256, length=56)
        training = train_predictive_model(
            model,
            process,
            TrainConfig(steps=steps, batch=128, length=24, learning_rate=0.004, seed=seed),
        )
    elif condition == "horizon":
        model = PositiveProjectiveModel(process.symbols, process.states)
        initial = evaluate_model(model, process, seed=seed + 500, batches=12, batch=256, length=56)
        training = train_predictive_model(
            model,
            process,
            TrainConfig(
                steps=steps,
                batch=128,
                length=24,
                learning_rate=0.004,
                seed=seed,
                horizon_aux=4,
                horizon_weight=0.25,
            ),
        )
    elif condition == "rank_continuation":
        source = PositiveProjectiveModel(process.symbols, 2)
        train_predictive_model(
            source,
            process,
            TrainConfig(
                steps=max(80, steps // 3),
                batch=128,
                length=24,
                learning_rate=0.005,
                seed=seed,
                horizon_aux=4,
                horizon_weight=0.25,
            ),
        )
        model = split_rank(source, process.states, noise=0.03, seed=seed + 77)
        initial = evaluate_model(model, process, seed=seed + 500, batches=12, batch=256, length=56)
        training = train_predictive_model(
            model,
            process,
            TrainConfig(
                steps=steps,
                batch=128,
                length=24,
                learning_rate=0.004,
                seed=seed + 1,
                horizon_aux=4,
                horizon_weight=0.20,
            ),
        )
    else:
        raise ValueError(f"unknown condition: {condition}")

    final = evaluate_model(model, process, seed=seed + 700, batches=24, batch=256, length=64)
    predictive = analyze_predictive_state(
        model,
        process,
        seed=seed + 800,
        train_samples=5000,
        test_samples=2500,
        history_length=28,
        future_horizon=5,
    )
    payload = {
        "condition": condition,
        "process": process_name,
        "seed": seed,
        "steps": steps,
        "stream_length": stream_length,
        "initial": initial,
        "final": final,
        "training": training["history"],
        "predictive_state": predictive,
        "diagnostics": model.diagnostics(),
        "factorization": factor_diagnostics,
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    torch.save({"state_dict": model.state_dict(), "payload": payload}, output.with_suffix(".pt"))
    print(json.dumps({"DONE": payload}, sort_keys=True), flush=True)
    return payload


def aggregate(directory: Path, output: Path) -> dict[str, Any]:
    rows = [json.loads(path.read_text()) for path in sorted(directory.glob("*.json"))]
    by_condition: dict[str, Any] = {}
    for condition in sorted({row["condition"] for row in rows}):
        group = [row for row in rows if row["condition"] == condition]
        values = np.array([row["final"]["excess_bps"] for row in group])
        by_condition[condition] = {
            "n": len(group),
            "mean_excess_bps": float(values.mean()),
            "sample_sd": float(values.std(ddof=1)) if len(values) > 1 else 0.0,
            "values": values.tolist(),
            "mean_initial_excess_bps": float(
                np.mean([row["initial"]["excess_bps"] for row in group])
            ),
            "mean_future_r2": float(
                np.mean(
                    [
                        row["predictive_state"]["future_prediction"]["weighted_future_r2"]
                        for row in group
                    ]
                )
            ),
        }
    payload = {"conditions": by_condition, "runs": rows}
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    print(json.dumps(payload, indent=2)[:100_000])
    return payload


def main() -> None:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run")
    run.add_argument(
        "--condition",
        choices=["exact", "random", "horizon", "nmf", "nmf_horizon", "rank_continuation"],
        required=True,
    )
    run.add_argument("--process", choices=PROCESS_SPECS, default="p6_baseline")
    run.add_argument("--seed", type=int, default=7)
    run.add_argument("--steps", type=int, default=300)
    run.add_argument("--stream-length", type=int, default=1_500_000)
    run.add_argument("--output", type=Path, required=True)
    analyze = sub.add_parser("aggregate")
    analyze.add_argument("--directory", type=Path, required=True)
    analyze.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "run":
        run_condition(
            args.condition,
            args.process,
            args.output,
            seed=args.seed,
            steps=args.steps,
            stream_length=args.stream_length,
        )
    else:
        aggregate(args.directory, args.output)


if __name__ == "__main__":
    main()
