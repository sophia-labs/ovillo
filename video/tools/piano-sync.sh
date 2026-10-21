#!/usr/bin/env bash
# Copy the Ariadne piano World's demo render into data/piano/, with its provenance.
#
#   tools/piano-sync.sh [piano-worktree]        (default ~/dev/ovillo-piano-20261021)
#
# Takes ovillo/renders/piano-demo.wav, -receipt.json, -notes.tsv, -energy.json and the score
# examples/piano/prelude-c-major.mus, refuses unless the piano engine is committed on that
# worktree's branch, and writes data/piano/SOURCE.txt (branch, commit, sha256 of every file).
# If PIANO_LEDGER_ENERGY names a re-render of the same score whose WAV is bit-identical to the
# demo's and whose trace carries ledgerFrames, that trace is used for the picture instead.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
src="${1:-$HOME/dev/ovillo-piano-20261021}"
r="$src/ovillo/renders"
git -C "$src" log --oneline -1 -- mus-rs/crates/mus-engine/src/ariadne_piano.rs | grep -q . \
  || { echo "piano-sync: the piano engine is not committed in $src yet" >&2; exit 1; }
for f in piano-demo.wav piano-demo-receipt.json piano-demo-notes.tsv piano-demo-energy.json; do
  [ -f "$r/$f" ] || { echo "piano-sync: missing $r/$f" >&2; exit 1; }
done
mkdir -p "$here/data/piano"
cp "$r/piano-demo.wav" "$r/piano-demo-receipt.json" "$r/piano-demo-notes.tsv" "$r/piano-demo-energy.json" "$here/data/piano/"
cp "$src/examples/piano/prelude-c-major.mus" "$here/data/piano/"
if [ -n "${PIANO_LEDGER_ENERGY:-}" ]; then
  cp "$PIANO_LEDGER_ENERGY" "$here/data/piano/piano-demo-energy.json"
fi
{
  echo "piano demo render, copied $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "from: $src ($(git -C "$src" rev-parse --abbrev-ref HEAD) @ $(git -C "$src" rev-parse --short HEAD))"
  echo "engine commit: $(git -C "$src" log --oneline -1 -- mus-rs/crates/mus-engine/src/ariadne_piano.rs)"
  [ -n "${PIANO_LEDGER_ENERGY:-}" ] && echo "energy trace: $PIANO_LEDGER_ENERGY (same score, WAV bit-identical, with ledgerFrames)"
  (cd "$here/data/piano" && shasum -a 256 piano-demo.wav piano-demo-receipt.json piano-demo-notes.tsv piano-demo-energy.json prelude-c-major.mus)
} > "$here/data/piano/SOURCE.txt"
cat "$here/data/piano/SOURCE.txt"
