#!/usr/bin/env bash
# Build the browser engine (mus check + Ariadne-World render, a plain-C-ABI module with no imports) and copy it to
# OUT (default: beside this script). Needs the wasm32-unknown-unknown target. Never `cargo ... --workspace` here.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
OUT="${1:-$HERE/mus_wasm.wasm}"
cargo build --manifest-path "$ROOT/Cargo.toml" --profile wasm --target wasm32-unknown-unknown -p mus-wasm
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
cp "$TARGET/wasm32-unknown-unknown/wasm/mus_wasm.wasm" "$OUT"
ls -l "$OUT"
shasum -a 256 "$OUT"
