from __future__ import annotations

import copy
import math

from extension_generation3.reference import ReferenceMedium, digest_json


def plan() -> dict:
    return {
        "schema_version": "ariadne-local-medium-v1",
        "name": "reference-test",
        "sample_rate_hz": 32.0,
        "sites": [
            {
                "id": f"site-{index}",
                "wave_metric": 1.0 + 0.25 * index,
                "wave_decay_per_second": 0.01,
                "evidence_decay_per_second": 0.02,
                "reservoir_capacity": 1.0,
                "reservoir_leak_per_second": 0.01,
                "active_rate_per_second": 0.2,
                "active_threshold": 0.1,
                "active_target_wave_energy": 0.5,
                "initial_wave": 0.0,
                "initial_evidence": 0.0,
                "initial_reservoir": 0.0,
            }
            for index in range(3)
        ],
        "edges": [
            {
                "id": "edge-0",
                "left": 0,
                "right": 1,
                "wave_rate_radians_per_second": 4.0,
                "evidence_rate_per_second": 3.0,
                "enabled": True,
            },
            {
                "id": "edge-1",
                "left": 1,
                "right": 2,
                "wave_rate_radians_per_second": -2.5,
                "evidence_rate_per_second": 1.5,
                "enabled": True,
            },
        ],
        "sources": [
            {
                "id": "source-0",
                "site": 0,
                "wave_gain": 1.0,
                "evidence_gain": 1.0,
                "work_gain": 1.0,
            }
        ],
        "observers": [
            {
                "id": "observer-0",
                "wave_weights": [1.0, -0.5, 0.25],
                "evidence_weights": [0.0, 1.0, -1.0],
                "reservoir_weights": [0.0, 0.0, 1.0],
                "bias": 0.0,
            }
        ],
    }


def test_zero_is_exact_and_digest_is_stable() -> None:
    first = ReferenceMedium(plan())
    second = ReferenceMedium(copy.deepcopy(plan()))
    assert first.digest == second.digest == digest_json(first.plan)
    for _ in range(16):
        receipt = first.step([])
        assert receipt.source_work == 0.0
        assert receipt.internal_loss == 0.0
        assert receipt.closure_residual == 0.0
    assert first.state_dict() == {
        "wave": [0.0, 0.0, 0.0],
        "evidence": [0.0, 0.0, 0.0],
        "reservoir": [0.0, 0.0, 0.0],
    }


def test_ledger_closes_and_positive_transport_preserves_mass() -> None:
    medium = ReferenceMedium(plan())
    cumulative_residual = 0.0
    for step in range(128):
        drives = [
            {
                "source": 0,
                "wave": math.sin(0.1 * step) * 0.02,
                "evidence": 0.01 if step % 7 == 0 else 0.0,
                "work_rate": 0.1,
            }
        ]
        receipt = medium.step(drives)
        cumulative_residual += abs(receipt.closure_residual)
        assert receipt.transport_residual < 1e-12
        assert receipt.positive_mass_residual < 1e-12
        assert min(medium.evidence) >= 0.0
    assert cumulative_residual < 1e-10


def test_damage_preserves_live_state_and_reconfiguration_books_work() -> None:
    medium = ReferenceMedium(plan())
    medium.step([{"source": 0, "wave": 0.4, "evidence": 0.2, "work_rate": 0.5}])
    state_before = medium.state_dict()
    damage = medium.set_edge_enabled(0, False)
    assert medium.state_dict() == state_before
    assert damage["state_energy_delta"] == 0.0

    replacement = copy.deepcopy(medium.plan["sites"][0])
    replacement["wave_metric"] *= 1.5
    receipt = medium.reconfigure_site(0, replacement)
    assert abs(receipt["closure_residual"]) < 1e-14
    assert receipt["configuration_work"] != 0.0


def test_committed_cross_runtime_fixture_replays() -> None:
    import json
    from pathlib import Path

    fixture = json.loads((Path(__file__).parents[1] / "fixtures/cross-runtime.json").read_text())
    medium = ReferenceMedium(fixture["plan"])
    source_work = 0.0
    internal_loss = 0.0
    for step in fixture["steps"]:
        if "edge_enabled" in step:
            medium.set_edge_enabled(
                int(step["edge_enabled"]["edge"]), bool(step["edge_enabled"]["enabled"])
            )
        receipt = medium.step(step.get("drives", []))
        source_work += receipt.source_work
        internal_loss += receipt.internal_loss
    expected = fixture["expected"]
    for name, actual in medium.state_dict().items():
        for left, right in zip(actual, expected["state"][name], strict=True):
            assert abs(left - right) < 1e-12
    assert abs(medium.stored_energy() - expected["stored_energy"]) < 1e-12
    assert abs(source_work - expected["cumulative_source_work"]) < 1e-12
    assert abs(internal_loss - expected["cumulative_internal_loss"]) < 1e-12
