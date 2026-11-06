"""Rolling takes: one continuous recording, cut into per-line takes by the marks Vera presses while it runs.

    start()        opens the mic once, recording <session>-roll-NN
    mark("next")   ends this line's take and moves to the next line
    mark("again")  ends this take and starts another of the same line
    mark("back")   ends this take and goes back a line
    stop()         stops, then cuts the roll at the marks (each cut snapped to the quietest 40 ms within 0.6 s)
                   into takes named <session>-<id>, -2, -3 ... Each gets its prompt and quick numbers, and the
                   last take of each line is circled. Stretches with no voice are dropped.

The roll's own dry.wav is kept untouched; the cuts are copies. State lives in <studio>/.roll.json while rolling.
"""
import time, subprocess, threading
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
    R["marks"].append(dict(t=t, line=R["line"], kind=kind))
    if kind == "next":
        R["line"] = min(n - 1, R["line"] + 1)
    elif kind == "back":
        R["line"] = max(0, R["line"] - 1)
    P["current"] = R["line"]
    studio.write_json(studio.STUDIO / "prompts.json", P); studio.write_json(path(), R)
    return dict(t=t, line=R["line"], id=P["prompts"][R["line"]]["id"], kind=kind,
                takes_this_roll=sum(1 for m in R["marks"] if m["line"] == R["line"]) + 1)


def _quiet(a, t, lo, hi, win=0.6):
    """The centre of the quietest 40 ms within +-win s of t, kept inside [lo, hi]."""
    n = int(0.04 * SR); a0 = max(int(lo * SR), int((t - win) * SR)); a1 = min(int(hi * SR), int((t + win) * SR))
    if a1 - a0 < 2 * n:
        return t
    x = a[a0:a1] ** 2; e = np.convolve(x, np.ones(n) / n, mode="valid")
    return (a0 + int(np.argmin(e)) + n // 2) / SR


def stop():
    R = studio.read_json(path())
    if not R:
        raise RuntimeError("not rolling")
    r = record.stop(); path().unlink(missing_ok=True)
    src = studio.STUDIO / "takes" / R["take"] / "dry.wav"
    s = analysis.load(src); a = s.values[0]; end = s.duration
    # the stretches: [previous cut, this mark] for the line the mark closed; the last runs to the end
    cuts, segs, start = [0.0], [], 0.0
    for m in R["marks"]:
        c = _quiet(a, min(m["t"], end), start + 0.2, end)
        segs.append((start, c, m["line"])); start = c
    segs.append((start, end, R["line"]))
    P = ops.prompts(); made = []
    # Marks are pressed by hand and often late, so the words decide the cuts when they can (align.py); the marks
    # still say which line each stretch is. If transcription isn't available, the mark cuts stand.
    try:
        from . import align
        cc, _ = align.cuts(src, [P["prompts"][i]["say"] for (_, _, i) in segs], cache=studio.STUDIO / "takes" / R["take"] / "transcript.words.json")
        segs = [(c[0], c[1], i) if c else (a0, a1, i) for (a0, a1, i), c in zip(segs, cc)]; method = "words"
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
    P = ops.prompts(); P.setdefault("selects", {})
    for m in made:
        P["selects"][m["line"]] = m["take"]
    studio.write_json(studio.STUDIO / "prompts.json", P)
    studio.log("roll_stop", take=R["take"], seconds=round(end, 1), takes=len(made), marks=len(R["marks"]), cut_by=method)

    def measure():
        for m in made:
            try:
                ops.quick(m["take"])
            except Exception as e:
                studio.log("error", take=m["take"], error=f"quick: {e}")
    threading.Thread(target=measure, daemon=True).start()
    return dict(roll=R["take"], seconds=round(end, 1), takes=made)
