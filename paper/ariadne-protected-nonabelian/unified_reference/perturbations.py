"""Deterministic perturbation corpus for the admitted Dirac coefficient box."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import math
from typing import Sequence

import numpy as np
from numpy.typing import NDArray

from .model import GAMMAS, IDENTITY4, wrap_torus
from .topology import CoefficientPerturbationBox
from .transport import LOOP_A, LOOP_B, SmoothLoopSpec, kato_direct_rotation

ComplexMatrix = NDArray[np.complex128]


@dataclass(frozen=True)
class DeformedDiracParameters:
    mass: float
    sine_weights: tuple[float, float, float, float]
    cosine_weights: tuple[float, float, float, float]


def deformed_d(
    momentum: Sequence[float], parameters: DeformedDiracParameters
) -> NDArray[np.float64]:
    k = wrap_torus(momentum)
    sine = np.asarray(parameters.sine_weights, dtype=np.float64) * np.sin(k)
    fifth = parameters.mass + float(
        np.asarray(parameters.cosine_weights, dtype=np.float64) @ np.cos(k)
    )
    return np.concatenate((sine, [fifth]))


def deformed_hamiltonian(
    momentum: Sequence[float], parameters: DeformedDiracParameters
) -> ComplexMatrix:
    coefficients = deformed_d(momentum, parameters)
    out = np.zeros((4, 4), dtype=np.complex128)
    for coefficient, gamma in zip(coefficients, GAMMAS, strict=True):
        out += coefficient * gamma
    return out


def deformed_projector(
    momentum: Sequence[float], parameters: DeformedDiracParameters
) -> ComplexMatrix:
    coefficients = deformed_d(momentum, parameters)
    radius = float(np.linalg.norm(coefficients))
    if radius <= 1.0e-14:
        raise ValueError("deformed family closes the gap")
    return 0.5 * (
        IDENTITY4 - deformed_hamiltonian(momentum, parameters) / radius
    )


def deformed_second_chern_degree(
    parameters: DeformedDiracParameters, *, grid: int = 8
) -> tuple[float, float]:
    axis = (np.arange(grid, dtype=np.float64) + 0.5) * (2.0 * math.pi / grid) - math.pi
    points = np.stack(np.meshgrid(axis, axis, axis, axis, indexing="ij"), axis=-1).reshape(-1, 4)
    velocities = np.asarray(parameters.sine_weights, dtype=np.float64)
    cosines = np.asarray(parameters.cosine_weights, dtype=np.float64)
    d = np.empty((points.shape[0], 5), dtype=np.float64)
    d[:, :4] = velocities[None, :] * np.sin(points)
    d[:, 4] = parameters.mass + np.cos(points) @ cosines
    jacobian = np.zeros((points.shape[0], 5, 4), dtype=np.float64)
    for direction in range(4):
        jacobian[:, direction, direction] = velocities[direction] * np.cos(
            points[:, direction]
        )
        jacobian[:, 4, direction] = -cosines[direction] * np.sin(
            points[:, direction]
        )
    radius = np.linalg.norm(d, axis=1)
    normal = d / radius[:, None]
    projected = np.einsum("ma,maj->mj", normal, jacobian)
    derivative = (
        jacobian - normal[:, :, None] * projected[:, None, :]
    ) / radius[:, None, None]
    orientation = np.concatenate([normal[:, :, None], derivative], axis=2)
    density = np.linalg.det(orientation)
    s4_volume = 8.0 * math.pi * math.pi / 3.0
    estimate = float(np.mean(density) * (2.0 * math.pi) ** 4 / s4_volume)
    return estimate, 2.0 * float(np.min(radius))


def _active_holonomy(
    spec: SmoothLoopSpec,
    parameters: DeformedDiracParameters,
    *,
    points: int,
) -> tuple[ComplexMatrix, float, float]:
    path = spec.points(points)
    projectors = tuple(deformed_projector(point, parameters) for point in path)
    total = np.eye(4, dtype=np.complex128)
    minimum_overlap = 1.0
    minimum_path_gap = math.inf
    for old, new, point in zip(projectors[:-1], projectors[1:], path[1:], strict=True):
        step, overlap = kato_direct_rotation(old, new)
        total = step @ total
        minimum_overlap = min(minimum_overlap, overlap)
        minimum_path_gap = min(
            minimum_path_gap, 2.0 * float(np.linalg.norm(deformed_d(point, parameters)))
        )
    _, frame = np.linalg.eigh(deformed_hamiltonian(path[0], parameters))
    occupied = frame[:, :2].conj().T @ total @ frame[:, :2]
    return occupied, minimum_overlap, minimum_path_gap


@dataclass(frozen=True)
class PerturbationTrial:
    index: int
    parameters: DeformedDiracParameters
    coefficient_radius: float
    second_chern_estimate: float
    sampled_global_gap: float
    minimum_loop_gap: float
    minimum_transition_singular_value: float
    commutator_fro: float


@dataclass(frozen=True)
class PerturbationSweepCertificate:
    seed: int
    trials: int
    degree_grid: int
    path_points: int
    maximum_coefficient_radius: float
    minimum_sampled_global_gap: float
    minimum_loop_gap: float
    maximum_second_chern_integer_error: float
    minimum_commutator_fro: float
    minimum_transition_singular_value: float
    trials_data: tuple[PerturbationTrial, ...]
    status: str

    def to_json_dict(self) -> dict:
        return asdict(self)


def run_perturbation_sweep(
    *,
    box: CoefficientPerturbationBox = CoefficientPerturbationBox(),
    trials: int = 12,
    seed: int = 0xC2A71AD,
    degree_grid: int = 8,
    path_points: int = 129,
) -> PerturbationSweepCertificate:
    box.validate()
    rng = np.random.default_rng(seed)
    results: list[PerturbationTrial] = []
    for index in range(trials):
        mass_delta = float(rng.uniform(-box.mass_error, box.mass_error))
        sine_delta = rng.uniform(
            -np.asarray(box.sine_weight_errors),
            np.asarray(box.sine_weight_errors),
        )
        cosine_delta = rng.uniform(
            -np.asarray(box.cosine_weight_errors),
            np.asarray(box.cosine_weight_errors),
        )
        parameters = DeformedDiracParameters(
            mass=-3.0 + mass_delta,
            sine_weights=tuple(float(value) for value in 1.0 + sine_delta),
            cosine_weights=tuple(float(value) for value in 1.0 + cosine_delta),
        )
        fifth_bound = abs(mass_delta) + float(np.sum(np.abs(cosine_delta)))
        coefficient_radius = float(
            math.sqrt(float(sine_delta @ sine_delta) + fifth_bound * fifth_bound)
        )
        c2, sampled_gap = deformed_second_chern_degree(
            parameters, grid=degree_grid
        )
        loop_a, overlap_a, path_gap_a = _active_holonomy(
            LOOP_A, parameters, points=path_points
        )
        loop_b, overlap_b, path_gap_b = _active_holonomy(
            LOOP_B, parameters, points=path_points
        )
        commutator = loop_a @ loop_b - loop_b @ loop_a
        results.append(
            PerturbationTrial(
                index=index,
                parameters=parameters,
                coefficient_radius=coefficient_radius,
                second_chern_estimate=c2,
                sampled_global_gap=sampled_gap,
                minimum_loop_gap=min(path_gap_a, path_gap_b),
                minimum_transition_singular_value=min(overlap_a, overlap_b),
                commutator_fro=float(np.linalg.norm(commutator, ord="fro")),
            )
        )
    return PerturbationSweepCertificate(
        seed=seed,
        trials=trials,
        degree_grid=degree_grid,
        path_points=path_points,
        maximum_coefficient_radius=max(result.coefficient_radius for result in results),
        minimum_sampled_global_gap=min(result.sampled_global_gap for result in results),
        minimum_loop_gap=min(result.minimum_loop_gap for result in results),
        maximum_second_chern_integer_error=max(
            abs(result.second_chern_estimate - 1.0) for result in results
        ),
        minimum_commutator_fro=min(result.commutator_fro for result in results),
        minimum_transition_singular_value=min(
            result.minimum_transition_singular_value for result in results
        ),
        trials_data=tuple(results),
        status=(
            "Empirical Wilson-loop certificate over deterministic coefficient draws. "
            "The gap/C2 preservation is separately analytic for the full box."
        ),
    )
