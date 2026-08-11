#!/usr/bin/env python
"""SPEAK warmup: evolve the instrument toward a TTS 'apple' by log-mel diff.

The deliberately modest loss (Vera, 2026-08-11): no ASR in the loop yet
— just spectral distance to a reference TTS utterance. Stated up front:
mimic-loss buys resemblance, not intelligibility; the engine has no
formant transitions, so no search over these controls can fully close
the gap. The converged RESIDUAL is the experiment's real product — a
number for what rung 1 structurally cannot say, receipt for rung 2.

Machinery: five-segment word template (V1, closure, burst+puff, V2,
lateral), ~25 continuous controls (durations, per-segment vowel-body
blends, bow/breath/pick/gain, global f0/damp/sus), rendered through
renderWeaveProbe, scored by 64-band log-mel L2 against the target,
optimized by a seeded (mu, lambda) evolution strategy in plain numpy.

Run: .venv/bin/python mus-rs/tools/speak_mimic.py [--gens N]
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

import numpy as np
import soundfile as sf

ROOT = Path(__file__).resolve().parents[2]
MUS = ROOT / "mus-rs/target/release/mus"
OUT = ROOT / "paper" / "ariadne-protected-nonabelian" / "garganta-humana"
SR = 48000
WORKERS = 10

import threading

_tls = threading.local()


def _get_service():
    if not hasattr(_tls, "svc"):
        _tls.svc = subprocess.Popen(
            [str(MUS), "service"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            text=True,
        )
        _tls.rpc = 0
    return _tls.svc


def call(method, params):
    svc = _get_service()
    _tls.rpc += 1
    svc.stdin.write(json.dumps({"id": _tls.rpc, "method": method, "params": params}) + "\n")
    svc.stdin.flush()
    response = json.loads(svc.stdout.readline())
    if "error" in response:
        raise RuntimeError(response["error"])
    return response["result"]


def probe(patch, midi, slot):
    result = call("renderWeaveProbe", {
        "params": patch, "midis": [midi], "slotSeconds": round(slot, 3), "trajectory": [],
    })
    return np.fromfile(result["pcm"]["path"], dtype=np.float32).astype(np.float64)


# ---- log-mel ----------------------------------------------------------
def mel_filterbank(n_mels=64, n_fft=1024, fmin=60.0, fmax=8000.0):
    def hz_to_mel(f): return 2595 * np.log10(1 + f / 700)
    def mel_to_hz(m): return 700 * (10 ** (m / 2595) - 1)
    mels = np.linspace(hz_to_mel(fmin), hz_to_mel(fmax), n_mels + 2)
    freqs = mel_to_hz(mels)
    bins = np.floor((n_fft + 1) * freqs / SR).astype(int)
    fb = np.zeros((n_mels, n_fft // 2 + 1))
    for m in range(1, n_mels + 1):
        lo, c, hi = bins[m - 1], bins[m], bins[m + 1]
        for k in range(lo, c):
            if c > lo: fb[m - 1, k] = (k - lo) / (c - lo)
        for k in range(c, hi):
            if hi > c: fb[m - 1, k] = (hi - k) / (hi - c)
    return fb


FB = mel_filterbank()
HOP, WIN = 480, 1024  # 10 ms hop, ~21 ms window


def logmel(x, n_frames):
    need = (n_frames - 1) * HOP + WIN
    x = np.pad(x, (0, max(0, need - len(x))))[:need]
    frames = np.lib.stride_tricks.sliding_window_view(x, WIN)[::HOP][:n_frames]
    spec = np.abs(np.fft.rfft(frames * np.hanning(WIN), axis=1)) ** 2
    return np.log10(FB @ spec.T + 1e-9)


# ---- the word template ------------------------------------------------
def sigmoid(x): return 1 / (1 + np.exp(-x))


def render_candidate(v, total_s):
    """v: 25-dim control vector -> audio of duration total_s."""
    d = np.exp(v[0:5])
    d = d / d.sum() * total_s  # segment durations
    f0 = 52 + 10 * sigmoid(v[5])           # midi 52..62
    damp = 0.2 + 0.3 * sigmoid(v[6])
    sus = 1.5 + 4 * sigmoid(v[7])
    def voiced(seg_v, dur, bodies):
        body = bodies[0] if seg_v[0] < 0 else bodies[1]
        patch = {"synth": "weave", "bodyp": body, "bow": float(0.2 + 0.5 * sigmoid(seg_v[1])),
                 "bow_speed": 0.5, "breath": float(0.4 * sigmoid(seg_v[2])),
                 "pick": float(0.15 + 0.5 * sigmoid(seg_v[3])), "sus": float(sus),
                 "damp": float(damp), "courses": 5, "couple": 0.06, "detune": 3,
                 "body": 1.0, "orbit": 0.0}
        return probe(patch, int(round(f0)), max(dur + 0.15, 0.2))[:int(dur * SR)] \
            * float(0.4 + 0.8 * sigmoid(seg_v[4]))
    parts = []
    parts.append(voiced(v[8:13], d[0], ("a", "e")))            # V1
    parts.append(np.zeros(int(d[1] * SR)))                      # closure
    bl = int(max(0.012, min(0.05, d[2] * 0.4)) * SR)            # burst
    burst_patch = {"synth": "weave", "bodyp": "u" if v[13] < 0 else "o", "bow": 0.0,
                   "breath": 0.0, "pick": float(0.6 + 0.35 * sigmoid(v[14])),
                   "sus": 0.4, "damp": 0.6, "courses": 5, "couple": 0.03, "body": 1.0}
    burst = probe(burst_patch, int(round(f0)), 0.15)[:bl] * np.linspace(1, 0.15, bl)
    puff_patch = dict(burst_patch, breath=1.0, pick=0.1)
    puff_len = max(int(d[2] * SR) - bl, int(0.01 * SR))
    puff = probe(puff_patch, 69, 0.12)[:puff_len] * float(0.5 * sigmoid(v[15]))
    parts.append(np.concatenate([burst * float(0.5 + 1.2 * sigmoid(v[16])), puff]))
    parts.append(voiced(v[17:22], d[3], ("o", "e")))            # V2 (schwa-ish)
    tail = voiced(np.append(v[22:25], [0.0, 0.0]), d[4], ("u", "o"))  # lateral
    tail *= np.linspace(1, 0.0, len(tail)) ** 0.8
    parts.append(tail)
    word = np.concatenate(parts)
    peak = np.abs(word).max()
    return word / peak * 0.8 if peak > 0 else word


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--gens", type=int, default=110)
    parser.add_argument("--pop", type=int, default=18)
    args = parser.parse_args()

    target, sr = sf.read(OUT / "apple_target.wav")
    assert sr == SR
    total_s = len(target) / SR
    n_frames = 1 + int((len(target) - WIN) / HOP)
    target_mel = logmel(target, n_frames)

    rng = np.random.default_rng(0x5EA4)
    dim = 25
    mean = np.zeros(dim)
    sigma = np.full(dim, 0.9)
    elite = max(4, args.pop // 4)
    best = (np.inf, None)
    history = []
    improvements = []  # (gen, loss, vector) — the audible fossil record
    from concurrent.futures import ThreadPoolExecutor

    def score(cand):
        audio = render_candidate(cand, total_s)
        return float(np.mean((logmel(audio, n_frames) - target_mel) ** 2))

    pool = ThreadPoolExecutor(max_workers=WORKERS)  # one pool for the whole
    # run: thread-local services live as long as their threads do
    for gen in range(args.gens):
        pop = mean + sigma * rng.standard_normal((args.pop, dim))
        losses = list(pool.map(score, pop))
        order = np.argsort(losses)
        if losses[order[0]] < best[0]:
            best = (losses[order[0]], pop[order[0]].copy())
            improvements.append((gen, best[0], best[1].copy()))
            sf.write(OUT / "apple_best.wav",
                     render_candidate(best[1], total_s).astype(np.float32),
                     SR, subtype="FLOAT")
        sel = pop[order[:elite]]
        mean = 0.7 * mean + 0.3 * sel.mean(axis=0)
        sigma = np.maximum(0.12, 0.55 * sigma + 0.45 * sel.std(axis=0))
        history.append(round(losses[order[0]], 5))
        if gen % 10 == 0:
            print(f"gen {gen}: best-this-gen {losses[order[0]]:.4f}  all-time {best[0]:.4f}",
                  flush=True)

    # ---- the change-over-time recording: every improvement, in order,
    # then the target itself as the final word — hear the system's
    # voicing move toward (and stop short of) speech.
    gap = np.zeros(int(0.4 * SR))
    montage = []
    snapshots = improvements[-30:] if len(improvements) > 30 else improvements
    for gen, loss, vec in snapshots:
        montage.extend([render_candidate(vec, total_s), gap])
    montage.append(np.zeros(int(0.5 * SR)))
    montage.append(target * 0.8)
    evo = np.concatenate(montage)
    sf.write(OUT / "apple_evolution.wav", evo.astype(np.float32), SR, subtype="FLOAT")
    print(f"evolution montage: {len(snapshots)} champions + target, "
          f"{len(evo)/SR:.1f}s", flush=True)

    hand_loss = None
    report = {
        "target_seconds": round(total_s, 3), "generations": args.gens,
        "population": args.pop, "renders": args.gens * args.pop,
        "final_best_logmel_mse": round(best[0], 5),
        "first_gen_best": history[0], "loss_curve_every10": history[::10],
        "residual_note": "the floor this search cannot pass is the rung-1 gap",
    }
    (OUT / "speak_mimic_report.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    

if __name__ == "__main__":
    main()
