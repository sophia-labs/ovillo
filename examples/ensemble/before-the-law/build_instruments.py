#!/usr/bin/env python3
"""Five fictional Ariadne tab anatomies for *Before the Law* (Vera's farewell film).

Stdlib only. Like Sunskip's (`examples/ensemble/sunskip/`), these are ordinary
canonical `WorldPlan` JSON built on the steel-string plan's structure: bodies
named `string/N` are the struck or plucked elements the score touches, three
bodies named `box/0..2` are the radiators they drive through reciprocal
`bridge/N/band` edges, and the plan's shared space is the room. `mus render`
plays them with `anatomy=<file>.plan.json`.

Every number this script changes is a design hypothesis. Where a mode family
comes from a textbook law (a free-free beam, a simply supported plate, an ideal
membrane, a thin ring), the law is named in the recipe; the dimensions and
losses fed to it are chosen, not measured. Nothing here claims to reconstruct a
real gate, contrabass, celesta, drum or flea.

    python3 build_instruments.py        # writes <name>.plan.json + <name>.design.json
"""
from copy import deepcopy
import hashlib
import json
import math
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
BASE = ROOT / "profiles/ariadne-steel-string-v0.plan.json"
N = 24              # MODE_COUNT: every Ariadne body holds 24 modes
CAP = 21600.0       # stay under the runtime band (0.48 x 48 kHz = 23040 Hz) with margin
TWO_PI = 2.0 * math.pi

# --------------------------------------------------------------- mode laws

# Free-free Euler-Bernoulli beam: beta_n L, ratio (beta_n / beta_1)^2.
BEAM_BL = [4.73004, 7.85320, 10.99561, 14.13717, 17.27876] + [(2 * n + 1) * math.pi / 2 for n in range(5, 40)]
BEAM = [(b / BEAM_BL[0]) ** 2 for b in BEAM_BL]

# Zeros j_{m,n} of the Bessel function J_m (ideal circular membrane, fixed rim).
BESSEL = {
    (0, 1): 2.40483, (0, 2): 5.52008, (0, 3): 8.65373, (0, 4): 11.79153, (0, 5): 14.93092,
    (1, 1): 3.83171, (1, 2): 7.01559, (1, 3): 10.17347, (1, 4): 13.32369,
    (2, 1): 5.13562, (2, 2): 8.41724, (2, 3): 11.61984, (2, 4): 14.79595,
    (3, 1): 6.38016, (3, 2): 9.76102, (3, 3): 13.01520,
    (4, 1): 7.58834, (4, 2): 11.06471, (4, 3): 14.37254,
    (5, 1): 8.77148, (5, 2): 12.33860,
    (6, 1): 9.93611, (6, 2): 13.58929,
    (7, 1): 11.08637, (7, 2): 14.82127,
    (8, 1): 12.22509, (9, 1): 13.35430, (10, 1): 14.47550, (11, 1): 15.58985,
}


def ring(n):
    """In-plane flexural mode n >= 2 of a thin ring, relative to n = 2."""
    g = lambda k: k * (k * k - 1) / math.sqrt(k * k + 1)
    return g(n) / g(2)


def plate(a, b, count, mmax=40):
    """Simply supported a x b plate: (ratio to f11, m, n), lowest first."""
    base = (1 / a) ** 2 + (1 / b) ** 2
    modes = [(((m / a) ** 2 + (n / b) ** 2) / base, m, n) for m in range(1, mmax) for n in range(1, mmax)]
    return sorted(modes)[:count]


def t60(f, s0, s2):
    """T60 from a two-term loss rate sigma = s0 + s2 omega^2 (the steel-string's law)."""
    return 6.91 / (s0 + s2 * (TWO_PI * f) ** 2)


def norm(v):
    s = math.sqrt(sum(x * x for x in v))
    assert s > 1e-12
    return [x / s for x in v]


def midi_of(f):
    return 69 + 12 * math.log2(f / 440.0)


NAMES = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"]


def note_name(m):
    m = round(m)
    return f"{NAMES[m % 12]}{m // 12 - 1}"


def hz(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def halton(i, base):
    f, r = 1.0, 0.0
    while i > 0:
        f /= base
        r += f * (i % base)
        i //= base
    return r


# ------------------------------------------------------------- the elements
# A cell is one `string/N` body: what the score touches. `freq`/`t60` are its
# 24 modes at configuration amount 0; `fretted` doubles every mode at amount 1
# (the steel-string convention: fret k = amount k/12), otherwise both ends are
# equal and the score uses fret 0 only. `couple(j)` is the cell-side direction
# of its edge to box j (the mode shapes where it joins the radiator).

def cell(name, family, freq, t60s, couple, fretted=False, note=""):
    assert len(freq) == N and len(t60s) == N, (name, len(freq))
    top = max(freq) * (2 if fretted else 1)
    assert top < CAP, (name, top)
    assert all(t > 0 for t in t60s)
    return dict(name=name, family=family, freq=freq, t60=t60s, couple=couple, fretted=fretted, note=note)


def alternating(scale=0.0):
    """End of a free bar / a string at its termination: every mode equal, alternating sign."""
    return lambda j: norm([(-1) ** i * (1 + scale * i) for i in range(N)])


def plate_point(modes, x, y):
    return lambda j: norm([math.sin(m * math.pi * x) * math.sin(n * math.pi * y) + 1e-3 for _, m, n in modes])


def scattered(seed):
    """Deterministic irregular signs and sizes (an element touching at many points)."""
    return lambda j: norm([(1 if halton(i + 1 + seed, 2) > 0.5 else -1) * (0.4 + halton(i + 1 + seed + 7 * j, 3)) for i in range(N)])


def strung(midi, B, s0, s2, fretted=True):
    f1 = hz(midi)
    freq = [f1 * n * math.sqrt((1 + B * n * n) / (1 + B)) for n in range(1, N + 1)]
    return freq, [t60(f, s0, s2) for f in freq]


# ------------------------------------------------------------- radiators
# A box is a `box/N` body. The tab engine makes boxes radiate (0.02 of their
# projection per sample, rising with the box's mean mode frequency) and the
# cells almost not at all, so what is heard is the boxes and the room.

def box(name, freq, t60s, radiation, attach):
    """`attach(cell_index)` -> the box-side direction of that cell's edge."""
    assert len(freq) == N and all(0 < f < CAP for f in freq), (name, freq)
    return dict(name=name, freq=freq, t60=t60s, radiation=norm(radiation), attach=attach)


def plate_box(name, a, b, f11, band, s0, s2, points, sample="log"):
    """24 modes of a large simply supported plate in a frequency band (sampled
    evenly in log frequency when the band holds more): a radiating leaf or
    panel. Radiation: odd-odd modes pump net volume; all modes get an edge term
    growing toward 1 at 2.5 kHz; times frequency (pressure is the time
    derivative of volume velocity) - the steel-string box's rule, applied here."""
    lo, hi = band
    modes = [(f11 * r, m, n) for r, m, n in plate(a, b, 6000, mmax=120) if lo <= f11 * r <= hi]
    assert len(modes) >= N, (name, len(modes))
    if sample == "log":
        picks = []
        for k in range(N):
            target = math.exp(math.log(lo) + (k + 0.5) / N * (math.log(hi) - math.log(lo)))
            best = min((m for m in modes if m not in picks), key=lambda m: abs(math.log(m[0] / target)))
            picks.append(best)
        modes = sorted(picks)
    else:
        modes = modes[:N]
    freq = [f for f, _, _ in modes]
    vol = [((1 - (-1) ** m) * (1 - (-1) ** n)) / (m * n * math.pi ** 2) for _, m, n in modes]
    rad = [(v + 0.08 * min(f / 2500.0, 1.0)) * (f / 1000.0) for v, (f, _, _) in zip(vol, modes)]
    attach = lambda c: norm([math.sin(m * math.pi * points[c][0]) * math.sin(n * math.pi * points[c][1]) + 1e-3 for _, m, n in modes])
    return box(name, freq, [t60(f, s0, s2) for f in freq], rad, attach)


def scaled_baseline_boxes(baseline, fscale, tscale, names):
    out = []
    for i, b in enumerate(baseline):
        left = b["body"]["configuration_path"]["left"]
        freq = [f * fscale for f in left["metric"]["frequency_hz"]]
        t = [x * tscale for x in left["metric"]["t60_seconds"]]
        out.append(dict(name=names[i], freq=freq, t60=t, radiation=left["radiation_direction"], attach=None, baseline_index=i))
    return out


# ------------------------------------------------------------- instruments

def gate():
    """THE GATE: riveted iron parts of a door, struck, driving the door leaf."""
    cells = []
    # Leaf panel (string 6): one riveted panel between ribs, 0.62 x 0.45 m,
    # simply supported; f11 set to D2.
    pm = plate(0.62, 0.45, N)
    f11 = hz(38)
    freq = [f11 * r for r, _, _ in pm]
    cells.append(cell("panel", "simply supported plate 0.62 x 0.45 (Leissa/Kirchhoff)", freq,
                      [t60(f, 1.2, 3.0e-9) for f in freq], plate_point(pm, 0.03, 0.47), note="the door's riveted panel"))
    # Long and cross bars (strings 5, 4): free-free square bars; bending in two
    # planes, split by an assumed 0.4 % asymmetry -> 12 pairs.
    for name, midi in (("long bar", 44), ("cross bar", 51)):
        f1 = hz(midi)
        freq = sorted([f1 * BEAM[k] * s for k in range(12) for s in (1.0, 1.004)])
        cells.append(cell(name, "free-free beam, two bending planes split 0.4 %", freq,
                          [t60(f, 1.5, 5.0e-9) for f in freq], alternating(), note="a grille bar"))
    # Bolt (string 3): a 0.51 m steel bolt; flattened, so its two bending planes
    # split 1.5 %; plus longitudinal (5.1 kHz) and torsional (3.2 kHz) series;
    # it sits in its keeper (heavy contact loss).
    f1 = hz(77)  # F5
    fam = [f1 * BEAM[k] for k in range(7)] + [f1 * BEAM[k] * 1.015 for k in range(7)]
    fam += [5100.0 * k for k in range(1, 5)] + [3200.0 * k for k in range(1, 7)]
    freq = sorted(fam)[:N]
    cells.append(cell("bolt", "free-free beam (2 planes) + longitudinal + torsional series", freq,
                      [t60(f, 18.0, 2.0e-8) for f in freq], alternating(0.02), note="the great bolt"))
    # Lock plate (string 2): 0.30 x 0.22 m plate, f11 set to Bb4.
    pm2 = plate(0.30, 0.22, N)
    f11 = hz(70)
    freq = [f11 * r for r, _, _ in pm2]
    cells.append(cell("lock plate", "simply supported plate 0.30 x 0.22", freq,
                      [t60(f, 4.0, 1.0e-8) for f in freq], plate_point(pm2, 0.5, 0.06), note="the lock's plate"))
    # Chain (string 1): eight links of a hanging chain, each a thin ring
    # (in-plane n = 2, 3 and an out-of-plane n = 2 at x1.06), sizes spread
    # 0.85-1.18; links clash, so the loss is high.
    f2 = hz(96)  # C7
    sizes = [1.0, 1.04, 1.09, 1.13, 0.97, 1.18, 0.93, 1.07]
    freq = sorted(f2 / s * r for s in sizes for r in (1.0, 1.06, ring(3)))
    cells.append(cell("chain", "thin rings (n = 2 in and out of plane, n = 3), 8 links", freq,
                      [t60(f, 25.0, 1.0e-8) for f in freq], scattered(3), note="the gate chain"))
    cells = list(reversed(cells))  # string 1 = highest-pitched (the chain)
    # The leaf: 2.4 x 1.4 m door leaf, a large plate (f11 ~ 10 Hz, its modes far
    # denser than 72), sampled in three bands. Long steel ring; the engine's box
    # radiation shortens it further.
    points = [(0.85, 0.20), (0.90, 0.55), (0.93, 0.47), (0.50, 0.18), (0.12, 0.50), (0.31, 0.42)]
    boxes = [plate_box("leaf low", 2.4, 1.4, 9.84, (38, 420), 0.9, 4.0e-9, points),
             plate_box("leaf mid", 2.4, 1.4, 9.84, (420, 2300), 0.9, 4.0e-9, points),
             plate_box("leaf high", 2.4, 1.4, 9.84, (2300, 9000), 0.9, 4.0e-9, points)]
    return dict(name="gate", world="before-the-law-gate-v1", cells=cells, boxes=boxes,
                room_scale=4.5, position=-0.25, bridge_rate=[1.2, 1.2, 1.2], bridge_weight=1.0,
                intention="Industrial metal percussion: the gate's panel, bars, bolt, lock and chain, struck; their energy rings in the door leaf and a stone room. The final slam is a gesture on it.")


def waiting():
    """THE WAITING: low plucked courses (a scordatura contrabass-like body)."""
    tuning = [26, 33, 38, 43, 48]  # D1 A1 D2 G2 C3, low to high
    cells = []
    for m in tuning:
        freq, tt = strung(m, 6.0e-5, 1.1, 3.5e-7)
        cells.append(cell(f"course {note_name(m)}", "stiff string f_n = n f1 sqrt((1+Bn^2)/(1+B)), B = 6e-5",
                          freq, tt, alternating(), fretted=True))
    cells = list(reversed(cells))
    return dict(name="waiting", world="before-the-law-waiting-v1", cells=cells, boxes="baseline:0.754:0.45",
                room_scale=3.0, position=0.05, bridge_rate=[0.9, 0.9, 0.9], bridge_weight=1.0,
                intention="Low string ostinato and a sub pedal: five plucked low courses (D1 A1 D2 G2 C3) on a large wooden body. Pizzicato, not bowed.")


def light():
    """THE LIGHT (bells): eight glass bars over tuned tube resonators."""
    bars = [62, 65, 67, 69, 72, 74, 77, 81]  # D4 F4 G4 A4 C5 D5 F5 A5
    rt, wt = 1.35, 3.2  # torsional/bending and width/thickness ratios of the bar section (design)
    cells = []
    for m in bars:
        f1 = hz(m)
        # Undercut (arched) bars, tuned 1 : 4 : 10 as vibraphone and marimba bars
        # are; above the third mode the free-free series is scaled to continue
        # from 10 (an invented continuation). The undercut lowers torsion too.
        tuned = [1.0, 4.0, 10.0] + [10.0 * BEAM[k] / BEAM[2] for k in range(3, 20)]
        fam = [(f1 * r, 0) for r in tuned] + [(f1 * wt * r, 1) for r in BEAM[:8]] + [(f1 * rt * k, 2) for k in range(1, 30)]
        fam = sorted((f, kind) for f, kind in fam if f < CAP)
        assert len(fam) >= N, (m, len(fam))
        # The 24 lowest modes, ordered by family: vertical bending, then lateral
        # bending, then torsion. The engine weights a hand's touch by mode index
        # (sin(n pi pos)/n), so this order is the physics of a mallet on the
        # bar's centre line: torsion has a nodal line there and is barely struck.
        keep = sorted(fam[:N], key=lambda fk: (fk[1], fk[0]))
        freq = [f for f, _ in keep]
        reach = [(1.0, 0.4, 0.1)[kind] for _, kind in keep]
        cells.append(cell(f"bar {note_name(m)}", "undercut glass bar tuned 1:4:10, then lateral bending and torsion (ratio 1.35), in that index order",
                          freq, [t60(f, 0.9, 1.6e-8) for f in freq],
                          lambda j, reach=reach: norm([(-1) ** i * r for i, r in enumerate(reach)])))
    cells = list(reversed(cells))  # string 1 = A5
    # box/0: the eight resonator tubes (closed-open: modes f, 3f, 5f), one per bar,
    # Q 150 each. A bar meets its own tube; the others barely.
    tubes = []
    for k, m in enumerate(reversed(bars)):
        for h in (1, 3, 5):
            tubes.append((hz(m) * h, k, h))
    tubes.sort()
    tfreq = [f for f, _, _ in tubes]
    tt = [6.91 * 150 / (math.pi * f) for f in tfreq]
    trad = [1.0 / h for _, _, h in tubes]
    tattach = lambda c: norm([(1.0 / h if k == c else 0.02) for _, k, h in tubes])
    resonators = box("resonator tubes", tfreq, tt, trad, tattach)
    case = scaled_baseline_boxes(BASE_BOXES, 1.35, 0.8, ["case", "case", "case"])
    boxes = [resonators, case[1], case[2]]
    boxes[1]["name"], boxes[2]["name"] = "case mid", "case high"
    return dict(name="light", world="before-the-law-light-v1", cells=cells, boxes=boxes,
                room_scale=3.5, position=0.35, bridge_rate=[0.35, 0.15, 0.15], bridge_weight=1.0,
                intention="Glassy bells for the radiance: eight glass bars (D4 F4 G4 A4 C5 D5 F5 A5) struck hard over tuned tubes, celesta-like.")


def fleas():
    """THE FLEAS: six tiny chitin clicks in a fur collar with a brass clasp."""
    tuning = [84, 86, 88, 90, 92, 94]  # C6 D6 E6 F#6 G#6 A#6: a whole-tone flea scale
    cells = []
    for i, m in enumerate(tuning):
        f1 = hz(m)
        top = min(10.5, (CAP - 200) / f1)
        ratios = [1.0] + sorted(1.0 + (top - 1.0) * (k + 0.6 * halton(k + 3 * i + 1, 2)) / (N - 0.4) for k in range(1, N))
        freq = [f1 * r for r in ratios]
        cells.append(cell(f"flea {i + 1}", "invented dense shell cluster (no law)", freq,
                          [t60(f, 260.0, 4.0e-8) for f in freq], scattered(11 * i + 5)))
    cells = list(reversed(cells))
    # The collar: fur over leather (very lossy, low-mid), the brass clasp (rings),
    # the coat's leather (lossy). All three are invented modal sets.
    fur = [180 * (1 + 0.62 * k + 0.018 * k * k) for k in range(N)]
    clasp = [1650 * (1 + 0.23 * k + 0.006 * k * k) for k in range(N)]
    coat = [420 * (1 + 0.4 * k + 0.012 * k * k) for k in range(N)]
    reach = [[0.9, 0.08, 0.3], [0.7, 0.6, 0.2], [0.9, 0.05, 0.4], [0.6, 0.7, 0.2], [0.9, 0.1, 0.3], [0.8, 0.3, 0.3]]
    boxes = [box("fur collar", fur, [t60(f, 110.0, 2.0e-7) for f in fur], [math.sqrt(f / 1000) for f in fur], scattered(1)),
             box("brass clasp", clasp, [t60(f, 4.0, 6.0e-9) for f in clasp], [(-1) ** k * math.sqrt(f / 1000) for k, f in enumerate(clasp)], scattered(2)),
             box("coat leather", coat, [t60(f, 60.0, 1.0e-7) for f in coat], [math.sqrt(f / 1000) for f in coat], scattered(4))]
    return dict(name="fleas", world="before-the-law-fleas-v1", cells=cells, boxes=boxes, reach=reach,
                room_scale=1.0, position=0.45, bridge_rate=[1.0, 1.0, 1.0], bridge_weight=1.0,
                intention="The fleas in the doorkeeper's fur collar: six tiny hard clicks, muffled by fur, two of them landing on a brass clasp.")


def slam():
    """THE SLAM: field percussion - bass drum, two pedal kettledrums, a field drum."""
    def membrane(f01, s0, s2, exclude=()):
        modes = sorted((j / BESSEL[(0, 1)], mn) for mn, j in BESSEL.items() if mn not in exclude)[:N]
        freq = [f01 * r for r, _ in modes]
        return freq, [t60(f, s0, s2) for f in freq], modes

    cells = []
    # Field drum (string 1): ideal membrane, f01 = G3; short. No snare wires.
    freq, tt, modes = membrane(hz(55), 9.0, 6.0e-7)
    cells.append(cell("field drum", "ideal membrane (Bessel zeros), no snare wires", freq, tt,
                      lambda j, modes=modes: norm([0.5 + 0.5 * (mn[0] == 0) for _, mn in modes])))
    # Kettledrums (strings 2, 3): index 0 is the principal (1,1); (2,1)..(5,1)
    # at the air-loaded ratios 1.50, 1.98, 2.44, 2.94 commonly quoted from
    # Fletcher & Rossing; (6,1) at 3.42 and (0,1) at 0.85, heavily damped (it
    # pumps the kettle's air), are design hypotheses; the rest ideal-membrane
    # ratios to (1,1). The pedal is the
    # configuration path (tension: every mode x2 at amount 1); scores use 0-7.
    for name, midi in (("timpano high", 45), ("timpano low", 38)):
        f11 = hz(midi)
        fam = [(1.00, 3.2), (1.50, 2.6), (1.98, 2.1), (2.44, 1.6), (2.94, 1.1), (3.42, 0.8), (0.85, 0.25)]
        rest = sorted((BESSEL[mn] / BESSEL[(1, 1)], mn) for mn in BESSEL if mn[1] >= 2 or mn[0] >= 7)
        fam += [(r, 0.45 / (1 + 0.1 * k)) for k, (r, _) in enumerate(rest[: N - len(fam)])]
        freq = [f11 * r for r, _ in fam]
        cells.append(cell(name, "kettledrum: (m,1) air-loaded ratios + ideal membrane", freq,
                          [t for _, t in fam], lambda j: norm([1.0 if k < 6 else 0.35 for k in range(N)]), fretted=True))
    # Bass drum (string 4): ideal membrane, f01 = A1.
    freq, tt, modes = membrane(hz(33), 3.0, 1.0e-6)
    cells.append(cell("bass drum", "ideal membrane (Bessel zeros), one head", freq, tt,
                      lambda j, modes=modes: norm([1.0 if mn[0] == 0 else 0.4 for _, mn in modes])))
    return dict(name="slam", world="before-the-law-slam-v1", cells=cells, boxes="baseline:0.5:1.0",
                room_scale=4.0, position=-0.05, bridge_rate=[1.0, 1.0, 1.0], bridge_weight=1.0,
                intention="Timpani, bass drum and a field drum (rolls without snare wires) on a wooden riser in a big stone room; the weight under the final slam.")


# ------------------------------------------------------------- assembly

BASELINE = json.loads(BASE.read_bytes())
BASE_STRINGS = [b for b in BASELINE["bodies"] if b["id"].startswith("string/")]
BASE_BOXES = [b for b in BASELINE["bodies"] if b["id"].startswith("box/")]


def build(spec):
    plan = deepcopy(BASELINE)
    plan["world_name"] = spec["world"]
    plan["bodies"] = []
    plan["direct_couplings"] = []
    boxes = spec["boxes"]
    if isinstance(boxes, str):
        _, fs, ts = boxes.split(":")
        boxes = scaled_baseline_boxes(BASE_BOXES, float(fs), float(ts), ["body low", "body mid", "body high"])
    receipts = []
    for idx, c in enumerate(spec["cells"]):
        number = idx + 1
        body = deepcopy(BASE_STRINGS[0])
        body["id"] = f"string/{number}"
        body["body"]["anatomy_name"] = f"{spec['name']}-{c['name'].replace(' ', '-')}"
        path = body["body"]["configuration_path"]
        if not c["fretted"]:
            path["coordinate"] = "fixed_modal_geometry"
            path["left_name"] = path["right_name"] = "rest"
        elif spec["name"] == "slam":
            path["coordinate"] = "membrane_tension"
            path["left_name"], path["right_name"] = "pedal_up", "pedal_octave"
        path["initial_amount"] = 0.0
        for end, mult in (("left", 1.0), ("right", 2.0 if c["fretted"] else 1.0)):
            path[end]["metric"]["frequency_hz"] = [f * mult for f in c["freq"]]
            path[end]["metric"]["t60_seconds"] = list(c["t60"])
        body["pose"]["position"][0] = spec["position"] + (number % 2 - 0.5) * 0.12
        plan["bodies"].append(body)
        for band in range(3):
            edge = deepcopy(next(e for e in BASELINE["direct_couplings"] if e["id"] == f"bridge/6/{band}"))
            edge["id"] = f"bridge/{number}/{band}"
            edge["left_body"] = body["id"]
            edge["right_body"] = f"box/{band}"
            edge["rate_hz"] *= spec["bridge_rate"][band]
            if "reach" in spec:
                edge["weight"] = spec["reach"][idx][band]
            if callable(c["couple"]):
                edge["left_direction"] = c["couple"](band)
            if boxes[band].get("attach"):
                edge["right_direction"] = boxes[band]["attach"](idx)
            plan["direct_couplings"].append(edge)
        f1 = c["freq"][0]
        receipts.append({
            "string": number, "element": c["name"], "family": c["family"], "note": c["note"],
            "tuningToken": note_name(midi_of(f1)), "firstModeHz": round(f1, 3),
            "centsFromToken": round(100 * (midi_of(f1) - round(midi_of(f1))), 2),
            "highestModeHz": round(max(c["freq"]) * (2 if c["fretted"] else 1), 1),
            "t60FirstModeS": round(c["t60"][0], 3), "t60LastModeS": round(c["t60"][-1], 4),
            "fretted": c["fretted"],
            "modesHz": [round(f, 2) for f in c["freq"]],
        })
    for band, b in enumerate(boxes):
        body = deepcopy(BASE_BOXES[band])
        body["body"]["anatomy_name"] = f"{spec['name']}-{b['name'].replace(' ', '-')}"
        body["pose"]["position"][0] = spec["position"]
        for end in ("left", "right"):
            cfg = body["body"]["configuration_path"][end]
            cfg["metric"]["frequency_hz"] = list(b["freq"])
            cfg["metric"]["t60_seconds"] = list(b["t60"])
            cfg["radiation_direction"] = list(b["radiation"])
        plan["bodies"].append(body)
    plan["shared_space"]["delays_seconds"] = [d * spec["room_scale"] for d in BASELINE["shared_space"]["delays_seconds"]]
    for b in plan["bodies"]:
        for end in ("left", "right"):
            cfg = b["body"]["configuration_path"][end]
            assert len(cfg["metric"]["frequency_hz"]) == N and all(0 < f < CAP for f in cfg["metric"]["frequency_hz"]), b["id"]
            assert all(t > 0 for t in cfg["metric"]["t60_seconds"]), b["id"]
            assert abs(sum(x * x for x in cfg["radiation_direction"]) - 1) < 1e-9, b["id"]
    path = HERE / f"{spec['name']}.plan.json"
    path.write_text(json.dumps(plan, indent=2) + "\n")
    tuning = " ".join(r["tuningToken"] for r in reversed(receipts))
    design = {
        "schema": "before-the-law.instrument-design.v1",
        "name": spec["name"], "worldName": spec["world"], "classification": "design-hypothesis",
        "intention": spec["intention"],
        "baseline": str(BASE.relative_to(ROOT)),
        "baselineFileSha256": hashlib.sha256(BASE.read_bytes()).hexdigest(),
        "planFileSha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "scoreTuning": tuning,
        "lossLaw": "T60 = 6.91 / (s0 + s2 (2 pi f)^2) unless a family says otherwise",
        "roomDelayScale": spec["room_scale"],
        "bridgeRateScale": spec["bridge_rate"],
        "cells": receipts,
        "boxes": [{"box": i, "name": b["name"], "lowestHz": round(min(b["freq"]), 1), "highestHz": round(max(b["freq"]), 1),
                   "t60RangeS": [round(min(b["t60"]), 3), round(max(b["t60"]), 3)]} for i, b in enumerate(boxes)],
        "inherited": "Baseline fields not listed (contact apertures before the hands overwrite them, active directions, transport, reservoirs, space ports, the room's scattering and listening directions) are the steel-string plan's, verbatim.",
        "engineOverrides": "mus-engine apply_hands replaces each cell's source direction with sin(n pi pos)/n by mode index, sets its contact port from the hand's pick, adds the hand's palm loss, and retunes a cell whose rounded first-mode pitch differs from the score's tuning token (so the score must use scoreTuning). Cells radiate 2e-6, boxes 0.02 x max(mean mode Hz / 1 kHz, 1).",
    }
    (HERE / f"{spec['name']}.design.json").write_text(json.dumps(design, indent=2) + "\n")
    print(f"{spec['name']:8s} tuning={tuning:32s} {len(plan['bodies'])} bodies, {len(plan['direct_couplings'])} edges, sha256 {design['planFileSha256'][:12]}")
    return design


if __name__ == "__main__":
    for make in (gate, waiting, light, fleas, slam):
        build(make())
