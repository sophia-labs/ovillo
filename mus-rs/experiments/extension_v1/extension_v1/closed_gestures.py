"""Train closed token gestures inside an energy-preserving Clifford medium.

Every token owns a piecewise-linear control loop whose increments sum to zero
exactly.  Noncommuting Clifford development nevertheless leaves a residual
operator.  Token operators compose recurrently; a linear pickup reads the final
multivector state.

The complete four-token permutation-parity task is deliberately tiny.  It is a
controlled proof that same-endpoint gesture history can carry sequence order,
not evidence of language modelling or algorithmic length generalisation.
"""
from __future__ import annotations

import itertools
import math
import random
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Sequence

import torch
from torch import Tensor, nn
import torch.nn.functional as F

from .clifford import CliffordConfig, permutation_parity, stack_generators
from .development import develop_operator


@dataclass(frozen=True)
class ClosedGestureConfig:
    seed: int = 101
    token_count: int = 4
    latent_dim: int = 4
    segments: int = 6
    fibers: int = 1
    train_steps: int = 600
    learning_rate: float = 0.02
    dtype: str = "float64"
    trace_every: int = 100
    loop_scale: float = 0.45
    area_regularization: float = 1e-4
    length_regularization: float = 2e-4
    cpu_threads: int = 1

    def torch_dtype(self) -> torch.dtype:
        try:
            return getattr(torch, self.dtype)
        except AttributeError as exc:  # pragma: no cover
            raise ValueError(f"unsupported torch dtype: {self.dtype}") from exc


def permutation_dataset(token_count: int = 4) -> tuple[Tensor, Tensor]:
    permutations = list(itertools.permutations(range(token_count)))
    tokens = torch.tensor(permutations, dtype=torch.long)
    labels = torch.tensor(
        [permutation_parity(permutation) for permutation in permutations],
        dtype=torch.long,
    )
    return tokens, labels


class ClosedCliffordGestureModel(nn.Module):
    """One or more norm-preserving Clifford fibers driven by closed loops."""

    def __init__(self, config: ClosedGestureConfig) -> None:
        super().__init__()
        if config.segments < 3:
            raise ValueError("closed gestures require at least three segments")
        self.config = config
        dtype = config.torch_dtype()
        generators = stack_generators(
            CliffordConfig(latent_dim=config.latent_dim, dtype=config.dtype)
        )
        self.register_buffer("generators", generators)

        # First K-1 increments are free; the final increment closes the loop.
        self.free_increments = nn.Parameter(
            torch.randn(
                config.fibers,
                config.token_count,
                config.segments - 1,
                config.latent_dim,
                dtype=dtype,
            )
            * config.loop_scale
        )
        identity = torch.eye(config.latent_dim, dtype=dtype)
        self.frame_raw = nn.Parameter(
            identity.repeat(config.fibers, 1, 1)
            + 0.03
            * torch.randn(
                config.fibers,
                config.latent_dim,
                config.latent_dim,
                dtype=dtype,
            )
        )
        state_dim = 1 << config.latent_dim
        self.readout = nn.Linear(config.fibers * state_dim, 2, dtype=dtype)

    @property
    def state_dim(self) -> int:
        return 1 << self.config.latent_dim

    def frames(self) -> Tensor:
        """Return per-fiber orthogonal coordinate frames."""

        q, r = torch.linalg.qr(self.frame_raw)
        diagonal = torch.diagonal(r, dim1=-2, dim2=-1)
        signs = torch.where(diagonal < 0, -torch.ones_like(diagonal), torch.ones_like(diagonal))
        return q * signs[:, None, :]

    def loops(self) -> Tensor:
        final_increment = -self.free_increments.sum(dim=-2, keepdim=True)
        increments = torch.cat([self.free_increments, final_increment], dim=-2)
        return torch.einsum("ftkd,fde->ftke", increments, self.frames())

    def token_operators(self) -> Tensor:
        loops = self.loops()
        flat = loops.reshape(
            self.config.fibers * self.config.token_count,
            self.config.segments,
            self.config.latent_dim,
        )
        operators = develop_operator(flat, self.generators)
        return operators.reshape(
            self.config.fibers,
            self.config.token_count,
            self.state_dim,
            self.state_dim,
        )

    def evolve(self, tokens: Tensor) -> Tensor:
        if tokens.ndim != 2:
            raise ValueError("tokens must have shape [batch, sequence]")
        operators = self.token_operators()
        batch, sequence = tokens.shape
        state = torch.zeros(
            batch,
            self.config.fibers,
            self.state_dim,
            dtype=operators.dtype,
            device=operators.device,
        )
        state[:, :, 0] = 1.0
        for position in range(sequence):
            selected = operators[:, tokens[:, position]].permute(1, 0, 2, 3)
            state = torch.einsum("bfi,bfij->bfj", state, selected)
        return state

    def forward(self, tokens: Tensor) -> tuple[Tensor, Tensor]:
        state = self.evolve(tokens)
        logits = self.readout(state.reshape(tokens.shape[0], -1))
        return logits, state

    def regularization(self) -> Tensor:
        loops = self.loops()
        length = loops.norm(dim=-1).sum(dim=-1)
        vertices = torch.cumsum(loops, dim=-2)
        previous = torch.cat(
            [torch.zeros_like(vertices[..., :1, :]), vertices[..., :-1, :]],
            dim=-2,
        )
        signed_areas: list[Tensor] = []
        for first in range(self.config.latent_dim):
            for second in range(first + 1, self.config.latent_dim):
                signed_areas.append(
                    0.5
                    * (
                        previous[..., first] * vertices[..., second]
                        - previous[..., second] * vertices[..., first]
                    ).sum(dim=-1)
                )
        area = torch.stack(signed_areas, dim=-1)
        length_penalty = self.config.length_regularization * (
            length - 2.5
        ).square().mean()
        noncollapse_penalty = self.config.area_regularization / (
            area.square().sum(dim=-1).mean() + 1e-3
        )
        return length_penalty + noncollapse_penalty


def _model_receipt(
    model: ClosedCliffordGestureModel,
    tokens: Tensor,
    labels: Tensor,
    *,
    best_accuracy: float,
    elapsed_seconds: float,
    trace: list[dict[str, float | int]],
) -> dict[str, object]:
    with torch.no_grad():
        logits, state = model(tokens)
        accuracy = float((logits.argmax(-1) == labels).double().mean())
        loops = model.loops()
        operators = model.token_operators()
        commutators: list[float] = []
        for first in range(model.config.token_count):
            for second in range(first + 1, model.config.token_count):
                residual = (
                    operators[:, first] @ operators[:, second]
                    - operators[:, second] @ operators[:, first]
                )
                commutators.append(
                    float(
                        torch.linalg.matrix_norm(
                            residual, dim=(-2, -1)
                        ).mean()
                    )
                )
    return {
        "model": "closed-clifford-token-gestures",
        "config": asdict(model.config),
        "accuracy": accuracy,
        "best_accuracy": best_accuracy,
        "parameter_count": sum(parameter.numel() for parameter in model.parameters()),
        "elapsed_seconds": elapsed_seconds,
        "max_loop_closure": float(loops.sum(dim=-2).abs().max()),
        "max_energy_drift": float(
            (state.square().sum(-1) - 1.0).abs().max()
        ),
        "mean_pairwise_commutator_frobenius": sum(commutators)
        / len(commutators),
        "trace": trace,
        "loops": loops.tolist(),
        "claim_discipline": {
            "all_token_control_paths_closed": True,
            "all_sequences_share_the_same_token_multiset": True,
            "operators_energy_preserving": True,
            "this_is_a_language_model": False,
            "training_set_is_the_complete_finite_task": True,
            "this_establishes_length_generalisation": False,
        },
    }


def train_closed_gestures(
    config: ClosedGestureConfig = ClosedGestureConfig(),
    *,
    output_dir: Path | None = None,
    save_checkpoint: bool = True,
) -> tuple[ClosedCliffordGestureModel, dict[str, object]]:
    """Fit closed token loops on complete permutation parity."""

    random.seed(config.seed)
    torch.manual_seed(config.seed)
    torch.set_num_threads(config.cpu_threads)
    tokens, labels = permutation_dataset(config.token_count)
    model = ClosedCliffordGestureModel(config)
    optimizer = torch.optim.AdamW(
        model.parameters(), lr=config.learning_rate, weight_decay=1e-5
    )
    scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(
        optimizer, T_max=config.train_steps
    )
    best_accuracy = 0.0
    best_state: dict[str, Tensor] | None = None
    trace: list[dict[str, float | int]] = []
    start = time.perf_counter()

    for step in range(1, config.train_steps + 1):
        logits, _ = model(tokens)
        loss = F.cross_entropy(logits, labels) + model.regularization()
        if not torch.isfinite(loss):
            raise RuntimeError("non-finite closed-gesture training loss")
        optimizer.zero_grad(set_to_none=True)
        loss.backward()
        nn.utils.clip_grad_norm_(model.parameters(), 5.0)
        optimizer.step()
        scheduler.step()

        if (
            step == 1
            or step % config.trace_every == 0
            or step == config.train_steps
        ):
            with torch.no_grad():
                evaluation_logits, state = model(tokens)
                accuracy = float(
                    (evaluation_logits.argmax(-1) == labels).double().mean()
                )
                margin = float(
                    (evaluation_logits[:, 1] - evaluation_logits[:, 0])
                    .abs()
                    .min()
                )
                loop_closure = float(model.loops().sum(dim=-2).abs().max())
                energy_drift = float(
                    (state.square().sum(-1) - 1.0).abs().max()
                )
            trace.append(
                {
                    "step": step,
                    "loss": float(loss.detach()),
                    "accuracy": accuracy,
                    "min_logit_margin": margin,
                    "loop_closure_max": loop_closure,
                    "max_energy_drift": energy_drift,
                }
            )
            if accuracy > best_accuracy:
                best_accuracy = accuracy
                best_state = {
                    name: value.detach().clone()
                    for name, value in model.state_dict().items()
                }
            if (
                accuracy == 1.0
                and margin > 1.0
                and len(trace) > 1
                and trace[-2]["accuracy"] == 1.0
            ):
                break

    if best_state is not None:
        model.load_state_dict(best_state)
    receipt = _model_receipt(
        model,
        tokens,
        labels,
        best_accuracy=best_accuracy,
        elapsed_seconds=time.perf_counter() - start,
        trace=trace,
    )
    if output_dir is not None:
        output_dir.mkdir(parents=True, exist_ok=True)
        import json

        (output_dir / "result.json").write_text(
            json.dumps(receipt, indent=2, sort_keys=True) + "\n"
        )
        if save_checkpoint:
            torch.save(
                {"config": asdict(config), "state_dict": model.state_dict()},
                output_dir / "model.pt",
            )
    return model, receipt


def _sequence_states(operators: Tensor) -> tuple[list[tuple[int, ...]], Tensor]:
    token_count = operators.shape[1]
    state_dim = operators.shape[-1]
    permutations = list(itertools.permutations(range(token_count)))
    states: list[Tensor] = []
    for permutation in permutations:
        state = torch.zeros(
            operators.shape[0], state_dim, dtype=operators.dtype
        )
        state[:, 0] = 1.0
        for token in permutation:
            state = torch.einsum(
                "fi,fij->fj", state, operators[:, token]
            )
        states.append(state.reshape(-1))
    return permutations, torch.stack(states)


def _linear_probe_accuracy(features: Tensor, labels: Tensor) -> tuple[float, int]:
    design = torch.cat(
        [features, torch.ones(features.shape[0], 1, dtype=features.dtype)],
        dim=1,
    )
    target = labels.to(features.dtype) * 2 - 1
    weights = torch.linalg.lstsq(design, target).solution
    prediction = (design @ weights >= 0).long()
    rank = int(torch.linalg.matrix_rank(features - features.mean(0)))
    return float((prediction == labels).double().mean()), rank


def commuting_control(
    model: ClosedCliffordGestureModel,
) -> dict[str, object]:
    """Collapse all latent generators to one commuting generator.

    Because each token path closes, the commuting development becomes identity.
    The noncommuting Clifford medium retains full parity-separating features.
    """

    with torch.no_grad():
        loops = model.loops()
        exact_generators = model.generators
        commuting_generators = exact_generators[0:1].repeat(
            model.config.latent_dim, 1, 1
        )
        flat = loops.reshape(
            model.config.fibers * model.config.token_count,
            model.config.segments,
            model.config.latent_dim,
        )
        exact_operators = develop_operator(flat, exact_generators).reshape(
            model.config.fibers,
            model.config.token_count,
            model.state_dim,
            model.state_dim,
        )
        commuting_operators = develop_operator(
            flat, commuting_generators
        ).reshape_as(exact_operators)
        permutations, exact_states = _sequence_states(exact_operators)
        _, commuting_states = _sequence_states(commuting_operators)
        labels = torch.tensor(
            [permutation_parity(permutation) for permutation in permutations]
        )
        exact_accuracy, exact_rank = _linear_probe_accuracy(
            exact_states, labels
        )
        commuting_accuracy, commuting_rank = _linear_probe_accuracy(
            commuting_states, labels
        )
        identity = torch.eye(model.state_dim, dtype=loops.dtype)
    return {
        "model": "closed-gesture-commuting-control",
        "examples": len(permutations),
        "all_sequences_same_token_multiset": True,
        "all_loop_endpoints_zero": float(loops.sum(dim=-2).abs().max())
        < 1e-12,
        "clifford_noncommuting": {
            "linear_probe_accuracy": exact_accuracy,
            "centered_feature_rank": exact_rank,
            "maximum_token_operator_identity_distance": float(
                torch.linalg.matrix_norm(
                    exact_operators - identity, dim=(-2, -1)
                ).max()
            ),
        },
        "commuting_collapse": {
            "linear_probe_accuracy": commuting_accuracy,
            "centered_feature_rank": commuting_rank,
            "maximum_token_operator_identity_distance": float(
                torch.linalg.matrix_norm(
                    commuting_operators - identity, dim=(-2, -1)
                ).max()
            ),
            "maximum_pairwise_state_distance": float(
                torch.cdist(commuting_states, commuting_states).max()
            ),
        },
        "endpoint_only_accuracy_ceiling": 0.5,
        "bag_of_tokens_accuracy_ceiling": 0.5,
        "claim_discipline": {
            "noncommutation_is_necessary_for_this_closed_loop_realization": True,
            "clifford_is_the_only_possible_noncommuting_solution": False,
        },
    }


def seed_sweep(
    *,
    fibers: Sequence[int] = (1, 2, 4),
    seeds: Sequence[int] = (7, 31, 101, 211, 307),
    train_steps: int = 600,
) -> dict[str, object]:
    rows: list[dict[str, float | int]] = []
    for fiber_count in fibers:
        for seed in seeds:
            _, result = train_closed_gestures(
                ClosedGestureConfig(
                    seed=seed,
                    fibers=fiber_count,
                    train_steps=train_steps,
                ),
                save_checkpoint=False,
            )
            rows.append(
                {
                    "fibers": fiber_count,
                    "seed": seed,
                    "accuracy": result["accuracy"],
                    "elapsed_seconds": result["elapsed_seconds"],
                    "max_loop_closure": result["max_loop_closure"],
                    "max_energy_drift": result["max_energy_drift"],
                    "mean_commutator": result[
                        "mean_pairwise_commutator_frobenius"
                    ],
                    "parameter_count": result["parameter_count"],
                }
            )
    summary: list[dict[str, float | int]] = []
    for fiber_count in fibers:
        group = [row for row in rows if row["fibers"] == fiber_count]
        summary.append(
            {
                "fibers": fiber_count,
                "successes": sum(row["accuracy"] == 1.0 for row in group),
                "runs": len(group),
                "mean_accuracy": sum(
                    float(row["accuracy"]) for row in group
                )
                / len(group),
                "mean_elapsed_seconds": sum(
                    float(row["elapsed_seconds"]) for row in group
                )
                / len(group),
                "max_loop_closure": max(
                    float(row["max_loop_closure"]) for row in group
                ),
                "max_energy_drift": max(
                    float(row["max_energy_drift"]) for row in group
                ),
                "mean_commutator": sum(
                    float(row["mean_commutator"]) for row in group
                )
                / len(group),
                "parameter_count": group[0]["parameter_count"],
            }
        )
    return {"rows": rows, "summary": summary}
