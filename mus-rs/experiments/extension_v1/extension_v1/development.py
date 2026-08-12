"""Controlled path development in the Clifford gesture medium.

For a latent path ``g`` and skew Clifford generators ``D_j``, the medium obeys

    dU = U sum_j D_j dg^j,    U(0) = I.

A piecewise-linear segment is integrated exactly by a matrix exponential.  The
result is a finite-dimensional Lie-group development: a structured quotient of
the full path signature, not a faithful replacement for it.
"""
from __future__ import annotations

import math
from dataclasses import asdict, dataclass
from typing import Iterable, Sequence

import torch
from torch import Tensor, nn

from .clifford import CliffordConfig, clifford_generators, stack_generators, vacuum_state


@dataclass(frozen=True)
class DevelopmentConfig:
    latent_dim: int = 4
    dtype: str = "float64"
    small_loop_min: float = 1e-3
    small_loop_max: float = 2e-1
    small_loop_points: int = 18
    seed: int = 73

    def torch_dtype(self) -> torch.dtype:
        try:
            return getattr(torch, self.dtype)
        except AttributeError as exc:  # pragma: no cover
            raise ValueError(f"unsupported torch dtype: {self.dtype}") from exc


def segment_generator(delta: Tensor, generators: Tensor | Sequence[Tensor]) -> Tensor:
    generator_tensor = (
        generators if isinstance(generators, Tensor) else torch.stack(list(generators))
    )
    if delta.shape[-1] != generator_tensor.shape[0]:
        raise ValueError("path increment dimension does not match generator count")
    return torch.einsum("...d,dij->...ij", delta, generator_tensor)


def segment_operator(delta: Tensor, generators: Tensor | Sequence[Tensor]) -> Tensor:
    return torch.matrix_exp(segment_generator(delta, generators))


def develop_operator(
    increments: Tensor,
    generators: Tensor | Sequence[Tensor],
) -> Tensor:
    """Develop piecewise-linear paths into orthogonal operators.

    ``increments`` may have shape ``[segments, d]`` or ``[..., segments, d]``.
    The returned operator has shape ``[..., state, state]``.
    """

    generator_tensor = (
        generators if isinstance(generators, Tensor) else torch.stack(list(generators))
    )
    state_dim = generator_tensor.shape[-1]
    batch_shape = increments.shape[:-2]
    operator = torch.eye(
        state_dim, dtype=increments.dtype, device=increments.device
    ).expand(*batch_shape, state_dim, state_dim).clone()
    for step in range(increments.shape[-2]):
        operator = operator @ segment_operator(
            increments[..., step, :], generator_tensor
        )
    return operator


def develop_state(
    increments: Tensor,
    generators: Tensor | Sequence[Tensor],
    initial_state: Tensor | None = None,
) -> tuple[Tensor, Tensor]:
    generator_tensor = (
        generators if isinstance(generators, Tensor) else torch.stack(list(generators))
    )
    operator = develop_operator(increments, generator_tensor)
    if initial_state is None:
        initial_state = vacuum_state(
            increments.shape[:-2],
            latent_dim=generator_tensor.shape[0],
            dtype=increments.dtype,
            device=increments.device,
        )
    state = torch.matmul(initial_state.unsqueeze(-2), operator).squeeze(-2)
    return state, operator


def axis_increment(
    latent_dim: int,
    axis: int,
    value: float,
    *,
    dtype: torch.dtype,
) -> Tensor:
    increment = torch.zeros(latent_dim, dtype=dtype)
    increment[axis] = value
    return increment


def rectangle_loop(
    first: float,
    second: float,
    *,
    latent_dim: int = 4,
    axes: tuple[int, int] = (0, 1),
    dtype: torch.dtype = torch.float64,
) -> Tensor:
    if axes[0] == axes[1]:
        raise ValueError("rectangle axes must differ")
    return torch.stack(
        [
            axis_increment(latent_dim, axes[0], first, dtype=dtype),
            axis_increment(latent_dim, axes[1], second, dtype=dtype),
            axis_increment(latent_dim, axes[0], -first, dtype=dtype),
            axis_increment(latent_dim, axes[1], -second, dtype=dtype),
        ]
    )


def inverse_path(increments: Tensor) -> Tensor:
    return -torch.flip(increments, dims=[-2])


def split_segments(increments: Tensor, parts: Sequence[int]) -> Tensor:
    if len(parts) != increments.shape[-2]:
        raise ValueError("one split count is required per segment")
    if any(part < 1 for part in parts):
        raise ValueError("split counts must be positive")
    output: list[Tensor] = []
    for delta, count in zip(increments, parts, strict=True):
        output.extend([delta / count] * count)
    return torch.stack(output)


def blade_mask(*indices: int) -> int:
    mask = 0
    for index in indices:
        mask |= 1 << index
    return mask


class CliffordDevelopmentLayer(nn.Module):
    """Trainable finite-dimensional path-development layer.

    Each fiber learns an orthogonal frame mapping input coordinates into the
    canonical Clifford control coordinates.  The recurrent state itself remains
    exactly norm preserving.  This is a scaffold for sequence experiments, not
    yet an extended string morphology.
    """

    def __init__(
        self,
        input_dim: int,
        *,
        latent_dim: int = 4,
        fibers: int = 1,
        dtype: torch.dtype = torch.float64,
    ) -> None:
        super().__init__()
        if input_dim < 1 or latent_dim < 1 or fibers < 1:
            raise ValueError("input_dim, latent_dim, and fibers must be positive")
        self.input_dim = input_dim
        self.latent_dim = latent_dim
        self.fibers = fibers
        generators = stack_generators(
            CliffordConfig(latent_dim=latent_dim, dtype=str(dtype).split(".")[-1])
        )
        self.register_buffer("generators", generators)
        self.projection = nn.Parameter(
            torch.randn(fibers, input_dim, latent_dim, dtype=dtype)
            / math.sqrt(input_dim)
        )
        self.raw_scale = nn.Parameter(torch.zeros(fibers, dtype=dtype))

    @property
    def state_dim(self) -> int:
        return 1 << self.latent_dim

    def mapped_increments(self, increments: Tensor) -> Tensor:
        mapped = torch.einsum("...td,fdk->...ftk", increments, self.projection)
        scale = torch.nn.functional.softplus(self.raw_scale) + 1e-4
        return mapped * scale.reshape(*([1] * (mapped.ndim - 3)), self.fibers, 1, 1)

    def forward(self, increments: Tensor) -> Tensor:
        if increments.shape[-1] != self.input_dim:
            raise ValueError("input path dimension does not match layer input_dim")
        mapped = self.mapped_increments(increments)
        operator = develop_operator(mapped, self.generators)
        vacuum = torch.zeros(
            *operator.shape[:-2], self.state_dim,
            dtype=operator.dtype,
            device=operator.device,
        )
        vacuum[..., 0] = 1.0
        return torch.matmul(vacuum.unsqueeze(-2), operator).squeeze(-2)


def development_receipt(
    config: DevelopmentConfig = DevelopmentConfig(),
) -> dict[str, object]:
    dtype = config.torch_dtype()
    generators = clifford_generators(
        CliffordConfig(latent_dim=config.latent_dim, dtype=config.dtype)
    )
    generator_tensor = torch.stack(generators)
    identity = torch.eye(1 << config.latent_dim, dtype=dtype)

    first, second = 0.7, 0.45
    path_ab = torch.stack(
        [
            axis_increment(config.latent_dim, 0, first, dtype=dtype),
            axis_increment(config.latent_dim, 1, second, dtype=dtype),
        ]
    )
    path_ba = torch.stack([path_ab[1], path_ab[0]])
    state_ab, operator_ab = develop_state(path_ab, generator_tensor)
    state_ba, operator_ba = develop_state(path_ba, generator_tensor)

    loop = rectangle_loop(
        first,
        second,
        latent_dim=config.latent_dim,
        dtype=dtype,
    )
    loop_state, loop_operator = develop_state(loop, generator_tensor)
    inverse_state, inverse_operator = develop_state(
        inverse_path(loop), generator_tensor
    )
    refined = split_segments(loop, [7, 3, 11, 5])
    refined_state, refined_operator = develop_state(refined, generator_tensor)

    scales = torch.logspace(
        math.log10(config.small_loop_min),
        math.log10(config.small_loop_max),
        config.small_loop_points,
        dtype=dtype,
    )
    area_rows: list[dict[str, float]] = []
    bivector_index = blade_mask(0, 1)
    for scale in scales:
        small_loop = rectangle_loop(
            float(scale),
            float(0.73 * scale),
            latent_dim=config.latent_dim,
            dtype=dtype,
        )
        state, operator = develop_state(small_loop, generator_tensor)
        area = float(scale * (0.73 * scale))
        area_rows.append(
            {
                "scale": float(scale),
                "signed_area": area,
                "bivector_01": float(state[bivector_index]),
                "operator_defect_frobenius": float(
                    torch.linalg.matrix_norm(operator - identity)
                ),
            }
        )
    log_area = torch.log(
        torch.tensor([row["signed_area"] for row in area_rows], dtype=dtype)
    )
    log_residue = torch.log(
        torch.tensor(
            [abs(row["bivector_01"]) for row in area_rows], dtype=dtype
        ).clamp_min(1e-30)
    )
    design = torch.stack([log_area, torch.ones_like(log_area)], dim=1)
    solution = torch.linalg.lstsq(design, log_residue).solution

    return {
        "model": "controlled-clifford-development",
        "config": asdict(config),
        "same_endpoint_order_contrast": {
            "endpoint": [first, second] + [0.0] * (config.latent_dim - 2),
            "state_l2": float((state_ab - state_ba).norm()),
            "operator_frobenius": float(
                torch.linalg.matrix_norm(operator_ab - operator_ba)
            ),
            "ab_energy": float(state_ab.square().sum()),
            "ba_energy": float(state_ba.square().sum()),
        },
        "closed_rectangle": {
            "control_endpoint_norm": float(loop.sum(dim=0).norm()),
            "state_distance_from_vacuum": float(
                (loop_state - identity[0]).norm()
            ),
            "operator_distance_from_identity": float(
                torch.linalg.matrix_norm(loop_operator - identity)
            ),
            "bivector_01": float(loop_state[bivector_index]),
            "energy": float(loop_state.square().sum()),
        },
        "inverse_loop": {
            "operator_inverse_residual": float(
                torch.linalg.matrix_norm(
                    loop_operator @ inverse_operator - identity
                )
            ),
            "state_roundtrip_residual": float(
                (loop_state @ inverse_operator - identity[0]).norm()
            ),
            "inverse_state_energy": float(inverse_state.square().sum()),
        },
        "reparameterization": {
            "coarse_segments": int(loop.shape[0]),
            "refined_segments": int(refined.shape[0]),
            "state_residual": float((loop_state - refined_state).norm()),
            "operator_residual": float(
                torch.linalg.matrix_norm(loop_operator - refined_operator)
            ),
        },
        "small_loop_area_law": {
            "bivector_abs_vs_area_loglog_exponent": float(solution[0]),
            "fitted_coefficient": float(torch.exp(solution[1])),
            "rows": area_rows,
        },
        "claim_discipline": {
            "closed_loop_residue_is_nonzero": True,
            "this_is_a_topologically_protected_invariant": False,
            "piecewise_linear_reparameterization_invariance_tested": True,
            "state_is_a_finite_clifford_quotient_of_the_tensor_signature": True,
            "one_fixed_quotient_is_signature_universal": False,
        },
    }
