# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy", "scipy", "soundfile", "pyloudnorm"]
# ///
"""Mix and master *Rule, Britannia!* from the World renders in out/.

THIS IS NOT PHYSICS. Every World was rendered on its own (`render.py`); the
Worlds share no room and do not hear one another. What happens here is a
studio's work outside Ovillo, in the way `~/dev/verabot/film/audio/tools/
ps1_chain.py` is: observation gains and pans per stem, a high-pass on two
stems, a bus limiter and loudness normalization. Nothing here changes a
World's receipt; the stems are written out untouched (gain and pan only) so
the mix can be redone.

    uv run mix.py [--out DIR] [--report]

Targets: -14 LUFS integrated (ITU-R BS.1770-4, via pyloudnorm), true peaks
at or below -1 dBTP (4x oversampled). Writes rule_britannia.wav (48 kHz,
24-bit), stems/*.wav (48 kHz, 24-bit, gain and pan applied, no limiter) and
mix-report.json (every gain, the measurements before and after).
"""
import argparse
import json
import math
from pathlib import Path

import numpy as np
import pyloudnorm as pyln
import soundfile as sf
from scipy.signal import butter, resample_poly, sosfilt

HERE = Path(__file__).resolve().parent
OUT = HERE / "out"
SR = 48000
BPM, TICKS, BEAT = 108, 96, 24
BAR_S = 60.0 / BPM * 4

# ---- the studio's decisions (dB, pan -1..1, high-pass Hz or None), per stem
STEMS = {
    # name:        (file,                        gain_dB, pan,  hpf_Hz)
    "walker":      ("rb-walker.tab.wav",          -1.5,  0.00, None),
    "kit":         ("rb-kit.tab.wav",             -2.0,  0.10, 60.0),
    "vibes":       ("rb-vibes.tab.wav",           +2.5, -0.20, None),
    "piano":       ("rb-piano.tab.wav",           -3.0,  0.30, None),
    "tb1":         ("rb-tb1.trombone.wav",        -1.0, -0.35, None),
    "tb2":         ("rb-tb2.trombone.wav",        -7.0,  0.15, None),
    "tb3":         ("rb-tb3.trombone.wav",        -7.0,  0.45, None),
    "choirlow":    ("rb-choir-low.choir.wav",     +7.0, 0.00, 90.0),
    "choirhigh":   ("rb-choir-high.choir.wav",    +12.0, 0.00, 120.0),
}
# Section gains in time (dB), an automation ride on a stem: (bar_from, bar_to, dB)
RIDES = {"kit": [(1, 15, 4.0), (27, 33, 4.0)],   # the brushes up where they carry the time
         "tb1": [(27, 33, 5.0)]}                      # the tune on the first trombone, verse 2
SECTIONS = [(1, "intro"), (9, "verse 1"), (15, "chorus 1"), (23, "hook"), (27, "verse 2"), (33, "chorus 2"),
            (41, "out"), (53, "chorus 3"), (61, "chorus 4 (Gb)"), (69, "hook (Gb)"), (73, "coda"), (75, "tail")]
# who carries the tune, per section (for the melody-against-the-rest measurement)
LEAD = {"verse 1": ["vibes"], "chorus 1": ["vibes", "tb1"], "hook": ["vibes", "tb1", "piano", "walker"], "verse 2": ["tb1"],
        "chorus 2": ["vibes", "tb1"], "chorus 3": ["vibes", "tb1", "choirhigh"], "chorus 4 (Gb)": ["vibes", "tb1", "choirhigh"], "hook (Gb)": ["vibes", "tb1", "piano", "walker"]}
TARGET_LUFS = -14.0
TARGET_TP_DB = -1.0


def load(name):
    x, sr = sf.read(OUT / name, always_2d=True, dtype="float64")
    assert sr == SR, (name, sr)
    if x.shape[1] == 1:
        x = np.repeat(x, 2, axis=1)
    return x


def pan(x, p):
    """Equal-power pan of a stereo stem (its own stereo image is kept, then tilted)."""
    theta = (p + 1) * math.pi / 4
    l, r = math.cos(theta), math.sin(theta)
    mono = x.mean(axis=1, keepdims=True)
    side = (x - mono)
    return np.hstack([mono * l * math.sqrt(2), mono * r * math.sqrt(2)]) + side


def shelf(x, hz, gain_db, kind):
    """A first-order shelf (bilinear), gain_db above (high) or below (low) hz. Not physics."""
    import math as _m
    g = 10 ** (gain_db / 20)
    k = _m.tan(_m.pi * hz / SR)
    if kind == "high":
        b0, b1, a0, a1 = (k + g), (k - g), (k + 1), (k - 1)
    else:
        b0, b1, a0, a1 = (g * k + 1), (g * k - 1), (k + 1), (k - 1)
    from scipy.signal import lfilter
    return np.column_stack([lfilter([b0 / a0, b1 / a0], [1.0, a1 / a0], x[:, c]) for c in range(x.shape[1])])


# master tone (dB): the Worlds' box radiation is warm (see SPEC-ARIADNE-STEEL-STRING.md,
# "High frequencies cross the bridge weakly"), so the studio tilts the bus: not physics.
MASTER_EQ = [("high", 3000.0, +6.0), ("low", 90.0, -2.0)]
STEM_EQ = {"kit": [("high", 4000.0, +4.0)], "walker": [("low", 70.0, -2.0)]}


def hpf(x, hz):
    sos = butter(2, hz, btype="highpass", fs=SR, output="sos")
    return np.column_stack([sosfilt(sos, x[:, c]) for c in range(x.shape[1])])


def true_peak_db(x):
    y = resample_poly(x, 4, 1, axis=0)
    return 20 * math.log10(max(np.abs(y).max(), 1e-12))


def lufs(x, meter):
    try:
        return float(meter.integrated_loudness(x))
    except ValueError:
        return float("-inf")


def limiter(x, ceiling_db=-1.0, lookahead_s=0.005, release_s=0.08):
    """A look-ahead peak limiter on the 4x-oversampled true peak envelope: a
    gain curve that is the minimum of the needed gain over the look-ahead,
    smoothed on release. Not physics; a studio tool."""
    ceiling = 10 ** (ceiling_db / 20)
    up = resample_poly(x, 4, 1, axis=0)
    env = np.abs(up).max(axis=1)
    n = int(lookahead_s * SR * 4)
    need = np.minimum(1.0, ceiling / np.maximum(env, 1e-12))
    # the minimum over the look-ahead window (so the gain is down before the peak)
    from scipy.ndimage import minimum_filter1d
    g = minimum_filter1d(need, size=max(n, 1), mode="nearest", origin=-(n // 2))
    # attack: ramp down over the look-ahead; release: one-pole up
    a_rel = math.exp(-1.0 / (release_s * SR * 4))
    out = np.empty_like(g)
    cur = 1.0
    for i, v in enumerate(g):
        if v < cur:
            cur = v
        else:
            cur = v + (cur - v) * a_rel
        out[i] = cur
    gain = resample_poly(out, 1, 4)[: len(x)]
    gain = np.minimum(gain, 1.0)
    y = x * gain[:, None]
    return y, float(gain.min())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(OUT / "mix"))
    ap.add_argument("--report", action="store_true", help="measure only; write nothing")
    args = ap.parse_args()
    outdir = Path(args.out)
    meter = pyln.Meter(SR)
    stems = {}
    for name, (file, gain_db, p, hp) in STEMS.items():
        if not (OUT / file).exists():
            print(f"MISSING stem {name}: {file} (not rendered yet)")
            continue
        x = load(file)
        if hp:
            x = hpf(x, hp)
        for kind, hz, db in STEM_EQ.get(name, []):
            x = shelf(x, hz, db, kind)
        x = pan(x, p) * 10 ** (gain_db / 20)
        for (b0, b1, db) in RIDES.get(name, []):
            i0, i1 = int((b0 - 1) * BAR_S * SR), int((b1 - 1) * BAR_S * SR)
            x[i0:i1] *= 10 ** (db / 20)
        stems[name] = x
    n = max(len(x) for x in stems.values())
    mix = np.zeros((n, 2))
    for name, x in stems.items():
        stems[name] = np.vstack([x, np.zeros((n - len(x), 2))])
        mix += stems[name]
    for kind, hz, db in MASTER_EQ:
        mix = shelf(mix, hz, db, kind)
    report = {"schema": "rule-britannia.mix-report.v1", "notPhysics": "observation gains, pans, high-pass filters, shelving EQ, a limiter and loudness normalization applied outside every World",
              "masterEq": MASTER_EQ, "stemEq": STEM_EQ, "rides": RIDES, "stems": {}, "sections": []}
    pre = lufs(mix, meter)
    pre_tp = true_peak_db(mix)
    # normalize to the target loudness, then limit true peaks
    g = 10 ** ((TARGET_LUFS - pre) / 20)
    master, gmin = limiter(mix * g, TARGET_TP_DB)
    post = lufs(master, meter)
    post_tp = true_peak_db(master)
    # a second pass if the limiter pulled the loudness down
    if post < TARGET_LUFS - 0.3:
        master, gmin2 = limiter(mix * g * 10 ** ((TARGET_LUFS - post) / 20), TARGET_TP_DB)
        gmin = min(gmin, gmin2)
        post = lufs(master, meter)
        post_tp = true_peak_db(master)
    report["master"] = {"lufsBeforeNormalization": pre, "truePeakDbBeforeNormalization": pre_tp, "normalizationGainDb": 20 * math.log10(g),
                        "limiterMinimumGainDb": 20 * math.log10(max(gmin, 1e-6)), "lufsIntegrated": post, "truePeakDbtp": post_tp,
                        "seconds": n / SR, "targetLufs": TARGET_LUFS, "targetTruePeakDbtp": TARGET_TP_DB}
    report["missingStems"] = [n for n in STEMS if n not in stems]
    for name, x in stems.items():
        report["stems"][name] = {"file": STEMS[name][0], "gainDb": STEMS[name][1], "pan": STEMS[name][2], "highPassHz": STEMS[name][3],
                                 "lufsIntegrated": lufs(x, meter), "peakDbfs": 20 * math.log10(max(np.abs(x).max(), 1e-12))}
    # per section: each stem's short loudness against the mix's
    for i, (bar, label) in enumerate(SECTIONS):
        end_bar = SECTIONS[i + 1][0] if i + 1 < len(SECTIONS) else None
        i0 = int((bar - 1) * BAR_S * SR)
        i1 = int((end_bar - 1) * BAR_S * SR) if end_bar else n
        if i1 - i0 < SR:
            continue
        row = {"bar": bar, "section": label, "seconds": round(i0 / SR, 2), "mixLufs": lufs(mix[i0:i1] * g, meter)}
        for name, x in stems.items():
            row[name] = lufs(x[i0:i1] * g, meter)
        # the tune against everything else, in the band where it lives (250-1500 Hz, RMS)
        leads = [n for n in LEAD.get(label, []) if n in stems]
        if leads:
            sos = butter(2, [250, 1500], btype="bandpass", fs=SR, output="sos")
            lead = sum(stems[n][i0:i1] for n in leads)
            rest = mix[i0:i1] - lead
            rl = np.sqrt(np.mean(sosfilt(sos, lead.mean(axis=1)) ** 2)) + 1e-12
            rr = np.sqrt(np.mean(sosfilt(sos, rest.mean(axis=1)) ** 2)) + 1e-12
            row["tuneOverRestDb"] = 20 * math.log10(rl / rr)
            row["tuneStems"] = leads
        report["sections"].append(row)
    print(f"mix: {pre:.1f} LUFS, {pre_tp:.1f} dBTP before; normalization {20 * math.log10(g):+.1f} dB; limiter min gain {20 * math.log10(max(gmin, 1e-6)):.1f} dB; "
          f"after {post:.2f} LUFS, {post_tp:.2f} dBTP; {n / SR:.1f} s")
    for name, r in report["stems"].items():
        print(f"  {name:7s} {r['lufsIntegrated']:7.1f} LUFS  peak {r['peakDbfs']:6.1f} dBFS  gain {r['gainDb']:+.1f} pan {r['pan']:+.2f}")
    hdr = "  bar  section        mix   " + " ".join(f"{s:>7s}" for s in stems)
    print(hdr)
    for row in report["sections"]:
        tune = f"  tune/rest {row['tuneOverRestDb']:+5.1f} dB ({','.join(row['tuneStems'])})" if "tuneOverRestDb" in row else ""
        print(f"  {row['bar']:3d}  {row['section']:13s} {row['mixLufs']:6.1f} " + " ".join(f"{row[s]:7.1f}" for s in stems) + tune)
    if args.report:
        return
    outdir.mkdir(parents=True, exist_ok=True)
    (outdir / "stems").mkdir(exist_ok=True)
    sf.write(outdir / "rule_britannia.wav", master, SR, subtype="PCM_24")
    for name, x in stems.items():
        sf.write(outdir / "stems" / f"{name}.wav", np.clip(x * g, -1, 1), SR, subtype="PCM_24")
    (outdir / "mix-report.json").write_text(json.dumps(report, indent=1) + "\n")
    print("wrote", outdir / "rule_britannia.wav")


if __name__ == "__main__":
    main()
