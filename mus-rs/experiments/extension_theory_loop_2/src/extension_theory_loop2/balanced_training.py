from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import numpy as np
import torch
from torch import nn
import torch.nn.functional as F

from .atlas import PROCESS_SPECS
from .core import ObservableHMM, StableCliffordBank, evaluate_model
from .metrics import analyze_predictive_state


def predictive_cca_score(states: torch.Tensor, tokens: torch.Tensor, horizon: int = 4) -> tuple[torch.Tensor, torch.Tensor]:
    """Fast, differentiable finite-horizon predictive-correlation score.

    Whitening matrices are recomputed from the current batch but detached from
    autograd. This prevents the objective from being won by state rescaling while
    avoiding costly derivatives through Cholesky/SVD. Gradients flow through the
    whitened state--future cross-covariance itself.
    """

    batch, length, width = states.shape
    future = torch.cat(
        [F.one_hot(tokens[:, h : h + length], num_classes=4).float() for h in range(1, horizon + 1)],
        dim=-1,
    )
    x = states.reshape(batch * length, width)
    y = future.reshape(batch * length, -1)
    # A deterministic subsample is enough for the macroscopic statistic.
    if x.shape[0] > 512:
        index = torch.linspace(0, x.shape[0] - 1, 512, device=x.device).long()
        x = x[index]
        y = y[index]
    x = x - x.mean(dim=0, keepdim=True)
    y = y - y.mean(dim=0, keepdim=True)
    n = max(x.shape[0] - 1, 1)
    with torch.no_grad():
        cxx = x.detach().T @ x.detach() / n + 3e-3 * torch.eye(width, device=x.device)
        cyy = y.detach().T @ y.detach() / n + 3e-3 * torch.eye(y.shape[1], device=y.device)
        vx, ux = torch.linalg.eigh(cxx)
        vy, uy = torch.linalg.eigh(cyy)
        wx = (ux * vx.clamp_min(1e-6).rsqrt()[None, :]) @ ux.T
        wy = (uy * vy.clamp_min(1e-6).rsqrt()[None, :]) @ uy.T
    cross = wx @ (x.T @ y / n) @ wy
    score = cross.square().sum()
    singular = torch.linalg.svdvals(cross.detach())
    return score, singular


def run_condition(
    condition: str,
    output: Path,
    *,
    seed: int,
    steps: int,
    process_name: str = "p6_baseline",
) -> dict[str, Any]:
    torch.set_num_threads(1)
    try:
        torch.set_num_interop_threads(1)
    except RuntimeError:
        pass
    torch.manual_seed(seed)
    process = ObservableHMM(PROCESS_SPECS[process_name])
    model = StableCliffordBank(process.symbols, latent_dim=3, fibers=1)
    future_heads = nn.ModuleList([nn.Linear(model.state_size, process.symbols) for _ in range(4)])
    parameters = list(model.parameters())
    if condition in {"future_aux", "aux_cca"}:
        parameters += list(future_heads.parameters())
    optimizer = torch.optim.AdamW(parameters, lr=0.006, weight_decay=1e-5)
    scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=steps)
    generator = torch.Generator().manual_seed(seed + 1009)
    history = []

    for step in range(1, steps + 1):
        tokens = process.sample(128, 29, generator)
        inputs = tokens[:, :24]
        targets = tokens[:, 1:25]
        logits, states = model.forward_with_states(inputs)
        next_loss = F.cross_entropy(logits.reshape(-1, process.symbols), targets.reshape(-1))
        auxiliary = torch.zeros((), dtype=next_loss.dtype)
        if condition in {"future_aux", "aux_cca"}:
            for horizon, head in enumerate(future_heads, start=1):
                auxiliary = auxiliary + F.cross_entropy(
                    head(states).reshape(-1, process.symbols),
                    tokens[:, horizon : horizon + 24].reshape(-1),
                ) / 4
        cca, singular = predictive_cca_score(states, tokens, horizon=4)
        cca_weight = 0.035 if condition in {"cca", "aux_cca"} else 0.0
        aux_weight = 0.20 if condition in {"future_aux", "aux_cca"} else 0.0
        loss = next_loss + aux_weight * auxiliary - cca_weight * cca + model.regularization()
        optimizer.zero_grad(set_to_none=True)
        loss.backward()
        gradient = float(nn.utils.clip_grad_norm_(parameters, 5.0))
        optimizer.step()
        scheduler.step()
        if step in {1, 40, 80, 160, steps}:
            history.append(
                {
                    "step": step,
                    "loss": float(loss.detach()),
                    "next_loss": float(next_loss.detach()),
                    "auxiliary": float(auxiliary.detach()),
                    "cca_score": float(cca.detach()),
                    "cca_singular_values": singular.detach().tolist(),
                    "gradient_norm": gradient,
                }
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
    payload = {
        "condition": condition,
        "process": process_name,
        "seed": seed,
        "steps": steps,
        "final": final,
        "predictive_state": predictive,
        "history": history,
        "diagnostics": model.diagnostics(),
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    torch.save({"state_dict": model.state_dict(), "payload": payload}, output.with_suffix(".pt"))
    print(json.dumps({"DONE": payload}, sort_keys=True), flush=True)
    return payload


def aggregate(directory: Path, output: Path) -> dict[str, Any]:
    rows = [json.loads(path.read_text()) for path in sorted(directory.glob("*.json"))]
    by_condition = {}
    for condition in sorted({row["condition"] for row in rows}):
        group = [row for row in rows if row["condition"] == condition]
        by_condition[condition] = {
            "n": len(group),
            "excess_bps": [row["final"]["excess_bps"] for row in group],
            "mean_excess_bps": float(np.mean([row["final"]["excess_bps"] for row in group])),
            "mean_future_r2": float(
                np.mean(
                    [
                        row["predictive_state"]["future_prediction"]["weighted_future_r2"]
                        for row in group
                    ]
                )
            ),
            "mean_balanced_rank": float(
                np.mean(
                    [row["predictive_state"]["balanced"]["participation_rank"] for row in group]
                )
            ),
            "mean_belief_r2": float(
                np.mean([row["predictive_state"]["belief_probe"]["r2"] for row in group])
            ),
            "mean_final_cca_score": float(np.mean([row["history"][-1]["cca_score"] for row in group])),
        }
    payload = {"conditions": by_condition, "runs": rows}
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    print(json.dumps(payload, indent=2)[:100_000])
    return payload


def main() -> None:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run")
    run.add_argument("--condition", choices=["baseline", "future_aux", "cca", "aux_cca"], required=True)
    run.add_argument("--process", choices=PROCESS_SPECS, default="p6_baseline")
    run.add_argument("--seed", type=int, default=7)
    run.add_argument("--steps", type=int, default=300)
    run.add_argument("--output", type=Path, required=True)
    analyze = sub.add_parser("aggregate")
    analyze.add_argument("--directory", type=Path, required=True)
    analyze.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "run":
        run_condition(args.condition, args.output, seed=args.seed, steps=args.steps, process_name=args.process)
    else:
        aggregate(args.directory, args.output)


if __name__ == "__main__":
    main()
