#!/usr/bin/env python
"""P0 — diagnostic-renderer validity battery (TWO-MACHINE-PLATEAU.md §6).

Six tests; one receipt. A failure classifies any plateau measured
through this renderer as renderer-limited until repaired.

  T1  flat-transfer identity (fixed OLA vs the old un-normalized OLA)
  T2  static transfer vs direct convolution
  T3  overlap-weight ripple, raw and post-normalization
  T4  absolute attenuation across articulatory states, per gain policy
  T5  cache-quantization audit (exact keys vs the old 1e-3 rounding)
  T6  metric self-calibration: the loss's own scale under benign
      target perturbations (time shift, level shift, lowpass)

Run: .venv/bin/python mus-rs/tools/p0_validate_renderer.py --receipt R
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np
import soundfile as sf

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "mus-rs" / "tools"))

from garganta_render import (  # noqa: E402
    ANCHOR_DIM,
    ANCHORS,
    CONTROL_DIM,
    FILT_FFT,
    GestureRenderer,
    HOP,
    NFFT,
    SR,
    WIN,
    load_fitted,
    neutral_start,
    periodic_hann,
)

TARGET = ROOT / "paper/ariadne-protected-nonabelian/garganta-humana/apple_target.wav"


def logmel_pair(a, b):
    """Same 64-band log-mel MSE as the mimic tools."""
    sys.path.insert(0, str(ROOT / "mus-rs" / "tools"))
    from speak_mimic_unified import logmel  # reuse verbatim

    n_frames = 1 + int((min(len(a), len(b)) - WIN) / HOP)
    return float(np.mean((logmel(a, n_frames) - logmel(b, n_frames)) ** 2))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--out", type=Path,
                        default=ROOT / "mus-rs" / "SPECS" / "garganta" / "p0_renderer_receipt.json")
    args = parser.parse_args()

    fitted = load_fitted(args.receipt)
    renderer = GestureRenderer(
        fitted["anatomy"], fitted["section_count"],
        median_f0=fitted["median_f0"], source_tilt=fitted["tilt"],
        normalize="neutral_reference",
    )
    rng = np.random.default_rng(0xB0)
    receipt: dict = {"battery": "P0 renderer validity", "date": "2026-08-11"}

    # T1 — flat-transfer identity: bypass the tract, OLA must reconstruct
    total_s = 0.5
    genome = neutral_start()
    source = renderer.source(genome.reshape(-1), total_s, np.random.default_rng(1))
    flat = np.ones(NFFT // 2 + 1)
    window = periodic_hann(WIN)
    legacy_window = np.hanning(WIN)
    n = len(source)

    def ola(signal, use_window, hop, normalize):
        out = np.zeros(n + NFFT)
        weight = np.zeros(n + NFFT)
        for start in range(0, n, hop):
            frame = signal[start:start + WIN]
            if len(frame) < WIN:
                frame = np.pad(frame, (0, WIN - len(frame)))
            filtered = np.fft.irfft(
                np.fft.rfft(frame * use_window, n=NFFT) * flat, n=NFFT
            )
            out[start:start + NFFT] += filtered
            weight[start:start + WIN] += use_window
        out = out[:n]
        if normalize:
            out = out / np.maximum(weight[:n], 1e-6)
        return out

    interior = slice(WIN, n - WIN)
    fixed = ola(source, window, HOP, True)
    legacy = ola(source, legacy_window, 480, False)
    denom = float(np.linalg.norm(source[interior]))
    receipt["t1_flat_identity"] = {
        "fixed_relative_error": float(np.linalg.norm((fixed - source)[interior]) / denom),
        "legacy_relative_error": float(np.linalg.norm((legacy - source)[interior]) / denom),
        "pass": bool(np.linalg.norm((fixed - source)[interior]) / denom < 1e-10),
    }

    # T2 — static transfer vs direct convolution
    static_controls = np.array([0.4, 0.2, -0.3, 0.1, 0.0])
    response = renderer.response_for(static_controls)
    impulse = np.fft.irfft(response, n=FILT_FFT)[:NFFT]
    direct = np.convolve(source, impulse)[:n]
    genome_static = neutral_start()
    framed = renderer.render(
        np.tile(np.concatenate([static_controls, genome_static[0, CONTROL_DIM:]]), ANCHORS),
        total_s, np.random.default_rng(1), peak_normalize=False,
    )
    # Steady-state comparison: the first WIN + NFFT samples are the
    # declared startup transient (frames before t=0 do not exist, so
    # early filter tails are underfed relative to full convolution).
    # Beyond it, frame-wise filtering IS direct convolution.
    steady = slice(WIN + NFFT, n - WIN - NFFT)
    a, b = framed[steady], direct[steady]
    scalar = float(np.dot(a, b) / max(np.dot(b, b), 1e-12))
    steady_residual = float(
        np.linalg.norm(a - scalar * b) / max(np.linalg.norm(a), 1e-12)
    )
    edge = slice(0, WIN + NFFT)
    receipt["t2_static_vs_convolution"] = {
        "best_scalar": scalar,
        "steady_state_relative_residual": steady_residual,
        "declared_edge_transient_samples": WIN + NFFT,
        "declared_edge_transient_ms": round(1000.0 * (WIN + NFFT) / SR, 2),
        "edge_relative_residual": float(
            np.linalg.norm((framed - direct)[edge])
            / max(np.linalg.norm(direct[edge]), 1e-12)
        ),
        "pass": bool(steady_residual < 1e-9),
    }

    # T3 — overlap-weight ripple: legacy grid vs the COLA grid
    def ripple(use_window, hop):
        weight = np.zeros(n + NFFT)
        for start in range(0, n, hop):
            weight[start:start + WIN] += use_window
        w_interior = weight[WIN:n - WIN]
        return float(w_interior.max() / w_interior.min())

    receipt["t3_ola_weight"] = {
        "legacy_ripple_max_over_min": ripple(legacy_window, 480),
        "cola_ripple_max_over_min": ripple(window, HOP),
        "pass": bool(ripple(window, HOP) < 1.0 + 1e-12),
    }

    # T4 — absolute attenuation per gain policy
    open_state = np.array([0.8, 0.2, 0.0, -0.2, 0.0])       # open vowel-ish
    closed_state = np.array([-1.0, 0.6, 0.8, 0.9, 1.0])     # severe constriction
    policies = {}
    for policy in ("per_state", "neutral_reference", "physical"):
        r = GestureRenderer(
            fitted["anatomy"], fitted["section_count"],
            median_f0=fitted["median_f0"], source_tilt=fitted["tilt"],
            normalize=policy,
        )
        open_gain = float(np.abs(np.fft.irfft(r.response_for(open_state), n=NFFT)).sum())
        closed_gain = float(np.abs(np.fft.irfft(r.response_for(closed_state), n=NFFT)).sum())
        policies[policy] = {
            "open_l1": open_gain,
            "closed_l1": closed_gain,
            "closed_over_open": closed_gain / max(open_gain, 1e-12),
        }
    receipt["t4_absolute_attenuation"] = {
        **policies,
        "confound_confirmed": bool(
            abs(policies["per_state"]["closed_over_open"]
                - policies["physical"]["closed_over_open"]) > 0.05
        ),
    }

    # T5 — cache-quantization audit
    r = renderer
    near_a = np.array([0.1000, 0.2, 0.3, -0.1, 0.0])
    near_b = near_a + 4.0e-4  # collides under the old 1e-3 rounding
    distinct = not np.allclose(r.response_for(near_a), r.response_for(near_b))
    receipt["t5_cache_audit"] = {
        "exact_keys_distinguish_4e-4": bool(distinct),
        "legacy_rounding": "1e-3 (collided; removed)",
        "pass": bool(distinct),
    }

    # T6 — the metric's own scale under benign target perturbations
    target, sr = sf.read(TARGET)
    assert sr == SR
    perturbations = {}
    for name, mutant in {
        "shift_1ms": np.roll(target, int(0.001 * SR)),
        "shift_5ms": np.roll(target, int(0.005 * SR)),
        "level_+1dB": target * 10 ** (1 / 20),
        "level_+3dB": target * 10 ** (3 / 20),
        "lowpass_8k": np.fft.irfft(
            np.fft.rfft(target)
            * (np.fft.rfftfreq(len(target), 1 / SR) < 8000),
            n=len(target),
        ),
    }.items():
        perturbations[name] = round(logmel_pair(mutant, target), 5)
    receipt["t6_metric_calibration"] = {
        "loss_vs_benign_target_perturbations": perturbations,
        "note": "the scale against which ~2.0 aggregate losses must be read",
    }

    args.out.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))


if __name__ == "__main__":
    main()
