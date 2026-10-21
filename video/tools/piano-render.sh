#!/usr/bin/env bash
# The piano segment's data: render examples/piano/prelude-c-major.mus on the Ariadne piano World
# into data/piano/: the WAV, its render receipt, its energy trace (with ledgerFrames, the World's
# books every 10 ms, where the source tree has them), the notes table, the score, and SOURCE.txt
# (which tree, and the sha256 of every file).
#
#   tools/piano-render.sh          the piano of this checkout
#   tools/piano-render.sh <rev>    the piano at commit <rev>: `git archive <rev>` into .cache/, built there
#
# The released MP4's piano is `tools/piano-render.sh ffb2219` (README.md, "Provenance").
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
repo="$(cd "$here/.." && pwd)"
rev="${1:-}"
mkdir -p "$here/.cache"
if [ -n "$rev" ]; then
  sha=$(git -C "$repo" rev-parse --verify "$rev^{commit}")
  src="$here/.cache/piano-src-${sha:0:12}"
  if [ ! -d "$src" ]; then
    rm -rf "$src.partial" && mkdir -p "$src.partial"
    git -C "$repo" archive "$sha" | tar -x -C "$src.partial"
    mv "$src.partial" "$src"
  fi
  target="$here/.cache/piano-target-${sha:0:12}"
  label="commit $sha (git archive)"
else
  src="$repo"
  target="${CARGO_TARGET_DIR:-$repo/mus-rs/target}"
  head=$(git -C "$repo" rev-parse HEAD 2>/dev/null || echo "not a git checkout")
  dirty=$(git -C "$repo" status --porcelain -- mus-rs profiles examples/piano 2>/dev/null | grep -q . && echo ", with local changes" || true)
  label="this checkout at $head$dirty"
fi
(cd "$src/mus-rs" && cargo build --release --locked -p mus-cli --target-dir "$target")
(cd "$src/mus-rs" && cargo build --release --locked -p mus-engine --example piano_world --target-dir "$target")

out="$here/data/piano"
work="$here/.cache/piano-run"
rm -rf "$work" && mkdir -p "$out" "$work"
echo "piano: mus render examples/piano/prelude-c-major.mus from $label"
(cd "$src" && "$target/release/mus" render examples/piano/prelude-c-major.mus -o "$out/piano-demo.wav" --json \
  --energy "$out/piano-demo-energy.json" > "$out/piano-demo-receipt.json")
echo "piano: the notes table (mus-engine example piano_world)"
(cd "$src" && "$target/release/examples/piano_world" examples/piano/prelude-c-major.mus "$work/piano" > /dev/null)
cp "$work/piano.notes.tsv" "$out/piano-demo-notes.tsv"
cp "$src/examples/piano/prelude-c-major.mus" "$out/"
rm -rf "$work"
{
  echo "piano demo render, $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "source: $label"
  (cd "$out" && shasum -a 256 piano-demo.wav piano-demo-receipt.json piano-demo-notes.tsv piano-demo-energy.json prelude-c-major.mus)
} > "$out/SOURCE.txt"
cat "$out/SOURCE.txt"
