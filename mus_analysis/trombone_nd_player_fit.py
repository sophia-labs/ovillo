"""Fit the v1 trombone player's skill from a calibration of the World.

Reads ``examples/trombone_nd_calibrate.rs``'s TSV (one tongued note per row)
and writes ``profiles/ariadne-trombone-v1.player.json``. A cell (partial n,
position k) speaks at (ratio, mouth) when the pitch is nearest bore mode n
(within 100 cents), clarity >= 0.9, and the level is within -40..+20 dB of the
calibration's 95th percentile. Per cell: the lip ratio at the middle of the
widest run of speaking ratios (ranked by how many mouth pressures speak
there), and at that ratio every speaking mouth pressure with its level, pitch
offset and harmonic centroid. The written-dynamic scale (``level_soft_db``,
``level_loud_db``) is the median over cells of their softest and loudest
levels. Everything is ``fit``: measured on this World.
"""

from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from pathlib import Path

import numpy as np


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("calibration")
    ap.add_argument("-o", "--out", default="profiles/ariadne-trombone-v1.player.json")
    args = ap.parse_args(argv)
    rows = []
    with open(args.calibration) as fh:
        head = fh.readline().split()
        for line in fh:
            r = {k: float(v) for k, v in zip(head, line.split())}
            r["partial"], r["position"] = int(r["partial"]), int(r["position"])
            rows.append(r)
    levels = sorted(r["rms"] for r in rows)
    ref = levels[int(0.95 * (len(levels) - 1))]
    lo, hi = ref * 10 ** (-40 / 20), ref * 10 ** (20 / 20)

    def speaks(r):
        if r["f_hz"] <= 0 or r["clarity"] < 0.9 or not (lo <= r["rms"] <= hi):
            return False
        return abs(1200 * math.log2(r["f_hz"] / r["bore_hz"])) < 100

    by = defaultdict(list)
    for r in rows:
        by[(r["partial"], r["position"])].append(r)
    out, grid = [], []
    softs, louds = [], []
    for n in sorted({r["partial"] for r in rows}):
        cells, line = [], []
        for k in range(1, 8):
            cs = by[(n, k)]
            ratios = sorted({r["ratio"] for r in cs})
            score = {q: sum(speaks(r) for r in cs if r["ratio"] == q) for q in ratios}
            best = max(score.values()) if score else 0
            if best == 0:
                cells.append(None)
                line.append("  .  ")
                continue
            runs, cur = [], []
            for q in ratios:
                if score[q] == best:
                    cur.append(q)
                elif cur:
                    runs.append(cur)
                    cur = []
            if cur:
                runs.append(cur)
            run = max(runs, key=len)
            q = run[len(run) // 2]
            sp = sorted((r for r in cs if r["ratio"] == q and speaks(r)), key=lambda r: r["mouth"])
            lv = [20 * math.log10(r["rms"]) for r in sp]
            softs.append(min(lv))
            louds.append(max(lv))
            # the widest run of ratios speaking at any one mouth pressure (how forgiving the cell is)
            width = 0
            for m in sorted({r["mouth"] for r in cs}):
                ok = [speaks(r) for r in sorted((r for r in cs if r["mouth"] == m), key=lambda r: r["ratio"])]
                w = c = 0
                for o in ok:
                    c = c + 1 if o else 0
                    w = max(w, c)
                width = max(width, w)
            cells.append(dict(position=k, slide_m=sp[0]["slide_m"], ratio=q,
                              offset_cents=round(float(np.mean([1200 * math.log2(r["f_hz"] / r["bore_hz"]) for r in sp])), 2),
                              window=[q - 0.0125 * width, q + 0.0125 * width],
                              mouths=[r["mouth"] for r in sp], levels_db=[round(v, 2) for v in lv],
                              centroids=[round(r["harmonic_centroid"], 3) for r in sp]))
            line.append(f"{len(sp)}m{width:2d}r")
        out.append(dict(n=n, cells=cells))
        grid.append(f"partial {n:2d}: " + " ".join(line))
    skill = dict(schema="ovillo.trombone-nd-player.v1", evidence="fit: measured on the World by trombone_nd_calibrate",
                 calibration_rows=len(rows), level_soft_db=round(float(np.median(softs)), 2),
                 level_loud_db=round(float(np.median(louds)), 2), partials=out)
    Path(args.out).write_text(json.dumps(skill, indent=1) + "\n")
    print("per position 1..7: speaking mouth pressures (m) and the widest ratio run (r)")
    print("\n".join(grid))
    print(f"written-dynamic scale: {skill['level_soft_db']} .. {skill['level_loud_db']} dB")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
