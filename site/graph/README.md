# The Ovillo graph

Every sound comes out of an Ariadne World. This directory builds the small graph behind ovillo.sophia-labs.com: `ovl.ttl` (the vocabulary), `build_graph.py` (catalogue + renders -> triples + files), `test_graph.py` (real data, no mocks), `deploy-sketch/` (the box, not applied). The binding contract is in hub `plans/2026-10-20-ovillo-sophia-app.md`, "The contract between the builders" (with a dated extension note).

## Build and test

```sh
uv run --with numpy --with soundfile --with rdflib python site/graph/build_graph.py   # -> site/graph/out/ovillo.nt, out/site/
uv run --with numpy --with soundfile --with rdflib python site/graph/test_graph.py    # 8 checks, ~2.5 min (one is a second full build compared byte for byte)
```

`--pieces` names the catalogue (`site/scores/pieces.py` writes it; default is the 2026-10-20 session's `scores/pieces.json`), `--out` the output directory. `out/` is not committed (about 12 MB of mp3s and 168 KB of triples); the same inputs give the same bytes.
`out/ovillo.nt` carries the vocabulary too (parsed from `ovl.ttl`), so one `gardend load` is the whole graph.

Bench pieces have no receipt and no energy trace: the graph has no `ovl:receipt`, `ovl:ledger`, `ovl:energyTrace` or `ovl:renderDigest` for them (omitted, not faked); their `ovl:BenchRun` holds the engine log's numbers.

## The local public gardend (port 7792)

Binary: gardend 0.1.4 (storage format 1), the v0.1.4 release archive's, copied to `/private/tmp/claude-501/-Users-vera-dev-sophia/ae46aca1-49fc-48f7-b253-025c47555890/scratchpad/ovillo-gardend/bin/` with its `ui/`; no services. The profile is `/private/tmp/claude-501/-Users-vera-dev-sophia/ae46aca1-49fc-48f7-b253-025c47555890/scratchpad/ovillo-gardend/profile` (its own key, its own graph; Vera's Sophia.app and its profile on 7674 are untouched).

```sh
D=/private/tmp/claude-501/-Users-vera-dev-sophia/ae46aca1-49fc-48f7-b253-025c47555890/scratchpad/ovillo-gardend
# start (foreground in a terminal, or detached as here)
cd $D && (nohup bin/gardend serve --profile $D/profile --port 7792 > serve.log 2>&1 & echo $! > gardend.pid)
curl -s localhost:7792/health
# stop (gardend flushes, then exits)
kill $(cat $D/gardend.pid)
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

