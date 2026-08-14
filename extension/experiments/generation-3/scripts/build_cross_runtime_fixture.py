#!/usr/bin/env python3
"""Rebuild the deterministic Python/Rust replay fixture from a compiled plan."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from extension_generation3.reference import ReferenceMedium, digest_json


STEPS = [
    {
        "drives": [
            {"source": 0, "wave": 0.8, "evidence": 0.0, "work_rate": 0.0},
            {"source": 2, "wave": 0.0, "evidence": 0.35, "work_rate": 0.0},
        ]
    },
    {
        "drives": [
            {"source": 1, "wave": 0.8, "evidence": 0.0, "work_rate": 0.0},
            {"source": 3, "wave": 0.0, "evidence": 0.35, "work_rate": 0.0},
        ]
    },
    {
        "edge_enabled": {"edge": 1, "enabled": False},
        "drives": [
            {"source": 2, "wave": 0.0, "evidence": 0.35, "work_rate": 0.0}
        ],
    },
    {
        "edge_enabled": {"edge": 1, "enabled": True},
        "drives": [
            {"source": 3, "wave": 0.0, "evidence": 0.35, "work_rate": 0.0}
        ],
    },
    {
        "drives": [
            {"source": 0, "wave": 0.8, "evidence": 0.0, "work_rate": 0.0},
            {"source": 3, "wave": 0.0, "evidence": 0.35, "work_rate": 0.0},
        ]
    },
]


def build(plan: dict) -> dict:
    medium = ReferenceMedium(plan)
    cumulative_source_work = 0.0
    cumulative_internal_loss = 0.0
    maximum_transport_residual = 0.0
    maximum_positive_mass_residual = 0.0
    maximum_closure_residual = 0.0
    receipts = []
    for step in STEPS:
        if "edge_enabled" in step:
            toggle = step["edge_enabled"]
            medium.set_edge_enabled(int(toggle["edge"]), bool(toggle["enabled"]))
        receipt = medium.step(step.get("drives", []))
        receipts.append(receipt.to_dict())
        cumulative_source_work += receipt.source_work
        cumulative_internal_loss += receipt.internal_loss
        maximum_transport_residual = max(
            maximum_transport_residual, abs(receipt.transport_residual)
        )
        maximum_positive_mass_residual = max(
            maximum_positive_mass_residual, abs(receipt.positive_mass_residual)
        )
        maximum_closure_residual = max(
            maximum_closure_residual, abs(receipt.closure_residual)
        )
    return {
        "format": "sophia.instrument.extension.generation3-cross-runtime/v1",
        "plan": plan,
        "plan_sha256": digest_json(plan),
        "steps": STEPS,
        "expected": {
            "state": medium.state_dict(),
            "stored_energy": medium.stored_energy(),
            "positive_mass": medium.positive_mass(),
            "observation": medium.observe(),
            "cumulative_source_work": cumulative_source_work,
            "cumulative_internal_loss": cumulative_internal_loss,
            "maximum_transport_residual": maximum_transport_residual,
            "maximum_positive_mass_residual": maximum_positive_mass_residual,
            "maximum_closure_residual": maximum_closure_residual,
        },
        "receipts": receipts,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("plan", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    fixture = build(json.loads(args.plan.read_text()))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(fixture, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
