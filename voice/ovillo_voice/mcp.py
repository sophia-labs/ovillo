"""ovillo-voice-mcp: an MCP server on stdio for the voice studio.

The JSON-RPC 2.0 is written by hand (no SDK), one message per line, like ovillo-mcp. It implements initialize, ping,
tools/list and tools/call, and ignores notifications. Each tool is one function in ops / record / studio, so the
CLI, the studio page and this server all act on the same files.
"""
import base64, json, sys, traceback
from . import ops, record, studio

VERSIONS = ["2025-06-18", "2025-03-26", "2024-11-05"]
INSTRUCTIONS = """\
The Ovillo voice studio. Vera (she/her) performs every spoken part of her film by speaking into an Ovillo Body: \
her own waveform is the energy entering a physical World at a boundary, and the Body shapes it. Her performance \
is never resynthesised. The studio is a directory you share with her, and she watches it on a local page \
(ovillo-voice serve, http://127.0.0.1:8023):
- takes/<take>/dry.wav: what she said, never altered.
- bodies/*.json: Body specs.
- renders/<take>/<body>/: wet.wav, receipt.json, compare.json and compare.png.
- log.jsonl: what happened, shown on her page.

The loop:
1. Record or pick a take.
2. Analyse it.
3. Read the Body surface (body_surface).
4. Render the take through a Body with overrides (render, set={...}).
5. Read the comparison: apparent vocal-tract length dry to wet (the size cue), alpha ratio (brightness), F0 (should not move), the transfer |H| peaks and notches the Body imposed, and coherence (below 1 where the Body is nonlinear or noisy).
6. Iterate.

Ovillo's rule: never invent physics or numbers. Cite the receipt or the measurement. Recording uses Vera's microphone, so start it only when she asks."""

IMG = {"type": "boolean", "description": "Also return the plot as an image (default true)."}
TOOLS = [
    ("studio_index", "What's in the studio: takes (with analysis summaries), renders (with comparison summaries), bodies, the recent log and the recording state.", {}),
    ("devices", "The microphone inputs ffmpeg can see.", {}),
    ("record_start", "Start recording Vera into takes/<take>/dry.wav (48 kHz mono). A chime plays. Only when she asks.",
     {"take": {"type": "string"}, "device": {"type": "string", "description": "Input name substring or index; default MacBook Pro Microphone."},
      "notes": {"type": "string"}}, ["take"]),
    ("record_stop", "Stop the current recording and return its length.", {}),
    ("record_status", "Whether a recording is running, and for how long.", {}),
    ("adopt", "Bring an existing WAV in as a take, converted to 48 kHz mono.", {"take": {"type": "string"}, "wav": {"type": "string"}, "notes": {"type": "string"}}, ["take", "wav"]),
    ("segments", "The stretches of sound between pauses in a take (start, end, voiced %, F0), for labelling sections.", {"take": {"type": "string"}}, ["take"]),
    ("set_sections", "Label parts of a take: sections {name: [t0, t1] or null to remove}. Known names drive the analysis: "
     "room, ah, ee, oo, uh, ah_low, ah_mid, ah_high, sweep, glide, ss, sh, mm, hvd, mama, passage, passage_v, v_lines, chat, man_low, man_weird.",
     {"take": {"type": "string"}, "sections": {"type": "object"}, "replace": {"type": "boolean"}}, ["take", "sections"]),
    ("analyse", "Measure a take. Returns the summary and the overview plot: spectrogram with F0, F0 track, vowel space against Hillenbrand 1995 averages, and LTAS per section. "
     "The full report is takes/<take>/analysis/report.json.", {"take": {"type": "string"}, "image": IMG}, ["take"]),
    ("body_surface", "The Body surface from ovillo-body describe: every knob, with its unit, range, default, physical meaning and what it does to sound.", {}),
    ("presets", "The renderer's preset bodies, copied into the studio's bodies/ (existing studio bodies win).", {}),
    ("bodies", "The studio's Body specs, name to JSON.", {}),
    ("save_body", "Write a Body spec to bodies/<name>.json, with a note for the log.", {"name": {"type": "string"}, "spec": {"type": ["object", "string"]}, "note": {"type": "string"}}, ["name", "spec"]),
    ("render", "Render a take through a Body (a studio body name, a path, or a renderer preset), with optional knob overrides set {id: value}. "
     "Writes renders/<take>/<as or body>/ and compares dry with wet. Returns the receipt, the comparison summary and the comparison plot.",
     {"take": {"type": "string"}, "body": {"type": "string"}, "set": {"type": "object"}, "as": {"type": "string", "description": "Name for this render (default: the body name)."}, "image": IMG},
     ["take", "body"]),
    ("formant_shift", "NOT Ovillo, NOT physics: Praat's 'Change gender' PSOLA resynthesis, every formant scaled by ratio (1.08-1.18 is the useful range), her pitch kept. "
     "A labelled comparison for the physical Bodies; say so whenever you present it.", {"take": {"type": "string"}, "ratio": {"type": "number"}, "as": {"type": "string"}, "image": IMG}, ["take", "ratio"]),
    ("compare", "Compare a take with its render through a Body again: metric deltas, the transfer |H| peaks and notches, coherence, and the plot.",
     {"take": {"type": "string"}, "body": {"type": "string"}, "image": IMG}, ["take", "body"]),
    ("set_prompts", "Give Vera a session of things to say. Her studio page shows one prompt at a time: she presses space to record each one as its own take, <session>-<id>, with -2, -3 for retakes. "
     "Each prompt is {id, say, kind (an analysis key, so the take measures itself: ah, ee, passage, passage_v, man_low ...), hint (how to say it), target_hz [lo, hi] (drawn on her live pitch meter)}.",
     {"session": {"type": "string"}, "prompts": {"type": "array", "items": {"type": "object"}}, "intro": {"type": "string"}}, ["session", "prompts"]),
    ("prompt_results", "Every prompt of a session with its takes' quick numbers (F0 stats, tract length, formants, alpha, level), newest first.", {"session": {"type": "string"}}),
    ("script_session", "Turn the film's script lines into a dialogue session on Vera's page: ids like 'R1-01..R1-18,G-01..G-06'; order 'character' groups each part (V., then RADIO, then GATEKEEPER) so she can hold a voice, or 'scene'. Each prompt shows its cue line.",
     {"session": {"type": "string"}, "ids": {"type": "string"}, "order": {"type": "string", "enum": ["character", "scene"]}}, ["session", "ids"]),
    ("select_take", "Circle a take for the current line (default its newest), or a named take.", {"take": {"type": "string"}}),
    ("export_selects", "Copy every circled take, trimmed, to <dest>/<line>.wav for the film (default ~/dev/verabot/film/vo/vera).", {"dest": {"type": "string"}}),
    ("quick", "Fast numbers for one short take (pitch, then formants or tract length depending on its kind).", {"take": {"type": "string"}}, ["take"]),
    ("play", "Play a take (or its render through a body) on the Mac's speakers.", {"take": {"type": "string"}, "body": {"type": "string"}}, ["take"]),
]


def schema(props, req=()):
    return {"type": "object", "properties": props, "required": list(req), "additionalProperties": False}


def tools_list():
    return [{"name": t[0], "description": t[1], "inputSchema": schema(t[2], t[3] if len(t) > 3 else ())} for t in TOOLS]


def image(path):
    return {"type": "image", "mimeType": "image/png", "data": base64.b64encode(open(path, "rb").read()).decode()}


def call(name, a):
    img = a.pop("image", True)
    if name == "studio_index": return studio.index(), None
    if name == "devices": return record.devices(), None
    if name == "record_start": return record.start(a["take"], a.get("device"), notes=a.get("notes", "")), None
    if name == "record_stop": return record.stop(), None
    if name == "record_status": return record.status() or "not recording", None
    if name == "adopt": return record.adopt(a["take"], a["wav"], a.get("notes", "")), None
    if name == "segments": return ops.segments(a["take"]), None
    if name == "set_sections": return ops.set_sections(a["take"], a["sections"], a.get("replace", False)), None
    if name == "analyse":
        r = ops.analyse(a["take"]); return r, (r["png"] if img else None)
    if name == "body_surface": return ops.describe(), None
    if name == "presets": return ops.presets(), None
    if name == "bodies": return ops.bodies(), None
    if name == "save_body": return ops.save_body(a["name"], a["spec"], a.get("note", "")), None
    if name == "render":
        r = ops.render(a["take"], a["body"], a.get("set"), a.get("as"))
        png = studio.STUDIO / "renders" / a["take"] / (a.get("as") or a["body"]) / "compare.png"
        return r, (str(png) if img and png.exists() else None)
    if name == "formant_shift":
        r = ops.formant_shift(a["take"], a["ratio"], a.get("as"))
        png = studio.STUDIO / "renders" / a["take"] / (a.get("as") or f"praat-x{a['ratio']:g}") / "compare.png"
        return r, (str(png) if img and png.exists() else None)
    if name == "compare":
        r = ops.compare(a["take"], a["body"]); return r, (r["png"] if img else None)
    if name == "set_prompts": return ops.set_prompts(a["session"], a["prompts"], a.get("intro", "")), None
    if name == "prompt_results": return ops.prompt_results(a.get("session")), None
    if name == "script_session": return ops.script_session(a["session"], a["ids"], a.get("order", "scene")), None
    if name == "select_take": return ops.select_take(a.get("take")), None
    if name == "export_selects": return ops.export_selects(a.get("dest")), None
    if name == "quick": return ops.quick(a["take"]), None
    if name == "play": return ops.play(a["take"], a.get("body")), None
    raise KeyError(f'unknown tool "{name}"')


def reply(id_, result=None, error=None):
    msg = {"jsonrpc": "2.0", "id": id_}
    msg.update({"error": error} if error else {"result": result})
    sys.stdout.write(json.dumps(msg) + "\n"); sys.stdout.flush()


def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            m = json.loads(line)
        except json.JSONDecodeError:
            reply(None, error={"code": -32700, "message": "parse error"}); continue
        method, id_, p = m.get("method"), m.get("id"), m.get("params") or {}
        if id_ is None:
            continue  # a notification
        if method == "initialize":
            v = p.get("protocolVersion"); v = v if v in VERSIONS else VERSIONS[0]
            reply(id_, {"protocolVersion": v, "capabilities": {"tools": {}},
                        "serverInfo": {"name": "ovillo-voice", "title": "Ovillo Voice Studio", "version": "0.1.0"}, "instructions": INSTRUCTIONS})
        elif method == "ping":
            reply(id_, {})
        elif method == "tools/list":
            reply(id_, {"tools": tools_list()})
        elif method == "tools/call":
            try:
                res, png = call(p.get("name", ""), dict(p.get("arguments") or {}))
                content = [{"type": "text", "text": res if isinstance(res, str) else json.dumps(res, indent=1)}]
                if png:
                    content.append(image(png))
                reply(id_, {"content": content, "isError": False})
            except Exception as e:
                reply(id_, {"content": [{"type": "text", "text": f"{e.__class__.__name__}: {e}\n{traceback.format_exc(limit=3)}"}], "isError": True})
        else:
            reply(id_, error={"code": -32601, "message": f"method not found: {method}"})


if __name__ == "__main__":
    main()
