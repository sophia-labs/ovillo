#!/usr/bin/env bash
# A contact sheet of PNG frames: tools/sheet.sh <out.png> <cols> <scale-width> [crop=w:h:x:y|-] <frame.png>...
set -euo pipefail
out=$1 cols=$2 width=$3 crop=$4; shift 4
n=$#
rows=$(( (n + cols - 1) / cols ))
list=$(mktemp "${TMPDIR:-/tmp}/sheet.XXXXXX")
for f in "$@"; do printf "file '%s'\n" "$(cd "$(dirname "$f")" && pwd)/$(basename "$f")" >> "$list"; done
vf="scale=${width}:-1:flags=lanczos,tile=${cols}x${rows}:padding=6:color=0x2c343c"
[ "$crop" != "-" ] && vf="crop=${crop},${vf}"
/usr/local/bin/ffmpeg -loglevel error -y -f concat -safe 0 -i "$list" -vf "$vf" -frames:v 1 "$out"
rm -f "$list"
