#!/usr/bin/env bash
# Frames + soundtrack -> the MP4 (LinkedIn: H.264 High, yuv420p, 30 fps, AAC 192k stereo 48 kHz, +faststart).
#   tools/assemble.sh <variant>          (4x5 or 16x9; reads manifest.json for the size and the output path)
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
cd "$here"
variant=${1:-4x5}
read -r width height out < <(python3 -c "import json,sys; v=json.load(open('manifest.json'))['variants']['$variant']; print(v['width'], v['height'], v['out'])")
frames="frames/$variant"
n=$(ls "$frames" | grep -c '\.png$')
want=$(python3 -c "import json; print(json.load(open('build/timeline.json'))['frames'])")
[ "$n" = "$want" ] || { echo "assemble: $frames has $n frames, the timeline wants $want" >&2; exit 1; }
mkdir -p "$(dirname "$out")"
# AAC at a constant 192 kb/s: Apple's AudioToolbox encoder in CBR where ffmpeg has it (the native
# encoder undershoots its target on the silent cards), else ffmpeg's own
if /usr/local/bin/ffmpeg -hide_banner -encoders 2>/dev/null | grep -q " aac_at "; then aac=(-c:a aac_at -aac_at_mode cbr -b:a 192k); else aac=(-c:a aac -b:a 192k); fi
/usr/local/bin/ffmpeg -hide_banner -loglevel warning -y \
  -framerate 30 -i "$frames/%05d.png" \
  -i build/audio/soundtrack.wav \
  -map 0:v -map 1:a \
  -vf "scale=${width}:${height}:flags=lanczos+accurate_rnd+full_chroma_int:out_color_matrix=bt709:out_range=tv,format=yuv420p,setparams=color_primaries=bt709:color_trc=bt709:colorspace=bt709:range=tv" \
  -c:v libx264 -profile:v high -level:v 4.1 -preset slow -crf 16 -tune animation -g 60 -bf 2 -r 30 \
  -colorspace bt709 -color_primaries bt709 -color_trc bt709 -color_range tv \
  "${aac[@]}" -ar 48000 -ac 2 \
  -movflags +faststart \
  "$out"
ls -l "$out"
