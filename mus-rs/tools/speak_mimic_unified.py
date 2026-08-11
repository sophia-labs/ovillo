#!/usr/bin/env python
"""speak_mimic_unified — the gesture machine drives the fitted throat.

The unified-SOTA experiment (Vera, 2026-08-11): last night's mimic
evolved a gesture program on a generic rung-1 instrument (floor: 1.92
log-mel MSE); today the mathematicians' speaker-throat evolution fitted
an ANATOMY to the same reference. This tool closes the bilevel loop —
a (mu, lambda)-ES over a time-varying articulatory program (their five
control dimensions + voicing + aspiration + F0) rendered through their
fitted MAP anatomy via their own transfer-response synthesis, scored by
the SAME 64-band log-mel loss against the same target, so the number is
directly comparable to 1.92.

Warm start: the receipt's state fits are time-stamped along the
reference, so the search begins from the throat fit's own implied
trajectory rather than from noise.

Honest expectations, stated up front: the anatomy match should buy loss
(the search stops spending dimensions faking the resonator), but
intelligibility still hangs on fast within-note articulation — the G1
fast-passage machinery. The diagnostic pulse source is theirs and is
declared diagnostic (G3 replaces it).

Run: .venv/bin/python mus-rs/tools/speak_mimic_unified.py [--gens N]
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

import numpy as np
import soundfile as sf

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from mus_analysis.garganta_anatomy import (  # noqa: E402
    AnatomyGenome,
    CONTROL_NAMES,
    _minimum_phase_response,
    transfer_response,
)

SR = 48000
HOP, WIN, NFFT = 480, 1024, 2048
ANCHORS = 8
CONTROL_DIM = 5
ANCHOR_DIM = CONTROL_DIM + 3  # + voicing gain, aspiration, f0 offset (semitones)
TARGET = ROOT / "paper/ariadne-protected-nonabelian/garganta-humana/apple_target.wav"


# ---- identical log-mel loss to speak_mimic.py -------------------------
def mel_filterbank(n_mels=64, n_fft=1024, fmin=60.0, fmax=8000.0):
    def hz_to_mel(f):
        return 2595 * np.log10(1 + f / 700)

    def mel_to_hz(m):
        return 700 * (10 ** (m / 2595) - 1)

    mels = np.linspace(hz_to_mel(fmin), hz_to_mel(fmax), n_mels + 2)
    freqs = mel_to_hz(mels)
    bins = np.floor((n_fft + 1) * freqs / SR).astype(int)
    fb = np.zeros((n_mels, n_fft // 2 + 1))
    for m in range(1, n_mels + 1):
        lo, c, hi = bins[m - 1], bins[m], bins[m + 1]
        for k in range(lo, c):
            if c > lo:
                fb[m - 1, k] = (k - lo) / (c - lo)
        for k in range(c, hi):
            if hi > c:
                fb[m - 1, k] = (hi - k) / (hi - c)
    return fb


FB = mel_filterbank()


def logmel(x, n_frames):
    need = (n_frames - 1) * HOP + WIN
    x = np.pad(x, (0, max(0, need - len(x))))[:need]
    frames = np.lib.stride_tricks.sliding_window_view(x, WIN)[::HOP][:n_frames]
    spec = np.abs(np.fft.rfft(frames * np.hanning(WIN), axis=1)) ** 2
    return np.log10(FB @ spec.T + 1e-9)


class FittedThroat:
    """Their MAP anatomy + reference medians, loaded from the receipt."""

    def __init__(self, receipt_path: Path):
        receipt = json.loads(receipt_path.read_text())
        self.anatomy = AnatomyGenome(**receipt["map_anatomy"])
        self.section_count = receipt["config"]["section_count"]
        analysis = receipt["reference_analysis"]
        self.median_f0 = analysis["median_f0_hz"] or 140.0
        self.tilt = analysis["median_source_tilt_db_per_octave"] or -8.0
        states = {state["state_id"]: state for state in analysis["states"]}
        fits = []
        for fit in receipt["state_fits"]:
            state = states[fit["state_id"]]
            controls = [fit["controls"][name] for name in CONTROL_NAMES]
            fits.append((state["time_seconds"], controls,
                         state["periodicity"], state["rms_dbfs"]))
        fits.sort(key=lambda item: item[0])
        self.timeline = fits
        self.freqs = np.fft.rfftfreq(NFFT, 1.0 / SR)
        self.freqs_positive = self.freqs.copy()
        self.freqs_positive[0] = max(1.0, self.freqs[1])
        self._response_cache: dict[bytes, np.ndarray] = {}

    def warm_anchors(self, total_s: float) -> np.ndarray:
        """Initial genome from the fit's own implied trajectory."""
        genome = np.zeros((ANCHORS, ANCHOR_DIM))
        times = [item[0] for item in self.timeline]
        controls = np.array([item[1] for item in self.timeline])
        periodicity = np.array([item[2] for item in self.timeline])
        rms = np.array([item[3] for item in self.timeline])
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

    def response_for(self, controls: np.ndarray) -> np.ndarray:
        key = np.round(controls, 3).tobytes()
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
        magnitude /= max(float(np.max(magnitude)), 1e-12)
        minimum_phase = _minimum_phase_response(magnitude)
        self._response_cache[key] = minimum_phase
        return minimum_phase

    def render(self, genome: np.ndarray, total_s: float, rng: np.random.Generator) -> np.ndarray:
        genome = genome.reshape(ANCHORS, ANCHOR_DIM)
        sample_count = int(total_s * SR)
        t = np.arange(sample_count) / SR
        anchor_times = np.linspace(0.0, total_s, ANCHORS)

        def track(column, lo, hi):
            return np.clip(np.interp(t, anchor_times, genome[:, column]), lo, hi)

        gain = track(CONTROL_DIM, 0.0, 1.2)
        aspiration = track(CONTROL_DIM + 1, 0.0, 1.0)
        f0 = self.median_f0 * 2.0 ** (track(CONTROL_DIM + 2, -12.0, 12.0) / 12.0)

        # voiced Rosenberg-like pulse train at the commanded f0 track
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

        # broad source tilt, matched to the reference median
        spectrum = np.fft.rfft(source)
        octaves = np.log2(np.maximum(np.fft.rfftfreq(sample_count, 1.0 / SR), 40.0) / 1000.0)
        spectrum *= 10.0 ** ((self.tilt * octaves) / 20.0)
        source = np.fft.irfft(spectrum, n=sample_count)

        # frame-wise articulation: interpolate controls, filter, overlap-add
        window = np.hanning(WIN)
        out = np.zeros(sample_count + NFFT)
        for start in range(0, sample_count, HOP):
            center = (start + WIN / 2) / SR
            # their area cone: controls live in [-1, 1]; the search may
            # propose outside, the render clamps at the lawful boundary
            controls = np.clip(
                np.array([
                    np.interp(center, anchor_times, genome[:, column])
                    for column in range(CONTROL_DIM)
                ]),
                -1.0,
                1.0,
            )
            frame = source[start:start + WIN]
            if len(frame) < WIN:
                frame = np.pad(frame, (0, WIN - len(frame)))
            filtered = np.fft.irfft(
                np.fft.rfft(frame * window, n=NFFT) * self.response_for(controls),
                n=NFFT,
            )
            out[start:start + NFFT] += filtered
        out = out[:sample_count]
        peak = np.abs(out).max()
        return out / peak * 0.8 if peak > 0 else out


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--gens", type=int, default=140)
    parser.add_argument("--pop", type=int, default=24)
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--out", type=Path,
                        default=Path.home() / "Desktop" / "mus-listening" / "garganta-evolved")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    target, sr = sf.read(TARGET)
    assert sr == SR
    total_s = len(target) / SR
    n_frames = 1 + int((len(target) - WIN) / HOP)
    target_mel = logmel(target, n_frames)

    throat = FittedThroat(args.receipt)
    rng = np.random.default_rng(0xA9B1E)
    render_rng = np.random.default_rng(0xF00D)  # fixed source noise per render

    dim = ANCHORS * ANCHOR_DIM
    mean = throat.warm_anchors(total_s).reshape(-1)
    sigma = np.full(dim, 0.25)
    elite = max(4, args.pop // 4)
    best = (np.inf, mean.copy())
    history = []
    improvements = []

    def score(candidate):
        audio = throat.render(candidate, total_s, np.random.default_rng(0xF00D))
        return float(np.mean((logmel(audio, n_frames) - target_mel) ** 2))

    warm_loss = score(mean)
    print(f"warm start (their trajectory, un-refined): {warm_loss:.4f}", flush=True)
    best = (warm_loss, mean.copy())
    improvements.append((-1, warm_loss, mean.copy()))

    for gen in range(args.gens):
        population = mean + sigma * rng.standard_normal((args.pop, dim))
        losses = [score(candidate) for candidate in population]
        order = np.argsort(losses)
        if losses[order[0]] < best[0]:
            best = (losses[order[0]], population[order[0]].copy())
            improvements.append((gen, best[0], best[1].copy()))
            sf.write(args.out / "unified_apple_best.wav",
                     throat.render(best[1], total_s, np.random.default_rng(0xF00D))
                     .astype(np.float32), SR, subtype="FLOAT")
        selected = population[order[:elite]]
        mean = 0.7 * mean + 0.3 * selected.mean(axis=0)
        sigma = np.maximum(0.03, 0.55 * sigma + 0.45 * selected.std(axis=0))
        history.append(round(losses[order[0]], 5))
        if gen % 10 == 0:
            print(f"gen {gen}: best-this-gen {losses[order[0]]:.4f}  "
                  f"all-time {best[0]:.4f}", flush=True)

    gap = np.zeros(int(0.4 * SR))
    montage = []
    for _, _, vector in improvements[-24:]:
        montage.extend([throat.render(vector, total_s, np.random.default_rng(0xF00D)), gap])
    montage.append(np.zeros(int(0.5 * SR)))
    montage.append(target * 0.8)
    evolution = np.concatenate(montage)
    sf.write(args.out / "unified_apple_evolution.wav", evolution.astype(np.float32),
             SR, subtype="FLOAT")

    report = {
        "warm_start_loss": round(warm_loss, 5),
        "final_best_logmel_mse": round(best[0], 5),
        "rung1_floor_same_loss_same_target": 1.9204,
        "generations": args.gens,
        "population": args.pop,
        "renders": args.gens * args.pop + 1,
        "loss_curve_every10": history[::10],
        "note": "gesture ES over the mathematicians' fitted MAP throat; "
                "diagnostic pulse source (G3 pending); comparability: "
                "identical mel loss and target as speak_mimic.py",
    }
    (args.out / "unified_apple_report.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2), flush=True)


if __name__ == "__main__":
    main()
