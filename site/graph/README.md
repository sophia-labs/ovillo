# The Ovillo graph

Every sound comes out of an Ariadne World. This directory builds the small graph behind ovillo.sophia-labs.com: `ovl.ttl` (the vocabulary), `build_graph.py` (catalogue + renders -> triples + files), `test_graph.py` (real data, no mocks), `deploy-sketch/` (a server deployment, sketched and not applied). The app that reads the graph is Shrubbery's `apps/ovillo` ([sophia-labs/shrubbery](https://github.com/sophia-labs/shrubbery)); the contract between the two is the vocabulary in `ovl.ttl` and the queries below (it was agreed in Sophia's planning notes, which are not in this repository).

## Build and test

```sh
uv run --with numpy --with soundfile --with rdflib python site/graph/build_graph.py --pieces <pieces.json>   # -> out/ovillo.nt, out/site/, out/pieces.json
uv run --with numpy --with soundfile --with rdflib python site/graph/test_graph.py    # 13 checks, 20 s to 2.5 min (one is a second full build compared byte for byte)
```

Needs `ffmpeg`/`ffprobe` on the path. `--pieces` names the catalogue (default `$OVILLO_RENDERS/scores/pieces.json`, the 17 pieces `site/scores/pieces.py` writes; see "The renders" below). With `OVILLO_RENDERS` set, the test also checks that the catalogue's first 17 entries are that file's.

**`pieces-r2.json` is a record, not a portable input.** It is the catalogue release r2 (2026-10-21) was built from: the 17 pieces of 2026-10-20 unchanged, plus the trombone. Its paths are absolute paths on the machine that built it (a session scratch directory under `/private/tmp/...` and a worktree under `/Users/...`), so on any other machine `--pieces site/graph/pieces-r2.json` fails until you render the pieces yourself and write your own catalogue in the same shape. The builder copies the catalogue it used to `out/pieces.json`, and the tests read that one. `--out` is the output directory. `out/` is not committed (about 12 MB of mp3s and 175 KB of triples; `out/pieces.json` names local paths and is not part of the site); the same inputs give the same bytes.

### Pieces a Rust example plays (2026-10-21: the trombone v1)

The trombone v1 World (one N-D body: a tube-network bore, the slide its coordinate, the lips the runtime's flow valve; ovillo `TROMBONE-WORLD.md`) is not played by `mus render` but by `ovillo/crates/ovillo-dsp/examples/trombone_nd_solo.rs`, which writes the WAV, the note list (TSV), a receipt (`ovillo.trombone-nd-render.v1`, its `ledger` the World's own receipt) and, since ovillo `3dcfc98`, an energy trace (air and lips every 10 ms; the lungs apart). A catalogue entry with `world` says so; the mapping is in `ovl.ttl`'s 2026-10-21 note. A second such piece is one more entry:

```json
{"slug": "…", "title": "…", "group": "repo", "tags": ["…"], "engine": "…", "note": "… only numbers from the receipt or analysis …",
 "wav": "<renders>/x.wav", "score": "<renders>/x-notes.tsv", "scoreRepoPath": null, "generator": "<repo path of the example>",
 "receipt": "<renders>/x-receipt.json", "energy": "<renders>/x.energy.json", "analysis": "<renders>/x-analysis.json",
 "sourceRevision": "<ovillo commit the example was built from>", "created": "YYYY-MM-DD",
 "world": {"kind": "…", "title": "…", "plan": "<the World plan JSON, ariadne-world-plan-v3>"}}
```

The receipt must carry `planDigest`, `seconds`, `notes`, `renderWallSeconds`, `peak` and a `ledger` with mus_ariadne's WorldReceipt fields (`sampleFrames`, `initialEnergy`, `finalEnergy`, `sourceWork`, `controlWork`, `internalLoss`, `radiation`, `cumulativeClosureResidual`, `globalClosureResidual`, `maximumFrameResidual`): `trombone_nd_solo.rs` writes exactly that. `energy` and `analysis` are optional (omitted, not faked). The test admits such a piece by a positive rule (`lawful_example_world`): its plan is a World plan naming no synthesized source, its books close at roundoff, its WAV has the World's frame count and the receipt's peak, its trace a frame per 10 ms; every number in its note must be one of its receipt's or analysis's. A new instrument kind (`world.kind`) is admitted by that rule alone; only the trombone also gets a structural test (`test_the_trombone_is_one_nd_body`).
`out/ovillo.nt` carries the vocabulary too (parsed from `ovl.ttl`), so one `gardend load` is the whole of `user:rdf`. `out/ux-config.nt` is the Shrubbery app's layouts (one `ux:LayoutDocument` per route, `ux:layoutJson`; the file `ux-config.nt` here is written by `pnpm --filter @shrubbery/ovillo fixture:ux -- --stdout`), loaded with `--into urn:mnemosyne:local:graph:ovillo:ux:config`; without it the app uses its built-in layouts and says so.

Bench pieces have no receipt and no energy trace: the graph has no `ovl:receipt`, `ovl:ledger`, `ovl:energyTrace` or `ovl:renderDigest` for them (omitted, not faked); their `ovl:BenchRun` holds the engine log's numbers. An example-World piece (the trombone) has a receipt and a ledger but no `ovl:renderDigest`, `ovl:engineVersion`, `ovl:listenerDigest` or touch counts: its receipt does not state them.

## The renders (inputs the builder reads)

Nothing the builder reads as audio is committed. To make a catalogue, render every piece into one directory `$OVILLO_RENDERS` and let `site/scores/pieces.py` write `$OVILLO_RENDERS/scores/pieces.json` (it names each file below and reads each bench log's numbers):

| piece | how it was rendered | files |
|---|---|---|
| a MUS piece (`site/scores/<name>.mus`; the `.py` beside it writes the `.mus`) | `mus render site/scores/<name>.mus -o $OVILLO_RENDERS/sounds/<num>-<name>.wav --json --energy $OVILLO_RENDERS/sounds/<num>-<name>.energy.json > $OVILLO_RENDERS/sounds/<num>-<name>.json`, after copying the score to `$OVILLO_RENDERS/scores/` | `sounds/NN-*.wav`, `.json` (the receipt), `.energy.json` |
| the two repository pieces (tour, Wayfaring Stranger) | the same, from `examples/tab/` of a clone at the commit you render (`OVILLO_CLONE`, default `$OVILLO_RENDERS/fresh/ovillo`) | `sounds/00-tour.*`, `sounds/01-wayfaring.*` |
| a bench piece | `mkdir -p $OVILLO_RENDERS/bench/<dir>`, then from `ovillo/`: `cargo run --release -p ovillo-dsp --example render -- <seconds> <pattern or score.json> $OVILLO_RENDERS/bench/<dir> <pickup> [../site/patches/<patch>.json] > $OVILLO_RENDERS/bench/<dir>/log` | `bench/<dir>/<name>-wet.wav`, `log` |
| the trombone v1 | from `ovillo/`: `cargo run --release -p ovillo-dsp --example trombone_nd_solo -- <out-dir> solo` (`TROMBONE-WORLD.md`); the optional analysis is `python -m mus_analysis.trombone_world_analyse` (as `video/build.sh` runs it); its catalogue entry is written by hand (shape above) | `trombone-nd-solo.wav`, `-notes.tsv`, `-receipt.json`, `.energy.json`, `-analysis.json` |

The numbers `pieces.py` uses (`NN`, the bench `<dir>`, pattern, seconds, pickup, patch) are in its source; the bench logs of 2026-10-20 began with, for example, `pattern Ambient  pickup Neck  45 s at 48000 Hz`. Re-rendered at another commit, a piece's bytes and digests can differ from release r2's; the builder and the test check a catalogue against its own renders, not against r2.

## The local public gardend (port 7792)

How the graph was served locally on 2026-10-20: gardend 0.1.4 (storage format 1) from the [v0.1.4 release](https://github.com/sophia-labs/gardend/releases/tag/v0.1.4), unpacked with its `ui/` into a directory `$D` of your choosing; no services; a profile of its own (its own key, its own graph), separate from any other gardend on the machine.

```sh
D=<a directory holding bin/gardend and profile/>
# start (foreground in a terminal, or detached as here)
cd $D && (nohup bin/gardend serve --profile $D/profile --port 7792 > serve.log 2>&1 & echo $! > gardend.pid)
curl -s localhost:7792/health
# stop (gardend flushes, then exits)
kill $(cat $D/gardend.pid)
# the app's layouts into :ux:config (out/ux-config.nt, from the Shrubbery app's `fixture:ux -- --stdout`)
$D/bin/gardend load --graph ovillo --profile $D/profile --into urn:mnemosyne:local:graph:ovillo:ux:config site/graph/out/ux-config.nt
# reload the graph after a rebuild (gardend must be stopped; the load is atomic and adds only what is new, so to replace, delete the graph's store first or make a new profile)
$D/bin/gardend load --graph ovillo --profile $D/profile site/graph/out/ovillo.nt
```

`profile/public.json` = `{"graphs":["ovillo"],"siteDir":"<repo>/site/graph/out/site"}` (public mode needs an `index.html` in `siteDir`; the builder writes a stand-in that the Shrubbery app's build replaces). The graph was created once with `create_graph` over `/mcp` using the owner key in `profile/selfhost-key.json` (never printed).

## Reading as a visitor (no credential)

- `POST http://127.0.0.1:7792/mcp` JSON-RPC `tools/list` shows 38 read tools (no `rdf_dump`, no writes); `_meta["sophia.publicGraphs"] = ["ovillo"]`.
- `tools/call` `sparql_query` with `{"graphId": "ovillo", "query": ...}` (note: `graphId`, camelCase). The answer's `structuredContent`/`content[0].text` is `{"resultType","variables","rows":[{var: term}], "warnings"}`; each term is an N-Triples string: IRIs as `<https://...>`, strings as `"text"` (JSON-escaped inside), numbers as `"0.0000012"^^<http://www.w3.org/2001/XMLSchema#double>` (the store writes doubles out in full decimal; parse as a float).
- Wrap patterns in `GRAPH ?g { ... }` (named-graph store: an unwrapped pattern returns nothing).
- **1,000 rows per call** (`sparql.maxRows`); a longer answer is cut with `warnings: ["result truncated at 1000 rows ..."]` and there is no cursor token: page with `ORDER BY` + `LIMIT 1000 OFFSET n`. The whole graph is about 1,064 quads, so only a full dump needs a second page; every query below is far under it.
- URLs in the data are absolute (`https://ovillo.sophia-labs.com/media/<slug>.mp3`); a client on another origin (this local gardend) fetches the same pathname from its own origin.
- A piece with two Worlds (`gigante-y-juguete`) has two ledgers: queries that join the ledger return two rows for it; group by piece in the client.

### 1. The playlists: every piece in order, with audio, peaks, energy trace (absent for bench pieces) and duration

```sparql
PREFIX ovl: <http://sophia.ai/ovillo#>
PREFIX dcterms: <http://purl.org/dc/terms/>
SELECT ?playlist ?plTitle ?position ?piece ?slug ?title ?audio ?peaks ?energy ?duration WHERE { GRAPH ?g {
  ?pl a ovl:Playlist ; dcterms:title ?plTitle ; ovl:item ?piece .
  ?piece ovl:slug ?slug ; dcterms:title ?title ; ovl:position ?position ; ovl:render ?r .
  ?r ovl:audio ?audio ; ovl:peaks ?peaks ; ovl:durationSeconds ?duration .
  OPTIONAL { ?r ovl:energyTrace ?energy } BIND(?pl AS ?playlist) } } ORDER BY ?playlist ?position LIMIT 1000 OFFSET 0
```

### 2. Every ledger (the World's own numbers, state units)

```sparql
PREFIX ovl: <http://sophia.ai/ovillo#>
PREFIX dcterms: <http://purl.org/dc/terms/>
SELECT ?slug ?world ?plucks ?sourceWork ?controlWork ?internalLoss ?radiation ?closureResidual ?maxFrameResidual WHERE { GRAPH ?g {
  ?piece ovl:slug ?slug ; ovl:render ?r . ?r ovl:ledger ?l .
  ?l ovl:world ?world ; ovl:plucks ?plucks ; ovl:sourceWork ?sourceWork ; ovl:controlWork ?controlWork ;
     ovl:internalLoss ?internalLoss ; ovl:radiation ?radiation ; ovl:closureResidual ?closureResidual ; ovl:maxFrameResidual ?maxFrameResidual } } ORDER BY ?slug ?world
```

### 3. The bench runs (engine log numbers; no ledger)

```sparql
PREFIX ovl: <http://sophia.ai/ovillo#>
PREFIX dcterms: <http://purl.org/dc/terms/>
SELECT ?slug ?pattern ?pickup ?closureResidual ?safetyLimitedFrames WHERE { GRAPH ?g {
  ?piece ovl:slug ?slug ; ovl:render/ovl:bench ?b . ?b ovl:safetyLimitedFrames ?safetyLimitedFrames .
  OPTIONAL { ?b ovl:pattern ?pattern ; ovl:pickup ?pickup } OPTIONAL { ?b ovl:closureResidual ?closureResidual } } } ORDER BY ?slug
```

### 4. Everything said directly about one piece

```sparql
PREFIX ovl: <http://sophia.ai/ovillo#>
PREFIX dcterms: <http://purl.org/dc/terms/>
SELECT ?p ?o WHERE { GRAPH ?g { ?piece ovl:slug "hilo-de-luz" ; ?p ?o } }
```

### 5. A score's source

```sparql
PREFIX ovl: <http://sophia.ai/ovillo#>
PREFIX dcterms: <http://purl.org/dc/terms/>
SELECT ?format ?text ?repoPath WHERE { GRAPH ?g { ?piece ovl:slug "bajo-la-casa" ; ovl:score ?s . ?s ovl:format ?format ; ovl:text ?text ; ovl:repoPath ?repoPath } }
```

