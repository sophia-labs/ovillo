"""Rolling takes: one continuous recording, cut into per-line takes by the marks Vera presses while it runs.

    start()        opens the mic once, recording <session>-roll-NN
    mark("next")   ends this line's take and moves to the next line
    mark("again")  ends this take and starts another of the same line
    mark("back")   ends this take and goes back a line
                   NEXT out of the last line of a set (ops.set_of) answers set_done, and the page stops the roll there:
                   short sets, so a dead mic or an interruption costs one set at most
    stop()         stops, then cuts the roll at the marks (each cut snapped to the quietest 40 ms within 0.6 s)
                   into takes named <session>-<id>, -2, -3 ... Each gets its prompt and quick numbers, and the
                   last take of each line is circled. Stretches with no voice are dropped.

The roll's own dry.wav is kept untouched; the cuts are copies. State lives in <studio>/.roll.json while rolling.
"""
import difflib, json, re, time, subprocess
import numpy as np
from . import studio, record, ops, analysis

SR = 48000


def path():
    return studio.STUDIO / ".roll.json"


def state():
    R = studio.read_json(path())
    if R and not record.status():  # the recorder died under us
        return dict(R, dead=True)
    return R


def start(device=None):
    if state():
        raise RuntimeError("already rolling")
    P = ops.prompts()
    if not P["prompts"]:
        raise RuntimeError("no prompts loaded")
    n = 1
    while (studio.STUDIO / "takes" / f"{P['session']}-roll-{n:02d}").exists():
        n += 1
    take = f"{P['session']}-roll-{n:02d}"
    rec = record.start(take, device, chime=False, notes="roll")
    wav = studio.STUDIO / "takes" / take / "dry.wav"
    offset = max(0.0, (wav.stat().st_size - 44) / (SR * 3))  # how much audio was already on disk when the clock started
    R = dict(take=take, session=P["session"], started=rec["started"], offset=round(offset, 3), line=P["current"], marks=[])
    studio.write_json(path(), R); studio.log("roll_start", take=take, line=P["prompts"][P["current"]]["id"])
    return R


def mark(kind):
    R = studio.read_json(path())
    if not R:
        raise RuntimeError("not rolling")
    P = ops.prompts(); n = len(P["prompts"])
    t = round(time.time() - R["started"] + R["offset"], 3)
    R["marks"].append(dict(t=t, line=R["line"], kind=kind)); was = R["line"]
    if kind == "next":
        R["line"] = min(n - 1, R["line"] + 1)
    elif kind == "back":
        R["line"] = max(0, R["line"] - 1)
    P["current"] = R["line"]
    studio.write_json(studio.STUDIO / "prompts.json", P); studio.write_json(path(), R)
    done = kind == "next" and (was == n - 1 or ops.set_of(P, was) != ops.set_of(P, R["line"]))
    return dict(t=t, line=R["line"], id=P["prompts"][R["line"]]["id"], kind=kind,
                takes_this_roll=sum(1 for m in R["marks"] if m["line"] == R["line"]) + 1,
                set=ops.set_of(P, was), set_done=done, next_set=None if was == n - 1 else ops.set_of(P, R["line"]))


def _stopped_after(take):
    """Seconds the recorder ran, from the log, for a roll whose recorder was stopped some other way."""
    for line in reversed((studio.STUDIO / "log.jsonl").read_text().splitlines()[-400:]):
        if '"record_stop"' in line and f'"{take}"' in line:
            return __import__("json").loads(line).get("seconds")


def _quiet(a, t, lo, hi, win=0.6):
    """The centre of the quietest 40 ms within +-win s of t, kept inside [lo, hi]."""
    n = int(0.04 * SR); a0 = max(int(lo * SR), int((t - win) * SR)); a1 = min(int(hi * SR), int((t + win) * SR))
    if a1 - a0 < 2 * n:
        return t
    x = a[a0:a1] ** 2; e = np.convolve(x, np.ones(n) / n, mode="valid")
    return (a0 + int(np.argmin(e)) + n // 2) / SR


def halt():
    """Stop the recorder and close the roll: quick. The marks are saved in the roll's take.json for cut()."""
    R = studio.read_json(path())
    if not R:
        raise RuntimeError("not rolling")
    cur = record.status()
    # the recorder may already be gone (died, or stopped by a bare record-stop): cut what reached the disk
    r = record.stop() if cur and cur["take"] == R["take"] else {}
    path().unlink(missing_ok=True)
    meta = studio.read_json(studio.STUDIO / "takes" / R["take"] / "take.json", {}) or {}
    meta["roll"] = dict(marks=R["marks"], first_line=R["marks"][0]["line"] if R["marks"] else R["line"], last_line=R["line"],
                        offset=R["offset"], ran=r.get("seconds") or _stopped_after(R["take"]), session=R["session"])
    studio.write_json(studio.STUDIO / "takes" / R["take"] / "take.json", meta)
    return dict(roll=R["take"], marks=len(R["marks"]), last_line=R["line"])


def stop():
    return cut(halt()["roll"])


def _norm(w):
    return re.sub(r"[^a-z]", "", w.lower())


def _like(a, b):
    a, b = _norm(a), _norm(b)
    return bool(a) and bool(b) and difflib.SequenceMatcher(None, a, b).ratio() >= 0.6


def rescue_edges(segs, texts, words, end, pre=0.15, post=0.25, reach=4):
    """Widen a cut over words just outside it that the line expects at that edge and no other take has claimed.
    Vera hams: 'But... in Buckinghamshire', 'and all twelve wrote... ay.' The aligner can read a long pause as a line
    boundary and leave the word before or after it out. Returns (segs, [(line index, edge, words added)])."""
    out, fixed = list(segs), []
    for k, (a0, a1, i) in enumerate(out):
        exp = texts[k].split()
        if not exp:
            continue
        lo = out[k - 1][1] if k else 0.0
        hi = out[k + 1][0] if k + 1 < len(out) else end
        inside = [n for n, w in enumerate(words) if a0 <= (w["start"] + w["end"]) / 2 <= a1]  # a cut may land mid-word
        if not inside:
            continue
        first, last = inside[0], inside[-1]
        # start: which expected word does the take open on? the ones before it should sit in the free gap just before
        j = next((n for n in range(min(reach + 1, len(exp))) if _like(exp[n], words[first]["word"])), 0)
        if j:
            gap = [w for w in words[max(0, first - j):first] if w["start"] >= lo]
            if len(gap) == j and all(_like(e, g["word"]) for e, g in zip(exp[:j], gap)):
                a0 = max(lo, gap[0]["start"] - pre); fixed.append((i, "start", " ".join(g["word"] for g in gap)))
        # end: which expected word does it close on? the ones after it should sit in the free gap just after
        j = next((n for n in range(min(reach + 1, len(exp))) if _like(exp[-1 - n], words[last]["word"])), 0)
        if j:
            gap = [w for w in words[last + 1:last + 1 + j] if w["end"] <= hi]
            if len(gap) == j and all(_like(e, g["word"]) for e, g in zip(exp[-j:], gap)):
                a1 = min(hi, gap[-1]["end"] + post); fixed.append((i, "end", " ".join(g["word"] for g in gap)))
        out[k] = (a0, a1, i)
    return out, fixed


def _session(Rm):
    """The prompts the roll was read from: the live list, or, if another session has been loaded since, its archive."""
    P = ops.prompts()
    if Rm.get("session") and P.get("session") != Rm["session"]:
        P = studio.read_json(studio.STUDIO / "sessions" / f"{Rm['session']}.json")
    return P


def cut(take):
    """Cut a halted roll into per-line takes (slow: it transcribes). serve runs this in its own process, one at a time,
    so the page stays live and the next set can roll while this one is cut."""
    import fcntl
    lock = open(studio.STUDIO / ".cut.lock", "w"); fcntl.flock(lock, fcntl.LOCK_EX)
    meta = studio.read_json(studio.STUDIO / "takes" / take / "take.json", {}) or {}
    Rm = meta["roll"]; R = dict(take=take, marks=Rm["marks"], offset=Rm["offset"], line=Rm.get("last_line", Rm["first_line"]))
    r = dict(device=meta.get("device", ""))
    src = studio.STUDIO / "takes" / R["take"] / "dry.wav"
    s = analysis.load(src); a = s.values[0]; end = s.duration
    # Marks are wall-clock times. A mic that drops buffers (the iPhone's Continuity mic loses 5-8%) leaves a file
    # shorter than the clock, so the marks are squeezed onto the file's own timeline.
    ran = Rm.get("ran"); k = 1.0
    if ran and ran > 5 and "clock_scale" not in Rm:
        k = (end - R["offset"]) / ran
        k = k if 0.8 < k < 0.995 else 1.0
        if k != 1.0:
            R["marks"] = [dict(m, t=round(R["offset"] + (m["t"] - R["offset"]) * k, 3)) for m in R["marks"]]
        meta["roll"] = dict(Rm, marks=R["marks"], clock_scale=round(k, 4))
        studio.write_json(studio.STUDIO / "takes" / R["take"] / "take.json", meta)
    # the stretches: [previous cut, this mark] for the line the mark closed; the last runs to the end
    cuts, segs, start = [0.0], [], 0.0
    for m in R["marks"]:
        c = _quiet(a, min(m["t"], end), start + 0.2, end)
        segs.append((start, c, m["line"])); start = c
    segs.append((start, end, R["line"]))
    P = _session(Rm); made = []
    # Marks are pressed by hand and often late, so the words decide the cuts when they can (align.py); the marks
    # still say which line each stretch is. If transcription isn't available, the mark cuts stand.
    try:
        from . import align
        cc, _ = align.cuts(src, [P["prompts"][i]["say"] for (_, _, i) in segs], cache=studio.STUDIO / "takes" / R["take"] / "transcript.words.json",
                           seq_who=[P["prompts"][i].get("who") for (_, _, i) in segs])
        segs = [(c[0], c[1], i) if c else (a0, a1, i) for (a0, a1, i), c in zip(segs, cc)]; method = "words"
        cache = studio.STUDIO / "takes" / R["take"] / "transcript.words.json"
        segs, rescued = rescue_edges(segs, [P["prompts"][i]["say"] for (_, _, i) in segs], json.loads(cache.read_text()), end)
        if rescued:
            studio.log("roll_rescue", take=R["take"], edges=[dict(line=P["prompts"][i]["id"], edge=e, words=w) for i, e, w in rescued])
    except Exception as e:
        method = f"marks ({e.__class__.__name__}: {str(e)[:80]})"
    for a0, a1, i in segs:
        if a1 - a0 < 0.3:
            continue
        x = analysis.part(s, a0, a1); _, f = analysis.pitch(x, two_pass=False)
        if (f > 0).sum() < 8:  # no voice in it: a double press, or silence
            continue
        take = ops.prompt_take(P, i)
        out = studio.take_dir(take) / "dry.wav"
        subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(src), "-ss", f"{a0:.3f}", "-to", f"{a1:.3f}",
                        "-c:a", "pcm_s24le", str(out)], check=True)
        studio.write_json(studio.take_dir(take) / "take.json", dict(
            take=take, device=r.get("device", ""), recorded=time.strftime("%Y-%m-%d %H:%M:%S"), notes="",
            sections={}, prompt=dict(P["prompts"][i], session=P["session"], index=i),
            from_roll=dict(roll=R["take"], start=round(a0, 3), end=round(a1, 3))))
        made.append(dict(take=take, line=P["prompts"][i]["id"], start=round(a0, 2), end=round(a1, 2)))
    # circle the last take of each line
    P = _session(Rm); P.setdefault("selects", {})  # re-read: only the selects change here
    for m in made:
        P["selects"][m["line"]] = m["take"]
    live = ops.prompts().get("session") == P["session"]
    studio.write_json(studio.STUDIO / "prompts.json" if live else studio.d("sessions") / f"{P['session']}.json", P)
    studio.log("roll_stop", take=R["take"], seconds=round(end, 1), takes=len(made), marks=len(R["marks"]), cut_by=method, clock_scale=Rm.get("clock_scale", round(k, 4)))

    for m in made:
        try:
            ops.quick(m["take"])
        except Exception as e:
            studio.log("error", take=m["take"], error=f"quick: {e}")
    return dict(roll=R["take"], seconds=round(end, 1), takes=made)
