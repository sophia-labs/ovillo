#!/usr/bin/env bash
# The piano's picture trace: re-render the demo score on the piano branch plus one read-only
# patch (ledgerFrames, the World's books per hop), check the WAV is bit-identical to the demo's,
# and keep that trace.
#
#   tools/piano-ledger.sh [ledger-worktree] [piano-worktree]
#     defaults ~/dev/ovillo-piano-ledger-20261021 (branch site/video-piano-ledger-20261021,
#     a20d520 + ffb2219) and ~/dev/ovillo-piano-20261021 (ariadne/piano-world-v1-20261021)
#
# Then tools/piano-sync.sh copies the demo render with this trace into data/piano/.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
ledger="${1:-$HOME/dev/ovillo-piano-ledger-20261021}"
piano="${2:-$HOME/dev/ovillo-piano-20261021}"
out="$here/data/piano-ledger"
mkdir -p "$out" "$here/.cache"
(cd "$ledger/mus-rs" && cargo build --release --locked -p mus-cli --target-dir "$here/.cache/piano-target")
(cd "$ledger" && "$here/.cache/piano-target/release/mus" render examples/piano/prelude-c-major.mus \
  -o "$out/prelude.wav" --json --energy "$out/prelude.energy.json" > "$out/prelude.receipt.json")
a=$(shasum -a 256 "$out/prelude.wav" | cut -d' ' -f1)
b=$(shasum -a 256 "$piano/ovillo/renders/piano-demo.wav" | cut -d' ' -f1)
[ "$a" = "$b" ] || { echo "piano-ledger: the re-render ($a) is not the demo ($b)" >&2; exit 1; }
echo "piano-ledger: WAV bit-identical to the demo ($a); trace with ledgerFrames in $out/prelude.energy.json"
PIANO_LEDGER_ENERGY="$out/prelude.energy.json" "$here/tools/piano-sync.sh" "$piano"
