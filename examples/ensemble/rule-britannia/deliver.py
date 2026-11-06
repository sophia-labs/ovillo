#!/usr/bin/env python3
"""Deliver the mixed *Rule, Britannia!* to the film's score folder. Stdlib only.

    python3 deliver.py [--dest DIR]

Copies out/mix/rule_britannia.wav (48 kHz / 24-bit) and the stems, encodes
rule_britannia.mp3 with ffmpeg (libmp3lame, 320 kb/s), runs the film's PS1
listening chain (`~/dev/verabot/film/audio/tools/ps1_chain.py`, not physics)
for rule_britannia_ps1.wav, copies the score sources (the MUS scores, the note
lists, the plans and designs, the scripts) and prints the receipt tables for
NOTES.md from out/validation.json and out/mix/mix-report.json.
"""
import argparse
import hashlib
import json
import shutil
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE / "out"
MIX = OUT / "mix"
PS1 = Path.home() / "dev/verabot/film/audio/tools/ps1_chain.py"
DEST = Path.home() / "dev/verabot/film/audio/score/rule_britannia"


def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dest", default=str(DEST))
    ap.add_argument("--no-ps1", action="store_true")
    args = ap.parse_args()
    dest = Path(args.dest)
    dest.mkdir(parents=True, exist_ok=True)
    (dest / "stems").mkdir(exist_ok=True)
    (dest / "score").mkdir(exist_ok=True)
    wav = MIX / "rule_britannia.wav"
    shutil.copy2(wav, dest / "rule_britannia.wav")
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(wav), "-codec:a", "libmp3lame", "-b:a", "320k",
                    str(dest / "rule_britannia.mp3")], check=True)
    if not args.no_ps1:
        subprocess.run(["uv", "run", "-q", str(PS1), str(wav), str(dest / "rule_britannia_ps1.wav")], check=True)
    for s in sorted((MIX / "stems").glob("*.wav")):
        shutil.copy2(s, dest / "stems" / s.name)
    for sub in ("scores", "brass", "choir"):
        (dest / "score" / sub).mkdir(exist_ok=True)
        for f in sorted((HERE / sub).glob("*")):
            if f.is_file():
                shutil.copy2(f, dest / "score" / sub / f.name)
    for f in sorted(HERE.glob("*.py")) + sorted(HERE.glob("*.json")) + [HERE / "README.md"]:
        shutil.copy2(f, dest / "score" / f.name)
    shutil.copy2(OUT / "validation.json", dest / "score" / "validation.json")
    shutil.copy2(MIX / "mix-report.json", dest / "score" / "mix-report.json")
    # the receipt tables
    v = json.loads((OUT / "validation.json").read_text())
    print("\n| part | World | source work (J) | control work (J) | internal loss (J) | radiation (J) | max frame residual (J) | cumulative residual (J) |")
    print("|---|---|---|---|---|---|---|---|")
    for e in v["pieces"]:
        if not e["piece"].startswith("rb-"):
            continue
        for w in e["worlds"]:
            print(f"| {e['piece']} | {w['instrument']} | {w['sourceWork']:.4g} | {w['controlWork']:.3g} | {w['internalLoss']:.4g} | {w['radiation']:.4g} | {w['maximumFrameResidual']:.2e} | {w['cumulativeClosureResidual']:.2e} |")
    print("\nfailures:", v["failures"])
    m = json.loads((MIX / "mix-report.json").read_text())
    print("\nmaster:", json.dumps(m["master"], indent=1))
    print("\n| stem | file | gain dB | pan | high-pass Hz | LUFS (gain applied) | peak dBFS |")
    print("|---|---|---|---|---|---|---|")
    for n, r in m["stems"].items():
        print(f"| {n} | {r['file']} | {r['gainDb']:+.1f} | {r['pan']:+.2f} | {r['highPassHz'] or '-'} | {r['lufsIntegrated']:.1f} | {r['peakDbfs']:.1f} |")
    print("\n| bar | section | s | mix LUFS | " + " | ".join(m["stems"]) + " |")
    print("|---|---|---|---|" + "---|" * len(m["stems"]))
    for row in m["sections"]:
        print(f"| {row['bar']} | {row['section']} | {row['seconds']} | {row['mixLufs']:.1f} | " + " | ".join(f"{row[s]:.1f}" for s in m["stems"]) + " |")
    print("\nsha256:")
    for f in ("rule_britannia.wav", "rule_britannia.mp3", "rule_britannia_ps1.wav"):
        p = dest / f
        if p.exists():
            print(f"  {f}  {sha(p)}")


if __name__ == "__main__":
    main()
