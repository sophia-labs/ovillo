from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path

import pytest

from extension_v1.trajectory_export import (
    TrajectoryExportError,
    closed_token_to_trajectory,
    program_digest,
)

ARTIFACTS = Path(__file__).resolve().parents[1] / "artifacts"
SOURCE = ARTIFACTS / "closed_gesture_run" / "result.json"
PROGRAM = ARTIFACTS / "token0.trajectory.json"
EXPECTED_DIGEST = "fb6dea90db1295b65b94e2927c72721a0ed1ecc83b96f359ecb2e4faca9f6894"


def load_source() -> dict[str, object]:
    return json.loads(SOURCE.read_text(encoding="utf-8"))


def test_committed_learned_loop_exports_to_the_shared_artifact() -> None:
    program, provenance = closed_token_to_trajectory(load_source())
    committed = json.loads(PROGRAM.read_text(encoding="utf-8"))

    assert program == committed
    assert program_digest(program) == EXPECTED_DIGEST
    assert provenance["program_sha256"] == EXPECTED_DIGEST
    assert provenance["source_loop_closure_residual"] <= 1e-12
    assert provenance["claim_discipline"]["body_adapter_still_required"] is True

    assert [lane["target"] for lane in program["lanes"]] == [
        "extension.f0.u0",
        "extension.f0.u1",
        "extension.f0.u2",
        "extension.f0.u3",
    ]
    for lane in program["lanes"]:
        assert lane["knots"][0]["value"] == 0.0
        assert lane["knots"][-1]["value"] == 0.0
        assert any(knot["value"] != 0.0 for knot in lane["knots"][1:-1])


def test_open_learned_path_is_refused_not_relabelled_closed() -> None:
    result = deepcopy(load_source())
    result["loops"][0][0][-1][0] += 0.01
    with pytest.raises(TrajectoryExportError, match="selected learned path is open"):
        closed_token_to_trajectory(result)


def test_each_token_has_a_distinct_content_addressed_path() -> None:
    result = load_source()
    digests = {
        program_digest(closed_token_to_trajectory(result, token=token)[0])
        for token in range(result["config"]["token_count"])
    }
    assert len(digests) == result["config"]["token_count"]
