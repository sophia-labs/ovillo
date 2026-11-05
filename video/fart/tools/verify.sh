#!/usr/bin/env bash
# Check an assembled MP4 without playing a sound:  tools/verify.sh <variant>
# ffprobe (out/<variant>-ffprobe.txt); every frame's brightest pixel (no blank frames);
# EBU R128 of the encoded soundtrack; stills every 40 s and a contact sheet (samples/<variant>/).
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
cd "$here"
variant=${1:-16x9}
mp4=$(python3 -c "import json; print(json.load(open('manifest.json'))['variants']['$variant']['out'])")
probe="out/$variant-ffprobe.txt"
/usr/local/bin/ffprobe -hide_banner -v error -show_format -show_streams "$mp4" > "$probe"
grep -E "^(codec_name|profile|width|height|pix_fmt|r_frame_rate|nb_frames|sample_rate|channels|bit_rate|duration|size)=" "$probe"
echo "-- frames (signalstats)"
/usr/local/bin/ffmpeg -hide_banner -nostats -i "$mp4" -vf "signalstats,metadata=print:key=lavfi.signalstats.YMAX:file=out/$variant-ymax.txt" -an -f null - 2>/dev/null
python3 - "$variant" <<'EOF'
import re, sys
v = sys.argv[1]
ymax = [int(float(m)) for m in re.findall(r"YMAX=([0-9.]+)", open(f"out/{v}-ymax.txt").read())]
blank = [i for i, y in enumerate(ymax) if y < 40]
print(f"{len(ymax)} frames; darkest frame's brightest pixel {min(ymax)}/255 (frame {ymax.index(min(ymax))}); blank frames: {len(blank) or 'none'}")
sys.exit(1 if blank else 0)
EOF
echo "-- soundtrack as encoded"
/usr/local/bin/ffmpeg -hide_banner -nostats -i "$mp4" -vn -af ebur128=peak=true -f null - 2>&1 | sed -n '/Summary:/,$p' | grep -E "I:|LRA:|Peak:"
echo "-- stills"
mkdir -p "samples/$variant"
dur=$(grep -E "^duration=" "$probe" | head -1 | cut -d= -f2)
python3 - "$variant" "$mp4" "$dur" <<'EOF'
import subprocess, sys
v, mp4, dur = sys.argv[1], sys.argv[2], float(sys.argv[3])
step = 40.0 if dur > 120 else 2.5
files = []
t = 1.0
while t < dur - 0.5:
    f = f"samples/{v}/{int(t):05d}s.png"
    subprocess.run(["/usr/local/bin/ffmpeg", "-loglevel", "error", "-y", "-ss", f"{t:.3f}", "-i", mp4, "-frames:v", "1", f], check=True)
    files.append(f); t += step
cols = 4 if len(files) > 8 else 3
subprocess.run(["../tools/sheet.sh", f"samples/{v}/sheet.png", str(cols), "480", "-", *files], check=True)
print(f"{len(files)} stills, samples/{v}/sheet.png")
EOF
