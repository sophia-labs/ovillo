#!/usr/bin/env python3
"""Check, render and verify Sunskip using the repository's release MUS binary.

python3 render.py                 # complete song, receipt, trace, MP3, summary
python3 render.py --groove        # percussion-only bars 37-40, independent master
"""
import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import re
import shutil
import subprocess
import wave

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
MUS = ROOT / "mus-rs/target/release/mus"
OUT = HERE / "out"


def run_json(args, destination):
    result = subprocess.run([str(MUS), *map(str, args)], capture_output=True, text=True)
    destination.write_text(result.stdout)
    if result.returncode:
        raise SystemExit(result.stderr + result.stdout)
    return json.loads(result.stdout)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--groove", action="store_true")
    parser.add_argument("--verify-only", action="store_true", help="verify existing render without running it again")
    args = parser.parse_args()
    OUT.mkdir(exist_ok=True)
    if not MUS.exists():
        raise SystemExit("First run: cd mus-rs && cargo build --release -p mus-cli")
    score = HERE / "sunskip.mus"
    name = "sunskip"
    expected_worlds = {"sw", "tb", "pr", "pn", "pk"}
    if args.groove:
        # An ordinary self-contained MUS excerpt. Keeping bar-major syntax
        # matters: this renderer does not execute track-major examples.
        source = score.read_text()
        header = source.split("\nbar 1 ", 1)[0]
        header = re.sub(r"bars: 48", "bars: 4", header)
        header = re.sub(r"# score:.*", "# score: Sunskip — Pocket groove, bars 37–40", header)
        header = "\n".join(line for line in header.splitlines()
                           if not line.startswith(("# summary:", "# section:", "# text @")))
        lines = [header]
        for output_bar, original_bar in enumerate(range(37, 41), 1):
            original = next(line for line in source.splitlines()
                            if line.startswith(f"bar {original_bar} "))
            fields = re.findall(r"(\S+)=(.*?)(?=  \S+=|$)", original.split(": ", 1)[1])
            lines.append(f"bar {output_bar}: " + "  ".join(
                f"{voice}={music}" for voice, music in fields if voice.startswith("pk.")
            ))
        name = "pocket-groove"
        score = OUT / f"{name}.mus"
        score.write_text("\n".join(lines) + "\n")
        expected_worlds = {"pk"}
    print(f"Checking {name}...", flush=True)
    check = run_json(["check", score, "--base", HERE], OUT / f"{name}.check.json")
    assert check["clean"] and check["events"], check["diagnostics"]
    print(f"Rendering {name} ({len(check['events'])} score events)...", flush=True)
    if args.verify_only:
        receipt = json.loads((OUT / f"{name}.receipt.json").read_text())
    else:
        receipt = run_json([
            "render", score, "--base", HERE, "-o", OUT / f"{name}.wav", "--json",
            "--energy", OUT / f"{name}.energy.json",
        ], OUT / f"{name}.receipt.json")
    assert receipt["sourceDigest"] == "sha256:" + hashlib.sha256(score.read_bytes()).hexdigest()
    assert receipt["renderDigest"] == "sha256:" + hashlib.sha256((OUT / f"{name}.wav").read_bytes()).hexdigest()
    assert receipt["badTokens"] == [], receipt["badTokens"]
    assert receipt["eventCount"] > 0 and receipt["peakDbfs"] > -100
    expected_touches = Counter()
    for event in check["events"]:
        expected_touches[event["track"]] += len(event["pitches"])
    for voice, count in expected_touches.items():
        if count:
            assert receipt["perTrackEvents"].get(voice) == count, (voice, count)
    assert receipt["eventCount"] == sum(expected_touches.values())
    worlds = receipt["ariadneWorlds"]
    assert {w["instrument"] for w in worlds} == expected_worlds
    for world in worlds:
        for key in ("sourceWork", "controlWork", "internalLoss", "radiation",
                    "cumulativeClosureResidual", "maximumFrameResidual"):
            assert math.isfinite(world[key]), (world["instrument"], key)
        assert abs(world["cumulativeClosureResidual"]) < 1e-8
        assert abs(world["maximumFrameResidual"]) < 1e-10
        assert world.get("refusedStrikes", 0) == 0
    if not args.groove:
        assert sum(w.get("intonationTouches", 0) for w in worlds) == 36
    with wave.open(str(OUT / f"{name}.wav")) as wav:
        audio = {"sampleRate": wav.getframerate(), "channels": wav.getnchannels(),
                 "frames": wav.getnframes(), "sampleBytes": wav.getsampwidth(),
                 "seconds": wav.getnframes()/wav.getframerate()}
    trace = json.loads((OUT / f"{name}.energy.json").read_text())
    summary = {
        "schema": "sunskip.validation.v1", "score": score.name,
        "sourceFileSha256": hashlib.sha256(score.read_bytes()).hexdigest(),
        "renderDigest": receipt["renderDigest"], "checkClean": check["clean"],
        "checkDiagnostics": check["diagnostics"], "badTokens": receipt["badTokens"],
        "audio": audio, "peakDbfs": receipt["peakDbfs"], "rmsDbfs": receipt["rmsDbfs"],
        "eventCount": receipt["eventCount"], "perTrackEvents": receipt["perTrackEvents"],
        "energyTraceSchema": trace["schema"],
        "mp3Export": {"codec": "libmp3lame", "quality": 2,
                      "finalFadeSeconds": 0.0 if args.groove else 0.5,
                      "note": "Listening copy only; the receipted WAV has no added export fade."},
        "worlds": [{k: v for k, v in w.items() if k not in ("keys", "strings", "bodies")}
                   | {"bodyCount": len(w["bodies"])} for w in worlds],
        "interpretation": "Numerical/render verification; no independent perceptual listening claim.",
    }
    (OUT / f"{name}.validation.json").write_text(json.dumps(summary, indent=2) + "\n")
    if not args.groove:
        (HERE / "validation.json").write_text(json.dumps(summary, indent=2) + "\n")
    if shutil.which("ffmpeg"):
        fade = [] if args.groove else ["-af", f"afade=t=out:st={audio['seconds'] - 0.5}:d=0.5"]
        subprocess.run([
            "ffmpeg", "-v", "error", "-y", "-i", str(OUT / f"{name}.wav"),
            *fade,
            "-codec:a", "libmp3lame", "-q:a", "2", "-metadata", "title=" + name,
            str(OUT / f"{name}.mp3"),
        ], check=True)
    print(json.dumps(summary, indent=2), flush=True)


if __name__ == "__main__":
    main()
