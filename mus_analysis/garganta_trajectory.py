"""Bind a generic trajectory program to the current GARGANTA renderer.

This is the second production-client adapter for ``mus-trajectory/v1``.
It lowers named tract/source lanes into the eight-anchor genome consumed by
the P0-certified diagnostic renderer.  The adapter does not promote that
renderer into the missing G1/G3 physics: formant motion is still the current
frequency-domain construction, and ``f0_ratio`` still drives the diagnostic
source rather than stateful vocal folds.  What becomes shared and real is the
control artifact, clock, validation, and receipt.
"""

from __future__ import annotations

import math
from typing import Any

import numpy as np

from mus_analysis.garganta_anatomy import CONTROL_NAMES
from mus_analysis.trajectory import TrajectoryError, TrajectoryProgram

GARGANTA_ANCHORS = 8
GARGANTA_CONTROL_DIM = len(CONTROL_NAMES)
GARGANTA_ANCHOR_DIM = GARGANTA_CONTROL_DIM + 3
GARGANTA_SOURCE_TARGETS = ("gain", "aspiration", "f0_ratio", "f0_semitones")
GARGANTA_TARGETS = frozenset((*CONTROL_NAMES, *GARGANTA_SOURCE_TARGETS))


def _refuse(code: str, message: str) -> None:
    raise TrajectoryError(code, message)


def trajectory_to_anchor_genome(
    program: TrajectoryProgram,
    *,
    anchors: int = GARGANTA_ANCHORS,
    base: np.ndarray | None = None,
) -> np.ndarray:
    """Sample one validated program onto GARGANTA's current anchor grid.

    Missing lanes retain ``base``.  With no base, tract controls and F0 offset
    begin neutral, gain is 0.5, and aspiration is 0.15—the same common neutral
    start used by the paired anatomy campaign.
    """

    program.validate()
    unknown = [lane.target for lane in program.lanes if lane.target not in GARGANTA_TARGETS]
    if unknown:
        _refuse(
            "garganta-trajectory-unsupported-target",
            f"GARGANTA does not bind trajectory targets {unknown!r}",
        )
    if anchors < 2:
        _refuse("garganta-trajectory-anchor-count", "GARGANTA needs at least two anchors")

    if base is None:
        genome = np.zeros((anchors, GARGANTA_ANCHOR_DIM), dtype=np.float64)
        genome[:, GARGANTA_CONTROL_DIM] = 0.5
        genome[:, GARGANTA_CONTROL_DIM + 1] = 0.15
    else:
        genome = np.asarray(base, dtype=np.float64).copy()
        if genome.shape != (anchors, GARGANTA_ANCHOR_DIM):
            _refuse(
                "garganta-trajectory-base-shape",
                f"base genome has shape {genome.shape}; expected "
                f"{(anchors, GARGANTA_ANCHOR_DIM)}",
            )

    lane_by_target = {lane.target: lane for lane in program.lanes}
    sample_positions = np.linspace(0, program.duration_samples - 1, anchors)
    sample_indices = np.rint(sample_positions).astype(np.int64)

    for column, target in enumerate(CONTROL_NAMES):
        lane = lane_by_target.get(target)
        if lane is not None:
            genome[:, column] = [lane.value_at(int(sample)) for sample in sample_indices]

    gain = lane_by_target.get("gain")
    if gain is not None:
        genome[:, GARGANTA_CONTROL_DIM] = [
            gain.value_at(int(sample)) for sample in sample_indices
        ]
    aspiration = lane_by_target.get("aspiration")
    if aspiration is not None:
        genome[:, GARGANTA_CONTROL_DIM + 1] = [
            aspiration.value_at(int(sample)) for sample in sample_indices
        ]

    f0_ratio = lane_by_target.get("f0_ratio")
    f0_semitones = lane_by_target.get("f0_semitones")
    if f0_ratio is not None and f0_semitones is not None:
        _refuse(
            "garganta-trajectory-duplicate-f0",
            "author either f0_ratio or f0_semitones, not both",
        )
    if f0_ratio is not None:
        ratios = [f0_ratio.value_at(int(sample)) for sample in sample_indices]
        if any(ratio <= 0.0 for ratio in ratios):
            _refuse(
                "garganta-trajectory-f0-ratio",
                "f0_ratio must remain positive",
            )
        genome[:, GARGANTA_CONTROL_DIM + 2] = [12.0 * math.log2(ratio) for ratio in ratios]
    elif f0_semitones is not None:
        genome[:, GARGANTA_CONTROL_DIM + 2] = [
            f0_semitones.value_at(int(sample)) for sample in sample_indices
        ]

    # The current renderer clips controls/source ranges internally. Refuse at
    # this boundary instead so the receipt describes what actually executes.
    if np.any(genome[:, :GARGANTA_CONTROL_DIM] < -1.0) or np.any(
        genome[:, :GARGANTA_CONTROL_DIM] > 1.0
    ):
        _refuse(
            "garganta-trajectory-control-range",
            "tract controls must remain inside [-1, 1]",
        )
    if np.any(genome[:, GARGANTA_CONTROL_DIM] < 0.0) or np.any(
        genome[:, GARGANTA_CONTROL_DIM] > 1.2
    ):
        _refuse("garganta-trajectory-gain-range", "gain must remain inside [0, 1.2]")
    if np.any(genome[:, GARGANTA_CONTROL_DIM + 1] < 0.0) or np.any(
        genome[:, GARGANTA_CONTROL_DIM + 1] > 1.0
    ):
        _refuse(
            "garganta-trajectory-aspiration-range",
            "aspiration must remain inside [0, 1]",
        )
    if np.any(genome[:, GARGANTA_CONTROL_DIM + 2] < -12.0) or np.any(
        genome[:, GARGANTA_CONTROL_DIM + 2] > 12.0
    ):
        _refuse(
            "garganta-trajectory-f0-range",
            "current diagnostic renderer only admits ±12 semitones",
        )
    return genome


def render_garganta_trajectory(
    renderer: Any,
    program: TrajectoryProgram,
    total_s: float,
    rng: np.random.Generator,
    *,
    peak_normalize: bool = True,
) -> np.ndarray:
    """Render through any P0 ``GestureRenderer``-compatible object."""

    genome = trajectory_to_anchor_genome(program)
    return renderer.render(
        genome,
        total_s,
        rng,
        peak_normalize=peak_normalize,
    )
