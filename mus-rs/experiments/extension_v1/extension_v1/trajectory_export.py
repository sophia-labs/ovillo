"""Lower learned closed Extension gestures into ``mus-trajectory/v1``.

Extension learns piecewise-linear *increments* in a latent control space.  A
persistent body executes coordinate *values* on a sample clock.  This module is
the narrow, inspectable adapter between those two statements:

``learned token loop -> cumulative vertices -> TrajectoryProgram JSON``.

The output deliberately remains body-neutral.  Targets such as
``extension.f0.u2`` are latent coordinates, not aliases for Weave coupling or
GARGANTA anatomy.  A morphology/body adapter must name and validate any
physical interpretation before the path can produce audio.
"""
from __future__ import annotations

import argparse
from copy import deepcopy
from hashlib import sha256
import json
from pathlib import Path
from typing import Any, Mapping, Sequence

FORMAT_V1 = "mus-trajectory/v1"
EXPORT_FORMAT_V1 = "extension-trajectory-export/v1"


class TrajectoryExportError(ValueError):
    """A learned gesture cannot be represented by the declared export."""


def _semantic_digest(value: Mapping[str, Any]) -> str:
    payload = json.dumps(
        value,
        sort_keys=True,
        ensure_ascii=False,
        allow_nan=False,
        separators=(",", ":"),
    ).encode("utf-8")
    return sha256(payload).hexdigest()


def canonical_program_json(program: Mapping[str, Any]) -> bytes:
    """The same compact insertion-order JSON used by mus-trajectory."""

    return json.dumps(
        program,
        ensure_ascii=False,
        allow_nan=False,
        separators=(",", ":"),
    ).encode("utf-8")


def program_digest(program: Mapping[str, Any]) -> str:
    return sha256(canonical_program_json(program)).hexdigest()


def _select_loop(
    result: Mapping[str, Any], *, fiber: int, token: int
) -> list[list[float]]:
    if result.get("model") != "closed-clifford-token-gestures":
        raise TrajectoryExportError(
            "source is not a closed-clifford-token-gestures receipt"
        )
    loops = result.get("loops")
    if not isinstance(loops, list) or not loops:
        raise TrajectoryExportError("source receipt contains no learned loops")
    try:
        raw_loop = loops[fiber][token]
    except (IndexError, TypeError) as exc:
        raise TrajectoryExportError(
            f"loop fiber={fiber} token={token} is outside the source receipt"
        ) from exc
    if not isinstance(raw_loop, list) or not raw_loop:
        raise TrajectoryExportError("selected loop has no segments")
    dimension = len(raw_loop[0])
    if dimension == 0 or any(len(segment) != dimension for segment in raw_loop):
        raise TrajectoryExportError("selected loop has inconsistent latent dimension")
    try:
        return [[float(value) for value in segment] for segment in raw_loop]
    except (TypeError, ValueError) as exc:
        raise TrajectoryExportError("selected loop contains a non-numeric value") from exc


def closed_token_to_trajectory(
    result: Mapping[str, Any],
    *,
    fiber: int = 0,
    token: int = 0,
    sample_rate_hz: int = 1_000,
    seconds_per_segment: float = 1.0,
    closure_tolerance: float = 1e-12,
) -> tuple[dict[str, Any], dict[str, Any]]:
    """Export one learned token loop and a provenance receipt.

    The learned JSON stores increments.  The returned program stores the
    cumulative vertex after each segment, beginning at zero.  The model
    constructs the final increment as the negative sum of all previous
    increments; after checking the serialized roundoff residual against
    ``closure_tolerance``, the exporter canonicalizes the final vertex to exact
    zero so downstream bodies receive the mathematical loop rather than its
    JSON rounding noise.
    """

    if sample_rate_hz <= 0:
        raise TrajectoryExportError("sample_rate_hz must be positive")
    if not (seconds_per_segment > 0.0):
        raise TrajectoryExportError("seconds_per_segment must be positive")
    if closure_tolerance < 0.0:
        raise TrajectoryExportError("closure_tolerance must be non-negative")

    increments = _select_loop(result, fiber=fiber, token=token)
    segment_samples = round(sample_rate_hz * seconds_per_segment)
    if segment_samples <= 0:
        raise TrajectoryExportError("segment duration rounds to zero samples")

    dimension = len(increments[0])
    current = [0.0] * dimension
    vertices: list[list[float]] = [current.copy()]
    for increment in increments:
        current = [left + right for left, right in zip(current, increment)]
        vertices.append(current.copy())
    source_residual = max(abs(value) for value in vertices[-1])
    if source_residual > closure_tolerance:
        raise TrajectoryExportError(
            f"selected learned path is open: residual {source_residual:.6e} exceeds "
            f"{closure_tolerance:.6e}"
        )
    vertices[-1] = [0.0] * dimension

    lanes: list[dict[str, Any]] = []
    for coordinate in range(dimension):
        lanes.append(
            {
                "target": f"extension.f{fiber}.u{coordinate}",
                "unit": "latent_control",
                "curve": "linear",
                "value_space": "linear",
                "knots": [
                    {
                        "at_sample": vertex_index * segment_samples,
                        "value": vertex[coordinate],
                    }
                    for vertex_index, vertex in enumerate(vertices)
                ],
            }
        )

    duration_samples = len(increments) * segment_samples + 1
    program: dict[str, Any] = {
        "format": FORMAT_V1,
        "sample_rate_hz": sample_rate_hz,
        "duration_samples": duration_samples,
        "hold_final": True,
        "lanes": lanes,
    }
    provenance = {
        "format": EXPORT_FORMAT_V1,
        "source_model": result["model"],
        "source_semantic_sha256": _semantic_digest(result),
        "fiber": fiber,
        "token": token,
        "segments": len(increments),
        "latent_dimension": dimension,
        "source_loop_closure_residual": source_residual,
        "closure_tolerance": closure_tolerance,
        "final_vertex_canonicalized_to_zero": True,
        "program_sha256": program_digest(program),
        "claim_discipline": {
            "learned_path_is_closed": True,
            "latent_coordinates_are_physical_controls": False,
            "body_adapter_still_required": True,
            "this_export_adds_no_audio_claim": True,
        },
    }
    return program, provenance


def write_closed_token_trajectory(
    result: Mapping[str, Any],
    program_path: Path,
    *,
    provenance_path: Path | None = None,
    fiber: int = 0,
    token: int = 0,
    sample_rate_hz: int = 1_000,
    seconds_per_segment: float = 1.0,
    closure_tolerance: float = 1e-12,
) -> tuple[dict[str, Any], dict[str, Any]]:
    program, provenance = closed_token_to_trajectory(
        result,
        fiber=fiber,
        token=token,
        sample_rate_hz=sample_rate_hz,
        seconds_per_segment=seconds_per_segment,
        closure_tolerance=closure_tolerance,
    )
    program_path.parent.mkdir(parents=True, exist_ok=True)
    program_path.write_text(
        json.dumps(program, indent=2, ensure_ascii=False, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    if provenance_path is not None:
        provenance_path.parent.mkdir(parents=True, exist_ok=True)
        provenance_path.write_text(
            json.dumps(provenance, indent=2, sort_keys=True, allow_nan=False) + "\n",
            encoding="utf-8",
        )
    return program, provenance


def _main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--provenance", type=Path)
    parser.add_argument("--fiber", type=int, default=0)
    parser.add_argument("--token", type=int, default=0)
    parser.add_argument("--sample-rate", type=int, default=1_000)
    parser.add_argument("--seconds-per-segment", type=float, default=1.0)
    args = parser.parse_args()
    result = json.loads(args.source.read_text(encoding="utf-8"))
    _, provenance = write_closed_token_trajectory(
        result,
        args.output,
        provenance_path=args.provenance,
        fiber=args.fiber,
        token=args.token,
        sample_rate_hz=args.sample_rate,
        seconds_per_segment=args.seconds_per_segment,
    )
    print(json.dumps(provenance, indent=2, sort_keys=True))


if __name__ == "__main__":
    _main()
