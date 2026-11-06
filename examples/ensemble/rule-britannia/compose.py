#!/usr/bin/env python3
"""*Rule, Britannia!* (Thomas Arne, 1740; words James Thomson) for Vera's
farewell film: an original arrangement for the three new tab Worlds here
(walker, vibes, kit), the Ariadne piano, three trombone v1 Worlds (a small
section) and the twelve-singer tract choir of *Before the Law* singing
doo-wop syllables.

Stdlib only. Everything sits on one grid of 96 ticks per 4/4 bar (24 per
beat), so swung eighths are triplet tokens (`q3 e3`: 16 + 8 ticks), straight
sixteenths are 6 ticks, and the MUS bars and the seconds in the trombone and
choir lists agree (bar 1 starts at 0 s in `mus render`).

The melody was taken from two public-domain transcriptions and cross-checked
(see README.md, "The melody"): where they differ, the choice is noted at
MELODY below. The harmony, the hook, the form and every part are original.

    python3 compose.py       # writes scores/*.mus, brass/*.tsv, choir/*.tsv
"""
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "before-the-law"))
from compose import midi_of, name_of, NAMES, CAST, write_singers  # noqa: E402

SCORES = HERE / "scores"
TICKS = 96  # per 4/4 bar: 24 per beat
BEAT = 24
# token durations in ticks, largest first (the bar writer is greedy)
DUR = [(96, "w"), (72, "h."), (48, "h"), (36, "q."), (24, "q"), (18, "e."), (16, "q3"), (12, "e"), (9, "s."), (8, "e3"), (6, "s"), (4, "s3"), (3, "t"), (2, "t3")]
BPM = 108
BARS = 74


def sw(beat, half):
    """Swung eighths: the beat's first eighth at 0, its second at 16 of 24."""
    return beat * BEAT + (16 if half else 0)


# ------------------------------------------------------------ instruments

WALK = dict(abbr="wk", title="The Walker (upright bass)", clef="bass", plan="walker.plan.json", tuning="E1 A1 D2 G2",
            hands={"f": (None, None, 0.30, 0.05, 0.12, 0)})
WALK_OPEN = {1: 43, 2: 38, 3: 33, 4: 28}
VIBES = dict(abbr="vb", title="The Vibes (bars over tubes)", clef="treble", plan="vibes.plan.json",
             tuning="C4 C#4 D4 Eb4 E4 F4 F#4 G4 Ab4 A4 Bb4 B4 C5 C#5 D5 Eb5 E5 F5 F#5 G5 Ab5 A5",
             hands={"m": (None, None, 0.42, 0.80, 0.0, 0)})
VIBES_LOW, VIBES_HIGH = 60, 81
KIT = dict(abbr="kt", title="The Kit (kick, snare, cross-stick, hi-hat, ride)", clef="perc", plan="kit.plan.json",
           tuning="F1 G3 G3 C4 C4 F4", hands={
               "kick": ("F1", 6, 0.20, 0.20, 0.0, 0),
               "sn": ("G3", 5, 0.12, 0.85, 0.0, 0),
               "br": ("G3", 5, 0.30, 0.15, 0.0, -10),    # the brush: a soft, round contact on the snare head (observation gain -10 dB: see README)
               "ride": ("G3", 4, 0.10, 0.90, 0.0, 0),
               "hho": ("C4", 3, 0.08, 0.90, 0.0, 0),
               "hhc": ("C4", 2, 0.08, 0.90, 0.0, 0),
               "rim": ("F4", 1, 0.05, 1.00, 0.0, 0),
           })
PIANO_LINES = ["#   pn = grand piano (treble, anatomy=piano-v1)", "#   pn.l = left hand (bass)", "#   pn.r = right hand (treble)",
               "#   pn.ped = sustain pedal (perc, pedal=sustain)"]


def vibes_string(m):
    assert VIBES_LOW <= m <= VIBES_HIGH, name_of(m)
    return VIBES_HIGH - m + 1


def walk_pos(m):
    """Lowest-fret position for a bass pitch (fret 0..12)."""
    for s in (4, 3, 2, 1):
        fret = m - WALK_OPEN[s]
        if 0 <= fret <= 12:
            return f"{name_of(m)}@{s}:{fret}"
    raise ValueError(name_of(m))


def instrument_lines(inst, gain=0):
    out = [f"#   {inst['abbr']} = {inst['title']} ({inst['clef']}, anatomy=../{inst['plan']}, tuning={inst['tuning']}, gain={gain})"]
    for h, (_, _, pos, pick, pm, g) in inst["hands"].items():
        out.append(f"#   {inst['abbr']}.{h} = {h} ({inst['clef']}, pos={pos}, pick={pick}, pm={pm}, gain={g})")
    return out


# ------------------------------------------------------------ the grid

class Part:
    """One hand's events: (bar, tick, length_ticks, body, dynamic or None, flags)."""

    def __init__(self):
        self.ev = []

    def add(self, bar, tick, length, body, dyn=None, flags=""):
        while tick >= TICKS:
            bar, tick = bar + 1, tick - TICKS
        assert 1 <= bar <= BARS, (bar, body)
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
            # the longest token that fits, whose remainder up to the next event rests can spell
            d = next((n, s) for n, s in DUR if n <= room and rest_tokens(nxt - t - n) is not None)
            if dyn:
                out.append("{" + dyn + "}")
            if "@" in body and not body.startswith("<"):
                pitch, pos = body.split("@")
                out.append(f"{pitch}{d[1]}@{pos}{flags}")
            else:  # a chord (positions inside), a piano key or the pedal's X
                out.append(f"{body}{d[1]}{flags}")
            now = t + d[0]
        out += rests(TICKS - now)
        return " ".join(out)


def rest_tokens(ticks):
    """The fewest rest tokens spelling `ticks`, or None if they cannot."""
    best = {0: []}
    for k in range(1, ticks + 1):
        cands = [best[k - n] + [s] for n, s in DUR if n <= k and best.get(k - n) is not None]
        best[k] = min(cands, key=len) if cands else None
    return best[ticks]


def rests(ticks):
    toks = rest_tokens(ticks)
    assert toks is not None, ticks
    return ["R" + s for s in toks]


def write_mus(path, title, summary, insts, parts, piano=False, texts=(), master=-20, bars=BARS):
    lines = [f"# score: {title}", f"# summary: {summary}", f"# tempo: {BPM}    time: 4/4    key: F    bars: {bars}",
             f"# master: rms={master}", "# reverb: 0.0", "# tab: standard"]
    lines += [f"# text @b{b}: {t}" for b, t in texts]
    lines += ["", "# instruments:"]
    for inst, gain in insts:
        lines += instrument_lines(inst, gain)
    if piano:
        lines += PIANO_LINES
    lines.append("")
    voices = [f"{inst['abbr']}.{h}" for inst, _ in insts for h in inst["hands"]]
    if piano:
        voices += ["pn.l", "pn.r", "pn.ped"]
    for b in range(1, bars + 1):
        cells = []
        for v in voices:
            p = parts.get(v)
            content = p.bar(b) if p else None
            cells.append(f"{v}={content if content else '-'}")
        lines.append(f"bar {b}: " + "  ".join(cells))
    path.write_text("\n".join(lines) + "\n")
    print("wrote", path.relative_to(HERE))


def seconds(bar, tick):
    return ((bar - 1) * TICKS + tick) * (60.0 / BPM) / BEAT


def write_trombone(path, name, notes):
    """notes: (bar, tick, length_ticks, midi, vel, vel_end, art, fx); sorted here."""
    lines = [f"# {name} (trombone v1 World). tempo {BPM}; times in seconds from bar 1.", "start_s\tend_s\tmidi\tvel\tvel_end\tart\tfx"]
    rows = []
    for bar, tick, length, m, vel, vel_end, art, fx in notes:
        rows.append((seconds(bar, tick), seconds(bar, tick + length), m, vel, vel_end, art, fx))
    rows.sort()
    for i in range(1, len(rows)):
        assert rows[i][0] >= rows[i - 1][1] - 1e-9, (name, rows[i - 1], rows[i])  # one voice: no overlaps
    for s, e, m, vel, vel_end, art, fx in rows:
        lines.append(f"{s:.4f}\t{e:.4f}\t{m}\t{vel:.2f}\t{vel_end:.2f}\t{art}\t{fx}")
    path.write_text("\n".join(lines) + "\n")
    print("wrote", path.relative_to(HERE), len(rows), "notes")


def write_choir(path, parts):
    """parts: {singer: [(bar, tick, length, midi, vowel, breath, vib, glide_s)]}"""
    lines = [f"# The choir, doo-wop (tract Worlds). tempo {BPM}; written times in seconds from bar 1 (each singer's own timing is in singers.tsv).",
             "# glide_s: how long the articulation takes to move from the previous vowel (a hum's sealed lips released fast is a 'b' or 'd').",
             "singer\tstart_s\tend_s\tmidi\tvowel\tbreath\tvib_depth\tglide_s"]
    n = 0
    for name, notes in parts.items():
        notes = sorted(notes)
        for i in range(1, len(notes)):
            a, b = notes[i - 1], notes[i]
            assert seconds(b[0], b[1]) >= seconds(a[0], a[1] + a[2]) - 1e-9, (name, a, b)
        for bar, tick, length, m, vowel, breath, vib, glide in notes:
            lines.append(f"{name}\t{seconds(bar, tick):.4f}\t{seconds(bar, tick + length):.4f}\t{m}\t{vowel}\t{breath:.2f}\t{vib:.2f}\t{glide:.3f}")
            n += 1
    path.write_text("\n".join(lines) + "\n")
    print("wrote", path.relative_to(HERE), n, "notes")


# ------------------------------------------------------------ the tune
# Bars 1-6 the verse ("When Britain first, at Heaven's command ..."), 7-14 the
# chorus ("Rule, Britannia! ..."), in F major, (tick, length, midi) per bar,
# with a pickup eighth before bars 1, 3, 4, 5(none), 7, 8(none), 9 ... as the
# sources have it. Eighths are swung (16 + 8), sixteenth runs straight (6).
# Sources: ABC "Rule Britannia" (D major, 4/4, L:1/8; abcnotation.com mirror of
# code.google.com/Rule_Britannia_1) and the CC0 MIDI "Rule, Britannia (pg).mid"
# on Wikimedia Commons (G major). They agree bar for bar on the pitches except
# at two places, both taken from the ABC: bar 5's fourth eighth (C4 here; the
# MIDI has E4) and the pickup to bar 9 (C4; the MIDI has F4). Bar 10's turn
# (G A G F in sixteenths) is the ABC's; the MIDI simplifies it to two quarters.
F4, G4, A4, Bb4, C5, D5 = 65, 67, 69, 70, 72, 74
C4, D4, E4, Bb3, B4 = 60, 62, 64, 58, 71
MELODY = {
    0: [(88, 8, C4)],
    1: [(0, 24, F4), (24, 24, F4), (48, 6, F4), (54, 6, G4), (60, 6, A4), (66, 6, Bb4), (72, 16, C5), (88, 8, F4)],
    2: [(0, 36, G4), (36, 6, A4), (42, 6, Bb4), (48, 24, A4), (88, 8, C4)],
    3: [(0, 6, F4), (6, 6, G4), (12, 6, F4), (18, 6, G4), (24, 6, A4), (30, 6, Bb4), (36, 6, A4), (42, 6, Bb4), (48, 16, C5), (64, 8, G4), (72, 16, A4), (88, 8, G4)],
    4: [(0, 16, F4), (16, 4, G4), (20, 4, A4), (24, 16, G4), (40, 8, F4), (48, 36, E4), (88, 8, C4)],
    5: [(0, 16, E4), (16, 8, C4), (24, 16, G4), (40, 8, C4), (48, 16, C5), (64, 4, B4), (68, 4, A4), (72, 6, G4), (78, 6, F4), (84, 6, E4), (90, 6, D4)],
    6: [(0, 24, C4), (24, 16, D4), (40, 8, C4), (48, 24, C4)],
    7: [(0, 24, F4), (24, 16, F4), (40, 8, C4), (48, 16, D4), (64, 8, Bb3), (88, 8, F4)],
    8: [(0, 16, Bb4), (16, 8, A4), (24, 16, G4), (40, 8, F4), (48, 24, E4), (88, 8, C4)],
    9: [(0, 24, C5), (24, 24, Bb4), (48, 6, A4), (54, 6, F4), (60, 6, Bb4), (66, 6, G4), (72, 16, C5), (88, 8, Bb4)],
    10: [(0, 24, A4), (24, 6, G4), (30, 6, A4), (36, 6, G4), (42, 6, F4), (48, 48, F4)],
    11: [(0, 24, A4), (40, 8, A4), (48, 16, Bb4), (64, 8, Bb4), (88, 8, A4)],
    12: [(0, 16, Bb4), (16, 8, A4), (24, 16, G4), (40, 8, F4), (48, 48, E4)],
}
MELODY[13] = MELODY[9]
MELODY[14] = MELODY[10]
VERSE, CHORUS = range(1, 7), range(7, 15)

# Harmony, per tune bar: (tick, length, chord). A chord is (root midi class,
# piano voicing (midi, middle register), the walking bass's scale degrees to
# aim for after the root). Extended, with tritone substitutions on the
# dominants (Gb13 for C7) and a chromatic plane Ab -> Gb -> F into the chorus.
Fb = 64
CH = {
    "Fmaj9":   (5, [57, 60, 64, 67], [69, 72]),           # A C E G
    "F69":     (5, [57, 60, 62, 67], [69, 72]),           # A C D G
    "F13":     (5, [57, 63, 64, 67], [69, 63]),           # A Eb E? no: A Eb G D
    "Dm9":     (2, [57, 60, 64, 65], [65, 69]),           # A C E F
    "Gm9":     (7, [58, 62, 65, 69], [70, 62]),           # Bb D F A
    "Gm11":    (7, [58, 60, 65, 69], [70, 62]),
    "Gb13#9":  (6, [58, 64, 69, 73], [70, 64]),           # Bb Fb A(#9) Db
    "Gb13#11": (6, [58, 64, 68, 72], [70, 64]),           # Bb Fb Ab C(#11)
    "Am9":     (9, [60, 64, 67, 71], [64, 67]),
    "C7b9":    (0, [64, 67, 70, 73], [64, 67]),
    "C13b9":   (0, [64, 69, 70, 73], [64, 67]),
    "C7alt":   (0, [64, 70, 73, 68], [64, 68]),           # E Bb Db Ab
    "Gm13":    (7, [58, 64, 65, 69], [70, 62]),
    "Abmaj7#11": (8, [60, 63, 67, 74], [63, 67]),         # C Eb G D
    "Bbmaj9":  (10, [62, 65, 69, 72], [65, 62]),
    "Bbmaj13": (10, [62, 65, 67, 69], [65, 62]),
    "D7b9":    (2, [66, 69, 72, 75], [66, 69]),
    "Ab13":    (8, [60, 66, 67, 65], [63, 66]),           # C Gb G? no: C Gb F Bb -> use C Gb Bb F
    "Eb9":     (3, [55, 61, 65, 67], [58, 61]),
    "Fmaj9#11": (5, [57, 59, 64, 67], [69, 71]),          # A B E G
}
CH["F13"] = (5, [57, 63, 67, 62], [69, 63])               # A Eb G D
CH["Ab13"] = (8, [60, 66, 70, 65], [63, 66])              # C Gb Bb F
HARMONY = {
    1: [(0, 48, "Fmaj9"), (48, 48, "Dm9")],
    2: [(0, 48, "Gm9"), (48, 48, "Gb13#9")],
    3: [(0, 48, "Fmaj9"), (48, 48, "Am9")],
    4: [(0, 48, "Dm9"), (48, 24, "Gm9"), (72, 24, "C7b9")],
    5: [(0, 48, "Gm13"), (48, 48, "C7alt")],
    6: [(0, 48, "Abmaj7#11"), (48, 48, "Gb13#11")],
    7: [(0, 48, "F69"), (48, 48, "Bbmaj13")],
    8: [(0, 48, "Gm9"), (48, 48, "Gb13#11")],
    9: [(0, 48, "Fmaj9"), (48, 48, "Bbmaj9")],
    10: [(0, 24, "Gm9"), (24, 24, "C13b9"), (48, 48, "F69")],
    11: [(0, 24, "Am9"), (24, 24, "D7b9"), (48, 48, "Gm9")],
    12: [(0, 48, "Gm11"), (48, 48, "Gb13#11")],
    13: [(0, 48, "Fmaj9"), (48, 48, "Bbmaj9")],
    14: [(0, 24, "Gm9"), (24, 24, "C7alt"), (48, 48, "Fmaj9")],
}
# The hook: two bars, a riff on the chorus head ("Rule, Bri-tan-nia") with a
# blue answer, over F13 | Ab13 Gb13#11 (the plane again).
HOOK = {0: [(0, 24, 53), (24, 16, 53), (40, 8, 48), (48, 16, 50), (64, 8, 46), (88, 8, 53)],
        1: [(0, 16, 56), (16, 8, 58), (24, 24, 60), (64, 8, 63), (72, 16, 60)]}
HOOK_HARMONY = {0: [(0, 96, "F13")], 1: [(0, 48, "Ab13"), (48, 48, "Gb13#11")]}

# ------------------------------------------------------------ the form
# (first bar, kind, transposition in semitones)
FORM = [(1, "intro", 0), (9, "verse", 0), (15, "chorus", 0), (23, "hook", 0), (27, "verse", 0), (33, "chorus", 0),
        (41, "out", 0), (53, "chorus", 0), (61, "chorus", 1), (69, "hook", 1), (73, "coda", 1)]
TEXTS = [(1, "Intro: the hook over the groove, brushes."), (9, "Verse 1: the tune on the vibes."), (15, "Chorus 1: horns under the vibes; the choir 'bah'."),
         (23, "The hook, everybody."), (27, "Verse 2: the trombone sings it; 'doo' pads."), (33, "Chorus 2: tutti."),
         (41, "Out: seven-eighth ostinato, clusters, the choir slides."), (53, "Chorus 3, warm: the sopranos and children sing it."),
         (61, "Chorus 4, up a semitone: G-flat."), (69, "Hook, in G-flat."), (73, "The last chord rings.")]


def sections():
    out = []
    for i, (bar, kind, tr) in enumerate(FORM):
        end = FORM[i + 1][0] if i + 1 < len(FORM) else BARS + 1
        out.append((bar, end, kind, tr))
    return out


def chord_at(kind, start, bar, tick):
    """The chord sounding at (bar, tick) in a verse/chorus/hook section, or None."""
    if kind in ("verse", "chorus"):
        tb = (bar - start) + (1 if kind == "verse" else 7)
        table = HARMONY
    elif kind in ("hook", "intro"):
        tb = (bar - start) % 2
        table = HOOK_HARMONY
    else:
        return None
    if tb not in table:
        return None
    for t, n, name in table[tb]:
        if t <= tick < t + n:
            return name
    return None


def chord_spans(kind, start, end, tr):
    """[(bar, tick, length, chord name, transposition)] over a section."""
    out = []
    for bar in range(start, end):
        if kind in ("verse", "chorus"):
            tb = (bar - start) + (1 if kind == "verse" else 7)
            table = HARMONY
        else:
            tb = (bar - start) % 2
            table = HOOK_HARMONY
        for t, n, name in table.get(tb, []):
            out.append((bar, t, n, name, tr))
    return out


def tune(kind, start, end, tr):
    """The melody events of a verse/chorus section, (bar, tick, length, midi), with pickups."""
    out = []
    first = 1 if kind == "verse" else 7
    for bar in range(start, end):
        tb = bar - start + first
        for t, n, m in MELODY.get(tb, []):
            out.append((bar, t, n, m + tr))
    if kind == "verse":
        for t, n, m in MELODY[0]:
            out.append((start - 1, t, n, m + tr))
    return out


def hook_line(start, end, tr):
    out = []
    for bar in range(start, end):
        for t, n, m in HOOK[(bar - start) % 2]:
            out.append((bar, t, n, m + tr))
    return out


# ------------------------------------------------------------ the parts

def nearest(pc, lo, hi, around):
    """The pitch of class pc in [lo, hi] nearest `around`."""
    cands = [m for m in range(lo, hi + 1) if m % 12 == pc % 12]
    return min(cands, key=lambda m: abs(m - around))


def bass_part(P, secs):
    """The Walker: walking quarters through the changes; the hook in unison;
    a seven-eighth ostinato in the out section."""
    f = P["wk.f"]
    prev = 41  # F2
    for start, end, kind, tr in secs:
        if kind in ("verse", "chorus", "intro", "hook"):
            spans = chord_spans(kind, start, end, tr)
            if kind in ("intro", "hook"):
                # the hook line in unison, an octave below the vibes
                for bar, t, n, m in hook_line(start, end, tr):
                    dyn = "mf" if t == 0 and bar == start else None
                    f.add(bar, t, n, walk_pos(m - 12), dyn, "[acc]" if t == 0 else "")
                prev = 41 + tr
                continue
            for i, (bar, t, n, name, tr_) in enumerate(spans):
                root_pc, _, aims = CH[name]
                root = nearest(root_pc + tr, 28, 50, prev)
                nxt_root = nearest(CH[spans[i + 1][3]][0] + tr, 28, 50, root) if i + 1 < len(spans) else nearest(5 + tr, 28, 50, root)
                beats = n // BEAT
                line = [root]
                if beats >= 2:
                    aim = nearest(aims[0] + tr, 28, 50, root)
                    if beats == 2:
                        # root, then a chromatic approach to the next root
                        step = 1 if nxt_root > root else -1
                        line.append(nxt_root - step if abs(nxt_root - root) > 1 else aim)
                    else:
                        line += [aim, nearest(aims[1] + tr, 28, 50, aim)]
                        step = 1 if nxt_root > line[-1] else -1
                        line.append(nxt_root - step if abs(nxt_root - line[-1]) > 1 else root + 7 if root + 7 <= 50 else root - 5)
                for k, m in enumerate(line[:beats]):
                    m = min(max(m, 28), 55)
                    dyn = "mf" if (bar == start and t == 0 and k == 0) else None
                    f.add(bar, t + k * BEAT, BEAT, walk_pos(m), dyn)
                    prev = m
                # skip-step: a swung eighth before the next bar's downbeat, now and then
                if beats == 2 and t == 48 and (bar * 7) % 3 == 0:
                    f.add(bar, 88, 8, walk_pos(min(max(prev + (2 if (bar % 2) else -1), 28), 55)))
        elif kind == "out":
            # bars 41: a lone low D; 42-51: the ostinato in seven eighths
            # (D D A D F E D), crossing the bar lines; 52: the fill bar, tacet.
            f.add(start, 0, 48, walk_pos(38), "f", "[acc]")
            riff = [38, 38, 33, 38, 41, 40, 38]
            tick = (start + 1 - 1) * TICKS
            k = 0
            while tick < (start + 11 - 1) * TICKS:
                bar, t = divmod(tick, TICKS)
                m = riff[k % 7]
                f.add(bar + 1, t, 12, walk_pos(m), "mf" if k == 0 else None, "[acc]" if k % 7 == 0 else "")
                tick += 12
                k += 1
            prev = 38
        elif kind == "coda":
            f.add(start, 0, 24, walk_pos(30), "f", "[acc]")     # Gb1
            f.add(start, 48, 24, walk_pos(30 + 12), "mf")
            f.add(start + 1, 0, 48, walk_pos(30), "ff", "[acc]")


def kit_part(P, secs):
    k = {h: P[f"kt.{h}"] for h in KIT["hands"]}

    def brushes(bar, until=TICKS):
        # a sweep: the brush's round contact drawn across the head as a run of
        # soft strokes every sixteenth-triplet, swelling into 2 and 4
        for t in range(0, until, 4):
            beat = t // BEAT
            on = t % BEAT == 0
            dyn = ("p" if beat in (1, 3) else "pp") if on else ("ppp" if t % BEAT in (4, 8, 12) else "pp")
            k["br"].add(bar, t, 4, "G3@5:0", dyn)
        k["hhc"].add(bar, 24, 12, "C4@2:0", "p").add(bar, 72, 12, "C4@2:0", "p")
        k["kick"].add(bar, 0, 12, "F1@6:0", "pp").add(bar, 48, 12, "F1@6:0", "pp")

    def ride_time(bar, loud="mf", kick_acc=False):
        for t, d in [(0, loud), (24, "mp"), (40, "mp"), (48, loud), (72, "mp"), (88, "mp")]:
            k["ride"].add(bar, t, 8, "G3@4:0", d)
        k["hhc"].add(bar, 24, 12, "C4@2:0", "mp").add(bar, 72, 12, "C4@2:0", "mp")
        k["kick"].add(bar, 0, 12, "F1@6:0", "mf" if kick_acc else "p").add(bar, 48, 12, "F1@6:0", "p")

    def comp(bar, pattern):
        for t, d in pattern:
            k["sn"].add(bar, t, 6, "G3@5:0", d)

    def fill(bar, start_tick=48, dyn=("mp", "mp", "mf", "mf", "f", "f", "ff", "ff")):
        n = (TICKS - start_tick) // 6
        for i in range(n):
            t = start_tick + i * 6
            k["sn"].add(bar, t, 6, "G3@5:0", dyn[min(i * len(dyn) // n, len(dyn) - 1)])
        k["kick"].add(bar, TICKS - 12, 12, "F1@6:0", "f")

    for start, end, kind, tr in secs:
        if kind == "intro":
            for bar in range(start, end):
                brushes(bar, 72 if bar == end - 1 else TICKS)
            k["rim"].add(start + 3, 72, 6, "F4@1:0", "mp")
            k["rim"].add(start + 7, 48, 6, "F4@1:0", "mp").add(start + 7, 72, 6, "F4@1:0", "mp")
            fill(end - 1, 72)
        elif kind == "verse":
            for bar in range(start, end):
                brushes(bar)
                if (bar - start) % 2 == 1:
                    k["rim"].add(bar, 72, 6, "F4@1:0", "mp")
            k["hho"].add(end - 1, 88, 8, "C4@3:0", "mf")
        elif kind == "chorus":
            big = start >= 53
            for bar in range(start, end):
                ride_time(bar, "f" if big else "mf", kick_acc=(bar - start) % 4 == 0)
                if bar < end - 1:
                    comp(bar, [(40, "mp"), (72, "p")] if (bar - start) % 2 == 0 else [(16, "p"), (64, "mp"), (88, "mp")])
                if big and (bar - start) % 4 == 3:
                    k["hho"].add(bar, 88, 8, "C4@3:0", "f")
            fill(end - 1, 48)
        elif kind == "hook":
            for bar in range(start, end):
                ride_time(bar, "f", kick_acc=True)
                # the hook's rhythm on the snare and kick: with the line
                for t in (0, 24, 40, 48, 64):
                    k["sn"].add(bar, t, 6, "G3@5:0", "mf" if t in (0, 48) else "mp")
                k["kick"].add(bar, 88, 8, "F1@6:0", "mf")
            fill(end - 1, 72)
        elif kind == "out":
            # 41: the kit alone, a ride wash and a cross-stick; 42-51 the seven:
            # ride every eighth, kick on 1 and 4 of the seven, snare on 6.
            k["ride"].add(start, 0, 8, "G3@4:0", "f", "[acc]")
            k["hho"].add(start, 0, 8, "C4@3:0", "f")
            for t in (36, 60, 84):
                k["rim"].add(start, t, 6, "F4@1:0", "mp")
            tick = start * TICKS
            j = 0
            while tick < (start + 10) * TICKS:
                bar, t = divmod(tick, TICKS)
                bar += 1
                k["ride"].add(bar, t, 8, "G3@4:0", "mf" if j % 7 == 0 else "mp")
                if j % 7 in (0, 3):
                    k["kick"].add(bar, t, 12, "F1@6:0", "mf")
                if j % 7 == 5:
                    k["sn"].add(bar, t, 6, "G3@5:0", "mf")
                if j % 7 == 0 and (j // 7) % 2 == 1:
                    k["hhc"].add(bar, t, 12, "C4@2:0", "mp")
                tick += 12
                j += 1
            fill(end - 1, 0, ("p", "mp", "mp", "mf", "mf", "f", "ff", "fff"))
            k["hho"].add(end - 1, 84, 8, "C4@3:0", "f")
        elif kind == "coda":
            k["kick"].add(start, 0, 12, "F1@6:0", "ff").add(start + 1, 0, 12, "F1@6:0", "fff")
            k["sn"].add(start, 0, 6, "G3@5:0", "ff").add(start + 1, 0, 6, "G3@5:0", "fff")
            k["hho"].add(start, 0, 8, "C4@3:0", "ff").add(start + 1, 0, 8, "C4@3:0", "fff")
            k["ride"].add(start, 48, 8, "G3@4:0", "mf").add(start + 1, 0, 8, "G3@4:0", "ff")


def vibes_part(P, secs):
    v = P["vb.m"]

    def note(bar, t, n, m, dyn=None, flags=""):
        while m > VIBES_HIGH:
            m -= 12
        while m < VIBES_LOW:
            m += 12
        v.add(bar, t, n, f"{name_of(m)}@{vibes_string(m)}:0", dyn, flags)

    def chord(bar, t, n, ms, dyn=None):
        ms = sorted({(m - VIBES_LOW) % 12 + VIBES_LOW + (12 if m >= 72 else 0) for m in ms})
        ms = [m for m in ms if VIBES_LOW <= m <= VIBES_HIGH]
        v.add(bar, t, n, "<" + " ".join(f"{name_of(m)}@{vibes_string(m)}:0" for m in ms) + ">", dyn)

    for start, end, kind, tr in secs:
        if kind == "intro":
            for bar, t, n, m in hook_line(start + 4, end, tr):
                note(bar, t, n, m + 12, "mp" if (bar == start + 4 and t == 0) else None)
        elif kind == "hook":
            for bar, t, n, m in hook_line(start, end, tr):
                note(bar, t, n, m + 12, "f" if t == 0 else None, "[acc]" if t == 0 else "")
        elif kind == "verse":
            if start == 9:
                for bar, t, n, m in tune(kind, start, end, tr):
                    note(bar, t, n, m, "mf" if t == 0 and bar == start else None)
            else:
                # verse 2: the trombone has the tune; the vibes comp in two-note dyads on the swung off-beats
                for bar, t, n, name, tr_ in chord_spans(kind, start, end, tr):
                    _, voicing, _ = CH[name]
                    chord(bar, t + 16, 8, [voicing[1] + tr, voicing[3] + tr], "p" if bar == start and t == 0 else None)
                    if n >= 48:
                        chord(bar, t + 40, 8, [voicing[0] + tr, voicing[2] + tr])
        elif kind == "chorus":
            dyn = "f" if start >= 53 else "mf"
            for bar, t, n, m in tune(kind, start, end, tr):
                d = dyn if t == 0 and bar == start else None
                # the last chorus: the tune in octaves where the bars reach
                if start == 61 and m + 12 <= VIBES_HIGH and n >= 16:
                    chord(bar, t, n, [m, m + 12], d)
                else:
                    note(bar, t, n, m, d)
        elif kind == "out":
            # clusters: four adjacent semitones planed down through the seven,
            # on the 1st and 4th eighths; a single high note answers now and then
            tick = (start + 1) * TICKS
            j = 0
            base = 75  # Eb5
            while tick < (start + 10) * TICKS:
                bar, t = divmod(tick, TICKS)
                bar += 1
                if j % 7 == 0:
                    c = base - (j // 7) % 5
                    chord(bar, t, 12, [c, c + 1, c + 2, c + 3], "mf" if j == 0 else None)
                if j % 7 == 3:
                    c = base - (j // 7) % 5
                    chord(bar, t, 12, [c - 5, c - 4, c + 1])
                if j % 14 == 12:
                    note(bar, t, 12, 81 - (j // 14) % 3, "mp")
                tick += 12
                j += 1
            # the fill bar: a rising whole-tone run into the chorus
            run = [60, 62, 64, 66, 68, 70, 72, 74, 76, 78, 80]
            for i, m in enumerate(run):
                note(end - 1, 30 + i * 6, 6, m, "mf" if i == 0 else None)
        elif kind == "coda":
            chord(start, 0, 96, [54 + 12, 58 + 12, 61 + 12, 65 + 12, 72], "f")       # Gb Bb Db F C: Gbmaj9 spread
            chord(start + 1, 0, 96, [66, 70, 73, 77, 80], "ff")                        # F#4 Bb4 C#5 F5 Ab5


def piano_part(P, secs):
    l, r, ped = P["pn.l"], P["pn.r"], P["pn.ped"]

    def voicing(name, tr):
        """Right hand: the chord's four-note voicing. Left hand: the root in the
        second octave, and the voicing's lowest note an octave down when that
        still sits above C3."""
        root_pc, v, _ = CH[name]
        rh = [m + tr for m in v]
        lh = [nearest(root_pc + tr, 36, 47, 42)]
        if min(rh) - 12 >= 48:
            lh.append(min(rh) - 12)
        return rh, lh

    def stab(bar, t, n, rh, lh, dyn=None):
        r.add(bar, t, n, "<" + " ".join(name_of(m) for m in rh) + ">", dyn)
        l.add(bar, t, n, "<" + " ".join(name_of(m) for m in lh) + ">")

    for start, end, kind, tr in secs:
        if kind in ("intro", "hook"):
            # the hook line in octaves, the chord on the long notes
            for bar, t, n, m in hook_line(start, end, tr):
                dyn = "mf" if t == 0 else None
                r.add(bar, t, n, f"<{name_of(m + 12)} {name_of(m + 24)}>", dyn)
                l.add(bar, t, n, f"<{name_of(m)} {name_of(m - 12)}>")
            for bar in range(start, end):
                ped.add(bar, 0, 48, "X").add(bar, 48, 48, "X")
        elif kind in ("verse", "chorus"):
            spans = chord_spans(kind, start, end, tr)
            for i, (bar, t, n, name, tr_) in enumerate(spans):
                rh, lh = voicing(name, tr)
                dyn = ("mp" if kind == "verse" else "mf") if i == 0 else None
                # Charleston comping: the chord on its beat, again on the "and" of the next beat
                if n >= 48:
                    stab(bar, t, 16, rh, lh, dyn)
                    stab(bar, t + 40, 8, rh, lh)
                else:
                    stab(bar, t, 16, rh, lh, dyn)
                ped.add(bar, t, n, "X")
        elif kind == "out":
            # low clusters on the 1st eighth of each seven, held
            tick = (start + 1) * TICKS
            j = 0
            while tick < (start + 10) * TICKS:
                bar, t = divmod(tick, TICKS)
                bar += 1
                if j % 7 == 0:
                    c = 38 - (j // 7) % 5
                    l.add(bar, t, 84, "<" + " ".join(name_of(m) for m in (c, c + 1, c + 2)) + ">", "f" if j == 0 else None)
                    r.add(bar, t, 84, "<" + " ".join(name_of(m) for m in (c + 25, c + 26, c + 27, c + 28)) + ">")
                    ped.add(bar, t, 84, "X")
                tick += 12
                j += 1
        elif kind == "coda":
            stab(start, 0, 96, [66, 70, 73, 77, 80], [30, 42], "f")
            stab(start + 1, 0, 96, [66, 70, 73, 77, 80, 84], [30, 42], "ff")
            ped.add(start, 0, 96, "X").add(start + 1, 0, 96, "X")


def brass_parts(secs):
    """Three trombone Worlds: tb1 leads, tb2 and tb3 fill the chord beneath."""
    tb = {"tb1": [], "tb2": [], "tb3": []}

    def under(m, name, tr, count):
        """The `count` chord tones nearest below m (from the voicing, any octave)."""
        root_pc, v, _ = CH[name]
        pcs = sorted({(x + tr) % 12 for x in v} | {(root_pc + tr) % 12})
        out, x = [], m - 1
        while len(out) < count and x > 40:
            if x % 12 in pcs and (not out or out[-1] - x >= 2):
                out.append(x)
            x -= 1
        return out

    for start, end, kind, tr in secs:
        if kind == "verse" and start == 9:
            # soft pads under verse 1 from bar 3: a swell per chord on the root's fifth and third
            for bar, t, n, name, tr_ in chord_spans(kind, start, end, tr):
                if bar < start + 2 or n < 48:
                    continue
                root_pc, v, _ = CH[name]
                a = nearest(v[2] + tr, 48, 60, 55)
                b = nearest(v[0] + tr, 44, 56, 50)
                tb["tb2"].append((bar, t, n - 4, a, 0.15, 0.55, "tongue", "vib"))
                tb["tb3"].append((bar, t, n - 4, b, 0.15, 0.50, "tongue", "-"))
        elif kind == "verse":
            # verse 2: the tune on tb1, an octave below the vibes' register, sung
            notes = tune(kind, start, end, tr)
            for i, (bar, t, n, m) in enumerate(notes):
                art = "slur" if n <= 8 and i > 0 and notes[i - 1][0] * TICKS + notes[i - 1][1] + notes[i - 1][2] == bar * TICKS + t else "tongue"
                fx = "vib" if n >= 24 else "-"
                vel = 0.70
                tb["tb1"].append((bar, t, max(n - 2, 4), m - 12, vel, vel, art, fx))
            # tb2/tb3 answer with held fifths under the long notes
            for bar, t, n, name, tr_ in chord_spans(kind, start, end, tr):
                if n < 48:
                    continue
                root_pc, v, _ = CH[name]
                tb["tb2"].append((bar, t + 24, n - 28, nearest(v[1] + tr, 46, 58, 52), 0.3, 0.45, "tongue", "-"))
                tb["tb3"].append((bar, t + 24, n - 28, nearest(root_pc + tr, 40, 52, 46), 0.3, 0.45, "tongue", "-"))
        elif kind == "chorus":
            big = start >= 53
            notes = tune(kind, start, end, tr)
            for i, (bar, t, n, m) in enumerate(notes):
                name = chord_at(kind, start, bar, t) or chord_at(kind, start, bar, max(t - 1, 0)) or "Fmaj9"
                lead = m - 12
                vel = 0.85 if big else 0.70
                art = "tongue"
                fx = "vib" if n >= 36 else "-"
                if n >= 48 and bar == end - 1:
                    fx = "vib,dim"
                length = max(n - 2, 4)
                if big and i == 0:
                    fx = "rip"  # a rip up into the big chorus
                tb["tb1"].append((bar, t, length, lead, vel, vel, art, fx))
                lower = under(lead, name, tr, 2)
                if len(lower) == 2 and n >= 8:
                    tb["tb2"].append((bar, t, length, lower[0], vel - 0.1, vel - 0.1, art, "-"))
                    tb["tb3"].append((bar, t, length, lower[1], vel - 0.1, vel - 0.1, art, "-"))
        elif kind == "hook":
            for bar, t, n, m in hook_line(start, end, tr):
                name = chord_at(kind, start, bar, t) or "F13"
                tb["tb1"].append((bar, t, max(n - 2, 4), m, 0.85, 0.85, "tongue", "-"))
                lower = under(m, name, tr, 2)
                if len(lower) == 2:
                    tb["tb2"].append((bar, t, max(n - 2, 4), lower[0], 0.75, 0.75, "tongue", "-"))
                    tb["tb3"].append((bar, t, max(n - 2, 4), lower[1], 0.8, 0.8, "tongue", "-"))
        elif kind == "out":
            # free time over the seven: slow smears and falls on the cluster's edges
            tb["tb1"].append((start + 2, 30, 150, 50, 0.3, 0.8, "tongue", "vib,swell"))
            tb["tb1"].append((start + 4, 12, 80, 51, 0.8, 0.8, "smear", "-"))
            tb["tb1"].append((start + 5, 20, 120, 49, 0.7, 0.3, "smear", "fall"))
            tb["tb1"].append((start + 8, 0, 200, 53, 0.2, 0.9, "tongue", "vib,swell"))
            tb["tb1"].append((start + 10, 40, 56, 58, 0.9, 0.9, "tongue", "rip"))
            tb["tb1"].append((start + 11, 24, 60, 57, 0.9, 0.4, "smear", "fall"))
            tb["tb2"].append((start + 3, 0, 180, 44, 0.3, 0.7, "tongue", "vib,swell"))
            tb["tb2"].append((start + 6, 48, 120, 45, 0.7, 0.7, "smear", "vib"))
            tb["tb2"].append((start + 9, 60, 150, 43, 0.3, 0.8, "tongue", "swell"))
            tb["tb3"].append((start + 1, 48, 200, 38, 0.3, 0.7, "tongue", "vib,swell"))
            tb["tb3"].append((start + 5, 0, 120, 39, 0.7, 0.7, "smear", "-"))
            tb["tb3"].append((start + 7, 72, 160, 37, 0.3, 0.8, "tongue", "swell"))
            tb["tb3"].append((start + 11, 0, 48, 40, 0.9, 0.9, "tongue", "rip"))
        elif kind == "coda":
            tb["tb1"].append((start, 0, 90, 66, 0.9, 0.9, "tongue", "vib"))
            tb["tb2"].append((start, 0, 90, 61, 0.8, 0.8, "tongue", "vib"))
            tb["tb3"].append((start, 0, 90, 54, 0.8, 0.8, "tongue", "vib"))
            tb["tb1"].append((start + 1, 0, 120, 66, 0.95, 0.95, "tongue", "vib"))
            tb["tb2"].append((start + 1, 0, 120, 61, 0.85, 0.85, "tongue", "vib"))
            # the Doorkeeper's joke: the third trombone falls off the last chord
            tb["tb3"].append((start + 1, 0, 110, 54, 0.9, 0.5, "tongue", "fall"))
    return tb


# The choir. Syllables are a hum's sealed lips released into a vowel: "bah"
# and "dah" = m (50 ms) then a, released in 20 ms; "doo" = m then o; "wah" =
# m then a over 0.2 s (the default glide); "ah" / "ooh" = a / o held; "mm" = m.
SYL = {"bah": ("a", 0.02), "dah": ("a", 0.02), "doo": ("o", 0.02), "wah": ("a", 0.2), "ah": ("a", None), "ooh": ("o", None), "mm": ("m", None)}
VOICE_OF = {name: voice for name, voice, _ in CAST}
SKILL = HERE.parent / "before-the-law" / "choir" / "skill"


def sung_ranges():
    """What each singer's calibration heard it sing, per vowel, as midi (lo, hi)
    with a quarter tone of margin inside: read from the skill tables."""
    import math
    out = {}
    for name, _, _ in CAST:
        rows = [l.split("\t") for l in (SKILL / f"{name}.tsv").read_text().splitlines() if l and not l.startswith("#") and not l.startswith("vowel")]
        by = {}
        for v, tuning, sounding, *_ in rows:
            f = float(sounding)
            if f > 0:
                by.setdefault(v, []).append(f)
        out[name] = {v: (69 + 12 * math.log2(min(f) / 440) + 0.5, 69 + 12 * math.log2(max(f) / 440) - 0.5) for v, f in by.items()}
    return out


RANGE = sung_ranges()


def choir_parts(secs):
    S = {name: [] for name, _, _ in CAST}

    def fit(singer, m, vowels):
        """The pitch, moved by octaves into what this singer's calibration heard on every vowel in `vowels`."""
        lo = max(RANGE[singer][v][0] for v in vowels)
        hi = min(RANGE[singer][v][1] for v in vowels)
        while m < lo:
            m += 12
        while m > hi:
            m -= 12
        assert lo <= m <= hi, (singer, m, vowels, lo, hi)
        return m

    def sing(singer, bar, tick, length, m, syl, breath, vib):
        vowel, glide = SYL[syl]
        m = fit(singer, m, (vowel, "m") if glide is not None else (vowel,))
        if glide is not None:
            # the sealed lips first: a short hum, then the vowel released
            S[singer].append((bar, tick, 2, m, "m", breath * 0.8, 0.0, 0.02))
            S[singer].append((bar, tick + 2, length - 2, m, vowel, breath, vib, glide))
        else:
            S[singer].append((bar, tick, length, m, vowel, breath, vib, 0.2))

    def voice_chord(name, tr, top=None):
        """Who sings what: basses the root, baritones the fifth/seventh, tenors the
        third/seventh, altos the ninth/third, sopranos the top of the voicing."""
        root_pc, v, _ = CH[name]
        v = [x + tr for x in v]
        root = root_pc + tr
        return {"bass1": root + 36, "bass2": root + 36 + 7 if root + 43 <= 60 else root + 36,
                "bari1": v[0], "bari2": v[1], "ten1": v[1], "ten2": v[2],
                "alto1": v[2], "alto2": v[3], "sop1": v[3], "sop2": v[2] + 12 if top is None else top}

    for start, end, kind, tr in secs:
        if kind == "intro":
            # from bar 5: "doo" pads, one per chord, the men and altos
            for bar, t, n, name, tr_ in chord_spans(kind, start + 4, end, tr):
                vc = voice_chord(name, tr)
                for s in ("bass1", "bass2", "bari1", "bari2", "ten1", "ten2", "alto1", "alto2"):
                    sing(s, bar, t, n - 4, vc[s], "doo", 0.5, 0.08)
        elif kind == "verse" and start == 27:
            # "doo ... wah" pads under the trombone's tune
            for bar, t, n, name, tr_ in chord_spans(kind, start, end, tr):
                vc = voice_chord(name, tr)
                for s in ("bass1", "bass2", "bari1", "bari2", "ten1", "ten2", "alto1", "alto2", "sop1", "sop2"):
                    if n >= 48:
                        sing(s, bar, t, 20, vc[s], "doo", 0.5, 0.06)
                        sing(s, bar, t + 24, n - 28, vc[s], "wah", 0.52, 0.12)
                    else:
                        sing(s, bar, t, n - 4, vc[s], "doo", 0.5, 0.06)
        elif kind == "chorus":
            big = start >= 53
            if big:
                # the sopranos and children sing the tune on "ah" (a "dah" on each new note)
                notes = tune(kind, start, end, tr)
                for i, (bar, t, n, m) in enumerate(notes):
                    for s in ("sop1", "sop2", "child1", "child2"):
                        sing(s, bar, t, n, m, "dah" if n <= 16 else "ah", 0.6, 0.12 if n >= 24 else 0.0)
                voices = ("bass1", "bass2", "bari1", "bari2", "ten1", "ten2", "alto1", "alto2")
            else:
                voices = ("bass1", "bass2", "bari1", "bari2", "ten1", "ten2", "alto1", "alto2", "sop1", "sop2")
            # "bah" stabs on the Charleston (the beat and the swung "and" of the next), with the piano
            for bar, t, n, name, tr_ in chord_spans(kind, start, end, tr):
                vc = voice_chord(name, tr)
                for s in voices:
                    if bar == end - 1 and t == 48:
                        # the last chord of a chorus is held
                        sing(s, bar, t, 44, vc[s], "wah", 0.56, 0.12)
                        continue
                    sing(s, bar, t, 14, vc[s], "bah", 0.58, 0.0)
                    if n >= 48:
                        sing(s, bar, t + 40, 8, vc[s], "bah", 0.55, 0.0)
        elif kind == "hook":
            for bar, t, n, name, tr_ in chord_spans(kind, start, end, tr):
                vc = voice_chord(name, tr)
                for s in ("bass1", "bass2", "bari1", "bari2", "ten1", "ten2", "alto1", "alto2", "sop1", "sop2"):
                    sing(s, bar, t, 14, vc[s], "bah", 0.58, 0.0)
                    if n >= 96:
                        sing(s, bar, t + 24, 14, vc[s], "bah", 0.55, 0.0)
                        sing(s, bar, t + 48, 40, vc[s], "wah", 0.55, 0.1)
        elif kind == "out":
            # the strange part: twelve singers on a chromatic cluster around D,
            # humming, each opening to "ah" at its own moment, then sliding
            # up by quarter tones (one short note per step, every 0.15 s or
            # so: the folds retuned, every move booked) to a stacked cluster a
            # tritone higher; then silence for the fill.
            base = {"bass1": 38, "bass2": 39, "bari1": 41, "bari2": 42, "ten1": 44, "ten2": 45,
                    "alto1": 56, "alto2": 57, "sop1": 59, "sop2": 60, "child1": 62, "child2": 63}
            for i, (s, m) in enumerate(base.items()):
                lo = max(RANGE[s]["a"][0], RANGE[s]["m"][0])
                hi = min(RANGE[s]["a"][1], RANGE[s]["m"][1])
                m0 = min(max(m, lo), hi - 6.5)
                S[s].append((start + 1, 24 + 8 * (i % 3), 60 + 8 * (i % 4), m0, "m", 0.5, 0.0, 0.2))
                S[s].append((start + 2, 48 + 12 * (i % 4), 96, m0, "a", 0.56, 0.15, 0.2))
                # the slide: 24 quarter-tone steps, 7 ticks each (0.162 s), rising a tritone
                t0 = (start + 5 - 1) * TICKS + 12 * (i % 4)
                for k in range(24):
                    bar, t = divmod(t0 + k * 7, TICKS)
                    S[s].append((bar + 1, t, 7, m0 + 0.25 * (k + 1), "a", 0.56, 0.0, 0.2))
                bar, t = divmod(t0 + 24 * 7, TICKS)
                S[s].append((bar + 1, t, 156 + 12 * (i % 3), m0 + 6, "a", 0.58, 0.25, 0.2))
        elif kind == "coda":
            vc = {"bass1": 30 + 12, "bass2": 30 + 19, "bari1": 30 + 24, "bari2": 30 + 28, "ten1": 30 + 31, "ten2": 30 + 35,
                  "alto1": 30 + 36, "alto2": 30 + 40, "sop1": 30 + 43, "sop2": 30 + 47, "child1": 30 + 40, "child2": 30 + 43}
            for s, m in vc.items():
                sing(s, start, 0, 90, m, "bah", 0.58, 0.1)
                sing(s, start + 1, 0, 150, m, "ooh" if "child" not in s else "ah", 0.55, 0.15)
    return S


# ------------------------------------------------------------ auditions

def auditions():
    """Short probes of each new World, for the measurements in README.md."""
    P = {"wk.f": Part()}
    for i, m in enumerate([28, 33, 38, 43, 45, 48, 50, 53, 55]):
        P["wk.f"].add(1 + i // 4, (i % 4) * BEAT, BEAT, walk_pos(m), "mf" if i == 0 else None)
    P["wk.f"].add(3, 24, 72, walk_pos(41), "f", "[acc]")
    write_mus(SCORES / "walker-audition.mus", "Rule Britannia: Walker audition", "3 bars / the Walker / open courses, a scale, one accent", [(WALK, 0)], P, bars=3)
    P = {"vb.m": Part()}
    for i, m in enumerate([60, 65, 69, 72, 76, 79, 81]):
        P["vb.m"].add(1 + i // 4, (i % 4) * BEAT, BEAT, f"{name_of(m)}@{vibes_string(m)}:0", "mf" if i == 0 else None)
    P["vb.m"].add(3, 0, 96, "<" + " ".join(f"{name_of(m)}@{vibes_string(m)}:0" for m in (65, 69, 72, 76)) + ">", "f")
    write_mus(SCORES / "vibes-audition.mus", "Rule Britannia: Vibes audition", "3 bars / the Vibes / an arpeggio, one chord", [(VIBES, 0)], P, bars=3)
    P = {f"kt.{h}": Part() for h in KIT["hands"]}
    for i, h in enumerate(["kick", "sn", "rim", "hhc", "hho", "ride", "br"]):
        tok, s = KIT["hands"][h][:2]
        P[f"kt.{h}"].add(1 + i // 4, (i % 4) * BEAT, 12, f"{tok}@{s}:0", "mf")
    for t in range(0, TICKS, 4):
        P["kt.br"].add(3, t, 4, "G3@5:0", "pp" if t % BEAT else "p")
    write_mus(SCORES / "kit-audition.mus", "Rule Britannia: Kit audition", "3 bars / the Kit / each piece once, a brush sweep", [(KIT, 0)], P, bars=3)


# ------------------------------------------------------------ the piece

def piece():
    secs = sections()
    P = {"wk.f": Part()}
    bass_part(P, secs)
    write_mus(SCORES / "rb-walker.mus", "Rule, Britannia! (Arne) - the Walker", f"{BARS} bars at {BPM}, swung / upright bass", [(WALK, 0)], P, texts=TEXTS, master=-18)
    P = {f"kt.{h}": Part() for h in KIT["hands"]}
    kit_part(P, secs)
    write_mus(SCORES / "rb-kit.mus", "Rule, Britannia! (Arne) - the Kit", f"{BARS} bars at {BPM}, swung / kit", [(KIT, 0)], P, texts=TEXTS, master=-18)
    P = {"vb.m": Part()}
    vibes_part(P, secs)
    write_mus(SCORES / "rb-vibes.mus", "Rule, Britannia! (Arne) - the Vibes", f"{BARS} bars at {BPM}, swung / vibes: the tune, the hook, clusters", [(VIBES, 0)], P, texts=TEXTS, master=-18)
    P = {"pn.l": Part(), "pn.r": Part(), "pn.ped": Part()}
    piano_part(P, secs)
    write_mus(SCORES / "rb-piano.mus", "Rule, Britannia! (Arne) - the piano", f"{BARS} bars at {BPM}, swung / Ariadne piano comping", [], P, piano=True, texts=TEXTS, master=-18)
    for name, notes in brass_parts(secs).items():
        write_trombone(HERE / "brass" / f"rb-{name}.tsv", name, notes)
    # two choir renders (two stems): the men and altos, and the sopranos and
    # children, so the studio can balance the tune against the "bah"s
    parts = choir_parts(secs)
    write_choir(HERE / "choir" / "rb-choir-low.tsv", {k: v for k, v in parts.items() if VOICE_OF[k] in ("bass", "baritone", "tenor", "alto")})
    write_choir(HERE / "choir" / "rb-choir-high.tsv", {k: v for k, v in parts.items() if VOICE_OF[k] in ("soprano", "child")})


if __name__ == "__main__":
    SCORES.mkdir(exist_ok=True)
    (HERE / "brass").mkdir(exist_ok=True)
    (HERE / "choir").mkdir(exist_ok=True)
    auditions()
    piece()
