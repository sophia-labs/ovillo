#!/usr/bin/env python
"""The diagnostic gesture renderer, P0-repaired.

Extracted from `speak_mimic_unified.py` after the mathematicians'
review (`TWO-MACHINE-PLATEAU.md` §4.4) confessed and confirmed its
confounds. The fixes, each behind a declared policy rather than a
silent change:

1. **Overlap-add normalization.** The renderer accumulates the shifted
   analysis windows alongside the signal and divides by the weight
   buffer (floored at 1e-6). Flat transfer now reconstructs its source
   exactly over the interior; the 1024-Hann/480-hop ripple is gone.
2. **Transfer-gain policy** (`normalize`): `"neutral_reference"`
   (default) scales every state's response by ONE constant — the peak
   of this anatomy's neutral-state response — preserving relative
   attenuation between states; `"physical"` applies no normalization;
   `"per_state"` reproduces the old confounded behavior for
   comparisons and is labeled as such.
3. **Exact cache keys.** Responses are cached under the controls'
   exact float64 bytes; the old 1e-3 rounding quantized the control
   space under the optimizer.

The P0 battery (`p0_validate_renderer.py`) is the acceptance harness;
its receipt rides with any number this renderer produces.
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from mus_analysis.garganta_anatomy import (  # noqa: E402
    AnatomyGenome,
    CONTROL_NAMES,
    _minimum_phase_response,
    transfer_response,
)

SR = 48000
# Render hop is 512: a periodic Hann of 1024 at half-window hop satisfies
# COLA exactly (shifted windows sum to 1), so static filtering equals
# direct convolution and flat transfer is the identity — P0 T1/T2. The
# LOSS featurization keeps its own independent 480 hop for target
# comparability; the two grids never mix.
HOP, WIN, NFFT = 512, 1024, 2048
# Filtering FFT: a WIN frame convolved with an NFFT-tap minimum-phase IR
# spans WIN + NFFT - 1 samples; filtering at NFFT wraps the tail
# (circular aliasing — P0 T2 caught it at 15%). 4096 holds the full
# linear convolution.
FILT_FFT = 4096
ANCHORS = 8
CONTROL_DIM = len(CONTROL_NAMES)
ANCHOR_DIM = CONTROL_DIM + 3  # + voicing gain, aspiration, f0 offset
OLA_WEIGHT_FLOOR = 1e-6
NEUTRAL_CONTROLS = (0.0,) * CONTROL_DIM


def periodic_hann(length: int) -> np.ndarray:
    """`0.5 - 0.5 cos(2 pi n / N)` — sums to exactly 1 at half-window hop."""
    return 0.5 - 0.5 * np.cos(2.0 * np.pi * np.arange(length) / length)


class GestureRenderer:
    """Time-varying articulation through one anatomy's transfer bank."""

    def __init__(
        self,
        anatomy: AnatomyGenome,
        section_count: int,
        *,
        median_f0: float,
        source_tilt: float,
        normalize: str = "neutral_reference",
    ):
        if normalize not in ("neutral_reference", "physical", "per_state"):
            raise ValueError(f"unknown normalize policy {normalize!r}")
        self.anatomy = anatomy
        self.section_count = section_count
        self.median_f0 = median_f0
        self.tilt = source_tilt
        self.normalize = normalize
        self.freqs = np.fft.rfftfreq(NFFT, 1.0 / SR)
        self.freqs_positive = self.freqs.copy()
        self.freqs_positive[0] = max(1.0, self.freqs[1])
        self._response_cache: dict[bytes, np.ndarray] = {}
        self._neutral_peak: float | None = None

    def neutral_peak(self) -> float:
        if self._neutral_peak is None:
            response = transfer_response(
                self.anatomy,
                NEUTRAL_CONTROLS,
                self.freqs_positive,
                section_count=self.section_count,
            )
            self._neutral_peak = max(float(np.max(np.abs(response))), 1e-12)
        return self._neutral_peak

    def response_for(self, controls: np.ndarray) -> np.ndarray:
        controls = np.clip(np.asarray(controls, dtype=np.float64), -1.0, 1.0)
        key = controls.tobytes()  # exact — no quantization under the optimizer
        cached = self._response_cache.get(key)
        if cached is not None:
            return cached
        response = transfer_response(
            self.anatomy,
            tuple(float(value) for value in controls),
            self.freqs_positive,
            section_count=self.section_count,
        )
        magnitude = np.abs(response)
        if self.normalize == "per_state":
            magnitude = magnitude / max(float(np.max(magnitude)), 1e-12)
        elif self.normalize == "neutral_reference":
            magnitude = magnitude / self.neutral_peak()
        minimum_phase = _minimum_phase_response(magnitude)
        # store the zero-padded filtering spectrum of the NFFT-tap IR so
        # the render loop's convolution is linear, never circular
        impulse = np.fft.irfft(minimum_phase, n=NFFT)
        padded = np.fft.rfft(impulse, n=FILT_FFT)
        self._response_cache[key] = padded
        return padded

    def source(self, genome: np.ndarray, total_s: float,
               rng: np.random.Generator) -> np.ndarray:
        genome = genome.reshape(ANCHORS, ANCHOR_DIM)
        sample_count = int(total_s * SR)
        t = np.arange(sample_count) / SR
        anchor_times = np.linspace(0.0, total_s, ANCHORS)

        def track(column, lo, hi):
            return np.clip(np.interp(t, anchor_times, genome[:, column]), lo, hi)

        gain = track(CONTROL_DIM, 0.0, 1.2)
        aspiration = track(CONTROL_DIM + 1, 0.0, 1.0)
        f0 = self.median_f0 * 2.0 ** (track(CONTROL_DIM + 2, -12.0, 12.0) / 12.0)

        phase = np.cumsum(f0 / SR) % 1.0
        open_q = 0.62
        flow = np.zeros(sample_count)
        opening = phase < open_q * 0.62
        closing = (phase >= open_q * 0.62) & (phase < open_q)
        flow[opening] = 0.5 * (1.0 - np.cos(math.pi * phase[opening] / (open_q * 0.62)))
        closing_phase = (phase[closing] - open_q * 0.62) / (open_q * 0.38)
        flow[closing] = np.cos(0.5 * math.pi * closing_phase)
        voiced = np.diff(flow, prepend=flow[0])
        noise = rng.standard_normal(sample_count)
        source = gain * ((1.0 - aspiration) * voiced + 0.25 * aspiration * noise)

        spectrum = np.fft.rfft(source)
        octaves = np.log2(
            np.maximum(np.fft.rfftfreq(sample_count, 1.0 / SR), 40.0) / 1000.0
        )
        spectrum *= 10.0 ** ((self.tilt * octaves) / 20.0)
        return np.fft.irfft(spectrum, n=sample_count)

    def render(self, genome: np.ndarray, total_s: float,
               rng: np.random.Generator, *, peak_normalize: bool = True) -> np.ndarray:
        genome = genome.reshape(ANCHORS, ANCHOR_DIM)
        sample_count = int(total_s * SR)
        anchor_times = np.linspace(0.0, total_s, ANCHORS)
        source = self.source(genome, total_s, rng)

        window = periodic_hann(WIN)
        out = np.zeros(sample_count + FILT_FFT)
        weight = np.zeros(sample_count + FILT_FFT)
        for start in range(0, sample_count, HOP):
            center = (start + WIN / 2) / SR
            controls = np.array([
                np.interp(center, anchor_times, genome[:, column])
                for column in range(CONTROL_DIM)
            ])
            frame = source[start:start + WIN]
            if len(frame) < WIN:
                frame = np.pad(frame, (0, WIN - len(frame)))
            filtered = np.fft.irfft(
                np.fft.rfft(frame * window, n=FILT_FFT) * self.response_for(controls),
                n=FILT_FFT,
            )
            out[start:start + FILT_FFT] += filtered
            weight[start:start + WIN] += window
        # COLA interior weight is exactly 1; the division only repairs the
        # half-window edges, so filter tails are never locally re-scaled.
        out = out[:sample_count] / np.maximum(weight[:sample_count], OLA_WEIGHT_FLOOR)
        if peak_normalize:
            peak = np.abs(out).max()
            if peak > 0:
                out = out / peak * 0.8
        return out


def load_fitted(receipt_path: Path):
    """Anatomy + reference medians + timestamped warm timeline from a
    speaker-throat receipt."""
    receipt = json.loads(receipt_path.read_text())
    anatomy = AnatomyGenome(**receipt["map_anatomy"])
    section_count = receipt["config"]["section_count"]
    analysis = receipt["reference_analysis"]
    states = {state["state_id"]: state for state in analysis["states"]}
    timeline = []
    for fit in receipt["state_fits"]:
        state = states[fit["state_id"]]
        controls = [fit["controls"][name] for name in CONTROL_NAMES]
        timeline.append((state["time_seconds"], controls,
                         state["periodicity"], state["rms_dbfs"]))
    timeline.sort(key=lambda item: item[0])
    return {
        "anatomy": anatomy,
        "section_count": section_count,
        "median_f0": analysis["median_f0_hz"] or 140.0,
        "tilt": analysis["median_source_tilt_db_per_octave"] or -8.0,
        "timeline": timeline,
        "posterior": receipt["posterior"],
    }


def warm_anchors(timeline, total_s: float) -> np.ndarray:
    genome = np.zeros((ANCHORS, ANCHOR_DIM))
    times = [item[0] for item in timeline]
    controls = np.array([item[1] for item in timeline])
    periodicity = np.array([item[2] for item in timeline])
    rms = np.array([item[3] for item in timeline])
    gain = np.clip((rms + 60.0) / 60.0, 0.0, 1.0)
    anchor_times = np.linspace(0.0, total_s, ANCHORS)
    for column in range(CONTROL_DIM):
        genome[:, column] = np.interp(anchor_times, times, controls[:, column])
    genome[:, CONTROL_DIM] = np.interp(anchor_times, times, gain)
    genome[:, CONTROL_DIM + 1] = np.clip(
        1.0 - np.interp(anchor_times, times, periodicity), 0.05, 0.9
    )
    genome[:, CONTROL_DIM + 2] = 0.0
    return genome


def neutral_start() -> np.ndarray:
    """Common control initialization for paired anatomy comparisons:
    neutral tract, moderate steady voicing, no aspiration, no F0 offset."""
    genome = np.zeros((ANCHORS, ANCHOR_DIM))
    genome[:, CONTROL_DIM] = 0.5
    genome[:, CONTROL_DIM + 1] = 0.15
    return genome
