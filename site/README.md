# site/ — ovillo.sophia-labs.com

A static site: the live bench (`bench/`, the Ovillo rig as WebAssembly in an AudioWorklet), the listening room (`sounds/`, built), and `llms.txt`.

Build:

```sh
bash site/bench/build.sh                     # bench/ovillo_web.wasm
uv run --with numpy --with scipy --with soundfile \
  python site/tools/build_sounds.py <pieces.json> site/sounds   # mp3, traces, scores, receipts, manifest
```

`pieces.json` names each piece's render (`wav`), score, receipt and energy trace; the scores written for the site are in `site/scores/`, rendered with `mus render <score> -o <wav> --json --energy <trace>` (MUS pieces) or `cargo run --release -p ovillo-dsp --example render -- <seconds> <score.json|pattern> <out> <pickup> <patch.json>` (bench pieces). `sounds/` and the wasm are build outputs and are not committed.

Serve locally: `python3 -m http.server -d site 8765`. Deployed by `platform-next/deploy/two-box/scripts/deploy-ovillo.sh` (static files under `/srv/sophia/static/ovillo/`, Caddy).
