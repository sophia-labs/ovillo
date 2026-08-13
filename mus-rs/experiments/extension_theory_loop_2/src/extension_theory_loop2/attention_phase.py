from __future__ import annotations

import argparse
import itertools
import json
import math
from pathlib import Path
from typing import Any

import numpy as np
import scipy.optimize
import torch
from torch import nn
import torch.nn.functional as F


def retrieval_batch(
    batch: int,
    keys: int,
    routes: int,
    generator: torch.Generator,
) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
    permutations = torch.stack(
        [
            torch.stack([torch.randperm(keys, generator=generator) for _ in range(batch)])
            for _ in range(routes)
        ],
        dim=1,
    )  # [B, R, K]
    values = torch.randn(batch, routes, keys, generator=generator)
    queries = torch.randint(keys, (batch, routes), generator=generator)
    dimension = routes * keys + routes + 1
    tokens = torch.zeros(batch, keys + 1, dimension)
    for route in range(routes):
        key_slice = slice(route * keys, (route + 1) * keys)
        tokens[:, :keys, key_slice] = F.one_hot(
            permutations[:, route], keys
        ).float()
        tokens[:, :keys, routes * keys + route] = values[:, route]
        tokens[:, keys, key_slice] = F.one_hot(queries[:, route], keys).float()
    tokens[:, keys, -1] = 1.0

    positions = torch.empty(batch, routes, dtype=torch.long)
    target = torch.empty(batch, routes)
    row = torch.arange(batch)
    for route in range(routes):
        positions[:, route] = (
            permutations[:, route] == queries[:, route, None]
        ).float().argmax(dim=1)
        target[:, route] = values[row, route, positions[:, route]]
    return tokens, target, positions


class RoutedMHA(nn.Module):
    def __init__(
        self,
        input_dim: int,
        outputs: int,
        heads: int,
        *,
        query_dim: int = 8,
        value_dim: int | None = None,
        collision_noise: float = 1e-3,
        output_multiplier: float = 1.0,
    ):
        super().__init__()
        value_dim = value_dim or outputs
        self.heads = heads
        self.outputs = outputs
        self.query_dim = query_dim
        self.value_dim = value_dim
        self.output_multiplier = output_multiplier

        base_q = torch.randn(input_dim, query_dim) * 0.20
        base_k = torch.randn(input_dim, query_dim) * 0.20
        base_v = torch.randn(input_dim, value_dim) * 0.20
        base_o = torch.randn(value_dim, outputs) * 0.20
        self.wq = nn.Parameter(
            base_q[None].repeat(heads, 1, 1)
            + collision_noise * torch.randn(heads, input_dim, query_dim)
        )
        self.wk = nn.Parameter(
            base_k[None].repeat(heads, 1, 1)
            + collision_noise * torch.randn(heads, input_dim, query_dim)
        )
        self.wv = nn.Parameter(
            base_v[None].repeat(heads, 1, 1)
            + collision_noise * torch.randn(heads, input_dim, value_dim)
        )
        self.wo = nn.Parameter(
            base_o[None].repeat(heads, 1, 1)
            + collision_noise * torch.randn(heads, value_dim, outputs)
        )
        self.log_temperature = nn.Parameter(torch.tensor(math.log(math.sqrt(query_dim))))

    def forward(
        self, tokens: torch.Tensor, *, return_parts: bool = False
    ) -> torch.Tensor | tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        data = tokens[:, :-1]
        query = tokens[:, -1]
        q = torch.einsum("bd,hdk->bhk", query, self.wq)
        k = torch.einsum("bnd,hdk->bhnk", data, self.wk)
        score = torch.einsum("bhk,bhnk->bhn", q, k) / self.log_temperature.exp().clamp_min(0.2)
        attention = torch.softmax(score, dim=-1)
        value = torch.einsum("bnd,hdv->bhnv", data, self.wv)
        head_state = torch.einsum("bhn,bhnv->bhv", attention, value)
        parts = torch.einsum("bhv,hvo->bho", head_state, self.wo)
        output = self.output_multiplier * parts.sum(dim=1)
        return (output, attention, parts) if return_parts else output

    @torch.no_grad()
    def invariant_components(self) -> tuple[torch.Tensor, torch.Tensor]:
        # B_h = WQ_h WK_h^T, C_h = WV_h WO_h are invariant to internal
        # query/key and value/output basis gauges.
        b = torch.einsum("hdi,hei->hde", self.wq, self.wk)
        c = torch.einsum("hdv,hvo->hdo", self.wv, self.wo)
        return b, c


@torch.no_grad()
def model_metrics(
    model: RoutedMHA,
    tokens: torch.Tensor,
    target: torch.Tensor,
    positions: torch.Tensor,
) -> dict[str, Any]:
    model.eval()
    output, attention, parts = model(tokens, return_parts=True)
    mse = float(F.mse_loss(output, target))
    align = torch.empty(model.heads, model.outputs)
    for head in range(model.heads):
        for route in range(model.outputs):
            align[head, route] = attention[:, head].gather(
                1, positions[:, route, None]
            ).mean()
    row, column = scipy.optimize.linear_sum_assignment(-align.cpu().numpy())
    assignment = float(align[row, column].mean()) if len(row) else 0.0

    contribution = parts.permute(1, 0, 2).reshape(model.heads, -1)
    singular = torch.linalg.svdvals(contribution)
    relative = singular / singular[0].clamp_min(1e-12)
    participation = float(singular.sum().square() / singular.square().sum().clamp_min(1e-12))
    effective = int((relative > 0.10).sum())

    b, c = model.invariant_components()
    distances = []
    for i in range(model.heads):
        for j in range(i + 1, model.heads):
            distances.append(
                float(torch.linalg.vector_norm(b[i] - b[j]) + torch.linalg.vector_norm(c[i] - c[j]))
            )
    norms = contribution.norm(dim=1)
    return {
        "mse": mse,
        "attention_alignment": align.tolist(),
        "assignment": assignment,
        "component_singular_values": singular.tolist(),
        "component_singular_relative": relative.tolist(),
        "component_participation_rank": participation,
        "component_effective_rank_0_1": effective,
        "mean_invariant_head_distance": float(np.mean(distances)) if distances else 0.0,
        "component_norms": norms.tolist(),
    }


def run_one(
    *,
    routes: int,
    heads: int,
    seed: int,
    steps: int,
    collision_noise: float,
    output_multiplier: float,
    output: Path,
) -> dict[str, Any]:
    torch.set_num_threads(1)
    try:
        torch.set_num_interop_threads(1)
    except RuntimeError:
        pass
    torch.manual_seed(seed)
    train_generator = torch.Generator().manual_seed(seed + 100)
    probe_generator = torch.Generator().manual_seed(2026 + routes)
    probe = retrieval_batch(4096, 5, routes, probe_generator)
    input_dim = routes * 5 + routes + 1
    model = RoutedMHA(
        input_dim,
        routes,
        heads,
        query_dim=max(6, 2 * routes),
        value_dim=max(2, routes),
        collision_noise=collision_noise,
        output_multiplier=output_multiplier,
    )
    optimizer = torch.optim.AdamW(model.parameters(), lr=0.010, weight_decay=1e-5)
    history = []
    checkpoints = sorted(set([0, 1, 5, 10, steps] + list(range(20, steps + 1, 20))))
    emergence = None

    for step in range(steps + 1):
        if step in checkpoints:
            row = {"step": step, **model_metrics(model, *probe)}
            history.append(row)
            required = min(routes, heads)
            rel = row["component_singular_relative"]
            if (
                emergence is None
                and len(rel) >= required
                and rel[required - 1] > 0.50
                and row["assignment"] > 0.75
            ):
                emergence = step
        if step == steps:
            break
        model.train()
        tokens, target, _ = retrieval_batch(256, 5, routes, train_generator)
        prediction = model(tokens)
        loss = F.mse_loss(prediction, target)
        optimizer.zero_grad(set_to_none=True)
        loss.backward()
        nn.utils.clip_grad_norm_(model.parameters(), 5.0)
        optimizer.step()

    final = model_metrics(model, *probe)
    payload = {
        "routes": routes,
        "heads": heads,
        "seed": seed,
        "steps": steps,
        "collision_noise": collision_noise,
        "output_multiplier": output_multiplier,
        "parameters": sum(parameter.numel() for parameter in model.parameters()),
        "emergence_step": emergence,
        "history": history,
        "final": final,
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    torch.save({"state_dict": model.state_dict(), "payload": payload}, output.with_suffix(".pt"))
    print(json.dumps({"DONE": payload}, sort_keys=True), flush=True)
    return payload


def aggregate(directory: Path, output: Path) -> dict[str, Any]:
    rows = [json.loads(path.read_text()) for path in sorted(directory.glob("*.json"))]
    phase = []
    for row in rows:
        final = row["final"]
        phase.append(
            {
                "routes": row["routes"],
                "heads": row["heads"],
                "seed": row["seed"],
                "collision_noise": row["collision_noise"],
                "mse": final["mse"],
                "functional_rank": final["component_participation_rank"],
                "effective_rank": final["component_effective_rank_0_1"],
                "assignment": final["assignment"],
                "emergence_step": row["emergence_step"],
            }
        )
    standard = [row for row in phase if abs(row["collision_noise"] - 1e-3) < 1e-12]
    target = np.array([min(row["routes"], row["heads"]) for row in standard], dtype=float)
    learned = np.array([row["functional_rank"] for row in standard], dtype=float)
    correlation = float(np.corrcoef(target, learned)[0, 1])
    mae = float(np.mean(np.abs(target - learned)))

    collision = [row for row in phase if row["routes"] == 3 and row["heads"] == 3]
    usable = [row for row in collision if row["emergence_step"] is not None]
    if len(usable) >= 2:
        x = np.array([-math.log10(row["collision_noise"]) for row in usable])
        y = np.array([row["emergence_step"] for row in usable])
        slope, intercept = np.polyfit(x, y, 1)
        emergence_fit = {
            "slope_steps_per_decade": float(slope),
            "intercept": float(intercept),
            "correlation": float(np.corrcoef(x, y)[0, 1]),
        }
    else:
        emergence_fit = None
    payload = {
        "phase_rows": phase,
        "functional_rank_vs_min_heads_routes": {
            "correlation": correlation,
            "mean_absolute_error": mae,
        },
        "collision_emergence_fit": emergence_fit,
        "runs": rows,
    }
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    print(json.dumps(payload, indent=2)[:100_000])
    return payload


def main() -> None:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run")
    run.add_argument("--routes", type=int, required=True)
    run.add_argument("--heads", type=int, required=True)
    run.add_argument("--seed", type=int, default=7)
    run.add_argument("--steps", type=int, default=600)
    run.add_argument("--collision-noise", type=float, default=1e-3)
    run.add_argument("--output-multiplier", type=float, default=1.0)
    run.add_argument("--output", type=Path, required=True)
    analyze = sub.add_parser("aggregate")
    analyze.add_argument("--directory", type=Path, required=True)
    analyze.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "run":
        run_one(
            routes=args.routes,
            heads=args.heads,
            seed=args.seed,
            steps=args.steps,
            collision_noise=args.collision_noise,
            output_multiplier=args.output_multiplier,
            output=args.output,
        )
    else:
        aggregate(args.directory, args.output)


if __name__ == "__main__":
    main()
