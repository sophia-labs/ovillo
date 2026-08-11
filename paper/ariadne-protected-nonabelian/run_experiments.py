#!/usr/bin/env python3
"""Generate the protected non-Abelian Ariadne reference packet."""

from __future__ import annotations

from dataclasses import asdict
from pathlib import Path
import argparse
import hashlib
import json
import math
import sys
import wave

import numpy as np

HERE = Path(__file__).resolve().parent
GESTURE_REFERENCE = HERE.parent / "ariadne-gesture-compiler" / "reference"
if str(GESTURE_REFERENCE) not in sys.path:
    sys.path.insert(0, str(GESTURE_REFERENCE))

from ariadne_gesture import (  # noqa: E402
    dirac4d_hamiltonian,
    occupied_projector,
    kato_direct_rotation,
    verify_protected_nonabelian_instrument,
)


def _complex_matrix_json(matrix: np.ndarray) -> list[list[list[float]]]:
    return [
        [[float(value.real), float(value.imag)] for value in row]
        for row in np.asarray(matrix)
    ]


def _fixture(experiment) -> dict:
    def lowering(value) -> dict:
        return {
            "target": np.asarray(value.matrix).tolist(),
            "schedule": [
                {"left": gate.i, "right": gate.j, "angle": gate.theta}
                for gate in value.compile.schedule
            ],
            "certificate": asdict(value.certificate),
        }

    return {
        "schema": "ariadne-protected-nonabelian-fixture/1",
        "base_point": list(experiment.base_point),
        "base_parameters": asdict(experiment.base_parameters),
        "occupied_holonomy_a": _complex_matrix_json(
            experiment.loop_a.occupied_holonomy
        ),
        "occupied_holonomy_b": _complex_matrix_json(
            experiment.loop_b.occupied_holonomy
        ),
        "full_holonomy_a": _complex_matrix_json(experiment.loop_a.full_holonomy),
        "full_holonomy_b": _complex_matrix_json(experiment.loop_b.full_holonomy),
        "acoustic_lowering_a": lowering(experiment.lowering_a),
        "acoustic_lowering_b": lowering(experiment.lowering_b),
        "z3": {
            "c3": experiment.z3.c3.tolist(),
            "c3_prime": experiment.z3.c3_prime.tolist(),
            "certificate": asdict(experiment.z3.certificate),
        },
    }


def _equal_power_pan(position: float) -> tuple[float, float]:
    angle = (position + 1.0) * math.pi / 4.0
    return math.cos(angle), math.sin(angle)


def _initial_complex_state(experiment) -> np.ndarray:
    _, frame = np.linalg.eigh(
        dirac4d_hamiltonian(experiment.base_point, experiment.base_parameters)
    )
    state = frame[:, 0] + (0.37 + 0.19j) * frame[:, 1]
    return state / np.linalg.norm(state)


def _transport_blocks(experiment, labels: str, block_samples: int):
    state = _initial_complex_state(experiment)
    states: list[tuple[np.ndarray, np.ndarray]] = []
    path_by_label = {
        "A": experiment.loop_a_path,
        "B": experiment.loop_b_path,
    }
    parameters = experiment.base_parameters
    for label in labels:
        path = path_by_label[label]
        old_projector = occupied_projector(path[0], parameters)
        for point in path[1:]:
            new_projector = occupied_projector(point, parameters)
            step, _ = kato_direct_rotation(old_projector, new_projector)
            next_state = step @ state
            states.append((state, next_state))
            state = next_state
            old_projector = new_projector
    return states, state


def _render_yang_sequence(
    experiment,
    labels: str,
    *,
    sample_rate: int,
    block_samples: int,
) -> tuple[np.ndarray, np.ndarray]:
    blocks, endpoint = _transport_blocks(experiment, labels, block_samples)
    frequencies = np.asarray([110.0, 164.8138, 246.9417, 369.9944])
    pans = np.asarray([-0.82, -0.28, 0.31, 0.84])
    pan = np.asarray([_equal_power_pan(float(value)) for value in pans])
    total_samples = len(blocks) * block_samples
    audio = np.zeros((total_samples, 2), dtype=np.float64)
    sample_cursor = 0
    for start, stop in blocks:
        for offset in range(block_samples):
            amount = (offset + 1.0) / block_samples
            state = (1.0 - amount) * start + amount * stop
            state /= max(float(np.linalg.norm(state)), 1.0e-30)
            time_s = sample_cursor / sample_rate
            carrier = np.exp(2.0j * math.pi * frequencies * time_s)
            modal = np.real(state * carrier)
            # A small, fixed overtone exposes mode identity without changing
            # the state transport.
            modal += 0.16 * np.real(
                np.conj(state) * np.exp(4.0j * math.pi * frequencies * time_s)
            )
            audio[sample_cursor] = modal @ pan / math.sqrt(len(frequencies))
            sample_cursor += 1
    if total_samples:
        fade = min(sample_rate // 80, total_samples // 4)
        envelope = np.ones(total_samples)
        envelope[:fade] = np.linspace(0.0, 1.0, fade, endpoint=False)
        envelope[-fade:] = np.linspace(1.0, 0.0, fade)
        audio *= envelope[:, None]
    return audio, endpoint


def _z3_states(experiment, labels: str) -> tuple[list[np.ndarray], np.ndarray]:
    cells = experiment.z3.certificate.cells
    dimension = 3 * cells
    center = cells // 2
    state = np.zeros(dimension)
    state[center * 3] = 1.0
    out = [state.copy()]
    operators = {
        "C": experiment.z3.c3,
        "D": experiment.z3.c3_prime,
    }
    for label in labels:
        state = operators[label] @ state
        out.append(state.copy())
    return out, state


def _render_z3_sequence(
    experiment,
    labels: str,
    *,
    sample_rate: int,
    seconds_per_state: float = 0.16,
) -> tuple[np.ndarray, np.ndarray]:
    states, endpoint = _z3_states(experiment, labels)
    block = int(round(seconds_per_state * sample_rate))
    cells = experiment.z3.certificate.cells
    center = cells // 2
    audio = np.zeros((block * len(states), 2), dtype=np.float64)
    internal_frequencies = np.asarray([220.0, 277.1826, 329.6276])
    for state_index, state in enumerate(states):
        basis = int(np.argmax(np.abs(state)))
        cell = basis // 3
        internal = basis % 3
        shift = cell - center
        frequency = internal_frequencies[internal] * 2.0 ** (2.0 * shift / 12.0)
        pan = max(-0.92, min(0.92, shift / max(1.0, center)))
        left, right = _equal_power_pan(pan)
        start = state_index * block
        time_s = np.arange(block) / sample_rate
        tone = np.sin(2.0 * math.pi * frequency * time_s)
        tone += (0.18 + 0.08 * internal) * np.sin(
            4.0 * math.pi * frequency * time_s + 0.3 * internal
        )
        envelope = np.sin(math.pi * np.arange(block) / max(1, block - 1)) ** 0.7
        audio[start : start + block, 0] = left * tone * envelope
        audio[start : start + block, 1] = right * tone * envelope
    return audio, endpoint


def _common_normalize(*signals: np.ndarray) -> tuple[np.ndarray, ...]:
    peak = max(float(np.max(np.abs(signal))) for signal in signals)
    scale = 0.88 / max(peak, 1.0e-30)
    return tuple(signal * scale for signal in signals)


def _write_wav(path: Path, audio: np.ndarray, sample_rate: int) -> None:
    clipped = np.clip(audio, -1.0, 1.0)
    pcm = np.round(clipped * 32767.0).astype("<i2")
    with wave.open(str(path), "wb") as target:
        target.setnchannels(2)
        target.setsampwidth(2)
        target.setframerate(sample_rate)
        target.writeframes(pcm.tobytes())


def _rms(audio: np.ndarray) -> float:
    return float(np.sqrt(np.mean(np.square(audio))))


def _centroid(audio: np.ndarray, sample_rate: int) -> float:
    mono = np.mean(audio, axis=1)
    spectrum = np.abs(np.fft.rfft(mono))
    frequencies = np.fft.rfftfreq(len(mono), 1.0 / sample_rate)
    return float(np.sum(frequencies * spectrum) / max(np.sum(spectrum), 1.0e-30))


def _audio_metrics(
    first: np.ndarray,
    second: np.ndarray,
    endpoint_distance: float,
    sample_rate: int,
) -> dict:
    first_rms = _rms(first)
    second_rms = _rms(second)
    return {
        "relative_rms_distance": _rms(first - second)
        / max(0.5 * (first_rms + second_rms), 1.0e-30),
        "rms_level_difference_db": 20.0
        * math.log10(max(first_rms, 1.0e-30) / max(second_rms, 1.0e-30)),
        "spectral_centroid_a_hz": _centroid(first, sample_rate),
        "spectral_centroid_b_hz": _centroid(second, sample_rate),
        "endpoint_state_distance": endpoint_distance,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--quick", action="store_true")
    parser.add_argument("--output", type=Path, default=HERE / "artifacts")
    parser.add_argument("--no-audio", action="store_true")
    args = parser.parse_args()

    args.output.mkdir(parents=True, exist_ok=True)
    experiment = verify_protected_nonabelian_instrument(
        grid=10 if args.quick else 12,
        perturbation_grid=8,
        perturbations=4 if args.quick else 12,
        path_steps=24 if args.quick else 48,
        adiabatic_durations=(16.0, 32.0, 64.0) if args.quick else (16.0, 32.0, 64.0, 128.0),
    )

    sample_rate = 48_000
    yang_ab, endpoint_ab = _render_yang_sequence(
        experiment,
        "AB",
        sample_rate=sample_rate,
        block_samples=64 if args.quick else 96,
    )
    yang_ba, endpoint_ba = _render_yang_sequence(
        experiment,
        "BA",
        sample_rate=sample_rate,
        block_samples=64 if args.quick else 96,
    )
    z3_path_one, z3_endpoint_one = _render_z3_sequence(
        experiment, "CCDC", sample_rate=sample_rate
    )
    z3_path_two, z3_endpoint_two = _render_z3_sequence(
        experiment, "DCCC", sample_rate=sample_rate
    )
    yang_ab, yang_ba = _common_normalize(yang_ab, yang_ba)
    z3_path_one, z3_path_two = _common_normalize(z3_path_one, z3_path_two)

    endpoint_distance = float(np.linalg.norm(endpoint_ab - endpoint_ba))
    results = {
        "schema": "ariadne-protected-nonabelian/1",
        "mode": "quick" if args.quick else "full",
        "certificate": asdict(experiment.certificate),
        "audio": {
            "yang_braid": _audio_metrics(
                yang_ab,
                yang_ba,
                endpoint_distance,
                sample_rate,
            ),
            "z3": {
                "endpoint_distance": float(
                    np.linalg.norm(z3_endpoint_one - z3_endpoint_two)
                ),
                "path_one_cell_shift": (
                    experiment.z3.certificate.path_one_cell_shift
                ),
                "path_two_cell_shift": (
                    experiment.z3.certificate.path_two_cell_shift
                ),
            },
        },
        "interpretation": [
            "The second Chern number protects the rank-two bundle phase while the sampled gap remains open.",
            "The Wilson-loop matrices are non-Abelian but remain geometric, path-dependent gates rather than individually quantized gates.",
            "The exact Z3 pump algebra supplies a discrete, literature-grounded acoustic routing target whose order changes transport.",
            "The realified full-space holonomies compile to authored graph-edge Givens schedules and preserve the declared Euclidean state energy.",
            "The WAVs are deterministic research listening stimuli, not a listener-study result or a measured physical metamaterial.",
        ],
    }
    (args.output / "results.json").write_text(
        json.dumps(results, indent=2, sort_keys=True) + "\n"
    )
    (args.output / "fixture.json").write_text(
        json.dumps(_fixture(experiment), indent=2, sort_keys=True) + "\n"
    )

    certificate = experiment.certificate
    summary = f"""# Ariadne protected non-Abelian reference result

- Analytic second Chern number: `{certificate.analytic_second_chern:.0f}`.
- Numerical degree estimate: `{certificate.numerical_second_chern.estimate:.12f}`.
- Sampled base gap: `{certificate.numerical_second_chern.sampled_gap:.9f}`.
- Trivial-phase estimate: `{certificate.trivial_second_chern.estimate:.12f}`.
- Wilson-loop commutator norm: `{certificate.loop_commutator_norm_fro:.12f}`.
- Endpoint state order distance: `{certificate.loop_order_state_distance:.12f}`.
- Minimum perturbation-family homotopy gap: `{certificate.perturbations.minimum_sampled_homotopy_gap:.9f}`.
- Minimum perturbed Wilson commutator norm: `{certificate.perturbations.minimum_nonabelian_commutator_norm_fro:.9f}`.
- Longest-duration adiabatic projective error: `{certificate.adiabatic.maximum_duration_projective_error_fro:.3e}`.
- Longest-duration occupied-subspace leakage: `{certificate.adiabatic.maximum_duration_leakage_fro:.3e}`.
- Acoustic lowering: `{certificate.lowering_a.gate_count}` gates per loop on an eight-coordinate path graph.
- Worst lowering reconstruction error: `{max(certificate.lowering_a.reconstruction_error_fro, certificate.lowering_b.reconstruction_error_fro):.3e}`.
- Z3 path shifts: `{certificate.z3_pump.path_one_cell_shift}` versus `{certificate.z3_pump.path_two_cell_shift}` cells.
- Yang AB/BA relative RMS distance: `{results['audio']['yang_braid']['relative_rms_distance']:.6f}`.

The protected object is the gapped rank-two bundle and its nonzero second Chern class. The non-Abelian Wilson gates live inside that protected family but are geometric and path-dependent. The exact Z3 pump fixture is a separate discrete acoustic routing target grounded in published acoustic Thouless-pump work.
"""
    (args.output / "SUMMARY.md").write_text(summary)

    if not args.no_audio:
        _write_wav(args.output / "yang-loop-ab.wav", yang_ab, sample_rate)
        _write_wav(args.output / "yang-loop-ba.wav", yang_ba, sample_rate)
        _write_wav(args.output / "z3-path-one.wav", z3_path_one, sample_rate)
        _write_wav(args.output / "z3-path-two.wav", z3_path_two, sample_rate)

    manifest = {}
    for artifact in sorted(args.output.iterdir()):
        if artifact.is_file() and artifact.name != "MANIFEST.sha256":
            manifest[artifact.name] = hashlib.sha256(artifact.read_bytes()).hexdigest()
    (args.output / "MANIFEST.sha256").write_text(
        "".join(f"{digest}  {name}\n" for name, digest in manifest.items())
    )

    # Fail-loud scientific gates.
    assert certificate.analytic_second_chern == 1.0
    assert certificate.numerical_second_chern.nearest_integer == 1
    assert certificate.numerical_second_chern.integer_error < (0.03 if args.quick else 0.01)
    assert certificate.trivial_second_chern.nearest_integer == 0
    assert certificate.critical_gap_at_origin <= 1.0e-14
    assert certificate.loop_commutator_norm_fro > 0.8
    assert certificate.loop_order_state_distance > 0.5
    assert certificate.perturbations.minimum_sampled_homotopy_gap > 1.4
    assert certificate.perturbations.maximum_second_chern_error < 0.10
    assert certificate.perturbations.minimum_nonabelian_commutator_norm_fro > 1.0
    assert certificate.adiabatic.maximum_duration_projective_error_fro < (0.003 if args.quick else 0.001)
    assert certificate.adiabatic.maximum_duration_leakage_fro < (0.05 if args.quick else 0.025)
    assert certificate.lowering_a.gate_count == 28
    assert certificate.lowering_b.gate_count == 28
    assert certificate.lowering_a.reconstruction_error_fro < 1.0e-11
    assert certificate.lowering_b.reconstruction_error_fro < 1.0e-11
    assert certificate.z3_pump.c3_order_three_error_fro == 0.0
    assert certificate.z3_pump.c3_prime_order_three_error_fro == 0.0
    assert certificate.z3_pump.path_one_cell_shift != certificate.z3_pump.path_two_cell_shift
    assert results["audio"]["yang_braid"]["relative_rms_distance"] > 0.1

    print(json.dumps(results, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
