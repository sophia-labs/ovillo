#!/usr/bin/env python
"""La Cantora — speech made to sing by the instrument alone.

The constraint (Vera, 2026-08-11): no foreign synthesis. WORLD/pyworld
would resynthesize the voice — a second instrument — so it is refused
entirely; even its analysis half is replaced by our own ears (numpy
autocorrelation, the aigua stack's approach). The only sound sources in
the render are the raw TTS take (dry, quiet, for consonants — the
canto draft-2 lesson) and the string network.

The science of singing, reassigned to the hilo:
  pitch discipline   -> string tuning + resonant selectivity: the take
                        drives strings tuned to the melody through the
                        bridge; the strings' resonance IS the pitch
                        correction (melody chosen near the take's own
                        register to keep the fight small).
  time discipline    -> the slot machinery binds syllables to the grid,
                        and the STRING RING is the vowel sustain: when
                        the syllable's drive ends before the note does,
                        the string keeps singing its resonance — vowel
                        elongation by physics, not time-stretching.
  spectral discipline-> bodyp=mandolin: its modal table climbs past
                        3 kHz, standing in for the singer's formant.
  vibrato            -> refused; detuned courses give choral shimmer
                        instead. La Cantora sings straight-tone with a
                        chorus in her throat. An aesthetic, declared.

Pipeline: segment the take into syllables (RMS onsets) -> per-syllable
F0 (autocorrelation) -> choose tonic near the take's median F0 ->
compose a modal melody bound to a 3/4 grid, phrase breaks where the
speaker breathed -> emit the score (vo weave-sung + vd dry words) ->
render -> measure sung-ness receipts (per-note F0 deviation on the vo
stem, vowel-ring fraction).

Run: .venv/bin/python mus-rs/tools/gen_cantora.py [--take WAV] [--out DIR]
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
SR = 48000
NOTE_NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]


def note_name(midi: int) -> str:
    return f"{NOTE_NAMES[midi % 12]}{midi // 12 - 1}"


def autocorr_f0(seg: np.ndarray, sr: int, lo=70.0, hi=420.0) -> float:
    if len(seg) < int(sr / lo) * 2:
        return 0.0
    seg = seg - seg.mean()
    n = len(seg)
    corr = np.correlate(seg, seg, mode="full")[n - 1:]
    lo_lag, hi_lag = int(sr / hi), int(sr / lo)
    if hi_lag >= n:
        return 0.0
    window = corr[lo_lag:hi_lag]
    if len(window) == 0 or corr[0] <= 0:
        return 0.0
    lag = lo_lag + int(np.argmax(window))
    if corr[lag] / corr[0] < 0.25:
        return 0.0
    return sr / lag


def segment_syllables(audio: np.ndarray, sr: int):
    hop = int(0.02 * sr)
    frames = len(audio) // hop
    rms = np.array([
        np.sqrt((audio[i * hop:(i + 1) * hop] ** 2).mean()) for i in range(frames)
    ])
    smooth = np.convolve(rms, np.hanning(5) / np.hanning(5).sum(), mode="same")
    floor = np.percentile(smooth, 20)
    # adaptive threshold: aim for 14..40 syllables
    for mult in (3.0, 2.4, 1.9, 1.5, 1.2):
        thresh = floor * mult
        active = smooth > thresh
        segments = []
        start = None
        for i, a in enumerate(active):
            if a and start is None:
                start = i
            elif not a and start is not None:
                segments.append((start * hop, i * hop))
                start = None
        if start is not None:
            segments.append((start * hop, frames * hop))
        merged = []
        for s, e in segments:
            if merged and s - merged[-1][1] < int(0.09 * sr):
                merged[-1] = (merged[-1][0], e)
            else:
                merged.append((s, e))
        merged = [(s, e) for s, e in merged if e - s > int(0.08 * sr)]
        if len(merged) >= 8:
            break
    # split long regions at internal RMS valleys: syllable nuclei are
    # separated by dips even when energy never reaches the floor
    out = []
    for s, e in merged:
        if e - s < int(0.45 * sr):
            out.append((s, e))
            continue
        fs, fe = s // hop, e // hop
        region = smooth[fs:fe]
        med = np.median(region)
        cut_frames = []
        for i in range(2, len(region) - 2):
            if (region[i] < 0.65 * med
                    and region[i] <= region[i - 1] and region[i] <= region[i + 1]
                    and (not cut_frames or i - cut_frames[-1] > int(0.12 * sr / hop))):
                cut_frames.append(i)
        prev = s
        for c in cut_frames:
            cut = (fs + c) * hop
            if cut - prev > int(0.09 * sr):
                out.append((prev, cut))
                prev = cut
        if e - prev > int(0.09 * sr):
            out.append((prev, e))
    return out


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--take", type=Path, default=ROOT / "aigua" / "tts_hilo_canto.wav")
    parser.add_argument("--out", type=Path,
                        default=ROOT / "paper" / "ariadne-protected-nonabelian" / "cantora")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    audio, sr = sf.read(args.take)
    if audio.ndim > 1:
        audio = audio.mean(axis=1)

    syllables = segment_syllables(audio, sr)
    f0s = []
    for s, e in syllables:
        mid = audio[s + (e - s) // 5: e - (e - s) // 5]
        f0s.append(autocorr_f0(mid, sr))
    voiced = [f for f in f0s if f > 0]
    median_f0 = float(np.median(voiced)) if voiced else 200.0
    tonic = int(round(69 + 12 * np.log2(median_f0 / 440.0)))
    print(f"take: {len(audio)/sr:.1f}s, {len(syllables)} syllables, "
          f"median F0 {median_f0:.1f} Hz -> tonic {note_name(tonic)}")

    # dorian mode around the tonic, melody within +/-5 st of the register
    mode = [0, 2, 3, 5, 7, 9, 10]
    rng = np.random.default_rng(0xCA27013A)

    # phrase breaks where the speaker breathed (gap > 0.25 s)
    phrases = [[]]
    for k, (s, e) in enumerate(syllables):
        if k and s - syllables[k - 1][1] > int(0.25 * sr):
            phrases.append([])
        phrases[-1].append(k)

    # melody: per phrase, an arc — start low, peak ~2/3 through, cadence
    melody = {}
    for phrase in phrases:
        n = len(phrase)
        peak_at = max(1, int(n * 0.62))
        degree = 0
        for i, k in enumerate(phrase):
            if i == 0:
                degree = int(rng.choice([0, 2, 4]))
            elif i < peak_at:
                degree = min(degree + int(rng.choice([0, 1, 1, 2])), 6)
            elif i == n - 1:
                degree = int(rng.choice([0, 1]))
            else:
                degree = max(degree - int(rng.choice([0, 1, 1])), 0)
            octave = 12 if degree > 4 and rng.random() < 0.15 else 0
            melody[k] = tonic + mode[degree] + (octave - 12 if octave else 0) * 0
    # durations from syllable lengths: short->e, long->q, phrase-final->q.
    events = []  # (ql_duration, syllable index or None for rest)
    for pi, phrase in enumerate(phrases):
        for i, k in enumerate(phrase):
            dur_s = (syllables[k][1] - syllables[k][0]) / sr
            if i == len(phrase) - 1:
                ql = 1.5
            elif dur_s > 0.28:
                ql = 1.0
            else:
                ql = 0.5
            events.append((ql, k))
        events.append((1.0 if pi % 2 == 0 else 1.5, None))  # the breath

    # bind to 3/4 bars
    bars = [[]]
    used = 0.0
    for ql, k in events:
        while ql > 0:
            room = 3.0 - used
            take_ql = min(ql, room)
            bars[-1].append((take_ql, k))
            used += take_ql
            ql -= take_ql
            if used >= 3.0 - 1e-9:
                bars.append([])
                used = 0.0
    if not bars[-1]:
        bars.pop()
    if bars and sum(q for q, _ in bars[-1]) < 3.0 - 1e-9:
        bars[-1].append((3.0 - sum(q for q, _ in bars[-1]), None))

    def dur_code(ql: float) -> str:
        return {0.5: "e", 1.0: "q", 1.5: "q.", 2.0: "h", 2.5: "h~e", 3.0: "h."}[round(ql, 2)]

    take_rel = args.take.name
    lines = [f"""# score: La Cantora
# summary: the take taught to sing by the strings alone — syllables bound to the grid, vowels sustained by string ring, pitch imposed by resonance, no foreign synthesis
# tempo: 66
# time: 3/4
# key: {note_name(tonic)[:-1]}m
# bars: {len(bars)}
# reverb: 1.6
# master: rms=-15

# text @b1: the constraint: nothing sings but the take and the strings.
# text @b1: the string ring IS the vowel; the mandolin body IS the
# text @b1: singer's formant; detuned courses are the chorus in her throat.

# instruments:
#   vo = la cantora (treble, synth=weave, sample={take_rel}, sing=0.85, bodyp=mandolin, sus=5, damp=0.2, courses=9, dimension=1.7, couple=0.09, detune=10, body=0.75, send=0.25, gain=0)
#   vd = las palabras (treble, sample={take_rel}, atk=0.015, rel=0.06, lpf=6500, send=0.1, gain=-7)
#   bj = el bordon (bass, synth=weave, bodyp=cello, bow=0.35, bow_speed=0.4, sus=6, damp=0.3, courses=5, couple=0.05, body=0.8, send=0.15, gain=-6)
"""]
    for bi, bar in enumerate(bars, start=1):
        vo_toks, vd_toks = [], []
        for ql, k in bar:
            code = dur_code(ql)
            if k is None:
                vo_toks.append(f"R{code}")
                vd_toks.append(f"R{code}")
            else:
                start_s = syllables[k][0] / sr
                midi = melody[k]
                vo_toks.append(f"{note_name(midi)}{code}[sing_at={start_s:.3f}]")
                vd_toks.append(f"X{code}[off={start_s*1000:.0f}]")
        phrase_bass = f"{note_name(tonic - 24)}h." if bi % 4 == 1 else "-"
        lines.append(f"bar {bi}: vo={' '.join(vo_toks)}  vd={' '.join(vd_toks)}  bj={phrase_bass}")

    score_path = args.out / "la_cantora.mus"
    score_path.write_text("\n".join(lines) + "\n")

    check = json.loads(subprocess.run(
        [str(MUS), "check", str(score_path), "--base", str(ROOT / "aigua"), "--json"],
        capture_output=True, text=True).stdout)
    assert check["clean"], check["diagnostics"][:3]

    render = json.loads(subprocess.run(
        [str(MUS), "render", str(score_path), "--base", str(ROOT / "aigua"),
         "-o", str(args.out / "la_cantora.wav"),
         "--stems", str(args.out / "stems"), "--json"],
        capture_output=True, text=True).stdout)
    assert not render["badTokens"], render["badTokens"][:3]

    # ---- sung-ness receipts on the vo stem --------------------------------
    stem, _ = sf.read(args.out / "stems" / "vo.wav")
    ql_s = 60.0 / 66.0
    cursor = 0.0
    deviations, ring_fractions = [], []
    for bar in bars:
        for ql, k in bar:
            if k is not None:
                syl_s = (syllables[k][1] - syllables[k][0]) / sr
                # measure the RING: after the syllable's drive ends, only
                # the string speaks — that is the vowel the instrument
                # sustains, and the pitch law it must answer for.
                ring_start = cursor + min(syl_s + 0.04, ql * ql_s * 0.6)
                seg = stem[int(ring_start * SR): int((cursor + ql * ql_s) * SR)]
                target = 440.0 * 2 ** ((melody[k] - 69) / 12)
                if len(seg) > int(0.12 * SR):
                    # selectivity receipt: energy at the melody pitch vs a
                    # semitone either side (autocorr is unreliable on the
                    # virtual-course halo; Goertzel asks the right question)
                    def goertzel(x, f):
                        w = 2 * np.pi * f / SR
                        c = 2 * np.cos(w)
                        s1 = s2 = 0.0
                        for v in x:
                            s0 = v + c * s1 - s2
                            s2, s1 = s1, s0
                        return s1 * s1 + s2 * s2 - c * s1 * s2
                    on = goertzel(seg, target) + goertzel(seg, 2 * target)
                    off = (goertzel(seg, target * 2 ** (1 / 12))
                           + goertzel(seg, target * 2 ** (-1 / 12))) / 2 + 1e-30
                    deviations.append(10 * np.log10(on / off))
                ring_fractions.append(max(0.0, 1.0 - syl_s / (ql * ql_s)))
            cursor += ql * ql_s
    report = {
        "syllables": len(syllables), "bars": len(bars),
        "tonic": note_name(tonic), "median_take_f0": round(median_f0, 1),
        "notes_measured": len(deviations),
        "median_ring_selectivity_db": round(float(np.median(deviations)), 1) if deviations else None,
        "mean_vowel_ring_fraction": round(float(np.mean(ring_fractions)), 3),
        "peak_dbfs": render["peakDbfs"], "duration": render["durationSeconds"],
    }
    print(json.dumps(report, indent=2))
    (args.out / "receipts.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
