"""Speaker-specific GARGANTA anatomy evolution.

This module implements the first executable slice of GARGANTA G7:

* infer a *speaker-conditioned acoustic throat* from a reference waveform;
* keep static anatomy separate from frame-varying articulation and source;
* return an ensemble of acoustically equivalent anatomies rather than
  pretending that one waveform uniquely reveals biological anatomy;
* score anatomy on held-out acoustic states so the outer loop cannot win by
  memorising only the calibration frames.

The implementation is intentionally a reference model, not the production
Rust lowering. It uses a smooth area-function family and a frequency-domain
Webster/transmission-line solver for the fast map consumed by the bilevel
search. The resulting JSON is suitable as an oracle/fixture for a later
Kelly-Lochbaum runtime implementation.

Claim discipline
----------------
The fitted object is an ``acoustically_equivalent_speaker_throat``. It is
not a medical reconstruction, and the API exposes posterior spread and
identifiability diagnostics explicitly. The reference audio may contain
microphone/room coloration and source-filter ambiguity; the analysis removes
shared low-order spectral tilt before fitting tract shape and reports the
removed slope as a separate source/channel nuisance estimate.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from hashlib import sha256
import json
import math
from pathlib import Path
from typing import Callable, Sequence

import numpy as np

try:  # Optional ``audio`` extra; keep the failure message local and useful.
    from scipy import ndimage, optimize, signal, stats
except ImportError as exc:  # pragma: no cover - exercised only without extras.
    raise ImportError(
        "GARGANTA anatomy evolution requires the project 'audio' extra "
        "(numpy, scipy, soundfile)."
    ) from exc


SPEED_OF_SOUND_M_S = 343.0
AIR_DENSITY_KG_M3 = 1.204
DEFAULT_ANALYSIS_SR = 16_000
DEFAULT_SECTION_COUNT = 24
CLAIM_CLASS = "acoustically_equivalent_speaker_throat"
SCHEMA_ID = "https://sophia.systems/schemas/garganta-speaker-throat-v1.json"

CONTROL_NAMES = (
    "jaw_open",
    "tongue_front",
    "tongue_back",
    "lip_round",
    "apical_constriction",
)
ANATOMY_PARAMETER_NAMES = (
    "tract_length_cm",
    "oral_pharyngeal_warp",
    "neutral_pharynx",
    "neutral_tongue_root",
    "neutral_oral",
    "neutral_lips",
    "front_mode_scale",
    "back_mode_scale",
    "lip_area_scale",
    "wall_loss",
)


def _require_finite(name: str, values: np.ndarray) -> None:
    if not np.all(np.isfinite(values)):
        raise ValueError(f"{name} contains NaN or infinity")


def _as_mono(audio: np.ndarray) -> np.ndarray:
    x = np.asarray(audio, dtype=np.float64)
    if x.ndim == 2:
        x = x.mean(axis=1)
    if x.ndim != 1:
        raise ValueError("reference audio must be mono or channels-last stereo")
    _require_finite("reference audio", x)
    if x.size < 256:
        raise ValueError("reference audio is too short")
    x = x - float(np.mean(x))
    peak = float(np.max(np.abs(x)))
    if peak <= 1e-12:
        raise ValueError("reference audio is silent")
    return x / peak


def _resample(audio: np.ndarray, source_sr: int, target_sr: int) -> np.ndarray:
    if source_sr <= 0 or target_sr <= 0:
        raise ValueError("sample rates must be positive")
    if source_sr == target_sr:
        return np.asarray(audio, dtype=np.float64)
    divisor = math.gcd(source_sr, target_sr)
    return signal.resample_poly(
        np.asarray(audio, dtype=np.float64),
        target_sr // divisor,
        source_sr // divisor,
    )


def _stable_softmax_from_losses(losses: np.ndarray, temperature: float) -> np.ndarray:
    losses = np.asarray(losses, dtype=np.float64)
    if losses.ndim != 1 or not losses.size:
        raise ValueError("losses must be a non-empty vector")
    scale = max(float(temperature), 1e-9)
    logits = -(losses - float(losses.min())) / scale
    logits -= float(logits.max())
    weights = np.exp(logits)
    total = float(weights.sum())
    if total <= 0 or not math.isfinite(total):
        return np.full(losses.shape, 1.0 / losses.size)
    return weights / total


def _json_float(value: float | np.floating) -> float:
    value = float(value)
    if not math.isfinite(value):
        raise ValueError("non-finite value cannot be serialized")
    return value


@dataclass(frozen=True)
class EvolutionConfig:
    """Deterministic search and analysis settings.

    Defaults are deliberately modest enough for a laptop smoke run. A
    publication run should use at least 48 generations, population 48, a
    512-point control bank, and 12-16 representative states.
    """

    seed: int = 0x6A726761  # "garga" in the same spirit as prior tools.
    analysis_sample_rate: int = DEFAULT_ANALYSIS_SR
    section_count: int = DEFAULT_SECTION_COUNT
    frequency_bins: int = 192
    frequency_min_hz: float = 90.0
    frequency_max_hz: float = 5_200.0
    representative_states: int = 10
    holdout_stride: int = 3
    control_bank_size: int = 128
    generations: int = 20
    population: int = 24
    elite_fraction: float = 0.25
    exploration_fraction: float = 0.25
    effort_weight: float = 0.015
    anatomy_prior_weight: float = 0.02
    holdout_weight: float = 0.45
    smoothness_weight: float = 0.015
    posterior_size: int = 12
    posterior_temperature: float = 0.06
    isophonemy_threshold: float = 0.055
    local_polish: bool = True

    def validate(self) -> None:
        if self.analysis_sample_rate < 8_000:
            raise ValueError("analysis sample rate must be at least 8 kHz")
        if self.section_count < 12:
            raise ValueError("section_count must be at least 12")
        if self.frequency_bins < 64:
            raise ValueError("frequency_bins must be at least 64")
        if not 0 < self.frequency_min_hz < self.frequency_max_hz:
            raise ValueError("invalid frequency range")
        if self.frequency_max_hz >= self.analysis_sample_rate / 2:
            raise ValueError("frequency_max_hz must be below Nyquist")
        if self.representative_states < 4:
            raise ValueError("at least four representative states are required")
        if self.holdout_stride < 2:
            raise ValueError("holdout_stride must be at least two")
        if self.control_bank_size < 16:
            raise ValueError("control_bank_size must be at least 16")
        if self.generations < 1 or self.population < 8:
            raise ValueError("search requires generations >= 1 and population >= 8")
        if not 0 < self.elite_fraction <= 0.5:
            raise ValueError("elite_fraction must be in (0, 0.5]")
        if not 0 <= self.exploration_fraction <= 0.75:
            raise ValueError("exploration_fraction must be in [0, 0.75]")
        if self.posterior_size < 1:
            raise ValueError("posterior_size must be positive")

    def to_dict(self) -> dict[str, object]:
        return {
            name: getattr(self, name)
            for name in self.__dataclass_fields__  # type: ignore[attr-defined]
        }


@dataclass(frozen=True)
class AnatomyGenome:
    """Static speaker-throat parameters.

    The neutral coefficients live in log-area space. All values are bounded
    by :meth:`from_unit_cube`; constructing directly is allowed for tests and
    hand-authored priors, but :meth:`validate` should be called before use.
    """

    tract_length_cm: float
    oral_pharyngeal_warp: float
    neutral_coefficients: tuple[float, float, float, float]
    front_mode_scale: float
    back_mode_scale: float
    lip_area_scale: float
    wall_loss: float

    @classmethod
    def from_unit_cube(cls, unit: Sequence[float]) -> "AnatomyGenome":
        u = np.asarray(unit, dtype=np.float64)
        if u.shape != (10,):
            raise ValueError("anatomy genome must have ten unit-cube coordinates")
        if np.any((u < 0.0) | (u > 1.0)):
            raise ValueError("unit-cube anatomy coordinates must be in [0, 1]")
        neutral = tuple(float(-0.72 + 1.44 * value) for value in u[2:6])
        return cls(
            tract_length_cm=float(13.5 + 7.5 * u[0]),
            oral_pharyngeal_warp=float(0.78 + 0.44 * u[1]),
            neutral_coefficients=neutral,  # type: ignore[arg-type]
            front_mode_scale=float(0.45 + 1.35 * u[6]),
            back_mode_scale=float(0.45 + 1.35 * u[7]),
            lip_area_scale=float(0.55 + 1.25 * u[8]),
            wall_loss=float(0.002 + 0.035 * u[9]),
        )

    def to_unit_cube(self) -> np.ndarray:
        values = np.empty(10, dtype=np.float64)
        values[0] = (self.tract_length_cm - 13.5) / 7.5
        values[1] = (self.oral_pharyngeal_warp - 0.78) / 0.44
        values[2:6] = (np.asarray(self.neutral_coefficients) + 0.72) / 1.44
        values[6] = (self.front_mode_scale - 0.45) / 1.35
        values[7] = (self.back_mode_scale - 0.45) / 1.35
        values[8] = (self.lip_area_scale - 0.55) / 1.25
        values[9] = (self.wall_loss - 0.002) / 0.035
        return np.clip(values, 0.0, 1.0)

    def validate(self) -> None:
        if not 12.0 <= self.tract_length_cm <= 23.0:
            raise ValueError("tract length is outside the supported range")
        if not 0.65 <= self.oral_pharyngeal_warp <= 1.35:
            raise ValueError("oral/pharyngeal warp is outside the supported range")
        if len(self.neutral_coefficients) != 4:
            raise ValueError("four neutral coefficients are required")
        if not np.all(np.isfinite(self.neutral_coefficients)):
            raise ValueError("neutral coefficients must be finite")
        if self.front_mode_scale <= 0 or self.back_mode_scale <= 0:
            raise ValueError("mode scales must be positive")
        if self.lip_area_scale <= 0:
            raise ValueError("lip area scale must be positive")
        if self.wall_loss < 0:
            raise ValueError("wall loss cannot be negative")

    def to_dict(self) -> dict[str, object]:
        return {
            "tract_length_cm": _json_float(self.tract_length_cm),
            "oral_pharyngeal_warp": _json_float(self.oral_pharyngeal_warp),
            "neutral_coefficients": [
                _json_float(value) for value in self.neutral_coefficients
            ],
            "front_mode_scale": _json_float(self.front_mode_scale),
            "back_mode_scale": _json_float(self.back_mode_scale),
            "lip_area_scale": _json_float(self.lip_area_scale),
            "wall_loss": _json_float(self.wall_loss),
        }


@dataclass(frozen=True)
class AcousticState:
    state_id: str
    time_seconds: float
    normalized_envelope: tuple[float, ...]
    formants_hz: tuple[float, ...]
    f0_hz: float | None
    periodicity: float
    rms_dbfs: float
    source_tilt_db_per_octave: float

    def envelope_array(self) -> np.ndarray:
        return np.asarray(self.normalized_envelope, dtype=np.float64)

    def to_dict(self) -> dict[str, object]:
        return {
            "state_id": self.state_id,
            "time_seconds": _json_float(self.time_seconds),
            "normalized_envelope": [
                _json_float(value) for value in self.normalized_envelope
            ],
            "formants_hz": [_json_float(value) for value in self.formants_hz],
            "f0_hz": None if self.f0_hz is None else _json_float(self.f0_hz),
            "periodicity": _json_float(self.periodicity),
            "rms_dbfs": _json_float(self.rms_dbfs),
            "source_tilt_db_per_octave": _json_float(
                self.source_tilt_db_per_octave
            ),
        }


@dataclass(frozen=True)
class ReferenceAnalysis:
    sample_rate: int
    duration_seconds: float
    frequency_grid_hz: tuple[float, ...]
    states: tuple[AcousticState, ...]
    training_state_ids: tuple[str, ...]
    holdout_state_ids: tuple[str, ...]
    voiced_frame_count: int
    total_frame_count: int
    median_f0_hz: float | None
    f0_iqr_hz: float | None
    jitter_local: float | None
    median_periodicity: float | None
    median_source_tilt_db_per_octave: float | None

    def to_dict(self) -> dict[str, object]:
        return {
            "sample_rate": self.sample_rate,
            "duration_seconds": _json_float(self.duration_seconds),
            "frequency_grid_hz": [
                _json_float(value) for value in self.frequency_grid_hz
            ],
            "states": [state.to_dict() for state in self.states],
            "training_state_ids": list(self.training_state_ids),
            "holdout_state_ids": list(self.holdout_state_ids),
            "voiced_frame_count": self.voiced_frame_count,
            "total_frame_count": self.total_frame_count,
            "median_f0_hz": (
                None if self.median_f0_hz is None else _json_float(self.median_f0_hz)
            ),
            "f0_iqr_hz": (
                None if self.f0_iqr_hz is None else _json_float(self.f0_iqr_hz)
            ),
            "jitter_local": (
                None if self.jitter_local is None else _json_float(self.jitter_local)
            ),
            "median_periodicity": (
                None
                if self.median_periodicity is None
                else _json_float(self.median_periodicity)
            ),
            "median_source_tilt_db_per_octave": (
                None
                if self.median_source_tilt_db_per_octave is None
                else _json_float(self.median_source_tilt_db_per_octave)
            ),
        }


@dataclass(frozen=True)
class StateFit:
    state_id: str
    controls: tuple[float, ...]
    spectral_loss: float
    target_formants_hz: tuple[float, ...]
    modeled_formants_hz: tuple[float, ...]
    area_function_cm2: tuple[float, ...]

    def to_dict(self) -> dict[str, object]:
        return {
            "state_id": self.state_id,
            "controls": {
                name: _json_float(value)
                for name, value in zip(CONTROL_NAMES, self.controls, strict=True)
            },
            "spectral_loss": _json_float(self.spectral_loss),
            "target_formants_hz": [
                _json_float(value) for value in self.target_formants_hz
            ],
            "modeled_formants_hz": [
                _json_float(value) for value in self.modeled_formants_hz
            ],
            "area_function_cm2": [
                _json_float(value) for value in self.area_function_cm2
            ],
        }


@dataclass(frozen=True)
class PosteriorMember:
    member_id: str
    weight: float
    anatomy: AnatomyGenome
    objective_loss: float
    training_loss: float
    holdout_loss: float
    prior_penalty: float
    smoothness_penalty: float
    isophonemy_distance_from_map: float

    def to_dict(self) -> dict[str, object]:
        return {
            "member_id": self.member_id,
            "weight": _json_float(self.weight),
            "anatomy": self.anatomy.to_dict(),
            "objective_loss": _json_float(self.objective_loss),
            "training_loss": _json_float(self.training_loss),
            "holdout_loss": _json_float(self.holdout_loss),
            "prior_penalty": _json_float(self.prior_penalty),
            "smoothness_penalty": _json_float(self.smoothness_penalty),
            "isophonemy_distance_from_map": _json_float(
                self.isophonemy_distance_from_map
            ),
        }


@dataclass(frozen=True)
class SpeakerThroatFit:
    reference_sha256: str
    reference_name: str
    consent_attestation: str | None
    config: EvolutionConfig
    analysis: ReferenceAnalysis
    map_anatomy: AnatomyGenome
    posterior: tuple[PosteriorMember, ...]
    state_fits: tuple[StateFit, ...]
    best_training_loss: float
    best_holdout_loss: float
    identifiability_rank: int
    weak_anatomy_directions: tuple[str, ...]
    diagnostics: tuple[str, ...] = field(default_factory=tuple)

    def to_dict(self) -> dict[str, object]:
        return {
            "$schema": SCHEMA_ID,
            "schema_version": "1.0.0",
            "claim": {
                "class": CLAIM_CLASS,
                "medical_anatomy_reconstruction": False,
                "speaker_identity_model": True,
                "statement": (
                    "Posterior family of speaker-conditioned acoustic throats; "
                    "not a uniquely identified biological anatomy."
                ),
            },
            "provenance": {
                "reference_sha256": self.reference_sha256,
                "reference_name": self.reference_name,
                "consent_attestation": self.consent_attestation,
            },
            "config": self.config.to_dict(),
            "reference_analysis": self.analysis.to_dict(),
            "map_anatomy": self.map_anatomy.to_dict(),
            "posterior": [member.to_dict() for member in self.posterior],
            "state_fits": [fit.to_dict() for fit in self.state_fits],
            "evidence": {
                "best_training_loss": _json_float(self.best_training_loss),
                "best_holdout_loss": _json_float(self.best_holdout_loss),
                "identifiability_rank": self.identifiability_rank,
                "anatomy_parameter_count": len(ANATOMY_PARAMETER_NAMES),
                "weak_anatomy_directions": list(self.weak_anatomy_directions),
                "diagnostics": list(self.diagnostics),
            },
        }

    def write_json(self, path: str | Path) -> Path:
        destination = Path(path)
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(
            json.dumps(self.to_dict(), indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        return destination


# ---------------------------------------------------------------------------
# Anatomy geometry and the fast acoustic map


def _warped_coordinate(x: np.ndarray, warp: float) -> np.ndarray:
    """Smooth oral/pharyngeal reallocation while preserving [0, 1]."""

    # A monotone power warp around the palate bend. Values >1 allocate more
    # normalized resolution/length to the oral side; values <1 to pharynx.
    x = np.asarray(x, dtype=np.float64)
    left = 0.46 * np.power(np.clip(x / 0.46, 0.0, 1.0), warp)
    right_fraction = np.clip((x - 0.46) / 0.54, 0.0, 1.0)
    right = 0.46 + 0.54 * (
        1.0 - np.power(1.0 - right_fraction, 1.0 / max(warp, 1e-6))
    )
    return np.where(x <= 0.46, left, right)


def _gaussian(x: np.ndarray, center: float, width: float) -> np.ndarray:
    return np.exp(-0.5 * ((x - center) / width) ** 2)


def neutral_log_area(anatomy: AnatomyGenome, section_count: int) -> np.ndarray:
    anatomy.validate()
    if section_count < 12:
        raise ValueError("section_count must be at least 12")
    x = (np.arange(section_count, dtype=np.float64) + 0.5) / section_count
    w = _warped_coordinate(x, anatomy.oral_pharyngeal_warp)

    # Neutral tract: gently narrow epilarynx and lips, generous pharyngeal
    # and oral cavities. Coefficients are speaker-static deformations.
    base = np.log(3.0) * np.ones_like(w)
    base += -0.62 * _gaussian(w, 0.06, 0.055)
    base += +0.28 * _gaussian(w, 0.27, 0.16)
    base += +0.16 * _gaussian(w, 0.68, 0.19)
    base += -0.34 * _gaussian(w, 0.97, 0.045)

    static_basis = np.stack(
        [
            _gaussian(w, 0.22, 0.15),
            _gaussian(w, 0.43, 0.13),
            _gaussian(w, 0.70, 0.18),
            _gaussian(w, 0.94, 0.075),
        ],
        axis=0,
    )
    base += np.asarray(anatomy.neutral_coefficients) @ static_basis
    base[-3:] += math.log(anatomy.lip_area_scale)
    return base


def articulation_basis(anatomy: AnatomyGenome, section_count: int) -> np.ndarray:
    x = (np.arange(section_count, dtype=np.float64) + 0.5) / section_count
    w = _warped_coordinate(x, anatomy.oral_pharyngeal_warp)

    jaw = 0.75 * _gaussian(w, 0.73, 0.22) + 0.18 * _gaussian(w, 0.40, 0.18)
    tongue_front = anatomy.front_mode_scale * (
        -1.15 * _gaussian(w, 0.72, 0.095)
        + 0.42 * _gaussian(w, 0.38, 0.16)
    )
    tongue_back = anatomy.back_mode_scale * (
        -1.12 * _gaussian(w, 0.43, 0.11)
        + 0.38 * _gaussian(w, 0.72, 0.16)
    )
    lip_round = -1.30 * _gaussian(w, 0.95, 0.055) - 0.20 * _gaussian(
        w, 0.78, 0.12
    )
    apical = -1.55 * _gaussian(w, 0.84, 0.050) + 0.25 * _gaussian(
        w, 0.65, 0.11
    )
    return np.stack([jaw, tongue_front, tongue_back, lip_round, apical], axis=0)


def area_function_cm2(
    anatomy: AnatomyGenome,
    controls: Sequence[float],
    section_count: int = DEFAULT_SECTION_COUNT,
) -> np.ndarray:
    q = np.asarray(controls, dtype=np.float64)
    if q.shape != (len(CONTROL_NAMES),):
        raise ValueError(f"controls must have {len(CONTROL_NAMES)} values")
    if np.any((q < -1.0) | (q > 1.0)):
        raise ValueError("controls must lie in [-1, 1]")
    log_area = neutral_log_area(anatomy, section_count)
    log_area += q @ articulation_basis(anatomy, section_count)
    # Positive area cone. 0.025 cm² approximates near closure without
    # allowing an exactly singular acoustic tube in this fast model.
    return np.exp(np.clip(log_area, math.log(0.025), math.log(18.0)))


def area_function_bank_cm2(
    anatomy: AnatomyGenome,
    controls: np.ndarray,
    section_count: int = DEFAULT_SECTION_COUNT,
) -> np.ndarray:
    q = np.asarray(controls, dtype=np.float64)
    if q.ndim != 2 or q.shape[1] != len(CONTROL_NAMES):
        raise ValueError("controls bank has the wrong shape")
    if np.any((q < -1.0) | (q > 1.0)):
        raise ValueError("controls bank must lie in [-1, 1]")
    log_area = neutral_log_area(anatomy, section_count)[None, :]
    log_area = log_area + q @ articulation_basis(anatomy, section_count)
    return np.exp(np.clip(log_area, math.log(0.025), math.log(18.0)))


def reflection_coefficients(areas_cm2: Sequence[float]) -> np.ndarray:
    areas = np.asarray(areas_cm2, dtype=np.float64)
    if areas.ndim != 1 or areas.size < 2:
        raise ValueError("at least two positive areas are required")
    if np.any(areas <= 0) or not np.all(np.isfinite(areas)):
        raise ValueError("areas must be positive and finite")
    # Pressure-wave reflection coefficient looking from section i into i+1.
    coefficients = (areas[:-1] - areas[1:]) / (areas[:-1] + areas[1:])
    return np.clip(coefficients, -1.0 + 1e-12, 1.0 - 1e-12)


def _radiation_impedance(
    frequencies_hz: np.ndarray,
    lip_area_m2: np.ndarray,
) -> np.ndarray:
    """Low-order unflanged-pipe radiation load.

    The approximation is intentionally simple but physical enough for the
    fast outer loop. The production runtime should replace it with the
    packet's certified radiation law and record the model version.
    """

    frequency = frequencies_hz[None, :]
    area = lip_area_m2[:, None]
    radius = np.sqrt(np.maximum(area, 1e-12) / math.pi)
    ka = 2.0 * math.pi * frequency * radius / SPEED_OF_SOUND_M_S
    resistance = 0.25 * ka**2 / (1.0 + 0.35 * ka**2)
    reactance = 0.61 * ka / (1.0 + 0.30 * ka)
    characteristic = AIR_DENSITY_KG_M3 * SPEED_OF_SOUND_M_S / area
    return characteristic * (resistance + 1j * reactance + 1e-5)


def transfer_response_bank(
    anatomy: AnatomyGenome,
    controls: np.ndarray,
    frequencies_hz: Sequence[float],
    *,
    section_count: int = DEFAULT_SECTION_COUNT,
) -> np.ndarray:
    """Return complex lip-volume-velocity / glottal-flow responses.

    ``controls`` is ``[candidate, articulator]``; the result is
    ``[candidate, frequency]``. The calculation is vectorized over both
    candidates and frequency and loops only over tube sections.
    """

    anatomy.validate()
    frequencies = np.asarray(frequencies_hz, dtype=np.float64)
    if frequencies.ndim != 1 or frequencies.size < 8:
        raise ValueError("frequency grid must be a one-dimensional vector")
    if np.any(frequencies <= 0):
        raise ValueError("frequency grid must be strictly positive")
    areas_cm2 = area_function_bank_cm2(anatomy, controls, section_count)
    areas_m2 = areas_cm2 * 1e-4
    count = areas_m2.shape[0]

    # Lip rounding extends the effective tract slightly. This is a dynamic
    # control effect, not a speaker-static anatomy parameter.
    rounding = np.maximum(np.asarray(controls)[:, 3], 0.0)
    effective_length_m = anatomy.tract_length_cm * 0.01 * (1.0 + 0.045 * rounding)
    dx = effective_length_m / section_count

    omega = 2.0 * math.pi * frequencies[None, :]
    # A small sqrt(f) propagation loss approximates wall/viscothermal loss.
    attenuation = anatomy.wall_loss * np.sqrt(frequencies[None, :] / 1_000.0)
    gamma_dx = (1j * omega / SPEED_OF_SOUND_M_S + attenuation) * dx[:, None]
    cosine = np.cosh(gamma_dx)
    sine = np.sinh(gamma_dx)

    m11 = np.ones((count, frequencies.size), dtype=np.complex128)
    m12 = np.zeros_like(m11)
    m21 = np.zeros_like(m11)
    m22 = np.ones_like(m11)

    for section in range(section_count):
        impedance = (
            AIR_DENSITY_KG_M3
            * SPEED_OF_SOUND_M_S
            / areas_m2[:, section]
        )[:, None]
        a11 = cosine
        a12 = impedance * sine
        a21 = sine / impedance
        a22 = cosine
        n11 = m11 * a11 + m12 * a21
        n12 = m11 * a12 + m12 * a22
        n21 = m21 * a11 + m22 * a21
        n22 = m21 * a12 + m22 * a22
        m11, m12, m21, m22 = n11, n12, n21, n22

    load = _radiation_impedance(frequencies, areas_m2[:, -1])
    denominator = m21 * load + m22
    response = 1.0 / np.where(np.abs(denominator) < 1e-18, 1e-18, denominator)
    _require_finite("transfer response real part", response.real)
    _require_finite("transfer response imaginary part", response.imag)
    return response


def transfer_response(
    anatomy: AnatomyGenome,
    controls: Sequence[float],
    frequencies_hz: Sequence[float],
    *,
    section_count: int = DEFAULT_SECTION_COUNT,
) -> np.ndarray:
    return transfer_response_bank(
        anatomy,
        np.asarray(controls, dtype=np.float64)[None, :],
        frequencies_hz,
        section_count=section_count,
    )[0]


def _tilt_projector(frequencies_hz: np.ndarray) -> np.ndarray:
    log_frequency = np.log2(frequencies_hz / 1_000.0)
    design = np.stack(
        [np.ones_like(log_frequency), log_frequency, log_frequency**2], axis=1
    )
    q, _ = np.linalg.qr(design)
    return q


def normalize_envelope_bank(
    log_magnitude_db: np.ndarray,
    frequencies_hz: Sequence[float],
) -> np.ndarray:
    envelope = np.asarray(log_magnitude_db, dtype=np.float64)
    if envelope.ndim == 1:
        envelope = envelope[None, :]
    frequencies = np.asarray(frequencies_hz, dtype=np.float64)
    if envelope.shape[1] != frequencies.size:
        raise ValueError("envelope and frequency grid disagree")
    smoothed = ndimage.gaussian_filter1d(envelope, sigma=1.15, axis=1, mode="nearest")
    basis = _tilt_projector(frequencies)
    residual = smoothed - (smoothed @ basis) @ basis.T
    scale = np.sqrt(np.mean(residual**2, axis=1, keepdims=True))
    return residual / np.maximum(scale, 1e-8)


def modeled_envelope_bank(
    anatomy: AnatomyGenome,
    controls: np.ndarray,
    frequencies_hz: Sequence[float],
    *,
    section_count: int = DEFAULT_SECTION_COUNT,
) -> np.ndarray:
    response = transfer_response_bank(
        anatomy,
        controls,
        frequencies_hz,
        section_count=section_count,
    )
    db = 20.0 * np.log10(np.maximum(np.abs(response), 1e-12))
    return normalize_envelope_bank(db, frequencies_hz)


def modeled_formants(
    anatomy: AnatomyGenome,
    controls: Sequence[float],
    frequencies_hz: Sequence[float],
    *,
    section_count: int = DEFAULT_SECTION_COUNT,
    maximum_formants: int = 4,
) -> tuple[float, ...]:
    frequencies = np.asarray(frequencies_hz, dtype=np.float64)
    response = transfer_response(
        anatomy, controls, frequencies, section_count=section_count
    )
    db = ndimage.gaussian_filter1d(
        20.0 * np.log10(np.maximum(np.abs(response), 1e-12)), sigma=1.1
    )
    spacing_hz = max(180.0, (frequencies[-1] - frequencies[0]) / 25.0)
    spacing_bins = max(1, int(round(spacing_hz / np.median(np.diff(frequencies)))))
    peaks, properties = signal.find_peaks(db, distance=spacing_bins, prominence=0.35)
    if peaks.size == 0:
        return ()
    prominences = properties.get("prominences", np.ones(peaks.size))
    candidates = sorted(
        zip(frequencies[peaks], prominences, strict=True), key=lambda item: item[0]
    )
    # Keep low-to-high resonances, rejecting near-duplicate shoulders.
    selected: list[float] = []
    for frequency, _prominence in candidates:
        if not selected or frequency - selected[-1] >= 180.0:
            selected.append(float(frequency))
        if len(selected) >= maximum_formants:
            break
    return tuple(selected)


# ---------------------------------------------------------------------------
# Reference-audio analysis


def _levinson_durbin(autocorrelation: np.ndarray, order: int) -> tuple[np.ndarray, float]:
    r = np.asarray(autocorrelation, dtype=np.float64)
    if r.size < order + 1:
        raise ValueError("autocorrelation vector is too short")
    if r[0] <= 1e-12:
        return np.concatenate(([1.0], np.zeros(order))), 1e-12
    coefficients = np.zeros(order + 1, dtype=np.float64)
    coefficients[0] = 1.0
    error = float(r[0])
    for index in range(1, order + 1):
        previous = coefficients.copy()
        reflection = -(
            r[index]
            + np.dot(previous[1:index], r[index - 1 : 0 : -1])
        ) / max(error, 1e-12)
        reflection = float(np.clip(reflection, -0.999, 0.999))
        coefficients[index] = reflection
        if index > 1:
            coefficients[1:index] = (
                previous[1:index]
                + reflection * previous[index - 1 : 0 : -1]
            )
        error *= max(1e-6, 1.0 - reflection * reflection)
    return coefficients, max(error, 1e-12)


def _frame_periodicity_and_f0(
    frame: np.ndarray,
    sample_rate: int,
    *,
    minimum_hz: float = 55.0,
    maximum_hz: float = 520.0,
) -> tuple[float | None, float]:
    x = np.asarray(frame, dtype=np.float64)
    x = (x - np.mean(x)) * np.hanning(x.size)
    energy = float(np.dot(x, x))
    if energy <= 1e-12:
        return None, 0.0
    nfft = 1 << int(math.ceil(math.log2(2 * x.size - 1)))
    spectrum = np.fft.rfft(x, n=nfft)
    autocorrelation = np.fft.irfft(np.abs(spectrum) ** 2, n=nfft)[: x.size]
    autocorrelation /= max(float(autocorrelation[0]), 1e-12)
    minimum_lag = max(1, int(sample_rate / maximum_hz))
    maximum_lag = min(x.size - 2, int(sample_rate / minimum_hz))
    if maximum_lag <= minimum_lag:
        return None, 0.0
    region = autocorrelation[minimum_lag : maximum_lag + 1]
    relative = int(np.argmax(region))
    lag = minimum_lag + relative
    periodicity = float(region[relative])
    # Parabolic lag refinement.
    if 1 <= lag < autocorrelation.size - 1:
        left, center, right = autocorrelation[lag - 1 : lag + 2]
        denominator = left - 2.0 * center + right
        if abs(denominator) > 1e-12:
            lag = lag + 0.5 * (left - right) / denominator
    if periodicity < 0.16 or lag <= 0:
        return None, max(0.0, periodicity)
    return float(sample_rate / lag), max(0.0, min(1.0, periodicity))


def _source_tilt_db_per_octave(
    frame: np.ndarray,
    sample_rate: int,
    minimum_hz: float = 250.0,
    maximum_hz: float = 4_500.0,
) -> float:
    nfft = max(1024, 1 << int(math.ceil(math.log2(frame.size))))
    spectrum = np.abs(np.fft.rfft(frame * np.hanning(frame.size), n=nfft))
    frequencies = np.fft.rfftfreq(nfft, 1.0 / sample_rate)
    mask = (frequencies >= minimum_hz) & (frequencies <= maximum_hz)
    if np.count_nonzero(mask) < 8:
        return 0.0
    x = np.log2(frequencies[mask] / 1_000.0)
    y = 20.0 * np.log10(np.maximum(spectrum[mask], 1e-10))
    # Robust-ish fit: trim the most resonant and deepest bins.
    residual_center = y - ndimage.median_filter(y, size=9, mode="nearest")
    keep = np.abs(residual_center) <= np.quantile(np.abs(residual_center), 0.75)
    if np.count_nonzero(keep) < 8:
        keep = np.ones_like(x, dtype=bool)
    slope, _intercept = np.polyfit(x[keep], y[keep], 1)
    return float(slope)


def _lpc_envelope_and_formants(
    frame: np.ndarray,
    sample_rate: int,
    frequencies_hz: np.ndarray,
) -> tuple[np.ndarray, tuple[float, ...]]:
    x = np.asarray(frame, dtype=np.float64)
    if x.size < 64:
        raise ValueError("analysis frame is too short")
    emphasized = signal.lfilter([1.0, -0.97], [1.0], x)
    emphasized *= np.hamming(emphasized.size)
    order = int(np.clip(round(2.0 + sample_rate / 1_000.0), 12, 28))
    autocorrelation = signal.correlate(emphasized, emphasized, mode="full", method="fft")
    midpoint = autocorrelation.size // 2
    r = autocorrelation[midpoint : midpoint + order + 1]
    coefficients, error = _levinson_durbin(r, order)
    angular = 2.0 * math.pi * frequencies_hz / sample_rate
    powers = np.exp(-1j * np.outer(angular, np.arange(coefficients.size)))
    denominator = powers @ coefficients
    envelope = 10.0 * np.log10(error / np.maximum(np.abs(denominator) ** 2, 1e-18))
    normalized = normalize_envelope_bank(envelope, frequencies_hz)[0]

    roots = np.roots(coefficients)
    roots = roots[np.imag(roots) > 0]
    angles = np.arctan2(np.imag(roots), np.real(roots))
    formant_frequencies = angles * sample_rate / (2.0 * math.pi)
    bandwidths = -0.5 * sample_rate / math.pi * np.log(np.maximum(np.abs(roots), 1e-12))
    valid = (
        (formant_frequencies >= 90.0)
        & (formant_frequencies <= min(5_200.0, sample_rate / 2 - 100.0))
        & (bandwidths > 0.0)
        & (bandwidths < 800.0)
    )
    formants = sorted(float(value) for value in formant_frequencies[valid])[:4]
    return normalized, tuple(formants)


def _farthest_point_indices(features: np.ndarray, count: int) -> list[int]:
    if features.ndim != 2 or features.shape[0] == 0:
        return []
    count = min(count, features.shape[0])
    center = np.median(features, axis=0)
    first = int(np.argmin(np.sum((features - center) ** 2, axis=1)))
    selected = [first]
    minimum_distance = np.sum((features - features[first]) ** 2, axis=1)
    while len(selected) < count:
        candidate = int(np.argmax(minimum_distance))
        selected.append(candidate)
        distance = np.sum((features - features[candidate]) ** 2, axis=1)
        minimum_distance = np.minimum(minimum_distance, distance)
        minimum_distance[selected] = -1.0
    return selected


def analyze_reference_audio(
    audio: np.ndarray,
    sample_rate: int,
    config: EvolutionConfig,
) -> ReferenceAnalysis:
    config.validate()
    mono = _as_mono(audio)
    analysis_audio = _resample(mono, sample_rate, config.analysis_sample_rate)
    sr = config.analysis_sample_rate
    frequency_grid = np.geomspace(
        config.frequency_min_hz,
        config.frequency_max_hz,
        config.frequency_bins,
    )

    frame_length = int(round(0.030 * sr))
    hop = int(round(0.010 * sr))
    if analysis_audio.size < frame_length:
        analysis_audio = np.pad(analysis_audio, (0, frame_length - analysis_audio.size))
    starts = np.arange(0, analysis_audio.size - frame_length + 1, hop, dtype=int)
    frames = np.stack([analysis_audio[start : start + frame_length] for start in starts])
    rms = np.sqrt(np.mean(frames**2, axis=1) + 1e-15)
    rms_db = 20.0 * np.log10(rms)
    energy_floor = max(-52.0, float(np.quantile(rms_db, 0.32)))

    raw_states: list[AcousticState] = []
    all_f0: list[float] = []
    all_periodicity: list[float] = []
    for index, (start, frame) in enumerate(zip(starts, frames, strict=True)):
        if rms_db[index] < energy_floor:
            continue
        f0, periodicity = _frame_periodicity_and_f0(frame, sr)
        if f0 is None or periodicity < 0.24:
            continue
        try:
            envelope, formants = _lpc_envelope_and_formants(frame, sr, frequency_grid)
        except (ValueError, np.linalg.LinAlgError):
            continue
        if len(formants) < 2:
            continue
        tilt = _source_tilt_db_per_octave(frame, sr)
        time_seconds = (start + frame_length / 2) / sr
        state = AcousticState(
            state_id=f"frame-{index:05d}",
            time_seconds=float(time_seconds),
            normalized_envelope=tuple(float(value) for value in envelope),
            formants_hz=formants,
            f0_hz=f0,
            periodicity=periodicity,
            rms_dbfs=float(rms_db[index]),
            source_tilt_db_per_octave=tilt,
        )
        raw_states.append(state)
        all_f0.append(f0)
        all_periodicity.append(periodicity)

    if len(raw_states) < 4:
        raise ValueError(
            "reference contains too few stable voiced frames; provide a clearer or "
            "longer sample with several vowels"
        )

    # Cap the medoid candidate set uniformly in time so long recordings do not
    # make the selection quadratic in every voiced frame.
    if len(raw_states) > 360:
        candidate_indices = np.linspace(0, len(raw_states) - 1, 360).astype(int)
        candidates = [raw_states[index] for index in candidate_indices]
    else:
        candidates = raw_states

    envelope_matrix = np.stack([state.envelope_array() for state in candidates])
    # Add explicitly scaled F1-F3 so farthest-point selection respects vowel
    # geometry rather than only fine spectral detail.
    formant_features = np.zeros((len(candidates), 3), dtype=np.float64)
    for row, state in enumerate(candidates):
        available = np.asarray(state.formants_hz[:3], dtype=np.float64)
        formant_features[row, : available.size] = available
        if available.size < 3:
            formant_features[row, available.size :] = np.nan
    column_medians = np.nanmedian(formant_features, axis=0)
    missing = np.where(np.isnan(formant_features))
    formant_features[missing] = np.take(column_medians, missing[1])
    formant_features /= np.array([900.0, 2_200.0, 3_200.0])
    # A compact DCT-like projection of the envelope.
    q, _ = np.linalg.qr(
        np.cos(
            math.pi
            * (np.arange(config.frequency_bins)[:, None] + 0.5)
            * np.arange(8)[None, :]
            / config.frequency_bins
        )
    )
    envelope_features = envelope_matrix @ q
    selection_features = np.concatenate(
        [1.35 * formant_features, 0.45 * envelope_features], axis=1
    )
    representative_indices = _farthest_point_indices(
        selection_features, config.representative_states
    )
    states = tuple(candidates[index] for index in representative_indices)

    holdout_ids = tuple(
        state.state_id
        for position, state in enumerate(states)
        if position % config.holdout_stride == config.holdout_stride - 1
    )
    if not holdout_ids:
        holdout_ids = (states[-1].state_id,)
    training_ids = tuple(
        state.state_id for state in states if state.state_id not in set(holdout_ids)
    )
    if not training_ids:
        training_ids = tuple(state.state_id for state in states[:-1])
        holdout_ids = (states[-1].state_id,)

    f0_values = np.asarray(all_f0, dtype=np.float64)
    periods = 1.0 / f0_values
    jitter = None
    if periods.size >= 3:
        jitter = float(
            np.mean(np.abs(np.diff(periods))) / max(float(np.mean(periods)), 1e-12)
        )
    return ReferenceAnalysis(
        sample_rate=sr,
        duration_seconds=float(analysis_audio.size / sr),
        frequency_grid_hz=tuple(float(value) for value in frequency_grid),
        states=states,
        training_state_ids=training_ids,
        holdout_state_ids=holdout_ids,
        voiced_frame_count=len(raw_states),
        total_frame_count=len(starts),
        median_f0_hz=float(np.median(f0_values)) if f0_values.size else None,
        f0_iqr_hz=(
            float(np.quantile(f0_values, 0.75) - np.quantile(f0_values, 0.25))
            if f0_values.size
            else None
        ),
        jitter_local=jitter,
        median_periodicity=(
            float(np.median(all_periodicity)) if all_periodicity else None
        ),
        median_source_tilt_db_per_octave=float(
            np.median([state.source_tilt_db_per_octave for state in raw_states])
        ),
    )


# ---------------------------------------------------------------------------
# Bilevel anatomy evolution


def build_control_bank(size: int, seed: int) -> np.ndarray:
    if size < 16:
        raise ValueError("control bank must contain at least sixteen points")
    dimension = len(CONTROL_NAMES)
    power = int(math.ceil(math.log2(size)))
    sampler = stats.qmc.Sobol(d=dimension, scramble=True, seed=seed)
    unit = sampler.random_base2(power)[:size]
    controls = 2.0 * unit - 1.0
    # Guaranteed landmarks improve stability and make the atlas interpretable.
    landmarks = [np.zeros(dimension)]
    for axis in range(dimension):
        positive = np.zeros(dimension)
        negative = np.zeros(dimension)
        positive[axis] = 0.9
        negative[axis] = -0.9
        landmarks.extend([positive, negative])
    controls[: len(landmarks)] = np.stack(landmarks)
    return controls


def _state_distance_matrix(
    predicted: np.ndarray,
    targets: np.ndarray,
) -> np.ndarray:
    # Mean squared Euclidean distance, implemented without a K*S*F tensor.
    frequency_count = predicted.shape[1]
    predicted_norm = np.sum(predicted**2, axis=1)[:, None]
    target_norm = np.sum(targets**2, axis=1)[None, :]
    squared = predicted_norm + target_norm - 2.0 * predicted @ targets.T
    return np.maximum(squared, 0.0) / frequency_count


def _anatomy_prior_penalty(unit_genome: np.ndarray) -> float:
    # Weak human prior: prefer the interior, especially for length and losses,
    # without preventing the posterior from moving to the boundary if audio
    # demands it.
    center = np.full(10, 0.5)
    weights = np.array([1.2, 0.8, 0.45, 0.45, 0.45, 0.55, 0.5, 0.5, 0.5, 0.8])
    return float(np.mean(weights * (unit_genome - center) ** 2))


def _anatomy_smoothness_penalty(
    anatomy: AnatomyGenome,
    section_count: int,
) -> float:
    neutral = neutral_log_area(anatomy, section_count)
    second_difference = np.diff(neutral, n=2)
    return float(np.mean(second_difference**2))


@dataclass(frozen=True)
class _CandidateScore:
    unit_genome: tuple[float, ...]
    anatomy: AnatomyGenome
    objective_loss: float
    training_loss: float
    holdout_loss: float
    prior_penalty: float
    smoothness_penalty: float
    chosen_control_indices: tuple[int, ...]
    state_losses: tuple[float, ...]


class _AnatomyObjective:
    def __init__(
        self,
        analysis: ReferenceAnalysis,
        config: EvolutionConfig,
        control_bank: np.ndarray,
    ) -> None:
        self.analysis = analysis
        self.config = config
        self.control_bank = np.asarray(control_bank, dtype=np.float64)
        self.frequencies = np.asarray(analysis.frequency_grid_hz, dtype=np.float64)
        self.states = analysis.states
        self.targets = np.stack([state.envelope_array() for state in self.states])
        training_set = set(analysis.training_state_ids)
        holdout_set = set(analysis.holdout_state_ids)
        self.training_indices = np.array(
            [index for index, state in enumerate(self.states) if state.state_id in training_set],
            dtype=int,
        )
        self.holdout_indices = np.array(
            [index for index, state in enumerate(self.states) if state.state_id in holdout_set],
            dtype=int,
        )
        self.effort = np.mean(
            self.control_bank**2
            * np.array([0.75, 1.0, 1.0, 0.85, 1.25])[None, :],
            axis=1,
        )
        self.cache: dict[bytes, _CandidateScore] = {}

    def __call__(self, unit_genome: Sequence[float]) -> _CandidateScore:
        unit = np.clip(np.asarray(unit_genome, dtype=np.float64), 0.0, 1.0)
        key = np.round(unit, 8).tobytes()
        cached = self.cache.get(key)
        if cached is not None:
            return cached
        anatomy = AnatomyGenome.from_unit_cube(unit)
        predicted = modeled_envelope_bank(
            anatomy,
            self.control_bank,
            self.frequencies,
            section_count=self.config.section_count,
        )
        distances = _state_distance_matrix(predicted, self.targets)
        weighted = distances + self.config.effort_weight * self.effort[:, None]
        chosen = np.argmin(weighted, axis=0)
        state_losses = distances[chosen, np.arange(len(self.states))]
        training_loss = float(np.mean(state_losses[self.training_indices]))
        holdout_loss = float(np.mean(state_losses[self.holdout_indices]))
        prior = _anatomy_prior_penalty(unit)
        smoothness = _anatomy_smoothness_penalty(
            anatomy, self.config.section_count
        )
        objective = (
            training_loss
            + self.config.holdout_weight * holdout_loss
            + self.config.anatomy_prior_weight * prior
            + self.config.smoothness_weight * smoothness
        )
        score = _CandidateScore(
            unit_genome=tuple(float(value) for value in unit),
            anatomy=anatomy,
            objective_loss=float(objective),
            training_loss=training_loss,
            holdout_loss=holdout_loss,
            prior_penalty=prior,
            smoothness_penalty=smoothness,
            chosen_control_indices=tuple(int(value) for value in chosen),
            state_losses=tuple(float(value) for value in state_losses),
        )
        self.cache[key] = score
        return score


ProgressCallback = Callable[[int, _CandidateScore], None]


def _evolve_anatomies(
    objective: _AnatomyObjective,
    config: EvolutionConfig,
    progress: ProgressCallback | None,
) -> list[_CandidateScore]:
    rng = np.random.default_rng(config.seed)
    sobol = stats.qmc.Sobol(d=10, scramble=True, seed=config.seed ^ 0xA11A)
    initial_power = int(math.ceil(math.log2(config.population)))
    population = sobol.random_base2(initial_power)[: config.population]
    archive: dict[bytes, _CandidateScore] = {}
    elite_count = max(3, int(round(config.population * config.elite_fraction)))
    exploration_count = int(round(config.population * config.exploration_fraction))
    mutation_count = config.population - exploration_count
    mean = np.full(10, 0.5)
    sigma = np.full(10, 0.28)

    for generation in range(config.generations):
        scores = [objective(candidate) for candidate in population]
        scores.sort(key=lambda item: item.objective_loss)
        for score in scores:
            key = np.round(np.asarray(score.unit_genome), 7).tobytes()
            prior = archive.get(key)
            if prior is None or score.objective_loss < prior.objective_loss:
                archive[key] = score
        elites = scores[:elite_count]
        elite_matrix = np.stack([member.unit_genome for member in elites])
        rank_weights = np.linspace(1.0, 0.25, elite_count)
        rank_weights /= rank_weights.sum()
        elite_mean = np.average(elite_matrix, axis=0, weights=rank_weights)
        elite_std = np.sqrt(
            np.average((elite_matrix - elite_mean) ** 2, axis=0, weights=rank_weights)
        )
        mean = 0.55 * mean + 0.45 * elite_mean
        sigma = np.clip(0.62 * sigma + 0.38 * (elite_std + 0.035), 0.025, 0.34)

        if progress is not None:
            progress(generation, scores[0])

        # Elitism: preserve the champion and seed the rest around the adapted
        # diagonal proposal. A Sobol exploration tranche prevents premature
        # collapse and keeps the manifest reproducible.
        next_population = [np.asarray(scores[0].unit_genome)]
        while len(next_population) < mutation_count:
            candidate = mean + sigma * rng.standard_normal(10)
            next_population.append(np.clip(candidate, 0.0, 1.0))
        if exploration_count:
            exploration = sobol.random(exploration_count)
            next_population.extend(exploration)
        population = np.stack(next_population[: config.population])

    ranked = sorted(archive.values(), key=lambda item: item.objective_loss)
    if config.local_polish and ranked:
        champion = np.asarray(ranked[0].unit_genome, dtype=np.float64)

        def scalar(unit: np.ndarray) -> float:
            return objective(unit).objective_loss

        result = optimize.minimize(
            scalar,
            champion,
            method="Powell",
            bounds=[(0.0, 1.0)] * 10,
            options={"maxiter": 45, "xtol": 2e-3, "ftol": 2e-4},
        )
        polished = objective(np.clip(result.x, 0.0, 1.0))
        ranked.append(polished)
        ranked.sort(key=lambda item: item.objective_loss)
    return ranked


def isophonemy_distance(
    first: AnatomyGenome,
    second: AnatomyGenome,
    frequencies_hz: Sequence[float],
    *,
    control_bank: np.ndarray,
    section_count: int = DEFAULT_SECTION_COUNT,
) -> float:
    """Symmetric atlas distance between two anatomy affordance sets.

    The value is a bidirectional Chamfer distance between normalized spectral
    envelopes over the same canonical control atlas. Zero means the two
    anatomies are indistinguishable under this observable and control bank;
    it does not claim geometric identity.
    """

    first_atlas = modeled_envelope_bank(
        first, control_bank, frequencies_hz, section_count=section_count
    )
    second_atlas = modeled_envelope_bank(
        second, control_bank, frequencies_hz, section_count=section_count
    )
    distances = _state_distance_matrix(first_atlas, second_atlas)
    return float(0.5 * (np.mean(np.min(distances, axis=1)) + np.mean(np.min(distances, axis=0))))


def _deduplicated_posterior(
    ranked: Sequence[_CandidateScore],
    analysis: ReferenceAnalysis,
    config: EvolutionConfig,
    control_bank: np.ndarray,
) -> tuple[PosteriorMember, ...]:
    if not ranked:
        return ()
    frequencies = np.asarray(analysis.frequency_grid_hz)
    atlas_bank = control_bank[: min(64, control_bank.shape[0])]
    selected: list[tuple[_CandidateScore, float]] = []
    map_score = ranked[0]
    for candidate in ranked:
        distance_to_map = isophonemy_distance(
            map_score.anatomy,
            candidate.anatomy,
            frequencies,
            control_bank=atlas_bank,
            section_count=config.section_count,
        )
        duplicate = False
        for existing, _ in selected:
            distance = isophonemy_distance(
                existing.anatomy,
                candidate.anatomy,
                frequencies,
                control_bank=atlas_bank,
                section_count=config.section_count,
            )
            if distance < config.isophonemy_threshold:
                duplicate = True
                break
        if duplicate and selected:
            continue
        selected.append((candidate, distance_to_map))
        if len(selected) >= config.posterior_size:
            break
    if not selected:
        selected = [(map_score, 0.0)]
    losses = np.array([candidate.objective_loss for candidate, _ in selected])
    weights = _stable_softmax_from_losses(losses, config.posterior_temperature)
    return tuple(
        PosteriorMember(
            member_id=f"anatomy-{index:03d}",
            weight=float(weights[index]),
            anatomy=candidate.anatomy,
            objective_loss=candidate.objective_loss,
            training_loss=candidate.training_loss,
            holdout_loss=candidate.holdout_loss,
            prior_penalty=candidate.prior_penalty,
            smoothness_penalty=candidate.smoothness_penalty,
            isophonemy_distance_from_map=distance,
        )
        for index, (candidate, distance) in enumerate(selected)
    )


def _polish_state_controls(
    anatomy: AnatomyGenome,
    state: AcousticState,
    initial: np.ndarray,
    frequencies: np.ndarray,
    config: EvolutionConfig,
) -> tuple[np.ndarray, float]:
    target = state.envelope_array()

    def objective(control: np.ndarray) -> float:
        modeled = modeled_envelope_bank(
            anatomy,
            np.asarray(control)[None, :],
            frequencies,
            section_count=config.section_count,
        )[0]
        spectral = float(np.mean((modeled - target) ** 2))
        effort = float(
            np.mean(
                np.asarray(control) ** 2
                * np.array([0.75, 1.0, 1.0, 0.85, 1.25])
            )
        )
        return spectral + config.effort_weight * effort

    result = optimize.minimize(
        objective,
        np.asarray(initial, dtype=np.float64),
        method="L-BFGS-B",
        bounds=[(-1.0, 1.0)] * len(CONTROL_NAMES),
        options={"maxiter": 55, "ftol": 2e-9, "gtol": 1e-5},
    )
    controls = np.clip(result.x, -1.0, 1.0)
    modeled = modeled_envelope_bank(
        anatomy,
        controls[None, :],
        frequencies,
        section_count=config.section_count,
    )[0]
    spectral_loss = float(np.mean((modeled - target) ** 2))
    return controls, spectral_loss


def _state_fits(
    champion: _CandidateScore,
    analysis: ReferenceAnalysis,
    control_bank: np.ndarray,
    config: EvolutionConfig,
) -> tuple[StateFit, ...]:
    frequencies = np.asarray(analysis.frequency_grid_hz)
    fits: list[StateFit] = []
    for state, control_index in zip(
        analysis.states, champion.chosen_control_indices, strict=True
    ):
        initial = control_bank[control_index]
        controls, spectral_loss = _polish_state_controls(
            champion.anatomy, state, initial, frequencies, config
        )
        areas = area_function_cm2(
            champion.anatomy, controls, config.section_count
        )
        formants = modeled_formants(
            champion.anatomy,
            controls,
            frequencies,
            section_count=config.section_count,
        )
        fits.append(
            StateFit(
                state_id=state.state_id,
                controls=tuple(float(value) for value in controls),
                spectral_loss=spectral_loss,
                target_formants_hz=state.formants_hz,
                modeled_formants_hz=formants,
                area_function_cm2=tuple(float(value) for value in areas),
            )
        )
    return tuple(fits)


def _identifiability_diagnostics(
    fit: SpeakerThroatFit | None,
    champion: _CandidateScore,
    objective: _AnatomyObjective,
) -> tuple[int, tuple[str, ...]]:
    del fit  # Reserved for future posterior-aware diagnostics.
    unit = np.asarray(champion.unit_genome, dtype=np.float64)
    baseline = champion.objective_loss
    steps = np.array([0.012, 0.018, 0.018, 0.018, 0.018, 0.018, 0.018, 0.018, 0.018, 0.015])
    curvatures: list[float] = []
    for index, step in enumerate(steps):
        plus = unit.copy()
        minus = unit.copy()
        plus[index] = min(1.0, plus[index] + step)
        minus[index] = max(0.0, minus[index] - step)
        effective_step = 0.5 * (plus[index] - minus[index])
        if effective_step <= 1e-8:
            curvatures.append(0.0)
            continue
        curvature = (
            objective(plus).objective_loss
            - 2.0 * baseline
            + objective(minus).objective_loss
        ) / (effective_step**2)
        curvatures.append(max(0.0, float(curvature)))
    values = np.asarray(curvatures)
    if np.all(values <= 1e-12):
        return 0, tuple(ANATOMY_PARAMETER_NAMES)
    threshold = max(float(np.max(values)) * 0.025, float(np.median(values)) * 0.18)
    identifiable = values > threshold
    rank = int(np.count_nonzero(identifiable))
    weak = tuple(
        name
        for name, strong in zip(ANATOMY_PARAMETER_NAMES, identifiable, strict=True)
        if not strong
    )
    return rank, weak


def fit_speaker_throat(
    audio: np.ndarray,
    sample_rate: int,
    *,
    reference_name: str = "reference.wav",
    reference_sha256: str | None = None,
    consent_attestation: str | None = None,
    config: EvolutionConfig | None = None,
    progress: ProgressCallback | None = None,
) -> SpeakerThroatFit:
    """Fit a speaker-conditioned throat posterior from one reference clip."""

    settings = config or EvolutionConfig()
    settings.validate()
    mono = _as_mono(audio)
    digest = reference_sha256 or sha256(
        np.asarray(mono, dtype="<f8").tobytes() + str(sample_rate).encode("ascii")
    ).hexdigest()
    analysis = analyze_reference_audio(mono, sample_rate, settings)
    control_bank = build_control_bank(settings.control_bank_size, settings.seed ^ 0xC071)
    objective = _AnatomyObjective(analysis, settings, control_bank)
    ranked = _evolve_anatomies(objective, settings, progress)
    if not ranked:
        raise RuntimeError("anatomy evolution produced no candidates")
    champion = ranked[0]
    posterior = _deduplicated_posterior(
        ranked, analysis, settings, control_bank
    )
    state_fits = _state_fits(champion, analysis, control_bank, settings)
    rank, weak = _identifiability_diagnostics(None, champion, objective)

    diagnostics: list[str] = []
    if analysis.duration_seconds < 3.0:
        diagnostics.append(
            "Reference shorter than 3 s; posterior uncertainty is likely understated."
        )
    if len(analysis.holdout_state_ids) < 2:
        diagnostics.append(
            "Only one held-out acoustic state was available; cross-content evidence is weak."
        )
    if rank < 5:
        diagnostics.append(
            "Fewer than half of anatomy directions are locally identified by this sample."
        )
    if consent_attestation is None:
        diagnostics.append(
            "No consent attestation was recorded; model should not be distributed as a voice identity artifact."
        )
    if champion.holdout_loss > 1.35 * champion.training_loss:
        diagnostics.append(
            "Held-out loss is materially above training loss; the inferred throat may be over-specialized."
        )

    return SpeakerThroatFit(
        reference_sha256=digest,
        reference_name=reference_name,
        consent_attestation=consent_attestation,
        config=settings,
        analysis=analysis,
        map_anatomy=champion.anatomy,
        posterior=posterior,
        state_fits=state_fits,
        best_training_loss=champion.training_loss,
        best_holdout_loss=champion.holdout_loss,
        identifiability_rank=rank,
        weak_anatomy_directions=weak,
        diagnostics=tuple(diagnostics),
    )


# ---------------------------------------------------------------------------
# Diagnostic rendering (not the production glottal-source law)


def _minimum_phase_response(magnitude: np.ndarray) -> np.ndarray:
    magnitude = np.asarray(magnitude, dtype=np.float64)
    if magnitude.ndim != 1 or magnitude.size < 8:
        raise ValueError("magnitude response must be a vector")
    full_magnitude = np.concatenate([magnitude, magnitude[-2:0:-1]])
    log_magnitude = np.log(np.maximum(full_magnitude, 1e-10))
    cepstrum = np.fft.ifft(log_magnitude).real
    minimum_phase_cepstrum = np.zeros_like(cepstrum)
    minimum_phase_cepstrum[0] = cepstrum[0]
    half = cepstrum.size // 2
    minimum_phase_cepstrum[1:half] = 2.0 * cepstrum[1:half]
    if cepstrum.size % 2 == 0:
        minimum_phase_cepstrum[half] = cepstrum[half]
    response = np.exp(np.fft.fft(minimum_phase_cepstrum))
    return response[: magnitude.size]


def _diagnostic_glottal_source(
    duration_seconds: float,
    sample_rate: int,
    f0_hz: float,
    tilt_db_per_octave: float,
    aspiration: float,
    rng: np.random.Generator,
) -> np.ndarray:
    sample_count = max(1, int(round(duration_seconds * sample_rate)))
    phase = np.cumsum(np.full(sample_count, f0_hz / sample_rate)) % 1.0
    # Smooth Rosenberg-like flow pulse, differentiated to approximate radiated
    # pressure. This is explicitly diagnostic; G3's physical fold source must
    # replace it in production.
    open_quotient = 0.62
    flow = np.zeros(sample_count, dtype=np.float64)
    opening = phase < open_quotient * 0.62
    closing = (phase >= open_quotient * 0.62) & (phase < open_quotient)
    flow[opening] = 0.5 * (
        1.0 - np.cos(math.pi * phase[opening] / (open_quotient * 0.62))
    )
    closing_phase = (
        phase[closing] - open_quotient * 0.62
    ) / (open_quotient * 0.38)
    flow[closing] = np.cos(0.5 * math.pi * closing_phase)
    source = np.diff(flow, prepend=flow[0])
    source += aspiration * 0.04 * rng.standard_normal(sample_count)

    # Apply requested broad source tilt in frequency domain.
    spectrum = np.fft.rfft(source)
    frequencies = np.fft.rfftfreq(sample_count, 1.0 / sample_rate)
    octaves = np.log2(np.maximum(frequencies, 40.0) / 1_000.0)
    gain = 10.0 ** ((tilt_db_per_octave * octaves) / 20.0)
    gain[0] = 0.0
    return np.fft.irfft(spectrum * gain, n=sample_count)


def render_throat_atlas(
    fit: SpeakerThroatFit,
    *,
    sample_rate: int = 48_000,
    state_seconds: float = 0.42,
    gap_seconds: float = 0.09,
    seed: int | None = None,
) -> np.ndarray:
    """Render the fitted representative states as an audible diagnostic atlas.

    This is not an acceptance-grade speaker-cloning render: it uses a declared
    diagnostic pulse source and holds each fitted articulation static. It is
    nevertheless useful for hearing whether the evolved tract has a coherent
    speaker-like timbral family before the Rust glottal/gesture lowering lands.
    """

    if sample_rate < 8_000:
        raise ValueError("sample rate is too low")
    rng = np.random.default_rng(fit.config.seed if seed is None else seed)
    f0 = fit.analysis.median_f0_hz or 140.0
    tilt = fit.analysis.median_source_tilt_db_per_octave or -8.0
    periodicity = fit.analysis.median_periodicity or 0.7
    aspiration = float(np.clip(1.0 - periodicity, 0.02, 0.7))
    state_sample_count = int(round(state_seconds * sample_rate))
    nfft = 1 << int(math.ceil(math.log2(state_sample_count + 4096)))
    frequencies = np.fft.rfftfreq(nfft, 1.0 / sample_rate)
    positive = frequencies.copy()
    positive[0] = max(1.0, frequencies[1] if frequencies.size > 1 else 1.0)
    pieces: list[np.ndarray] = []
    gap = np.zeros(int(round(gap_seconds * sample_rate)), dtype=np.float64)

    for state_fit in fit.state_fits:
        source = _diagnostic_glottal_source(
            state_seconds,
            sample_rate,
            f0,
            tilt,
            aspiration,
            rng,
        )
        response = transfer_response(
            fit.map_anatomy,
            state_fit.controls,
            positive,
            section_count=fit.config.section_count,
        )
        magnitude = np.abs(response)
        magnitude /= max(float(np.max(magnitude)), 1e-12)
        minimum_phase = _minimum_phase_response(magnitude)
        spectrum = np.fft.rfft(source, n=nfft) * minimum_phase
        rendered = np.fft.irfft(spectrum, n=nfft)[:state_sample_count]
        fade = min(int(0.018 * sample_rate), state_sample_count // 4)
        if fade > 1:
            rendered[:fade] *= np.linspace(0.0, 1.0, fade)
            rendered[-fade:] *= np.linspace(1.0, 0.0, fade)
        rendered /= max(float(np.max(np.abs(rendered))), 1e-12)
        pieces.extend([0.72 * rendered, gap])
    if not pieces:
        return np.zeros(0, dtype=np.float32)
    atlas = np.concatenate(pieces)
    return atlas.astype(np.float32)


__all__ = [
    "AIR_DENSITY_KG_M3",
    "ANATOMY_PARAMETER_NAMES",
    "AcousticState",
    "AnatomyGenome",
    "CLAIM_CLASS",
    "CONTROL_NAMES",
    "DEFAULT_ANALYSIS_SR",
    "DEFAULT_SECTION_COUNT",
    "EvolutionConfig",
    "PosteriorMember",
    "ReferenceAnalysis",
    "SCHEMA_ID",
    "SPEED_OF_SOUND_M_S",
    "SpeakerThroatFit",
    "StateFit",
    "analyze_reference_audio",
    "area_function_bank_cm2",
    "area_function_cm2",
    "articulation_basis",
    "build_control_bank",
    "fit_speaker_throat",
    "isophonemy_distance",
    "modeled_envelope_bank",
    "modeled_formants",
    "neutral_log_area",
    "normalize_envelope_bank",
    "reflection_coefficients",
    "render_throat_atlas",
    "transfer_response",
    "transfer_response_bank",
]
