#!/usr/bin/env bash
# Check an assembled MP4 the way a viewer meets it, without playing a sound:
#   tools/verify.sh <variant>
# - ffprobe of the container and streams (written to out/<variant>-ffprobe.txt);
# - every frame's luma (signalstats): no frame may be blank (nothing brighter than 40/255);
# - loudness of the encoded soundtrack (EBU R128, true peak), decoded from the AAC;
# - stills at the start, middle and end of every segment, and one contact sheet per segment
#   (samples/<variant>/), for reading back.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
cd "$here"
variant=${1:-4x5}
mp4=$(python3 -c "import json; print(json.load(open('manifest.json'))['variants']['$variant']['out'])")
probe="out/$variant-ffprobe.txt"
/usr/local/bin/ffprobe -hide_banner -v error -show_format -show_streams "$mp4" > "$probe"
grep -E "^(codec_name|profile|width|height|pix_fmt|r_frame_rate|nb_frames|sample_rate|channels|bit_rate|duration|size|color_space)=" "$probe"
echo "-- frames (signalstats)"
/usr/local/bin/ffmpeg -hide_banner -nostats -i "$mp4" -vf "signalstats,metadata=print:key=lavfi.signalstats.YMAX:file=out/$variant-ymax.txt" -an -f null - 2>/dev/null
python3 - "$variant" <<'EOF'
import re, sys
v = sys.argv[1]
ymax = [int(float(m)) for m in re.findall(r"YMAX=([0-9.]+)", open(f"out/{v}-ymax.txt").read())]
blank = [i for i, y in enumerate(ymax) if y < 40]
print(f"{len(ymax)} frames; darkest frame's brightest pixel {min(ymax)}/255 (frame {ymax.index(min(ymax))}); blank frames: {blank or 'none'}")
sys.exit(1 if blank else 0)
EOF
echo "-- soundtrack as encoded"
/usr/local/bin/ffmpeg -hide_banner -nostats -i "$mp4" -vn -af ebur128=peak=true -f null - 2>&1 | sed -n '/Summary:/,$p' | grep -E "I:|LRA:|Peak:"
echo "-- stills"
mkdir -p "samples/$variant"
python3 - "$variant" "$mp4" <<'EOF'
import json, subprocess, sys
v, mp4 = sys.argv[1], sys.argv[2]
tl = json.load(open("build/timeline.json"))
fps = tl["fps"]
for s in tl["segments"]:
    a, n = s["firstFrame"], s["frames"]
    picks = [a + 2, a + n // 4, a + n // 2, a + (3 * n) // 4, a + n - 3]
    files = []
    for k in picks:
        f = f"samples/{v}/{s['id']}-{k:05d}.png"
        subprocess.run(["/usr/local/bin/ffmpeg", "-loglevel", "error", "-y", "-ss", f"{k / fps:.4f}", "-i", mp4, "-frames:v", "1", f], check=True)
        files.append(f)
    subprocess.run(["tools/sheet.sh", f"samples/{v}/sheet-{s['id']}.png", str(len(files)), "432", "-", *files], check=True)
    print(f"{s['id']}: {', '.join(files)}")
EOF
