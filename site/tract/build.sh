#!/usr/bin/env bash
# Build the tract World for the page: ovillo-tract-world's C-ABI wasm cdylib
# (no wasm-bindgen; the same ABI as ovillo-tract-wasm, so either engine runs
# the page), copied next to index.html as tract.wasm. Serve site/ with any static server:
#   cd site && uv run python -m http.server 8137   ->  http://localhost:8137/tract/
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here/../../ovillo"
cargo build --release --target wasm32-unknown-unknown -p ovillo-tract-world
cp target/wasm32-unknown-unknown/release/ovillo_tract_world.wasm "$here/tract.wasm"
ls -l "$here/tract.wasm"
