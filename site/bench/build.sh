#!/usr/bin/env bash
# Build the bench's WebAssembly (the Ovillo rig: guitar, guitarist, Ariadne World)
# and put it beside bench.js. Needs the wasm32-unknown-unknown target.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
cargo build --manifest-path "$ROOT/ovillo/Cargo.toml" --profile wasm --target wasm32-unknown-unknown -p ovillo-web
cp "$ROOT/ovillo/target/wasm32-unknown-unknown/wasm/ovillo_web.wasm" "$HERE/ovillo_web.wasm"
ls -l "$HERE/ovillo_web.wasm"
