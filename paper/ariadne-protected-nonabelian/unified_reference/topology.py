"""Topology, gap, perturbation, and spectral-crowding certificates."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import math
from typing import Sequence

import numpy as np
from numpy.typing import NDArray

from .model import (
    GAMMAS,
    FamilyParameters,
    dirac_d,
    dirac_hamiltonian,
    occupied_projector,
)

ComplexMatrix = NDArray[np.complex128]
S4_VOLUME = 8.0 * math.pi * math.pi / 3.0


def analytic_second_chern(mass: float) -> int:
    critical = (-4.0, -2.0, 0.0, 2.0, 4.0)
    if any(abs(mass - value) <= 1.0e-12 for value in critical):
        raise ValueError("second Chern number is undefined at a gap closing")
    total = 0.0
    for occupied_corners in range(5):
        corner_mass = mass + 4.0 - 2.0 * occupied_corners
        sign = 1.0 if corner_mass > 0.0 else -1.0
        total += (
            ((-1.0) ** occupied_corners)
            * math.comb(4, occupied_corners)
            * sign
        )
    return int(round(0.5 * total))


@dataclass(frozen=True)
class DegreeChernCertificate:
    grid: int
    estimate: float
    nearest_integer: int
    integer_error: float
    sampled_gap: float


def second_chern_degree(*, mass: float = -3.0, grid: int = 12) -> DegreeChernCertificate:
    """Gauge-free degree integral for d/|d|: T4 -> S4."""
    if grid < 4:
        raise ValueError("grid must be at least four")
    axis = (np.arange(grid, dtype=np.float64) + 0.5) * (2.0 * math.pi / grid) - math.pi
    points = np.stack(np.meshgrid(axis, axis, axis, axis, indexing="ij"), axis=-1).reshape(-1, 4)
    count = points.shape[0]
    d = np.empty((count, 5), dtype=np.float64)
    d[:, :4] = np.sin(points)
    d[:, 4] = mass + np.sum(np.cos(points), axis=1)

    jacobian = np.zeros((count, 5, 4), dtype=np.float64)
    for axis_index in range(4):
        jacobian[:, axis_index, axis_index] = np.cos(points[:, axis_index])
        jacobian[:, 4, axis_index] = -np.sin(points[:, axis_index])

    radius = np.linalg.norm(d, axis=1)
    if float(np.min(radius)) <= 1.0e-14:
        raise ValueError("sampled model closes the gap")
    normal = d / radius[:, None]
    normal_dot_jacobian = np.einsum("ma,maj->mj", normal, jacobian)
    normal_derivative = (
        jacobian - normal[:, :, None] * normal_dot_jacobian[:, None, :]
    ) / radius[:, None, None]
    orientation_matrix = np.concatenate(
        [normal[:, :, None], normal_derivative], axis=2
    )
    density = np.linalg.det(orientation_matrix)
    estimate = float(np.mean(density) * (2.0 * math.pi) ** 4 / S4_VOLUME)
    nearest = int(round(estimate))
    return DegreeChernCertificate(
        grid=grid,
        estimate=estimate,
        nearest_integer=nearest,
        integer_error=abs(estimate - nearest),
        sampled_gap=2.0 * float(np.min(radius)),
    )


def _occupied_frame(momentum: Sequence[float], mass: float) -> ComplexMatrix:
    _, vectors = np.linalg.eigh(dirac_hamiltonian(momentum, mass=mass))
    return vectors[:, :2]


def _polar_unitary(matrix: ComplexMatrix) -> tuple[ComplexMatrix, float]:
    left, singular_values, right = np.linalg.svd(matrix)
    return left @ right, float(np.min(singular_values))


def _principal_antihermitian_log(unitary: ComplexMatrix) -> tuple[ComplexMatrix, float]:
    values, vectors = np.linalg.eig(unitary)
    phases = np.angle(values)
    logarithm = vectors @ np.diag(1.0j * phases) @ np.linalg.inv(vectors)
    logarithm = 0.5 * (logarithm - logarithm.conj().T)
    return logarithm, float(np.max(np.abs(phases)))


@dataclass(frozen=True)
class LatticeChernCertificate:
    grid: int
    estimate: float
    nearest_integer: int
    integer_error: float
    minimum_link_singular_value: float
    maximum_plaquette_eigenphase: float
    branch_safe: bool
    convention: str


def second_chern_lattice_gauge(*, mass: float = -3.0, grid: int = 10) -> LatticeChernCertificate:
    """Independent non-Abelian lattice-gauge estimate of C2.

    Occupied frames are used only to form polar-unitary links.  The final
    plaquette expression is gauge invariant.  The sign is pinned to the same
    T4 orientation as :func:`second_chern_degree`.
    """
    if grid < 4:
        raise ValueError("grid must be at least four")
    shape = (grid, grid, grid, grid)
    axis = np.arange(grid, dtype=np.float64) * (2.0 * math.pi / grid) - math.pi
    frames = np.empty(shape + (4, 2), dtype=np.complex128)
    for index in np.ndindex(shape):
        frames[index] = _occupied_frame([axis[value] for value in index], mass)

    links = np.empty((4,) + shape + (2, 2), dtype=np.complex128)
    minimum_link = 1.0
    for direction in range(4):
        for index in np.ndindex(shape):
            neighbor = list(index)
            neighbor[direction] = (neighbor[direction] + 1) % grid
            link, retained = _polar_unitary(
                frames[index].conj().T @ frames[tuple(neighbor)]
            )
            links[(direction,) + index] = link
            minimum_link = min(minimum_link, retained)

    total = 0.0
    maximum_phase = 0.0
    for index in np.ndindex(shape):
        curvature: dict[tuple[int, int], ComplexMatrix] = {}
        for first in range(4):
            for second in range(first + 1, 4):
                shifted_first = list(index)
                shifted_first[first] = (shifted_first[first] + 1) % grid
                shifted_second = list(index)
                shifted_second[second] = (shifted_second[second] + 1) % grid
                plaquette = (
                    links[(first,) + index]
                    @ links[(second,) + tuple(shifted_first)]
                    @ links[(first,) + tuple(shifted_second)].conj().T
                    @ links[(second,) + index].conj().T
                )
                logarithm, phase = _principal_antihermitian_log(plaquette)
                curvature[(first, second)] = logarithm
                maximum_phase = max(maximum_phase, phase)
        density = np.trace(
            curvature[(0, 1)] @ curvature[(2, 3)]
            - curvature[(0, 2)] @ curvature[(1, 3)]
            + curvature[(0, 3)] @ curvature[(1, 2)]
        )
        total += float(density.real)

    estimate = total / (4.0 * math.pi * math.pi)
    nearest = int(round(estimate))
    return LatticeChernCertificate(
        grid=grid,
        estimate=float(estimate),
        nearest_integer=nearest,
        integer_error=abs(float(estimate) - nearest),
        minimum_link_singular_value=float(minimum_link),
        maximum_plaquette_eigenphase=float(maximum_phase),
        branch_safe=maximum_phase < math.pi - 1.0e-8,
        convention="Tr(F01 F23 - F02 F13 + F03 F12)/(4 pi^2)",
    )


@dataclass(frozen=True)
class ExactGapCertificate:
    mass: float
    minimum_radius: float
    maximum_radius: float
    protected_cluster_gap: float
    proof_identity: str
    coupling_homotopy_isospectral: bool


def exact_gap_certificate(parameters: FamilyParameters = FamilyParameters()) -> ExactGapCertificate:
    parameters.validate()
    if abs(parameters.mass + 3.0) > 1.0e-14:
        raise ValueError("the closed-form certificate currently covers mass=-3 exactly")
    return ExactGapCertificate(
        mass=parameters.mass,
        minimum_radius=1.0,
        maximum_radius=7.0,
        protected_cluster_gap=2.0,
        proof_identity="|d|^2 = 1 + 2 sum_{i<j}(1-cos k_i)(1-cos k_j)",
        coupling_homotopy_isospectral=True,
    )


@dataclass(frozen=True)
class CoefficientPerturbationBox:
    mass_error: float = 0.05
    sine_weight_errors: tuple[float, float, float, float] = (0.03, 0.03, 0.03, 0.03)
    cosine_weight_errors: tuple[float, float, float, float] = (0.015, 0.015, 0.015, 0.015)
    extra_hermitian_operator_norm: float = 0.05

    def validate(self) -> None:
        values = (
            self.mass_error,
            *self.sine_weight_errors,
            *self.cosine_weight_errors,
            self.extra_hermitian_operator_norm,
        )
        if any(not math.isfinite(value) or value < 0.0 for value in values):
            raise ValueError("perturbation radii must be finite and nonnegative")


@dataclass(frozen=True)
class PerturbationGapCertificate:
    coefficient_radius_bound: float
    total_operator_radius_bound: float
    protected_gap_lower_bound: float
    cut_distance_lower_bound: float
    rank_preserved: bool
    c2_preserved: bool
    derivation: str


def certify_perturbation_box(
    box: CoefficientPerturbationBox = CoefficientPerturbationBox(),
) -> PerturbationGapCertificate:
    """Weyl/Clifford certificate for a complete coefficient uncertainty box."""
    box.validate()
    first_four = np.asarray(box.sine_weight_errors, dtype=np.float64)
    fifth = box.mass_error + float(np.sum(box.cosine_weight_errors))
    coefficient_radius = float(math.sqrt(float(first_four @ first_four) + fifth * fifth))
    total_radius = coefficient_radius + box.extra_hermitian_operator_norm
    gap = 2.0 - 2.0 * total_radius
    cut = 1.0 - total_radius
    preserved = total_radius < 1.0
    return PerturbationGapCertificate(
        coefficient_radius_bound=coefficient_radius,
        total_operator_radius_bound=total_radius,
        protected_gap_lower_bound=gap,
        cut_distance_lower_bound=cut,
        rank_preserved=preserved,
        c2_preserved=preserved,
        derivation=(
            "Clifford anticommutation gives ||sum delta d_a Gamma_a||=||delta d||_2; "
            "Weyl gives gap >= 2-2||Delta K||."
        ),
    )


@dataclass(frozen=True)
class ProjectorPerturbationCertificate:
    perturbation_norm: float
    unperturbed_gap: float
    projector_operator_norm_bound: float
    admissible: bool
    convention: str


def davis_kahan_projector_bound(
    perturbation_norm: float, *, unperturbed_gap: float = 2.0
) -> ProjectorPerturbationCertificate:
    if perturbation_norm < 0.0 or not math.isfinite(perturbation_norm):
        raise ValueError("perturbation_norm must be finite and nonnegative")
    admissible = 2.0 * perturbation_norm < unperturbed_gap
    denominator = max(unperturbed_gap - perturbation_norm, np.finfo(float).tiny)
    # A deliberately conservative resolvent/Davis--Kahan style receipt bound.
    bound = min(1.0, perturbation_norm / denominator)
    return ProjectorPerturbationCertificate(
        perturbation_norm=perturbation_norm,
        unperturbed_gap=unperturbed_gap,
        projector_operator_norm_bound=float(bound),
        admissible=admissible,
        convention="conservative sin-theta/resolvent bound",
    )


@dataclass(frozen=True)
class DelayCrowdingCertificate:
    delay_samples: int
    sample_rate_hz: float
    delay_seconds: float
    quasienergy_spacing_rad_per_sample: float
    modal_spacing_hz: float
    conclusion: str


def pure_delay_spectral_crowding(
    delay_samples: int, *, sample_rate_hz: float = 48_000.0
) -> DelayCrowdingCertificate:
    """Exact spacing obstruction for a lossless periodic delay loop."""
    if delay_samples < 2:
        raise ValueError("delay_samples must be at least two")
    if sample_rate_hz <= 0.0 or not math.isfinite(sample_rate_hz):
        raise ValueError("sample_rate_hz must be positive")
    seconds = delay_samples / sample_rate_hz
    spacing_rad = 2.0 * math.pi / delay_samples
    spacing_hz = sample_rate_hz / delay_samples
    return DelayCrowdingCertificate(
        delay_samples=delay_samples,
        sample_rate_hz=sample_rate_hz,
        delay_seconds=seconds,
        quasienergy_spacing_rad_per_sample=spacing_rad,
        modal_spacing_hz=spacing_hz,
        conclusion=(
            "A raw long lossless delay has eigenphases at all L-th roots of unity; "
            "its maximum adjacent quasienergy gap is 2*pi/L.  A fixed rank-two "
            "Floquet window therefore loses a uniform gap as L grows unless the "
            "delay modes are band-gapped, reduced, or exposed as passive ports."
        ),
    )


def certificate_dict(value: object) -> dict:
    if hasattr(value, "__dataclass_fields__"):
        return asdict(value)
    raise TypeError("certificate_dict requires a dataclass instance")
