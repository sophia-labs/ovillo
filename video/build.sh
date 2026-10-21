#!/usr/bin/env bash
# Rebuild the Ovillo release video from the Worlds' renders.
#
#   ./build.sh                    renders (only those missing), prep, soundtrack, frames, MP4s, checks
#   ./build.sh --renders          re-render every World first (mus render, trombone, tract, piano trace)
#   ./build.sh --variant 4x5      one variant (4x5 | 16x9 | all; default all)
#   ./build.sh --dpr 2 --jobs 3   capture supersampling and parallel pages (defaults 2 and 3)
#   ./build.sh --no-capture       stop after the soundtrack (data and audio only)
#
# Every sound is an Ariadne World's own WAV (see README.md); the browser is
# muted headless Chromium and never makes a sound.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/.." && pwd)"
cd "$here"

renders=0 variant=all dpr=2 jobs=3 capture=1
while [ $# -gt 0 ]; do
  case "$1" in
    --renders) renders=1 ;;
    --variant) variant=$2; shift ;;
    --dpr) dpr=$2; shift ;;
    --jobs) jobs=$2; shift ;;
    --no-capture) capture=0 ;;
    *) echo "unknown option $1" >&2; exit 2 ;;
  esac
  shift
done
step() { printf '\n== %s\n' "$*"; }

step "node modules (three, playwright-core: pinned in package.json)"
[ -d node_modules/three ] && [ -d node_modules/playwright-core ] || pnpm install --frozen-lockfile

step "tools: mus CLI, trombone example, tract World wasm"
(cd "$repo/mus-rs" && cargo build --release -p mus-cli)
(cd "$repo/ovillo" && cargo build --release -p ovillo-dsp --example trombone_nd_solo)
(cd "$repo/ovillo" && cargo build --release --target wasm32-unknown-unknown -p ovillo-tract-world)
MUS="$repo/mus-rs/target/release/mus"
TBN="$repo/ovillo/target/release/examples/trombone_nd_solo"
WASM="$repo/ovillo/target/wasm32-unknown-unknown/release/ovillo_tract_world.wasm"

step "renders"
mkdir -p data/guitar data/trombone data/tract
if [ $renders = 1 ] || [ ! -f data/guitar/wayfaring.energy.json ]; then
  echo "guitar: mus render examples/tab/wayfaring-stranger-doc.mus --energy (about 3 minutes)"
  "$MUS" render "$repo/examples/tab/wayfaring-stranger-doc.mus" -o data/guitar/wayfaring.wav --json \
    --energy data/guitar/wayfaring.energy.json > data/guitar/wayfaring.receipt.json
fi
if [ $renders = 1 ] || [ ! -f data/trombone/trombone-nd-solo-trace.json ]; then
  echo "trombone: trombone_nd_solo solo trace"
  "$TBN" data/trombone solo trace
  (cd "$repo" && uv run --no-project --with numpy --with scipy --with soundfile python -m mus_analysis.trombone_world_analyse \
    video/data/trombone/trombone-nd-solo.wav video/data/trombone/trombone-nd-solo-notes.tsv \
    --plan profiles/ariadne-trombone-v1.plan.json --json video/data/trombone/trombone-nd-solo-analysis.json > /dev/null)
fi
if [ $renders = 1 ] || [ ! -f data/tract/tract-frames.json ]; then
  echo "tract: tools/tract-render.mjs (audio and geometry from one run)"
  node tools/tract-render.mjs "$WASM" data/tract --check-observers
fi
python3 tools/piano-status.py
if [ $renders = 1 ] || [ ! -f data/piano/piano-demo-energy.json ]; then
  echo "piano: the demo render from the piano branch, its trace re-rendered with the World's books"
  tools/piano-ledger.sh
fi

step "prep (build/*.json)"
uv run --no-project --with numpy python tools/prep.py manifest.json build/

step "soundtrack (build/audio/soundtrack.wav)"
uv run --no-project python tools/audio.py manifest.json build/

[ $capture = 1 ] || exit 0
variants=$([ "$variant" = all ] && echo "4x5 16x9" || echo "$variant")
for v in $variants; do
  step "frames $v (muted headless Chromium, dpr $dpr, $jobs pages)"
  rm -rf "frames/$v"
  node tools/capture.mjs --variant "$v" --dpr "$dpr" --jobs "$jobs"
  step "assemble $v"
  tools/assemble.sh "$v"
  step "verify $v"
  tools/verify.sh "$v"
done
