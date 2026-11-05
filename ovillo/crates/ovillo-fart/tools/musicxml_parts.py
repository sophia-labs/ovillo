# /// script
# requires-python = ">=3.10"
# dependencies = ["music21>=9"]
# ///
"""MusicXML -> an ovillo-fart score directory, deterministically.

    uv run --script musicxml_parts.py <score.mxl> <out dir> [--voices 6] [--lo 36] [--hi 300] [--max-bars 16]

Writes:
  <out>/source/sections.json             sections (split at tempo and time-signature changes): id, measures,
                                          bpm (quarter notes), bar, beats, tempo text, and per part its note count,
                                          range and chord count
  <out>/source/<section>/<part>.notes    every part's top line at sounding pitch (ties merged, tuplets as
                                          fractions, unpitched notes as X, a note held over a bar line
                                          tied with ~): reference for arrangers
  <out>/score.json, <out>/<section>/<voice>.notes
                                          a baseline arrangement in ovillo-fart's part format (tensions naive,
                                          nothing fed: the composers' work): per section, the busiest pitched parts by
                                          register into voices v1 (high) .. vN (low), each moved by whole
                                          octaves toward its voice's register inside [lo, hi] Hz (the
                                          valve's measured range), stray notes folded in by octaves;
                                          `cannon` from the Canon part

Nothing here is a model's judgment: the same file gives the same directory.
"""

import json
import math
import re
import sys
from fractions import Fraction
from pathlib import Path

from music21 import converter, meter, note, chord

ARGS = sys.argv[1:]
if len(ARGS) < 2:
    sys.exit(__doc__)
SRC, OUT = Path(ARGS[0]), Path(ARGS[1])
opt = lambda flag, d: type(d)(ARGS[ARGS.index(flag) + 1]) if flag in ARGS else d
N_VOICES, LO, HI, MAX_BARS = opt("--voices", 6), opt("--lo", 36.0), opt("--hi", 300.0), opt("--max-bars", 16)
# Each voice's register target (Hz), high to low, spread over [LO, HI] in log.
TARGETS = [HI * 0.75 * (LO * 1.6 / (HI * 0.75)) ** (k / max(1, N_VOICES - 1)) for k in range(N_VOICES)]
PANS = [-0.6, 0.6, -0.3, 0.3, -0.1, 0.1, -0.45, 0.45]
CANNON = re.compile(r"canon|cannon", re.I)
SKIP_BASELINE = re.compile(r"timpani|tamb|triang|militare|piatti|cassa|cloches|bell|canon|cannon|banda", re.I)
NAMES = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"]


def slug(text):
    return re.sub(r"[^a-z0-9]+", "-", text.lower().replace("♭", "b").replace("♯", "#")).strip("-") or "part"


def name_of(midi):
    return f"{NAMES[midi % 12]}{midi // 12 - 1}"


def hz(midi):
    return 440.0 * 2 ** ((midi - 69) / 12)


def frac(q):
    f = Fraction(q).limit_denominator(96)
    return str(f.numerator) if f.denominator == 1 else f"{f.numerator}/{f.denominator}"


CACHE = OUT / "source" / "events.json"


def parse():
    """The slow part (music21): every part as events, measures, tempo marks
    and time signatures, all as offsets in quarter notes. Cached in
    source/events.json, keyed by the source's size and name."""
    key = f"{SRC.name}:{SRC.stat().st_size}"
    if CACHE.exists():
        cached = json.loads(CACHE.read_text())
        if cached.get("key") == key:
            return cached
    score = converter.parse(str(SRC)).toSoundingPitch()
    q = lambda x: str(Fraction(x).limit_denominator(96))
    first = score.parts[0]
    data = {
        "key": key,
        "title": score.metadata.title if score.metadata else None,
        "total": q(score.highestTime),
        "measures": [[q(m.offset), m.number] for m in first.getElementsByClass("Measure")],
        # The tempo's own words only (music21 names bare numbers itself).
        "marks": [[q(a), mm.getQuarterBPM(), "" if getattr(mm, "textImplicit", True) else (mm.text or "")] for a, _, mm in score.metronomeMarkBoundaries()],
        "sigs": [[q(ts.getOffsetInHierarchy(first)), q(ts.barDuration.quarterLength)] for ts in first.recurse().getElementsByClass(meter.TimeSignature)],
        "parts": [],
    }
    for p in score.parts:
        pname = p.partName or p.id
        events = []
        for el in p.stripTies().chordify().flatten().notesAndRests:
            dur = Fraction(el.quarterLength).limit_denominator(96)
            if dur <= 0:
                continue  # grace notes
            if isinstance(el, note.Rest):
                what = None
            elif isinstance(el, chord.Chord):
                pitched = [n.pitch.midi for n in el.notes if hasattr(n, "pitch")]
                what = max(pitched) if pitched else "X"
            elif isinstance(el, note.Note):
                what = el.pitch.midi
            else:
                what = "X"
            if CANNON.search(pname) and what is not None:
                what = "X"
            events.append([q(el.offset), str(dur), what, isinstance(el, chord.Chord) and len(el.notes) > 1])
        data["parts"].append({"name": pname, "events": events})
    CACHE.parent.mkdir(parents=True, exist_ok=True)
    CACHE.write_text(json.dumps(data))
    return data


data = parse()
F = Fraction
total = F(data["total"])
measure_at = [(F(o), n) for o, n in data["measures"]]
marks = [(F(o), (bpm, text)) for o, bpm, text in data["marks"]]
sigs = [(F(o), F(bar)) for o, bar in data["sigs"]]
parts = [{"name": p["name"], "slug": slug(p["name"]), "events": [(F(o), F(d), w, c) for o, d, w, c in p["events"]]} for p in data["parts"]]
seen = {}
for p in parts:  # unique slugs
    n = seen.get(p["slug"], 0)
    seen[p["slug"]] = n + 1
    if n:
        p["slug"] = f"{p['slug']}-{n + 1}"


def measure_of(offset):
    num = measure_at[0][1]
    for o, n in measure_at:
        if o <= offset:
            num = n
    return num


def in_effect(items, offset):
    found = items[0][1]
    for o, it in items:
        if o <= offset:
            found = it
    return found


# Sections: at tempo marks and time signatures, then at most MAX_BARS bars
# each (at bar lines), so no part is too long for one agent.
bar_starts = {o for o, _ in measure_at}
cuts = sorted({o for o, _ in marks} | {o for o, _ in sigs} | {F(0)})
cuts = [c for c in cuts if c < total]
chunked = []
for i, c in enumerate(cuts):
    end = cuts[i + 1] if i + 1 < len(cuts) else total
    chunked.append(c)
    bar = in_effect(sigs, c)
    if c in bar_starts:
        k = 1
        while c + k * MAX_BARS * bar < end:
            chunked.append(c + k * MAX_BARS * bar)
            k += 1
cuts = chunked
sections = []
for i, start in enumerate(cuts):
    end = cuts[i + 1] if i + 1 < len(cuts) else total
    bpm, text = in_effect(marks, start)
    sections.append({
        "start": start, "end": end,
        "bpm": round(float(bpm), 3),
        "bar": float(in_effect(sigs, start)),
        "tempoText": text.strip().rstrip("."),
        "on_bar": start in bar_starts,
    })
for i, s in enumerate(sections):
    s["id"] = f"{i + 1:02d}-m{measure_of(s['start'])}" + (f"-{slug(s['tempoText'])}" if s["tempoText"] else "")


def window(events, start, end):
    """Events clipped to [start, end), gaps as rests: (duration, what)."""
    out, t = [], start
    for on, dur, what, _ in events:
        # never before the previous event's end: chordified voices can overlap
        a, b = max(on, start, t), min(on + dur, end)
        if b <= a:
            continue
        if a > t:
            out.append((a - t, None))
        out.append((b - a, what))
        t = b
    if t < end:
        out.append((end - t, None))
    merged = []
    for d, w in out:  # adjacent rests become one
        if merged and w is None and merged[-1][1] is None:
            merged[-1] = (merged[-1][0] + d, None)
        else:
            merged.append((d, w))
    return merged


def render_line(items, start, bar, on_bar, shift=lambda m: m):
    """Tokens with bar lines (when the section starts on a bar)."""
    toks, t = [], Fraction(0)
    for d, w in items:
        name = "r" if w is None else "X" if w == "X" else name_of(shift(w))
        # split at bar lines so every | falls between tokens
        while on_bar and bar and d > 0:
            to_bar = Fraction(bar).limit_denominator(96) - (t % Fraction(bar).limit_denominator(96))
            if d <= to_bar:
                break
            # a held note continues over the bar line as a tie (~); rests split freely
            toks.append(f"{name}:{frac(to_bar)}" + ("" if name == "r" else "~"))
            t += to_bar
            d -= to_bar
            toks.append("|")
        toks.append(f"{name}:{frac(d)}")
        t += d
        if on_bar and bar and t % Fraction(bar).limit_denominator(96) == 0:
            toks.append("|")
    lines, line = [], []
    for tok in toks:
        line.append(tok)
        if tok == "|" and sum(1 for x in line if x == "|") >= 2:
            lines.append(" ".join(line))
            line = []
    if line:
        lines.append(" ".join(line))
    return "\n".join(lines) + "\n"


src = OUT / "source"
src.mkdir(parents=True, exist_ok=True)
summary, arrangement = [], []
voice_names = [f"v{k + 1}" for k in range(N_VOICES)]
for s in sections:
    sdir = src / s["id"]
    sdir.mkdir(exist_ok=True)
    info = {k: (float(v) if isinstance(v, Fraction) else v) for k, v in s.items() if k not in ("start", "end")}
    info["measures"] = [measure_of(s["start"]), measure_of(s["end"] - Fraction(1, 96))]
    info["beats"] = float(s["end"] - s["start"])
    info["parts"] = {}
    activity = []
    for p in parts:
        items = window(p["events"], s["start"], s["end"])
        sounding = [(d, w) for d, w in items if w is not None]
        if not sounding:
            continue
        pitched = [w for _, w in sounding if w != "X"]
        chords = sum(1 for on, dur, w, c in p["events"] if c and s["start"] <= on < s["end"])
        (sdir / f"{p['slug']}.notes").write_text(f"# {p['name']}, top line, sounding pitch\n" + render_line(items, s["start"], s["bar"], s["on_bar"]))
        info["parts"][p["slug"]] = {"name": p["name"], "notes": len(sounding), "chords": chords,
                                    "range": [name_of(min(pitched)), name_of(max(pitched))] if pitched else None}
        if pitched and not SKIP_BASELINE.search(p["name"]):
            busy = float(sum(d for d, w in sounding))
            med = sorted(pitched)[len(pitched) // 2]
            activity.append((busy, med, p, items))
    summary.append(info)
    # Baseline: the busiest pitched parts, by register, into v1..vN.
    chosen = sorted(sorted(activity, key=lambda a: -a[0])[:N_VOICES], key=lambda a: -a[1])
    adir = OUT / s["id"]
    adir.mkdir(parents=True, exist_ok=True)
    used = []
    for k, vname in enumerate(voice_names):
        if k < len(chosen):
            busy, med, p, items = chosen[k]
            target = TARGETS[round(k * (N_VOICES - 1) / max(1, len(chosen) - 1))] if len(chosen) > 1 else TARGETS[0]
            octs = round(math.log2(target / hz(med)))

            def shift(m, octs=octs):
                m += 12 * octs
                while hz(m) > HI:
                    m -= 12
                while hz(m) < LO:
                    m += 12
                return m

            head = (f"# baseline: {p['name']}, top line, {octs:+d} octave(s), strays folded into {LO:.0f}-{HI:.0f} Hz\n"
                    "# tensions are the targets' own frequencies (naive: the valve runs sharp of them); nothing eaten, no digestion\n"
                    "push=0.7 cheeks=0.4 tone=0.75 tongue=on\n")
            (adir / f"{vname}.notes").write_text(head + render_line(items, s["start"], s["bar"], s["on_bar"], shift))
            used.append(p["name"])
        else:
            (adir / f"{vname}.notes").write_text(f"# baseline: tacet\n{render_line([(s['end'] - s['start'], None)], s['start'], s['bar'], s['on_bar'])}")
    canon = next((p for p in parts if CANNON.search(p["name"])), None)
    citems = window(canon["events"], s["start"], s["end"]) if canon else [(s["end"] - s["start"], None)]
    (adir / "cannon.notes").write_text("# baseline: the Canon part\npush=0.85 cheeks=0.45\n" + render_line(citems, s["start"], s["bar"], s["on_bar"]))
    arrangement.append({"id": s["id"], "bpm": info["bpm"], "beats": info["beats"], **({"bar": info["bar"]} if s["on_bar"] else {}),
                        "voices": voice_names + ["cannon"], "about": f"m{info['measures'][0]}-{info['measures'][1]} {s['tempoText']}: {', '.join(used)}".strip()})

(src / "sections.json").write_text(json.dumps(summary, indent=1, ensure_ascii=False))
voices = {v: {"pan": PANS[k % len(PANS)], "gain": 0.7, "about": f"baseline register {TARGETS[k]:.0f} Hz"} for k, v in enumerate(voice_names)}
voices["cannon"] = {"kind": "cannon", "pan": 0.0, "gain": 0.9, "about": "the Canon part"}
title = (data["title"] or SRC.stem) + " (Tchaikovsky, 1880), for guts: OpenScore (CC0) parts"
(OUT / "score.json").write_text(json.dumps({"title": title, "about": f"from {SRC.name} by musicxml_parts.py", "voices": voices, "sections": arrangement}, indent=1, ensure_ascii=False))
print(f"{len(sections)} sections, {len(parts)} parts -> {OUT}")
for a in arrangement:
    print(f"  {a['id']:<34} {a['bpm']:7.2f} bpm  {a['beats']:7.2f} beats  {a.get('bar', '-')}  {a['about'][:90]}")
