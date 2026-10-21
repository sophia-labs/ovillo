# site/: the sources behind ovillo.sophia-labs.com

[ovillo.sophia-labs.com](https://ovillo.sophia-labs.com) is a Sophia app: Shrubbery's `apps/ovillo` ([sophia-labs/shrubbery](https://github.com/sophia-labs/shrubbery)) reading a small graph served by a public-mode gardend. This directory holds the parts that live in this repository:

| dir | what |
|---|---|
| `graph/` | the graph: the `ovl:` vocabulary, the builder (catalogue + renders -> triples, mp3s, peaks, energy traces, receipts), its real-data test, a deployment sketch (`graph/README.md`) |
| `scores/` | the pieces written for the site on 2026-10-20: each `.mus` and the `.py` that writes it (`tab.py` spells tab tokens from string and fret, `mk.py` checks each bar's length), `agua-mansa.json` (a bench score), and `pieces.py`, which writes the catalogue the graph builder reads |
| `patches/` | the bench patches four bench pieces were played with |
| `bench/` | the bench in a browser: the Ovillo rig (`ovillo/crates/ovillo-web`) as WebAssembly in an AudioWorklet (`bench/README.md`) |
| `tract/` | the vocal tract page: the tract World (`ovillo/crates/ovillo-tract-world`) as WebAssembly, drawn with three.js (vendored, `tract/vendor/three/`) |
| `tools/build_sounds.py` | the listening-room builder of the first, static draft of the site (mp3s, spectral-waterfall and energy SVGs, `sounds/manifest.json`); the draft's pages were replaced by the app, and the graph builder now does this job |

Every piece's sound comes out of an Ariadne World: a steel-string World played by `mus render`, the bench (a simulated guitarist into a World), or the trombone v1 World played by a Rust example. `graph/test_graph.py` refuses a piece whose source is not one.

## Build

```sh
bash site/bench/build.sh      # bench/ovillo_web.wasm (not committed)
bash site/tract/build.sh      # tract/tract.wasm (committed: the build the page serves)
```

Serve locally: `python3 -m http.server -d site 8765`, then `/bench/` and `/tract/`.

## Rendering the pieces

The renders are not committed. `graph/README.md` ("The renders") says how each kind of piece is rendered into one directory `OVILLO_RENDERS`, and `scores/pieces.py` (run with `OVILLO_RENDERS` set) writes the catalogue from it:

```sh
export OVILLO_RENDERS=$PWD/renders                    # any directory
(cd site/scores && python3 hilo_de_luz.py)            # each .py rewrites its .mus beside it
python3 site/scores/pieces.py                         # -> $OVILLO_RENDERS/scores/pieces.json
uv run --with numpy --with soundfile --with rdflib python site/graph/build_graph.py   # reads that catalogue
```

`graph/pieces-r2.json` is the catalogue release r2 was built from; its paths are the building machine's (see `graph/README.md`).
