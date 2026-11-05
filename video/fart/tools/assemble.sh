#!/usr/bin/env bash
# Frames + the guts' mix -> the MP4 (H.264 High, yuv420p, 30 fps, AAC 192k stereo 48 kHz, +faststart).
#   tools/assemble.sh <variant>        16x9 (the film) or 9x16 (the short; cut from manifest.json's cuts)
# The soundtrack is the score's out/mix.wav (the Worlds' listener copies, panned and summed by
# ovillo-fart mix), padded with silence for the cards, then loudnorm two-pass to the manifest's
# target. loudnorm applies one linear gain when the mix's true peak leaves room for it; when the
# gain needed would push the peak past the target (this mix sits near -34 LUFS with the cannon
# at -1 dBTP), it falls back to its dynamic mode, a time-varying gain. Which one was applied is
# in build/audio/<variant>-receipt.json ("applied".normalization_type); the sound of the Worlds is
# not changed in either case beyond that gain.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
cd "$here"
variant=${1:-16x9}
read -r width height out score fps lufs tp lra < <(python3 -c "
import json; m=json.load(open('manifest.json')); v=m['variants']['$variant']; l=m['loudness']
print(v['width'], v['height'], v['out'], m['score'], m['fps'], l['integratedLufs'], l['truePeakDbtp'], l['lra'])")
frames="frames/$variant"
n=$(ls "$frames" | grep -c '\.png$')
mix="$score/out/mix.wav"
mkdir -p build/audio "$(dirname "$out")"
cut=$(python3 -c "import json; print(json.dumps(json.load(open('manifest.json')).get('cuts',{}).get('$variant')))")
if [ "$cut" = "null" ]; then
  read -r leader trailer want < <(python3 -c "import json; f=json.load(open('build/film.json')); print(f['leaderS'], f['trailerS'], f['frames'])")
  afilter="adelay=$(python3 -c "print(int($leader*1000))")|$(python3 -c "print(int($leader*1000))"),apad=pad_dur=$trailer"
  ain=(-i "$mix")
else
  read -r t0 t1 leader trailer fin fout want < <(python3 -c "
import json; m=json.load(open('manifest.json')); c=m['cuts']['$variant']; fps=m['fps']
print(c['t0'], c['t1'], c['leaderS'], c['trailerS'], c['fadeInS'], c['fadeOutS'], round((c['leaderS']+c['t1']-c['t0']+c['trailerS'])*fps))")
  dur=$(python3 -c "print($t1-$t0)")
  afilter="atrim=start=$t0:end=$t1,asetpts=PTS-STARTPTS,afade=t=in:d=$fin,afade=t=out:st=$(python3 -c "print($dur-$fout)"):d=$fout,adelay=$(python3 -c "print(int($leader*1000))")|$(python3 -c "print(int($leader*1000))"),apad=pad_dur=$trailer"
  ain=(-i "$mix")
fi
[ "$n" = "$want" ] || { echo "assemble: $frames has $n frames, the cut wants $want" >&2; exit 1; }
# loudnorm two-pass on the padded soundtrack
/usr/local/bin/ffmpeg -hide_banner -loglevel warning -y "${ain[@]}" -af "$afilter" -ar 48000 -ac 2 build/audio/$variant-padded.wav
stats=$(/usr/local/bin/ffmpeg -hide_banner -nostats -i build/audio/$variant-padded.wav -af "loudnorm=I=$lufs:TP=$tp:LRA=$lra:print_format=json" -f null - 2>&1 | sed -n '/^{/,/^}/p')
read -r mi mtp mlra mth < <(python3 -c "import json,sys; s=json.loads(sys.argv[1]); print(s['input_i'], s['input_tp'], s['input_lra'], s['input_thresh'])" "$stats")
/usr/local/bin/ffmpeg -hide_banner -nostats -y -i build/audio/$variant-padded.wav \
  -af "loudnorm=I=$lufs:TP=$tp:LRA=$lra:measured_I=$mi:measured_TP=$mtp:measured_LRA=$mlra:measured_thresh=$mth:linear=true:print_format=json" \
  -ar 48000 -ac 2 build/audio/$variant-soundtrack.wav 2> build/audio/$variant-loudnorm.txt
applied=$(sed -n '/^{/,/^}/p' build/audio/$variant-loudnorm.txt)
echo "{\"measured\": $stats, \"target\": {\"I\": $lufs, \"TP\": $tp, \"LRA\": $lra}, \"applied\": $applied}" > build/audio/$variant-receipt.json
python3 -c "import json; a=json.load(open('build/audio/$variant-receipt.json'))['applied']; print('loudnorm', a['normalization_type'], 'output_i', a['output_i'], 'output_tp', a['output_tp'])"
if /usr/local/bin/ffmpeg -hide_banner -encoders 2>/dev/null | grep -q " aac_at "; then aac=(-c:a aac_at -aac_at_mode cbr -b:a 192k); else aac=(-c:a aac -b:a 192k); fi
/usr/local/bin/ffmpeg -hide_banner -loglevel warning -y \
  -framerate "$fps" -i "$frames/%05d.png" \
  -i build/audio/$variant-soundtrack.wav \
  -map 0:v -map 1:a -shortest \
  -vf "scale=${width}:${height}:flags=lanczos+accurate_rnd+full_chroma_int:out_color_matrix=bt709:out_range=tv,format=yuv420p,setparams=color_primaries=bt709:color_trc=bt709:colorspace=bt709:range=tv" \
  -c:v libx264 -profile:v high -level:v 4.2 -preset medium -crf 17 -g 60 -bf 2 -r "$fps" \
  -colorspace bt709 -color_primaries bt709 -color_trc bt709 -color_range tv \
  "${aac[@]}" -ar 48000 -ac 2 \
  -movflags +faststart \
  "$out"
ls -l "$out"
