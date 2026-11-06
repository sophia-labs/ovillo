"""The studio: one directory both of us read and write, on disk.

    <studio>/takes/<take>/dry.wav           what Vera said (48 kHz mono, never altered)
    <studio>/takes/<take>/take.json         device, time, notes, sections {name: [t0, t1]}
    <studio>/takes/<take>/analysis/         report.json, overview.png
    <studio>/bodies/<body>.json             Body specs: ovillo-body presets, plus our own edits
    <studio>/renders/<take>/<body>/         wet.wav, receipt.json, body.json (as rendered), compare.json, compare.png
    <studio>/log.jsonl                      what happened, newest last (the page shows it)

The studio is $OVILLO_VOICE_STUDIO, or else ~/dev/verabot/film/audio/voice/studio (the film owns the takes).
The Body renderer is $OVILLO_BODY, or else the ovillo-body built in this checkout's cargo target.
"""
import json, os, pathlib, time

HERE = pathlib.Path(__file__).resolve().parent
CHECKOUT = HERE.parents[1]
STUDIO = pathlib.Path(os.environ.get("OVILLO_VOICE_STUDIO", pathlib.Path.home() / "dev/verabot/film/audio/voice/studio")).expanduser()


def d(*parts):
    p = STUDIO.joinpath(*parts); p.mkdir(parents=True, exist_ok=True); return p


def take_dir(take):
    return d("takes", take)


def render_dir(take, body):
    return d("renders", take, body)


def body_bin():
    env = os.environ.get("OVILLO_BODY")
    if env:
        return pathlib.Path(env)
    for prof in ("release", "debug"):
        p = CHECKOUT / "ovillo/target" / prof / "ovillo-body"
        if p.exists():
            return p
    return None


def read_json(p, default=None):
    try:
        return json.loads(pathlib.Path(p).read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return default


def write_json(p, obj):
    p = pathlib.Path(p); p.parent.mkdir(parents=True, exist_ok=True)
    tmp = p.with_suffix(p.suffix + ".tmp"); tmp.write_text(json.dumps(obj, indent=1)); tmp.replace(p)


def log(event, **kw):
    d()
    with open(STUDIO / "log.jsonl", "a") as f:
        f.write(json.dumps(dict(t=time.strftime("%Y-%m-%d %H:%M:%S"), event=event, **kw)) + "\n")


def index():
    """Everything in the studio, as the page and the MCP server see it."""
    takes = []
    for t in sorted(d("takes").iterdir()) if d("takes").exists() else []:
        if not (t / "dry.wav").exists():
            continue
        renders = []
        rd = STUDIO / "renders" / t.name
        if rd.exists():
            for b in sorted(rd.iterdir()):
                if (b / "wet.wav").exists():
                    renders.append(dict(body=b.name, mtime=(b / "wet.wav").stat().st_mtime,
                                        compare=(b / "compare.png").exists(), receipt=read_json(b / "receipt.json"),
                                        summary=(read_json(b / "compare.json") or {}).get("summary")))
        meta = read_json(t / "take.json", {})
        takes.append(dict(take=t.name, mtime=(t / "dry.wav").stat().st_mtime, meta=meta,
                          analysed=(t / "analysis/report.json").exists(),
                          summary=(read_json(t / "analysis/report.json") or {}).get("summary"), quick=meta.get("quick"), renders=renders))
    bodies = sorted(p.stem for p in d("bodies").glob("*.json"))
    logp = STUDIO / "log.jsonl"
    tail = [json.loads(l) for l in logp.read_text().splitlines()[-30:]] if logp.exists() else []
    rec = read_json(STUDIO / ".recording.json")
    prompts = read_json(STUDIO / "prompts.json", {"session": None, "prompts": [], "current": 0})
    return dict(studio=str(STUDIO), takes=takes, bodies=bodies, log=tail, recording=rec, prompts=prompts,
                body_bin=str(body_bin()) if body_bin() else None)
