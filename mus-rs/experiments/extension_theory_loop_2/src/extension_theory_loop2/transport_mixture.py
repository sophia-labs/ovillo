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
from .core import ObservableHMM, PositiveProjectiveModel, clifford_generators
from .projective_init import initialize_from_factorization, positive_hankel_factorization, empirical_hankel


MOVE_DELTAS = torch.tensor(
    [[0.65, 0.0], [0.0, 0.65], [-0.65, 0.0], [0.0, -0.65]], dtype=torch.float32
)
PLUS_LOOP = torch.tensor([0, 1, 2, 3], dtype=torch.long)
MINUS_LOOP = torch.tensor([1, 0, 3, 2], dtype=torch.long)


def segment_operator(delta: torch.Tensor, generators: torch.Tensor) -> torch.Tensor:
    g = torch.einsum("...a,aij->...ij", delta, generators)
    radius = delta.square().sum(dim=-1).sqrt()
    safe = radius.clamp_min(1e-7)
    sinc = torch.sin(radius) / safe
    identity = torch.eye(g.shape[-1], dtype=delta.dtype, device=delta.device)
    return torch.cos(radius)[..., None, None] * identity + sinc[..., None, None] * g


class FixedCliffordPath(nn.Module):
    def __init__(self, commuting: bool = False):
        super().__init__()
        generators = clifford_generators(2)
        if commuting:
            generators = torch.stack([generators[0], generators[0]])
        self.register_buffer("generators", generators)
        self.state_size = 4

    def forward(self, moves: torch.Tensor) -> torch.Tensor:
        state = torch.zeros(moves.shape[0], 4, device=moves.device)
        state[:, 0] = 1.0
        for t in range(moves.shape[1]):
            delta = MOVE_DELTAS.to(moves.device)[moves[:, t]]
            operator = segment_operator(delta, self.generators)
            state = torch.einsum("bi,bij->bj", state, operator)
        return state


@torch.no_grad()
def build_projective_from_observations(process: ObservableHMM, seed: int) -> PositiveProjectiveModel:
    _, hankel, shifted, _ = empirical_hankel(
        process, stream_length=100_000, word_depth=2, seed=seed + 100
    )
    factor = positive_hankel_factorization(hankel, shifted, process.states, seed=seed + 101)
    model = PositiveProjectiveModel(process.symbols, process.states)
    initialize_from_factorization(model, factor)
    model.eval()
    for parameter in model.parameters():
        parameter.requires_grad_(False)
    return model


def mixed_batch(
    process: ObservableHMM,
    batch: int,
    loops: int,
    generator: torch.Generator,
) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor]:
    length = loops * 4
    observations = process.sample(batch, length + 1, generator)
    signs = torch.where(
        torch.rand(batch, loops, generator=generator) > 0.5,
        torch.ones(batch, loops, dtype=torch.long),
        -torch.ones(batch, loops, dtype=torch.long),
    )
    moves = torch.empty(batch, length, dtype=torch.long)
    for block in range(loops):
        plus = signs[:, block] > 0
        moves[plus, block * 4 : (block + 1) * 4] = PLUS_LOOP
        moves[~plus, block * 4 : (block + 1) * 4] = MINUS_LOOP
    orientation = (signs.sum(dim=1) > 0).long()  # loops is odd, so no ties.
    return observations[:, :-1], observations[:, -1], moves, orientation


class TypedTransportBank(nn.Module):
    def __init__(
        self,
        projective: PositiveProjectiveModel,
        *,
        commuting: bool = False,
        mode: str = "full",
    ):
        super().__init__()
        self.projective = projective
        self.clifford = FixedCliffordPath(commuting=commuting)
        self.mode = mode
        self.projective_obs = nn.Linear(projective.state_size, 4)
        self.clifford_obs = nn.Linear(4, 4)
        self.projective_area = nn.Linear(projective.state_size, 2)
        self.clifford_area = nn.Linear(4, 2)
        self.obs_gate = nn.Parameter(torch.zeros(2))
        self.area_gate = nn.Parameter(torch.zeros(2))

    def projective_state(self, observations: torch.Tensor) -> torch.Tensor:
        return self.projective.extract_state(observations)

    def forward(
        self, observations: torch.Tensor, moves: torch.Tensor
    ) -> tuple[torch.Tensor, torch.Tensor, dict[str, torch.Tensor]]:
        projective = self.projective_state(observations)
        clifford = self.clifford(moves)
        obs_parts = torch.stack(
            [self.projective_obs(projective), self.clifford_obs(clifford)], dim=1
        )
        area_parts = torch.stack(
            [self.projective_area(projective), self.clifford_area(clifford)], dim=1
        )
        if self.mode == "projective_only":
            obs_weight = area_weight = torch.tensor([1.0, 0.0], device=observations.device)
        elif self.mode == "clifford_only":
            obs_weight = area_weight = torch.tensor([0.0, 1.0], device=observations.device)
        else:
            obs_weight = torch.softmax(self.obs_gate, dim=0)
            area_weight = torch.softmax(self.area_gate, dim=0)
        obs_logits = torch.einsum("k,bko->bo", obs_weight, obs_parts)
        area_logits = torch.einsum("k,bko->bo", area_weight, area_parts)
        return obs_logits, area_logits, {
            "obs_weight": obs_weight,
            "area_weight": area_weight,
            "projective_state": projective,
            "clifford_state": clifford,
        }


def run_condition(
    condition: str,
    output: Path,
    *,
    seed: int,
    steps: int,
) -> dict[str, Any]:
    torch.set_num_threads(1)
    try:
        torch.set_num_interop_threads(1)
    except RuntimeError:
        pass
    torch.manual_seed(seed)
    process = ObservableHMM(PROCESS_SPECS["p6_baseline"])
    projective = build_projective_from_observations(process, seed)
    model = TypedTransportBank(
        projective,
        commuting=condition == "commuting",
        mode=condition if condition in {"projective_only", "clifford_only"} else "full",
    )
    parameters = [parameter for parameter in model.parameters() if parameter.requires_grad]
    optimizer = torch.optim.AdamW(parameters, lr=0.015, weight_decay=1e-5)
    generator = torch.Generator().manual_seed(seed + 500)
    history = []

    for step in range(1, steps + 1):
        observations, obs_target, moves, area_target = mixed_batch(
            process, 256, 3, generator
        )
        obs_logits, area_logits, info = model(observations, moves)
        obs_loss = F.cross_entropy(obs_logits, obs_target)
        area_loss = F.cross_entropy(area_logits, area_target)
        entropy = torch.zeros(())
        if condition not in {"projective_only", "clifford_only"}:
            for weight in [info["obs_weight"], info["area_weight"]]:
                entropy = entropy - (weight * weight.clamp_min(1e-8).log()).sum()
        loss = obs_loss + area_loss + 0.01 * entropy
        optimizer.zero_grad(set_to_none=True)
        loss.backward()
        nn.utils.clip_grad_norm_(parameters, 5.0)
        optimizer.step()
        if step in {1, 40, 80, 160, steps}:
            history.append(
                {
                    "step": step,
                    "obs_loss": float(obs_loss.detach()),
                    "area_loss": float(area_loss.detach()),
                    "obs_gate": info["obs_weight"].detach().tolist(),
                    "area_gate": info["area_weight"].detach().tolist(),
                }
            )

    model.eval()
    generator = torch.Generator().manual_seed(seed + 9000)
    obs_loss = area_loss = 0.0
    obs_correct = area_correct = total = 0
    with torch.no_grad():
        for _ in range(24):
            observations, obs_target, moves, area_target = mixed_batch(
                process, 512, 3, generator
            )
            obs_logits, area_logits, info = model(observations, moves)
            obs_loss += float(F.cross_entropy(obs_logits, obs_target, reduction="sum"))
            area_loss += float(F.cross_entropy(area_logits, area_target, reduction="sum"))
            obs_correct += int((obs_logits.argmax(dim=-1) == obs_target).sum())
            area_correct += int((area_logits.argmax(dim=-1) == area_target).sum())
            total += len(obs_target)
    payload = {
        "condition": condition,
        "seed": seed,
        "steps": steps,
        "final": {
            "obs_nll": obs_loss / total,
            "area_nll": area_loss / total,
            "obs_accuracy": obs_correct / total,
            "area_accuracy": area_correct / total,
            "obs_gate": info["obs_weight"].tolist(),
            "area_gate": info["area_weight"].tolist(),
        },
        "history": history,
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"DONE": payload}, sort_keys=True), flush=True)
    return payload


def aggregate(directory: Path, output: Path) -> dict[str, Any]:
    rows = [json.loads(path.read_text()) for path in sorted(directory.glob("*.json"))]
    conditions = {}
    for condition in sorted({row["condition"] for row in rows}):
        group = [row for row in rows if row["condition"] == condition]
        conditions[condition] = {
            key: float(np.mean([row["final"][key] for row in group]))
            if key not in {"obs_gate", "area_gate"}
            else np.mean([row["final"][key] for row in group], axis=0).tolist()
            for key in group[0]["final"]
        }
        conditions[condition]["n"] = len(group)
    payload = {"conditions": conditions, "runs": rows}
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    print(json.dumps(payload, indent=2)[:100_000])
    return payload


def main() -> None:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run")
    run.add_argument(
        "--condition",
        choices=["full", "commuting", "projective_only", "clifford_only"],
        required=True,
    )
    run.add_argument("--seed", type=int, default=7)
    run.add_argument("--steps", type=int, default=300)
    run.add_argument("--output", type=Path, required=True)
    analyze = sub.add_parser("aggregate")
    analyze.add_argument("--directory", type=Path, required=True)
    analyze.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "run":
        run_condition(args.condition, args.output, seed=args.seed, steps=args.steps)
    else:
        aggregate(args.directory, args.output)


if __name__ == "__main__":
    main()
