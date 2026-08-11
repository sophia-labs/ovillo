"""Protected non-Abelian reference instruments for Ariadne.

This module deliberately separates three claims:

* the Z3 pump algebra is an exact, finite non-Abelian routing target grounded
  in the published acoustic Thouless-pump experiment;
* the 4D Wilson--Dirac family carries a gap-protected rank-two bundle with
  nonzero second Chern number;
* Wilson-loop values inside that bundle are non-Abelian and robust over a
  declared perturbation family, but remain geometric rather than individually
  quantized or fault-tolerant.

The full-space Kato/direct-rotation transport is gauge free.  Its realification
is orthogonal and can therefore be lowered by the existing graph-restricted
Givens compiler without hidden work in the declared Euclidean metric.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Sequence
import math

import numpy as np
from numpy.typing import NDArray

from .core import Graph, CompileCertificate, compile_so_on_tree, schedule_matrix

try:
    from scipy.linalg import expm
except ImportError:  # pragma: no cover - fail loudly at call sites
    expm = None

FloatMatrix = NDArray[np.float64]
ComplexMatrix = NDArray[np.complex128]


def _pauli() -> tuple[ComplexMatrix, ComplexMatrix, ComplexMatrix, ComplexMatrix]:
    sigma_x = np.array([[0.0, 1.0], [1.0, 0.0]], dtype=np.complex128)
    sigma_y = np.array([[0.0, -1.0j], [1.0j, 0.0]], dtype=np.complex128)
    sigma_z = np.array([[1.0, 0.0], [0.0, -1.0]], dtype=np.complex128)
    identity = np.eye(2, dtype=np.complex128)
    return sigma_x, sigma_y, sigma_z, identity


def clifford_gammas() -> tuple[ComplexMatrix, ...]:
    """A Hermitian 4x4 representation of Cl(5)."""
    sx, sy, sz, identity = _pauli()
    return (
        np.kron(sx, sx),
        np.kron(sx, sy),
        np.kron(sx, sz),
        np.kron(sy, identity),
        np.kron(sz, identity),
    )


_GAMMAS = clifford_gammas()
_IDENTITY4 = np.eye(4, dtype=np.complex128)
_S4_VOLUME = 8.0 * math.pi * math.pi / 3.0


@dataclass(frozen=True)
class Dirac4DParameters:
    """Gap-preserving deformation family around the C2=1 Wilson--Dirac phase."""

    mass: float = -3.0
    velocities: tuple[float, float, float, float] = (1.0, 1.0, 1.0, 1.0)
    cosine_weights: tuple[float, float, float, float] = (1.0, 1.0, 1.0, 1.0)
    coordinate_warp: tuple[float, float, float, float] = (0.0, 0.0, 0.0, 0.0)
    warp_phase: tuple[float, float, float, float] = (0.0, 0.0, 0.0, 0.0)


def _as_four(values: Sequence[float], name: str) -> NDArray[np.float64]:
    out = np.asarray(values, dtype=np.float64)
    if out.shape != (4,) or not np.all(np.isfinite(out)):
        raise ValueError(f"{name} must contain four finite values")
    return out


def _dirac_d_and_jacobian(
    momentum: Sequence[float],
    parameters: Dirac4DParameters,
) -> tuple[NDArray[np.float64], NDArray[np.float64]]:
    k = _as_four(momentum, "momentum")
    velocity = _as_four(parameters.velocities, "velocities")
    cosine = _as_four(parameters.cosine_weights, "cosine_weights")
    warp = _as_four(parameters.coordinate_warp, "coordinate_warp")
    phase = _as_four(parameters.warp_phase, "warp_phase")
    successor = np.array([1, 2, 3, 0], dtype=np.int64)

    q = k + warp * np.sin(k[successor] + phase)
    dq = np.eye(4, dtype=np.float64)
    for axis, source in enumerate(successor):
        dq[axis, source] += warp[axis] * math.cos(k[source] + phase[axis])

    d = np.empty(5, dtype=np.float64)
    d[:4] = velocity * np.sin(q)
    d[4] = parameters.mass + float(np.sum(cosine * np.cos(q)))

    jacobian = np.empty((5, 4), dtype=np.float64)
    jacobian[:4, :] = (velocity * np.cos(q))[:, None] * dq
    jacobian[4, :] = np.sum((-cosine * np.sin(q))[:, None] * dq, axis=0)
    return d, jacobian


def dirac4d_hamiltonian(
    momentum: Sequence[float],
    parameters: Dirac4DParameters = Dirac4DParameters(),
) -> ComplexMatrix:
    d, _ = _dirac_d_and_jacobian(momentum, parameters)
    return sum((d[index] * _GAMMAS[index] for index in range(5)), start=np.zeros((4, 4), dtype=np.complex128))


def occupied_projector(
    momentum: Sequence[float],
    parameters: Dirac4DParameters = Dirac4DParameters(),
) -> ComplexMatrix:
    """Projector onto the doubly-degenerate negative-energy subspace."""
    d, _ = _dirac_d_and_jacobian(momentum, parameters)
    radius = float(np.linalg.norm(d))
    if radius <= 1.0e-14:
        raise ValueError("occupied projector is undefined at a closed gap")
    hamiltonian = sum(
        (d[index] * _GAMMAS[index] for index in range(5)),
        start=np.zeros((4, 4), dtype=np.complex128),
    )
    return 0.5 * (_IDENTITY4 - hamiltonian / radius)


def analytic_second_chern(mass: float) -> float:
    """Second Chern number of the undeformed isotropic Wilson--Dirac model.

    Critical masses -4,-2,0,2,4 close the gap and therefore have no phase
    label.  The sign convention matches ``second_chern_degree``.
    """
    critical = (-4.0, -2.0, 0.0, 2.0, 4.0)
    if any(abs(mass - value) <= 1.0e-12 for value in critical):
        raise ValueError("second Chern number is undefined at a gap closing")
    total = 0.0
    for occupied_corners in range(5):
        corner_mass = mass + 4.0 - 2.0 * occupied_corners
        sign = 1.0 if corner_mass > 0.0 else -1.0
        total += ((-1.0) ** occupied_corners) * math.comb(4, occupied_corners) * sign
    return 0.5 * total


@dataclass(frozen=True)
class SecondChernEstimate:
    grid: int
    estimate: float
    nearest_integer: int
    integer_error: float
    sampled_gap: float


def second_chern_degree(
    parameters: Dirac4DParameters = Dirac4DParameters(),
    *,
    grid: int = 12,
) -> SecondChernEstimate:
    """Integrate the degree of d/|d| : T4 -> S4.

    The expression is gauge free and equivalent to the second Chern number of
    the occupied rank-two bundle for this Dirac family.
    """
    if grid < 4:
        raise ValueError("grid must be at least four")
    axis = (np.arange(grid, dtype=np.float64) + 0.5) * (2.0 * math.pi / grid) - math.pi
    points = np.stack(np.meshgrid(axis, axis, axis, axis, indexing="ij"), axis=-1).reshape(-1, 4)
    count = points.shape[0]

    velocity = _as_four(parameters.velocities, "velocities")
    cosine = _as_four(parameters.cosine_weights, "cosine_weights")
    warp = _as_four(parameters.coordinate_warp, "coordinate_warp")
    phase = _as_four(parameters.warp_phase, "warp_phase")
    successor = np.array([1, 2, 3, 0], dtype=np.int64)

    q = points + warp * np.sin(points[:, successor] + phase)
    dq = np.broadcast_to(np.eye(4, dtype=np.float64), (count, 4, 4)).copy()
    for target, source in enumerate(successor):
        dq[:, target, source] += warp[target] * np.cos(points[:, source] + phase[target])

    d = np.empty((count, 5), dtype=np.float64)
    d[:, :4] = velocity * np.sin(q)
    d[:, 4] = parameters.mass + np.sum(cosine * np.cos(q), axis=1)

    jacobian = np.empty((count, 5, 4), dtype=np.float64)
    jacobian[:, :4, :] = (velocity * np.cos(q))[:, :, None] * dq
    jacobian[:, 4, :] = np.sum((-cosine * np.sin(q))[:, :, None] * dq, axis=1)

    radius = np.linalg.norm(d, axis=1)
    if float(np.min(radius)) <= 1.0e-14:
        raise ValueError("sampled model closes the gap")
    normal = d / radius[:, None]
    normal_dot_jacobian = np.einsum("ma,maj->mj", normal, jacobian)
    normal_derivative = (
        jacobian - normal[:, :, None] * normal_dot_jacobian[:, None, :]
    ) / radius[:, None, None]
    orientation_matrix = np.concatenate(
        [normal[:, :, None], normal_derivative],
        axis=2,
    )
    density = np.linalg.det(orientation_matrix)
    estimate = float(np.mean(density) * (2.0 * math.pi) ** 4 / _S4_VOLUME)
    nearest = int(round(estimate))
    return SecondChernEstimate(
        grid=grid,
        estimate=estimate,
        nearest_integer=nearest,
        integer_error=abs(estimate - nearest),
        sampled_gap=2.0 * float(np.min(radius)),
    )


def _interpolate_parameters(
    left: Dirac4DParameters,
    right: Dirac4DParameters,
    amount: float,
) -> Dirac4DParameters:
    amount = float(amount)
    def mix(a: Sequence[float], b: Sequence[float]) -> tuple[float, float, float, float]:
        aa = _as_four(a, "left")
        bb = _as_four(b, "right")
        return tuple(float(value) for value in ((1.0 - amount) * aa + amount * bb))
    return Dirac4DParameters(
        mass=(1.0 - amount) * left.mass + amount * right.mass,
        velocities=mix(left.velocities, right.velocities),
        cosine_weights=mix(left.cosine_weights, right.cosine_weights),
        coordinate_warp=mix(left.coordinate_warp, right.coordinate_warp),
        warp_phase=mix(left.warp_phase, right.warp_phase),
    )


def sampled_homotopy_gap(
    target: Dirac4DParameters,
    *,
    source: Dirac4DParameters = Dirac4DParameters(),
    grid: int = 6,
    homotopy_steps: int = 5,
) -> float:
    if homotopy_steps < 2:
        raise ValueError("homotopy_steps must be at least two")
    minimum = math.inf
    for amount in np.linspace(0.0, 1.0, homotopy_steps):
        estimate = second_chern_degree(
            _interpolate_parameters(source, target, float(amount)),
            grid=grid,
        )
        minimum = min(minimum, estimate.sampled_gap)
    return float(minimum)


def wrap_torus(momentum: Sequence[float]) -> NDArray[np.float64]:
    value = _as_four(momentum, "momentum")
    return (value + math.pi) % (2.0 * math.pi) - math.pi


def torus_rectangle_loop(
    base: Sequence[float],
    first_axis: int,
    second_axis: int,
    first_extent: float,
    second_extent: float,
    *,
    steps_per_edge: int = 48,
) -> tuple[NDArray[np.float64], ...]:
    if first_axis == second_axis or not (
        0 <= first_axis < 4 and 0 <= second_axis < 4
    ):
        raise ValueError("invalid torus coordinate plane")
    if steps_per_edge < 4:
        raise ValueError("steps_per_edge must be at least four")
    origin = _as_four(base, "base")
    points: list[NDArray[np.float64]] = []

    def point(a: float, b: float) -> NDArray[np.float64]:
        value = origin.copy()
        value[first_axis] += a
        value[second_axis] += b
        return wrap_torus(value)

    for value in np.linspace(0.0, first_extent, steps_per_edge, endpoint=False):
        points.append(point(float(value), 0.0))
    for value in np.linspace(0.0, second_extent, steps_per_edge, endpoint=False):
        points.append(point(first_extent, float(value)))
    for value in np.linspace(first_extent, 0.0, steps_per_edge, endpoint=False):
        points.append(point(float(value), second_extent))
    for value in np.linspace(second_extent, 0.0, steps_per_edge + 1):
        points.append(point(0.0, float(value)))
    return tuple(points)


def torus_smooth_loop(
    base: Sequence[float],
    first_axis: int,
    second_axis: int,
    first_radius: float,
    second_radius: float,
    *,
    points: int = 193,
) -> tuple[NDArray[np.float64], ...]:
    """Smooth same-basepoint loop on a two-coordinate torus plane."""
    if first_axis == second_axis or not (
        0 <= first_axis < 4 and 0 <= second_axis < 4
    ):
        raise ValueError("invalid torus coordinate plane")
    if points < 17:
        raise ValueError("a smooth loop needs at least seventeen points")
    origin = _as_four(base, "base")
    path: list[NDArray[np.float64]] = []
    for phase in np.linspace(0.0, 2.0 * math.pi, points):
        value = origin.copy()
        value[first_axis] += first_radius * (1.0 - math.cos(float(phase)))
        value[second_axis] += second_radius * math.sin(float(phase))
        path.append(wrap_torus(value))
    return tuple(path)


def _polar_unitary(matrix: ComplexMatrix) -> tuple[ComplexMatrix, float]:
    left, singular_values, right = np.linalg.svd(matrix)
    return left @ right, float(np.min(singular_values))


def kato_direct_rotation(
    old_projector: ComplexMatrix,
    new_projector: ComplexMatrix,
) -> tuple[ComplexMatrix, float]:
    """Minimal full-space unitary carrying one projector to the next."""
    transition = (
        new_projector @ old_projector
        + (_IDENTITY4 - new_projector) @ (_IDENTITY4 - old_projector)
    )
    unitary, minimum_overlap = _polar_unitary(transition)
    return unitary, minimum_overlap


@dataclass(frozen=True)
class KatoLoopCertificate:
    points: int
    minimum_gap: float
    minimum_adjacent_overlap: float
    full_unitarity_error_fro: float
    occupied_unitarity_error_fro: float
    projector_return_error_fro: float
    determinant_error: float
    eigenphases: tuple[float, float]


@dataclass(frozen=True)
class KatoLoop:
    occupied_holonomy: ComplexMatrix
    full_holonomy: ComplexMatrix
    certificate: KatoLoopCertificate


def kato_wilson_loop(
    path: Sequence[Sequence[float]],
    parameters: Dirac4DParameters = Dirac4DParameters(),
) -> KatoLoop:
    if len(path) < 2:
        raise ValueError("a loop needs at least two points")
    start = wrap_torus(path[0])
    end = wrap_torus(path[-1])
    if float(np.linalg.norm(start - end)) > 1.0e-10:
        raise ValueError("the Kato path must close at its base point")

    start_hamiltonian = dirac4d_hamiltonian(start, parameters)
    _, frame = np.linalg.eigh(start_hamiltonian)
    occupied_frame = frame[:, :2]
    old_projector = occupied_projector(start, parameters)
    total = np.eye(4, dtype=np.complex128)
    minimum_gap = math.inf
    minimum_overlap = 1.0

    for point in path[1:]:
        wrapped = wrap_torus(point)
        d, _ = _dirac_d_and_jacobian(wrapped, parameters)
        minimum_gap = min(minimum_gap, 2.0 * float(np.linalg.norm(d)))
        new_projector = occupied_projector(wrapped, parameters)
        step, overlap = kato_direct_rotation(old_projector, new_projector)
        total = step @ total
        minimum_overlap = min(minimum_overlap, overlap)
        old_projector = new_projector

    occupied = occupied_frame.conj().T @ total @ occupied_frame
    phases = tuple(
        float(value)
        for value in np.sort(np.angle(np.linalg.eigvals(occupied)))
    )
    if len(phases) != 2:
        raise AssertionError("rank-two holonomy must have two eigenphases")
    return KatoLoop(
        occupied_holonomy=occupied,
        full_holonomy=total,
        certificate=KatoLoopCertificate(
            points=len(path),
            minimum_gap=float(minimum_gap),
            minimum_adjacent_overlap=float(minimum_overlap),
            full_unitarity_error_fro=float(
                np.linalg.norm(total.conj().T @ total - _IDENTITY4, ord="fro")
            ),
            occupied_unitarity_error_fro=float(
                np.linalg.norm(
                    occupied.conj().T @ occupied - np.eye(2),
                    ord="fro",
                )
            ),
            projector_return_error_fro=float(
                np.linalg.norm(old_projector - occupied_projector(start, parameters), ord="fro")
            ),
            determinant_error=abs(complex(np.linalg.det(occupied)) - 1.0),
            eigenphases=(phases[0], phases[1]),
        ),
    )


@dataclass(frozen=True)
class AdiabaticRunCertificate:
    duration: float
    projective_holonomy_error_fro: float
    leakage_fro: float
    minimum_retained_singular_value: float
    full_unitarity_error_fro: float


@dataclass(frozen=True)
class AdiabaticConvergenceCertificate:
    loop_a: tuple[AdiabaticRunCertificate, ...]
    loop_b: tuple[AdiabaticRunCertificate, ...]
    maximum_duration_projective_error_fro: float
    maximum_duration_leakage_fro: float


def simulate_adiabatic_loop(
    path: Sequence[Sequence[float]],
    parameters: Dirac4DParameters,
    *,
    duration: float,
    reference: KatoLoop | None = None,
) -> AdiabaticRunCertificate:
    """Finite-time Schrödinger replay against the geometric Kato oracle.

    The occupied pair is exactly degenerate, so its dynamical phase is scalar.
    The comparison removes that global phase and reports leakage separately.
    """
    if expm is None:
        raise RuntimeError("scipy is required for adiabatic replay")
    if duration <= 0.0 or len(path) < 2:
        raise ValueError("duration and path must be nontrivial")
    start = wrap_torus(path[0])
    _, frame = np.linalg.eigh(dirac4d_hamiltonian(start, parameters))
    occupied_frame = frame[:, :2]
    base_projector = occupied_frame @ occupied_frame.conj().T
    evolution = np.eye(4, dtype=np.complex128)
    step_duration = duration / (len(path) - 1)
    for left, right in zip(path, path[1:]):
        left_value = wrap_torus(left)
        right_value = wrap_torus(right)
        displacement = (right_value - left_value + math.pi) % (2.0 * math.pi) - math.pi
        midpoint = wrap_torus(left_value + 0.5 * displacement)
        hamiltonian = dirac4d_hamiltonian(midpoint, parameters)
        evolution = expm(-1.0j * hamiltonian * step_duration) @ evolution

    occupied_block = occupied_frame.conj().T @ evolution @ occupied_frame
    unitary_block, retained = _polar_unitary(occupied_block)
    geometric = (reference or kato_wilson_loop(path, parameters)).occupied_holonomy
    overlap = np.vdot(geometric, unitary_block)
    aligned = unitary_block * np.exp(-1.0j * np.angle(overlap))
    leakage = (_IDENTITY4 - base_projector) @ evolution @ occupied_frame
    return AdiabaticRunCertificate(
        duration=float(duration),
        projective_holonomy_error_fro=float(
            np.linalg.norm(aligned - geometric, ord="fro")
        ),
        leakage_fro=float(np.linalg.norm(leakage, ord="fro")),
        minimum_retained_singular_value=float(retained),
        full_unitarity_error_fro=float(
            np.linalg.norm(evolution.conj().T @ evolution - _IDENTITY4, ord="fro")
        ),
    )


def verify_adiabatic_convergence(
    path_a: Sequence[Sequence[float]],
    path_b: Sequence[Sequence[float]],
    parameters: Dirac4DParameters,
    *,
    durations: Sequence[float] = (16.0, 32.0, 64.0, 128.0),
    reference_a: KatoLoop | None = None,
    reference_b: KatoLoop | None = None,
) -> AdiabaticConvergenceCertificate:
    if not durations:
        raise ValueError("at least one adiabatic duration is required")
    runs_a = tuple(
        simulate_adiabatic_loop(
            path_a,
            parameters,
            duration=float(duration),
            reference=reference_a,
        )
        for duration in durations
    )
    runs_b = tuple(
        simulate_adiabatic_loop(
            path_b,
            parameters,
            duration=float(duration),
            reference=reference_b,
        )
        for duration in durations
    )
    return AdiabaticConvergenceCertificate(
        loop_a=runs_a,
        loop_b=runs_b,
        maximum_duration_projective_error_fro=max(
            runs_a[-1].projective_holonomy_error_fro,
            runs_b[-1].projective_holonomy_error_fro,
        ),
        maximum_duration_leakage_fro=max(
            runs_a[-1].leakage_fro,
            runs_b[-1].leakage_fro,
        ),
    )


def realify_unitary(unitary: ComplexMatrix) -> FloatMatrix:
    """Real action on [Re psi, Im psi]."""
    unitary = np.asarray(unitary, dtype=np.complex128)
    if unitary.shape[0] != unitary.shape[1]:
        raise ValueError("unitary must be square")
    real = np.real(unitary)
    imag = np.imag(unitary)
    return np.block([[real, -imag], [imag, real]]).astype(np.float64)


@dataclass(frozen=True)
class AcousticLoweringCertificate:
    real_dimension: int
    graph_edges: tuple[tuple[int, int], ...]
    gate_count: int
    reconstruction_error_fro: float
    orthogonality_error_fro: float
    determinant_error: float
    all_gates_on_graph: bool
    maximum_basis_energy_error: float


@dataclass(frozen=True)
class AcousticLowering:
    matrix: FloatMatrix
    compile: CompileCertificate
    certificate: AcousticLoweringCertificate


def lower_unitary_to_acoustic_graph(
    unitary: ComplexMatrix,
    *,
    graph: Graph | None = None,
) -> AcousticLowering:
    real = realify_unitary(unitary)
    dimension = real.shape[0]
    graph = graph or Graph(
        n=dimension,
        edges=tuple((index, index + 1) for index in range(dimension - 1)),
    )
    compiled = compile_so_on_tree(real, graph, tolerance=1.0e-8)
    replay = schedule_matrix(dimension, compiled.schedule)
    energy_errors: list[float] = []
    for basis_index in range(dimension):
        basis = np.zeros(dimension, dtype=np.float64)
        basis[basis_index] = 1.0
        transformed = replay @ basis
        energy_errors.append(abs(float(transformed @ transformed) - 1.0))
    return AcousticLowering(
        matrix=real,
        compile=compiled,
        certificate=AcousticLoweringCertificate(
            real_dimension=dimension,
            graph_edges=graph.edges,
            gate_count=compiled.gate_count,
            reconstruction_error_fro=compiled.reconstruction_error_fro,
            orthogonality_error_fro=compiled.orthogonality_error_fro,
            determinant_error=compiled.determinant_error,
            all_gates_on_graph=compiled.all_gates_on_graph,
            maximum_basis_energy_error=max(energy_errors),
        ),
    )


@dataclass(frozen=True)
class Z3PumpCertificate:
    cells: int
    dimension: int
    c3_order_three_error_fro: float
    c3_prime_order_three_error_fro: float
    commutator_norm_fro: float
    path_one_cell_shift: int
    path_two_cell_shift: int
    final_internal_state_match: bool
    inverse_error_fro: float


@dataclass(frozen=True)
class Z3PumpOperators:
    c3: FloatMatrix
    c3_prime: FloatMatrix
    certificate: Z3PumpCertificate


def z3_pump_operators(*, cells: int = 7) -> Z3PumpOperators:
    """Exact finite action reported for the two acoustic Z3 pump cycles.

    Basis order within each periodic cell is (+, -, link).  The orientation of
    the cell shift is conventional; changing it exchanges left and right.
    """
    if cells < 3:
        raise ValueError("at least three periodic cells are required")
    dimension = 3 * cells
    c3 = np.zeros((dimension, dimension), dtype=np.float64)
    c3_prime = np.zeros((dimension, dimension), dtype=np.float64)

    def index(cell: int, state: int) -> int:
        return (cell % cells) * 3 + state

    for cell in range(cells):
        # C3: + -> link -> - -> + within one cell.
        c3[index(cell, 2), index(cell, 0)] = 1.0
        c3[index(cell, 1), index(cell, 2)] = 1.0
        c3[index(cell, 0), index(cell, 1)] = 1.0

        # C3': +_N -> -_N -> link_{N-1} -> +_N.
        c3_prime[index(cell, 1), index(cell, 0)] = 1.0
        c3_prime[index(cell - 1, 2), index(cell, 1)] = 1.0
        c3_prime[index(cell + 1, 0), index(cell, 2)] = 1.0

    identity = np.eye(dimension)
    center = cells // 2
    initial = np.zeros(dimension)
    initial[index(center, 0)] = 1.0

    def execute(operators: Sequence[FloatMatrix]) -> NDArray[np.float64]:
        state = initial.copy()
        for operator in operators:
            state = operator @ state
        return state

    path_one = execute((c3, c3, c3_prime, c3))
    path_two = execute((c3_prime, c3, c3, c3))
    index_one = int(np.argmax(np.abs(path_one)))
    index_two = int(np.argmax(np.abs(path_two)))
    state_one = index_one % 3
    state_two = index_two % 3
    cell_one = index_one // 3
    cell_two = index_two // 3
    return Z3PumpOperators(
        c3=c3,
        c3_prime=c3_prime,
        certificate=Z3PumpCertificate(
            cells=cells,
            dimension=dimension,
            c3_order_three_error_fro=float(
                np.linalg.norm(np.linalg.matrix_power(c3, 3) - identity, ord="fro")
            ),
            c3_prime_order_three_error_fro=float(
                np.linalg.norm(
                    np.linalg.matrix_power(c3_prime, 3) - identity,
                    ord="fro",
                )
            ),
            commutator_norm_fro=float(
                np.linalg.norm(c3 @ c3_prime - c3_prime @ c3, ord="fro")
            ),
            path_one_cell_shift=cell_one - center,
            path_two_cell_shift=cell_two - center,
            final_internal_state_match=state_one == state_two,
            inverse_error_fro=float(
                max(
                    np.linalg.norm(np.linalg.matrix_power(c3, 2) - c3.T, ord="fro"),
                    np.linalg.norm(
                        np.linalg.matrix_power(c3_prime, 2) - c3_prime.T,
                        ord="fro",
                    ),
                )
            ),
        ),
    )


@dataclass(frozen=True)
class ProtectedPerturbationSummary:
    perturbations: int
    maximum_second_chern_error: float
    minimum_sampled_gap: float
    minimum_sampled_homotopy_gap: float
    minimum_nonabelian_commutator_norm_fro: float
    maximum_loop_unitarity_error_fro: float


@dataclass(frozen=True)
class ProtectedNonAbelianCertificate:
    model: str
    second_chern_convention: str
    analytic_second_chern: float
    numerical_second_chern: SecondChernEstimate
    trivial_second_chern: SecondChernEstimate
    critical_gap_at_origin: float
    loop_a: KatoLoopCertificate
    loop_b: KatoLoopCertificate
    loop_commutator_norm_fro: float
    loop_group_commutator_distance_fro: float
    loop_order_state_distance: float
    lowering_a: AcousticLoweringCertificate
    lowering_b: AcousticLoweringCertificate
    adiabatic: AdiabaticConvergenceCertificate
    perturbations: ProtectedPerturbationSummary
    z3_pump: Z3PumpCertificate
    claim_scope: tuple[str, ...]


@dataclass(frozen=True)
class ProtectedNonAbelianExperiment:
    certificate: ProtectedNonAbelianCertificate
    base_parameters: Dirac4DParameters
    base_point: tuple[float, float, float, float]
    loop_a_path: tuple[NDArray[np.float64], ...]
    loop_b_path: tuple[NDArray[np.float64], ...]
    loop_a: KatoLoop
    loop_b: KatoLoop
    lowering_a: AcousticLowering
    lowering_b: AcousticLowering
    z3: Z3PumpOperators


def _random_gap_preserving_parameters(
    rng: np.random.Generator,
) -> Dirac4DParameters:
    return Dirac4DParameters(
        mass=float(-3.0 + rng.uniform(-0.15, 0.15)),
        velocities=tuple(float(value) for value in (1.0 + rng.uniform(-0.12, 0.12, 4))),
        cosine_weights=tuple(float(value) for value in (1.0 + rng.uniform(-0.08, 0.08, 4))),
        coordinate_warp=tuple(float(value) for value in rng.uniform(-0.18, 0.18, 4)),
        warp_phase=tuple(float(value) for value in rng.uniform(-math.pi, math.pi, 4)),
    )


def verify_protected_nonabelian_instrument(
    *,
    grid: int = 12,
    perturbation_grid: int = 8,
    perturbations: int = 12,
    path_steps: int = 48,
    adiabatic_durations: Sequence[float] = (16.0, 32.0, 64.0, 128.0),
    seed: int = 0xC2A71AD,
) -> ProtectedNonAbelianExperiment:
    """Build the protected bundle, non-Abelian gates, and acoustic lowering."""
    base_parameters = Dirac4DParameters()
    analytic = analytic_second_chern(base_parameters.mass)
    numerical = second_chern_degree(base_parameters, grid=grid)
    trivial = second_chern_degree(
        Dirac4DParameters(mass=-5.0),
        grid=grid,
    )
    critical_d, _ = _dirac_d_and_jacobian(
        (0.0, 0.0, 0.0, 0.0),
        Dirac4DParameters(mass=-4.0),
    )
    critical_gap = 2.0 * float(np.linalg.norm(critical_d))

    base = (-0.4, 0.0, 0.9, -0.7)
    path_points = 4 * path_steps + 1
    path_a = torus_smooth_loop(
        base, 2, 3, 1.50, 1.20, points=path_points
    )
    path_b = torus_smooth_loop(
        base, 0, 3, 1.30, 1.00, points=path_points
    )
    loop_a = kato_wilson_loop(path_a, base_parameters)
    loop_b = kato_wilson_loop(path_b, base_parameters)
    occupied_a = loop_a.occupied_holonomy
    occupied_b = loop_b.occupied_holonomy
    commutator_norm = float(
        np.linalg.norm(
            occupied_a @ occupied_b - occupied_b @ occupied_a,
            ord="fro",
        )
    )
    group_commutator = (
        occupied_a
        @ occupied_b
        @ occupied_a.conj().T
        @ occupied_b.conj().T
    )
    group_commutator_distance = float(
        np.linalg.norm(group_commutator - np.eye(2), ord="fro")
    )

    _, base_frame = np.linalg.eigh(dirac4d_hamiltonian(base, base_parameters))
    initial = base_frame[:, 0]
    state_ab = loop_b.full_holonomy @ loop_a.full_holonomy @ initial
    state_ba = loop_a.full_holonomy @ loop_b.full_holonomy @ initial
    state_order_distance = float(np.linalg.norm(state_ab - state_ba))

    lowering_a = lower_unitary_to_acoustic_graph(loop_a.full_holonomy)
    lowering_b = lower_unitary_to_acoustic_graph(loop_b.full_holonomy)
    adiabatic = verify_adiabatic_convergence(
        path_a,
        path_b,
        base_parameters,
        durations=adiabatic_durations,
        reference_a=loop_a,
        reference_b=loop_b,
    )

    rng = np.random.default_rng(seed)
    c2_errors: list[float] = []
    sampled_gaps: list[float] = []
    homotopy_gaps: list[float] = []
    commutator_norms: list[float] = []
    unitarity_errors: list[float] = []
    for _ in range(perturbations):
        parameters = _random_gap_preserving_parameters(rng)
        estimate = second_chern_degree(parameters, grid=perturbation_grid)
        perturbed_a = kato_wilson_loop(path_a, parameters)
        perturbed_b = kato_wilson_loop(path_b, parameters)
        c2_errors.append(abs(estimate.estimate - analytic))
        sampled_gaps.append(estimate.sampled_gap)
        homotopy_gaps.append(
            sampled_homotopy_gap(
                parameters,
                grid=max(4, min(6, perturbation_grid)),
                homotopy_steps=5,
            )
        )
        commutator_norms.append(
            float(
                np.linalg.norm(
                    perturbed_a.occupied_holonomy @ perturbed_b.occupied_holonomy
                    - perturbed_b.occupied_holonomy @ perturbed_a.occupied_holonomy,
                    ord="fro",
                )
            )
        )
        unitarity_errors.extend(
            (
                perturbed_a.certificate.occupied_unitarity_error_fro,
                perturbed_b.certificate.occupied_unitarity_error_fro,
            )
        )

    z3 = z3_pump_operators(cells=7)
    perturbation_summary = ProtectedPerturbationSummary(
        perturbations=perturbations,
        maximum_second_chern_error=max(c2_errors, default=0.0),
        minimum_sampled_gap=min(sampled_gaps, default=numerical.sampled_gap),
        minimum_sampled_homotopy_gap=min(
            homotopy_gaps, default=numerical.sampled_gap
        ),
        minimum_nonabelian_commutator_norm_fro=min(
            commutator_norms, default=commutator_norm
        ),
        maximum_loop_unitarity_error_fro=max(unitarity_errors, default=0.0),
    )

    certificate = ProtectedNonAbelianCertificate(
        model="4D Wilson-Dirac rank-two bundle + exact acoustic Z3 pump algebra",
        second_chern_convention="degree(d/|d|): T4 -> S4",
        analytic_second_chern=analytic,
        numerical_second_chern=numerical,
        trivial_second_chern=trivial,
        critical_gap_at_origin=critical_gap,
        loop_a=loop_a.certificate,
        loop_b=loop_b.certificate,
        loop_commutator_norm_fro=commutator_norm,
        loop_group_commutator_distance_fro=group_commutator_distance,
        loop_order_state_distance=state_order_distance,
        lowering_a=lowering_a.certificate,
        lowering_b=lowering_b.certificate,
        adiabatic=adiabatic,
        perturbations=perturbation_summary,
        z3_pump=z3.certificate,
        claim_scope=(
            "The rank-two bundle has nonzero second Chern number and remains in the same phase under the tested gap-preserving homotopy.",
            "The two Kato/Wilson gates are noncommuting and remain noncommuting over the declared perturbation family.",
            "Each full-space unitary is exactly lowered to a zero-work real orthogonal graph program.",
            "Individual Wilson matrices are geometric and path-dependent; this is not a fault-tolerant or quantized gate claim.",
            "The Z3 operators are an exact algebraic acoustic-pump target; this package does not rederive the published waveguide Hamiltonian.",
        ),
    )
    return ProtectedNonAbelianExperiment(
        certificate=certificate,
        base_parameters=base_parameters,
        base_point=base,
        loop_a_path=path_a,
        loop_b_path=path_b,
        loop_a=loop_a,
        loop_b=loop_b,
        lowering_a=lowering_a,
        lowering_b=lowering_b,
        z3=z3,
    )
