#!/usr/bin/env bash
# Build the tract World for the page: a C-ABI wasm cdylib (no wasm-bindgen),
# copied next to index.html as tract.wasm. Serve site/ with any static server:
#   cd site && uv run python -m http.server 8137   ->  http://localhost:8137/tract/
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here/../../ovillo"
cargo build --release --target wasm32-unknown-unknown -p ovillo-tract-wasm
cp target/wasm32-unknown-unknown/release/ovillo_tract_wasm.wasm "$here/tract.wasm"
ls -l "$here/tract.wasm"
