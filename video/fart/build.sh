#!/usr/bin/env bash
# The 1812-for-guts film and its 30-second short, from the score's render.
#
#   ./build.sh                 prep, frames (both variants), MP4s, checks
#   ./build.sh --variant 16x9  one variant (16x9 the film, 9x16 the short)
#   ./build.sh --render        ovillo-fart render the score first (cached parts are reused)
#   ./build.sh --jobs 3 --dpr 1 --gpu
#
# Needs the release video's node modules (../node_modules: three, playwright-core; `pnpm install`
# in ../), uv, ffmpeg 8 at /usr/local/bin, and the score rendered (ovillo-fart render).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
cd "$here"
variant=all jobs=3 dpr=1 gpu= render=0
while [ $# -gt 0 ]; do
  case "$1" in
    --variant) variant=$2; shift ;;
    --jobs) jobs=$2; shift ;;
    --dpr) dpr=$2; shift ;;
    --gpu) gpu=--gpu ;;
    --render) render=1 ;;
    *) echo "unknown option $1" >&2; exit 2 ;;
  esac
  shift
done
step() { printf '\n== %s\n' "$*"; }
score="$repo/ovillo/scores/fart/1812"
[ -d ../node_modules/playwright-core ] || (cd .. && pnpm install --frozen-lockfile)
if [ $render = 1 ] || [ ! -f "$score/out/mix.wav" ]; then
  step "render (ovillo-fart)"
  (cd "$repo/ovillo" && cargo build --release -p ovillo-fart && ./target/release/ovillo-fart render scores/fart/1812 --jobs 8 | tail -2)
fi
step "prep (build/film.json, build/frames.f32)"
uv run --no-project --with numpy python tools/prep.py "$score" build/
variants=$([ "$variant" = all ] && echo "16x9 9x16" || echo "$variant")
for v in $variants; do
  step "frames $v (muted headless Chromium, dpr $dpr, $jobs pages)"
  rm -rf "frames/$v"
  node tools/capture.mjs --variant "$v" --dpr "$dpr" --jobs "$jobs" $gpu
  step "assemble $v"
  tools/assemble.sh "$v"
  step "verify $v"
  tools/verify.sh "$v"
done
