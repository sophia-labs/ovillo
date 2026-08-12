"""Exact Clifford gesture medium.

The state space is the exterior algebra ``Lambda* R^d`` with one coordinate per
basis blade.  Right Clifford multiplication by a vector is represented using
``wedge - contraction``.  With this sign convention the generators satisfy

    D_i D_j + D_j D_i = -2 delta_ij I,

so every unit-vector generator is skew-orthogonal and its exponential is an
energy-preserving flow.  In dimension four the medium has 16 modes grouped by
grade as ``1 + 4 + 6 + 4 + 1``.

The implementation intentionally uses dense reference matrices.  Each basis
generator also decomposes into eight disjoint signed plane rotations, which is
the representation that should lower to ``mus-watsn``/``mus-gesture``.
"""
from __future__ import annotations

import itertools
import math
from dataclasses import asdict, dataclass
from typing import Iterable, Sequence

import torch
from torch import Tensor


@dataclass(frozen=True)
class CliffordConfig:
    """Configuration for exact reference experiments."""

    latent_dim: int = 4
    dtype: str = "float64"
    random_trials: int = 4096
    seed: int = 19

    def torch_dtype(self) -> torch.dtype:
        try:
            return getattr(torch, self.dtype)
        except AttributeError as exc:  # pragma: no cover - defensive API check
            raise ValueError(f"unsupported torch dtype: {self.dtype}") from exc


def bit_count(value: int) -> int:
    return int(value).bit_count()


def masks_by_grade(dim: int) -> dict[int, list[int]]:
    """Return basis-blade bitmasks grouped by exterior grade."""

    return {
        grade: [mask for mask in range(1 << dim) if bit_count(mask) == grade]
        for grade in range(dim + 1)
    }


def right_wedge_sign(mask_without_axis: int, axis: int) -> int:
    """Sign of ``e_mask wedge e_axis`` in ascending basis order."""

    greater = bit_count(mask_without_axis >> (axis + 1))
    return -1 if greater % 2 else 1


def plane_rotation_matrix(
    size: int,
    source: int,
    target: int,
    theta: float | Tensor,
    *,
    dtype: torch.dtype,
    device: torch.device | None = None,
) -> Tensor:
    """Return one row-state plane rotation.

    The convention is ``x_next = x @ R`` and a positive quarter turn sends the
    source basis row to the target basis row.
    """

    matrix = torch.eye(size, dtype=dtype, device=device)
    theta_tensor = torch.as_tensor(theta, dtype=dtype, device=device)
    cosine = torch.cos(theta_tensor)
    sine = torch.sin(theta_tensor)
    matrix[source, source] = cosine
    matrix[source, target] = sine
    matrix[target, source] = -sine
    matrix[target, target] = cosine
    return matrix


def clifford_generators(config: CliffordConfig = CliffordConfig()) -> list[Tensor]:
    """Construct right-action generators for ``Cl(0, d)``.

    Each generator is the product of ``2**(d-1)`` disjoint signed quarter-turn
    blocks.  The result is real, orthogonal, determinant-one, and squares to
    ``-I``.
    """

    dim = config.latent_dim
    state_dim = 1 << dim
    dtype = config.torch_dtype()
    generators: list[Tensor] = []
    for axis in range(dim):
        matrix = torch.eye(state_dim, dtype=dtype)
        for mask in range(state_dim):
            if mask & (1 << axis):
                continue
            paired = mask | (1 << axis)
            sign = right_wedge_sign(mask, axis)
            matrix = matrix @ plane_rotation_matrix(
                state_dim,
                mask,
                paired,
                sign * math.pi / 2,
                dtype=dtype,
            )
        generators.append(matrix)
    return generators


def stack_generators(config: CliffordConfig = CliffordConfig()) -> Tensor:
    return torch.stack(clifford_generators(config), dim=0)


def vacuum_state(
    batch_shape: Sequence[int] = (),
    *,
    latent_dim: int = 4,
    dtype: torch.dtype = torch.float64,
    device: torch.device | None = None,
) -> Tensor:
    state = torch.zeros(*batch_shape, 1 << latent_dim, dtype=dtype, device=device)
    state[..., 0] = 1.0
    return state


def vector_operator(vector: Tensor, generators: Tensor | Sequence[Tensor]) -> Tensor:
    """Map vectors to their Clifford right-multiplication operators."""

    generator_tensor = (
        generators if isinstance(generators, Tensor) else torch.stack(list(generators))
    )
    if vector.shape[-1] != generator_tensor.shape[0]:
        raise ValueError("vector dimension does not match generator count")
    return torch.einsum("...d,dij->...ij", vector, generator_tensor)


def multiply_vector(
    state: Tensor,
    vector: Tensor,
    generators: Tensor | Sequence[Tensor],
) -> Tensor:
    operator = vector_operator(vector, generators)
    return torch.matmul(state.unsqueeze(-2), operator).squeeze(-2)


def multiply_word(
    state: Tensor,
    vectors: Iterable[Tensor],
    generators: Tensor | Sequence[Tensor],
) -> Tensor:
    output = state
    for vector in vectors:
        output = multiply_vector(output, vector, generators)
    return output


def permutation_parity(permutation: Sequence[int]) -> int:
    return sum(
        permutation[i] > permutation[j]
        for i in range(len(permutation))
        for j in range(i + 1, len(permutation))
    ) % 2


def algebra_checks(config: CliffordConfig, generators: Sequence[Tensor]) -> dict[str, float]:
    """Numerically verify the defining Clifford and energy identities."""

    state_dim = 1 << config.latent_dim
    identity = torch.eye(state_dim, dtype=generators[0].dtype)
    max_square = 0.0
    max_anticommutator = 0.0
    max_orthogonality = 0.0
    determinants: list[float] = []
    for i, first in enumerate(generators):
        max_square = max(max_square, float((first @ first + identity).abs().max()))
        max_orthogonality = max(
            max_orthogonality,
            float((first.T @ first - identity).abs().max()),
        )
        determinants.append(float(torch.linalg.det(first)))
        for j, second in enumerate(generators):
            if i != j:
                max_anticommutator = max(
                    max_anticommutator,
                    float((first @ second + second @ first).abs().max()),
                )
    return {
        "max_square_residual": max_square,
        "max_anticommutator_residual": max_anticommutator,
        "max_orthogonality_residual": max_orthogonality,
        "minimum_determinant": min(determinants),
        "maximum_determinant": max(determinants),
    }


def local_rotation_programs(config: CliffordConfig) -> dict[str, list[dict[str, object]]]:
    """Emit the sparse signed Givens program for each basis gesture."""

    state_dim = 1 << config.latent_dim
    programs: dict[str, list[dict[str, object]]] = {}
    for axis in range(config.latent_dim):
        events: list[dict[str, object]] = []
        for mask in range(state_dim):
            if mask & (1 << axis):
                continue
            sign = right_wedge_sign(mask, axis)
            events.append(
                {
                    "from_blade": mask,
                    "to_blade": mask | (1 << axis),
                    "angle_radians": sign * math.pi / 2,
                    "angle_quarter_turns": sign,
                }
            )
        programs[str(axis)] = events
    return programs


def permutation_experiment(
    config: CliffordConfig,
    generators: Sequence[Tensor] | None = None,
) -> dict[str, object]:
    """Classify all basis-vector permutations by their oriented top grade."""

    generators = list(generators or clifford_generators(config))
    state_dim = 1 << config.latent_dim
    vacuum = vacuum_state(latent_dim=config.latent_dim, dtype=generators[0].dtype)
    top_grade = state_dim - 1
    rows: list[dict[str, object]] = []
    for permutation in itertools.permutations(range(config.latent_dim)):
        state = vacuum
        for token in permutation:
            state = state @ generators[token]
        amplitude = float(state[top_grade])
        expected = -1.0 if permutation_parity(permutation) else 1.0
        rows.append(
            {
                "permutation": list(permutation),
                "parity": permutation_parity(permutation),
                "pseudoscalar_amplitude": amplitude,
                "expected_sign": expected,
                "exact_amplitude_correct": abs(amplitude - expected) < 1e-12,
                "sign_correct": (amplitude >= 0) == (expected >= 0),
                "energy": float(state.square().sum()),
            }
        )
    return {
        "exact_amplitude_accuracy": sum(
            bool(row["exact_amplitude_correct"]) for row in rows
        )
        / len(rows),
        "sign_accuracy": sum(bool(row["sign_correct"]) for row in rows) / len(rows),
        "minimum_abs_amplitude": min(
            abs(float(row["pseudoscalar_amplitude"])) for row in rows
        ),
        "maximum_energy_drift": max(abs(float(row["energy"]) - 1.0) for row in rows),
        "rows": rows,
    }


def continuous_volume_experiment(
    config: CliffordConfig,
    generators: Sequence[Tensor] | None = None,
) -> dict[str, float | int]:
    """Verify that four vector gestures encode oriented four-volume.

    For arbitrary vectors ``v_1, ..., v_d``, the top-grade coefficient of their
    Clifford product is the exterior product and therefore the determinant.
    """

    torch.manual_seed(config.seed)
    generator_tensor = torch.stack(list(generators or clifford_generators(config)))
    dtype = generator_tensor.dtype
    dim = config.latent_dim
    vectors = torch.randn(config.random_trials, dim, dim, dtype=dtype)
    vectors = vectors / vectors.norm(dim=-1, keepdim=True).clamp_min(1e-12)
    state = vacuum_state(
        (config.random_trials,), latent_dim=dim, dtype=dtype
    )
    for step in range(dim):
        state = multiply_vector(state, vectors[:, step], generator_tensor)
    pseudoscalar = state[:, -1]
    determinant = torch.linalg.det(vectors)
    orientation = 1.0 if torch.dot(pseudoscalar, determinant) >= 0 else -1.0
    residual = pseudoscalar - orientation * determinant
    energy = state.square().sum(-1)
    nondegenerate = determinant.abs() > 1e-8
    sign_accuracy = (
        torch.sign(pseudoscalar[nondegenerate])
        == torch.sign(orientation * determinant[nondegenerate])
    ).double().mean()
    return {
        "trials": config.random_trials,
        "global_orientation_convention": orientation,
        "maximum_volume_residual": float(residual.abs().max()),
        "rms_volume_residual": float(residual.square().mean().sqrt()),
        "sign_accuracy": float(sign_accuracy),
        "maximum_energy_drift": float((energy - 1.0).abs().max()),
        "mean_abs_oriented_volume": float(determinant.abs().mean()),
    }


def exact_receipt(config: CliffordConfig = CliffordConfig()) -> dict[str, object]:
    generators = clifford_generators(config)
    return {
        "model": "clifford-gesture-medium",
        "config": asdict(config),
        "signature": f"Cl(0,{config.latent_dim})",
        "state_dimension": 1 << config.latent_dim,
        "grade_string_site_counts": [
            math.comb(config.latent_dim, grade)
            for grade in range(config.latent_dim + 1)
        ],
        "local_rotations_per_basis_gesture": 1 << (config.latent_dim - 1),
        "algebra": algebra_checks(config, generators),
        "permutation_parity": permutation_experiment(config, generators),
        "continuous_oriented_volume": continuous_volume_experiment(
            config, generators
        ),
        "programs": local_rotation_programs(config),
        "claim_discipline": {
            "this_is_a_language_model": False,
            "this_is_an_exact_finite_dimensional_gesture_memory": True,
            "one_fixed_representation_is_signature_universal": False,
            "four_dimensions_are_uniquely_optimal": False,
            "top_grade_is_an_exterior_path_order_observable": True,
        },
    }
