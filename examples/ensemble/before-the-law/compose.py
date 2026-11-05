#!/usr/bin/env python3
"""Write the auditions and the cue for *Before the Law*: MUS scores for the five
tab Worlds (scores/*.mus), note lists for the trombone World
(doorkeeper/*.tsv) and voice lists for the tract choir (choir/*.tsv).

Stdlib only. Everything is composed on one 4/4 grid of 32nd-note ticks per
piece, so the MUS bars and the seconds in the TSVs agree (bar 1 starts at
0 s in `mus render`; checked against each render's touch trace by render.py).
All music here is original.

    python3 compose.py
"""
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCORES = HERE / "scores"
TICKS = 32  # 32nd notes per 4/4 bar
DUR = [(32, "w"), (24, "h."), (16, "h"), (12, "q."), (8, "q"), (6, "e."), (4, "e"), (3, "s."), (2, "s"), (1, "t")]

# ------------------------------------------------------------ instruments
# Each cell has its own hand, as Sunskip's Pocket does, so pos/pick/pm are the
# hand's way of touching that element. (abbrev, name, pitch token, string, pos, pick, pm, gain)
GATE = dict(abbr="gt", title="The Gate (iron parts struck, ringing in the door leaf)", clef="perc",
            plan="gate.plan.json", tuning="D2 Ab2 Eb3 F5 Bb4 A6", hands={
                "panel": ("D2", 6, 0.07, 0.75, 0.0, 0),
                "lbar": ("Ab2", 5, 0.04, 0.95, 0.0, -2),
                "xbar": ("Eb3", 4, 0.04, 0.95, 0.0, -3),
                "bolt": ("F5", 3, 0.11, 0.90, 0.0, -2),
                "lock": ("Bb4", 2, 0.05, 1.00, 0.0, -6),
                "chain": ("A6", 1, 0.03, 1.00, 0.0, -4),
            })
SLAM = dict(abbr="sl", title="The Slam (kettledrums, bass drum, field drum)", clef="perc",
            plan="slam.plan.json", tuning="A1 D2 A2 G3", hands={
                "bd": ("A1", 4, 0.20, 0.20, 0.0, 0),
                "tlo": ("D2", 3, 0.13, 0.30, 0.0, 0),
                "thi": ("A2", 2, 0.13, 0.30, 0.0, -1),
                "fd": ("G3", 1, 0.08, 0.85, 0.0, -4),
            })
LIGHT = dict(abbr="lt", title="The Light, bells (glass bars over tubes)", clef="treble",
             plan="light.plan.json", tuning="D4 F4 G4 A4 C5 D5 F5 A5", hands={
                 "m": (None, None, 0.42, 0.80, 0.0, 0),
             })
FLEAS = dict(abbr="fl", title="The Fleas (six clicks in a fur collar)", clef="perc",
             plan="fleas.plan.json", tuning="C6 D6 E6 F#6 Ab6 Bb6", hands={
                 "f": (None, None, 0.03, 1.00, 0.0, 0),
             })
WAIT = dict(abbr="wt", title="The Waiting (low plucked courses)", clef="bass",
            plan="waiting.plan.json", tuning="D1 A1 D2 G2 C3", hands={
                "o": (None, None, 0.18, 0.25, 0.45, 0),
                "s": (None, None, 0.22, 0.10, 0.00, -2),
            })

LIGHT_STRING = {"A5": 1, "F5": 2, "D5": 3, "C5": 4, "A4": 5, "G4": 6, "F4": 7, "D4": 8}
FLEA_STRING = {"Bb6": 1, "Ab6": 2, "F#6": 3, "E6": 4, "D6": 5, "C6": 6}
# Waiting: open courses, string 1 = C3 ... string 5 = D1.
WAIT_OPEN = {1: 48, 2: 43, 3: 38, 4: 33, 5: 26}
NAMES = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"]


def midi_of(name):
    letter = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}[name[0]]
    rest = name[1:]
    acc = 0
    while rest and rest[0] in "#b":
        acc += 1 if rest[0] == "#" else -1
        rest = rest[1:]
    return 12 * (int(rest) + 1) + letter + acc


def name_of(m):
    return f"{NAMES[m % 12]}{m // 12 - 1}"


def wait_pos(pitch, string):
    fret = midi_of(pitch) - WAIT_OPEN[string]
    assert 0 <= fret <= 12, (pitch, string)
    return f"{pitch}@{string}:{fret}"


def instrument_lines(inst, gain=0):
    out = [f"#   {inst['abbr']} = {inst['title']} ({inst['clef']}, anatomy=../{inst['plan']}, tuning={inst['tuning']}, gain={gain})"]
    for h, (_, _, pos, pick, pm, g) in inst["hands"].items():
        out.append(f"#   {inst['abbr']}.{h} = {h} ({inst['clef']}, pos={pos}, pick={pick}, pm={pm}, gain={g})")
    return out


# ------------------------------------------------------------ the grid

class Part:
    """One hand's events: (bar, tick, length_ticks, token_without_duration, dynamic or None, flags)."""

    def __init__(self):
        self.ev = []

    def add(self, bar, tick, length, body, dyn=None, flags=""):
        self.ev.append((bar, tick, length, body, dyn, flags))
        return self

    def bar(self, b):
        evs = sorted(e for e in self.ev if e[0] == b)
        if not evs:
            return None
        out, now = [], 0
        for i, (_, t, length, body, dyn, flags) in enumerate(evs):
            assert t >= now, (b, t, now, body)
            out += rests(t - now)
            nxt = evs[i + 1][1] if i + 1 < len(evs) else TICKS
            room = min(length, nxt - t)
            d = next((n, s) for n, s in DUR if n <= room)
            if dyn:
                out.append("{" + dyn + "}")
            if body.startswith("<"):
                out.append(f"{body}{d[1]}{flags}")
            else:
                pitch, pos = body.split("@")
                out.append(f"{pitch}{d[1]}@{pos}{flags}")
            now = t + d[0]
        out += rests(TICKS - now)
        return " ".join(out)


def rests(ticks):
    out = []
    for n, s in DUR:
        while ticks >= n:
            out.append("R" + s)
            ticks -= n
    return out


def hit(part, inst, hand, bar, tick, dyn=None, length=1, flags=""):
    pitch, string = inst["hands"][hand][:2]
    part.add(bar, tick, length, f"{pitch}@{string}:0", dyn, flags)


def write_mus(path, title, summary, bpm, bars, insts, parts, texts=(), master=-20):
    lines = [f"# score: {title}", f"# summary: {summary}", f"# tempo: {bpm}    time: 4/4    key: Dm    bars: {bars}",
             f"# master: rms={master}", "# reverb: 0.3", "# tab: standard"]
    lines += [f"# text @b{b}: {t}" for b, t in texts]
    lines += ["", "# instruments:"]
    for inst, gain in insts:
        lines += instrument_lines(inst, gain)
    lines.append("")
    voices = [f"{inst['abbr']}.{h}" for inst, _ in insts for h in inst["hands"]]
    for b in range(1, bars + 1):
        cells = []
        for v in voices:
            p = parts.get(v)
            content = p.bar(b) if p else None
            cells.append(f"{v}={content if content else '-'}")
        lines.append(f"bar {b}: " + "  ".join(cells))
    path.write_text("\n".join(lines) + "\n")
    print("wrote", path.relative_to(HERE))


def seconds(bpm, bar, tick):
    return ((bar - 1) * TICKS + tick) * (60.0 / bpm) / 8.0


def write_trombone(path, bpm, notes):
    """notes: (bar, tick, length_ticks, pitch, vel, vel_end, art, fx)"""
    lines = [f"# Doorkeeper (trombone v1 World). tempo {bpm}; times in seconds from bar 1.",
             "start_s\tend_s\tmidi\tvel\tvel_end\tart\tfx"]
    for bar, tick, length, pitch, vel, vel_end, art, fx in notes:
        s = seconds(bpm, bar, tick)
        e = seconds(bpm, bar, tick + length)
        lines.append(f"{s:.4f}\t{e:.4f}\t{midi_of(pitch)}\t{vel:.2f}\t{vel_end:.2f}\t{art}\t{fx}")
    path.write_text("\n".join(lines) + "\n")
    print("wrote", path.relative_to(HERE))


# The chorus: twelve singers, two per voice type. Each is its own tract World
# with its own anatomy (tract_choir.rs VOICES, scaled by tract_scale and
# fold_scale) and its own manner of singing. The individual numbers are drawn
# once from a fixed seed and written out in choir/singers.tsv, so they can be
# read and edited there.
CAST = [("bass1", "bass", -0.55), ("bass2", "bass", 0.45), ("bari1", "baritone", -0.25), ("bari2", "baritone", 0.65),
        ("ten1", "tenor", 0.20), ("ten2", "tenor", -0.70), ("alto1", "alto", -0.40), ("alto2", "alto", 0.55),
        ("sop1", "soprano", 0.30), ("sop2", "soprano", -0.60), ("child1", "child", -0.15), ("child2", "child", 0.75)]
# Mix gains (the listener's, dB) by voice type, set from the singers' measured RMS.
VOICE_GAIN = {"bass": 0.0, "baritone": 0.0, "tenor": 0.0, "alto": 0.0, "soprano": 1.0, "child": 2.0}


def write_singers(path):
    import random
    rng = random.Random(1998)
    cols = ["singer", "voice", "tract_scale", "fold_scale", "vib_rate_hz", "wobble", "timing_ms", "timing_jitter_ms",
            "intonation_cents", "intonation_jitter_cents", "breath_scale", "tongue_offset", "lips_offset", "pan", "gain_db"]
    lines = ["# The chorus. tract_scale/fold_scale scale the voice type's anatomy (tract_choir.rs VOICES); the rest is",
             "# each singer's manner (timing in ms, intonation in cents, vibrato, breath, vowel shape) and the listener's pan/gain.",
             "# Drawn once with random.Random(1998) in compose.py; edit freely.", "\t".join(cols)]
    for name, voice, pan in CAST:
        row = [name, voice, f"{rng.uniform(0.96, 1.04):.3f}", f"{rng.uniform(0.95, 1.05):.3f}", f"{rng.uniform(4.8, 6.0):.2f}",
               f"{rng.uniform(0.12, 0.40):.2f}", f"{rng.uniform(-25, 30):.1f}", f"{rng.uniform(12, 35):.1f}",
               f"{rng.uniform(-6, 6):.1f}", f"{rng.uniform(2, 6):.1f}", f"{rng.uniform(0.92, 1.08):.3f}",
               f"{rng.uniform(-0.02, 0.02):.3f}", f"{rng.uniform(-0.03, 0.03):.3f}", f"{pan:.2f}", f"{VOICE_GAIN[voice]:.1f}"]
        lines.append("\t".join(row))
    path.write_text("\n".join(lines) + "\n")
    print("wrote", path.relative_to(HERE))


def write_choir(path, bpm, parts):
    """parts: {singer: [(bar, tick, length, pitch, vowel, breath, vib)]}"""
    lines = [f"# Light, choir (tract Worlds). tempo {bpm}; written times in seconds from bar 1 (each singer's own timing is in singers.tsv).",
             "singer\tstart_s\tend_s\tmidi\tvowel\tbreath\tvib_depth"]
    for name, notes in parts.items():
        for bar, tick, length, pitch, vowel, breath, vib in notes:
            lines.append(f"{name}\t{seconds(bpm, bar, tick):.4f}\t{seconds(bpm, bar, tick + length):.4f}\t{midi_of(pitch)}\t{vowel}\t{breath:.2f}\t{vib:.2f}")
    path.write_text("\n".join(lines) + "\n")
    print("wrote", path.relative_to(HERE))


def chords(spans):
    """spans: [(bar, tick, length, vowel, breath, vib, {singer: pitch})] -> {singer: notes}"""
    parts = {}
    for bar, tick, length, vowel, breath, vib, voicing in spans:
        for singer, pitch in voicing.items():
            parts.setdefault(singer, []).append((bar, tick, length, pitch, vowel, breath, vib))
    return parts


# ------------------------------------------------------------ auditions

def gate_audition():
    P = {f"gt.{h}": Part() for h in GATE["hands"]}
    g = lambda h, *a, **k: hit(P[f"gt.{h}"], GATE, h, *a, **k)
    # bar 1: each part once
    g("panel", 1, 0, "p"); g("lbar", 1, 8, "mp"); g("xbar", 1, 16, "mp"); g("lock", 1, 24, "mf")
    # bar 2: the chain rattles, the bolt clunks, the panel answers
    for t, d in [(0, "pp"), (1, "p"), (3, "pp"), (4, "p"), (5, "pp"), (7, "p"), (10, "pp"), (13, "ppp")]:
        g("chain", 2, t, d)
    g("bolt", 2, 16, "mf"); g("panel", 2, 26, "mf")
    # bar 3: the slam - everything at once, then the chain settles
    for h, d in [("panel", "fff"), ("bolt", "ff"), ("lbar", "f"), ("xbar", "f"), ("lock", "f")]:
        g(h, 3, 0, d, flags="[acc]" if h == "panel" else "")
    for t, d in [(3, "p"), (5, "pp"), (6, "p"), (9, "pp"), (14, "ppp"), (21, "ppp")]:
        g("chain", 3, t, d)
    write_mus(SCORES / "gate-audition.mus", "Before the Law: Gate audition", "3 bars / Gate / parts, a rattle, the slam",
              60, 3, [(GATE, 0)], P, texts=[(3, "The slam: every part of the gate at once.")], master=-22)


def slam_audition():
    P = {f"sl.{h}": Part() for h in SLAM["hands"]}
    s = lambda h, *a, **k: hit(P[f"sl.{h}"], SLAM, h, *a, **k)
    s("tlo", 1, 0, "mf"); s("thi", 1, 12, "mp"); s("tlo", 1, 16, "p"); s("bd", 1, 24, "mp")
    # bar 2: a pedal glissando on the low drum (D2 up to A2), then a field-drum roll
    P["sl.tlo"].add(2, 0, 4, "D2@3:0", "f")
    P["sl.tlo"].add(2, 4, 4, "A2@3:7", None, "[sl]")
    for k, t in enumerate(range(16, 32)):
        P["sl.fd"].add(2, t, 1, "G3@1:0", ["pp", "pp", "p", "p", "p", "mp", "mp", "mp", "mf", "mf", "mf", "f", "f", "f", "ff", "ff"][k])
    # bar 3: the hit, then a kettledrum roll swelling on A2
    s("bd", 3, 0, "fff", flags="[acc]"); P["sl.tlo"].add(3, 0, 8, "D2@3:0", "ff")
    for k, t in enumerate(range(8, 32)):
        P["sl.thi"].add(3, t, 1, "A2@2:0", ["pp", "pp", "pp", "p", "p", "p", "p", "mp", "mp", "mp", "mp", "mf",
                                             "mf", "mf", "mf", "f", "f", "f", "f", "ff", "ff", "ff", "ff", "fff"][k])
    s("bd", 4, 0, "ff"); P["sl.tlo"].add(4, 0, 8, "D2@3:0", "ff"); s("fd", 4, 0, "f", flags="[acc]")
    write_mus(SCORES / "slam-audition.mus", "Before the Law: Slam audition", "4 bars / Slam / timpani, pedal glissando, rolls",
              72, 4, [(SLAM, 0)], P, master=-22)


def waiting_pattern(P, bar, alt=False, dyn="mf", low="A1"):
    """The ostinato: eighths grouped 3+3+2 with the accents on the groups."""
    o = P["wt.o"]
    seq = [("D2", 3, "[acc]"), ("D2", 3, ""), ("D2", 3, ""), (low, 4, "[acc]"), ("D2", 3, ""), ("D2", 3, ""),
           ("F2" if not alt else "Eb2", 3, "[acc]"), ("Eb2" if not alt else "D2", 3, "")]
    for k, (p, st, fl) in enumerate(seq):
        o.add(bar, 4 * k, 4, wait_pos(p, st), dyn if k == 0 else None, fl)


def waiting_audition():
    P = {"wt.o": Part(), "wt.s": Part()}
    for b in (1, 2, 3):
        P["wt.s"].add(b, 0, 32, wait_pos("D1", 5), "mp" if b == 1 else None)
        waiting_pattern(P, b, alt=(b == 2), dyn="mf")
    # bar 4: a slide up the D2 course into F2, the sub, and a last accent
    P["wt.o"].add(4, 0, 8, wait_pos("D2", 3), "f", "[acc]")
    P["wt.o"].add(4, 4, 8, wait_pos("F2", 3), None, "[sl]")
    P["wt.o"].add(4, 16, 4, wait_pos("D2", 3), "mf")
    P["wt.s"].add(4, 16, 16, wait_pos("D1", 5), "mf")
    write_mus(SCORES / "waiting-audition.mus", "Before the Law: Waiting audition", "4 bars / Waiting / the ostinato over a sub pedal",
              76, 4, [(WAIT, 0)], P, master=-20)


def bell(P, bar, tick, pitch, dyn=None, length=8):
    P["lt.m"].add(bar, tick, length, f"{pitch}@{LIGHT_STRING[pitch]}:0", dyn)


def chord(P, bar, tick, pitches, dyn=None, length=8):
    body = "<" + " ".join(f"{p}@{LIGHT_STRING[p]}:0" for p in pitches) + ">"
    P["lt.m"].add(bar, tick, length, body, dyn)


def light_audition():
    P = {"lt.m": Part()}
    bell(P, 1, 0, "A4", "mp"); bell(P, 1, 8, "D5"); bell(P, 1, 16, "G4"); bell(P, 1, 24, "C5")
    bell(P, 2, 0, "F5", "mf"); bell(P, 2, 12, "D5", "mp"); bell(P, 2, 16, "A5", "mf")
    chord(P, 3, 0, ["D4", "A4", "D5", "F5"], "mf", 32)
    bell(P, 4, 8, "A5", "p"); bell(P, 4, 20, "D5", "pp")
    write_mus(SCORES / "light-bells-audition.mus", "Before the Law: Light (bells) audition", "4 bars / Light bells / rising fourths, a chord",
              66, 4, [(LIGHT, 0)], P, master=-22)


def fleas_audition():
    P = {"fl.f": Part()}
    f = lambda b, t, p, d=None: P["fl.f"].add(b, t, 1, f"{p}@{FLEA_STRING[p]}:0", d)
    run = [(0, "C6", "p"), (1, "E6", None), (2, "D6", None), (4, "F#6", None), (5, "Bb6", "pp"), (6, "Ab6", None),
           (9, "D6", "p"), (10, "C6", None), (13, "E6", "pp"), (14, "F#6", None), (15, "Ab6", None)]
    for t, p, d in run:
        f(1, t, p, d)
    for t, p, d in [(20, "D6", "mp"), (27, "E6", "p"), (28, "D6", "pp")]:  # D6 and E6 sit on the brass clasp
        f(1, t, p, d)
    for t, p, d in [(0, "Bb6", "pp"), (1, "Ab6", None), (2, "F#6", None), (3, "E6", None), (4, "D6", None), (5, "C6", None),
                    (12, "E6", "mf"), (24, "C6", "ppp")]:
        f(2, t, p, d)
    write_mus(SCORES / "fleas-audition.mus", "Before the Law: Fleas audition", "2 bars / Fleas / skitter, clasp, descent",
              90, 2, [(FLEAS, 0)], P, master=-24)


def doorkeeper_audition():
    bpm = 66
    notes = [
        (1, 0, 24, "D3", 0.15, 0.85, "tongue", "vib"),       # a swell from almost nothing
        (2, 0, 8, "Eb3", 0.55, 0.55, "tongue", "-"),         # the Phrygian sigh
        (2, 8, 16, "D3", 0.55, 0.30, "slur", "vib"),
        (3, 0, 2, "A2", 0.80, 0.80, "tongue", "-"),          # stabs
        (3, 4, 2, "A2", 0.80, 0.80, "tongue", "-"),
        (3, 10, 4, "Bb2", 0.90, 0.90, "tongue", "-"),
        (3, 16, 14, "Bb3", 0.85, 0.85, "tongue", "rip"),     # a rip up
        (4, 0, 8, "A3", 0.6, 0.6, "slur", "-"),
        (4, 8, 20, "F3", 0.55, 0.55, "tongue", "fall"),      # and a fall, for the comedy
    ]
    write_trombone(HERE / "doorkeeper" / "doorkeeper-audition.tsv", bpm, notes)


def choir_audition():
    bpm = 60
    hum = {"bass1": "D2", "bass2": "A2", "bari1": "D3", "bari2": "F3", "ten1": "A3", "ten2": "D4"}
    dmin = dict(hum, alto1="F4", alto2="A4", sop1="D5", sop2="F5", child1="A4", child2="D5")
    bflat = {"bass1": "Bb1", "bass2": "F2", "bari1": "D3", "bari2": "F3", "ten1": "Bb3", "ten2": "D4",
             "alto1": "F4", "alto2": "Bb4", "sop1": "D5", "sop2": "F5", "child1": "Bb4", "child2": "F5"}
    oh = dict(hum, alto1="F4", alto2="A4", sop1="A4", sop2="D5", child1="F4", child2="D5")
    parts = chords([(1, 0, 16, "m", 0.50, 0.06, hum),
                    (1, 16, 30, "a", 0.56, 0.12, dmin),
                    (2, 16, 16, "a", 0.58, 0.14, bflat),
                    (3, 0, 30, "o", 0.52, 0.12, oh)])
    write_choir(HERE / "choir" / "choir-audition.tsv", bpm, parts)


def alert():
    """A short alert sting: field-drum pickup, then brass, metal, kettle and glass together."""
    bpm = 120
    P = {f"gt.{h}": Part() for h in GATE["hands"]}
    P.update({f"sl.{h}": Part() for h in SLAM["hands"]})
    P["lt.m"] = Part()
    P["sl.fd"].add(1, 28, 1, "G3@1:0", "mf").add(1, 30, 1, "G3@1:0", "f")
    for h, d in [("xbar", "ff"), ("lbar", "f"), ("lock", "f")]:
        hit(P[f"gt.{h}"], GATE, h, 2, 0, d, flags="[acc]")
    hit(P["sl.bd"], SLAM, "bd", 2, 0, "ff"); hit(P["sl.thi"], SLAM, "thi", 2, 0, "ff")
    hit(P["sl.fd"], SLAM, "fd", 2, 0, "ff", flags="[acc]")
    chord(P, 2, 0, ["G4", "C5", "F5", "A5"], "f", 32)
    for t, d in [(2, "p"), (4, "pp"), (7, "ppp")]:
        hit(P["gt.chain"], GATE, "chain", 2, t, d)
    write_mus(SCORES / "alert.mus", "Before the Law: alert sting", "2 bars at 120 / a pickup and one hit",
              bpm, 2, [(GATE, 8), (SLAM, -3), (LIGHT, -10)], P, master=-20)
    write_trombone(HERE / "doorkeeper" / "alert.tsv", bpm, [
        (1, 28, 2, "F3", 0.7, 0.7, "tongue", "-"),
        (2, 0, 10, "Bb3", 0.95, 0.95, "tongue", "-"),
        (2, 12, 14, "A3", 0.8, 0.3, "smear", "fall"),
    ])


# ------------------------------------------------------------ the cue

CUE_BPM = 76
CUE_BARS = 10


def cue():
    """Instrument gains (mix, not physics) were set from each World's radiated
    power in the energy trace (render.py's balance study): Slam -20, Light -10,
    Fleas +17 dB against the Waiting.

    'Waiting before the gate', ~35 s. D minor with a Phrygian Eb.
    1-2 the ostinato alone over the sub, a distant clang; 3-4 the doorkeeper
    breathes, light leaks through; 5-6 the choir hums then opens, the ostinato
    leans on Eb; 7 the music thins - the fleas; 8-9 the build on A, a kettle
    roll, the choir on Bb then A; 10 the gate answers, and rings out."""
    P = {f"wt.{h}": Part() for h in WAIT["hands"]}
    P.update({f"gt.{h}": Part() for h in GATE["hands"]})
    P.update({f"sl.{h}": Part() for h in SLAM["hands"]})
    P["lt.m"] = Part()
    P["fl.f"] = Part()
    # The Waiting
    for b in range(1, 10):
        if b == 7:
            P["wt.o"].add(7, 0, 4, wait_pos("D2", 3), "p", "[acc]")
            P["wt.o"].add(7, 12, 4, wait_pos("A1", 4), "pp")
            continue
        dyn = {1: "mp", 3: "mf", 5: "mf", 8: "mf"}.get(b)
        sub = {8: "Bb1", 9: "A1"}.get(b, "D1")
        waiting_pattern(P, b, alt=b in (5, 6), dyn=dyn, low=sub if sub != "D1" else "A1")
        # bars 8-9: the pedal moves to the A1 course (Bb1 is its first fret), which the ostinato shares
        P["wt.s"].add(b, 0, 12 if sub != "D1" else 32, wait_pos(sub, 4 if sub != "D1" else 5), "mp" if b in (1, 8) else None)
    P["wt.o"].add(10, 0, 8, wait_pos("D2", 3), "f", "[acc]")
    P["wt.s"].add(10, 0, 32, wait_pos("D1", 5), "f")
    P["wt.o"].add(10, 8, 4, wait_pos("D2", 3), "mp")
    P["wt.o"].add(10, 12, 4, wait_pos("D2", 3), "p")
    # The Gate: a distant clang, a rattle, then it answers
    hit(P["gt.panel"], GATE, "panel", 1, 0, "pp")
    for t, d in [(24, "ppp"), (25, "pp"), (27, "ppp"), (29, "pp"), (31, "ppp")]:
        hit(P["gt.chain"], GATE, "chain", 4, t, d)
    hit(P["gt.lock"], GATE, "lock", 7, 24, "p")
    for t, d in [(16, "pp"), (17, "pp"), (19, "p"), (20, "pp"), (22, "p"), (24, "p"), (25, "mp"), (27, "p"), (29, "mp"), (31, "mp")]:
        hit(P["gt.chain"], GATE, "chain", 9, t, d)
    for h, d in [("panel", "fff"), ("bolt", "ff"), ("lbar", "f"), ("xbar", "f")]:
        hit(P[f"gt.{h}"], GATE, h, 10, 0, d, flags="[marc]" if h == "panel" else "[acc]")
    for t, d in [(3, "p"), (6, "pp"), (11, "ppp")]:
        hit(P["gt.chain"], GATE, "chain", 10, t, d)
    # The Slam
    for b in (1, 3, 5):
        P["sl.tlo"].add(b, 0, 8, "D2@3:0", "p")
    P["sl.thi"].add(5, 16, 8, "A2@2:0", "pp")
    P["sl.fd"].add(7, 4, 1, "G3@1:0", "pp").add(7, 5, 1, "G3@1:0", "p")
    P["sl.tlo"].add(8, 0, 2, "D2@3:0", "mp")
    P["sl.tlo"].add(8, 2, 4, "Eb2@3:1", None, "[sl]")  # the pedal leans up a semitone
    roll = ["pp", "pp", "pp", "p", "p", "p", "p", "mp", "mp", "mp", "mp", "mf", "mf", "mf", "mf", "f"]
    for k, t in enumerate(range(16, 32)):
        P["sl.thi"].add(8, t, 1, "A2@2:0", roll[k // 2])
    for k, t in enumerate(range(0, 32)):
        P["sl.thi"].add(9, t, 1, "A2@2:0", (roll + ["f"] * 16)[min(31, 12 + k // 2)])
    for k, t in enumerate(range(24, 32)):
        P["sl.fd"].add(9, t, 1, "G3@1:0", ["p", "p", "mp", "mp", "mf", "mf", "f", "f"][k])
    P["sl.bd"].add(10, 0, 8, "A1@4:0", "fff", "[acc]")
    P["sl.tlo"].add(10, 0, 8, "D2@3:0", "ff")
    P["sl.thi"].add(10, 16, 8, "A2@2:0", "p")
    P["sl.tlo"].add(10, 24, 8, "D2@3:0", "pp")
    # The Light (bells): light through the cracks
    for b, t, p, d in [(3, 0, "A4", "p"), (3, 12, "D5", None), (3, 24, "G4", None), (4, 4, "C5", "p"), (4, 16, "F5", None),
                       (4, 26, "A5", "pp"), (6, 0, "D5", "p"), (6, 8, "F5", None), (6, 16, "A5", None), (6, 24, "F5", "pp")]:
        bell(P, b, t, p, d)
    chord(P, 10, 0, ["D4", "A4", "D5", "F5"], "mf", 16)
    bell(P, 10, 20, "A5", "pp")
    # The Fleas, in bar 7, when everything else draws back
    for t, p, d in [(8, "C6", "p"), (9, "E6", None), (10, "D6", None), (12, "F#6", "pp"), (13, "Bb6", None), (14, "Ab6", None),
                    (17, "D6", "p"), (18, "C6", None), (20, "E6", "mp"), (28, "Bb6", "pp"), (29, "Ab6", None), (30, "F#6", None)]:
        P["fl.f"].add(7, t, 1, f"{p}@{FLEA_STRING[p]}:0", d)
    write_mus(SCORES / "cue-waiting.mus", "Before the Law: Waiting before the gate (tab Worlds)",
              f"{CUE_BARS} bars at {CUE_BPM} / Waiting, Gate, Slam, Light bells, Fleas (the trombone and choir are their own Worlds)",
              CUE_BPM, CUE_BARS, [(WAIT, 0), (GATE, -2), (SLAM, -20), (LIGHT, -10), (FLEAS, 17)], P,
              texts=[(1, "The ostinato alone; the gate is far away."), (3, "The doorkeeper breathes; light through the cracks."),
                     (5, "Hum, then open. Lean on the E-flat."), (7, "Everything draws back. Fleas."),
                     (8, "Build on A."), (10, "The gate answers.")], master=-19)
    # The Doorkeeper
    write_trombone(HERE / "doorkeeper" / "cue-waiting.tsv", CUE_BPM, [
        (3, 0, 60, "D3", 0.10, 0.70, "tongue", "vib"),
        (5, 0, 28, "Eb3", 0.55, 0.40, "tongue", "vib"),
        (6, 0, 28, "D3", 0.40, 0.25, "slur", "vib"),
        (8, 0, 3, "A2", 0.80, 0.80, "tongue", "-"),
        (8, 6, 3, "A2", 0.75, 0.75, "tongue", "-"),
        (8, 16, 46, "Bb2", 0.30, 0.90, "tongue", "vib"),
        (10, 0, 14, "D3", 0.95, 0.60, "tongue", "-"),
        (10, 16, 14, "D3", 0.35, 0.10, "tongue", "dim"),
    ])
    # The Light (choir): men and altos hum, then open; sopranos join on Bb;
    # the children only at the A major of bar 9, the radiance at its brightest.
    low = {"bass1": "D2", "bass2": "A2", "bari1": "D3", "bari2": "F3"}
    open_d = dict(low, ten1="A3", ten2="D4", alto1="F4", alto2="A4")
    bflat = {"bass1": "Bb1", "bass2": "F2", "bari1": "D3", "bari2": "F3", "ten1": "Bb3", "ten2": "D4",
             "alto1": "F4", "alto2": "Bb4", "sop1": "D5", "sop2": "F5"}
    amaj = {"bass1": "A1", "bass2": "E2", "bari1": "C#3", "bari2": "E3", "ten1": "A3", "ten2": "C#4",
            "alto1": "E4", "alto2": "A4", "sop1": "C#5", "sop2": "E5", "child1": "A4", "child2": "E5"}
    oh = dict(open_d, sop1="A4", sop2="D5", child1="F4", child2="D5")
    S = chords([(5, 0, 16, "m", 0.48, 0.06, dict(low, alto1="A3", alto2="D4")),
                (5, 16, 46, "a", 0.55, 0.12, open_d),
                (8, 0, 30, "a", 0.57, 0.13, bflat),
                (9, 0, 30, "a", 0.60, 0.14, amaj),
                (10, 0, 30, "o", 0.54, 0.12, oh)])
    write_choir(HERE / "choir" / "cue-waiting.tsv", CUE_BPM, S)


if __name__ == "__main__":
    SCORES.mkdir(exist_ok=True)
    gate_audition()
    slam_audition()
    waiting_audition()
    light_audition()
    fleas_audition()
    doorkeeper_audition()
    write_singers(HERE / "choir" / "singers.tsv")
    choir_audition()
    alert()
    cue()
