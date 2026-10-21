"""Real-data checks on the built graph (no mocks). Build first, then:

    uv run --with numpy --with soundfile --with rdflib python site/graph/test_graph.py -v
"""
import hashlib, json, subprocess, sys, tempfile, unittest
from pathlib import Path

from rdflib import Graph, Namespace, RDF, URIRef

HERE = Path(__file__).resolve().parent
OUT = HERE / "out"
OVL = Namespace("http://sophia.ai/ovillo#")
SCRATCH = "/private/tmp/claude-501/-Users-vera-dev-sophia/ae46aca1-49fc-48f7-b253-025c47555890/scratchpad"
PIECES = json.loads(Path(f"{SCRATCH}/scores/pieces.json").read_text())
BANNED = ("trombone", "lip reed", "buzz", "synth", "weave", "aiguá", "aigua", "bird", "subtractive", "oscillator", "sample", "pluck voice", "telar", "pájaros")
G = Graph().parse(OUT / "ovillo.nt", format="nt")


def path_of(u):
    return OUT / "site" / str(u).split("ovillo.sophia-labs.com/", 1)[1]


def one(s, p):
    v = list(G.objects(s, p))
    assert len(v) == 1, (s, p, v)
    return v[0]


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
        n = 0
        for src in PIECES:
            if not src.get("receipt"):
                continue
            n += 1
            rec = json.loads(Path(src["receipt"]).read_text())
            served = json.loads((OUT / "site" / "receipts" / f"{src['slug']}.json").read_text())
            r = URIRef(f"https://ovillo.sophia-labs.com/id/render/{src['slug']}")
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
            self.assertEqual(served["ariadneWorlds"], rec["ariadneWorlds"])
            self.assertEqual(str(one(r, OVL.renderDigest)), rec["renderDigest"])
        self.assertEqual(n, 7)

    def test_bench_pieces_say_nothing_they_do_not_know(self):
        for src in PIECES:
            if src.get("receipt"):
                continue
            r = URIRef(f"https://ovillo.sophia-labs.com/id/render/{src['slug']}")
            for pr in (OVL.ledger, OVL.receipt, OVL.energyTrace, OVL.renderDigest, OVL.engineVersion):
                self.assertEqual(list(G.objects(r, pr)), [], (src["slug"], pr))
            self.assertFalse((OUT / "site" / "media" / f"{src['slug']}.energy.json").exists())
            b = one(r, OVL.bench)
            self.assertEqual(int(one(b, OVL.safetyLimitedFrames)), src["bench"]["safetyLimitedFrames"])
        b = one(URIRef("https://ovillo.sophia-labs.com/id/render/toward-the-bore"), OVL.bench)
        self.assertEqual(int(one(b, OVL.safetyLimitedFrames)), 22863)
        self.assertEqual(json.loads(one(b, OVL.patch))["material_organization"], 1.0)

    def test_only_ariadne_worlds(self):
        for p in G.subjects(RDF.type, OVL.Piece):
            text = " ".join(str(o).lower() for pr in (OVL.engine, OVL.note) for o in G.objects(p, pr))
            for w in BANNED:
                self.assertNotIn(w, text, (p, w))
            for ins in G.objects(p, OVL.instrument):
                self.assertIn(str(one(ins, OVL.kind)), ("steel-string", "bench"))
        for src in PIECES:
            if src.get("receipt"):
                rec = json.loads(Path(src["receipt"]).read_text())
                self.assertEqual(rec["voiceCount"], len(rec["ariadneWorlds"]), src["slug"])  # every voice is a World
                self.assertTrue(all(w["anatomy"] == "steel-string-v0" for w in rec["ariadneWorlds"]))
        for f in (OUT / "site" / "media").iterdir():
            for w in BANNED:
                self.assertNotIn(w, f.name.lower())
        for s in G.objects(None, OVL.text):
            self.assertNotIn("weave", str(s).lower())

    def test_no_piece_with_a_synthesized_source(self):
        """The trombone (additive buzz into a passive mouthpiece, bell band filtered outside the World) is out, and so is
        anything like it: a piece is a steel-string World (a receipt whose Worlds are all steel-string-v0, played by plucks,
        hammer-ons, pull-offs and slides) or a bench run of the simulated guitarist into the World."""
        self.assertNotIn("trombone", {str(s) for s in G.objects(None, OVL.slug)})
        self.assertFalse((OUT / "site" / "media" / "trombone.mp3").exists())
        for p in G.subjects(RDF.type, OVL.Piece):
            kinds = {str(one(i, OVL.kind)) for i in G.objects(p, OVL.instrument)}
            self.assertTrue(kinds and kinds <= {"steel-string", "bench"}, (p, kinds))
            r = one(p, OVL.render)
            if list(G.objects(r, OVL.bench)):
                self.assertEqual(kinds, {"bench"})
                self.assertEqual(len(list(G.objects(one(r, OVL.bench), OVL.pattern))), 1)  # a simulated guitarist, nothing else
            else:
                self.assertEqual(kinds, {"steel-string"})
        self.assertEqual(len(PIECES), 17)

    def test_energy_traces_are_the_worlds(self):
        for src in PIECES:
            if not src.get("energy"):
                continue
            raw = json.loads(Path(src["energy"]).read_text())
            e = json.loads((OUT / "site" / "media" / f"{src['slug']}.energy.json").read_text())
            self.assertEqual(e["channels"], ["strings", "box", "room"])
            self.assertEqual(e["hopSeconds"], 0.01)
            self.assertEqual(len(e["touches"]), sum(len(w["touches"]) for w in raw["worlds"]))
            self.assertEqual(len(e["frames"]), max(len(w["frames"]) for w in raw["worlds"]))
            self.assertEqual(("worlds" in e), len(raw["worlds"]) > 1)
            w0 = raw["worlds"][0]; ch = w0["channels"]; i = [k for k, c in enumerate(ch) if c == "room"][0]
            if len(raw["worlds"]) == 1:
                self.assertAlmostEqual(e["frames"][100][2], w0["frames"][100][i], delta=abs(w0["frames"][100][i]) * 1e-3 + 1e-30)

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
        self.assertEqual(len(with_rad), 7)

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
            subprocess.check_call([sys.executable, str(HERE / "build_graph.py"), "--out", d], stdout=subprocess.DEVNULL)
            self.assertEqual(tree(d), tree(OUT))


if __name__ == "__main__":
    unittest.main()
