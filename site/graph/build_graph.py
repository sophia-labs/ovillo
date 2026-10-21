#!/usr/bin/env python3
"""Build the Ovillo graph: out/ovillo.nt (the user:rdf triples), out/ux-config.nt (the app's layouts, for :ux:config)
and out/site/ (the files the triples point at).

    uv run --with numpy --with soundfile --with rdflib python site/graph/build_graph.py \
        [--pieces <pieces.json>] [--out site/graph/out]

pieces.json is what site/scores/pieces.py writes: each piece's wav, score, receipt and energy-trace paths.
Same inputs, same bytes: triples are sorted, JSON is key-sorted and compact, the mp3 is encoded bit-exact.
Every sound comes out of an Ariadne World; there is no other engine to describe.

Three kinds of piece (2026-10-21):
  * a MUS piece: `mus render`'s receipt (mus.audio.render-receipt.v1), one ledger per ariadneWorlds[] entry;
  * a bench piece (`bench`): the engine's log, no receipt;
  * an example-World piece (`world`): a Rust example played a World plan (ariadne-world-plan-v3) and wrote a receipt
    whose `ledger` is the World's own receipt (mus_ariadne::WorldReceipt), a note list (TSV) as its score, an energy
    trace in mus-engine's shape, and optionally an analysis (a measurement of the render). One entry:
      {"slug", "title", "group", "tags", "engine", "note", "wav", "score": <notes.tsv>, "scoreRepoPath": null,
       "generator": <repo path of the example>, "receipt", "energy", "analysis", "sourceRevision", "created",
       "world": {"kind", "title", "plan": <path of the plan JSON>}}
The catalogue used is copied to out/pieces.json (not part of the site: it names local paths).
"""
import argparse, hashlib, json, re, shutil, subprocess
from pathlib import Path

import numpy as np
import soundfile as sf
from rdflib import Graph, Literal, Namespace, URIRef, RDF, XSD

HERE = Path(__file__).resolve().parent
SCRATCH = "/private/tmp/claude-501/-Users-vera-dev-sophia/ae46aca1-49fc-48f7-b253-025c47555890/scratchpad"
BASE = "https://ovillo.sophia-labs.com"
OVL = Namespace("http://sophia.ai/ovillo#")
DCT = Namespace("http://purl.org/dc/terms/")
SOURCE_REVISION = "e48d8f87d1297d20ab0964214a1fa18cd9f9a30e"  # the ovillo commit the renders were made from
CREATED = "2026-10-20"
TARGET_RMS, PEAK_CEIL = -18.0, -1.0
PATCH_FILES = {"agua-mansa": "glass", "toward-the-bore": "brass", "hungry-world": "hungry", "halfway": "midway"}  # else the default patch
GROUP_TITLES = {"today": "Today (2026-10-20)", "repo": "From the repository", "bench": "The bench"}
RECEIPT_KEEP = ("schema", "renderDigest", "sourceDigest", "engineVersion", "peakDbfs", "rmsDbfs", "durationSeconds",
                "wallSeconds", "eventCount", "voiceCount", "badTokens", "warnings", "ariadneWorlds")
SCORE_FORMATS = {".mus": "mus", ".json": "ovillo-score-json", ".tsv": "ovillo-notes-tsv"}
# An example-World receipt's `ledger`: mus_ariadne::WorldReceipt as the example writes it (camelCase).
WORLD_LEDGER = (("sourceWork", "sourceWork"), ("controlWork", "controlWork"), ("internalLoss", "internalLoss"),
                ("radiation", "radiation"), ("cumulativeClosureResidual", "closureResidual"),
                ("maximumFrameResidual", "maxFrameResidual"), ("initialEnergy", "initialEnergy"), ("finalEnergy", "finalEnergy"))
STRING_UNITS = ("stored energy in the World's own state units (raw energy, not decibels); strings, box and room each "
                "summed over their bodies; 4 significant digits")


def db(x):
    return 20 * np.log10(np.maximum(x, 1e-12))


def sha(b):
    return "sha256:" + hashlib.sha256(b).hexdigest()


def jdump(o):
    return json.dumps(o, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def sig4(a):
    return np.array([float(f"{v:.4g}") for v in a.ravel()]).reshape(a.shape)


def iri(kind, name):
    return URIRef(f"{BASE}/id/{kind}/{name}")


def url(path):
    return URIRef(f"{BASE}/{path}")


def note_name(n):
    return n.lower().replace("#", "s")


def body_kind(channel):
    """The body kind a raw engine channel belongs to: string/6 -> strings, box/0 -> box, room -> room, air -> air."""
    head = channel.split("/")[0]
    return "strings" if head == "string" else head


def compact_energy(trace):
    """Per World: stored energy of each body kind every hop (strings, box, room for the steel-string; air and lips for
    the trombone), in the order the trace names them; top level is the sum over Worlds."""
    kinds = []
    for w in trace["worlds"]:
        for c in w["channels"]:
            if body_kind(c) not in kinds:
                kinds.append(body_kind(c))
    strings = kinds == ["strings", "box", "room"]
    worlds, touches = [], []
    for w in trace["worlds"]:
        ch = w["channels"]
        fr = np.array(w["frames"], dtype=float)
        cols = [fr[:, [i for i, c in enumerate(ch) if body_kind(c) == k]].sum(axis=1) for k in kinds]
        worlds.append((w["instrument"], sig4(np.stack(cols, axis=1))))
        for t in w.get("touches", []):
            keep = ("string", "kind", "fret", "db", "hand") if strings else tuple(k for k in ("kind", "midi", "partial", "position") if k in t)
            touches.append({"t": round(t["seconds"], 4), **{k: t.get(k) for k in keep}, "world": w["instrument"]})
    n = max(len(f) for _, f in worlds)
    total = np.zeros((n, len(kinds)))
    for _, f in worlds:
        total[: len(f)] += f
    touches.sort(key=lambda t: (t["t"], t["world"], t["string"] if t.get("string") is not None else -1))
    if strings:
        units = STRING_UNITS
    else:
        parts = trace["worlds"][0].get("parts", {})
        reservoirs = trace["worlds"][0].get("reservoirs", [])
        units = ("stored energy in the World's own state units (raw energy, not decibels), by part: "
                 + "; ".join(f"{k} = {parts[k]}" for k in kinds if k in parts)
                 + ("; not in the frames: " + "; ".join(f"{k} = {parts[k]}" for k in reservoirs if k in parts) if reservoirs else "")
                 + "; the touches are the note list's starts; 4 significant digits")
    out = {"schema": "ovillo.energy.v1", "hopSeconds": trace["worlds"][0]["hopSeconds"], "startSeconds": 0,
           "channels": kinds, "units": units, "frames": sig4(total).tolist(), "touches": touches}
    if len(worlds) > 1:
        out["worlds"] = [{"world": name, "frames": f.tolist()} for name, f in worlds]
    return out


def plan_world(plan):
    """A World plan's name for its ledger and its anatomy: a one-body plan is named by its body (and its anatomy is the
    body's anatomy_name); a plan of several bodies by its world_name."""
    assert plan.get("schema_version") == "ariadne-world-plan-v3", plan.get("schema_version")
    bodies = plan["bodies"]
    if len(bodies) == 1:
        return bodies[0]["id"], bodies[0]["constitution"]["body"].get("anatomy_name") or plan["world_name"]
    return plan["world_name"], plan["world_name"]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pieces", default=f"{SCRATCH}/scores/pieces.json")
    ap.add_argument("--out", default=str(HERE / "out"))
    a = ap.parse_args()
    catalogue = Path(a.pieces).read_bytes()  # read before out/ is cleared: the catalogue may live there
    pieces = json.loads(catalogue)
    out = Path(a.out)
    if out.exists():
        shutil.rmtree(out)
    site = out / "site"
    for d in ("media", "scores", "receipts"):
        (site / d).mkdir(parents=True)
    (out / "pieces.json").write_bytes(catalogue)
    g = Graph()
    for p, f in (("ovl", OVL), ("dcterms", DCT)):
        g.bind(p, f)
    g.parse(HERE / "ovl.ttl", format="turtle")  # the vocabulary travels with the data
    L = lambda v, dt=None: Literal(v, datatype=dt) if dt else Literal(v)
    instruments, groups = {}, {}

    def add_instrument(kind, title, anatomy=None, tuning=None, slug=None):
        slug = slug or kind
        s = iri("instrument", slug)
        if slug not in instruments:
            instruments[slug] = s
            g.add((s, RDF.type, OVL.Instrument)); g.add((s, DCT.title, L(title))); g.add((s, OVL.kind, L(kind)))
            if anatomy: g.add((s, OVL.anatomy, L(anatomy)))
            if tuning: g.add((s, OVL.tuning, L(tuning)))
        return s

    for p in pieces:
        slug = p["slug"]
        ps, rs = iri("piece", slug), iri("render", slug)
        g.add((ps, RDF.type, OVL.Piece)); g.add((ps, DCT.title, L(p["title"]))); g.add((ps, OVL.slug, L(slug)))
        g.add((ps, OVL.group, L(p["group"]))); g.add((ps, OVL.note, L(p["note"]))); g.add((ps, OVL.engine, L(p["engine"])))
        for t in p["tags"]:
            g.add((ps, OVL.tag, L(t)))
        groups.setdefault(p["group"], []).append(ps)
        g.add((ps, OVL.position, L(len(groups[p["group"]]), XSD.integer)))
        g.add((ps, OVL.render, rs)); g.add((rs, RDF.type, OVL.Render)); g.add((rs, DCT.created, L(p.get("created", CREATED), XSD.date)))
        g.add((rs, OVL.sourceRevision, L(p.get("sourceRevision", SOURCE_REVISION))))

        # --- audio: the World's output, then the page's gain, mp3 and peaks
        x, sr = sf.read(p["wav"], always_2d=True)
        mono = x.mean(axis=1)
        rms, peak = float(db(np.sqrt(np.mean(mono**2)))), float(db(np.max(np.abs(x))))
        gain = round(min(TARGET_RMS - rms, PEAK_CEIL - peak), 2)
        mp3 = site / "media" / f"{slug}.mp3"
        subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-i", p["wav"], "-map_metadata", "-1", "-fflags", "+bitexact",
                        "-flags:a", "+bitexact", "-af", f"highpass=f=18,volume={gain:.2f}dB", "-codec:a", "libmp3lame",
                        "-q:a", "2", str(mp3)], check=True)
        spp = int(round(sr / 100))  # 10 ms, the energy trace's hop
        m = mono * 10 ** (gain / 20)
        m = np.pad(m, (0, (-len(m)) % spp)).reshape(-1, spp)
        pk = np.stack([m.min(axis=1), m.max(axis=1)], axis=1).round(3).ravel().tolist()
        (site / "media" / f"{slug}.peaks.json").write_text(jdump({"sr": int(sr), "samplesPerPeak": spp, "peaks": pk}))
        g.add((rs, OVL.audio, url(f"media/{slug}.mp3"))); g.add((rs, OVL.audioDigest, L(sha(mp3.read_bytes()))))
        g.add((rs, OVL.peaks, url(f"media/{slug}.peaks.json")))
        g.add((rs, OVL.durationSeconds, L(round(len(mono) / sr, 3), XSD.double)))
        g.add((rs, OVL.worldPeakDbfs, L(round(peak, 1), XSD.double))); g.add((rs, OVL.worldRmsDbfs, L(round(rms, 1), XSD.double)))
        g.add((rs, OVL.pageGainDb, L(gain, XSD.double)))

        # --- score
        if p.get("score"):
            sp = Path(p["score"]); raw = sp.read_bytes(); ext = sp.suffix
            (site / "scores" / f"{slug}{ext}").write_bytes(raw)
            ss = iri("score", slug)
            g.add((ps, OVL.score, ss)); g.add((ss, RDF.type, OVL.Score))
            g.add((ss, OVL['format'], L(SCORE_FORMATS[ext])))
            g.add((ss, OVL.text, L(raw.decode("utf-8")))); g.add((ss, OVL.sourceDigest, L(sha(raw))))
            if "scoreRepoPath" in p:  # stated; null = the score is not in the repository (an example wrote it)
                rel = p["scoreRepoPath"]
            else:
                rel = p["score"].split("/fresh/ovillo/")[-1] if "/fresh/ovillo/" in p["score"] else f"site/scores/{sp.name}"
            if rel:
                g.add((ss, OVL.repoPath, L(rel)))
            gen = HERE.parent / "scores" / (sp.stem + ".py")
            if p.get("generator"):
                g.add((ss, OVL.generator, L(p["generator"])))
            elif gen.exists():
                g.add((ss, OVL.generator, L(f"site/scores/{gen.name}")))

        # --- an example-World piece: the example's receipt, whose ledger is the World's own (mus_ariadne::WorldReceipt)
        if p.get("world"):
            w = p["world"]
            r = json.loads(Path(p["receipt"]).read_text())
            name, anatomy = plan_world(json.loads(Path(w["plan"]).read_text()))
            (site / "receipts" / f"{slug}.json").write_text(jdump(r))  # whole: it carries no logs
            g.add((rs, OVL.receipt, url(f"receipts/{slug}.json")))
            # it states no renderDigest and no engineVersion: omitted, not made up
            g.add((rs, OVL.wallSeconds, L(float(r["renderWallSeconds"]), XSD.double)))
            g.add((rs, OVL.eventCount, L(int(r["notes"]), XSD.integer)))
            ls = iri("ledger", f"{slug}-{name}")
            g.add((rs, OVL.ledger, ls)); g.add((ls, RDF.type, OVL.Ledger))
            g.add((ls, OVL.world, L(name))); g.add((ls, OVL.anatomy, L(anatomy))); g.add((ls, OVL.planDigest, L(r["planDigest"])))
            g.add((ls, OVL.simulatedSeconds, L(float(r["seconds"]), XSD.double)))
            g.add((ls, OVL.sampleFrames, L(int(r["ledger"]["sampleFrames"]), XSD.integer)))
            for k, pr in WORLD_LEDGER:  # no plucks, hammer-ons, pull-offs or slides: this World is not touched that way
                g.add((ls, OVL[pr], L(float(r["ledger"][k]), XSD.double)))
            g.add((ps, OVL.instrument, add_instrument(w["kind"], w["title"], anatomy, None, anatomy)))
            if p.get("analysis"):
                (site / "receipts" / f"{slug}.analysis.json").write_text(jdump(json.loads(Path(p["analysis"]).read_text())))
                g.add((rs, OVL.analysis, url(f"receipts/{slug}.analysis.json")))

        # --- receipt, ledgers, energy (MUS pieces)
        elif p.get("receipt"):
            r = json.loads(Path(p["receipt"]).read_text())
            (site / "receipts" / f"{slug}.json").write_text(jdump({k: r[k] for k in RECEIPT_KEEP if k in r}))
            g.add((rs, OVL.receipt, url(f"receipts/{slug}.json")))
            g.add((rs, OVL.renderDigest, L(r["renderDigest"]))); g.add((rs, OVL.engineVersion, L(r["engineVersion"])))
            g.add((rs, OVL.wallSeconds, L(float(r["wallSeconds"]), XSD.double))); g.add((rs, OVL.eventCount, L(r["eventCount"], XSD.integer)))
            for w in r["ariadneWorlds"]:
                ls = iri("ledger", f"{slug}-{w['instrument']}")
                g.add((rs, OVL.ledger, ls)); g.add((ls, RDF.type, OVL.Ledger))
                g.add((ls, OVL.world, L(w["instrument"]))); g.add((ls, OVL.anatomy, L(w["anatomy"])))
                g.add((ls, OVL.planDigest, L(w["planDigest"]))); g.add((ls, OVL.listenerDigest, L(w["listenerDigest"])))
                g.add((ls, OVL.simulatedSeconds, L(float(w["simulatedSeconds"]), XSD.double)))
                for k, pr in (("plucks", OVL.plucks), ("hammerOns", OVL.hammerOns), ("pullOffs", OVL.pullOffs), ("slides", OVL.slides)):
                    g.add((ls, pr, L(int(w[k]), XSD.integer)))
                for k, pr in (("sourceWork", OVL.sourceWork), ("controlWork", OVL.controlWork), ("internalLoss", OVL.internalLoss),
                              ("radiation", OVL.radiation), ("cumulativeClosureResidual", OVL.closureResidual),
                              ("maximumFrameResidual", OVL.maxFrameResidual)):
                    g.add((ls, pr, L(float(w[k]), XSD.double)))
                tuning = " ".join(s["open"] for s in reversed(w["strings"]))
                ins = add_instrument("steel-string", f"Steel-string, {tuning}", w["anatomy"], tuning,
                                     "steel-string-" + "-".join(note_name(n) for n in tuning.split()))
                g.add((ps, OVL.instrument, ins))
        if p.get("energy") and Path(p["energy"]).exists():
            tr = json.loads(Path(p["energy"]).read_text())
            (site / "media" / f"{slug}.energy.json").write_text(jdump(compact_energy(tr)))
            g.add((rs, OVL.energyTrace, url(f"media/{slug}.energy.json")))

        # --- bench run: the engine's log; no receipt, no ledger, no trace, by omission
        if p.get("bench"):
            b = p["bench"]; bs = iri("bench", slug)
            g.add((rs, OVL.bench, bs)); g.add((bs, RDF.type, OVL.BenchRun))
            if b.get("safetyLimitedFrames") is not None:
                g.add((bs, OVL.safetyLimitedFrames, L(int(b["safetyLimitedFrames"]), XSD.integer)))
            if b.get("closureResidual") is not None:
                g.add((bs, OVL.closureResidual, L(float(b["closureResidual"]), XSD.double)))
            logdir = Path(p['wav']).parent
            if True:
                m = re.search(r"pattern (\S+)\s+pickup (\S+)", (logdir / "log").read_text())
                g.add((bs, OVL.pattern, L(m.group(1)))); g.add((bs, OVL.pickup, L(m.group(2))))
                pf = Path(SCRATCH) / "patches" / f"{PATCH_FILES.get(slug, 'none')}.json"
                patch = json.loads(pf.read_text()) if pf.exists() else {"preset": "ovillo-default"}
                g.add((bs, OVL.patch, L(jdump(patch))))
                add = add_instrument("bench", "The Ovillo bench: a simulated guitarist into an Ariadne World")
            g.add((ps, OVL.instrument, add))

    for grp, items in groups.items():
        pl = iri("playlist", grp)
        g.add((pl, RDF.type, OVL.Playlist)); g.add((pl, DCT.title, L(GROUP_TITLES[grp])))
        for it in items:
            g.add((pl, OVL.item, it))

    lines = sorted(set(g.serialize(format="nt").splitlines()) - {""})
    (out / "ovillo.nt").write_text("\n".join(lines) + "\n", encoding="utf-8")

    # The app's layouts (one LayoutDocument per route as ux:layoutJson), for the graph's :ux:config named graph:
    # ux-config.nt is written by the Shrubbery app (`pnpm --filter @shrubbery/ovillo fixture:ux -- --stdout`), checked
    # here to parse and to name only the app's surfaces, and loaded with `gardend load --into <...:ux:config>`.
    ux = Graph().parse(HERE / "ux-config.nt", format="nt")
    surfaces = sorted({str(s) for s in ux.subjects()})
    assert surfaces and all(s.startswith(f"{BASE}/id/surface/") for s in surfaces), surfaces
    shutil.copyfile(HERE / "ux-config.nt", out / "ux-config.nt")
    (site / "llms.txt").write_text(
        "# Ovillo\nPieces played by Ariadne Worlds (physical models of instruments and rooms), with each World's energy ledger.\n"
        "Read the graph `ovillo` with sparql_query at /mcp (read-only). Vocabulary: http://sophia.ai/ovillo#\n"
        "Files: /media/<slug>.mp3, /media/<slug>.peaks.json, /media/<slug>.energy.json, /scores/<slug>.mus (.json for a bench "
        "score, .tsv for the note list an example played), /receipts/<slug>.json (and /receipts/<slug>.analysis.json where the "
        "render was measured)\n")
    # public mode wants an index.html; this is a stand-in the Shrubbery app's build replaces
    rows = "".join(f'<li><a href="media/{p["slug"]}.mp3">{p["title"]}</a> ({p["group"]})</li>' for p in pieces)
    (site / "index.html").write_text(
        '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Ovillo</title>'
        "<h1>Ovillo</h1><p>Pieces played by Ariadne Worlds. The app replaces this page; the graph is at /mcp (sparql_query).</p>"
        f"<ul>{rows}</ul>\n")
    (site / "index.md").write_text("# Ovillo\n\nPieces played by Ariadne Worlds. Read the graph `ovillo` with `sparql_query` at /mcp.\n\n"
                                   + "".join(f"- {p['title']} ({p['group']}): media/{p['slug']}.mp3\n" for p in pieces))
    print(f"{len(pieces)} pieces, {len(lines)} triples -> {out}; {len(ux)} :ux:config triples ({len(surfaces)} layouts)")


if __name__ == "__main__":
    main()
