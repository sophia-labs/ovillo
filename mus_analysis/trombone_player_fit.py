"""Fit the trombone player's skill from a calibration of the World.

Reads the TSV written by ``ovillo/crates/ovillo-dsp/examples/trombone_world_calibrate.rs``
(one tongued note per row: partial n, slide, lip ratio, mouth pressure, bore
resonance, measured pitch, clarity, level) and writes
``profiles/ariadne-trombone-v0.player.json``. For each partial and each
calibrated slide point: whether the partial speaks there, the lip ratio (lip
natural frequency / bore resonance n) at the middle of the widest run of
ratios that speak it at one pressure, the pressures around that one where
that ratio still speaks it, and the measured pitch offset from the bore
resonance. A cell "speaks" partial n when its pitch is nearest bore
resonance n (within 150 cents), its autocorrelation clarity is at least 0.9,
and its level is within -40..+20 dB of the calibration's 95th-percentile level
(quieter: it did not speak; louder: it ran away).
Everything here is ``fit``: measured on this World, not taken from players.
"""

from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from pathlib import Path


def speaks(row, floor) -> bool:
    n, f, fb, clar, rms = row["partial"], row["f_hz"], row["bore_hz"], row["clarity"], row["rms"]
    low, high = floor
    if f <= 0 or clar < 0.9 or rms < low or rms > high:
        return False
    return abs(1200 * math.log2(f / fb)) < 150 and round(n * f / fb) == n


def runs(values):
    out, cur = [], []
    for v, ok in values:
        if ok:
            cur.append(v)
        elif cur:
            out.append(cur)
            cur = []
    if cur:
        out.append(cur)
    return out


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("calibration")
    ap.add_argument("-o", "--out", default="profiles/ariadne-trombone-v0.player.json")
    ap.add_argument("--plan-receipt", default="profiles/ariadne-trombone-v0.receipt.json")
    args = ap.parse_args(argv)
    rows = []
    with open(args.calibration) as fh:
        head = fh.readline().split()
        for line in fh:
            r = {k: float(x) for k, x in zip(head, line.split())}
            r["partial"] = int(r["partial"])
            rows.append(r)
    levels = sorted(r["rms"] for r in rows)
    ref = levels[int(0.95 * (len(levels) - 1))]
    floor = (ref * 10 ** (-40 / 20), ref * 10 ** (20 / 20))   # quieter: did not speak; louder: ran away
    cell = {(r["partial"], r["slide"], r["ratio"], r["pressure"]): r for r in rows}
    partials = sorted({r["partial"] for r in rows})
    slides = sorted({r["slide"] for r in rows})
    ratios = sorted({r["ratio"] for r in rows})
    pressures = sorted({r["pressure"] for r in rows})
    out = []
    grid = []
    for n in partials:
        entry = dict(n=n, slides=slides, speaks=[], ratio=[], pressure_soft=[], pressure_loud=[], offset_cents=[], window=[])
        line = []
        for s in slides:
            best = None
            for p in pressures:
                for run in runs([(q, speaks(cell[(n, s, q, p)], floor)) for q in ratios]):
                    mid = run[len(run) // 2]
                    level = cell[(n, s, mid, p)]["rms"]
                    key = (len(run), level)
                    if best is None or key > best[0]:
                        best = (key, p, run, mid)
            if best is None:
                entry["speaks"].append(False)
                for k in ("ratio", "pressure_soft", "pressure_loud", "offset_cents"):
                    entry[k].append(None)
                entry["window"].append(None)
                line.append("  .  ")
                continue
            _, p0, run, mid = best
            i0 = pressures.index(p0)
            lo = hi = i0
            while lo > 0 and speaks(cell[(n, s, mid, pressures[lo - 1])], floor):
                lo -= 1
            while hi < len(pressures) - 1 and speaks(cell[(n, s, mid, pressures[hi + 1])], floor):
                hi += 1
            good = pressures[lo : hi + 1]
            offs = [1200 * math.log2(cell[(n, s, mid, p)]["f_hz"] / cell[(n, s, mid, p)]["bore_hz"]) for p in good]
            entry["speaks"].append(True)
            entry["ratio"].append(mid)
            entry["pressure_soft"].append(good[0])
            entry["pressure_loud"].append(good[-1])
            entry["offset_cents"].append(round(sum(offs) / len(offs), 2))
            entry["window"].append([run[0], run[-1]])
            line.append(f"{len(run):2d}r{len(good)}p")
        out.append(entry)
        grid.append(f"partial {n:2d}: " + " ".join(line))
    plan_digest = None
    if Path(args.plan_receipt).exists():
        plan_digest = json.loads(Path(args.plan_receipt).read_text()).get("plan_digest")
    skill = dict(schema="ovillo.trombone-player.v1", evidence="fit: measured on the World by trombone_world_calibrate",
                 plan_digest=plan_digest, calibration_rows=len(rows), speaking_floor_db=-40.0, partials=out)
    Path(args.out).write_text(json.dumps(skill, indent=1) + "\n")
    print("speaking runs per slide position 1..7 (ratios r, pressures p at that ratio):")
    print("\n".join(grid))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
