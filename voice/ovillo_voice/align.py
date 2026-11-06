"""Cut a roll at the line boundaries the words themselves say, not where the button was pressed.

A roll's marks are pressed by hand, and often late: Vera starts the next line before hitting NEXT, so a cut snapped
near the press can land inside the next line. Here the roll is transcribed with word timestamps (faster-whisper,
small.en, on CPU). The words are aligned to the script lines in the order they were spoken (difflib over normalised
tokens), and each line's span runs from its first word to its last. Consecutive lines are cut at the quietest 20 ms
in the gap between them, never inside a word. Words not in the script (ad libs such as "Oh,") go with the line of
the nearest scripted word.
"""
import difflib, json, re, subprocess, time
import numpy as np
from . import studio, analysis

SR = 48000
PRE, POST, FADE = 0.15, 0.25, 0.03


def tokens(text):
    return [t for t in re.sub(r"[^a-z0-9' ]+", " ", text.lower().replace("’", "'")).split() if t]


def transcribe(wav, cache=None):
    if cache and cache.exists():
        return json.loads(cache.read_text())
    from faster_whisper import WhisperModel
    s = analysis.load(wav).resample(16000); a = s.values[0].astype(np.float32)
    m = WhisperModel("small.en", device="cpu", compute_type="int8")
    segs, _ = m.transcribe(a, word_timestamps=True, vad_filter=False, condition_on_previous_text=False)
    words = [dict(start=round(float(w.start), 3), end=round(float(w.end), 3), word=w.word.strip()) for sg in segs for w in sg.words]
    if cache:
        cache.write_text(json.dumps(words, indent=0))
    return words


def _line_of(words, seq_texts):
    exp, owner = [], []
    for i, t in enumerate(seq_texts):
        tk = tokens(t); exp += tk; owner += [i] * len(tk)
    hyp = [(tokens(w["word"]) or [""])[0] for w in words]
    line_of = [None] * len(words)
    for op, a0, a1, b0, b1 in difflib.SequenceMatcher(None, exp, hyp, autojunk=False).get_opcodes():
        if op in ("equal", "replace"):
            for k in range(b1 - b0):
                line_of[b0 + k] = owner[a0 + min(a1 - a0 - 1, int(k * (a1 - a0) / max(1, b1 - b0)))]
    known = [i for i, l in enumerate(line_of) if l is not None]
    for i, l in enumerate(line_of):          # ad libs: the nearest scripted word, by time
        if l is None and known:
            j = min(known, key=lambda k: min(abs(words[k]["start"] - words[i]["end"]), abs(words[i]["start"] - words[k]["end"])))
            line_of[i] = line_of[j]
    return line_of


def align(words, seq_texts):
    """words: [{start, end, word}]; seq_texts: the lines in spoken order. Returns [(start, end) or None] per line."""
    line_of = _line_of(words, seq_texts)
    spans = [None] * len(seq_texts)
    for w, l in zip(words, line_of):
        if l is None:
            continue
        s = spans[l]; spans[l] = (min(s[0], w["start"]), max(s[1], w["end"])) if s else (w["start"], w["end"])
    return spans


def quietest(a, t0, t1):
    n = int(0.02 * SR); i0, i1 = int(t0 * SR), int(t1 * SR)
    if i1 - i0 < 2 * n:
        return (t0 + t1) / 2
    e = np.convolve(a[i0:i1] ** 2, np.ones(n) / n, mode="valid")
    return (i0 + int(np.argmin(e)) + n // 2) / SR


def cuts(roll_wav, seq_texts, cache=None):
    """[(a, b)] per line in roll time: the line's words, padded, never crossing the cut to a neighbour."""
    snd = analysis.load(roll_wav); a = snd.values[0]; dur = len(a) / SR
    words = transcribe(roll_wav, cache)
    spans = align(words, seq_texts)
    # Word timestamps say which line a stretch of speech belongs to; the audio says where the speech is. Whisper's
    # first words can be a second off, so every stretch of actual voice (not clicks or UI blips) joins the line whose
    # words overlap it most, else the line of the nearest word, and that line's span grows to cover it.
    owner_spans = [(w, l) for w, l in zip(words, _line_of(words, seq_texts)) if l is not None]
    for g in analysis.segments(snd, min_gap=0.15, min_len=0.12)["segments"]:
        if g["voiced_pct"] < 15 or g.get("f0_median", 0) > 450:
            continue
        ov = {}
        for w, l in owner_spans:
            o = min(g["end"], w["end"]) - max(g["start"], w["start"])
            if o > 0: ov[l] = ov.get(l, 0) + o
        if ov:
            l = max(ov, key=ov.get)
        elif owner_spans:
            l = min(owner_spans, key=lambda wl: min(abs(wl[0]["start"] - g["end"]), abs(g["start"] - wl[0]["end"])))[1]
        else:
            continue
        s = spans[l]; spans[l] = (min(s[0], g["start"]), max(s[1], g["end"])) if s else (g["start"], g["end"])
    bounds = []                               # the cut between consecutive voiced lines
    voiced = [i for i, s in enumerate(spans) if s]
    for i, j in zip(voiced, voiced[1:]):
        g0, g1 = spans[i][1], spans[j][0]
        if g1 - g0 < 0.12:   # words touching (word timestamps are only good to ~0.1 s): look for the silence around them
            g0, g1 = min(g0, g1) - 0.25, max(g0, g1) + 0.25
        bounds.append((i, j, quietest(a, g0, g1)))
    lo = {j: c for i, j, c in bounds}; hi = {i: c for i, j, c in bounds}
    n = int(0.02 * SR); fr = np.sqrt(np.convolve(a ** 2, np.ones(n) / n, mode="same")) + 1e-9
    quiet = float(np.percentile(fr[::n], 10)) * 10 ** (10 / 20)      # the room, plus 10 dB
    loud = lambda t: fr[min(len(fr) - 1, max(0, int(t * SR)))] > quiet
    out = []
    for i, s in enumerate(spans):
        if not s:
            out.append(None); continue
        a0, a1 = max(lo.get(i, 0.0), s[0] - PRE), min(hi.get(i, dur), s[1] + POST)
        while loud(a0) and a0 - 0.02 > lo.get(i, 0.0):      # don't start on a breath or an onset: back up to the room
            a0 -= 0.02
        while loud(a1) and a1 + 0.02 < hi.get(i, dur):      # nor end on a tail
            a1 += 0.02
        out.append((round(a0, 3), round(a1, 3)))
    return out, spans


def export_cut(roll_wav, a0, a1, dst):
    """Trim inside the filter graph and reset the clock BEFORE fading. (An output-side -ss leaves the filters on the
    roll's own timeline, so the fade-out fired at the line's length into the roll and zeroed the rest: that shipped
    silent files once.) Refuses to leave a silent file behind."""
    d = a1 - a0
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(roll_wav),
                    "-af", f"atrim=start={a0:.3f}:end={a1:.3f},asetpts=PTS-STARTPTS,afade=t=in:d={FADE},afade=t=out:st={max(0, d - FADE):.3f}:d={FADE}",
                    "-ac", "1", "-ar", str(SR), "-c:a", "pcm_s24le", str(dst)], check=True)
    x = analysis.load(dst).values[0]
    rms = float(np.sqrt(np.mean(x ** 2))) if len(x) else 0.0
    if len(x) < int(0.8 * d * SR) or rms < 1e-4:
        raise RuntimeError(f"export of {dst.name} came out silent or short ({len(x) / SR:.2f} s, rms {rms:.1e})")


def recut_session(session, dest=None):
    """Re-export a rolled session's circled takes cut at the words. The takes' own dry.wav are left untouched
    (the studio rule); each gets aligned.json, and the film export gets selects.json with the cut points."""
    S = studio.read_json(studio.STUDIO / "sessions" / f"{session}.json")
    if not S:
        raise FileNotFoundError(f"no archived session {session}")
    text = {p["id"]: p["say"] for p in S["prompts"]}
    takes = []
    for t in (studio.STUDIO / "takes").glob(f"{session}-*"):
        m = studio.read_json(t / "take.json", {}) or {}
        if m.get("from_roll") and m.get("prompt"):
            takes.append((m["from_roll"]["roll"], m["from_roll"]["start"], t.name, m["prompt"]["id"]))
    dest = studio.pathlib.Path(dest or studio.pathlib.Path.home() / f"dev/verabot/film/vo/vera/{session}"); dest.mkdir(parents=True, exist_ok=True)
    result = {}
    order = [p["id"] for p in S["prompts"]]
    for roll in sorted({r for r, *_ in takes}):
        seq = sorted([x for x in takes if x[0] == roll], key=lambda x: x[1])
        # Lines read straight on without a NEXT press have no take. Slot every prompt skipped between one take's line
        # and the next (and after the last) in after it: the words will say whether they were really spoken.
        full = []
        for k, x in enumerate(seq):
            full.append(x)
            i = order.index(x[3]); j = order.index(seq[k + 1][3]) if k + 1 < len(seq) else len(order)
            for lid in order[i + 1:j]:
                if not any(y[3] == lid for y in takes):
                    full.append((roll, None, None, lid))
        wav = studio.STUDIO / "takes" / roll / "dry.wav"
        cc, spans = cuts(wav, [text[lid] for *_, lid in full], cache=studio.STUDIO / "takes" / roll / "transcript.words.json")
        for (r, _, take, lid), c, sp in zip(full, cc, spans):
            if take is None:                       # a line spoken without its own take: give it one, cut from the roll
                if not c:
                    continue
                take = f"{session}-{lid}"; td = studio.take_dir(take)
                if not (td / "dry.wav").exists():
                    export_cut(wav, c[0], c[1], td / "dry.wav")
                    pr = next(p for p in S["prompts"] if p["id"] == lid)
                    studio.write_json(td / "take.json", dict(take=take, device="cut from the roll by its words", recorded=time.strftime("%Y-%m-%d %H:%M:%S"),
                                                             notes="read on without a NEXT press; recovered by align.py", sections={},
                                                             prompt=dict(pr, session=session, index=order.index(lid)), from_roll=dict(roll=r, start=c[0], end=c[1])))
                S.setdefault("selects", {})[lid] = take
            studio.write_json(studio.STUDIO / "takes" / take / "aligned.json", dict(roll=r, cut=c, words_span=sp, method="faster-whisper small.en words, difflib alignment"))
            if c and (S.get("selects") or {}).get(lid) == take:
                export_cut(wav, c[0], c[1], dest / f"{lid}.wav")
                result[lid] = dict(take=take, roll=r, start=c[0], end=c[1], seconds=round(c[1] - c[0], 2))
    studio.write_json(studio.STUDIO / "sessions" / f"{session}.json", S)
    old = studio.read_json(dest / "selects.json", {}) or {}
    studio.write_json(dest / "selects.json", dict(session=session, version=int(old.get("version", 1)) + 1, exported=time.strftime("%Y-%m-%d %H:%M:%S"),
                                                  method="cut at the words: faster-whisper word timestamps aligned to the script; cuts at the quietest 20 ms between lines; 150 ms before, 250 ms after, 30 ms fades",
                                                  lines=result))
    studio.log("recut", session=session, lines=len(result))
    return dict(dest=str(dest), lines=result)
