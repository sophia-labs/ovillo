"""The operations, one function each. The CLI, the MCP server and the studio page call the same functions here."""
import json, pathlib, subprocess, time
from . import studio, record, analysis


def take_meta(take):
    return studio.read_json(studio.take_dir(take) / "take.json", {}) or {}


def dry(take):
    p = studio.STUDIO / "takes" / take / "dry.wav"
    if not p.exists():
        raise FileNotFoundError(f"no take {take!r} (looked for {p})")
    return p


def segments(take):
    return analysis.segments(analysis.load(dry(take)))


def set_sections(take, sections, replace=False):
    """sections: {name: [t0, t1]}. Unknown names are allowed (they're just labels); known ones drive the analysis."""
    meta = take_meta(take); cur = {} if replace else dict(meta.get("sections", {}))
    for k, v in sections.items():
        if v is None:
            cur.pop(k, None)
        else:
            cur[k] = [round(float(v[0]), 3), round(float(v[1]), 3)]
    meta["sections"] = dict(sorted(cur.items(), key=lambda kv: kv[1][0])); meta.setdefault("take", take)
    studio.write_json(studio.take_dir(take) / "take.json", meta)
    studio.log("sections", take=take, names=list(sections))
    return meta["sections"]


def analyse(take):
    meta = take_meta(take); secs = meta.get("sections") or {}
    t0 = time.time(); R = analysis.analyse_take(dry(take), secs)
    out = studio.take_dir(take) / "analysis"; out.mkdir(exist_ok=True)
    studio.write_json(out / "report.json", R)
    analysis.plot_take(dry(take), R, secs, out / "overview.png")
    studio.log("analyse", take=take, seconds=round(time.time() - t0, 1), summary=R["summary"])
    return dict(report=str(out / "report.json"), png=str(out / "overview.png"), summary=R["summary"])


def _body_bin():
    b = studio.body_bin()
    if not b:
        raise RuntimeError("ovillo-body is not built yet (cd ovillo && cargo build --release -p ovillo-body), or set $OVILLO_BODY")
    return b


def describe():
    return json.loads(subprocess.run([str(_body_bin()), "describe"], capture_output=True, text=True, check=True).stdout)


def presets(write=True):
    """The renderer's presets. With write, each lands in <studio>/bodies/ unless a body of that name is already there (ours win)."""
    b = _body_bin(); out = subprocess.run([str(b), "presets"], capture_output=True, text=True, check=True).stdout
    names = {ln.split(None, 1)[0]: (ln.split(None, 1) + [""])[1].strip() for ln in out.splitlines() if ln.strip() and not ln[0].isspace()}
    if write:
        tmp = studio.d(".presets"); subprocess.run([str(b), "presets", "--write", str(tmp)], check=True, capture_output=True)
        for p in tmp.glob("*.json"):
            dst = studio.d("bodies") / p.name
            if not dst.exists():
                dst.write_text(p.read_text())
    return names


def bodies():
    out = {p.stem: studio.read_json(p) for p in sorted(studio.d("bodies").glob("*.json"))}
    return out


def save_body(name, spec, note=""):
    if isinstance(spec, str):
        spec = json.loads(spec)
    studio.write_json(studio.d("bodies") / f"{name}.json", spec)
    studio.log("save_body", body=name, note=note)
    return str(studio.STUDIO / "bodies" / f"{name}.json")


def render(take, body, sets=None, as_name=None, do_compare=True):
    """Render a take through a Body. body: a studio body name, a path, or a renderer preset name.
    sets: {id: value} overrides. The result is renders/<take>/<as_name or body>/."""
    b = _body_bin(); src = dry(take)
    bp = studio.STUDIO / "bodies" / f"{body}.json"
    body_arg = str(bp) if bp.exists() else body
    name = as_name or (pathlib.Path(body).stem if body.endswith(".json") else body)
    out = studio.render_dir(take, name)
    cmd = [str(b), "render", "--body", body_arg, "--in", str(src), "--out", str(out / "wet.wav"), "--receipt", str(out / "receipt.json")]
    for k, v in (sets or {}).items():
        cmd += ["--set", f"{k}={v}"]
    t0 = time.time(); p = subprocess.run(cmd, capture_output=True, text=True)
    if p.returncode:
        raise RuntimeError(f"ovillo-body failed ({p.returncode}): {p.stderr.strip()[-2000:]}")
    wall = time.time() - t0
    spec = studio.read_json(bp) if bp.exists() else {"preset": body}
    studio.write_json(out / "body.json", dict(body=body, set=sets or {}, spec=spec, rendered=time.strftime("%Y-%m-%d %H:%M:%S"),
                                              wall_s=round(wall, 2), stderr=p.stderr.strip()[-1000:]))
    res = dict(take=take, body=name, wet=str(out / "wet.wav"), receipt=studio.read_json(out / "receipt.json"), wall_s=round(wall, 2))
    if do_compare:
        res["compare"] = compare(take, name)["summary"]
    studio.log("render", take=take, body=name, set=sets or {}, wall_s=round(wall, 2), summary=res.get("compare"))
    return res


def compare(take, body):
    out = studio.STUDIO / "renders" / take / body
    if not (out / "wet.wav").exists():
        raise FileNotFoundError(f"no render of {take!r} through {body!r}")
    R = analysis.compare(dry(take), out / "wet.wav", out / "compare.png", title=f"{take} through {body}")
    studio.write_json(out / "compare.json", R)
    return dict(json=str(out / "compare.json"), png=str(out / "compare.png"), summary=R["summary"], delta=R["delta"], transfer=R["transfer"])


def play(take, body=None):
    p = (studio.STUDIO / "renders" / take / body / "wet.wav") if body else dry(take)
    subprocess.Popen(["afplay", str(p)]); return str(p)


# ---------- prompts: Claude writes what to say, Vera records each one as its own take

def prompts():
    return studio.read_json(studio.STUDIO / "prompts.json", {"session": None, "prompts": [], "current": 0})


def set_prompts(session, items, intro=""):
    """items: [{id, say, kind?, hint?, target_hz?: [lo, hi], seconds?}]. kind is an analysis key (ah, passage, ...)."""
    for i, p in enumerate(items):
        p.setdefault("id", f"{i + 1:02d}")
    P = dict(session=session, intro=intro, prompts=items, current=0)
    studio.write_json(studio.STUDIO / "prompts.json", P); studio.log("prompts", session=session, n=len(items))
    return P


def goto_prompt(i):
    P = prompts(); P["current"] = max(0, min(len(P["prompts"]) - 1, int(i)))
    studio.write_json(studio.STUDIO / "prompts.json", P); return P


def prompt_take(P, i, retake=False):
    """The take name for prompt i: <session>-<id>, then -2, -3 ... for retakes."""
    base = f"{P['session']}-{P['prompts'][i]['id']}"
    if not retake and not (studio.STUDIO / "takes" / base / "dry.wav").exists():
        return base
    n = 2
    while (studio.STUDIO / "takes" / f"{base}-{n}" / "dry.wav").exists():
        n += 1
    return f"{base}-{n}"


def quick(take):
    """Fast numbers for one short take, right after it's recorded: pitch, and resonance appropriate to what it was."""
    meta = take_meta(take); kind = (meta.get("prompt") or {}).get("kind")
    s = analysis.load(dry(take))
    segs = analysis.segments(s)["segments"]
    if segs:  # the sound itself, without the silence around it
        s = analysis.part(s, segs[0]["start"], segs[-1]["end"])
    t, f = analysis.pitch(s, floor=45, ceil=900)
    q = dict(kind=kind, seconds=round(s.duration, 2), f0=analysis.f0_stats(f), level_dbfs=analysis.rms_dbfs(s))
    if kind and kind.split("_")[0] in ("ah", "ee", "oo", "uh", "mm") and s.duration > 0.8:  # held sounds: ah, ee_v, ah_low ...
        fm = analysis.formants_steady(s); q["formants_hz"] = fm and fm["F"]
    elif kind not in ("room", "ss", "sh") and s.duration > 1.0:
        c = analysis.connected(s); q.update(apparent_vtl_cm=c["apparent_vtl_cm"], formant_medians_hz=c["formant_medians_hz"],
                                            alpha_ratio_db=c["alpha_ratio_db"], cpps_db=c["cpps_db"])
    meta["quick"] = q
    if kind and not meta.get("sections"):
        meta["sections"] = {kind: [round(segs[0]["start"], 3), round(segs[-1]["end"], 3)]} if segs else {}
    studio.write_json(studio.take_dir(take) / "take.json", meta)
    studio.log("quick", take=take, f0=(q["f0"] or {}).get("median_hz"), vtl=q.get("apparent_vtl_cm"))
    return q


def prompt_results(session=None):
    """Every prompt in a session with its takes' quick numbers, newest take first."""
    P = prompts(); session = session or P["session"]; out = []
    takes = [(t, studio.read_json(t / "take.json", {}) or {}) for t in (studio.STUDIO / "takes").glob(f"{session}-*") if (t / "dry.wav").exists()]
    for p in P["prompts"] if P["session"] == session else []:
        mine = sorted([(t, m) for t, m in takes if (m.get("prompt") or {}).get("id") == p["id"]], key=lambda tm: tm[0].stat().st_mtime, reverse=True)
        out.append(dict(id=p["id"], kind=p.get("kind"), say=p["say"], takes=[dict(take=t.name, quick=m.get("quick")) for t, m in mine]))
    return dict(session=session, prompts=out)


def formant_shift(take, ratio, as_name=None, do_compare=True):
    """NOT Ovillo and NOT physics: Praat's "Change gender" (PSOLA resynthesis) with every formant scaled by `ratio`
    and her own pitch kept. It's here as a labelled comparison for the physical Bodies, like ps1_chain.py is for the
    PS1 sound. The render's body.json says so."""
    from parselmouth.praat import call
    name = as_name or f"praat-x{ratio:g}"; out = studio.render_dir(take, name)
    t0 = time.time(); y = call(analysis.load(dry(take)), "Change gender", 75, 600, float(ratio), 0, 1, 1)
    y.save(str(out / "wet.wav"), "WAV")
    studio.write_json(out / "body.json", dict(body=name, non_physics=True, method="Praat 'Change gender' (PSOLA): formants x ratio, pitch median kept",
                                              ratio=ratio, rendered=time.strftime("%Y-%m-%d %H:%M:%S"), wall_s=round(time.time() - t0, 2)))
    (out / "receipt.json").unlink(missing_ok=True)
    res = dict(take=take, body=name, wet=str(out / "wet.wav"), non_physics=True)
    if do_compare:
        res["compare"] = compare(take, name)["summary"]
    studio.log("formant_shift", take=take, body=name, ratio=ratio, non_physics=True, summary=res.get("compare"))
    return res
