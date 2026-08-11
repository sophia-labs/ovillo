"""Persistent coupled instrument, observability, causality, and audible readout."""

from __future__ import annotations

from dataclasses import asdict, dataclass, field
import math
from pathlib import Path
import wave
from typing import Sequence

import numpy as np
from numpy.typing import NDArray

from .model import (
    COMPLEX_DIMENSION,
    FamilyParameters,
    Region,
    normalized_energy,
    physical_hamiltonian_rad_s,
    protected_projector,
    region_participation,
    region_slice,
    relative_hamiltonian,
)
from .open_system import (
    EnergyLedger,
    LedgerFrame,
    PassiveRadiationPort,
    apply_unitary_control,
    inject_source,
)
from .transport import KatoPath

ComplexMatrix = NDArray[np.complex128]
ComplexVector = NDArray[np.complex128]
FloatMatrix = NDArray[np.float64]


def region_rows(region: int | Region) -> ComplexMatrix:
    rows = np.zeros((4, COMPLEX_DIMENSION), dtype=np.complex128)
    rows[:, region_slice(region)] = np.eye(4, dtype=np.complex128)
    return rows


@dataclass
class PersistentInstrument:
    parameters: FamilyParameters = field(default_factory=FamilyParameters)
    state: ComplexVector = field(
        default_factory=lambda: np.zeros(COMPLEX_DIMENSION, dtype=np.complex128)
    )
    ledger: EnergyLedger = field(default_factory=EnergyLedger)

    def __post_init__(self) -> None:
        self.parameters.validate()
        self.state = np.asarray(self.state, dtype=np.complex128).copy()
        if self.state.shape != (COMPLEX_DIMENSION,):
            raise ValueError("instrument state must contain sixteen complex modes")

    def reset(self, state: ComplexVector | None = None) -> None:
        self.state = (
            np.zeros(COMPLEX_DIMENSION, dtype=np.complex128)
            if state is None
            else np.asarray(state, dtype=np.complex128).copy()
        )
        if self.state.shape != (COMPLEX_DIMENSION,):
            raise ValueError("reset state has wrong shape")
        self.ledger = EnergyLedger()

    def inject(self, source: ComplexVector, *, label: str = "source") -> None:
        self.state, frame = inject_source(self.state, source, label=label)
        self.ledger.append(frame)

    def apply_unitary(
        self,
        unitary: ComplexMatrix,
        *,
        label: str,
        actuator_overhead_external: float = 0.0,
    ) -> None:
        self.state, frame = apply_unitary_control(
            self.state,
            unitary,
            label=label,
            actuator_overhead_external=actuator_overhead_external,
        )
        self.ledger.append(frame)

    def execute_kato_path(self, path: KatoPath) -> None:
        for index, step in enumerate(path.steps):
            self.apply_unitary(step, label=f"kato:{path.spec.name}:{index}")

    def scatter(self, port: PassiveRadiationPort, *, label: str) -> ComplexVector:
        self.state, outgoing, frame = port.scatter(self.state, label=label)
        self.ledger.append(frame)
        return outgoing


@dataclass(frozen=True)
class ObservabilityCertificate:
    order_convention: str
    invariant_commutator_operator_norm: float
    raw_string_to_room_shape: tuple[int, int]
    raw_string_to_room_singular_values: tuple[float, float, float, float]
    raw_top_transfer_gain: float
    raw_blind_transfer_gain: float
    raw_best_input_protected_fraction: float
    calibrated_transfer_gain: float
    calibrated_difference_outside_room_fro: float
    calibrated_prepared_state_outside_protected_fro: float
    source_port: str
    radiation_port: str
    preparation_unitarity_error_fro: float
    readout_unitarity_error_fro: float
    source_vector: tuple[tuple[float, float], ...]
    prepared_vector: tuple[tuple[float, float], ...]
    raw_best_input_string_vector: tuple[tuple[float, float], ...]
    first_output_room_vector: tuple[tuple[float, float], ...]


@dataclass(frozen=True)
class ObservableDesign:
    certificate: ObservabilityCertificate
    source_vector: ComplexVector
    prepared_vector: ComplexVector
    preparation_unitary: ComplexMatrix
    readout_unitary: ComplexMatrix
    output_directions: ComplexMatrix
    raw_transfer: ComplexMatrix
    calibrated_transfer: ComplexMatrix


def _complex_pairs(vector: ComplexVector) -> tuple[tuple[float, float], ...]:
    return tuple((float(value.real), float(value.imag)) for value in vector)


def _deterministic_frame(first: ComplexVector) -> ComplexMatrix:
    """Complete one normalized vector to a deterministic orthonormal frame."""
    first = np.asarray(first, dtype=np.complex128)
    norm = float(np.linalg.norm(first))
    if first.shape != (COMPLEX_DIMENSION,) or norm <= 1.0e-14:
        raise ValueError("frame seed must be a nonzero instrument vector")
    columns: list[ComplexVector] = [first / norm]
    for index in range(COMPLEX_DIMENSION):
        candidate = np.zeros(COMPLEX_DIMENSION, dtype=np.complex128)
        candidate[index] = 1.0
        for column in columns:
            candidate -= column * np.vdot(column, candidate)
        candidate_norm = float(np.linalg.norm(candidate))
        if candidate_norm > 1.0e-12:
            columns.append(candidate / candidate_norm)
        if len(columns) == COMPLEX_DIMENSION:
            break
    if len(columns) != COMPLEX_DIMENSION:
        raise RuntimeError("failed to complete deterministic unitary frame")
    frame = np.column_stack(columns)
    if np.linalg.norm(frame.conj().T @ frame - np.eye(COMPLEX_DIMENSION), ord="fro") > 1.0e-10:
        raise RuntimeError("deterministic frame lost orthonormality")
    return frame


def unitary_map_vector(source: ComplexVector, target: ComplexVector) -> ComplexMatrix:
    """Deterministic unitary U with U source = target for equal-norm vectors."""
    source = np.asarray(source, dtype=np.complex128)
    target = np.asarray(target, dtype=np.complex128)
    source_norm = float(np.linalg.norm(source))
    target_norm = float(np.linalg.norm(target))
    if source.shape != target.shape or source.shape != (COMPLEX_DIMENSION,):
        raise ValueError("source and target must be instrument vectors")
    if source_norm <= 1.0e-14 or target_norm <= 1.0e-14:
        raise ValueError("source and target must be nonzero")
    if abs(source_norm - target_norm) > 1.0e-10 * max(source_norm, target_norm):
        raise ValueError("a unitary map requires equal vector norms")
    source_frame = _deterministic_frame(source)
    target_frame = _deterministic_frame(target)
    unitary = target_frame @ source_frame.conj().T
    if np.linalg.norm(unitary @ source - target, ord=2) > 1.0e-10:
        raise RuntimeError("unitary vector calibration failed")
    return unitary


def design_order_observable(loop_a: KatoPath, loop_b: KatoPath) -> ObservableDesign:
    """Design both raw and calibrated physical observability receipts.

    The raw receipt exposes what a direct string-local excitation and room-local
    pickup can and cannot see.  The calibrated experiment begins at one string
    source port, applies one fixed graph-lowerable preparation unitary into the
    protected doublet, executes AB or BA, and applies one fixed readout unitary
    before a two-port room radiation junction.  The preparation/readout maps do
    not depend on order, so they cannot carry the AB/BA distinction themselves.
    """
    string = region_rows(Region.STRING)
    room = region_rows(Region.LATE_ROOM)
    # Execution order: A then B gives W_B W_A.
    difference = (
        loop_b.full_holonomy @ loop_a.full_holonomy
        - loop_a.full_holonomy @ loop_b.full_holonomy
    )
    projector = protected_projector(loop_a.points[0])
    protected_difference = projector @ difference @ projector

    raw_transfer = room @ protected_difference @ string.conj().T
    _, raw_singular_values, raw_right = np.linalg.svd(raw_transfer)
    raw_input_local = raw_right.conj().T[:, 0]
    raw_input = string.conj().T @ raw_input_local
    raw_protected_fraction = normalized_energy(projector @ raw_input) / normalized_energy(raw_input)
    raw_padded = np.pad(
        raw_singular_values, (0, max(0, 4 - len(raw_singular_values)))
    )[:4]

    # Gauge-free protected singular vector.  It is a vector in the physical
    # sixteen-mode state, not an occupied-frame coordinate.
    _, protected_singular_values, protected_right = np.linalg.svd(protected_difference)
    prepared_vector = protected_right.conj().T[:, 0]
    prepared_vector /= np.linalg.norm(prepared_vector)
    source_vector = np.zeros(COMPLEX_DIMENSION, dtype=np.complex128)
    source_vector[region_slice(Region.STRING).start] = 1.0
    preparation = unitary_map_vector(source_vector, prepared_vector)

    calibrated_difference = protected_difference @ prepared_vector
    calibrated_gain = float(np.linalg.norm(calibrated_difference))
    if calibrated_gain <= 1.0e-14:
        raise RuntimeError("selected protected paths have no calibratable order defect")
    first_room_vector = np.zeros(COMPLEX_DIMENSION, dtype=np.complex128)
    first_room_vector[region_slice(Region.LATE_ROOM).start] = 1.0
    readout = unitary_map_vector(
        calibrated_difference / calibrated_gain, first_room_vector
    )
    output_directions = np.zeros((2, COMPLEX_DIMENSION), dtype=np.complex128)
    output_directions[0, region_slice(Region.LATE_ROOM).start] = 1.0
    output_directions[1, region_slice(Region.LATE_ROOM).start + 1] = 1.0
    calibrated_transfer = (
        output_directions
        @ readout
        @ difference
        @ preparation
        @ source_vector[:, None]
    )
    calibrated_delta = readout @ difference @ prepared_vector
    room_projector = output_directions.conj().T @ output_directions
    certificate = ObservabilityCertificate(
        order_convention="AB means execute A, then execute B: W_B W_A",
        invariant_commutator_operator_norm=float(
            np.linalg.norm(protected_difference, ord=2)
        ),
        raw_string_to_room_shape=raw_transfer.shape,
        raw_string_to_room_singular_values=tuple(
            float(value) for value in raw_padded
        ),  # type: ignore[arg-type]
        raw_top_transfer_gain=float(raw_singular_values[0]),
        raw_blind_transfer_gain=float(raw_padded[-1]),
        raw_best_input_protected_fraction=float(raw_protected_fraction),
        calibrated_transfer_gain=float(abs(calibrated_transfer[0, 0])),
        calibrated_difference_outside_room_fro=float(
            np.linalg.norm((np.eye(COMPLEX_DIMENSION) - room_projector) @ calibrated_delta)
        ),
        calibrated_prepared_state_outside_protected_fro=float(
            np.linalg.norm((np.eye(COMPLEX_DIMENSION) - projector) @ prepared_vector)
        ),
        source_port="string/channel-0",
        radiation_port="late-room/channels-0-1",
        preparation_unitarity_error_fro=float(
            np.linalg.norm(
                preparation.conj().T @ preparation - np.eye(COMPLEX_DIMENSION),
                ord="fro",
            )
        ),
        readout_unitarity_error_fro=float(
            np.linalg.norm(
                readout.conj().T @ readout - np.eye(COMPLEX_DIMENSION),
                ord="fro",
            )
        ),
        source_vector=_complex_pairs(source_vector),
        prepared_vector=_complex_pairs(prepared_vector),
        raw_best_input_string_vector=_complex_pairs(raw_input),
        first_output_room_vector=_complex_pairs(output_directions[0]),
    )
    return ObservableDesign(
        certificate=certificate,
        source_vector=source_vector,
        prepared_vector=prepared_vector,
        preparation_unitary=preparation,
        readout_unitary=readout,
        output_directions=output_directions,
        raw_transfer=raw_transfer,
        calibrated_transfer=calibrated_transfer,
    )


@dataclass(frozen=True)
class PersistenceCertificate:
    segmented_vs_concatenated_error: float
    direct_vs_step_replay_error: float
    reset_between_loops_difference: float
    initial_energy: float
    final_energy: float
    ledger_closure_error: float


def verify_persistence(
    loop_a: KatoPath, loop_b: KatoPath, input_vector: ComplexVector
) -> PersistenceCertificate:
    segmented = PersistentInstrument(state=input_vector.copy())
    segmented.execute_kato_path(loop_a)
    segmented.execute_kato_path(loop_b)

    concatenated = PersistentInstrument(state=input_vector.copy())
    for index, step in enumerate(loop_a.steps + loop_b.steps):
        concatenated.apply_unitary(step, label=f"concatenated:{index}")

    direct = loop_b.full_holonomy @ loop_a.full_holonomy @ input_vector
    reset = PersistentInstrument(state=input_vector.copy())
    reset.execute_kato_path(loop_a)
    reset.reset(input_vector.copy())
    reset.execute_kato_path(loop_b)
    totals = segmented.ledger.totals()
    return PersistenceCertificate(
        segmented_vs_concatenated_error=float(
            np.linalg.norm(segmented.state - concatenated.state)
        ),
        direct_vs_step_replay_error=float(np.linalg.norm(segmented.state - direct)),
        reset_between_loops_difference=float(np.linalg.norm(segmented.state - reset.state)),
        initial_energy=normalized_energy(input_vector),
        final_energy=normalized_energy(segmented.state),
        ledger_closure_error=float(totals["closure_error"]),
    )


@dataclass(frozen=True)
class CausalFeedbackCertificate:
    coupling_fraction: float
    maximum_string_to_room_transfer_fro: float
    maximum_room_to_string_transfer_fro: float
    reciprocity_of_transfer_norms_error: float
    intervention_string_response: float
    decoupled_room_to_string_transfer_fro: float
    maximizing_time_seconds: float


def _frozen_unitary(
    momentum: Sequence[float], parameters: FamilyParameters, seconds: float
) -> ComplexMatrix:
    hamiltonian = physical_hamiltonian_rad_s(momentum, parameters)
    values, vectors = np.linalg.eigh(hamiltonian)
    return vectors @ np.diag(np.exp(-1.0j * values * seconds)) @ vectors.conj().T


def causal_feedback_certificate(
    momentum: Sequence[float],
    parameters: FamilyParameters = FamilyParameters(),
    *,
    maximum_time_seconds: float = 0.05,
    time_points: int = 501,
) -> CausalFeedbackCertificate:
    string = region_rows(Region.STRING)
    room = region_rows(Region.LATE_ROOM)
    best_sr = 0.0
    best_rs = 0.0
    best_rs_time = 0.0
    best_rs_u = np.eye(COMPLEX_DIMENSION, dtype=np.complex128)
    for seconds in np.linspace(0.0, maximum_time_seconds, time_points):
        unitary = _frozen_unitary(momentum, parameters, float(seconds))
        string_to_room = room @ unitary @ string.conj().T
        room_to_string = string @ unitary @ room.conj().T
        sr = float(np.linalg.norm(string_to_room, ord="fro"))
        rs = float(np.linalg.norm(room_to_string, ord="fro"))
        best_sr = max(best_sr, sr)
        if rs > best_rs:
            best_rs = rs
            best_rs_time = float(seconds)
            best_rs_u = unitary
    room_pulse = np.zeros(COMPLEX_DIMENSION, dtype=np.complex128)
    room_pulse[region_slice(Region.LATE_ROOM).start] = 1.0
    response = string @ best_rs_u @ room_pulse
    decoupled = FamilyParameters(
        mass=parameters.mass,
        carrier_hz=parameters.carrier_hz,
        dirac_scale_hz=parameters.dirac_scale_hz,
        trivial_offsets=parameters.trivial_offsets,
        coupling_fraction=0.0,
    )
    decoupled_u = _frozen_unitary(momentum, decoupled, best_rs_time)
    decoupled_transfer = string @ decoupled_u @ room.conj().T
    return CausalFeedbackCertificate(
        coupling_fraction=parameters.coupling_fraction,
        maximum_string_to_room_transfer_fro=best_sr,
        maximum_room_to_string_transfer_fro=best_rs,
        reciprocity_of_transfer_norms_error=abs(best_sr - best_rs),
        intervention_string_response=float(np.linalg.norm(response)),
        decoupled_room_to_string_transfer_fro=float(
            np.linalg.norm(decoupled_transfer, ord="fro")
        ),
        maximizing_time_seconds=best_rs_time,
    )


@dataclass(frozen=True)
class CouplingSweepPoint:
    coupling_fraction: float
    protected_gap: float
    participation: tuple[float, float, float, float]
    string_room_hamiltonian_block_fro: float


def coupling_sweep(
    momentum: Sequence[float],
    *,
    fractions: Sequence[float] = (0.0, 0.25, 0.5, 0.75, 1.0),
) -> tuple[CouplingSweepPoint, ...]:
    points: list[CouplingSweepPoint] = []
    for fraction in fractions:
        parameters = FamilyParameters(coupling_fraction=float(fraction))
        hamiltonian = relative_hamiltonian(momentum, parameters)
        eigenvalues = np.linalg.eigvalsh(hamiltonian)
        projector = protected_projector(momentum, parameters)
        block = hamiltonian[region_slice(Region.STRING), region_slice(Region.LATE_ROOM)]
        points.append(
            CouplingSweepPoint(
                coupling_fraction=float(fraction),
                protected_gap=float(eigenvalues[2] - eigenvalues[1]),
                participation=region_participation(projector),
                string_room_hamiltonian_block_fro=float(np.linalg.norm(block, ord="fro")),
            )
        )
    return tuple(points)


@dataclass(frozen=True)
class ReadoutCertificate:
    sample_rate_hz: int
    samples: int
    duration_seconds: float
    initial_energy: float
    final_energy: float
    total_radiated_energy: float
    ledger_closure_error: float
    radiation_port_law_residual_fro: float
    raw_peak: float
    encoded_peak: float
    encoding_gain: float
    rms_left: float
    rms_right: float
    spectral_centroid_left_hz: float
    spectral_centroid_right_hz: float


def _spectral_centroid(signal: NDArray[np.float64], sample_rate_hz: int) -> float:
    spectrum = np.abs(np.fft.rfft(signal))
    frequencies = np.fft.rfftfreq(signal.shape[0], 1.0 / sample_rate_hz)
    denominator = float(np.sum(spectrum))
    if denominator <= 0.0:
        return 0.0
    return float(np.sum(frequencies * spectrum) / denominator)


def _simulate_passive_readout(
    initial_state: ComplexVector,
    output_directions: ComplexMatrix,
    parameters: FamilyParameters,
    *,
    momentum: Sequence[float],
    duration_seconds: float,
    sample_rate_hz: int,
    amplitude_decay_seconds: float,
) -> tuple[NDArray[np.float64], dict[str, float]]:
    state = np.asarray(initial_state, dtype=np.complex128).copy()
    samples = int(round(duration_seconds * sample_rate_hz))
    hamiltonian = physical_hamiltonian_rad_s(momentum, parameters)
    values, vectors = np.linalg.eigh(hamiltonian)
    unitary_step = (
        vectors
        @ np.diag(np.exp(-1.0j * values / sample_rate_hz))
        @ vectors.conj().T
    )
    energy_coupling = 1.0 - math.exp(-2.0 / (amplitude_decay_seconds * sample_rate_hz))
    port = PassiveRadiationPort(output_directions, energy_coupling)
    output = np.zeros((samples, output_directions.shape[0]), dtype=np.float64)
    initial_energy = normalized_energy(state)
    radiated = 0.0
    numerical = 0.0
    for sample in range(samples):
        before = normalized_energy(state)
        state = unitary_step @ state
        numerical += normalized_energy(state) - before
        state, outgoing, frame = port.scatter(state, label="readout")
        output[sample] = outgoing.real
        radiated += frame.radiation
        numerical += frame.numerical_residual
    final_energy = normalized_energy(state)
    metrics = {
        "samples": float(samples),
        "initial_energy": initial_energy,
        "final_energy": final_energy,
        "total_radiated_energy": float(radiated),
        "ledger_closure_error": float(final_energy - (initial_energy - radiated) - numerical),
        "radiation_port_law_residual_fro": port.law_residual(),
        "numerical_residual": float(numerical),
    }
    return output, metrics


def _write_encoded_wave(
    output: NDArray[np.float64],
    output_path: Path,
    *,
    sample_rate_hz: int,
    encoding_gain: float,
) -> None:
    encoded = np.clip(output * encoding_gain, -1.0, 1.0)
    pcm = np.asarray(np.round(encoded * 32767.0), dtype="<i2")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(output_path), "wb") as handle:
        handle.setnchannels(output.shape[1])
        handle.setsampwidth(2)
        handle.setframerate(sample_rate_hz)
        handle.writeframes(pcm.tobytes())


def _readout_certificate(
    output: NDArray[np.float64],
    metrics: dict[str, float],
    *,
    sample_rate_hz: int,
    encoding_gain: float,
) -> ReadoutCertificate:
    encoded = np.clip(output * encoding_gain, -1.0, 1.0)
    return ReadoutCertificate(
        sample_rate_hz=sample_rate_hz,
        samples=int(metrics["samples"]),
        duration_seconds=int(metrics["samples"]) / sample_rate_hz,
        initial_energy=metrics["initial_energy"],
        final_energy=metrics["final_energy"],
        total_radiated_energy=metrics["total_radiated_energy"],
        ledger_closure_error=metrics["ledger_closure_error"],
        radiation_port_law_residual_fro=metrics["radiation_port_law_residual_fro"],
        raw_peak=float(np.max(np.abs(output))),
        encoded_peak=float(np.max(np.abs(encoded))),
        encoding_gain=float(encoding_gain),
        rms_left=float(np.sqrt(np.mean(encoded[:, 0] ** 2))),
        rms_right=float(np.sqrt(np.mean(encoded[:, 1] ** 2))),
        spectral_centroid_left_hz=_spectral_centroid(encoded[:, 0], sample_rate_hz),
        spectral_centroid_right_hz=_spectral_centroid(encoded[:, 1], sample_rate_hz),
    )


def render_passive_readout(
    initial_state: ComplexVector,
    output_directions: ComplexMatrix,
    parameters: FamilyParameters,
    output_path: Path,
    *,
    momentum: Sequence[float],
    duration_seconds: float = 1.25,
    sample_rate_hz: int = 48_000,
    amplitude_decay_seconds: float = 0.45,
    encoded_peak: float = 0.92,
    encoding_gain: float | None = None,
) -> ReadoutCertificate:
    """Radiate the persistent state itself; no oscillator-bank sidecar exists."""
    output, metrics = _simulate_passive_readout(
        initial_state,
        output_directions,
        parameters,
        momentum=momentum,
        duration_seconds=duration_seconds,
        sample_rate_hz=sample_rate_hz,
        amplitude_decay_seconds=amplitude_decay_seconds,
    )
    raw_peak = float(np.max(np.abs(output)))
    gain = (
        float(encoding_gain)
        if encoding_gain is not None
        else (encoded_peak / raw_peak if raw_peak > 0.0 else 1.0)
    )
    _write_encoded_wave(output, output_path, sample_rate_hz=sample_rate_hz, encoding_gain=gain)
    return _readout_certificate(
        output, metrics, sample_rate_hz=sample_rate_hz, encoding_gain=gain
    )


def render_order_pair(
    loop_a: KatoPath,
    loop_b: KatoPath,
    design: ObservableDesign,
    output_directory: Path,
    parameters: FamilyParameters = FamilyParameters(),
) -> dict[str, object]:
    amplitude = 0.65

    def execute(paths: Sequence[KatoPath], label: str) -> PersistentInstrument:
        instrument = PersistentInstrument(parameters=parameters)
        instrument.inject(amplitude * design.source_vector, label=f"{label}:source")
        instrument.apply_unitary(
            design.preparation_unitary, label=f"{label}:prepare-protected"
        )
        for path in paths:
            instrument.execute_kato_path(path)
        instrument.apply_unitary(
            design.readout_unitary, label=f"{label}:room-readout-calibration"
        )
        return instrument

    ab_instrument = execute((loop_a, loop_b), "AB")
    ba_instrument = execute((loop_b, loop_a), "BA")
    ab = ab_instrument.state.copy()
    ba = ba_instrument.state.copy()
    options = {
        "momentum": loop_a.points[0],
        "duration_seconds": 1.25,
        "sample_rate_hz": 48_000,
        "amplitude_decay_seconds": 0.45,
    }
    ab_output, ab_metrics = _simulate_passive_readout(
        ab, design.output_directions, parameters, **options
    )
    ba_output, ba_metrics = _simulate_passive_readout(
        ba, design.output_directions, parameters, **options
    )
    shared_peak = max(
        float(np.max(np.abs(ab_output))), float(np.max(np.abs(ba_output)))
    )
    shared_gain = 0.92 / shared_peak if shared_peak > 0.0 else 1.0
    ab_path = output_directory / "unified-protected-ab.wav"
    ba_path = output_directory / "unified-protected-ba.wav"
    _write_encoded_wave(
        ab_output, ab_path, sample_rate_hz=48_000, encoding_gain=shared_gain
    )
    _write_encoded_wave(
        ba_output, ba_path, sample_rate_hz=48_000, encoding_gain=shared_gain
    )
    ab_receipt = _readout_certificate(
        ab_output, ab_metrics, sample_rate_hz=48_000, encoding_gain=shared_gain
    )
    ba_receipt = _readout_certificate(
        ba_output, ba_metrics, sample_rate_hz=48_000, encoding_gain=shared_gain
    )
    encoded_ab = np.clip(ab_output * shared_gain, -1.0, 1.0)
    encoded_ba = np.clip(ba_output * shared_gain, -1.0, 1.0)
    relative_rms_distance = float(
        np.sqrt(np.mean((encoded_ab - encoded_ba) ** 2))
        / max(
            np.sqrt(np.mean(encoded_ab**2)),
            np.sqrt(np.mean(encoded_ba**2)),
            np.finfo(float).tiny,
        )
    )

    raw_rms_ab = float(np.sqrt(np.mean(ab_output**2)))
    raw_rms_ba = float(np.sqrt(np.mean(ba_output**2)))
    normalized_ab = ab_output / max(raw_rms_ab, np.finfo(float).tiny)
    normalized_ba = ba_output / max(raw_rms_ba, np.finfo(float).tiny)
    gain_removed_distance = float(
        np.sqrt(np.mean((normalized_ab - normalized_ba) ** 2))
    )
    flat_ab = ab_output.reshape(-1)
    flat_ba = ba_output.reshape(-1)
    optimal_scalar = float(
        np.dot(flat_ab, flat_ba)
        / max(float(np.dot(flat_ba, flat_ba)), np.finfo(float).tiny)
    )
    optimal_scalar_residual = float(
        np.linalg.norm(flat_ab - optimal_scalar * flat_ba)
        / max(np.linalg.norm(flat_ab), np.finfo(float).tiny)
    )
    magnitude_ab = np.abs(np.fft.rfft(ab_output, axis=0)).reshape(-1)
    magnitude_ba = np.abs(np.fft.rfft(ba_output, axis=0)).reshape(-1)
    magnitude_ab /= max(np.linalg.norm(magnitude_ab), np.finfo(float).tiny)
    magnitude_ba /= max(np.linalg.norm(magnitude_ba), np.finfo(float).tiny)
    normalized_spectral_magnitude_distance = float(
        np.linalg.norm(magnitude_ab - magnitude_ba)
    )

    # Constant per-file gains are a presentation-only level-matched control.
    # They are not part of the instrument or the energy ledger.
    crest_ab = float(np.max(np.abs(ab_output))) / max(raw_rms_ab, np.finfo(float).tiny)
    crest_ba = float(np.max(np.abs(ba_output))) / max(raw_rms_ba, np.finfo(float).tiny)
    matched_target_rms = min(0.18, 0.90 / max(crest_ab, crest_ba, 1.0))
    matched_gain_ab = matched_target_rms / max(raw_rms_ab, np.finfo(float).tiny)
    matched_gain_ba = matched_target_rms / max(raw_rms_ba, np.finfo(float).tiny)
    ab_matched_path = output_directory / "unified-protected-ab-level-matched.wav"
    ba_matched_path = output_directory / "unified-protected-ba-level-matched.wav"
    _write_encoded_wave(
        ab_output,
        ab_matched_path,
        sample_rate_hz=48_000,
        encoding_gain=matched_gain_ab,
    )
    _write_encoded_wave(
        ba_output,
        ba_matched_path,
        sample_rate_hz=48_000,
        encoding_gain=matched_gain_ba,
    )

    def whole_ledger(
        instrument: PersistentInstrument, metrics: dict[str, float]
    ) -> dict[str, float]:
        controls = instrument.ledger.totals()
        numerical = controls["numerical_residual"] + metrics["numerical_residual"]
        expected = (
            controls["initial_energy"]
            + controls["source_work"]
            + controls["control_work"]
            + controls["metric_work"]
            - controls["internal_loss"]
            - metrics["total_radiated_energy"]
            - controls["retirement_radiation"]
            - controls["retirement_dissipation"]
            + numerical
        )
        return {
            "initial_energy": controls["initial_energy"],
            "final_energy": metrics["final_energy"],
            "source_work": controls["source_work"],
            "control_work": controls["control_work"],
            "metric_work": controls["metric_work"],
            "internal_loss": controls["internal_loss"],
            "radiation": metrics["total_radiated_energy"],
            "retirement_radiation": controls["retirement_radiation"],
            "retirement_dissipation": controls["retirement_dissipation"],
            "numerical_residual": numerical,
            "actuator_overhead_external": controls["actuator_overhead_external"],
            "closure_error": metrics["final_energy"] - expected,
        }

    return {
        "ab_path": ab_path.name,
        "ba_path": ba_path.name,
        "ab_level_matched_path": ab_matched_path.name,
        "ba_level_matched_path": ba_matched_path.name,
        "ab": asdict(ab_receipt),
        "ba": asdict(ba_receipt),
        "ab_whole_instrument_ledger": whole_ledger(ab_instrument, ab_metrics),
        "ba_whole_instrument_ledger": whole_ledger(ba_instrument, ba_metrics),
        "shared_encoding_gain": float(shared_gain),
        "relative_rms_waveform_distance": relative_rms_distance,
        "gain_removed_waveform_distance": gain_removed_distance,
        "optimal_scalar_fit": optimal_scalar,
        "optimal_scalar_residual_fraction": optimal_scalar_residual,
        "normalized_spectral_magnitude_distance": normalized_spectral_magnitude_distance,
        "level_matched_target_rms": float(matched_target_rms),
        "level_matched_gain_ab": float(matched_gain_ab),
        "level_matched_gain_ba": float(matched_gain_ba),
        "presentation_gains_excluded_from_ledger": True,
        "spectral_matching_claimed": False,
        "final_state_distance": float(np.linalg.norm(ab - ba)),
        "initial_energy": amplitude * amplitude,
        "ab_final_energy_before_readout": normalized_energy(ab),
        "ba_final_energy_before_readout": normalized_energy(ba),
    }

