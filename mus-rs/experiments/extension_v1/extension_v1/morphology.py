"""Inverse-design extended-string morphology for a target Clifford program.

Only the piecewise-linear shapes of five grade strings are optimized.  The
sparse signed quarter-turn schedule remains fixed.  This separates two claims:

* the target operator algebra is known exactly;
* morphology can be trained so local finite-width contacts approximate it.

The optimization is an oracle experiment, not autonomous topology discovery.
"""
from __future__ import annotations

import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Sequence

import torch
from torch import Tensor, nn

from .clifford import (
    CliffordConfig,
    algebra_checks,
    clifford_generators,
    continuous_volume_experiment,
    masks_by_grade,
    permutation_experiment,
)
from .geometry import GeometryConfig, soft_generators


@dataclass(frozen=True)
class MorphologyConfig:
    ambient_dim: int = 4
    latent_dim: int = 4
    seed: int = 41
    kernel_width: float = 0.50
    segment_length: float = 1.0
    layer_separation: float = 2.6
    train_steps: int = 200
    learning_rate: float = 0.035
    curvature_weight: float = 0.0015
    direction_isotropy_weight: float = 0.0002
    cpu_threads: int = 1
    eval_trials: int = 2048
    dtype: str = "float32"

    def torch_dtype(self) -> torch.dtype:
        try:
            return getattr(torch, self.dtype)
        except AttributeError as exc:  # pragma: no cover
            raise ValueError(f"unsupported torch dtype: {self.dtype}") from exc


class GradeStringMorphology(nn.Module):
    """Five learnable piecewise-linear strings, one per exterior grade."""

    def __init__(self, config: MorphologyConfig) -> None:
        super().__init__()
        self.config = config
        self.groups = masks_by_grade(config.latent_dim)
        generator = torch.Generator().manual_seed(config.seed)
        dtype = config.torch_dtype()
        self.raw_directions = nn.ParameterList()
        for grade in range(config.latent_dim + 1):
            count = len(self.groups[grade])
            directions = torch.randn(
                max(0, count - 1),
                config.ambient_dim,
                generator=generator,
                dtype=dtype,
            )
            if directions.numel():
                directions[:, 0] += 0.55
            self.raw_directions.append(nn.Parameter(directions))
        anchors = torch.zeros(
            config.latent_dim + 1, config.ambient_dim, dtype=dtype
        )
        anchors[:, 0] = (
            torch.arange(config.latent_dim + 1, dtype=dtype)
            * config.layer_separation
        )
        if config.ambient_dim > 1:
            anchors[:, 1] = torch.linspace(
                -0.65, 0.65, config.latent_dim + 1, dtype=dtype
            )
        self.register_buffer("anchors", anchors)

    def coordinates(self) -> Tensor:
        config = self.config
        state_dim = 1 << config.latent_dim
        coordinates = torch.empty(
            state_dim,
            config.ambient_dim,
            dtype=self.anchors.dtype,
            device=self.anchors.device,
        )
        for grade in range(config.latent_dim + 1):
            masks = self.groups[grade]
            coordinates[masks[0]] = self.anchors[grade]
            raw = self.raw_directions[grade]
            if raw.numel():
                directions = raw / raw.norm(
                    dim=-1, keepdim=True
                ).clamp_min(1e-8)
                points = self.anchors[grade] + config.segment_length * torch.cumsum(
                    directions, dim=0
                )
                indices = torch.tensor(masks[1:], device=coordinates.device)
                coordinates[indices] = points
        return coordinates

    def curvature(self) -> Tensor:
        penalties: list[Tensor] = []
        for raw in self.raw_directions:
            if raw.shape[0] >= 2:
                directions = raw / raw.norm(
                    dim=-1, keepdim=True
                ).clamp_min(1e-8)
                penalties.append(
                    (directions[1:] - directions[:-1])
                    .square()
                    .sum(-1)
                    .mean()
                )
        if not penalties:
            return torch.zeros((), dtype=self.anchors.dtype, device=self.anchors.device)
        return torch.stack(penalties).mean()

    def isotropy(self) -> Tensor:
        directions = [
            raw / raw.norm(dim=-1, keepdim=True).clamp_min(1e-8)
            for raw in self.raw_directions
            if raw.numel()
        ]
        concatenated = torch.cat(directions)
        covariance = concatenated.T @ concatenated / concatenated.shape[0]
        target = torch.eye(
            self.config.ambient_dim,
            dtype=concatenated.dtype,
            device=concatenated.device,
        ) / self.config.ambient_dim
        return (covariance - target).square().mean()


def operator_mse(realized: Sequence[Tensor], exact: Sequence[Tensor]) -> Tensor:
    return torch.stack(
        [
            (first - second).square().mean()
            for first, second in zip(realized, exact, strict=True)
        ]
    ).mean()


def evaluate_morphology(
    config: MorphologyConfig,
    coordinates: Tensor,
) -> dict[str, object]:
    geometry_config = GeometryConfig(
        ambient_dim=config.ambient_dim,
        latent_dim=config.latent_dim,
        kernel_width=config.kernel_width,
        seed=config.seed,
        segment_length=config.segment_length,
        layer_separation=config.layer_separation,
        dtype=config.dtype,
        random_trials=config.eval_trials,
    )
    realized = soft_generators(geometry_config, coordinates)
    clifford_config = CliffordConfig(
        latent_dim=config.latent_dim,
        dtype=config.dtype,
        random_trials=config.eval_trials,
        seed=config.seed,
    )
    exact = clifford_generators(clifford_config)
    return {
        "algebra": algebra_checks(clifford_config, realized),
        "operator_mse": float(operator_mse(realized, exact)),
        "parity": permutation_experiment(clifford_config, realized),
        "volume": continuous_volume_experiment(clifford_config, realized),
    }


def optimize_morphology(
    config: MorphologyConfig = MorphologyConfig(),
    *,
    output_dir: Path | None = None,
    save_checkpoint: bool = True,
) -> tuple[GradeStringMorphology, dict[str, object]]:
    torch.manual_seed(config.seed)
    torch.set_num_threads(config.cpu_threads)
    model = GradeStringMorphology(config)
    clifford_config = CliffordConfig(
        latent_dim=config.latent_dim,
        dtype=config.dtype,
        random_trials=config.eval_trials,
        seed=config.seed,
    )
    exact = clifford_generators(clifford_config)
    geometry_config = GeometryConfig(
        ambient_dim=config.ambient_dim,
        latent_dim=config.latent_dim,
        kernel_width=config.kernel_width,
        seed=config.seed,
        segment_length=config.segment_length,
        layer_separation=config.layer_separation,
        dtype=config.dtype,
        random_trials=config.eval_trials,
    )
    optimizer = torch.optim.Adam(model.parameters(), lr=config.learning_rate)
    scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(
        optimizer, T_max=config.train_steps
    )
    trace: list[dict[str, float | int]] = []
    best = float("inf")
    best_state: dict[str, Tensor] | None = None
    start = time.perf_counter()
    with torch.no_grad():
        initial = evaluate_morphology(config, model.coordinates())

    for step in range(1, config.train_steps + 1):
        coordinates = model.coordinates()
        realized = soft_generators(geometry_config, coordinates)
        operator_loss = operator_mse(realized, exact)
        curvature = model.curvature()
        isotropy = model.isotropy()
        loss = (
            operator_loss
            + config.curvature_weight * curvature
            + config.direction_isotropy_weight * isotropy
        )
        optimizer.zero_grad(set_to_none=True)
        loss.backward()
        nn.utils.clip_grad_norm_(model.parameters(), 5.0)
        optimizer.step()
        scheduler.step()
        operator_value = float(operator_loss.detach())
        if operator_value < best:
            best = operator_value
            best_state = {
                name: value.detach().clone()
                for name, value in model.state_dict().items()
            }
        if step == 1 or step % 25 == 0 or step == config.train_steps:
            with torch.no_grad():
                evaluation = evaluate_morphology(config, model.coordinates())
            trace.append(
                {
                    "step": step,
                    "operator_mse": operator_value,
                    "curvature": float(curvature.detach()),
                    "anticommutator_residual": evaluation["algebra"][
                        "max_anticommutator_residual"
                    ],
                    "parity_sign_accuracy": evaluation["parity"][
                        "sign_accuracy"
                    ],
                    "minimum_parity_amplitude": evaluation["parity"][
                        "minimum_abs_amplitude"
                    ],
                    "volume_sign_accuracy": evaluation["volume"][
                        "sign_accuracy"
                    ],
                    "volume_rms_residual": evaluation["volume"][
                        "rms_volume_residual"
                    ],
                }
            )

    if best_state is not None:
        model.load_state_dict(best_state)
    coordinates = model.coordinates().detach()
    final = evaluate_morphology(config, coordinates)
    receipt = {
        "model": "inverse-designed-clifford-grade-strings",
        "config": asdict(config),
        "trainable_parameters": sum(
            parameter.numel() for parameter in model.parameters()
        ),
        "initial": initial,
        "final": final,
        "best_operator_mse": best,
        "elapsed_seconds": time.perf_counter() - start,
        "coordinates": coordinates.tolist(),
        "trace": trace,
        "claim_discipline": {
            "target_clifford_program_is_known": True,
            "only_morphology_is_optimized": True,
            "topology_is_not_discovered": True,
            "continuous_volume_test_uses_out_of_sample_random_vectors": True,
        },
    }
    if output_dir is not None:
        import json

        output_dir.mkdir(parents=True, exist_ok=True)
        (output_dir / "result.json").write_text(
            json.dumps(receipt, indent=2, sort_keys=True) + "\n"
        )
        if save_checkpoint:
            torch.save(
                {"config": asdict(config), "coordinates": coordinates},
                output_dir / "morphology.pt",
            )
    return model, receipt
