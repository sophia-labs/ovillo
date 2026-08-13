#!/usr/bin/env python3
"""Reference campaign for one Ariadne body played as string or brass.

This executable is a controlled research oracle, not the production renderer.
It uses one fixed 24-mode Hamiltonian state and one compliant aperture for all
three renders: plucked configuration, sustained brass configuration, and a
state-continuous morph. No target audio or residual renderer enters playback.
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import time
import wave

import numpy as np
from numba import njit


@njit
def _rot(left: float, right: float, angle: float) -> tuple[float, float]:
    cosine = math.cos(angle)
    sine = math.sin(angle)
    return cosine * left - sine * right, sine * left + cosine * right


@njit
def _normalize_inplace(values: np.ndarray) -> None:
    norm = 0.0
    for value in values:
        norm += value * value
    norm = math.sqrt(norm)
    if norm > 1.0e-15:
        for index in range(values.size):
            values[index] /= norm


@njit
def render_shared_body(sr, duration, regime):
    modes = 24
    frames = int(sr * duration)
    displacement = np.zeros(modes)
    momentum = np.zeros(modes)
    source_row = np.zeros(modes)
    radiation_row = np.zeros(modes)
    omega = np.zeros(modes)
    cosine = np.ones(modes)
    sine = np.zeros(modes)
    retain = np.ones(modes)
    previous_omega = np.zeros(modes)
    aperture_displacement = 0.0
    aperture_momentum = 1.0e-8
    output = np.zeros(frames)
    source_work = 0.0
    configuration_work = 0.0
    declared_loss = 0.0
    maximum_state = 0.0
    pluck_times = np.array([0.05, 0.48, 0.91, 1.34, 1.77, 2.20])
    pluck_semitones = np.array([0.0, 3.0, 5.0, 7.0, 5.0, 3.0])

    for frame in range(frames):
        time_seconds = frame / sr
        if regime == 0:
            morph = 0.0
        elif regime == 1:
            morph = 1.0
        else:
            position = (time_seconds - 1.15) / 0.75
            morph = (
                0.0
                if position <= 0.0
                else (
                    1.0
                    if position >= 1.0
                    else position * position * (3.0 - 2.0 * position)
                )
            )

        semitone = 0.0
        pulse = 0.0
        for event in range(pluck_times.size):
            if time_seconds >= pluck_times[event]:
                semitone = pluck_semitones[event]
            event_time = time_seconds - pluck_times[event]
            if 0.0 <= event_time < 0.003:
                phase = event_time / 0.003
                pulse += 0.85 * 0.5 * (1.0 - math.cos(2.0 * math.pi * phase))

        guitar_length = 2.0 ** (-semitone / 12.0)
        length_ratio = (1.0 - morph) * guitar_length + morph * 1.015
        dispersion = 1.8e-5 + (1.9e-5 - 1.8e-5) * morph
        boundary_phase = -0.012 * morph
        aperture_frequency = 540.0 + (110.0 - 540.0) * morph
        quartic_stiffness = 4.0e6 + (4.1e6 - 4.0e6) * morph
        opening_bias = 0.025 + (0.055 - 0.025) * morph
        aperture_damping = 48.0 + (7.9 - 48.0) * morph
        source_to_aperture_rate = 1_100.0 + (12_000.0 - 1_100.0) * morph
        source_to_field_rate = 1_800.0 + (2_400.0 - 1_800.0) * morph
        sweep_rate = 420.0 + (3_500.0 - 420.0) * morph
        active_rate = 38.0 * morph
        active_target = 0.25 + (0.334 - 0.25) * morph
        low_t60 = 2.2 + (3.1 - 2.2) * morph
        high_t60 = 0.28 + (0.66 - 0.28) * morph

        breath_start = 0.02 if regime == 1 else 1.55
        breath = 0.0
        if morph > 0.0 and time_seconds > breath_start:
            breath = (
                0.264
                * morph
                * min(1.0, (time_seconds - breath_start) / 0.12)
                * min(1.0, (duration - time_seconds) / 0.25)
            )
        source_wave = (1.0 - morph) * pulse + breath

        for index in range(modes):
            mode = index + 1.0
            guitar_source = math.sin(math.pi * mode * 0.18) / mode
            guitar_radiation = math.sin(math.pi * mode * 0.18) / (
                1.0 + 0.003 * index * index
            )
            brass_source = (1.0 / (1.0 + 0.08 * index * index)) * (
                1.0 if index % 2 == 0 else 0.85
            )
            brass_radiation = (1.0 + 0.97 * index / (modes - 1)) / (
                1.0 + 0.02 * index * index
            )
            source_row[index] = (
                (1.0 - morph) * guitar_source + morph * brass_source
            )
            radiation_row[index] = (
                (1.0 - morph) * guitar_radiation + morph * brass_radiation
            )
        _normalize_inplace(source_row)
        _normalize_inplace(radiation_row)

        if frame % 8 == 0:
            for index in range(modes):
                mode = index + 1.0 + boundary_phase
                next_omega = (
                    2.0
                    * math.pi
                    * 116.5
                    * mode
                    * math.sqrt(max(0.05, 1.0 + dispersion * mode * mode))
                    / length_ratio
                )
                if previous_omega[index] != 0.0:
                    configuration_work += (
                        0.5
                        / sr
                        * (next_omega * next_omega - previous_omega[index] ** 2)
                        * displacement[index]
                        * displacement[index]
                    )
                previous_omega[index] = next_omega
                omega[index] = next_omega
                cosine[index] = math.cos(next_omega / sr)
                sine[index] = math.sin(next_omega / sr)
                modal_position = index / (modes - 1)
                t60 = low_t60 + modal_position * (high_t60 - low_t60)
                retain[index] = math.exp(-math.log(1000.0) / (t60 * sr))

        opening = max(0.0, opening_bias + aperture_displacement)
        opening_fraction = opening / (1.0 + opening)
        field_flow = 0.0
        field_effort = 0.0
        for index in range(modes):
            field_flow += source_row[index] * momentum[index]
            field_effort += source_row[index] * omega[index] * displacement[index]

        incoming = source_wave
        source_wave, aperture_momentum = _rot(
            source_wave,
            aperture_momentum,
            2.0 * math.pi * source_to_aperture_rate / sr * opening_fraction,
        )
        source_wave, next_flow = _rot(
            source_wave,
            field_flow,
            2.0 * math.pi * source_to_field_rate / sr * opening_fraction,
        )
        flow_increment = next_flow - field_flow
        for index in range(modes):
            momentum[index] += flow_increment * source_row[index]
        field_flow = next_flow

        geometry = math.tanh(2.5 * aperture_displacement + 0.2 * field_effort)
        aperture_momentum, next_flow = _rot(
            aperture_momentum,
            field_flow,
            2.0
            * math.pi
            * sweep_rate
            / sr
            * (0.2 + 0.8 * opening_fraction)
            * geometry,
        )
        flow_increment = next_flow - field_flow
        for index in range(modes):
            momentum[index] += flow_increment * source_row[index]
        field_flow = next_flow
        source_work += 0.5 / sr * (incoming * incoming - source_wave * source_wave)

        if active_rate > 0.0 and incoming != 0.0 and abs(aperture_momentum) > 1.0e-15:
            synchronization = 1.0 + 0.4 * math.tanh(
                -field_flow
                * aperture_momentum
                / (0.01 + abs(aperture_momentum))
            )
            work = (
                active_rate
                / sr
                * abs(incoming)
                * opening_fraction
                * synchronization
                * aperture_momentum
                * aperture_momentum
                * (
                    1.0
                    - aperture_momentum
                    * aperture_momentum
                    / (active_target * active_target)
                )
            )
            if work > 0.0:
                aperture_momentum = math.copysign(
                    math.sqrt(aperture_momentum * aperture_momentum + 2.0 * work * sr),
                    aperture_momentum,
                )
                source_work += work
            else:
                available = 0.5 * aperture_momentum * aperture_momentum / sr
                loss = min(-work, available)
                aperture_momentum = math.copysign(
                    math.sqrt(
                        max(
                            0.0,
                            aperture_momentum * aperture_momentum - 2.0 * loss * sr,
                        )
                    ),
                    aperture_momentum,
                )
                declared_loss += loss

        stiffness = (2.0 * math.pi * aperture_frequency) ** 2
        force = (
            stiffness * aperture_displacement
            + quartic_stiffness
            * aperture_displacement
            * (aperture_displacement + opening_bias)
            * (aperture_displacement + 2.0 * opening_bias)
        )
        aperture_momentum -= 0.5 * force / sr
        aperture_displacement += aperture_momentum / sr
        force = (
            stiffness * aperture_displacement
            + quartic_stiffness
            * aperture_displacement
            * (aperture_displacement + opening_bias)
            * (aperture_displacement + 2.0 * opening_bias)
        )
        aperture_momentum -= 0.5 * force / sr
        before_damping = 0.5 * aperture_momentum * aperture_momentum / sr
        aperture_momentum *= math.exp(-aperture_damping / sr)
        declared_loss += max(
            0.0,
            before_damping - 0.5 * aperture_momentum * aperture_momentum / sr,
        )

        for left, right in ((0, 2), (1, 3), (2, 5), (3, 7)):
            resonance_geometry = math.tanh(
                1.6 * displacement[left] * displacement[right]
                + 0.25
                * (
                    displacement[left] * displacement[left]
                    - displacement[right] * displacement[right]
                )
            )
            momentum[left], momentum[right] = _rot(
                momentum[left],
                momentum[right],
                2.0 * math.pi * (180.0 + 180.0 * morph) / sr * resonance_geometry,
            )

        for index in range(modes):
            old_displacement = displacement[index]
            old_momentum = momentum[index]
            displacement[index] = (
                cosine[index] * old_displacement
                + sine[index] * old_momentum / omega[index]
            )
            momentum[index] = (
                cosine[index] * old_momentum
                - sine[index] * omega[index] * old_displacement
            )
            energy_before = 0.5 / sr * (
                momentum[index] * momentum[index]
                + (omega[index] * displacement[index]) ** 2
            )
            displacement[index] *= retain[index]
            momentum[index] *= retain[index]
            energy_after = 0.5 / sr * (
                momentum[index] * momentum[index]
                + (omega[index] * displacement[index]) ** 2
            )
            declared_loss += max(0.0, energy_before - energy_after)
            maximum_state = max(
                maximum_state,
                abs(momentum[index]),
                abs(omega[index] * displacement[index]),
            )

        emitted = 0.0
        for index in range(modes):
            emitted += radiation_row[index] * momentum[index]
        output[frame] = emitted

    return output, np.array(
        [source_work, configuration_work, declared_loss, maximum_state]
    )


def _rms(values: np.ndarray) -> float:
    return float(np.sqrt(np.mean(np.asarray(values, dtype=np.float64) ** 2)))


def _spectral_metrics(values: np.ndarray, sample_rate: int, target_hz: float) -> dict[str, float]:
    values = np.asarray(values, dtype=np.float64)
    segment = values[int(0.50 * values.size) :]
    window = np.hanning(segment.size)
    spectrum = np.abs(np.fft.rfft(segment * window)) ** 2
    frequencies = np.fft.rfftfreq(segment.size, 1.0 / sample_rate)
    audible = (frequencies >= 20.0) & (frequencies <= 8_000.0)
    total = float(spectrum[audible].sum()) + 1.0e-30
    width = max(3.0, 0.025 * target_hz)
    fundamental = float(
        spectrum[
            (frequencies >= target_hz - width)
            & (frequencies <= target_hz + width)
        ].sum()
    )
    search = (frequencies >= 40.0) & (frequencies <= 2_000.0)
    peak_frequency = float(frequencies[search][np.argmax(spectrum[search])])
    rms = _rms(segment)
    return {
        "rms": rms,
        "peak_frequency_hz": peak_frequency,
        "fundamental_fraction": fundamental / total,
        "harmonic_richness": 1.0 - fundamental / total,
        "crest_factor": float(np.max(np.abs(segment)) / (rms + 1.0e-15)),
    }


def _write_pcm16(path: Path, values: np.ndarray, sample_rate: int, scale: float) -> None:
    samples = np.clip(np.asarray(values) * scale, -1.0, 1.0)
    pcm = (samples * 32767.0).astype("<i2")
    with wave.open(str(path), "wb") as destination:
        destination.setnchannels(1)
        destination.setsampwidth(2)
        destination.setframerate(sample_rate)
        destination.writeframes(pcm.tobytes())


def run(output_directory: Path, sample_rate: int = 96_000) -> dict[str, object]:
    output_directory.mkdir(parents=True, exist_ok=True)
    render_shared_body(48_000, 0.05, 0)

    guitar, guitar_receipt = render_shared_body(sample_rate, 3.0, 0)
    trombone, trombone_receipt = render_shared_body(sample_rate, 3.0, 1)
    morph, morph_receipt = render_shared_body(sample_rate, 4.0, 2)

    performance: dict[str, dict[str, float]] = {}
    for rate in (48_000, 96_000, 192_000):
        attempts = []
        for _ in range(5):
            started = time.perf_counter()
            render_shared_body(rate, 1.0, 1)
            attempts.append(time.perf_counter() - started)
        elapsed = min(attempts)
        performance[str(rate)] = {
            "seconds": elapsed,
            "real_time_factor": 1.0 / elapsed,
            "ns_per_sample": 1.0e9 * elapsed / rate,
        }

    shared_peak = max(
        float(np.max(np.abs(guitar))),
        float(np.max(np.abs(trombone))),
        float(np.max(np.abs(morph))),
    )
    delivery_scale = 0.78 / shared_peak
    _write_pcm16(output_directory / "01_same_body_electric_guitar_clean.wav", guitar, sample_rate, delivery_scale)
    _write_pcm16(output_directory / "02_same_body_trombone_sustain.wav", trombone, sample_rate, delivery_scale)
    _write_pcm16(output_directory / "03_same_body_continuous_guitar_to_trombone.wav", morph, sample_rate, delivery_scale)

    first_attack = guitar[int(0.05 * sample_rate) : int(0.15 * sample_rate)]
    first_tail = guitar[int(0.30 * sample_rate) : int(0.42 * sample_rate)]
    late_morph = morph[int(2.2 * sample_rate) :]
    result: dict[str, object] = {
        "sample_rate_hz": sample_rate,
        "state_dimension": 24,
        "same_body": True,
        "renderer_switch": False,
        "field_reset_in_morph": False,
        "shared_delivery_scale": delivery_scale,
        "guitar": {
            "peak": float(np.max(np.abs(guitar))),
            "rms": _rms(guitar),
            "first_attack_rms": _rms(first_attack),
            "first_tail_rms": _rms(first_tail),
            "first_note_decay_ratio": _rms(first_tail) / (_rms(first_attack) + 1.0e-15),
            "receipt": {
                "source_work": float(guitar_receipt[0]),
                "configuration_work": float(guitar_receipt[1]),
                "declared_loss": float(guitar_receipt[2]),
                "maximum_state": float(guitar_receipt[3]),
            },
        },
        "trombone": {
            "peak": float(np.max(np.abs(trombone))),
            "rms": _rms(trombone),
            **_spectral_metrics(trombone, sample_rate, 116.5),
            "receipt": {
                "source_work": float(trombone_receipt[0]),
                "configuration_work": float(trombone_receipt[1]),
                "declared_loss": float(trombone_receipt[2]),
                "maximum_state": float(trombone_receipt[3]),
            },
        },
        "continuous_morph": {
            "peak": float(np.max(np.abs(morph))),
            "rms": _rms(morph),
            "late": _spectral_metrics(late_morph, sample_rate, 116.5),
            "receipt": {
                "source_work": float(morph_receipt[0]),
                "configuration_work": float(morph_receipt[1]),
                "declared_loss": float(morph_receipt[2]),
                "maximum_state": float(morph_receipt[3]),
            },
        },
        "performance": performance,
        "claim_boundary": {
            "listener_validated": False,
            "named_instrument_fidelity": False,
            "rust_execution_certified": False,
        },
    }
    (output_directory / "campaign-results.json").write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=Path("artifacts/ariadne-string-brass-v1"))
    parser.add_argument("--sample-rate", type=int, default=96_000)
    arguments = parser.parse_args()
    result = run(arguments.output, arguments.sample_rate)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
