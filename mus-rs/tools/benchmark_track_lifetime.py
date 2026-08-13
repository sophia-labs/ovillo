#!/usr/bin/env python3
"""Benchmark event-local versus work-receipted track-lifetime rendering.

Both scores contain three notated events, three physical unity attacks, the same
constant authored control trajectory, and no postmix dynamic attenuation. The
intervention is body lifetime:

- event-local: three body constructions and tails;
- track lifetime: one persistent body and tail, plus exact attack-work receipt.

Audio digests are not expected to match because retained state is the feature
under test. One-attack byte identity is covered by Rust unit tests.
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

EVENT_SCORE = """# score: event-local work-gate benchmark
# tempo: 120
# time: 4/4
# bars: 2
# instruments:
#   w = weave (treble, synth=weave, courses=11, strum=0, couple=0.10, body=0.36, sus=2.2, damp=0.31, traj_curve=linear, traj_hz=240)

bar 1: w={fff} A3q[couple=0.10@0->0.10@1] Rq A3q[couple=0.10@0->0.10@1] Rq
bar 2: w=A3h[couple=0.10@0->0.10@1] Rh
"""

TRACK_SCORE = """# score: track-owned work-gate benchmark
# tempo: 120
# time: 4/4
# bars: 2
# instruments:
#   w = weave (treble, synth=weave, body_lifetime=track, courses=11, strum=0, couple=0.10@0->0.10@1, body=0.36, sus=2.2, damp=0.31, traj_curve=linear, traj_hz=240)

bar 1: w={fff} A3q Rq A3q Rq
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


def trajectory_summary(receipt: dict[str, Any]) -> dict[str, Any]:
    trajectory_receipts = receipt.get("trajectoryReceipts", [])
    rearticulation = None
    if trajectory_receipts:
        rearticulation = trajectory_receipts[0].get("receipt", {}).get("rearticulation")
    return {
        "trajectoryReceiptCount": len(trajectory_receipts),
        "rearticulation": (
            {
                "attackCount": rearticulation["schedule"]["attack_count"],
                "gains": [attack["gain"] for attack in rearticulation["attacks"]],
                "totalSignedWork": rearticulation["total_signed_work"],
                "totalPositiveWork": rearticulation["total_positive_work"],
                "totalExtractedWork": rearticulation["total_extracted_work"],
                "peakAbsCourseSampleWork": rearticulation[
                    "peak_abs_course_sample_work"
                ],
            }
            if rearticulation is not None
            else None
        ),
    }


def profile(mus: Path, score: Path, root: Path, runs: int) -> dict[str, Any]:
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
            **trajectory_summary(receipt),
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
        "schema": "mus.performance.track-lifetime-benchmark.v3",
        "mus": str(mus),
        "semantics": {
            "eventLocal": (
                "three notated events, three unity attacks, "
                "three body constructions and tails"
            ),
            "trackLifetime": (
                "three notated events, three unity attacks, one persistent "
                "body and tail, exact per-attack work receipt"
            ),
            "controlLaw": "constant authored coupling trajectory on both paths",
            "dynamicLaw": "fff = unity on both paths",
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
