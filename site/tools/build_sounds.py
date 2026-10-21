"""Build the listening room: site/sounds/<slug>.{mp3,wave.svg,energy.svg,mus|json,receipt.json} + manifest.json.

The listening room was the first, static draft of ovillo.sophia-labs.com (2026-10-20); its pages were replaced by the
Sophia app, which reads the graph that site/graph/build_graph.py builds from the same catalogue.

    uv run --with numpy --with scipy --with soundfile python site/tools/build_sounds.py <pieces.json> <out-dir>

pieces.json lists every piece: slug, title, group, tags, engine, note, wav, score, receipt, energy.
Loudness is matched after the World (a gain to a common RMS, peak kept under -1 dBFS); the receipts
and the waveforms' numbers are the World's own, before that gain.
"""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy import signal

TARGET_RMS = -18.0
PEAK_CEIL = -1.0
W, H = 720, 220


def db(x):
    return 20 * np.log10(np.maximum(x, 1e-12))


def waterfall_svg(mono, sr, path):
    """A waterfall of line traces: 36 spectra, log frequency 50 Hz-12 kHz, newest at the front."""
    n_slices, n_bins = 36, 160
    hop = max(1, (len(mono) - 4096) // n_slices)
    edges = np.geomspace(50, min(12000, sr / 2 - 1), n_bins + 1)
    win = np.hanning(4096)
    freqs = np.fft.rfftfreq(4096, 1 / sr)
    rows = []
    for i in range(n_slices):
        seg = mono[i * hop : i * hop + 4096]
        if len(seg) < 4096:
            seg = np.pad(seg, (0, 4096 - len(seg)))
        mag = np.abs(np.fft.rfft(seg * win))
        idx = np.digitize(freqs, edges) - 1
        band = np.array([mag[idx == b].max() if np.any(idx == b) else 0 for b in range(n_bins)])
        rows.append(db(band))
    rows = np.array(rows)
    top = rows.max()
    rows = np.clip((rows - (top - 70)) / 70, 0, 1)
    lines = []
    dy = (H - 70) / n_slices
    for i, r in enumerate(rows):
        y0 = 30 + i * dy + 40
        x0 = 40 + (n_slices - i) * 4
        span = W - 80 - n_slices * 4
        pts = " ".join(f"{x0 + span * b / (n_bins - 1):.1f},{y0 - 52 * v:.1f}" for b, v in enumerate(r))
        lvl = 0.35 + 0.65 * i / (n_slices - 1)
        lines.append(f'<polyline class="w" style="stroke-opacity:{lvl:.2f}" points="{pts}"/>')
    ticks = []
    for f, label in [(100, "100"), (1000, "1k"), (10000, "10k")]:
        x = 40 + n_slices * 4 + (W - 80 - n_slices * 4) * np.log(f / edges[0]) / np.log(edges[-1] / edges[0])
        ticks.append(f'<line class="g" x1="{x:.1f}" y1="{H-24}" x2="{x:.1f}" y2="{H-18}"/><text x="{x:.1f}" y="{H-6}" text-anchor="middle">{label}</text>')
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" role="img" aria-label="spectral waterfall">'
        '<style>.w{fill:none;stroke:var(--trace-a,#7CFC9A);stroke-width:1}.g{stroke:var(--grat,#2a3a2f);stroke-width:1}'
        'text{fill:var(--dim,#6f8f78);font:10px ui-monospace,Menlo,monospace}</style>'
        + "".join(lines) + "".join(ticks)
        + f'<text x="40" y="{H-6}">LOG HZ</text></svg>'
    )


def energy_svg(trace, path):
    """10 log10 E of the strings, the box and the room, every 10 ms, with each touch marked."""
    worlds = trace["worlds"]
    series = {"STRINGS": None, "BOX": None, "ROOM": None}
    touches = []
    for w in worlds:
        ch = w["channels"]
        fr = np.array(w["frames"], dtype=float)
        s = fr[:, [i for i, c in enumerate(ch) if c.startswith("string/")]].sum(axis=1)
        b = fr[:, [i for i, c in enumerate(ch) if c.startswith("box/")]].sum(axis=1)
        r = fr[:, [i for i, c in enumerate(ch) if c == "room"]].sum(axis=1)
        for k, v in (("STRINGS", s), ("BOX", b), ("ROOM", r)):
            series[k] = v if series[k] is None else series[k][: len(v)] + v[: len(series[k])]
        touches += [t["seconds"] for t in w.get("touches", [])]
    hop = worlds[0]["hopSeconds"]
    n = len(series["STRINGS"])
    dur = n * hop
    allv = np.concatenate([10 * np.log10(np.maximum(v, 1e-30)) for v in series.values()])
    hi = np.percentile(allv, 99.5) + 3
    lo = hi - 60
    x = lambda t: 40 + (W - 80) * t / dur
    y = lambda e: 20 + (H - 60) * (1 - (np.clip(e, lo, hi) - lo) / (hi - lo))
    step = max(1, n // 700)
    cls = {"STRINGS": "s", "BOX": "b", "ROOM": "r"}
    paths = []
    for k, v in series.items():
        e = 10 * np.log10(np.maximum(v[::step], 1e-30))
        pts = " ".join(f"{x(i * step * hop):.1f},{y(val):.1f}" for i, val in enumerate(e))
        paths.append(f'<polyline class="{cls[k]}" points="{pts}"/>')
    tk = "".join(f'<line class="t" x1="{x(t):.1f}" y1="{H-38}" x2="{x(t):.1f}" y2="{H-32}"/>' for t in touches)
    grat = "".join(f'<line class="g" x1="40" y1="{y(lo + d):.1f}" x2="{W-40}" y2="{y(lo + d):.1f}"/>' for d in (0, 20, 40, 60))
    secs = "".join(f'<text x="{x(t):.1f}" y="{H-18}" text-anchor="middle">{t:.0f}</text>' for t in np.arange(0, dur, 10 if dur > 30 else 5))
    legend = (f'<text x="40" y="12" class="s">STRINGS</text><text x="110" y="12" class="b">BOX</text>'
              f'<text x="150" y="12" class="r">ROOM</text><text x="{W-40}" y="12" text-anchor="end">10·LOG10 E, 60 DB SPAN · TICKS = TOUCHES · S</text>')
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" role="img" aria-label="stored energy by body">'
        '<style>polyline{fill:none;stroke-width:1.2}.s{stroke:var(--amber,#FFB547);fill:var(--amber,#FFB547)}'
        '.b{stroke:var(--trace-a,#7CFC9A);fill:var(--trace-a,#7CFC9A)}.r{stroke:var(--blue,#6CB8FF);fill:var(--blue,#6CB8FF)}'
        'polyline.s,polyline.b,polyline.r{fill:none}.g{stroke:var(--grat,#2a3a2f);stroke-width:1}.t{stroke:var(--dim,#6f8f78);stroke-width:1}'
        'text{fill:var(--dim,#6f8f78);font:10px ui-monospace,Menlo,monospace}text.s,text.b,text.r{stroke:none}</style>'
        + grat + "".join(paths) + tk + secs + legend + "</svg>"
    )


def main():
    pieces = json.loads(Path(sys.argv[1]).read_text())
    out = Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    manifest = []
    for p in pieces:
        slug = p["slug"]
        x, sr = sf.read(p["wav"], always_2d=True)
        mono = x.mean(axis=1)
        rms = db(np.sqrt(np.mean(mono**2)))
        peak = db(np.max(np.abs(x)))
        gain = min(TARGET_RMS - rms, PEAK_CEIL - peak)
        subprocess.run(
            ["ffmpeg", "-loglevel", "error", "-y", "-i", p["wav"], "-af", f"highpass=f=18,volume={gain:.2f}dB",
             "-codec:a", "libmp3lame", "-q:a", "2", str(out / f"{slug}.mp3")],
            check=True,
        )
        (out / f"{slug}.wave.svg").write_text(waterfall_svg(mono, sr, None))
        entry = {k: p[k] for k in ("slug", "title", "group", "tags", "engine", "note") if k in p}
        entry.update(seconds=round(len(mono) / sr, 1), worldPeakDbfs=round(float(peak), 1),
                     worldRmsDbfs=round(float(rms), 1), pageGainDb=round(float(gain), 1),
                     audio=f"sounds/{slug}.mp3", wave=f"sounds/{slug}.wave.svg")
        if p.get("energy") and Path(p["energy"]).exists():
            tr = json.loads(Path(p["energy"]).read_text())
            if tr.get("worlds"):
                (out / f"{slug}.energy.svg").write_text(energy_svg(tr, None))
                entry["energySvg"] = f"sounds/{slug}.energy.svg"
        if p.get("score"):
            ext = Path(p["score"]).suffix
            shutil.copyfile(p["score"], out / f"{slug}{ext}")
            entry["score"] = f"sounds/{slug}{ext}"
        if p.get("receipt"):
            r = json.loads(Path(p["receipt"]).read_text())
            keep = {k: r[k] for k in ("schema", "renderDigest", "sourceDigest", "engineVersion", "peakDbfs", "rmsDbfs",
                                      "durationSeconds", "wallSeconds", "eventCount", "badTokens", "warnings") if k in r}
            if r.get("ariadneWorlds"):
                keep["ariadneWorlds"] = [{k: v for k, v in w.items() if k != "strings"} for w in r["ariadneWorlds"]]
                entry["ledger"] = [{k: w[k] for k in ("instrument", "plucks", "hammerOns", "pullOffs", "slides", "sourceWork",
                                                      "controlWork", "internalLoss", "radiation", "cumulativeClosureResidual")}
                                   for w in r["ariadneWorlds"]]
            (out / f"{slug}.receipt.json").write_text(json.dumps(keep, indent=1))
            entry["receipt"] = f"sounds/{slug}.receipt.json"
        if p.get("bench"):
            entry["bench"] = p["bench"]
        manifest.append(entry)
        print(f"{slug:24s} {entry['seconds']:6.1f}s world rms {rms:6.1f} peak {peak:6.1f} -> gain {gain:+5.1f} dB")
    (out / "manifest.json").write_text(json.dumps(manifest, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
