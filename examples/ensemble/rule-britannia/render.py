#!/usr/bin/env python3
"""Check and render every part of *Rule, Britannia!* through its World, and
the three auditions. Stdlib only. Needs the release builds (repository root):

    (cd mus-rs && cargo build --release -p mus-cli)
    (cd ovillo && cargo build --release -p ovillo-dsp --no-default-features --example trombone_nd_score)
    (cd ovillo && cargo build --release -p ovillo-tract-world --example tract_choir)

then:

    python3 examples/ensemble/rule-britannia/build_instruments.py
    python3 examples/ensemble/rule-britannia/compose.py
    python3 examples/ensemble/rule-britannia/render.py [name ...] [--reuse]

`--reuse` rebuilds validation.json from the receipts already in out/ (no
render); without it, every named piece (or all) is rendered again.

Writes into out/: one WAV per piece and World family (each instrument of the
piece is its own score, so each render is a stem), its receipt, the energy
trace for the tab, piano and trombone Worlds, and validation.json: per World
the ledger (source work, control work, internal loss, radiation) and the
closure residuals, bad tokens, touch counts and the first touch time. It
fails if a score does not check clean, a render reports bad tokens, a ledger
is not finite, or a residual is larger than 1e-9 J per frame. The choir uses
the twelve singers and calibration tables of ../before-the-law/choir.
"""
import json
import math
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OUT = HERE / "out"
MUS = ROOT / "mus-rs/target/release/mus"
TROMBONE = ROOT / "ovillo/target/release/examples/trombone_nd_score"
CHOIR = ROOT / "ovillo/target/release/examples/tract_choir"
BTL = HERE.parent / "before-the-law"
SINGERS = BTL / "choir/singers.tsv"
SKILL = BTL / "choir/skill"

TAB = ["walker-audition", "vibes-audition", "kit-audition", "rb-walker", "rb-vibes", "rb-kit", "rb-piano"]
TROMBONE_LISTS = ["rb-tb1", "rb-tb2", "rb-tb3"]
CHOIR_LISTS = ["rb-choir-low", "rb-choir-high"]
FRAME_BOUND = 1e-9
# --reuse: rebuild validation.json from the receipts already in out/ instead of rendering again
REUSE = "--reuse" in sys.argv


def run(cmd):
    p = subprocess.run([str(c) for c in cmd], capture_output=True, text=True)
    if p.returncode != 0:
        raise SystemExit(f"failed: {' '.join(map(str, cmd))}\n{p.stdout[-2000:]}\n{p.stderr[-2000:]}")
    return p.stdout


def tab(name):
    score = HERE / "scores" / f"{name}.mus"
    check = json.loads(run([MUS, "check", score, "--json"]))
    (OUT / f"{name}.check.json").write_text(json.dumps(check, indent=1) + "\n")
    if not check["clean"]:
        raise SystemExit(f"{name}: mus check is not clean: {check['diagnostics']}")
    stem = f"{name}.tab"
    if REUSE and (OUT / f"{stem}.receipt.json").exists():
        receipt = json.loads((OUT / f"{stem}.receipt.json").read_text())
    else:
        receipt = json.loads(run([MUS, "render", score, "-o", OUT / f"{stem}.wav", "--json", "--energy", OUT / f"{stem}.energy.json"]))
        (OUT / f"{stem}.receipt.json").write_text(json.dumps(receipt, indent=1) + "\n")
    energy = json.loads((OUT / f"{stem}.energy.json").read_text())
    first = {}
    for w in energy.get("worlds", []):
        ts = [t["seconds"] for t in w.get("touches", [])]
        if ts:
            first[w.get("instrument", "?")] = min(ts)
    worlds = []
    for w in receipt["ariadneWorlds"]:
        keep = {k: w.get(k) for k in ("instrument", "anatomy", "planDigest", "listenerDigest", "plucks", "hammerOns", "pullOffs", "slides",
                                       "strikes", "refusedStrikes", "damperFalls", "pedalSpans", "simulatedSeconds", "sourceWork", "controlWork",
                                       "internalLoss", "radiation", "boardRadiation", "stringRadiation", "cumulativeClosureResidual", "maximumFrameResidual")}
        if isinstance(w.get("keys"), list):
            keep["keyCount"] = len(w["keys"])
        keep["firstTouchSeconds"] = first.get(w["instrument"])
        worlds.append(keep)
    return {"piece": name, "kind": "tab", "wav": f"{stem}.wav", "renderDigest": receipt["renderDigest"],
            "badTokens": receipt["badTokens"], "warnings": receipt.get("warnings"), "durationSeconds": receipt["durationSeconds"],
            "peakDbfs": receipt["peakDbfs"], "rmsDbfs": receipt["rmsDbfs"], "wallSeconds": receipt["wallSeconds"], "worlds": worlds}


def trombone(name):
    stem = f"{name}.trombone"
    out = "" if REUSE and (OUT / f"{stem}-receipt.json").exists() else run([TROMBONE, HERE / "brass" / f"{name}.tsv", OUT, stem])
    r = json.loads((OUT / f"{stem}-receipt.json").read_text())
    led = r["ledger"]
    return {"piece": name, "kind": "trombone", "wav": f"{stem}.wav", "planDigest": r["planDigest"], "notes": r["notes"],
            "hairpins": r["hairpins"], "partials": r["partials"], "positions": r["positions"], "peak": r["peak"],
            "realTimeFactorWall": r["realTimeFactorWall"],
            "worlds": [{"instrument": name, "sourceWork": led["sourceWork"], "controlWork": led["controlWork"],
                        "internalLoss": led["internalLoss"], "radiation": led["radiation"],
                        "cumulativeClosureResidual": led["cumulativeClosureResidual"], "maximumFrameResidual": led["maximumFrameResidual"]}],
            "log": out.strip().splitlines()}


def choir(name):
    stem = f"{name}.choir"
    out = "" if REUSE and (OUT / f"{stem}-receipt.json").exists() else run([CHOIR, SINGERS, HERE / "choir" / f"{name}.tsv", SKILL, OUT, stem])
    r = json.loads((OUT / f"{stem}-receipt.json").read_text())
    worlds = []
    for s in r["singers"]:
        led = s["ledger"]
        worlds.append({"instrument": f"choir/{s['singer']}", "voice": s["anatomy"]["voice"], "planDigest": led["planDigest"], "sourceWork": led["sourceWork"],
                       "controlWork": led["controlWork"], "internalLoss": led["internalLoss"], "radiation": led["radiation"],
                       "cumulativeClosureResidual": led["cumulativeClosureResidual"], "maximumFrameResidual": led["maximumFrameResidual"],
                       "safetyLimited": s["safetyLimited"], "nonFinite": s["nonFinite"]})
    return {"piece": name, "kind": "choir", "wav": f"{stem}.wav", "mixPeak": r["mixPeak"], "worlds": worlds, "log": out.strip().splitlines()}


def validate(entry):
    problems = []
    if entry.get("badTokens"):
        problems.append(f"bad tokens {entry['badTokens']}")
    for w in entry["worlds"]:
        for k in ("sourceWork", "internalLoss", "radiation", "cumulativeClosureResidual", "maximumFrameResidual"):
            if w.get(k) is None or not math.isfinite(w[k]):
                problems.append(f"{w['instrument']}: {k} not finite")
        if abs(w.get("maximumFrameResidual") or 0) > FRAME_BOUND:
            problems.append(f"{w['instrument']}: frame residual {w['maximumFrameResidual']:.3e} J")
        if w.get("nonFinite"):
            problems.append(f"{w['instrument']}: non-finite samples")
    return problems


def main():
    OUT.mkdir(exist_ok=True)
    only = {a for a in sys.argv[1:] if not a.startswith("--")}
    jobs = [(tab, n) for n in TAB] + [(trombone, n) for n in TROMBONE_LISTS] + [(choir, n) for n in CHOIR_LISTS]
    jobs = [(f, n) for f, n in jobs if not only or n in only or f"{n}.{f.__name__}" in only]
    with ThreadPoolExecutor(max_workers=8) as pool:
        results = list(pool.map(lambda j: j[0](j[1]), jobs))
    path = OUT / "validation.json"
    old = json.loads(path.read_text()) if path.exists() and only else {"pieces": []}
    keep = [e for e in old["pieces"] if (e["piece"], e["kind"]) not in {(r["piece"], r["kind"]) for r in results}]
    pieces = sorted(keep + results, key=lambda e: (e["piece"], e["kind"]))
    failures = {f"{e['piece']}.{e['kind']}": validate(e) for e in pieces if validate(e)}
    path.write_text(json.dumps({"schema": "rule-britannia.validation.v1", "frameResidualBoundJ": FRAME_BOUND,
                                "pieces": pieces, "failures": failures}, indent=1) + "\n")
    for e in results:
        worst = max(abs(w.get("maximumFrameResidual") or 0) for w in e["worlds"])
        cum = max(abs(w.get("cumulativeClosureResidual") or 0) for w in e["worlds"])
        print(f"{e['piece']:18s} {e['kind']:8s} worlds {len(e['worlds']):2d}  worst frame residual {worst:.2e} J  worst cumulative {cum:.2e} J  -> out/{e['wav']}")
    if failures:
        raise SystemExit(f"validation failures: {json.dumps(failures, indent=1)}")


if __name__ == "__main__":
    main()
