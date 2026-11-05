"""Shape the 1812-for-guts render for the film's page, and nothing else.

    uv run --no-project --with numpy python tools/prep.py <score dir> build/ [--fps 30] [--leader 3] [--trailer 5]

Reads the score (`score.json`), the mix's timeline (`out/mix.json`), every
stem's report (`out/stems/<section>/<voice>.json`) and trace
(`out/stems/<section>/<voice>.trace.f32`: the World's observers at every
10 ms hop, columns listed in the report), and writes

    build/film.json    the timeline (fps, leader, sections with their start,
                       length, bar range, source parts, bpm), the voices, the
                       per-section note tables, per-voice maxima for levels,
                       the shot list, and the frame file's layout
    build/frames.f32   one row per video frame: for each voice in order, the
                       FRAME_COLUMNS below (little-endian f32)

Nothing is invented here: every number is a field of a stem's report or
trace, resampled from 10 ms hops to video frames (state fields: the hop at
the frame; the valve's gap: its smallest and largest over the frame's hops;
cumulative energies: their change over the frame, as power; eaten gas: the
sum). The shot list is a direction choice and says so; the guts' places on
the stage follow the score's pans.
"""

from __future__ import annotations

import json
import struct
import sys
from pathlib import Path

import numpy as np

VOICES = ["v1", "v2", "v3", "v4", "v5", "v6", "cannon"]
# the trace's scalar columns (ovillo-fart TRACE_SCALARS), then areaM2[i], pressureRmsPa[i]
FRAME_SCALARS = [
    "f0Hz", "gasFraction", "supplyPa", "reservoirJ", "stomachJ", "gapMinM", "gapMaxM", "flowM3S", "fieldJ",
    "radW", "turbW", "jetApertureM2", "jetPosition", "push", "cheeks", "tone", "tensionHz", "clench", "rasp",
    "digest", "sound", "shot", "eatenJ",
]


def main() -> None:
    args = sys.argv[1:]
    score_dir = Path(args[0])
    out = Path(args[1])
    opt = lambda k, d: float(args[args.index(k) + 1]) if k in args else d
    fps = opt("--fps", 30.0)
    leader = opt("--leader", 3.0)
    trailer = opt("--trailer", 5.0)
    out.mkdir(parents=True, exist_ok=True)

    score = json.loads((score_dir / "score.json").read_text())
    mix = json.loads((score_dir / "out" / "mix.json").read_text())
    stems = score_dir / "out" / "stems"
    total_s = leader + mix["seconds"] + trailer
    frames = int(round(total_s * fps))
    n_oral = None

    sections = []
    notes = {}
    rows = None
    maxima = {v: {"radW": 0.0, "turbW": 0.0, "prms": 0.0, "fieldJ": 0.0, "flowM3S": 0.0, "stomachJ": 0.0, "supplyPa": 0.0} for v in VOICES}
    books = {}
    for i, s in enumerate(score["sections"]):
        m = next(x for x in mix["sections"] if x["section"] == s["id"])
        start = leader + m["startS"]
        sections.append({
            "id": s["id"], "index": i, "startS": round(start, 3), "seconds": m["seconds"], "bpm": s["bpm"], "bar": s.get("bar"),
            "about": s.get("about", ""), "voices": s["voices"],
        })
        f0 = int(np.floor(start * fps))
        f1 = int(np.ceil((start + m["seconds"]) * fps))
        for v in s["voices"]:
            rep = json.loads((stems / s["id"] / f"{v}.json").read_text())
            tr = rep["trace"]
            n_oral = tr["oralSections"]
            ncol = len(tr["columns"])
            data = np.fromfile(stems / s["id"] / tr["file"], dtype="<f4").reshape(-1, ncol).astype(np.float64)
            hop = tr["hopS"]
            col = {c: k for k, c in enumerate(tr["columns"])}
            if rows is None:
                rows = np.zeros((frames, len(VOICES), len(FRAME_SCALARS) + 2 * n_oral), dtype=np.float64)
            vi = VOICES.index(v)
            # hop boundaries per frame, in this stem's hops (frame f spans [t_f, t_f+1) relative to the section start)
            fr = np.arange(f0, min(f1, frames) + 1)
            b = np.clip(np.round((fr / fps - start) / hop).astype(int), 0, data.shape[0])
            keep = b[1:] > b[:-1]
            lo, hi, fk = b[:-1][keep], b[1:][keep], fr[:-1][keep]
            at = np.clip(hi - 1, 0, data.shape[0] - 1)
            state = lambda name: data[at, col[name]]
            red = lambda fn, name: fn.reduceat(data[:, col[name]], lo)[: len(lo)] if len(lo) else np.zeros(0)
            dt = (hi - lo) * hop
            cum = lambda name: (data[at, col[name]] - np.where(lo > 0, data[np.clip(lo - 1, 0, None), col[name]], 0.0)) / dt
            R = rows[fk, vi]
            R[:, 0] = state("f0Hz"); R[:, 1] = state("gasFraction"); R[:, 2] = state("supplyPa"); R[:, 3] = state("reservoirJ"); R[:, 4] = state("stomachJ")
            R[:, 5] = red(np.minimum, "gapMinM"); R[:, 6] = red(np.maximum, "gapMaxM"); R[:, 7] = state("flowM3S"); R[:, 8] = state("fieldJ")
            R[:, 9] = cum("radiatedJ"); R[:, 10] = cum("turbulenceJ"); R[:, 11] = state("jetApertureM2"); R[:, 12] = state("jetPosition")
            for k, name in enumerate(["push", "cheeks", "tone", "tensionHz"]): R[:, 13 + k] = state(name)
            R[:, 17] = red(np.maximum, "clench"); R[:, 18] = state("rasp"); R[:, 19] = state("digest"); R[:, 20] = red(np.maximum, "sound"); R[:, 21] = red(np.maximum, "shot")
            R[:, 22] = red(np.add, "eatenJ")
            a0 = col["areaM2[0]"]; p0 = col["pressureRmsPa[0]"]
            R[:, 23 : 23 + n_oral] = data[at, a0 : a0 + n_oral]
            ms = data[:, p0 : p0 + n_oral] ** 2
            R[:, 23 + n_oral :] = np.sqrt(np.add.reduceat(ms, lo, axis=0)[: len(lo)] / (hi - lo)[:, None])
            rows[fk, vi] = R
            mx = maxima[v]
            mx["radW"] = max(mx["radW"], float(R[:, 9].max(initial=0))); mx["turbW"] = max(mx["turbW"], float(R[:, 10].max(initial=0)))
            mx["prms"] = max(mx["prms"], float(R[:, 23 + n_oral :].max(initial=0))); mx["fieldJ"] = max(mx["fieldJ"], float(R[:, 8].max(initial=0)))
            mx["flowM3S"] = max(mx["flowM3S"], float(np.abs(R[:, 7]).max(initial=0))); mx["stomachJ"] = max(mx["stomachJ"], float(R[:, 4].max(initial=0)))
            mx["supplyPa"] = max(mx["supplyPa"], float(R[:, 2].max(initial=0)))
            notes.setdefault(s["id"], {})[v] = [
                {k: n.get(k) for k in ("atS", "token", "targetHz", "tensionHz", "measuredHz", "cents", "gas", "line") if k in n} for n in rep["notes"]
            ]
            books.setdefault(s["id"], {})[v] = {"inTune50Cents": rep["inTune50Cents"], **rep["books"]}
        print(f"{s['id']}: {len(s['voices'])} parts, frames {f0}..{f1}", file=sys.stderr)

    # frames with no World yet (the cards, a gut's rests between sections' ring-outs) show its nearest
    # state: the anatomy at rest, nothing sounding
    anatomies = {}
    for vi, v in enumerate(VOICES):
        has = rows[:, vi, len(FRAME_SCALARS)] > 0
        if not has.any():
            continue
        idx = np.where(has, np.arange(frames), -1)
        fwd = np.maximum.accumulate(idx)
        first = int(np.argmax(has))
        src = np.where(fwd >= 0, fwd, first)
        missing = ~has
        rest = rows[src[missing], vi].copy()
        rest[:, :3] = 0.0  # f0, gas fraction stays? no: keep gas and reservoir; silence f0 and supply
        rest[:, 0] = 0.0; rest[:, 2] = 0.0; rest[:, 5:13] = 0.0; rest[:, 13] = 0.0; rest[:, 17:23] = 0.0
        rest[:, 1] = rows[src[missing], vi, 1]
        rest[:, len(FRAME_SCALARS) + n_oral :] = 0.0
        rows[missing, vi] = rest
    for v in VOICES:
        for s in score["sections"]:
            if v in s["voices"]:
                rep = json.loads((stems / s["id"] / f"{v}.json").read_text())
                anatomies[v] = rep.get("anatomy", {"name": "cleft-nd-v1", "oralLengthM": 0.20})
                break
    shots = plan_shots(sections, rows, fps, leader, mix["seconds"], notes)
    film = {
        "schema": "ovillo.fart-film.v1",
        "title": score["title"], "about": score.get("about", ""), "direction": score.get("direction", {}),
        "fps": fps, "frames": frames, "leaderS": leader, "trailerS": trailer, "musicS": mix["seconds"], "totalS": round(total_s, 3),
        "voices": VOICES, "voiceSpecs": {v: score["voices"][v] for v in VOICES},
        "oralSections": n_oral, "frameColumns": FRAME_SCALARS + [f"areaM2[{i}]" for i in range(n_oral)] + [f"pressureRmsPa[{i}]" for i in range(n_oral)],
        "layout": {"rows": frames, "voices": len(VOICES), "columns": rows.shape[2], "file": "frames.f32", "dtype": "<f4"},
        "sections": sections, "notes": notes, "books": books, "maxima": maxima, "shots": shots,
        "anatomies": anatomies, "sealAt": 0.85, "raspAt": 0.93,
    }
    (out / "film.json").write_text(json.dumps(film, indent=1))
    rows.astype("<f4").tofile(out / "frames.f32")
    print(f"{frames} frames x {len(VOICES)} voices x {rows.shape[2]} columns -> {out / 'frames.f32'} ({rows.size * 4 / 1e6:.1f} MB); {len(shots)} shots", file=sys.stderr)


def plan_shots(sections, rows, fps, leader, music_s, notes):
    """The shot list: a direction choice. Cuts at section starts and every few
    beats inside a section; the subject of a close shot is the gut that
    radiated the most power over that shot (the trace's radiated energy);
    a cannon shot in the window makes it a cannon shot; the finale cranes."""
    rad = rows[:, :, 9]  # frames x voices
    shots = []
    kinds_cycle = ["close", "orbit", "two", "wide", "low", "close", "orbit", "dolly"]
    k = 0
    for s in sections:
        t0, t1 = s["startS"], s["startS"] + s["seconds"]
        beat = 60.0 / s["bpm"]
        length = min(9.0, max(3.5, 8 * beat))
        finale = s["index"] >= 45
        intro = s["index"] == 0
        # cannon shots in this section (frames where the cannon's `shot` flag is up)
        cannon_frames = np.where(rows[int(t0 * fps) : int(t1 * fps), 6, 21] > 0.5)[0] + int(t0 * fps)
        cannon_times = []
        for f in cannon_frames:
            if not cannon_times or f / fps - cannon_times[-1] > 0.5:
                cannon_times.append(f / fps)
        t = t0
        first = True
        while t < t1 - 0.5:
            end = min(t1, t + length)
            if end > t1 - 1.5:
                end = t1
            # a cannon shot begins at the first cannon time inside the window
            cannon = next((c for c in cannon_times if t <= c - 0.3 < end), None)
            f0, f1 = int(t * fps), max(int(t * fps) + 1, int(end * fps))
            power = rad[f0:f1, :6].sum(axis=0)
            order = list(np.argsort(-power))
            subject = int(order[0]) if power[order[0]] > 0 else 2
            second = int(order[1])
            if intro and first:
                kind = "push"
            elif cannon is not None:
                kind = "cannon"
                end = min(t1, max(end, cannon + 2.5))
            elif finale:
                kind = ["crane", "wide", "low", "crane"][k % 4]
            elif first and s["index"] > 0 and sections[s["index"] - 1]["bpm"] != s["bpm"]:
                kind = "wide"
            else:
                kind = kinds_cycle[k % len(kinds_cycle)]
            shots.append({"t0": round(t, 3), "t1": round(end, 3), "kind": kind, "subject": subject, "second": second, "section": s["id"]})
            k += 1
            first = False
            t = end
    # the cards
    shots.insert(0, {"t0": 0.0, "t1": leader, "kind": "card", "subject": -1, "second": -1, "section": "title"})
    shots.append({"t0": round(leader + music_s, 3), "t1": round(leader + music_s + 5.0, 3), "kind": "end", "subject": -1, "second": -1, "section": "end"})
    return shots


if __name__ == "__main__":
    main()
