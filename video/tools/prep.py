"""Shape each segment's World data for the page, and nothing else.

    uv run --no-project --with numpy python tools/prep.py manifest.json build/

Reads the manifest's segments (the renders under data/) and writes one
build/<id>.json per instrument segment, cut to its window (plus a little
history before it, for the wavefronts that are still travelling), and
build/timeline.json (every segment's first frame and frame count).

Nothing is invented here: every array is a field of a receipt, a trace, a
notes table or the guitar's visual anatomy, resampled or summed. The only
choices are display choices (which top modes to draw, a grid to draw them
on), and they are written into the output next to the data.
"""

from __future__ import annotations

import csv
import json
import math
import re
import sys
from pathlib import Path

import numpy as np

HISTORY_S = 1.6  # wavefronts drawn this long after they leave


def rounded(a, digits=6):
    """Floats to `digits` significant digits (JSON size), keeping exact zeros."""
    a = np.asarray(a, dtype=np.float64)
    out = np.zeros_like(a)
    nz = a != 0
    mag = np.floor(np.log10(np.abs(a[nz])))
    scale = 10.0 ** (digits - 1 - mag)
    out[nz] = np.round(a[nz] * scale) / scale
    return out.tolist()


def point_in_polygon(px, py, poly):
    inside = np.zeros(px.shape, dtype=bool)
    x0, y0 = poly[-1]
    for x1, y1 in poly:
        cond = ((y1 > py) != (y0 > py)) & (px < (x0 - x1) * (py - y1) / (y0 - y1 + 1e-30) + x1)
        inside ^= cond
        x0, y0 = x1, y1
    return inside


def barycentric_lookup(points, verts, faces):
    """For each point, the containing triangle's vertex ids and weights (or None)."""
    v = np.asarray(verts)
    f = np.asarray(faces)
    a, b, c = v[f[:, 0]], v[f[:, 1]], v[f[:, 2]]
    det = (b[:, 1] - c[:, 1]) * (a[:, 0] - c[:, 0]) + (c[:, 0] - b[:, 0]) * (a[:, 1] - c[:, 1])
    out = []
    for p in points:
        l1 = ((b[:, 1] - c[:, 1]) * (p[0] - c[:, 0]) + (c[:, 0] - b[:, 0]) * (p[1] - c[:, 1])) / det
        l2 = ((c[:, 1] - a[:, 1]) * (p[0] - c[:, 0]) + (a[:, 0] - c[:, 0]) * (p[1] - c[:, 1])) / det
        l3 = 1.0 - l1 - l2
        ok = np.where((l1 >= -1e-9) & (l2 >= -1e-9) & (l3 >= -1e-9))[0]
        if len(ok) == 0:
            out.append(None)
        else:
            k = ok[0]
            out.append((f[k], (l1[k], l2[k], l3[k])))
    return out


# ------------------------------------------------------------------ guitar

TOKEN = re.compile(r"\{[^}]*\}|<[^>]*>\S*|\S+")


def mus_bars(score_path, first, last, touches):
    """The score's own text for bars first..last, each voice's tokens, and which touch group
    (by onset) each sounding token became, matched in order within the bar."""
    text = Path(score_path).read_text().splitlines()
    tempo = None
    for line in text:
        m = re.match(r"#\s*tempo:\s*([0-9.]+)", line)
        if m:
            tempo = float(m.group(1))
            break
    bars, start = [], 0.0
    for line in text:
        m = re.match(r"bar (\d+)( \[[^\]]*\])?:\s*(.*)$", line)
        if not m:
            continue
        n, attrs, body = int(m.group(1)), m.group(2) or "", m.group(3)
        tm = re.search(r"tempo=([0-9.]+)", attrs)
        bar_tempo = float(tm.group(1)) if tm else tempo
        seconds = 4 * 60.0 / bar_tempo
        if first <= n <= last:
            voices = []
            for vm in re.finditer(r"([a-z]+\.\w+)=(.*?)(?=\s+[a-z]+\.\w+=|$)", body):
                hand, rest = vm.group(1), vm.group(2).strip()
                toks = TOKEN.findall(rest)
                mine = sorted({round(tc["seconds"], 4) for tc in touches
                               if tc["hand"] == hand and start - 0.02 <= tc["seconds"] < start + seconds - 0.02})
                sounding = [i for i, tk in enumerate(toks) if not (tk.startswith("{") or tk.startswith("R") or tk == "-")]
                onsets = [None] * len(toks)
                if len(sounding) == len(mine):
                    for i, t in zip(sounding, mine):
                        onsets[i] = t
                else:
                    print(f"  bar {n} {hand}: {len(sounding)} tokens, {len(mine)} touch onsets: no highlight", file=sys.stderr)
                voices.append(dict(hand=hand, tokens=toks, onsets=onsets))
            bars.append(dict(bar=n, attrs=attrs.strip(), start=start, seconds=seconds, voices=voices,
                             line=line.strip()))
        start += seconds
    return bars



def prep_guitar(seg, root):
    vis = json.loads((root / seg["visual"]).read_text())
    trace = json.loads((root / seg["events"]).read_text())
    receipt = json.loads((root / seg["receipt"]).read_text())
    w = trace["worlds"][0]
    hop = w["hopSeconds"]
    w0, w1 = seg["window"]
    t_all = w["startSeconds"] + hop * np.arange(len(w["frames"]))
    keep = (t_all >= w0 - HISTORY_S - 1e-9) & (t_all <= w1 + 0.05)
    idx = np.where(keep)[0]
    t = t_all[idx]
    E = np.asarray(w["frames"], dtype=np.float64)[idx]
    ch = w["channels"]
    modes = np.asarray(w["modeFrames"], dtype=np.float64)[idx]
    freq = np.asarray(vis["modes"]["frequency_hz"])
    air_share = np.asarray(vis["modes"]["air_share"])
    rad = np.asarray(w["radiationFrames"], dtype=np.float64)[idx].sum(axis=1) / hop  # W
    bridge = np.asarray(w["bridgeFrames"], dtype=np.float64)[idx]
    strings = []
    for s in vis["strings"]:
        n = s["number"]
        edges = [i for i, e in enumerate(w["edges"]) if e.startswith(f"bridge/{n}/")]
        strings.append(dict(
            number=n, course=s["course"], openMidi=s["open_midi"], gauge=s["gauge_m"],
            saddleX=s["saddle_x"], nutX=s["nut_x"],
            energy=rounded(E[:, ch.index(f"string/{n}")], 5),
            # energy per second leaving the string into the box through its bridge edges
            intoBox=rounded(-bridge[:, edges].sum(axis=1) / hop, 4),
        ))
    air = (modes * air_share).sum(axis=1)
    top = (modes * (1.0 - air_share)).sum(axis=1)
    room = E[:, ch.index("room")]
    ledger = np.asarray(w["ledgerFrames"], dtype=np.float64)[idx]

    # The top: displacement law sqrt(2E)/omega for each coupled mode (as blender/ariadne_scene.py),
    # high modes lifted by (f/f0)^flatten so the eye can find them (a display choice, stated).
    flatten = seg.get("flatten", 0.6)
    amp = np.sqrt(2.0 * np.maximum(modes, 0.0)) / (2 * np.pi * freq) * (freq / freq[0]) ** flatten
    in_window = (t >= w0) & (t <= w1)
    nmodes = seg.get("modes", 24)
    chosen = np.argsort(-amp[in_window].max(axis=0))[:nmodes]
    chosen = np.sort(chosen)

    # A drawing grid on the top (lines across and along), clipped to the outline and the soundhole.
    outline = np.asarray(vis["outline"])
    hole_c, hole_r = np.asarray(vis["soundhole"]["center"]), vis["soundhole"]["radius"]
    verts, faces = vis["top"]["vertices"], vis["top"]["faces"]
    shapes = np.asarray(vis["modes"]["shapes"])[chosen]
    shapes = shapes / np.maximum(np.abs(shapes).max(axis=1, keepdims=True), 1e-30)

    def polyline_runs(xs, ys):
        inside = point_in_polygon(xs, ys, outline) & (np.hypot(xs - hole_c[0], ys - hole_c[1]) > hole_r + 0.004)
        runs, cur = [], []
        for k in range(len(xs)):
            if inside[k]:
                cur.append((xs[k], ys[k]))
            elif cur:
                runs.append(cur)
                cur = []
        if cur:
            runs.append(cur)
        return [r for r in runs if len(r) >= 3]

    y_lo, y_hi = outline[:, 1].min(), outline[:, 1].max()
    x_lo, x_hi = outline[:, 0].min(), outline[:, 0].max()
    lines = []
    for y in np.linspace(y_lo + 0.012, y_hi - 0.012, seg.get("gridAcross", 30)):
        xs = np.linspace(x_lo, x_hi, 90)
        lines += polyline_runs(xs, np.full_like(xs, y))
    for x in np.linspace(x_lo + 0.02, x_hi - 0.02, seg.get("gridAlong", 13)):
        ys = np.linspace(y_lo, y_hi, 110)
        lines += polyline_runs(np.full_like(ys, x), ys)
    grid = []
    for run in lines:
        look = barycentric_lookup(run, verts, faces)
        pts, sh = [], []
        for p, lk in zip(run, look):
            if lk is None:
                continue
            ids, wts = lk
            pts.append([float(p[0]), float(p[1])])
            sh.append([float(sum(wts[j] * shapes[m][ids[j]] for j in range(3))) for m in range(len(chosen))])
        if len(pts) >= 3:
            grid.append(dict(xy=rounded(np.asarray(pts), 5), shape=rounded(np.asarray(sh), 3)))

    touches = [tc for tc in w["touches"] if w0 - HISTORY_S <= tc["seconds"] <= w1]
    # Each string's stop as the window opens: the last touch before it (fret 0 if none).
    stop0 = {}
    for tc in w["touches"]:
        if tc["seconds"] < w0 - HISTORY_S:
            stop0[tc["string"]] = tc["fret"]
    aw = receipt["ariadneWorlds"][0]
    return dict(
        kind="guitar",
        window=[w0, w1],
        hop=hop,
        t0=float(t[0]),
        n=len(t),
        geometry=dict(
            outline=rounded(outline, 5), depth=vis["depth"], soundhole=vis["soundhole"], bridge=vis["bridge"],
            saddleY=vis["saddle_y"], nutY=vis["nut_y"], neck=vis["neck"], scaleLength=vis["scale_length"],
        ),
        strings=strings,
        stopAtStart={str(k): v for k, v in stop0.items()},
        touches=touches,
        modes=dict(chosen=chosen.tolist(), frequencyHz=rounded(freq[chosen], 5), airShare=rounded(air_share[chosen], 4),
                   amp=rounded(amp[:, chosen], 4), flatten=flatten),
        grid=grid,
        air=rounded(air, 5), top=rounded(top, 5), room=rounded(room, 5), radiatedW=rounded(rad, 5),
        ledgerColumns=w["ledgerColumns"], ledger=[rounded(r, 10) for r in ledger],
        receipt=dict(
            planDigest=aw["planDigest"], renderDigest=receipt["renderDigest"], seconds=receipt["durationSeconds"],
            plucks=aw["plucks"], hammerOns=aw["hammerOns"], pullOffs=aw["pullOffs"], slides=aw["slides"],
            sourceWork=aw["sourceWork"], controlWork=aw["controlWork"], internalLoss=aw["internalLoss"],
            radiation=aw["radiation"], cumulativeClosureResidual=aw["cumulativeClosureResidual"],
            maximumFrameResidual=aw["maximumFrameResidual"], badTokens=receipt["badTokens"],
        ),
        bars=mus_bars(root / seg["score"], seg["bars"][0], seg["bars"][1], w["touches"]) if seg.get("score") else [],
    )


# --------------------------------------------------------------- trombone


def prep_trombone(seg, root):
    trace = json.loads((root / seg["events"]).read_text())
    receipt = json.loads((root / seg["receipt"]).read_text())
    analysis = json.loads((root / seg["analysis"]).read_text())
    plan = json.loads((root / seg["plan"]).read_text())
    anatomy = json.loads((root / seg["anatomy"]).read_text())
    cols = trace["columns"]
    rows = np.asarray(trace["rows"], dtype=np.float64)
    t_all = rows[:, cols.index("t")]
    w0, w1 = seg["window"]
    keep = (t_all >= w0 - HISTORY_S) & (t_all <= w1 + 0.05)
    rows = rows[keep]
    pressure = np.asarray(trace["pressureRmsPa"], dtype=np.float64)[keep]
    net = plan["bodies"][0]["constitution"]["body"]["configuration_space"]["map"]["network"]
    tubes = [dict(id=tb["id"], lengthM=tb["length_m"], sections=len(tb["base_log_area_m2"])) for tb in net["tubes"]]
    slide_term = next(a for a in net["area_terms"] if a["kind"] == "tube_length")
    notes = []
    with open(root / seg["notes"]) as fh:
        for r, a in zip(csv.DictReader(fh, delimiter="\t"), analysis["notes"]):
            start, end = float(r["start_s"]), float(r["end_s"])
            if end < w0 - HISTORY_S or start > w1:
                continue
            notes.append(dict(start=start, end=end, midi=int(float(r["midi"])), partial=int(r["partial"]),
                              slide=float(r["slide"]), position=int(r["position"]), art=r["art"], fx=int(r["fx"]),
                              vel=float(r["vel"]), boreHz=float(r["bore_hz"]),
                              measuredHz=a["f"], cents=a["cents"], soundedPartial=a["sounded_partial"]))
    return dict(
        kind="trombone",
        window=[w0, w1],
        hop=trace["hopSeconds"],
        columns=cols,
        rows=[rounded(r, 8) for r in rows],
        pressureRmsPa=[rounded(p, 4) for p in pressure],
        areasM2=rounded(trace["areasM2"], 6),
        tubes=tubes,
        slideTerm=slide_term,
        positionsM=anatomy["slide"]["positions_m"],
        notes=notes,
        initialEnergy=trace["initialEnergy"],
        receipt=dict(planDigest=receipt["planDigest"], seconds=receipt["seconds"], ledger=receipt["ledger"],
                     moves=receipt["moves"], notes=receipt["notes"]),
        analysis=analysis["summary"],
    )


# ------------------------------------------------------------------ tract


def prep_tract(seg, root):
    fr = json.loads((root / seg["events"]).read_text())
    receipt = json.loads((root / seg["receipt"]).read_text())
    w0, w1 = seg["window"]
    fps = fr["fps"]
    frames = fr["frames"]
    books = fr["receipts"]
    # frame k is the World after (k+1)/fps seconds
    t = (np.arange(len(frames)) + 1) / fps
    keep = [k for k in range(len(frames)) if w0 - HISTORY_S <= t[k] <= w1 + 0.05]
    return dict(
        kind="tract",
        window=[w0, w1],
        fps=fps,
        t0=float(t[keep[0]]),
        layout=fr["layout"],
        frames=[frames[k] for k in keep],
        receiptColumns=fr["receiptColumns"],
        receipts=[books[k] for k in keep],
        script=receipt["script"],
        receipt=dict(receipt=receipt["receipt"], books=receipt["books"], wavSha256=receipt["wavSha256"],
                     wasmSha256=receipt["wasmSha256"], observersCheck=receipt["observersCheck"]),
    )


# ------------------------------------------------------------------ piano


def read_piano_notes(path):
    """onset sec, midi note, velocity, release (sec): the piano World's notes table."""
    rows = []
    with open(path) as fh:
        reader = csv.reader(fh, delimiter="\t")
        header = next(reader)
        for r in reader:
            if not r or r[0].startswith("#"):
                continue
            rows.append(dict(onset=float(r[0]), midi=int(float(r[1])), velocity=float(r[2]), release=float(r[3])))
    return header, rows


def prep_piano(seg, root):
    header, notes = read_piano_notes(root / seg["events"])
    w0, w1 = seg["window"]
    notes = [n for n in notes if n["release"] >= w0 - HISTORY_S and n["onset"] <= w1]
    out = dict(kind="piano", window=[w0, w1], placeholder=bool(seg.get("placeholder")), header=header,
               notes=notes, energy=None, receipt=None, bars=[])
    if not seg.get("energy"):
        return out
    # The Ariadne piano's trace (mus-engine ariadne_piano::PianoEnergyTrace, under mus render
    # --energy's "worlds"): bodies "board/...", "string/<key>/<k>", "hammer/<key>", then "room".
    e = json.loads((root / seg["energy"]).read_text())
    tr = next(w for w in e.get("worlds", [e]) if "strikes" in w)
    hop, start = tr["hopSeconds"], tr["startSeconds"]
    F_all = np.asarray(tr["frames"], dtype=np.float64)
    t_all = start + hop * np.arange(len(F_all))
    keep = (t_all >= w0 - HISTORY_S - 1e-9) & (t_all <= w1 + 0.05)
    ch = tr["channels"]
    F = F_all[keep]
    name2midi = {s_["name"]: s_["midi"] for s_ in tr["strikes"]}
    def by_key(prefix):
        groups = {}
        for i, c in enumerate(ch):
            if c.startswith(prefix):
                m = name2midi.get(c.split("/")[1])
                if m is not None:
                    groups.setdefault(m, []).append(i)
        return {str(m): rounded(F[:, idx].sum(axis=1), 5) for m, idx in groups.items()}
    board_idx = [i for i, c in enumerate(ch) if c.startswith("board")]
    rad = np.asarray(tr["radiationFrames"], dtype=np.float64)
    src = np.asarray(tr["sourceFrames"], dtype=np.float64)
    loss = np.asarray(tr["lossFrames"], dtype=np.float64)
    bridge = np.asarray(tr["bridgeFrames"], dtype=np.float64)
    energy = dict(
        hop=hop, t0=float(t_all[keep][0]),
        strings=by_key("string/"), hammers=by_key("hammer/"),
        board=rounded(F[:, board_idx].sum(axis=1), 5),
        room=rounded(F[:, ch.index("room")], 5),
        radiatedW=rounded(rad[keep].sum(axis=1) / hop, 5),
        intoBoardW=rounded(-bridge[keep].sum(axis=1) / hop, 5),
        # running books from the trace's own per-hop flows (f32 sums: not a residual)
        inJ=rounded(np.cumsum(src.sum(axis=1))[keep], 7), lostJ=rounded(np.cumsum(loss.sum(axis=1))[keep], 7),
        radiatedJ=rounded(np.cumsum(rad.sum(axis=1))[keep], 7),
        ledgerColumns=tr.get("ledgerColumns"),
        ledger=[rounded(r, 10) for r in np.asarray(tr["ledgerFrames"])[keep]] if tr.get("ledgerFrames") else None,
        strikes=[s_ for s_ in tr["strikes"] if w0 - HISTORY_S <= s_["seconds"] <= w1],
        pedal=tr.get("pedal", []),
    )
    out["energy"] = energy
    if seg.get("receipt"):
        r = json.loads((root / seg["receipt"]).read_text())
        aw = next((w for w in r.get("ariadneWorlds", []) if "keys" in w), r)
        out["receipt"] = {k: aw.get(k) for k in ("planDigest", "strikes", "refusedStrikes", "damperFalls", "pedalSpans",
                                                    "simulatedSeconds", "contactMsMin", "contactMsMax", "sourceWork",
                                                    "controlWork", "internalLoss", "radiation", "boardRadiation",
                                                    "stringRadiation", "cumulativeClosureResidual", "maximumFrameResidual")}
        out["receipt"]["renderDigest"] = r.get("renderDigest")
    if seg.get("score"):
        touches = [dict(seconds=s_["seconds"], hand=s_.get("staff", ""), string=0, fret=0) for s_ in tr["strikes"]]
        out["bars"] = mus_bars(root / seg["score"], seg["bars"][0], seg["bars"][1], touches)
    return out


def main(argv):
    manifest_path, out = Path(argv[1]), Path(argv[2])
    root = manifest_path.parent
    manifest = json.loads(manifest_path.read_text())
    out.mkdir(parents=True, exist_ok=True)
    fps = manifest["fps"]
    timeline, frame = [], 0
    for seg in manifest["segments"]:
        n = int(round(seg["seconds"] * fps))
        timeline.append(dict(id=seg["id"], kind=seg["kind"], firstFrame=frame, frames=n))
        frame += n
        fn = dict(guitar=prep_guitar, trombone=prep_trombone, tract=prep_tract, piano=prep_piano).get(seg["kind"])
        if fn is None:
            continue
        data = fn(seg, root)
        data["id"] = seg["id"]
        (out / f"{seg['id']}.json").write_text(json.dumps(data, separators=(",", ":")))
        print(f"{seg['id']}: {(out / (seg['id'] + '.json')).stat().st_size / 1e6:.2f} MB")
    (out / "timeline.json").write_text(json.dumps(dict(fps=fps, frames=frame, segments=timeline), indent=1))
    print(f"timeline: {frame} frames, {frame / fps:.2f} s")


if __name__ == "__main__":
    main(sys.argv)
