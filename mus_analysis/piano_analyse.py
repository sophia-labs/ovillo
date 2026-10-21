"""Measure an Ariadne piano render (``SPEC-ARIADNE-PIANO.md`` §9, the gates).

Reads what the World emitted (``mus-engine`` example ``piano_world``: the
listener's observation before the engine's bus and master), the strikes it
made (``notes.tsv``) and its receipt, and reports per note:

* the partials it sounds, against the stiff-string law ``f_n = n f0
  sqrt(1 + B n^2)`` with the wire's own B (the receipt's ``keys``), and the
  (f0, B) a least-squares fit of the measured partials returns;
* the fundamental against equal temperament (A4 = 440 Hz), in cents;
* the spectral centroid of the first 0.5 s and the felt's contact time;
* T60 from the decay of the note's level, early (first 10 dB) and overall
  (down to -35 dB or the note's end), and after a release.

    uv run --no-project --with numpy --with scipy --with soundfile python -m mus_analysis.piano_analyse \\
        prefix [--json out.json]

where ``prefix.world.wav``, ``prefix.notes.tsv`` and ``prefix.receipt.json``
exist. Nothing here judges how anything sounds.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
from pathlib import Path

import numpy as np
import soundfile as sf

A4 = 440.0


def et_hz(midi: float) -> float:
    return A4 * 2.0 ** ((midi - 69.0) / 12.0)


def cents(a: float, b: float) -> float:
    return 1200.0 * math.log2(a / b)


def load(prefix: str):
    """Both channels; spectra and levels below add their powers (the two
    listening points are not summed as signals)."""
    x, fs = sf.read(f"{prefix}.world.wav", always_2d=True)
    mono = x
    with open(f"{prefix}.notes.tsv") as handle:
        notes = list(csv.DictReader(handle, delimiter="\t"))
    receipt = json.loads(Path(f"{prefix}.receipt.json").read_text())
    return mono, fs, notes, receipt


def segment_ends(notes: list[dict], total_s: float) -> list[float]:
    onsets = [float(n["onset_s"]) for n in notes]
    ends = []
    for i, onset in enumerate(onsets):
        later = [o for o in onsets[i + 1:] if o > onset + 1e-6]
        ends.append(min(later) if later else total_s)
    return ends


def partials(x, fs, start, stop, f0, b, count, max_hz=12_000.0):
    """Measured frequency and level of partials 1..count over [start, stop)."""
    a, z = int(start * fs), int(stop * fs)
    seg = x[a:z] * np.hanning(max(z - a, 16))[:, None]
    n_fft = 1 << max(20, int(math.ceil(math.log2(len(seg) * 8))))
    spectrum = np.sqrt((np.abs(np.fft.rfft(seg, n_fft, axis=0)) ** 2).sum(axis=1))
    freqs = np.fft.rfftfreq(n_fft, 1.0 / fs)
    top = spectrum.max()
    out = []
    for k in range(1, count + 1):
        predicted = k * f0 * math.sqrt(1.0 + b * k * k)
        if predicted > max_hz:
            break
        half = min(0.03 * predicted, 0.35 * f0)
        window = (freqs > predicted - half) & (freqs < predicted + half)
        if not window.any():
            break
        i = int(np.argmax(np.where(window, spectrum, 0.0)))
        if 0 < i < len(spectrum) - 1:
            y0, y1, y2 = (math.log(spectrum[i + d] + 1e-300) for d in (-1, 0, 1))
            shift = 0.5 * (y0 - y2) / (y0 - 2 * y1 + y2) if (y0 - 2 * y1 + y2) != 0 else 0.0
        else:
            shift = 0.0
        out.append(dict(n=k, predicted_hz=predicted, measured_hz=(i + shift) * fs / n_fft,
                        level_db=20 * math.log10(spectrum[i] / top + 1e-300)))
    return out


def fit_stiff_string(found, floor_db=-45.0):
    """Least squares of (f_n / n)^2 = f0^2 + f0^2 B n^2 over partials above `floor_db`."""
    good = [p for p in found if p["level_db"] > floor_db]
    if len(good) < 3:
        return None
    n = np.array([p["n"] for p in good], float)
    y = (np.array([p["measured_hz"] for p in good]) / n) ** 2
    slope, intercept = np.polyfit(n * n, y, 1)
    f0 = math.sqrt(intercept)
    return dict(f0=f0, b=slope / intercept, used=[int(v) for v in n])


def centroid(x, fs, start, stop, lo=20.0, hi=12_000.0):
    a, z = int(start * fs), int(stop * fs)
    seg = x[a:z] * np.hanning(z - a)[:, None]
    power = (np.abs(np.fft.rfft(seg, axis=0)) ** 2).sum(axis=1)
    freqs = np.fft.rfftfreq(len(seg), 1.0 / fs)
    band = (freqs >= lo) & (freqs <= hi)
    return float((power[band] * freqs[band]).sum() / power[band].sum())


def high_share_db(x, fs, start, stop, edge=1_000.0, hi=12_000.0):
    """Energy above `edge` against all energy in 20 Hz..`hi`, over [start, stop) (dB)."""
    a, z = int(start * fs), int(stop * fs)
    seg = x[a:z] * np.hanning(z - a)[:, None]
    power = (np.abs(np.fft.rfft(seg, axis=0)) ** 2).sum(axis=1)
    freqs = np.fft.rfftfreq(len(seg), 1.0 / fs)
    band = (freqs >= 20.0) & (freqs <= hi)
    return float(10 * math.log10(power[band & (freqs >= edge)].sum() / power[band].sum()))


def level_curve(x, fs, start, stop, hop=0.01):
    step = int(hop * fs)
    a, z = int(start * fs), int(stop * fs)
    frames = [x[i:i + step] for i in range(a, max(a, z - step), step)]
    rms = np.array([math.sqrt(float(np.mean((f * f).sum(axis=-1))) + 1e-30) for f in frames])
    return np.arange(len(rms)) * hop, 20 * np.log10(rms + 1e-30)


def t60(times, db, start_drop=0.0, stop_drop=35.0):
    """T60 from a straight line through the level between `start_drop` and
    `stop_drop` dB below the peak (seconds), and the span it used."""
    peak = int(np.argmax(db))
    ref = db[peak]
    after = np.arange(peak, len(db))
    lo = after[db[after] <= ref - start_drop]
    first = int(lo[0]) if len(lo) else peak
    hi = after[db[after] <= ref - stop_drop]
    last = int(hi[0]) if len(hi) else len(db) - 1
    if last - first < 5:
        return None
    slope, _ = np.polyfit(times[first:last + 1], db[first:last + 1], 1)
    if slope >= 0:
        return None
    return dict(t60_s=-60.0 / slope, span_s=[float(times[first]), float(times[last])],
                dropped_db=float(ref - db[last]))


def analyse(prefix: str) -> dict:
    x, fs, notes, receipt = load(prefix)
    keys = {k["key"] + 20: k for k in receipt["keys"]}
    total = len(x) / fs
    ends = segment_ends(notes, total)
    rows = []
    for note, end in zip(notes, ends):
        midi = int(note["midi"])
        key = keys[midi]
        onset, release = float(note["onset_s"]), float(note["release_s"])
        f0_law, b_law = key["fundamentalHz"], key["inharmonicity"]
        start = onset + 0.03
        stop = min(end, release, onset + 1.2)
        found = partials(x, fs, start, stop, f0_law, b_law, key["retainedPartials"])
        top = [p for p in found if p["level_db"] > -45.0]
        fit = fit_stiff_string(found)
        law_dev = [cents(p["measured_hz"], p["predicted_hz"]) for p in top]
        fit_dev = ([cents(p["measured_hz"], p["n"] * fit["f0"] * math.sqrt(1 + fit["b"] * p["n"] ** 2)) for p in top]
                   if fit else [])
        times, db = level_curve(x, fs, onset, min(end, release))
        early = t60(times, db, 0.0, 10.0)
        overall = t60(times, db, 0.0, 35.0)
        after = None
        if release < end - 0.1:
            rt, rdb = level_curve(x, fs, release + 0.02, min(end, release + 1.0))
            if len(rdb) > 8:
                slope, _ = np.polyfit(rt[: min(len(rt), 30)], rdb[: min(len(rdb), 30)], 1)
                after = dict(t60_s=-60.0 / slope if slope < 0 else None,
                             dropped_db_in_0_3_s=float(rdb[0] - rdb[min(len(rdb) - 1, 30)]))
        first = found[0] if found else None
        rows.append(dict(
            onset_s=onset, midi=midi, name=key["name"], velocity_m_s=float(note["velocity_m_s"]),
            contact_ms=float(note["contact_ms"]), peak_force_n=float(note["peak_force_n"]),
            strings=key["strings"], b_law=b_law, f0_law_hz=f0_law,
            partials_used=len(top), partial_law_max_abs_cents=max((abs(v) for v in law_dev), default=None),
            partial_law_median_abs_cents=float(np.median(np.abs(law_dev))) if law_dev else None,
            fit_f0_hz=fit["f0"] if fit else None, fit_b=fit["b"] if fit else None,
            fit_max_abs_cents=max((abs(v) for v in fit_dev), default=None),
            partial1_hz=first["measured_hz"] if first else None,
            partial1_vs_et_cents=cents(first["measured_hz"], et_hz(midi)) if first else None,
            fit_f0_vs_et_cents=cents(fit["f0"], et_hz(midi)) if fit else None,
            centroid_hz=centroid(x, fs, onset, min(end, onset + 0.5)),
            above_1khz_db=high_share_db(x, fs, onset, min(end, onset + 0.5)),
            t60_early_s=early["t60_s"] if early else None,
            t60_s=overall["t60_s"] if overall else None,
            t60_span_s=overall["span_s"] if overall else None,
            released=after,
            partials=found,
        ))
    return dict(prefix=prefix, receipt=dict(
        maximum_frame_residual=receipt["maximumFrameResidual"],
        cumulative_closure_residual=receipt["cumulativeClosureResidual"],
        source_work=receipt["sourceWork"], radiation=receipt["radiation"],
        board_radiation=receipt["boardRadiation"], string_radiation=receipt["stringRadiation"],
        wall_seconds=receipt.get("wallSeconds"), simulated_seconds=receipt["simulatedSeconds"]), notes=rows)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("prefix")
    ap.add_argument("--json", default=None)
    args = ap.parse_args(argv)
    result = analyse(args.prefix)
    r = result["receipt"]
    print(f"ledger: max frame {r['maximum_frame_residual']:.3e} J, cumulative {r['cumulative_closure_residual']:.3e} J; "
          f"radiated {r['radiation']:.3e} J of {r['source_work']:.3e} J (board {r['board_radiation']:.3e}, strings {r['string_radiation']:.3e})")
    print("note  v(m/s) contact  partials  law|c|max  fitB/lawB  p1-ET(c)  fitf0-ET(c)  centroid  >1kHz  T60early  T60   released")
    for n in result["notes"]:
        fmt = lambda v, f: (f.format(v) if v is not None else "-")
        ratio = n["fit_b"] / n["b_law"] if n["fit_b"] is not None and n["b_law"] > 0 else None
        rel = n["released"]["dropped_db_in_0_3_s"] if n["released"] else None
        print(f"{n['name']:4s} {n['velocity_m_s']:6.2f} {n['contact_ms']:6.2f}ms {n['partials_used']:4d}   "
              f"{fmt(n['partial_law_max_abs_cents'], '{:7.2f}')}   {fmt(ratio, '{:7.3f}')}   "
              f"{fmt(n['partial1_vs_et_cents'], '{:+7.2f}')}    {fmt(n['fit_f0_vs_et_cents'], '{:+7.2f}')}   "
              f"{n['centroid_hz']:7.0f} {n['above_1khz_db']:6.1f}dB  {fmt(n['t60_early_s'], '{:6.2f}')}  {fmt(n['t60_s'], '{:6.2f}')}  "
              f"{fmt(rel, '{:5.1f}dB/0.3s')}")
    if args.json:
        Path(args.json).write_text(json.dumps(result, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
