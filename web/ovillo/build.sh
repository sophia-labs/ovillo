#!/usr/bin/env bash
# Build the portable Ovillo module (ovillo/crates/ovillo-portable: every World,
# one C-ABI wasm, no bindgen) into web/ovillo/ovillo.wasm, then write the
# catalog it carries to web/ovillo/ovillo-catalog.json. Needs the
# wasm32-unknown-unknown target and node.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
# Panic messages carry source paths: name them from the repository and cargo's home, not this machine's.
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:---remap-path-prefix=$root=ovillo --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=cargo -C target-feature=+simd128}"
cargo build --manifest-path "$root/ovillo/Cargo.toml" --profile wasm --target wasm32-unknown-unknown -p ovillo-portable
cp "$root/ovillo/target/wasm32-unknown-unknown/wasm/ovillo_portable.wasm" "$here/ovillo.wasm"
ls -l "$here/ovillo.wasm"
node "$here/tools/catalog.mjs" "$here/ovillo.wasm" "$here/ovillo-catalog.json"
