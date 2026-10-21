"""The soundtrack: each segment's excerpt of its World's WAV, cut sample-exactly, joined.

    uv run --no-project python tools/audio.py manifest.json build/

For every segment (build/timeline.json gives its frame count, so its length in
samples is frames * sampleRate / fps exactly):
  - an instrument segment takes samples [round(window[0] * sr), + length) of its WAV,
    with short fades (fadeIn / fadeOut from the manifest), mono duplicated to stereo;
  - a card, or a segment with no audio yet (the piano placeholder), is digital silence.
Each excerpt is gained to the same integrated loudness (EBU R128 measured by ffmpeg),
the excerpts are joined, and ffmpeg's loudnorm runs two-pass over the whole
(integrated -16 LUFS, true peak -1.5 dBTP, from the manifest). Gain, fades and
loudnorm are the only processing; nothing is added to the Worlds' sound.

Writes build/audio/<id>.wav, build/audio/soundtrack.wav and build/audio/receipt.json.
"""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

FFMPEG = "/usr/local/bin/ffmpeg"


def run(args, capture=True):
    p = subprocess.run([FFMPEG, "-hide_banner", "-nostats", *args], capture_output=capture, text=True)
    if p.returncode != 0:
        sys.stderr.write(p.stderr)
        raise SystemExit(f"ffmpeg failed: {' '.join(args)}")
    return p.stderr


def loudnorm_measure(path, target):
    err = run(["-i", str(path), "-af", f"loudnorm=I={target['integratedLufs']}:TP={target['truePeakDbtp']}:LRA={target['lra']}:print_format=json", "-f", "null", "-"])
    return json.loads(err[err.rindex("{"):err.rindex("}") + 1])


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main(argv):
    manifest_path, build = Path(argv[1]), Path(argv[2])
    root = manifest_path.parent
    manifest = json.loads(manifest_path.read_text())
    timeline = json.loads((build / "timeline.json").read_text())
    sr, fps = manifest["sampleRate"], manifest["fps"]
    target = manifest["loudness"]
    out = build / "audio"
    out.mkdir(parents=True, exist_ok=True)
    per = sr // fps
    assert per * fps == sr, "the sample rate must be a whole number of samples per frame"
    parts, receipt = [], dict(sampleRate=sr, fps=fps, target=target, segments=[])
    for seg in manifest["segments"]:
        tl = next(s for s in timeline["segments"] if s["id"] == seg["id"])
        n = tl["frames"] * per
        dst = out / f"{seg['id']}.wav"
        src = seg.get("audio")
        entry = dict(id=seg["id"], firstSample=tl["firstFrame"] * per, samples=n)
        if src:
            src_path = root / src
            start = round(seg["window"][0] * sr)
            fi, fo = seg.get("fadeIn", 0.0), seg.get("fadeOut", 0.0)
            dur = n / sr
            info = run(["-i", str(src_path), "-f", "null", "-t", "0", "-"])
            mono = re.search(r"Audio: .*?, \d+ Hz, (mono|1 channels)", info) is not None
            chain = [f"atrim=start_sample={start}:end_sample={start + n}", "asetpts=N/SR/TB", f"apad=whole_len={n}"]
            if mono:
                chain.append("pan=stereo|c0=c0|c1=c0")
            if fi > 0:
                chain.append(f"afade=t=in:st=0:d={fi}:curve=hsin")
            if fo > 0:
                chain.append(f"afade=t=out:st={dur - fo:.6f}:d={fo}:curve=hsin")
            chain.append("aformat=sample_fmts=flt:channel_layouts=stereo:sample_rates=48000")
            raw = out / f"{seg['id']}.raw.wav"
            run(["-y", "-i", str(src_path), "-af", ",".join(chain), "-c:a", "pcm_f32le", str(raw)])
            m = loudnorm_measure(raw, target)
            gain = target["integratedLufs"] - float(m["input_i"])
            # A segment whose true peak would pass the ceiling once gained (the tract's lip
            # closure and burst) goes through a look-ahead limiter set just under it; the
            # receipt records it. Nothing else is limited.
            ceiling = target["truePeakDbtp"] - 1.0
            limited = float(m["input_tp"]) + gain > ceiling
            af = f"volume={gain:.4f}dB"
            if limited:
                af += f",alimiter=limit={10 ** (ceiling / 20):.5f}:level=false:attack=5:release=60:asc=1"
            run(["-y", "-i", str(raw), "-af", af, "-c:a", "pcm_f32le", str(dst)])
            raw.unlink()
            after = loudnorm_measure(dst, target)
            entry.update(source=src, sourceSha256=sha256(src_path), startSample=start, endSample=start + n,
                         window=[start / sr, (start + n) / sr], fadeIn=fi, fadeOut=fo, mono=mono,
                         measuredLufs=float(m["input_i"]), measuredTruePeak=float(m["input_tp"]), gainDb=round(gain, 4),
                         limited=limited, limiterCeilingDbtp=ceiling if limited else None,
                         afterLufs=float(after["input_i"]), afterTruePeak=float(after["input_tp"]))
        else:
            run(["-y", "-f", "lavfi", "-i", f"anullsrc=r={sr}:cl=stereo", "-af", f"atrim=end_sample={n}", "-c:a", "pcm_f32le", str(dst)])
            entry.update(source=None, silence=True)
        parts.append(dst)
        receipt["segments"].append(entry)
        print(f"{seg['id']:9s} {n:8d} samples  {entry.get('source') or 'silence'}"
              + (f"  {entry['measuredLufs']:.1f} LUFS -> gain {entry['gainDb']:+.2f} dB"
                 + (f", limited to {entry['limiterCeilingDbtp']} dBTP" if entry['limited'] else "")
                 + f" -> {entry['afterLufs']:.1f} LUFS, TP {entry['afterTruePeak']:.1f}" if src else ""))
    lst = out / "concat.txt"
    lst.write_text("".join(f"file '{p.resolve()}'\n" for p in parts))
    joined = out / "joined.wav"
    run(["-y", "-f", "concat", "-safe", "0", "-i", str(lst), "-c:a", "pcm_f32le", str(joined)])
    m1 = loudnorm_measure(joined, target)
    ln = (f"loudnorm=I={target['integratedLufs']}:TP={target['truePeakDbtp']}:LRA={target['lra']}:"
          f"measured_I={m1['input_i']}:measured_TP={m1['input_tp']}:measured_LRA={m1['input_lra']}:"
          f"measured_thresh={m1['input_thresh']}:offset={m1['target_offset']}:linear=true:print_format=json")
    final = out / "soundtrack.wav"
    # loudnorm resamples internally to 192 kHz; bring it back to 48 kHz and the exact length
    total = timeline["frames"] * per
    err = run(["-y", "-i", str(joined), "-af", f"{ln},aresample=48000,atrim=end_sample={total},apad=whole_len={total}", "-ar", "48000", "-c:a", "pcm_f32le", str(final)])
    m2 = json.loads(err[err.rindex("{"):err.rindex("}") + 1])
    check = loudnorm_measure(final, target)
    receipt.update(pass1=m1, pass2=m2, check=check, soundtrackSha256=sha256(final), totalSamples=total)
    (out / "receipt.json").write_text(json.dumps(receipt, indent=1))
    print(f"loudnorm: {m2.get('normalization_type')} · result {check['input_i']} LUFS, true peak {check['input_tp']} dBTP, LRA {check['input_lra']}")


if __name__ == "__main__":
    main(sys.argv)
