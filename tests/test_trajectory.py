from __future__ import annotations

import json

import pytest

from mus_analysis.garganta_anatomy import CONTROL_NAMES
from mus_analysis.garganta_trajectory import (
    GARGANTA_ANCHOR_DIM,
    trajectory_to_anchor_genome,
)
from mus_analysis.trajectory import (
    FORMAT_V1,
    Knot,
    Lane,
    LaneTemplate,
    TrajectoryError,
    TrajectoryProgram,
    program_from_arrow_params,
)


def program(*lanes: Lane, duration: int = 101, sample_rate: int = 100) -> TrajectoryProgram:
    return TrajectoryProgram(
        format=FORMAT_V1,
        sample_rate_hz=sample_rate,
        duration_samples=duration,
        hold_final=True,
        lanes=tuple(lanes),
    )


def lane(
    target: str,
    values: tuple[tuple[int, float], ...],
    *,
    curve: str = "linear",
    value_space: str = "linear",
    unit: str = "unitless",
    minimum: float | None = None,
    maximum: float | None = None,
    max_rate: float | None = None,
) -> Lane:
    return Lane(
        target=target,
        unit=unit,
        curve=curve,
        value_space=value_space,
        min_value=minimum,
        max_value=maximum,
        max_rate_per_second=max_rate,
        knots=tuple(Knot(at_sample=sample, value=value) for sample, value in values),
    )


def test_linear_and_log2_interpolation() -> None:
    direct = lane("x", ((0, 2.0), (100, 6.0)))
    ratio = lane("ratio", ((0, 1.0), (100, 4.0)), value_space="log2")
    trajectory = program(direct, ratio)
    trajectory.validate()
    assert direct.value_at(50) == 4.0
    assert ratio.value_at(50) == pytest.approx(2.0)
    assert trajectory.values_at(10_000) == {"x": 6.0, "ratio": 4.0}


def test_arrow_compilation_and_static_start() -> None:
    templates = {
        "body": LaneTemplate(
            target="body",
            unit="unitless",
            min_value=0.0,
            max_value=1.0,
        )
    }
    static, trajectory = program_from_arrow_params(
        {"body": "0.2 -> 0.8 -> 0.3", "pan": "-1->1"},
        templates,
        sample_rate_hz=100,
        duration_samples=101,
        curve="smootherstep",
    )
    assert static["body"] == "0.2"
    assert static["pan"] == "-1->1", "unbound MUS-A arrows stay unrelated"
    assert [knot.at_sample for knot in trajectory.lanes[0].knots] == [0, 50, 100]
    assert trajectory.receipt()["lanes"][0]["continuity"] == "c2"


def test_refusals_are_typed() -> None:
    out_of_range = program(
        lane("body", ((0, 0.2), (100, 1.4)), minimum=0.0, maximum=1.0)
    )
    with pytest.raises(TrajectoryError) as caught:
        out_of_range.validate()
    assert caught.value.code == "trajectory-above-maximum"

    discontinuous = program(
        lane("body", ((0, 0.2), (100, 0.8)), curve="step", max_rate=1000.0)
    )
    with pytest.raises(TrajectoryError) as caught:
        discontinuous.validate()
    assert caught.value.code == "trajectory-rate-exceeded"


def test_digest_is_stable_across_python_roundtrip() -> None:
    trajectory = program(
        lane("body", ((0, 0.2), (100, 0.8)), curve="smoothstep"),
        lane("couple", ((0, 0.1), (100, 0.3))),
    )
    encoded = trajectory.canonical_json()
    decoded = TrajectoryProgram.from_dict(json.loads(encoded))
    assert decoded.digest_hex() == trajectory.digest_hex()
    assert decoded.to_dict() == trajectory.to_dict()


def test_garganta_uses_the_same_program_and_clock() -> None:
    trajectory = program(
        lane("jaw_open", ((0, -0.2), (100, 0.8)), curve="smoothstep"),
        lane("tongue_front", ((0, 0.5), (100, -0.5))),
        lane("gain", ((0, 0.1), (100, 0.9))),
        lane("aspiration", ((0, 0.4), (100, 0.05))),
        lane("f0_ratio", ((0, 1.0), (100, 2.0)), value_space="log2", unit="ratio"),
    )
    genome = trajectory_to_anchor_genome(trajectory)
    assert genome.shape == (8, GARGANTA_ANCHOR_DIM)
    assert genome[0, CONTROL_NAMES.index("jaw_open")] == pytest.approx(-0.2)
    assert genome[-1, CONTROL_NAMES.index("jaw_open")] == pytest.approx(0.8)
    assert genome[0, len(CONTROL_NAMES)] == pytest.approx(0.1)
    assert genome[-1, len(CONTROL_NAMES) + 2] == pytest.approx(12.0)


def test_garganta_refuses_unbound_coordinates() -> None:
    trajectory = program(lane("couple", ((0, 0.1), (100, 0.2))))
    with pytest.raises(TrajectoryError) as caught:
        trajectory_to_anchor_genome(trajectory)
    assert caught.value.code == "garganta-trajectory-unsupported-target"
