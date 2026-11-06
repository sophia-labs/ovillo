"""ovillo-voice: Vera speaks, a Body shapes it.

    ovillo-voice devices                              input devices
    ovillo-voice record start <take> [--device D] [--max S] [--notes N]
    ovillo-voice record stop | status
    ovillo-voice adopt <take> <file.wav>              bring an existing recording in as a take
    ovillo-voice segments <take>                      stretches of sound between pauses
    ovillo-voice sections <take> name=t0:t1 ...       label parts of a take (name=- removes one)
    ovillo-voice analyse <take>                       report.json + overview.png
    ovillo-voice presets                              the renderer's bodies, copied into the studio
    ovillo-voice describe                             the Body surface: every knob
    ovillo-voice render <take> <body> [--as NAME] [--set id=v ...]   then compares dry with wet
    ovillo-voice shift <take> <ratio> [--as NAME]       NOT physics: Praat formant shift (pitch kept), a labelled comparison
    ovillo-voice compare <take> <body>
    ovillo-voice play <take> [<body>]
    ovillo-voice prompts set <session> <file.json>    what Vera is asked to say next (a JSON list of {id, say, kind, hint, target_hz})
    ovillo-voice prompts show | results [<session>]
    ovillo-voice script <session> R1-01..R1-18,G-01..G-06 [--order character|scene]   the film's lines as prompts
    ovillo-voice select [<take>]                      circle a take for the current line (default: its newest)
    ovillo-voice export-selects [<dir>]               circled takes, trimmed, to <dir>/<line>.wav (default verabot/film/vo/vera)
    ovillo-voice fx <take> <chain>                    a character chain (mom, hot, vera, gatekeeper): Ariadne Body, then shaping, then the line
    ovillo-voice scene <session> [--dry] [--gap LINE=s ...]   the session's circled takes end to end, each through its character
    ovillo-voice babel <name> <session|take ...> [--seconds 60]   many takes as many speakers, entering faster and faster
    ovillo-voice unison <take> [--copies 9]          one line as a choir of herself, in Ariadne's cathedral
    ovillo-voice cutroll <roll>                       cut a halted roll into per-line takes (the studio runs this itself)
    ovillo-voice recut <session> [<dir>]              re-export a rolled session's circled takes cut at the words (faster-whisper)
    ovillo-voice quick <take>                         fast numbers for one short take
    ovillo-voice index                                the studio as JSON
    ovillo-voice serve [--port 8023]                  the studio page
"""
import argparse, json, sys
from . import ops, record, roll, studio


def kv(items):
    out = {}
    for it in items or []:
        k, _, v = it.partition("=")
        try:
            out[k] = json.loads(v)
        except json.JSONDecodeError:
            out[k] = v
    return out


def main(argv=None):
    ap = argparse.ArgumentParser(prog="ovillo-voice", description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sp = ap.add_subparsers(dest="cmd", required=True)
    sp.add_parser("devices")
    r = sp.add_parser("record"); r.add_argument("action", choices=["start", "stop", "status"]); r.add_argument("take", nargs="?")
    r.add_argument("--device"); r.add_argument("--max", type=float, default=900); r.add_argument("--notes", default="")
    a = sp.add_parser("adopt"); a.add_argument("take"); a.add_argument("wav"); a.add_argument("--notes", default="")
    s = sp.add_parser("segments"); s.add_argument("take")
    s = sp.add_parser("sections"); s.add_argument("take"); s.add_argument("spans", nargs="*"); s.add_argument("--replace", action="store_true")
    s = sp.add_parser("analyse", aliases=["analyze"]); s.add_argument("take")
    sp.add_parser("presets"); sp.add_parser("describe"); sp.add_parser("index")
    s = sp.add_parser("prompts"); s.add_argument("action", choices=["set", "show", "results"]); s.add_argument("session", nargs="?"); s.add_argument("file", nargs="?")
    s = sp.add_parser("quick"); s.add_argument("take")
    s = sp.add_parser("recut"); s.add_argument("session"); s.add_argument("dest", nargs="?")
    s = sp.add_parser("cutroll"); s.add_argument("roll")
    s = sp.add_parser("fx"); s.add_argument("take"); s.add_argument("chain")
    s = sp.add_parser("babel"); s.add_argument("name"); s.add_argument("sources", nargs="+"); s.add_argument("--seconds", type=float, default=60.0)
    s = sp.add_parser("unison"); s.add_argument("take"); s.add_argument("--copies", type=int, default=9)
    s = sp.add_parser("scene"); s.add_argument("session"); s.add_argument("--dry", action="store_true"); s.add_argument("--gap", action="append")
    s = sp.add_parser("script"); s.add_argument("session"); s.add_argument("ids"); s.add_argument("--order", default="scene", choices=["scene", "character"])
    s = sp.add_parser("select"); s.add_argument("take", nargs="?")
    s = sp.add_parser("export-selects"); s.add_argument("dest", nargs="?")
    s = sp.add_parser("render"); s.add_argument("take"); s.add_argument("body"); s.add_argument("--as", dest="as_name"); s.add_argument("--set", action="append")
    s.add_argument("--no-compare", action="store_true")
    s = sp.add_parser("compare"); s.add_argument("take"); s.add_argument("body")
    s = sp.add_parser("shift"); s.add_argument("take"); s.add_argument("ratio", type=float); s.add_argument("--as", dest="as_name")
    s = sp.add_parser("play"); s.add_argument("take"); s.add_argument("body", nargs="?")
    s = sp.add_parser("serve"); s.add_argument("--port", type=int, default=8023)
    x = ap.parse_args(argv)

    if x.cmd == "devices":
        res = record.devices()
    elif x.cmd == "record":
        res = {"start": lambda: record.start(x.take, x.device, x.max, x.notes), "stop": lambda: (roll.stop() if roll.state() else record.stop()), "status": lambda: record.status() or "not recording"}[x.action]()
    elif x.cmd == "adopt":
        res = record.adopt(x.take, x.wav, x.notes)
    elif x.cmd == "segments":
        res = ops.segments(x.take)
        print(f"noise floor {res['noise_floor_db']} dB, gate {res['gate_db']} dB")
        for i, e in enumerate(res["segments"]):
            f0 = f"  F0 {e['f0_median']:4d} Hz ({e['f0_start']}->{e['f0_end']}, {e['f0_min']}-{e['f0_max']})" if "f0_median" in e else ""
            print(f"  [{i:2d}] {e['start']:7.2f}-{e['end']:7.2f} ({e['seconds']:5.2f} s) voiced {e['voiced_pct']:3d}%{f0}")
        return
    elif x.cmd == "sections":
        spans = {}
        for it in x.spans:
            k, _, v = it.partition("=")
            spans[k] = None if v == "-" else [float(t) for t in v.split(":")]
        res = ops.set_sections(x.take, spans, x.replace)
    elif x.cmd in ("analyse", "analyze"):
        res = ops.analyse(x.take)
    elif x.cmd == "presets":
        res = ops.presets()
    elif x.cmd == "describe":
        res = ops.describe()
    elif x.cmd == "prompts":
        if x.action == "set":
            items = json.load(open(x.file)); intro = ""
            if isinstance(items, dict):
                intro, items = items.get("intro", ""), items["prompts"]
            res = ops.set_prompts(x.session, items, intro)
        else:
            res = ops.prompts() if x.action == "show" else ops.prompt_results(x.session)
    elif x.cmd == "script":
        res = ops.script_session(x.session, x.ids, x.order)
    elif x.cmd == "select":
        res = ops.select_take(x.take)
    elif x.cmd == "export-selects":
        res = ops.export_selects(x.dest)
    elif x.cmd == "babel":
        from . import fx
        takes = []
        for src in x.sources:   # a session name expands to its takes; anything else is a take
            ses = studio.read_json(studio.STUDIO / "sessions" / f"{src}.json") or (ops.prompts() if ops.prompts().get("session") == src else None)
            if ses:
                takes += [v for v in (ses.get("selects") or {}).values()] or sorted(p.name for p in (studio.STUDIO / "takes").glob(f"{src}-*") if "roll" not in p.name and "scene" not in p.name)
            else:
                takes.append(src)
        res = fx.babel(takes, name=x.name, seconds=x.seconds)
    elif x.cmd == "unison":
        from . import fx
        res = fx.unison(x.take, copies=x.copies)
    elif x.cmd == "fx":
        from . import fx
        res = str(fx.render(x.take, x.chain))
    elif x.cmd == "scene":
        from . import fx
        res = fx.scene(x.session, gaps={k: float(v) for k, v in (g.split("=") for g in (x.gap or []))}, chains=not x.dry)
    elif x.cmd == "cutroll":
        res = roll.cut(x.roll)
    elif x.cmd == "recut":
        from . import align
        res = align.recut_session(x.session, x.dest)
    elif x.cmd == "quick":
        res = ops.quick(x.take)
    elif x.cmd == "index":
        res = studio.index()
    elif x.cmd == "render":
        res = ops.render(x.take, x.body, kv(x.set), x.as_name, not x.no_compare)
    elif x.cmd == "shift":
        res = ops.formant_shift(x.take, x.ratio, x.as_name)
    elif x.cmd == "compare":
        res = ops.compare(x.take, x.body)
    elif x.cmd == "play":
        res = ops.play(x.take, x.body)
    elif x.cmd == "serve":
        from . import serve
        return serve.main(x.port)
    print(json.dumps(res, indent=1) if not isinstance(res, str) else res)


if __name__ == "__main__":
    sys.exit(main())
