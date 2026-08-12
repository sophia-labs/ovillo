from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import pytest
from jsonschema import Draft202012Validator

from mus_analysis.garganta_anatomy import AnatomyGenome, area_function_cm2
from mus_analysis.trajectory import (
    CoordinateContract,
    DisposalPolicy,
    GargantaTrajectoryAdapter,
    Interpolation,
    StateSemantics,
    TrajectoryError,
    TrajectoryKnot,
    TrajectoryProgram,
    TrajectoryTrack,
    equally_spaced_track,
    lower_string_trajectory,
    parse_arrow_values,
    program_from_inline_string_params,
)


ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "mus-rs" / "crates" / "mus-trajectory" / "fixtures" / "trajectory-program-v1.json"
EXPECTED = FIXTURE.with_name("trajectory-program-v1.expected.json")


def load_fixture() -> TrajectoryProgram:
    return TrajectoryProgram.from_json(FIXTURE.read_text("utf-8"))


def test_cross_runtime_fixture_has_pinned_semantic_and_frame_digests() -> None:
    expected = json.loads(EXPECTED.read_text("utf-8"))
    program = load_fixture()
    compiled = program.compile()
    assert program.semantic_digest() == expected["program_digest"]
    assert compiled.receipt.frame_digest == expected["frame_digest"]
    assert compiled.receipt.frame_count == expected["frame_count"]


def test_digest_ignores_track_order_but_not_values() -> None:
    program = load_fixture()
    reordered = TrajectoryProgram(
        schema_version=program.schema_version,
        sample_rate_hz=program.sample_rate_hz,
        duration_samples=program.duration_samples,
        control_stride_samples=program.control_stride_samples,
        metadata=program.metadata,
        tracks=tuple(reversed(program.tracks)),
    )
    assert reordered.semantic_digest() == program.semantic_digest()
    changed_track = reordered.tracks[0]
    changed_knots = list(changed_track.knots)
    changed_knots[1] = TrajectoryKnot(
        changed_knots[1].at_sample, changed_knots[1].value + 0.001
    )
    changed = TrajectoryProgram(
        schema_version=reordered.schema_version,
        sample_rate_hz=reordered.sample_rate_hz,
        duration_samples=reordered.duration_samples,
        control_stride_samples=reordered.control_stride_samples,
        metadata=reordered.metadata,
        tracks=(
            TrajectoryTrack(
                changed_track.coordinate,
                changed_track.interpolation,
                changed_track.contract,
                tuple(changed_knots),
            ),
            *reordered.tracks[1:],
        ),
    )
    assert changed.semantic_digest() != program.semantic_digest()


def test_program_and_receipt_conform_to_schemas() -> None:
    program = load_fixture()
    compiled = program.compile()
    program_schema = json.loads(
        (ROOT / "schemas" / "trajectory-program.schema.json").read_text("utf-8")
    )
    receipt_schema = json.loads(
        (ROOT / "schemas" / "trajectory-receipt.schema.json").read_text("utf-8")
    )
    assert list(Draft202012Validator(program_schema).iter_errors(program.to_dict())) == []
    assert (
        list(
            Draft202012Validator(receipt_schema).iter_errors(
                compiled.receipt.to_dict()
            )
        )
        == []
    )


def test_inline_string_surface_leaves_unrelated_mus_a_arrows_untouched() -> None:
    static, program = program_from_inline_string_params(
        {
            "couple": "0.12@0->0.42@0.6->0.18@1",
            "body": "0.25->0.70->0.35",
            "pitch_ratio": "1->1.0594630943592953",
            "pan": "-1->1",
            "lpf": "70->900",
            "traj_curve": "cosine",
            "traj_hz": "100",
        },
        2.0,
    )
    assert program is not None
    assert static["pan"] == "-1->1"
    assert static["lpf"] == "70->900"
    assert static["couple"] == "0.12"
    assert static["body"] == "0.25"
    assert static["pitch_ratio"] == "1.0"
    assert program.control_stride_samples == 480
    pitch = next(
        track for track in program.tracks if track.coordinate == "string.pitch_ratio"
    )
    assert pitch.interpolation is Interpolation.LINEAR


def test_string_reference_lowering_uses_native_pitch_subset() -> None:
    static, program = program_from_inline_string_params(
        {
            "couple": "0.1->0.4->0.2",
            "pitch_ratio": "1->1.0594630943592953",
            "traj_hz": "200",
        },
        0.5,
    )
    assert program is not None
    assert static["couple"] == "0.1"
    lowered = lower_string_trajectory(program, [110.0, 165.0], 0.5)
    assert lowered.frequencies_hz == (110.0, 165.0)
    assert lowered.native_gliss_ratio == pytest.approx(1.0594630943592953)
    assert lowered.native_pitch_transport == "persistent-linear-f0-to-f1"
    assert "integer-delay-state-transport" in lowered.unresolved_obligations
    assert len(lowered.control_frames) > 20
    assert all(
        target != "pitch_ratio"
        for frame in lowered.control_frames
        for target, _value in frame.sets
    )


def test_nonlinear_pitch_and_strict_open_transport_refuse() -> None:
    _, nonlinear = program_from_inline_string_params(
        {
            "pitch_ratio": "1->1.05->1.1",
            "pitch_ratio_curve": "smoothstep",
        },
        0.5,
    )
    assert nonlinear is not None
    with pytest.raises(TrajectoryError) as nonlinear_error:
        lower_string_trajectory(nonlinear, [110.0], 0.5)
    assert nonlinear_error.value.code == "string-trajectory-pitch-path-unsupported"

    _, strict = program_from_inline_string_params(
        {
            "pitch_ratio": "1->1.02",
            "bow": "0.1->0.5",
            "traj_strict": "1",
        },
        0.5,
    )
    assert strict is not None
    with pytest.raises(TrajectoryError) as strict_error:
        lower_string_trajectory(strict, [110.0], 0.5)
    assert strict_error.value.code == "string-trajectory-transport-uncertified"


def test_garganta_adapter_moves_real_area_map_with_static_anatomy() -> None:
    anatomy = AnatomyGenome.from_unit_cube([0.5] * 10)
    before = anatomy.to_dict()
    duration = 1_600
    tracks = (
        equally_spaced_track(
            "garganta.control.jaw_open",
            (-0.2, 0.6),
            duration,
            Interpolation.SMOOTHSTEP,
            CoordinateContract.fixed("normalized", -1.0, 1.0),
        ),
        equally_spaced_track(
            "garganta.control.tongue_front",
            (0.0, 0.45),
            duration,
            Interpolation.COSINE,
            CoordinateContract.fixed("normalized", -1.0, 1.0),
        ),
        equally_spaced_track(
            "garganta.source.pitch_ratio",
            (1.0, 1.08),
            duration,
            Interpolation.LINEAR,
            CoordinateContract.fixed("ratio", 0.5, 2.0),
        ),
        equally_spaced_track(
            "garganta.tract_area_cm2.3",
            (0.8, 0.12),
            duration,
            Interpolation.SMOOTHERSTEP,
            CoordinateContract.fixed("cm2", 0.025, 18.0),
        ),
    )
    program = TrajectoryProgram(
        schema_version="trajectory-program-v1",
        sample_rate_hz=16_000,
        duration_samples=duration,
        control_stride_samples=80,
        metadata={"purpose": "garganta-adapter-test"},
        tracks=tracks,
    )
    adapter = GargantaTrajectoryAdapter(program)
    start = adapter.state_at(anatomy, 0)
    middle = adapter.state_at(anatomy, duration // 2)
    end = adapter.state_at(anatomy, duration)
    assert start.area_function_cm2[3] == pytest.approx(0.8)
    assert end.area_function_cm2[3] == pytest.approx(0.12)
    assert middle.pitch_ratio == pytest.approx(1.04)
    assert start.articulation_controls != end.articulation_controls
    neutral = area_function_cm2(anatomy, [0.0] * 5)
    assert not np.allclose(end.area_function_cm2, neutral)
    states, receipt = adapter.compile_states(anatomy)
    assert len(states) == program.compile().receipt.frame_count
    assert receipt.anatomy_static is True
    assert anatomy.to_dict() == before


def test_garganta_adapter_refuses_foreign_coordinates_and_bad_area_indices() -> None:
    program = TrajectoryProgram(
        schema_version="trajectory-program-v1",
        sample_rate_hz=16_000,
        duration_samples=1_600,
        control_stride_samples=80,
        tracks=(
            equally_spaced_track(
                "string.couple",
                (0.1, 0.2),
                1_600,
                Interpolation.LINEAR,
                CoordinateContract.fixed("radian", 0.0, 0.45),
            ),
        ),
    )
    with pytest.raises(TrajectoryError) as raised:
        GargantaTrajectoryAdapter(program)
    assert raised.value.code == "garganta-unsupported-coordinate"


def test_arrow_parser_and_topology_semantics_are_explicit() -> None:
    assert parse_arrow_values("0.1 -> 0.5 -> -0.2") == (0.1, 0.5, -0.2)
    topology = TrajectoryTrack(
        coordinate="body.branch.velum",
        interpolation=Interpolation.STEP,
        contract=CoordinateContract(
            unit="open",
            minimum=0.0,
            maximum=1.0,
            state_semantics=StateSemantics(
                kind="topology_edit", disposition=DisposalPolicy.RADIATE
            ),
        ),
        knots=(TrajectoryKnot(0, 0.0), TrajectoryKnot(100, 1.0)),
    )
    program = TrajectoryProgram(
        schema_version="trajectory-program-v1",
        sample_rate_hz=1_000,
        duration_samples=100,
        control_stride_samples=10,
        tracks=(topology,),
    )
    assert program.compile().frames[-1].sets[0].value == 1.0


def test_committed_examples_are_executable_against_the_real_adapters() -> None:
    string_program = TrajectoryProgram.from_json(
        (ROOT / "examples" / "trajectory" / "string-breathing.json").read_text(
            "utf-8"
        )
    )
    string_lowering = lower_string_trajectory(
        string_program, [110.0, 165.0], 2.0
    )
    assert string_lowering.control_frames
    assert string_lowering.native_gliss_ratio > 1.0

    garganta_program = TrajectoryProgram.from_json(
        (ROOT / "examples" / "trajectory" / "garganta-ba.json").read_text(
            "utf-8"
        )
    )
    anatomy = AnatomyGenome.from_unit_cube([0.5] * 10)
    states, receipt = GargantaTrajectoryAdapter(garganta_program).compile_states(
        anatomy
    )
    assert states
    assert receipt.area_override_indices == (17,)
    assert min(state.area_function_cm2[17] for state in states) == pytest.approx(
        0.025
    )
