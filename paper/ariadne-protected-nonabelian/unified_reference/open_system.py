"""Passive ports and a single acoustic energy ledger."""

from __future__ import annotations

from dataclasses import asdict, dataclass, field
import math
from typing import Iterable, Literal, Sequence

import numpy as np
from numpy.typing import NDArray

from .model import COMPLEX_DIMENSION, normalized_energy

ComplexMatrix = NDArray[np.complex128]
ComplexVector = NDArray[np.complex128]


@dataclass(frozen=True)
class LedgerFrame:
    label: str
    initial_energy: float
    final_energy: float
    source_work: float = 0.0
    control_work: float = 0.0
    metric_work: float = 0.0
    internal_loss: float = 0.0
    radiation: float = 0.0
    retirement_radiation: float = 0.0
    retirement_dissipation: float = 0.0
    numerical_residual: float = 0.0
    actuator_overhead_external: float = 0.0

    @property
    def accounted_final_energy(self) -> float:
        return (
            self.initial_energy
            + self.source_work
            + self.control_work
            + self.metric_work
            - self.internal_loss
            - self.radiation
            - self.retirement_radiation
            - self.retirement_dissipation
            + self.numerical_residual
        )

    @property
    def closure_error(self) -> float:
        return self.final_energy - self.accounted_final_energy


@dataclass
class EnergyLedger:
    frames: list[LedgerFrame] = field(default_factory=list)

    def append(self, frame: LedgerFrame) -> None:
        self.frames.append(frame)

    def totals(self) -> dict[str, float]:
        if not self.frames:
            return {
                "initial_energy": 0.0,
                "final_energy": 0.0,
                "source_work": 0.0,
                "control_work": 0.0,
                "metric_work": 0.0,
                "internal_loss": 0.0,
                "radiation": 0.0,
                "retirement_radiation": 0.0,
                "retirement_dissipation": 0.0,
                "numerical_residual": 0.0,
                "actuator_overhead_external": 0.0,
                "closure_error": 0.0,
            }
        total = {
            "initial_energy": self.frames[0].initial_energy,
            "final_energy": self.frames[-1].final_energy,
        }
        for key in (
            "source_work",
            "control_work",
            "metric_work",
            "internal_loss",
            "radiation",
            "retirement_radiation",
            "retirement_dissipation",
            "numerical_residual",
            "actuator_overhead_external",
        ):
            total[key] = float(sum(getattr(frame, key) for frame in self.frames))
        total["closure_error"] = float(sum(frame.closure_error for frame in self.frames))
        return total

    def to_json_dict(self) -> dict:
        return {
            "frames": [asdict(frame) | {"closure_error": frame.closure_error} for frame in self.frames],
            "totals": self.totals(),
        }


def inject_source(
    state: ComplexVector, source: ComplexVector, *, label: str = "source"
) -> tuple[ComplexVector, LedgerFrame]:
    state = np.asarray(state, dtype=np.complex128)
    source = np.asarray(source, dtype=np.complex128)
    if state.shape != source.shape:
        raise ValueError("source and state shapes must match")
    initial = normalized_energy(state)
    updated = state + source
    final = normalized_energy(updated)
    work = final - initial
    return updated, LedgerFrame(
        label=label,
        initial_energy=initial,
        final_energy=final,
        source_work=work,
        numerical_residual=0.0,
    )


def apply_unitary_control(
    state: ComplexVector,
    unitary: ComplexMatrix,
    *,
    label: str = "unitary_control",
    actuator_overhead_external: float = 0.0,
) -> tuple[ComplexVector, LedgerFrame]:
    state = np.asarray(state, dtype=np.complex128)
    unitary = np.asarray(unitary, dtype=np.complex128)
    initial = normalized_energy(state)
    updated = unitary @ state
    final = normalized_energy(updated)
    residual = final - initial
    return updated, LedgerFrame(
        label=label,
        initial_energy=initial,
        final_energy=final,
        control_work=0.0,
        numerical_residual=residual,
        actuator_overhead_external=float(actuator_overhead_external),
    )


def apply_internal_loss(
    state: ComplexVector,
    amplitude_retention: Sequence[float] | float,
    *,
    label: str = "internal_loss",
) -> tuple[ComplexVector, LedgerFrame]:
    state = np.asarray(state, dtype=np.complex128)
    retention = np.asarray(amplitude_retention, dtype=np.float64)
    if retention.ndim == 0:
        retention = np.full(state.shape[0], float(retention))
    if retention.shape != state.shape or np.any(retention < 0.0) or np.any(retention > 1.0):
        raise ValueError("retention must lie in [0,1] and match state")
    initial = normalized_energy(state)
    updated = retention * state
    final = normalized_energy(updated)
    return updated, LedgerFrame(
        label=label,
        initial_energy=initial,
        final_energy=final,
        internal_loss=initial - final,
    )


@dataclass(frozen=True)
class PassiveRadiationPort:
    """Two or more orthonormal outgoing directions with exact complement map."""

    directions: ComplexMatrix
    energy_coupling: float

    def __post_init__(self) -> None:
        directions = np.asarray(self.directions, dtype=np.complex128)
        if directions.ndim != 2 or directions.shape[1] != COMPLEX_DIMENSION:
            raise ValueError("radiation directions must have shape (ports,16)")
        if np.linalg.norm(directions @ directions.conj().T - np.eye(directions.shape[0]), ord="fro") > 1.0e-9:
            raise ValueError("radiation rows must be orthonormal")
        if not (0.0 <= self.energy_coupling <= 1.0):
            raise ValueError("energy_coupling must lie in [0,1]")
        object.__setattr__(self, "directions", directions)

    @property
    def projector(self) -> ComplexMatrix:
        return self.directions.conj().T @ self.directions

    @property
    def retained_operator(self) -> ComplexMatrix:
        coefficient = math.sqrt(1.0 - self.energy_coupling) - 1.0
        return np.eye(COMPLEX_DIMENSION, dtype=np.complex128) + coefficient * self.projector

    @property
    def outgoing_operator(self) -> ComplexMatrix:
        return math.sqrt(self.energy_coupling) * self.directions

    def scatter(
        self, state: ComplexVector, *, label: str = "radiation"
    ) -> tuple[ComplexVector, ComplexVector, LedgerFrame]:
        state = np.asarray(state, dtype=np.complex128)
        initial = normalized_energy(state)
        outgoing = self.outgoing_operator @ state
        retained = self.retained_operator @ state
        final = normalized_energy(retained)
        radiation = normalized_energy(outgoing)
        residual = initial - final - radiation
        return retained, outgoing, LedgerFrame(
            label=label,
            initial_energy=initial,
            final_energy=final,
            radiation=radiation,
            numerical_residual=-residual,
        )

    def law_residual(self) -> float:
        retained = self.retained_operator
        outgoing = self.outgoing_operator
        return float(
            np.linalg.norm(
                retained.conj().T @ retained
                + outgoing.conj().T @ outgoing
                - np.eye(COMPLEX_DIMENSION),
                ord="fro",
            )
        )


def retire_modes(
    state: ComplexVector,
    indices: Iterable[int],
    *,
    policy: Literal["radiate", "dissipate", "reject"] = "reject",
    label: str = "retirement",
) -> tuple[ComplexVector, LedgerFrame]:
    state = np.asarray(state, dtype=np.complex128)
    selected = tuple(sorted(set(int(index) for index in indices)))
    if any(index < 0 or index >= state.shape[0] for index in selected):
        raise ValueError("retired index out of range")
    if policy == "reject":
        raise ValueError("rank contraction requires radiate or dissipate policy")
    initial = normalized_energy(state)
    removed = float(sum(abs(state[index]) ** 2 for index in selected))
    updated = state.copy()
    updated[list(selected)] = 0.0
    final = normalized_energy(updated)
    return updated, LedgerFrame(
        label=label,
        initial_energy=initial,
        final_energy=final,
        retirement_radiation=removed if policy == "radiate" else 0.0,
        retirement_dissipation=removed if policy == "dissipate" else 0.0,
        numerical_residual=final - (initial - removed),
    )


def observer_tap(state: ComplexVector, rows: ComplexMatrix) -> ComplexVector:
    """Observer-only read; it never enters the acoustic energy ledger."""
    state = np.asarray(state, dtype=np.complex128)
    rows = np.asarray(rows, dtype=np.complex128)
    return rows @ state
