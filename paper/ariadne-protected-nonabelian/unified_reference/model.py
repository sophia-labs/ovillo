"""Certified finite-dimensional acoustic family for the unified Ariadne instrument.

The primary state is a vector of positive-frequency, energy-normalized complex
modal amplitudes.  Four physical regions (string, body, near field, late room)
carry four internal channels each.  The Wilson--Dirac block is stabilized by
three trivial four-mode bands and then distributed over the physical regions by
an explicitly unitary, graph-local regional mixer.

This file intentionally contains no audio-host or MUS dependencies.  It is the
construction-time mathematical oracle that production code must replay.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import IntEnum
import math
from typing import Iterable, Sequence

import numpy as np
from numpy.typing import NDArray

ComplexMatrix = NDArray[np.complex128]
FloatMatrix = NDArray[np.float64]
ComplexVector = NDArray[np.complex128]


class Region(IntEnum):
    STRING = 0
    BODY = 1
    NEAR_FIELD = 2
    LATE_ROOM = 3


REGION_NAMES = ("string", "body", "near_field", "late_room")
REGION_COUNT = 4
INTERNAL_MODES = 4
COMPLEX_DIMENSION = REGION_COUNT * INTERNAL_MODES
REAL_DIMENSION = 2 * COMPLEX_DIMENSION


def _pauli() -> tuple[ComplexMatrix, ComplexMatrix, ComplexMatrix, ComplexMatrix]:
    sx = np.array([[0.0, 1.0], [1.0, 0.0]], dtype=np.complex128)
    sy = np.array([[0.0, -1.0j], [1.0j, 0.0]], dtype=np.complex128)
    sz = np.array([[1.0, 0.0], [0.0, -1.0]], dtype=np.complex128)
    identity = np.eye(2, dtype=np.complex128)
    return sx, sy, sz, identity


def clifford_gammas() -> tuple[ComplexMatrix, ...]:
    """Hermitian 4x4 representation of Cl(5)."""
    sx, sy, sz, identity = _pauli()
    return (
        np.kron(sx, sx),
        np.kron(sx, sy),
        np.kron(sx, sz),
        np.kron(sy, identity),
        np.kron(sz, identity),
    )


GAMMAS = clifford_gammas()
IDENTITY4 = np.eye(4, dtype=np.complex128)
IDENTITY16 = np.eye(COMPLEX_DIMENSION, dtype=np.complex128)


@dataclass(frozen=True)
class FamilyParameters:
    """Content-addressable parameters of the protected modal instrument.

    The topological statements concern the dimensionless relative Hamiltonian.
    ``carrier_hz`` and ``dirac_scale_hz`` only map that object to physical time.
    """

    mass: float = -3.0
    carrier_hz: float = 440.0
    dirac_scale_hz: float = 20.0
    trivial_offsets: tuple[float, float, float] = (9.0, 11.0, 13.0)
    coupling_fraction: float = 1.0

    def validate(self) -> None:
        if not math.isfinite(self.mass):
            raise ValueError("mass must be finite")
        if not (self.carrier_hz > 0.0 and self.dirac_scale_hz > 0.0):
            raise ValueError("carrier and scale must be positive")
        if len(self.trivial_offsets) != 3 or not all(
            math.isfinite(value) for value in self.trivial_offsets
        ):
            raise ValueError("three finite trivial offsets are required")
        if min(self.trivial_offsets) <= 7.0:
            raise ValueError(
                "trivial offsets must exceed the undeformed active upper edge 7"
            )
        if not (0.0 <= self.coupling_fraction <= 1.0):
            raise ValueError("coupling_fraction must lie in [0, 1]")
        # Keep every physical frequency positive for the certified r <= 7 band.
        if self.carrier_hz <= 7.0 * self.dirac_scale_hz:
            raise ValueError("carrier must exceed seven Dirac scale units")


def wrap_torus(momentum: Sequence[float]) -> NDArray[np.float64]:
    value = np.asarray(momentum, dtype=np.float64)
    if value.shape != (4,) or not np.all(np.isfinite(value)):
        raise ValueError("momentum must contain four finite values")
    return (value + math.pi) % (2.0 * math.pi) - math.pi


def dirac_d(momentum: Sequence[float], *, mass: float = -3.0) -> NDArray[np.float64]:
    k = wrap_torus(momentum)
    return np.concatenate((np.sin(k), [mass + float(np.sum(np.cos(k)))]))


def dirac_hamiltonian(momentum: Sequence[float], *, mass: float = -3.0) -> ComplexMatrix:
    d = dirac_d(momentum, mass=mass)
    out = np.zeros((4, 4), dtype=np.complex128)
    for coefficient, gamma in zip(d, GAMMAS, strict=True):
        out += coefficient * gamma
    return out


def occupied_projector(momentum: Sequence[float], *, mass: float = -3.0) -> ComplexMatrix:
    d = dirac_d(momentum, mass=mass)
    radius = float(np.linalg.norm(d))
    if radius <= 1.0e-14:
        raise ValueError("occupied projector is undefined at a closed gap")
    return 0.5 * (IDENTITY4 - dirac_hamiltonian(momentum, mass=mass) / radius)


def exact_base_radius_squared(momentum: Sequence[float]) -> float:
    """Exact algebraic form for m=-3: |d|^2 = 1 + 2 sum_{i<j} t_i t_j.

    Here t_i=1-cos(k_i) lies in [0,2], proving 1 <= |d| <= 7 globally.
    """
    k = wrap_torus(momentum)
    t = 1.0 - np.cos(k)
    value = 1.0
    for i in range(4):
        for j in range(i + 1, 4):
            value += 2.0 * float(t[i] * t[j])
    return value


def exact_base_gap_bounds() -> tuple[float, float, float]:
    """Return (minimum radius, maximum radius, protected inter-band gap)."""
    return 1.0, 7.0, 2.0


def plane_unitary(n: int, i: int, j: int, angle: float) -> ComplexMatrix:
    if not (0 <= i < n and 0 <= j < n and i != j):
        raise ValueError("invalid plane")
    c = math.cos(float(angle))
    s = math.sin(float(angle))
    out = np.eye(n, dtype=np.complex128)
    out[i, i] = c
    out[i, j] = -s
    out[j, i] = s
    out[j, j] = c
    return out


def regional_mixer(coupling_fraction: float = 1.0) -> ComplexMatrix:
    """Four-region butterfly made from four lawful two-mode beam splitters.

    At zero it is the decoupled regional limit.  At one, every vector initially
    in the first supermode region has equal 1/4 energy participation in the
    four physical regions.  Intermediate values provide an exact isospectral
    coupling homotopy.
    """
    if not (0.0 <= coupling_fraction <= 1.0):
        raise ValueError("coupling_fraction must lie in [0,1]")
    angle = coupling_fraction * math.pi / 4.0
    stages = (
        plane_unitary(4, 0, 1, angle),
        plane_unitary(4, 2, 3, angle),
        plane_unitary(4, 0, 2, angle),
        plane_unitary(4, 1, 3, angle),
    )
    total = np.eye(4, dtype=np.complex128)
    for stage in stages:
        total = stage @ total
    return total


def physical_mixer(coupling_fraction: float = 1.0) -> ComplexMatrix:
    return np.kron(regional_mixer(coupling_fraction), IDENTITY4)


def stabilized_hamiltonian(
    momentum: Sequence[float], parameters: FamilyParameters = FamilyParameters()
) -> ComplexMatrix:
    """Dimensionless block family before physical regional coupling."""
    parameters.validate()
    blocks = [dirac_hamiltonian(momentum, mass=parameters.mass)]
    blocks.extend(
        offset * IDENTITY4 for offset in parameters.trivial_offsets
    )
    out = np.zeros((COMPLEX_DIMENSION, COMPLEX_DIMENSION), dtype=np.complex128)
    cursor = 0
    for block in blocks:
        width = block.shape[0]
        out[cursor : cursor + width, cursor : cursor + width] = block
        cursor += width
    return out


def relative_hamiltonian(
    momentum: Sequence[float], parameters: FamilyParameters = FamilyParameters()
) -> ComplexMatrix:
    """Frozen coupled conservative family K(lambda), in scale units."""
    base = stabilized_hamiltonian(momentum, parameters)
    mixer = physical_mixer(parameters.coupling_fraction)
    return mixer @ base @ mixer.conj().T


def physical_hamiltonian_rad_s(
    momentum: Sequence[float], parameters: FamilyParameters = FamilyParameters()
) -> ComplexMatrix:
    """Self-adjoint positive-frequency generator in radians per second."""
    parameters.validate()
    return 2.0 * math.pi * (
        parameters.carrier_hz * IDENTITY16
        + parameters.dirac_scale_hz * relative_hamiltonian(momentum, parameters)
    )


def protected_projector(
    momentum: Sequence[float], parameters: FamilyParameters = FamilyParameters()
) -> ComplexMatrix:
    parameters.validate()
    raw = np.zeros((COMPLEX_DIMENSION, COMPLEX_DIMENSION), dtype=np.complex128)
    raw[:4, :4] = occupied_projector(momentum, mass=parameters.mass)
    mixer = physical_mixer(parameters.coupling_fraction)
    return mixer @ raw @ mixer.conj().T


def region_slice(region: int | Region) -> slice:
    index = int(region)
    if not (0 <= index < REGION_COUNT):
        raise ValueError("invalid region")
    start = index * INTERNAL_MODES
    return slice(start, start + INTERNAL_MODES)


def region_participation(projector: ComplexMatrix) -> tuple[float, float, float, float]:
    projector = np.asarray(projector, dtype=np.complex128)
    if projector.shape != (COMPLEX_DIMENSION, COMPLEX_DIMENSION):
        raise ValueError("projector has the wrong shape")
    rank = float(np.trace(projector).real)
    if rank <= 0.0:
        raise ValueError("projector rank must be positive")
    values = []
    for region in Region:
        block = projector[region_slice(region), region_slice(region)]
        values.append(float(np.trace(block).real / rank))
    return tuple(values)  # type: ignore[return-value]


def realify(matrix: ComplexMatrix) -> FloatMatrix:
    matrix = np.asarray(matrix, dtype=np.complex128)
    return np.block(
        [[matrix.real, -matrix.imag], [matrix.imag, matrix.real]]
    ).astype(np.float64)


def complex_structure(dimension: int) -> FloatMatrix:
    identity = np.eye(dimension, dtype=np.float64)
    zero = np.zeros_like(identity)
    return np.block([[zero, -identity], [identity, zero]])


def real_generator_from_hamiltonian(hamiltonian: ComplexMatrix) -> FloatMatrix:
    """Real skew generator for z_dot=-i H z on [Re z, Im z]."""
    return realify(-1.0j * np.asarray(hamiltonian, dtype=np.complex128))


def unitary_law_residuals(unitary: ComplexMatrix) -> dict[str, float]:
    unitary = np.asarray(unitary, dtype=np.complex128)
    n = unitary.shape[0]
    if unitary.shape != (n, n):
        raise ValueError("unitary must be square")
    real = realify(unitary)
    identity = np.eye(2 * n)
    structure = complex_structure(n)
    return {
        "complex_unitarity_fro": float(
            np.linalg.norm(unitary.conj().T @ unitary - np.eye(n), ord="fro")
        ),
        "real_orthogonality_fro": float(
            np.linalg.norm(real.T @ real - identity, ord="fro")
        ),
        "real_symplectic_fro": float(
            np.linalg.norm(real.T @ structure @ real - structure, ord="fro")
        ),
        "complex_structure_commutator_fro": float(
            np.linalg.norm(real @ structure - structure @ real, ord="fro")
        ),
    }


def ordinary_real_givens_symplectic_residual(
    real_dimension: int = 8, i: int = 0, j: int = 1, angle: float = 0.37
) -> float:
    """Counterexample metric for an arbitrary SO(2n) factor.

    Coordinates are ordered [Re z, Im z].  A Givens factor that rotates only
    two real coordinates generally fails to preserve the complex structure.
    """
    if real_dimension % 2:
        raise ValueError("real_dimension must be even")
    gate = np.eye(real_dimension)
    c = math.cos(angle)
    s = math.sin(angle)
    gate[i, i] = c
    gate[i, j] = -s
    gate[j, i] = s
    gate[j, j] = c
    structure = complex_structure(real_dimension // 2)
    return float(np.linalg.norm(gate.T @ structure @ gate - structure, ord="fro"))


def pseudo_time_reversal_unitary() -> ComplexMatrix:
    """Unitary part of T=(I_region tensor I tensor sigma_y) complex conjugation."""
    _, sy, _, identity = _pauli()
    internal = np.kron(identity, sy)
    return np.kron(np.eye(REGION_COUNT, dtype=np.complex128), internal)


def pseudo_time_reversal_residual(
    momentum: Sequence[float], parameters: FamilyParameters = FamilyParameters()
) -> float:
    t_unitary = pseudo_time_reversal_unitary()
    left = t_unitary @ relative_hamiltonian(momentum, parameters).conj() @ t_unitary.conj().T
    right = relative_hamiltonian(-wrap_torus(momentum), parameters)
    return float(np.linalg.norm(left - right, ord="fro"))


def pseudo_time_reversal_square_residual() -> float:
    unitary = pseudo_time_reversal_unitary()
    return float(
        np.linalg.norm(unitary @ unitary.conj() + np.eye(COMPLEX_DIMENSION), ord="fro")
    )


def regional_coupling_norms(
    momentum: Sequence[float], parameters: FamilyParameters = FamilyParameters()
) -> dict[str, float]:
    hamiltonian = relative_hamiltonian(momentum, parameters)
    out: dict[str, float] = {}
    for source in Region:
        for target in Region:
            if source >= target:
                continue
            block = hamiltonian[region_slice(source), region_slice(target)]
            out[f"{REGION_NAMES[int(source)]}<->{REGION_NAMES[int(target)]}"] = float(
                np.linalg.norm(block, ord="fro")
            )
    return out


def normalized_energy(state: ComplexVector) -> float:
    state = np.asarray(state, dtype=np.complex128)
    return float(np.vdot(state, state).real)
