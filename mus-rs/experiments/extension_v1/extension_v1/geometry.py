"""Geometric realization of Clifford operators on extended one-dimensional bodies.

The 16 exterior-basis modes of ``Cl(0,4)`` are placed on five strings, one per
grade.  A local event is an exactly orthogonal rotation between two Gaussian
field modes centred at material sites.  Finite Gaussian width causes geometric
crosstalk but never numerical gain: Gram--Schmidt creates an orthonormal local
plane before every event.
"""
from __future__ import annotations

import math
from dataclasses import asdict, dataclass
from typing import Sequence

import torch
from torch import Tensor

from .clifford import (
    CliffordConfig,
    algebra_checks,
    clifford_generators,
    continuous_volume_experiment,
    masks_by_grade,
    permutation_experiment,
    right_wedge_sign,
)


@dataclass(frozen=True)
class GeometryConfig:
    ambient_dim: int = 4
    latent_dim: int = 4
    seed: int = 31
    segment_length: float = 1.0
    layer_separation: float = 2.6
    kernel_width: float = 0.28
    dtype: str = "float64"
    random_trials: int = 2048

    def torch_dtype(self) -> torch.dtype:
        try:
            return getattr(torch, self.dtype)
        except AttributeError as exc:  # pragma: no cover
            raise ValueError(f"unsupported torch dtype: {self.dtype}") from exc


def soft_plane_rotate(
    state: Tensor,
    coordinates: Tensor,
    source: int,
    target: int,
    theta: float | Tensor,
    width: float,
) -> Tensor:
    """Rotate between two local Gaussian field modes.

    ``state`` may be a state vector or a stack of row states.  The operation is
    exactly orthogonal in the local two-plane even when the lobes overlap.
    """

    if width <= 0:
        raise ValueError("kernel width must be positive")
    epsilon = torch.finfo(state.dtype).eps
    distance_source = (coordinates - coordinates[source]).square().sum(-1)
    distance_target = (coordinates - coordinates[target]).square().sum(-1)
    first = torch.exp(-distance_source / (2 * width * width))
    second = torch.exp(-distance_target / (2 * width * width))
    first = first / first.norm().clamp_min(epsilon)
    second = second - torch.dot(first, second) * first
    second = second / second.norm().clamp_min(epsilon)

    coefficient_first = state @ first
    coefficient_second = state @ second
    angle = torch.as_tensor(theta, dtype=state.dtype, device=state.device)
    cosine = torch.cos(angle)
    sine = torch.sin(angle)
    delta_first = (cosine - 1.0) * coefficient_first - sine * coefficient_second
    delta_second = sine * coefficient_first + (cosine - 1.0) * coefficient_second
    return (
        state
        + delta_first[..., None] * first
        + delta_second[..., None] * second
    )


def random_grade_string_geometry(config: GeometryConfig) -> Tensor:
    """Create one piecewise-linear material string for every exterior grade."""

    dtype = config.torch_dtype()
    generator = torch.Generator().manual_seed(config.seed)
    coordinates = torch.empty(
        1 << config.latent_dim, config.ambient_dim, dtype=dtype
    )
    groups = masks_by_grade(config.latent_dim)
    for grade, masks in groups.items():
        base = torch.zeros(config.ambient_dim, dtype=dtype)
        base[0] = grade * config.layer_separation
        if config.ambient_dim > 1:
            base[1:] = (
                torch.randn(
                    config.ambient_dim - 1,
                    generator=generator,
                    dtype=dtype,
                )
                * 0.25
            )
        point = base
        direction = torch.randn(
            config.ambient_dim, generator=generator, dtype=dtype
        )
        direction = direction / direction.norm().clamp_min(1e-12)
        for local_index, mask in enumerate(masks):
            if local_index:
                noise = torch.randn(
                    config.ambient_dim, generator=generator, dtype=dtype
                )
                noise = noise / noise.norm().clamp_min(1e-12)
                direction = 0.72 * direction + 0.28 * noise
                direction = direction / direction.norm().clamp_min(1e-12)
                point = point + config.segment_length * direction
            coordinates[mask] = point
    return coordinates


def soft_generators(
    config: GeometryConfig,
    coordinates: Tensor,
) -> list[Tensor]:
    """Lower the exact Clifford pairing schedule onto local Gaussian events."""

    state_dim = 1 << config.latent_dim
    if coordinates.shape != (state_dim, config.ambient_dim):
        raise ValueError(
            f"expected coordinates {(state_dim, config.ambient_dim)}, "
            f"received {tuple(coordinates.shape)}"
        )
    identity = torch.eye(
        state_dim, dtype=coordinates.dtype, device=coordinates.device
    )
    generators: list[Tensor] = []
    for axis in range(config.latent_dim):
        matrix = identity
        for mask in range(state_dim):
            if mask & (1 << axis):
                continue
            paired = mask | (1 << axis)
            sign = right_wedge_sign(mask, axis)
            matrix = soft_plane_rotate(
                matrix,
                coordinates,
                mask,
                paired,
                sign * math.pi / 2,
                config.kernel_width,
            )
        generators.append(matrix)
    return generators


def generator_errors(
    realized: Sequence[Tensor], exact: Sequence[Tensor]
) -> dict[str, float]:
    errors = torch.tensor(
        [
            torch.linalg.matrix_norm(first - second).item()
            for first, second in zip(realized, exact, strict=True)
        ]
    )
    return {
        "mean_generator_frobenius_error": float(errors.mean()),
        "maximum_generator_frobenius_error": float(errors.max()),
    }


def geometry_receipt(config: GeometryConfig = GeometryConfig()) -> dict[str, object]:
    coordinates = random_grade_string_geometry(config)
    realized = soft_generators(config, coordinates)
    clifford_config = CliffordConfig(
        latent_dim=config.latent_dim,
        dtype=config.dtype,
        random_trials=config.random_trials,
        seed=config.seed,
    )
    exact = clifford_generators(clifford_config)
    return {
        "model": "geometric-clifford-gesture-medium",
        "config": asdict(config),
        "state_dimension": 1 << config.latent_dim,
        "string_site_counts": [
            math.comb(config.latent_dim, grade)
            for grade in range(config.latent_dim + 1)
        ],
        "algebra": algebra_checks(clifford_config, realized),
        "operator_error": generator_errors(realized, exact),
        "permutation_parity": permutation_experiment(
            clifford_config, realized
        ),
        "continuous_oriented_volume": continuous_volume_experiment(
            clifford_config, realized
        ),
        "coordinates": coordinates.tolist(),
        "claim_discipline": {
            "each_local_event_is_exactly_energy_preserving": True,
            "finite_width_breaks_exact_clifford_relations": True,
            "ambient_four_dimensions_are_uniquely_optimal": False,
        },
    }


def dimension_sweep(
    *,
    ambient_dims: Sequence[int] = (2, 3, 4, 8),
    widths: Sequence[float] = (0.12, 0.20, 0.28, 0.38, 0.50, 0.65),
    seeds: Sequence[int] = tuple(range(5)),
    random_trials: int = 512,
) -> dict[str, object]:
    rows: list[dict[str, float | int]] = []
    for ambient_dim in ambient_dims:
        for width in widths:
            for seed_index in seeds:
                config = GeometryConfig(
                    ambient_dim=ambient_dim,
                    kernel_width=width,
                    seed=200 + seed_index,
                    random_trials=random_trials,
                )
                result = geometry_receipt(config)
                rows.append(
                    {
                        "ambient_dim": ambient_dim,
                        "kernel_width": width,
                        "seed": seed_index,
                        "parity_accuracy": result["permutation_parity"][
                            "sign_accuracy"
                        ],
                        "minimum_parity_amplitude": result[
                            "permutation_parity"
                        ]["minimum_abs_amplitude"],
                        "volume_sign_accuracy": result[
                            "continuous_oriented_volume"
                        ]["sign_accuracy"],
                        "rms_volume_residual": result[
                            "continuous_oriented_volume"
                        ]["rms_volume_residual"],
                        "anticommutator_residual": result["algebra"][
                            "max_anticommutator_residual"
                        ],
                        "generator_error": result["operator_error"][
                            "mean_generator_frobenius_error"
                        ],
                    }
                )
    summary: list[dict[str, float | int]] = []
    for ambient_dim in ambient_dims:
        for width in widths:
            group = [
                row
                for row in rows
                if row["ambient_dim"] == ambient_dim
                and row["kernel_width"] == width
            ]
            summary.append(
                {
                    "ambient_dim": ambient_dim,
                    "kernel_width": width,
                    "mean_parity_accuracy": sum(
                        float(row["parity_accuracy"]) for row in group
                    )
                    / len(group),
                    "mean_volume_sign_accuracy": sum(
                        float(row["volume_sign_accuracy"]) for row in group
                    )
                    / len(group),
                    "mean_rms_volume_residual": sum(
                        float(row["rms_volume_residual"]) for row in group
                    )
                    / len(group),
                    "mean_anticommutator_residual": sum(
                        float(row["anticommutator_residual"]) for row in group
                    )
                    / len(group),
                    "mean_generator_error": sum(
                        float(row["generator_error"]) for row in group
                    )
                    / len(group),
                    "worst_parity_amplitude": min(
                        float(row["minimum_parity_amplitude"])
                        for row in group
                    ),
                }
            )
    return {"rows": rows, "summary": summary}
