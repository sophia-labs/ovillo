#!/usr/bin/env python3
"""Benchmark event-local versus rearticulated track-lifetime rendering.

This is a measurement harness, not a CI threshold. Both scores contain three
notated events and three physical attacks. The event-local score constructs
three bodies and tails; the track-lifetime score injects all three attacks into
one persistent body and renders one tail.

Audio digests are not expected to match because retained physical state is the
feature under test. The one-attack byte law is covered by Rust unit tests.
"""

from __future__ import annotations

import argparse
import json
import statistics
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any

# A constant arrow is intentional. It enters the trajectory executor while
# leaving coupling physically static, so the benchmark isolates body lifetime
# and attack scheduling rather than comparing different control walks.
EVENT_SCORE = """# score: event-local body benchmark
# tempo: 120
# time: 4/4
# bars: 2
# instruments:
#   w = weave (treble, synth=weave, courses=11, strum=0, couple=0.10, body=0.36, sus=2.2, damp=0.31, traj_curve=linear, traj_hz=240)

bar 1: w=A3q[couple=0.10@0->0.10@1] Rq A3q[couple=0.10@0->0.10@1] Rq
bar 2: w=A3h[couple=0.10@0->0.10@1] Rh
"""

TRACK_SCORE = """# score: track-owned rearticulated body benchmark
# tempo: 120
# time: 4/4
# bars: 2
# instruments:
#   w = weave (treble, synth=weave, body_lifetime=track, courses=11, strum=0, couple=0.10@0->0.10@1, body=0.36, sus=2.2, damp=0.31, traj_curve=linear, traj_hz=240)

bar 1: w=A3q Rq A3q Rq
bar 2: w=A3h Rh
"""


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    if not ordered:
        raise ValueError("percentile of empty sample")
    index = round((len(ordered) - 1) * fraction)
    return ordered[index]


def render_once(mus: Path, score: Path, output: Path) -> tuple[float, dict[str, Any]]:
    started = time.perf_counter()
    completed = subprocess.run(
        [str(mus), "render", str(score), "-o", str(output), "--json"],
        check=False,
        capture_output=True,
        text=True,
    )
    elapsed = time.perf_counter() - started
    if completed.returncode != 0:
        raise RuntimeError(
            f"render failed for {score.name}\n"
            f"stdout:\n{completed.stdout}\n"
            f"stderr:\n{completed.stderr}"
        )
    try:
        receipt = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(
            f"render returned non-JSON stdout for {score.name}: {completed.stdout!r}"
        ) from error
    return elapsed, receipt


def profile(mus: Path, score: Path, root: Path, runs: int) -> dict[str, Any]:
    # One untimed warm-up pays process/filesystem cold effects without hiding
    # them inside the measured distribution.
    render_once(mus, score, root / f"{score.stem}-warm.wav")
    elapsed: list[float] = []
    receipt: dict[str, Any] | None = None
    for index in range(runs):
        seconds, receipt = render_once(
            mus,
            score,
            root / f"{score.stem}-{index:02d}.wav",
        )
        elapsed.append(seconds)
    assert receipt is not None
    duration = float(receipt.get("durationSeconds", 0.0))
    trajectory_receipts = receipt.get("trajectoryReceipts", [])
    unresolved: list[str] = []
    for item in trajectory_receipts:
        unresolved.extend(item.get("receipt", {}).get("unresolved_obligations", []))
    return {
        "runs": runs,
        "wallSeconds": {
            "min": min(elapsed),
            "median": statistics.median(elapsed),
            "p90": percentile(elapsed, 0.90),
            "max": max(elapsed),
            "samples": elapsed,
        },
        "wallPerRenderedSecond": (
            statistics.median(elapsed) / duration if duration > 0.0 else None
        ),
        "receipt": {
            "engineVersion": receipt.get("engineVersion"),
            "renderDigest": receipt.get("renderDigest"),
            "eventCount": receipt.get("eventCount"),
            "durationSeconds": receipt.get("durationSeconds"),
            "trajectoryReceiptCount": len(trajectory_receipts),
            "unresolvedObligations": sorted(set(unresolved)),
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--mus",
        type=Path,
        required=True,
        help="path to a built mus CLI, preferably target/release/mus",
    )
    parser.add_argument("--runs", type=int, default=9)
    parser.add_argument(
        "--keep",
        type=Path,
        help="optional directory in which to retain scores and rendered WAVs",
    )
    args = parser.parse_args()
    if args.runs < 1:
        parser.error("--runs must be positive")
    mus = args.mus.expanduser().resolve()
    if not mus.is_file():
        parser.error(f"--mus does not exist: {mus}")

    temporary: tempfile.TemporaryDirectory[str] | None = None
    if args.keep is None:
        temporary = tempfile.TemporaryDirectory(prefix="mus-track-lifetime-bench-")
        root = Path(temporary.name)
    else:
        root = args.keep.expanduser().resolve()
        root.mkdir(parents=True, exist_ok=True)

    event_score = root / "event-local.mus"
    track_score = root / "track-lifetime.mus"
    event_score.write_text(EVENT_SCORE, encoding="utf-8")
    track_score.write_text(TRACK_SCORE, encoding="utf-8")

    event = profile(mus, event_score, root, args.runs)
    track = profile(mus, track_score, root, args.runs)
    event_median = float(event["wallSeconds"]["median"])
    track_median = float(track["wallSeconds"]["median"])
    report = {
        "schema": "mus.performance.track-lifetime-benchmark.v2",
        "mus": str(mus),
        "semantics": {
            "eventLocal": (
                "three notated events, three physical attacks, "
                "three body constructions and tails"
            ),
            "trackLifetime": (
                "three notated events, three physical attacks, "
                "one persistent body construction and tail"
            ),
            "controlLaw": "constant authored coupling trajectory on both paths",
            "audioParityExpected": False,
            "reasonParityIsWithheld": (
                "the track-owned body retains state between attacks; "
                "event-local bodies begin from zero"
            ),
        },
        "eventLocal": event,
        "trackLifetime": track,
        "medianSpeedup": event_median / track_median if track_median > 0.0 else None,
        "artifacts": str(root) if args.keep is not None else None,
    }
    print(json.dumps(report, indent=2, sort_keys=True))
    if temporary is not None:
        temporary.cleanup()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
