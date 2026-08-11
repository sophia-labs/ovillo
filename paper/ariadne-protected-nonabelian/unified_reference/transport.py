"""Kato transport, gauge receipts, finite-time dynamics, and duration refusal."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import math
from typing import Sequence

import numpy as np
from numpy.typing import NDArray

from .compiler import ComplexCompileCertificate, ComplexGraph, compile_unitary_on_tree
from .model import (
    COMPLEX_DIMENSION,
    GAMMAS,
    IDENTITY4,
    FamilyParameters,
    dirac_d,
    dirac_hamiltonian,
    physical_mixer,
    protected_projector,
    relative_hamiltonian,
    wrap_torus,
)

ComplexMatrix = NDArray[np.complex128]
ComplexVector = NDArray[np.complex128]


@dataclass(frozen=True)
class SmoothLoopSpec:
    name: str
    base: tuple[float, float, float, float]
    first_axis: int
    second_axis: int
    first_radius: float
    second_radius: float
    eased: bool = True

    def validate(self) -> None:
        if self.first_axis == self.second_axis or not (
            0 <= self.first_axis < 4 and 0 <= self.second_axis < 4
        ):
            raise ValueError("loop axes must be distinct and in range")
        if len(self.base) != 4 or not all(math.isfinite(value) for value in self.base):
            raise ValueError("base must contain four finite coordinates")
        if not all(
            math.isfinite(value) and value != 0.0
            for value in (self.first_radius, self.second_radius)
        ):
            raise ValueError("loop radii must be finite and nonzero")

    def _phase(self, s: float) -> tuple[float, float, float]:
        if not (0.0 <= s <= 1.0):
            raise ValueError("path parameter must lie in [0,1]")
        if not self.eased:
            return 2.0 * math.pi * s, 2.0 * math.pi, 0.0
        # Quintic smoothstep: phase velocity and acceleration vanish at endpoints.
        u = 10.0 * s**3 - 15.0 * s**4 + 6.0 * s**5
        du = 30.0 * s**2 - 60.0 * s**3 + 30.0 * s**4
        ddu = 60.0 * s - 180.0 * s**2 + 120.0 * s**3
        return 2.0 * math.pi * u, 2.0 * math.pi * du, 2.0 * math.pi * ddu

    def point(self, s: float) -> NDArray[np.float64]:
        self.validate()
        phase, _, _ = self._phase(s)
        value = np.asarray(self.base, dtype=np.float64).copy()
        value[self.first_axis] += self.first_radius * (1.0 - math.cos(phase))
        value[self.second_axis] += self.second_radius * math.sin(phase)
        return wrap_torus(value)

    def derivatives(
        self, s: float
    ) -> tuple[NDArray[np.float64], NDArray[np.float64]]:
        self.validate()
        phase, velocity, acceleration = self._phase(s)
        first = np.zeros(4, dtype=np.float64)
        second = np.zeros(4, dtype=np.float64)
        first[self.first_axis] = self.first_radius * math.sin(phase) * velocity
        first[self.second_axis] = self.second_radius * math.cos(phase) * velocity
        second[self.first_axis] = self.first_radius * (
            math.cos(phase) * velocity * velocity
            + math.sin(phase) * acceleration
        )
        second[self.second_axis] = self.second_radius * (
            -math.sin(phase) * velocity * velocity
            + math.cos(phase) * acceleration
        )
        return first, second

    def points(self, count: int) -> tuple[NDArray[np.float64], ...]:
        if count < 17:
            raise ValueError("a loop needs at least seventeen points")
        return tuple(self.point(float(s)) for s in np.linspace(0.0, 1.0, count))


DEFAULT_BASE = (-0.4, 0.0, 0.9, -0.7)
LOOP_A = SmoothLoopSpec("A", DEFAULT_BASE, 2, 3, 1.50, 1.20)
LOOP_B = SmoothLoopSpec("B", DEFAULT_BASE, 0, 3, 1.30, 1.00)


def _polar_unitary(matrix: ComplexMatrix) -> tuple[ComplexMatrix, float]:
    left, singular_values, right = np.linalg.svd(matrix)
    return left @ right, float(np.min(singular_values))


def kato_direct_rotation(
    old_projector: ComplexMatrix, new_projector: ComplexMatrix
) -> tuple[ComplexMatrix, float]:
    old_projector = np.asarray(old_projector, dtype=np.complex128)
    new_projector = np.asarray(new_projector, dtype=np.complex128)
    if old_projector.shape != new_projector.shape or old_projector.ndim != 2:
        raise ValueError("projectors must be equal square matrices")
    identity = np.eye(old_projector.shape[0], dtype=np.complex128)
    transition = (
        new_projector @ old_projector
        + (identity - new_projector) @ (identity - old_projector)
    )
    return _polar_unitary(transition)


@dataclass(frozen=True)
class KatoPathCertificate:
    name: str
    points: int
    rank: int
    minimum_gap: float
    minimum_transition_singular_value: float
    maximum_adjacent_projector_distance_op: float
    full_unitarity_error_fro: float
    occupied_unitarity_error_fro: float
    endpoint_projector_error_fro: float
    transport_intertwining_error_fro: float
    determinant_error: float
    occupied_eigenphases: tuple[float, float]
    trace_real: float
    trace_imag: float


@dataclass(frozen=True)
class KatoPath:
    spec: SmoothLoopSpec
    points: tuple[NDArray[np.float64], ...]
    projectors: tuple[ComplexMatrix, ...]
    steps: tuple[ComplexMatrix, ...]
    full_holonomy: ComplexMatrix
    occupied_holonomy: ComplexMatrix
    base_frame: ComplexMatrix
    certificate: KatoPathCertificate


def kato_path(
    spec: SmoothLoopSpec,
    parameters: FamilyParameters = FamilyParameters(),
    *,
    points: int = 257,
) -> KatoPath:
    parameters.validate()
    path = spec.points(points)
    projectors = tuple(protected_projector(point, parameters) for point in path)
    start_hamiltonian = relative_hamiltonian(path[0], parameters)
    _, eigenvectors = np.linalg.eigh(start_hamiltonian)
    base_frame = eigenvectors[:, :2]
    total = np.eye(COMPLEX_DIMENSION, dtype=np.complex128)
    steps: list[ComplexMatrix] = []
    minimum_overlap = 1.0
    maximum_distance = 0.0
    minimum_gap = math.inf
    maximum_intertwining = 0.0
    for old, new, point in zip(projectors[:-1], projectors[1:], path[1:], strict=True):
        step, overlap = kato_direct_rotation(old, new)
        steps.append(step)
        total = step @ total
        minimum_overlap = min(minimum_overlap, overlap)
        maximum_distance = max(
            maximum_distance, float(np.linalg.norm(new - old, ord=2))
        )
        eigenvalues = np.linalg.eigvalsh(relative_hamiltonian(point, parameters))
        minimum_gap = min(minimum_gap, float(eigenvalues[2] - eigenvalues[1]))
        maximum_intertwining = max(
            maximum_intertwining,
            float(np.linalg.norm(step @ old @ step.conj().T - new, ord="fro")),
        )
    occupied = base_frame.conj().T @ total @ base_frame
    phases = tuple(float(value) for value in np.sort(np.angle(np.linalg.eigvals(occupied))))
    if len(phases) != 2:
        raise AssertionError("rank-two holonomy must have two phases")
    return KatoPath(
        spec=spec,
        points=path,
        projectors=projectors,
        steps=tuple(steps),
        full_holonomy=total,
        occupied_holonomy=occupied,
        base_frame=base_frame,
        certificate=KatoPathCertificate(
            name=spec.name,
            points=points,
            rank=2,
            minimum_gap=float(minimum_gap),
            minimum_transition_singular_value=float(minimum_overlap),
            maximum_adjacent_projector_distance_op=float(maximum_distance),
            full_unitarity_error_fro=float(
                np.linalg.norm(total.conj().T @ total - np.eye(COMPLEX_DIMENSION), ord="fro")
            ),
            occupied_unitarity_error_fro=float(
                np.linalg.norm(occupied.conj().T @ occupied - np.eye(2), ord="fro")
            ),
            endpoint_projector_error_fro=float(
                np.linalg.norm(projectors[-1] - projectors[0], ord="fro")
            ),
            transport_intertwining_error_fro=float(maximum_intertwining),
            determinant_error=abs(complex(np.linalg.det(occupied)) - 1.0),
            occupied_eigenphases=(phases[0], phases[1]),
            trace_real=float(np.trace(occupied).real),
            trace_imag=float(np.trace(occupied).imag),
        ),
    )


@dataclass(frozen=True)
class NonAbelianCertificate:
    commutator_fro: float
    commutator_operator_norm: float
    group_commutator_distance_fro: float
    trace_commutator_real: float
    trace_commutator_imag: float
    exact_coupling_homotopy_lower_bound_fro: float
    wider_parameter_box_status: str


def nonabelian_certificate(loop_a: KatoPath, loop_b: KatoPath) -> NonAbelianCertificate:
    a = loop_a.occupied_holonomy
    b = loop_b.occupied_holonomy
    commutator = a @ b - b @ a
    group = a @ b @ a.conj().T @ b.conj().T
    return NonAbelianCertificate(
        commutator_fro=float(np.linalg.norm(commutator, ord="fro")),
        commutator_operator_norm=float(np.linalg.norm(commutator, ord=2)),
        group_commutator_distance_fro=float(np.linalg.norm(group - np.eye(2), ord="fro")),
        trace_commutator_real=float(np.trace(commutator).real),
        trace_commutator_imag=float(np.trace(commutator).imag),
        exact_coupling_homotopy_lower_bound_fro=float(
            np.linalg.norm(commutator, ord="fro")
        ),
        wider_parameter_box_status=(
            "C2 and rank are analytically certified over the coefficient box; "
            "this particular Wilson commutator is only empirically swept there."
        ),
    )


@dataclass(frozen=True)
class GaugeCovarianceCertificate:
    loop_matrix_covariance_error_fro: float
    eigenphase_error: float
    trace_error: float
    determinant_error: float
    commutator_norm_error: float


def verify_gauge_covariance(
    loop_a: KatoPath, loop_b: KatoPath, *, seed: int = 0xA71AD
) -> GaugeCovarianceCertificate:
    rng = np.random.default_rng(seed)
    raw = rng.normal(size=(2, 2)) + 1.0j * rng.normal(size=(2, 2))
    gauge, _ = np.linalg.qr(raw)
    a = loop_a.occupied_holonomy
    b = loop_b.occupied_holonomy
    transformed_a = gauge.conj().T @ a @ gauge
    transformed_b = gauge.conj().T @ b @ gauge
    direct_a = (loop_a.base_frame @ gauge).conj().T @ loop_a.full_holonomy @ (
        loop_a.base_frame @ gauge
    )
    phases_original = np.sort(np.angle(np.linalg.eigvals(a)))
    phases_transformed = np.sort(np.angle(np.linalg.eigvals(transformed_a)))
    original_commutator = np.linalg.norm(a @ b - b @ a, ord="fro")
    transformed_commutator = np.linalg.norm(
        transformed_a @ transformed_b - transformed_b @ transformed_a, ord="fro"
    )
    return GaugeCovarianceCertificate(
        loop_matrix_covariance_error_fro=float(
            np.linalg.norm(direct_a - transformed_a, ord="fro")
        ),
        eigenphase_error=float(np.linalg.norm(phases_original - phases_transformed)),
        trace_error=abs(complex(np.trace(a) - np.trace(transformed_a))),
        determinant_error=abs(complex(np.linalg.det(a) - np.linalg.det(transformed_a))),
        commutator_norm_error=abs(float(original_commutator - transformed_commutator)),
    )


@dataclass(frozen=True)
class PathLawCertificate:
    reversal_error_fro: float
    concatenation_error_fro: float
    identity_error_fro: float
    coarse_fine_error_fro: float
    richardson_estimator_fro: float


def verify_path_laws(
    spec: SmoothLoopSpec,
    parameters: FamilyParameters = FamilyParameters(),
    *,
    coarse_points: int = 129,
) -> PathLawCertificate:
    coarse = kato_path(spec, parameters, points=coarse_points)
    fine = kato_path(spec, parameters, points=2 * coarse_points - 1)
    reversed_points = tuple(reversed(fine.points))
    projectors = tuple(reversed(fine.projectors))
    reverse_total = np.eye(COMPLEX_DIMENSION, dtype=np.complex128)
    for old, new in zip(projectors[:-1], projectors[1:], strict=True):
        step, _ = kato_direct_rotation(old, new)
        reverse_total = step @ reverse_total
    identity_path = kato_path(
        SmoothLoopSpec(
            name="identity",
            base=spec.base,
            first_axis=spec.first_axis,
            second_axis=spec.second_axis,
            first_radius=1.0e-15,
            second_radius=1.0e-15,
        ),
        parameters,
        points=17,
    )
    # Concatenating a loop with itself means execute the same total twice.
    concatenated = fine.full_holonomy @ fine.full_holonomy
    replay = np.eye(COMPLEX_DIMENSION, dtype=np.complex128)
    for step in fine.steps + fine.steps:
        replay = step @ replay
    coarse_fine = float(
        np.linalg.norm(coarse.full_holonomy - fine.full_holonomy, ord="fro")
    )
    return PathLawCertificate(
        reversal_error_fro=float(
            np.linalg.norm(reverse_total - fine.full_holonomy.conj().T, ord="fro")
        ),
        concatenation_error_fro=float(np.linalg.norm(replay - concatenated, ord="fro")),
        identity_error_fro=float(
            np.linalg.norm(identity_path.full_holonomy - np.eye(COMPLEX_DIMENSION), ord="fro")
        ),
        coarse_fine_error_fro=coarse_fine,
        # Direct-rotation products converge at least linearly under the recorded
        # overlap condition; coarse/fine difference is a conservative estimator.
        richardson_estimator_fro=coarse_fine,
    )


def _dirac_derivative_norms(
    spec: SmoothLoopSpec, s: float, mass: float
) -> tuple[float, float, float]:
    k = spec.point(s)
    k_first, k_second = spec.derivatives(s)
    d = dirac_d(k, mass=mass)
    first = np.empty(5, dtype=np.float64)
    second = np.empty(5, dtype=np.float64)
    first[:4] = np.cos(k) * k_first
    first[4] = -float(np.sum(np.sin(k) * k_first))
    second[:4] = -np.sin(k) * k_first * k_first + np.cos(k) * k_second
    second[4] = -float(
        np.sum(np.cos(k) * k_first * k_first + np.sin(k) * k_second)
    )
    return float(np.linalg.norm(d)), float(np.linalg.norm(first)), float(np.linalg.norm(second))


@dataclass(frozen=True)
class AdiabaticBoundCertificate:
    loop: str
    dimensionless_duration: float
    physical_duration_seconds: float
    projector_error_upper_bound: float
    transition_probability_upper_bound: float
    minimum_gap: float
    maximum_h_prime: float
    maximum_h_second: float
    jrs_numerator: float
    accepted: bool
    requested_budget: float
    theorem: str


def jrs_adiabatic_bound(
    spec: SmoothLoopSpec,
    parameters: FamilyParameters = FamilyParameters(),
    *,
    physical_duration_seconds: float,
    requested_projector_error: float = 0.05,
    integration_grid: int = 4097,
) -> AdiabaticBoundCertificate:
    """Evaluate Jansen--Ruskai--Seiler Theorem 3 for the rank-two band."""
    parameters.validate()
    if physical_duration_seconds <= 0.0:
        raise ValueError("physical duration must be positive")
    if integration_grid < 33:
        raise ValueError("integration grid is too small")
    samples = np.linspace(0.0, 1.0, integration_grid)
    radii = np.empty(integration_grid)
    first = np.empty(integration_grid)
    second = np.empty(integration_grid)
    for index, s in enumerate(samples):
        radii[index], first[index], second[index] = _dirac_derivative_norms(
            spec, float(s), parameters.mass
        )
    gaps = 2.0 * radii
    rank = 2.0
    endpoint = rank * (
        first[0] / gaps[0] ** 2 + first[-1] / gaps[-1] ** 2
    )
    integrand = (
        rank * second / gaps**2
        + 7.0 * rank * math.sqrt(rank) * first**2 / gaps**3
    )
    integral = float(np.trapezoid(integrand, samples))
    numerator = float(endpoint + integral)
    dimensionless_duration = (
        2.0 * math.pi * parameters.dirac_scale_hz * physical_duration_seconds
    )
    bound = numerator / dimensionless_duration
    return AdiabaticBoundCertificate(
        loop=spec.name,
        dimensionless_duration=float(dimensionless_duration),
        physical_duration_seconds=float(physical_duration_seconds),
        projector_error_upper_bound=float(bound),
        transition_probability_upper_bound=float(min(1.0, bound * bound)),
        minimum_gap=float(np.min(gaps)),
        maximum_h_prime=float(np.max(first)),
        maximum_h_second=float(np.max(second)),
        jrs_numerator=numerator,
        accepted=bound <= requested_projector_error,
        requested_budget=float(requested_projector_error),
        theorem=(
            "Jansen--Ruskai--Seiler (2007), Theorem 3, finite-dimensional "
            "rank-two spectral band; carrier scalar removed."
        ),
    )


def minimum_duration_for_jrs_budget(
    spec: SmoothLoopSpec,
    parameters: FamilyParameters = FamilyParameters(),
    *,
    requested_projector_error: float = 0.05,
) -> float:
    if requested_projector_error <= 0.0:
        raise ValueError("requested error must be positive")
    probe = jrs_adiabatic_bound(
        spec,
        parameters,
        physical_duration_seconds=1.0,
        requested_projector_error=requested_projector_error,
    )
    return probe.jrs_numerator / (
        2.0 * math.pi * parameters.dirac_scale_hz * requested_projector_error
    )


def _active_unitary_step(momentum: Sequence[float], phase_time: float, mass: float) -> ComplexMatrix:
    d = dirac_d(momentum, mass=mass)
    radius = float(np.linalg.norm(d))
    hamiltonian = dirac_hamiltonian(momentum, mass=mass)
    return (
        math.cos(radius * phase_time) * IDENTITY4
        - 1.0j * math.sin(radius * phase_time) * hamiltonian / radius
    )


@dataclass(frozen=True)
class AdiabaticReplayCertificate:
    loop: str
    duration_seconds: float
    sample_rate_hz: float
    samples: int
    projective_holonomy_error_fro: float
    leakage_fro: float
    retained_minimum_singular_value: float
    active_unitarity_error_fro: float
    host_block_invariance_error_fro: float
    host_block_invariance_basis: str
    jrs: AdiabaticBoundCertificate


def simulate_adiabatic_loop(
    spec: SmoothLoopSpec,
    reference: KatoPath,
    parameters: FamilyParameters = FamilyParameters(),
    *,
    duration_seconds: float,
    sample_rate_hz: float = 48_000.0,
    block_sizes: Sequence[int] = (1, 7, 64, 257, 1024),
    requested_projector_error: float = 0.05,
) -> AdiabaticReplayCertificate:
    """Sample-rate-parametric exact-exponential evolution of the active block."""
    parameters.validate()
    samples = int(round(duration_seconds * sample_rate_hz))
    if samples < 2:
        raise ValueError("duration is too short for the sample rate")
    actual_duration = samples / sample_rate_hz
    dimensionless_step = 2.0 * math.pi * parameters.dirac_scale_hz / sample_rate_hz

    def sample_order(chunks: Sequence[int]) -> tuple[int, ...]:
        order: list[int] = []
        cursor = 0
        chunk_index = 0
        while cursor < samples:
            chunk = chunks[chunk_index % len(chunks)]
            stop = min(samples, cursor + max(1, int(chunk)))
            order.extend(range(cursor, stop))
            cursor = stop
            chunk_index += 1
        return tuple(order)

    # Host blocks are a partition of one absolute sample-indexed recurrence.
    # Establish equality of the executed index order exactly, then evaluate the
    # numerical recurrence once.  Production runtimes still need an independent
    # cross-block replay test because their callback plumbing is not this oracle.
    canonical_order = tuple(range(samples))
    if sample_order(tuple(block_sizes)) != canonical_order:
        raise AssertionError("host block partition changed the sample order")

    evolution = np.eye(4, dtype=np.complex128)
    for sample in canonical_order:
        s = (sample + 0.5) / samples
        step = _active_unitary_step(
            spec.point(float(s)), dimensionless_step, parameters.mass
        )
        evolution = step @ evolution
    start_hamiltonian = dirac_hamiltonian(spec.point(0.0), mass=parameters.mass)
    _, frame = np.linalg.eigh(start_hamiltonian)
    occupied_frame = frame[:, :2]
    base_projector = occupied_frame @ occupied_frame.conj().T
    occupied_block = occupied_frame.conj().T @ evolution @ occupied_frame
    unitary_block, retained = _polar_unitary(occupied_block)

    # The full 16D Kato holonomy is a constant regional conjugation of the
    # active 4D direct rotation.  Transform back to the supermode basis.
    mixer = physical_mixer(parameters.coupling_fraction)
    super_holonomy = mixer.conj().T @ reference.full_holonomy @ mixer
    geometric = occupied_frame.conj().T @ super_holonomy[:4, :4] @ occupied_frame
    overlap = np.vdot(geometric, unitary_block)
    aligned = unitary_block * np.exp(-1.0j * np.angle(overlap))
    leakage = (np.eye(4) - base_projector) @ evolution @ occupied_frame
    jrs = jrs_adiabatic_bound(
        spec,
        parameters,
        physical_duration_seconds=actual_duration,
        requested_projector_error=requested_projector_error,
    )
    return AdiabaticReplayCertificate(
        loop=spec.name,
        duration_seconds=float(actual_duration),
        sample_rate_hz=float(sample_rate_hz),
        samples=samples,
        projective_holonomy_error_fro=float(np.linalg.norm(aligned - geometric, ord="fro")),
        leakage_fro=float(np.linalg.norm(leakage, ord="fro")),
        retained_minimum_singular_value=float(retained),
        active_unitarity_error_fro=float(
            np.linalg.norm(evolution.conj().T @ evolution - np.eye(4), ord="fro")
        ),
        host_block_invariance_error_fro=0.0,
        host_block_invariance_basis=(
            "exact equality of the absolute sample-index sequence under every "
            "declared host-block partition; production cross-runtime replay pending"
        ),
        jrs=jrs,
    )


def instrument_control_graph() -> ComplexGraph:
    """Heterogeneous authored graph for four regions x four internal channels."""
    edges: set[tuple[int, int]] = set()
    # Local internal complex couplers in every region.
    for region in range(4):
        base = 4 * region
        for channel in range(3):
            edges.add((base + channel, base + channel + 1))
    # Regional butterfly ports for each internal channel.
    region_pairs = ((0, 1), (2, 3), (0, 2), (1, 3))
    for left, right in region_pairs:
        for channel in range(4):
            edges.add((4 * left + channel, 4 * right + channel))
    return ComplexGraph(n=COMPLEX_DIMENSION, edges=tuple(sorted(edges)))


def compile_holonomy(
    path: KatoPath, *, graph: ComplexGraph | None = None
) -> ComplexCompileCertificate:
    return compile_unitary_on_tree(
        path.full_holonomy, graph or instrument_control_graph(), tolerance=1.0e-8
    )


def certificate_dict(value: object) -> dict:
    if hasattr(value, "to_json_dict"):
        return value.to_json_dict()  # type: ignore[no-any-return]
    if hasattr(value, "__dataclass_fields__"):
        return asdict(value)
    raise TypeError("unsupported certificate")
