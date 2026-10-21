#!/usr/bin/env bash
# Build the browser engine (mus check + Ariadne-World render, a plain-C-ABI module with no imports) and copy it to
# OUT (default: target/mus_wasm.wasm). Needs the wasm32-unknown-unknown target. Never `cargo ... --workspace` here.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
OUT="${1:-$TARGET/mus_wasm.wasm}"
# SIMD (every current browser has it): about a sixth less CPU per render, the same samples and receipt as without.
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:--C target-feature=+simd128}" \
  cargo build --manifest-path "$ROOT/Cargo.toml" --profile wasm --target wasm32-unknown-unknown -p mus-wasm
cp "$TARGET/wasm32-unknown-unknown/wasm/mus_wasm.wasm" "$OUT"
ls -l "$OUT"
shasum -a 256 "$OUT"
