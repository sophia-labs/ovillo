"""Real-data checks on the built graph (no mocks). Build first, then:

    uv run --with numpy --with soundfile --with rdflib python site/graph/test_graph.py -v

The catalogue is the one the build used (out/pieces.json, the builder's copy of --pieces).
"""
import hashlib, json, math, re, subprocess, sys, tempfile, unittest
from pathlib import Path

import numpy as np
import soundfile as sf
from rdflib import Graph, Namespace, RDF, URIRef

HERE = Path(__file__).resolve().parent
OUT = HERE / "out"
OVL = Namespace("http://sophia.ai/ovillo#")
SCRATCH = "/private/tmp/claude-501/-Users-vera-dev-sophia/ae46aca1-49fc-48f7-b253-025c47555890/scratchpad"
PIECES = json.loads((OUT / "pieces.json").read_text())
ORIGINAL = Path(f"{SCRATCH}/scores/pieces.json")  # the 17 pieces of 2026-10-20, reviewed then
MUS = [p for p in PIECES if p.get("receipt") and not p.get("world")]
EXAMPLES = [p for p in PIECES if p.get("world")]
# Never in a piece's text: the names of what is not an Ariadne World. ("trombone" was here until 2026-10-21: the
# trombone dropped on 10-20 was an additive source; a piece may now say "trombone" only if it passes
# lawful_example_world below, a positive rule, and is a trombone World.)
BANNED = ("lip reed", "buzz", "synth", "weave", "aiguá", "aigua", "bird", "subtractive", "oscillator", "sample", "pluck voice", "telar", "pájaros")
# Never in a World plan: a source that is not physics.
PLAN_BANNED = ("oscillator", "additive", "buzz", "synth", "wavetable", "biquad", "lowpass", "highpass", "bandpass", "filter")
G = Graph().parse(OUT / "ovillo.nt", format="nt")


def path_of(u):
    return OUT / "site" / str(u).split("ovillo.sophia-labs.com/", 1)[1]


def one(s, p):
    v = list(G.objects(s, p))
    assert len(v) == 1, (s, p, v)
    return v[0]


def render_of(slug):
    return URIRef(f"https://ovillo.sophia-labs.com/id/render/{slug}")


def piece_of(slug):
    return URIRef(f"https://ovillo.sophia-labs.com/id/piece/{slug}")


def body_kind(channel):
    head = channel.split("/")[0]
    return "strings" if head == "string" else head


def plan_words(o):
    if isinstance(o, dict):
        for k, v in o.items():
            yield str(k).lower()
            yield from plan_words(v)
    elif isinstance(o, list):
        for v in o:
            yield from plan_words(v)
    elif isinstance(o, str):
        yield o.lower()


def lawful_example_world(src):
    """The positive rule for a piece a Rust example plays (the trombone v1 is the first): a list of reasons it is NOT an
    Ariadne World; empty means lawful. It is one: its plan is a World plan (ariadne-world-plan-v3) naming no synthesized
    source; its receipt's `ledger` is the World's own receipt and closes at roundoff (each frame <= 1e-12, the sum <=
    1e-15 per frame, and initial + work in - loss - radiation = final to the global residual); its audio is that run's
    (as many frames as the World stepped, and the receipt's peak); and its energy trace has a frame per 10 ms of it."""
    why = []
    plan = json.loads(Path(src["world"]["plan"]).read_text())
    if plan.get("schema_version") != "ariadne-world-plan-v3":
        why.append(f"plan schema {plan.get('schema_version')}")
    if not plan.get("bodies") or any("kind" not in b.get("constitution", {}) for b in plan["bodies"]):
        why.append("a body without a constitution")
    words = list(plan_words(plan))
    why += [f"the plan names {w!r}" for w in PLAN_BANNED if any(w in x for x in words)]
    r = json.loads(Path(src["receipt"]).read_text())
    led = r.get("ledger", {})
    need = ("sampleFrames", "initialEnergy", "finalEnergy", "sourceWork", "controlWork", "internalLoss", "radiation",
            "cumulativeClosureResidual", "globalClosureResidual", "maximumFrameResidual")
    if not all(k in led for k in need) or not r.get("planDigest", "").startswith("sha256:"):
        return why + ["the receipt carries no World receipt"]
    n = led["sampleFrames"]
    if not (abs(led["maximumFrameResidual"]) <= 1e-12 and abs(led["cumulativeClosureResidual"]) <= 1e-15 * n):
        why.append(f"the books do not close ({led['maximumFrameResidual']}, {led['cumulativeClosureResidual']})")
    books = led["finalEnergy"] - (led["initialEnergy"] + led["sourceWork"] + led["controlWork"] - led["internalLoss"] - led["radiation"])
    if not math.isclose(books, led["globalClosureResidual"], rel_tol=1e-3, abs_tol=1e-12):
        why.append(f"initial + in - out - final = {books}, the receipt says {led['globalClosureResidual']}")
    x, sr = sf.read(src["wav"], always_2d=True)
    if len(x) != n or not math.isclose(float(np.max(np.abs(x))), r["peak"], rel_tol=1e-6):
        why.append(f"the audio is not this run's ({len(x)} frames, peak {np.max(np.abs(x))}; receipt {n}, {r['peak']})")
    if src.get("energy"):
        tr = json.loads(Path(src["energy"]).read_text())["worlds"][0]
        if len(tr["frames"]) != -(-n // round(tr["hopSeconds"] * sr)):
            why.append(f"{len(tr['frames'])} energy frames for {n} World frames")
    return why


def facts_of(src):
    """Every number a note may cite for an example-World piece: its receipt's and its analysis's."""
    out = []

    def walk(o):
        if isinstance(o, bool):
            return
        if isinstance(o, (int, float)):
            out.append(float(o))
        elif isinstance(o, dict):
            for v in o.values():
                walk(v)
        elif isinstance(o, list):
            for v in o:
                walk(v)
    walk(json.loads(Path(src["receipt"]).read_text()))
    if src.get("analysis"):
        walk(json.loads(Path(src["analysis"]).read_text())["summary"])
    return out


def cites(token, facts):
    """A number as written (12, 5.7, 3.6e-15, 1,629,529) is one of the facts at the precision written."""
    t = token.replace(",", "")
    if "e" in t:
        mant, ex = t.split("e")
        digits = len(mant.split(".")[1]) if "." in mant else 0
        return any(f"{f:.{digits}e}".replace("e-0", "e-").replace("e+0", "e") == t for f in facts if f)
    digits = len(t.split(".")[1]) if "." in t else 0
    return any(f"{f:.{digits}f}" == t for f in facts) or any(f"{abs(f):.{digits}f}" == t.lstrip("-") for f in facts)


class GraphTests(unittest.TestCase):
    def test_every_piece_has_a_render_with_audio(self):
        pieces = list(G.subjects(RDF.type, OVL.Piece))
        self.assertEqual(len(pieces), len(PIECES))
        for p in pieces:
            r = one(p, OVL.render)
            audio = path_of(one(r, OVL.audio))
            self.assertTrue(audio.exists() and audio.stat().st_size > 10_000, audio)
            self.assertTrue(path_of(one(r, OVL.peaks)).exists())
            dur = float(subprocess.check_output(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(audio)]))
            self.assertAlmostEqual(dur, float(one(r, OVL.durationSeconds)), delta=0.15, msg=audio.name)
            self.assertEqual(one(r, OVL.audioDigest).toPython(), "sha256:" + hashlib.sha256(audio.read_bytes()).hexdigest())

    def test_mus_pieces_have_ledgers_equal_to_receipts(self):
        for src in MUS:
            rec = json.loads(Path(src["receipt"]).read_text())
            self.assertEqual(rec["schema"], "mus.audio.render-receipt.v1")
            served = json.loads((OUT / "site" / "receipts" / f"{src['slug']}.json").read_text())
            r = render_of(src["slug"])
            ledgers = {str(one(l, OVL.world)): l for l in G.objects(r, OVL.ledger)}
            self.assertEqual(sorted(ledgers), sorted(w["instrument"] for w in rec["ariadneWorlds"]))
            for w in rec["ariadneWorlds"]:
                l = ledgers[w["instrument"]]
                for k, pr in (("sourceWork", OVL.sourceWork), ("controlWork", OVL.controlWork), ("internalLoss", OVL.internalLoss),
                              ("radiation", OVL.radiation), ("cumulativeClosureResidual", OVL.closureResidual),
                              ("maximumFrameResidual", OVL.maxFrameResidual)):
                    self.assertEqual(float(one(l, pr)), w[k], (src["slug"], k))
                for k, pr in (("plucks", OVL.plucks), ("hammerOns", OVL.hammerOns), ("pullOffs", OVL.pullOffs), ("slides", OVL.slides)):
                    self.assertEqual(int(one(l, pr)), w[k])
                self.assertEqual(str(one(l, OVL.planDigest)), w["planDigest"])
                for pr in (OVL.initialEnergy, OVL.finalEnergy, OVL.sampleFrames):  # mus render does not state them
                    self.assertEqual(list(G.objects(l, pr)), [], (src["slug"], pr))
            self.assertEqual(served["ariadneWorlds"], rec["ariadneWorlds"])
            self.assertEqual(str(one(r, OVL.renderDigest)), rec["renderDigest"])
        self.assertEqual(len(MUS), 7)

    def test_example_world_pieces_are_their_receipts(self):
        """A piece a Rust example played (trombone v1): the ledger is the receipt's World receipt, field by field; the
        served receipt is the receipt whole; the score is the note list, byte for byte; what the receipt does not state
        is absent; the trace is the example's, compacted; the analysis is served as measured."""
        for src in EXAMPLES:
            slug = src["slug"]
            rec = json.loads(Path(src["receipt"]).read_text())
            led = rec["ledger"]
            r = render_of(slug)
            self.assertEqual(json.loads((OUT / "site" / "receipts" / f"{slug}.json").read_text()), rec)
            self.assertEqual(str(one(r, OVL.receipt)), f"https://ovillo.sophia-labs.com/receipts/{slug}.json")
            ledgers = list(G.objects(r, OVL.ledger))
            self.assertEqual(len(ledgers), 1)
            l = ledgers[0]
            plan = json.loads(Path(src["world"]["plan"]).read_text())
            self.assertEqual(str(one(l, OVL.world)), plan["bodies"][0]["id"] if len(plan["bodies"]) == 1 else plan["world_name"])
            self.assertEqual(str(one(l, OVL.planDigest)), rec["planDigest"])
            self.assertEqual(float(one(l, OVL.simulatedSeconds)), rec["seconds"])
            self.assertEqual(int(one(l, OVL.sampleFrames)), led["sampleFrames"])
            for k, pr in (("sourceWork", OVL.sourceWork), ("controlWork", OVL.controlWork), ("internalLoss", OVL.internalLoss),
                          ("radiation", OVL.radiation), ("cumulativeClosureResidual", OVL.closureResidual),
                          ("maximumFrameResidual", OVL.maxFrameResidual), ("initialEnergy", OVL.initialEnergy), ("finalEnergy", OVL.finalEnergy)):
                self.assertEqual(float(one(l, pr)), led[k], (slug, k))
            for pr in (OVL.plucks, OVL.hammerOns, OVL.pullOffs, OVL.slides, OVL.listenerDigest):
                self.assertEqual(list(G.objects(l, pr)), [], (slug, pr))
            for pr in (OVL.renderDigest, OVL.engineVersion):
                self.assertEqual(list(G.objects(r, pr)), [], (slug, pr))
            self.assertEqual(float(one(r, OVL.wallSeconds)), rec["renderWallSeconds"])
            self.assertEqual(int(one(r, OVL.eventCount)), rec["notes"])
            self.assertEqual(str(one(r, OVL.sourceRevision)), src["sourceRevision"])
            # the score is the note list the player was given
            s = one(piece_of(slug), OVL.score)
            tsv = Path(src["score"]).read_bytes()
            self.assertEqual(str(one(s, OVL["format"])), "ovillo-notes-tsv")
            self.assertEqual(str(one(s, OVL.text)).encode(), tsv)
            self.assertEqual((OUT / "site" / "scores" / f"{slug}.tsv").read_bytes(), tsv)
            self.assertEqual(str(one(s, OVL.sourceDigest)), "sha256:" + hashlib.sha256(tsv).hexdigest())
            self.assertEqual(list(G.objects(s, OVL.repoPath)), [])
            self.assertEqual(str(one(s, OVL.generator)), src["generator"])
            self.assertEqual(len(tsv.decode().strip().splitlines()) - 1, rec["notes"])
            # the energy trace: the example's frames, compacted (sum by body kind, 4 significant digits)
            raw = json.loads(Path(src["energy"]).read_text())["worlds"][0]
            e = json.loads((OUT / "site" / "media" / f"{slug}.energy.json").read_text())
            kinds = list(dict.fromkeys(body_kind(c) for c in raw["channels"]))
            self.assertEqual(e["channels"], kinds)
            self.assertEqual(len(e["frames"]), len(raw["frames"]))
            for i in (0, 100, len(raw["frames"]) // 2, len(raw["frames"]) - 1):
                for j, k in enumerate(kinds):
                    want = sum(v for c, v in zip(raw["channels"], raw["frames"][i]) if body_kind(c) == k)
                    self.assertAlmostEqual(e["frames"][i][j], want, delta=abs(want) * 1e-3 + 1e-30)
            self.assertEqual([t["t"] for t in e["touches"]], [round(t["seconds"], 4) for t in raw["touches"]])
            self.assertEqual(len(e["touches"]), rec["notes"])
            self.assertIn("not decibels", e["units"])
            if src.get("analysis"):
                self.assertEqual(json.loads((OUT / "site" / "receipts" / f"{slug}.analysis.json").read_text()),
                                 json.loads(Path(src["analysis"]).read_text()))
                self.assertEqual(str(one(r, OVL.analysis)), f"https://ovillo.sophia-labs.com/receipts/{slug}.analysis.json")

    def test_example_world_notes_cite_only_their_receipts(self):
        """Every number in such a piece's note is its receipt's or its analysis's, at the precision written
        (a note name like C5 is not a number)."""
        for src in EXAMPLES:
            note = str(one(piece_of(src["slug"]), OVL.note))
            facts = facts_of(src)
            tokens = re.findall(r"(?<![A-Za-z♯♭\d.,])-?\d[\d,]*(?:\.\d+)?(?:e-?\d+)?", note)
            self.assertTrue(tokens, src["slug"])
            for tok in tokens:
                self.assertTrue(cites(tok.rstrip(","), facts), (src["slug"], tok))

    def test_the_trombone_is_one_nd_body(self):
        """Trombone v1 as TROMBONE-WORLD.md says: one modal_space body, its bore a tube network, the slide its one
        coordinate (moved under the mass-momentum connection), the lips the runtime's flow valve, the lungs its
        reservoir; no room (the space port couples nothing): heard dry."""
        tbn = [p for p in EXAMPLES if p["world"]["kind"] == "trombone"]
        for src in tbn:
            plan = json.loads(Path(src["world"]["plan"]).read_text())
            (b,) = plan["bodies"]
            c = b["constitution"]
            self.assertEqual(c["kind"], "modal_space")
            space = c["body"]["configuration_space"]
            self.assertEqual(space["map"]["kind"], "tube_network")
            self.assertEqual([x["name"] for x in space["coordinates"]], ["slide_extension_m"])
            self.assertEqual(space["connection"]["kind"], "mass_momentum")
            self.assertIn("valve", c["body"]["laws"])
            self.assertGreater(c["body"]["reservoir"]["capacity"], 0)
            self.assertEqual(set(b["space_port"]["coupling_weights"]), {0.0})
            rec = json.loads(Path(src["receipt"]).read_text())
            self.assertEqual(rec["schema"], "ovillo.trombone-nd-render.v1")
            self.assertEqual(rec["ledger"]["initialEnergy"], c["body"]["reservoir"]["capacity"])  # the lungs, full
            ins = one(piece_of(src["slug"]), OVL.instrument)
            self.assertEqual(str(one(ins, OVL.kind)), "trombone")
            self.assertEqual(str(one(ins, OVL.anatomy)), c["body"]["anatomy_name"])

    def test_bench_pieces_say_nothing_they_do_not_know(self):
        for src in PIECES:
            if src.get("receipt"):
                continue
            r = render_of(src["slug"])
            for pr in (OVL.ledger, OVL.receipt, OVL.energyTrace, OVL.renderDigest, OVL.engineVersion, OVL.analysis):
                self.assertEqual(list(G.objects(r, pr)), [], (src["slug"], pr))
            self.assertFalse((OUT / "site" / "media" / f"{src['slug']}.energy.json").exists())
            b = one(r, OVL.bench)
            self.assertEqual(int(one(b, OVL.safetyLimitedFrames)), src["bench"]["safetyLimitedFrames"])
        b = one(render_of("toward-the-bore"), OVL.bench)
        self.assertEqual(int(one(b, OVL.safetyLimitedFrames)), 22863)
        self.assertEqual(json.loads(one(b, OVL.patch))["material_organization"], 1.0)

    def test_only_ariadne_worlds(self):
        lawful = {p["slug"]: p["world"]["kind"] for p in EXAMPLES if not lawful_example_world(p)}
        for p in G.subjects(RDF.type, OVL.Piece):
            slug = str(one(p, OVL.slug))
            text = " ".join(str(o).lower() for pr in (OVL.engine, OVL.note) for o in G.objects(p, pr))
            for w in BANNED:
                self.assertNotIn(w, text, (p, w))
            if "trombone" in text:
                self.assertEqual(lawful.get(slug), "trombone", (p, "says trombone but is not a lawful trombone World"))
            for ins in G.objects(p, OVL.instrument):
                self.assertIn(str(one(ins, OVL.kind)), {"steel-string", "bench", *lawful.values()})
        for src in MUS:
            rec = json.loads(Path(src["receipt"]).read_text())
            self.assertEqual(rec["voiceCount"], len(rec["ariadneWorlds"]), src["slug"])  # every voice is a World
            self.assertTrue(all(w["anatomy"] == "steel-string-v0" for w in rec["ariadneWorlds"]))
        for f in (OUT / "site" / "media").iterdir():
            for w in BANNED:
                self.assertNotIn(w, f.name.lower())
        for s in G.objects(None, OVL.text):
            self.assertNotIn("weave", str(s).lower())

    def test_no_piece_with_a_synthesized_source(self):
        """The trombone of 2026-10-20 (additive buzz into a passive mouthpiece, bell band filtered outside the World) is
        out, and so is anything like it: a piece is a steel-string World (a receipt whose Worlds are all steel-string-v0,
        played by plucks, hammer-ons, pull-offs and slides), a bench run of the simulated guitarist into the World, or a
        World a Rust example plays that passes lawful_example_world (the trombone v1: one N-D body). The 17 pieces of
        2026-10-20 are the catalogue's first 17, unchanged; everything after them is such an example World."""
        self.assertNotIn("trombone", {str(s) for s in G.objects(None, OVL.slug)})
        self.assertFalse((OUT / "site" / "media" / "trombone.mp3").exists())
        for src in EXAMPLES:
            self.assertEqual(lawful_example_world(src), [], src["slug"])
        examples = {p["slug"]: p["world"]["kind"] for p in EXAMPLES}
        for p in G.subjects(RDF.type, OVL.Piece):
            kinds = {str(one(i, OVL.kind)) for i in G.objects(p, OVL.instrument)}
            r = one(p, OVL.render)
            slug = str(one(p, OVL.slug))
            if list(G.objects(r, OVL.bench)):
                self.assertEqual(kinds, {"bench"})
                self.assertEqual(len(list(G.objects(one(r, OVL.bench), OVL.pattern))), 1)  # a simulated guitarist, nothing else
            elif slug in examples:
                self.assertEqual(kinds, {examples[slug]})
            else:
                self.assertEqual(kinds, {"steel-string"})
        if ORIGINAL.exists():
            self.assertEqual(PIECES[:17], json.loads(ORIGINAL.read_text()))
        self.assertEqual(len(PIECES[:17]), 17)
        self.assertTrue(all(p.get("world") for p in PIECES[17:]), [p["slug"] for p in PIECES[17:]])

    def test_energy_traces_are_the_worlds(self):
        for src in PIECES:
            if not src.get("energy"):
                continue
            raw = json.loads(Path(src["energy"]).read_text())
            e = json.loads((OUT / "site" / "media" / f"{src['slug']}.energy.json").read_text())
            kinds = list(dict.fromkeys(body_kind(c) for w in raw["worlds"] for c in w["channels"]))
            self.assertEqual(e["channels"], kinds)
            if not src.get("world"):
                self.assertEqual(kinds, ["strings", "box", "room"])
            self.assertEqual(e["hopSeconds"], 0.01)
            self.assertEqual(len(e["touches"]), sum(len(w["touches"]) for w in raw["worlds"]))
            self.assertEqual(len(e["frames"]), max(len(w["frames"]) for w in raw["worlds"]))
            self.assertEqual(("worlds" in e), len(raw["worlds"]) > 1)
            if len(raw["worlds"]) == 1:
                w0 = raw["worlds"][0]
                for j, k in enumerate(kinds):
                    want = sum(v for c, v in zip(w0["channels"], w0["frames"][100]) if body_kind(c) == k)
                    self.assertAlmostEqual(e["frames"][100][j], want, delta=abs(want) * 1e-3 + 1e-30)

    def test_playlists_and_positions(self):
        for pl in G.subjects(RDF.type, OVL.Playlist):
            pos = sorted(int(one(i, OVL.position)) for i in G.objects(pl, OVL.item))
            self.assertEqual(pos, list(range(1, len(pos) + 1)))

    def test_sparql_pieces_with_audio_and_ledger(self):
        q = """PREFIX ovl: <http://sophia.ai/ovillo#> PREFIX dcterms: <http://purl.org/dc/terms/>
        SELECT ?title ?audio (SUM(?rad) AS ?radiation) WHERE {
          ?p a ovl:Piece ; dcterms:title ?title ; ovl:render ?r . ?r ovl:audio ?audio .
          OPTIONAL { ?r ovl:ledger ?l . ?l ovl:radiation ?rad } } GROUP BY ?title ?audio"""
        rows = list(G.query(q))
        self.assertEqual(len(rows), len(PIECES))
        with_rad = [r for r in rows if r.radiation is not None and float(r.radiation) > 0]
        self.assertEqual(len(with_rad), len(MUS) + len(EXAMPLES))

    def test_the_apps_layouts_for_ux_config(self):
        """out/ux-config.nt: one ux:LayoutDocument per route of the Ovillo app, its layoutJson a LayoutDocument."""
        import json as _json
        from rdflib import Graph as _G, URIRef as _U
        ux = _G().parse(OUT / "ux-config.nt", format="nt")
        layout = _U("http://mnemosyne.dev/ux#layoutJson")
        docs = {str(s).rsplit("/", 1)[1]: _json.loads(str(o)) for s, o in ux.subject_objects(layout)}
        self.assertEqual(sorted(docs), ["bench", "home", "piece", "score", "tract"])
        for route, doc in docs.items():
            self.assertEqual(doc["schemaVersion"], 1, route)
            self.assertEqual(doc["graphId"], "ovillo", route)
            self.assertIn(doc["rootNodeId"], doc["nodes"], route)

    def test_deterministic(self):
        def tree(root):
            return {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(Path(root).rglob("*")) if p.is_file()}
        with tempfile.TemporaryDirectory() as d:
            subprocess.check_call([sys.executable, str(HERE / "build_graph.py"), "--pieces", str(OUT / "pieces.json"), "--out", d],
                                  stdout=subprocess.DEVNULL)
            self.assertEqual(tree(d), tree(OUT))


if __name__ == "__main__":
    unittest.main()
