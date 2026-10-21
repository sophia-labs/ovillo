"""Measure a trombone-World render against what was meant (gates T3, T4).

usage: python -m mus_analysis.trombone_world_analyse <render.wav> <notes.tsv> [--plan profiles/ariadne-trombone-v0.plan.json]

Per note, over its steady part: pitch (autocorrelation, median of 60 ms
windows), cents from the intended MIDI note, the bore resonance nearest the
sounded pitch at the note's slide (the partial that sounded), RMS level and
spectral centroid. Prints a table and a summary; ``--json`` writes both.
Nothing here is a claim about listeners; it measures the render.
"""

from __future__ import annotations

import argparse
import json
import math

import numpy as np
import soundfile as sf

FALL, VIB, SCOOP, RIP = 2, 4, 1, 16


def pitch(x: np.ndarray, sr: float, fmin=25.0, fmax=1500.0):
    """Autocorrelation pitch: the windowed autocorrelation divided by the window's
    own (unbiased), then the first interior local maximum within 85 % of the best
    interior one, refined parabolically. Returns (Hz, clarity)."""
    x = x - x.mean()
    n = len(x)
    if n < 64 or np.sqrt(np.mean(x * x)) < 1e-12:
        return 0.0, 0.0
    w = np.hanning(n)
    nfft = 1 << int(math.ceil(math.log2(2 * n)))
    ac = np.fft.irfft(np.abs(np.fft.rfft(x * w, nfft)) ** 2)[:n]
    aw = np.fft.irfft(np.abs(np.fft.rfft(w, nfft)) ** 2)[:n]
    lo, hi = max(2, int(sr / fmax)), min(int(sr / fmin), n // 2)
    r = ac / (ac[0] + 1e-300) / np.maximum(aw / aw[0], 1e-3)
    peaks = [i for i in range(lo + 1, hi - 1) if r[i] >= r[i - 1] and r[i] >= r[i + 1]]
    if not peaks:
        return 0.0, 0.0
    best = max(r[i] for i in peaks)
    idx = next(i for i in peaks if r[i] >= best - 0.15 * abs(best))
    a, b, c = r[idx - 1], r[idx], r[idx + 1]
    den = a - 2 * b + c
    d = 0.5 * (a - c) / den if den != 0 else 0.0
    return sr / (idx + max(-0.5, min(0.5, d))), float(min(b, 1.0))


def centroid(x: np.ndarray, sr: float, fmax=8000.0) -> float:
    spec = np.abs(np.fft.rfft(x * np.hanning(len(x))))
    f = np.fft.rfftfreq(len(x), 1 / sr)
    keep = (f > 20) & (f < fmax)
    p = spec[keep] ** 2
    return float(np.sum(f[keep] * p) / max(np.sum(p), 1e-300))


def bore_resonances(plan: dict, slide: float) -> np.ndarray:
    if plan.get("schema_version") == "ariadne-world-plan-v3":
        # v1: the anatomy receipt's table of the runtime's modes against slide extension (m)
        table = plan["_receipt"]["mode_table"]
        x = np.array(table["extension_m"])
        f = np.log(np.array(table["frequency_hz"]))
        return np.exp(np.array([np.interp(slide, x, f[:, k]) for k in range(f.shape[1])]))
    bore = next(b for b in plan["bodies"] if b["id"] == "bore")["body"]["configuration_path"]
    fl = np.array(bore["left"]["metric"]["frequency_hz"])
    fr = np.array(bore["right"]["metric"]["frequency_hz"])
    return np.exp(np.log(fl) + slide * (np.log(fr) - np.log(fl)))


def steady_window(start, end, fx, art):
    dur = end - start
    a = start + max(0.3 * dur, 0.07)
    b = start + 0.85 * dur
    if fx & (SCOOP | RIP) or art == "Smear":
        a = max(a, start + 0.2)
    if fx & FALL:
        b = start + 0.5 * dur
    return a, max(b, a + 0.05)


def analyse(wav: str, notes_tsv: str, plan_path: str):
    x, sr = sf.read(wav)
    mono = x[:, 0] if x.ndim > 1 else x
    plan = json.load(open(plan_path))
    if plan.get("schema_version") == "ariadne-world-plan-v3":
        plan["_receipt"] = json.load(open(plan_path.replace(".plan.json", ".receipt.json")))
    rows = []
    with open(notes_tsv) as fh:
        head = fh.readline().rstrip("\n").split("\t")
        for line in fh:
            v = dict(zip(head, line.rstrip("\n").split("\t")))
            start, end = float(v["start_s"]), float(v["end_s"])
            midi, fx, art = float(v["midi"]), int(v["fx"]), v["art"]
            partial, slide = int(v["partial"]), float(v["slide"])
            a, b = steady_window(start, end, fx, art)
            seg = mono[int(a * sr) : int(b * sr)]
            hop = int(0.03 * sr)
            win = int(0.06 * sr)
            fs, cs = [], []
            for s in range(0, max(1, len(seg) - win), hop):
                f, c = pitch(seg[s : s + win], sr)
                if c > 0.8 and f > 0:
                    fs.append(f)
                    cs.append(c)
            f = float(np.median(fs)) if fs else 0.0
            target = 440 * 2 ** ((midi - 69) / 12)
            cents = 1200 * math.log2(f / target) if f > 0 else float("nan")
            res = bore_resonances(plan, slide)
            sounded = int(np.argmin(np.abs(np.log(res / f)))) + 1 if f > 0 else 0
            rms = float(np.sqrt(np.mean(seg**2))) if len(seg) else 0.0
            rows.append(dict(start=start, end=end, midi=midi, partial=partial, position=int(v["position"]),
                             slide=slide, art=art, fx=fx, vel=float(v.get("vel", "nan")), f=f, cents=cents,
                             sounded_partial=sounded, voiced_fraction=len(fs) / max(1, len(range(0, max(1, len(seg) - win), hop))),
                             rms_db=20 * math.log10(rms + 1e-300), centroid=centroid(seg, sr) if len(seg) > 256 else 0.0))
    return rows


def summary(rows):
    c = np.array([abs(r["cents"]) for r in rows if r["f"] > 0])
    ok = [r for r in rows if r["f"] > 0]
    same = sum(r["sounded_partial"] == r["partial"] for r in ok)
    reached = sorted({(r["position"], r["sounded_partial"]) for r in ok if r["sounded_partial"] == r["partial"]
                      and abs(r["cents"]) < 50})
    return dict(notes=len(rows), voiced=len(ok), mean_abs_cents=float(c.mean()) if len(c) else None,
                median_abs_cents=float(np.median(c)) if len(c) else None, max_abs_cents=float(c.max()) if len(c) else None,
                within_10=int((c <= 10).sum()), within_25=int((c <= 25).sum()), within_50=int((c <= 50).sum()),
                sounded_intended_partial=same, partials_by_position={p: sorted({k for q, k in reached if q == p}) for p in range(1, 8)})


def brightness(rows):
    """Per written note (same midi), spectral centroid against level across its
    dynamics: least-squares slope (Hz per dB), and Spearman rank correlation."""
    out = {}
    for midi in sorted({r["midi"] for r in rows}):
        rs = [r for r in rows if r["midi"] == midi and r["f"] > 0]
        if len(rs) < 3:
            continue
        lv = np.array([r["rms_db"] for r in rs])
        ce = np.array([r["centroid"] for r in rs])
        slope = float(np.polyfit(lv, ce, 1)[0])
        rank = lambda v: np.argsort(np.argsort(v))
        rho = float(np.corrcoef(rank(lv), rank(ce))[0, 1])
        out[str(int(midi))] = dict(levels_db=[round(float(x), 1) for x in lv], centroid_hz=[round(float(x)) for x in ce],
                                   centroid_over_f0=[round(float(c / r["f"]), 2) for c, r in zip(ce, rs)],
                                   pitch_hz=[round(r["f"], 1) for r in rs], slope_hz_per_db=round(slope, 2), spearman=round(rho, 3))
    return out


def level_spread(rows, min_s=0.25):
    """Gate T6: among sustained notes (>= min_s long) written at the same dynamic,
    the dB between the quietest and loudest; per dynamic and summarized."""
    groups = {}
    for r in rows:
        if r["f"] > 0 and r["end"] - r["start"] >= min_s:
            groups.setdefault(round(r["vel"], 2), []).append(r["rms_db"])
    spread = {str(v): round(max(g) - min(g), 1) for v, g in sorted(groups.items()) if len(g) >= 2}
    values = list(spread.values())
    return dict(per_dynamic=spread, median_db=float(np.median(values)) if values else None,
                max_db=max(values) if values else None, all_notes_db=round(max(r["rms_db"] for r in rows if r["f"] > 0) - min(r["rms_db"] for r in rows if r["f"] > 0), 1))


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("wav")
    ap.add_argument("notes")
    ap.add_argument("--plan", default="profiles/ariadne-trombone-v0.plan.json")
    ap.add_argument("--json")
    ap.add_argument("--brightness", action="store_true", help="centroid against level per written note (gate T4)")
    args = ap.parse_args(argv)
    rows = analyse(args.wav, args.notes, args.plan)
    print("  t_s   midi  part pos  slide   f_Hz    cents  sounded  voiced  level_dB  centroid_Hz")
    for r in rows:
        print(f"{r['start']:6.2f} {r['midi']:5.0f} {r['partial']:5d} {r['position']:3d} {r['slide']:6.3f} {r['f']:7.1f} "
              f"{r['cents']:+7.1f} {r['sounded_partial']:7d} {r['voiced_fraction']:7.2f} {r['rms_db']:8.1f} {r['centroid']:10.0f}")
    s = summary(rows)
    if args.brightness:
        s["brightness"] = brightness(rows)
    s["level_spread"] = level_spread(rows)
    print(json.dumps(s, indent=1))
    if args.json:
        json.dump(dict(summary=s, notes=rows), open(args.json, "w"), indent=1)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
