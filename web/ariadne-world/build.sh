#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
wasm-pack build "$ROOT/mus-rs/crates/mus-ariadne-webgpu-web" \
  --target web \
  --release \
  --out-dir "$ROOT/web/ariadne-world/pkg" \
  --out-name ariadne_world_web
