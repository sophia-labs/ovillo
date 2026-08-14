#!/usr/bin/env python3
"""Verify committed Generation 3 plans, receipts, and replay artifacts."""

from __future__ import annotations

import json
from pathlib import Path

from extension_generation3.compiler import physical_digest
from extension_generation3.reference import ReferenceMedium, digest_json

ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    artifact = ROOT / "artifacts/construction-seed-20260814"
    plan = json.loads((artifact / "local-medium.plan.json").read_text())
    receipt = json.loads((artifact / "compiler.receipt.json").read_text())
    if digest_json(plan) != receipt["executable_plan_sha256"]:
        raise SystemExit("compiled plan digest does not match its receipt")
    if physical_digest(plan) != receipt["physical_plan_sha256"]:
        raise SystemExit("physical plan digest does not match its receipt")

    fixture = json.loads((ROOT / "fixtures/cross-runtime.json").read_text())
    if fixture["plan_sha256"] != digest_json(fixture["plan"]):
        raise SystemExit("cross-runtime fixture plan digest is invalid")
    medium = ReferenceMedium(fixture["plan"])
    for step in fixture["steps"]:
        if "edge_enabled" in step:
            toggle = step["edge_enabled"]
            medium.set_edge_enabled(int(toggle["edge"]), bool(toggle["enabled"]))
        medium.step(step.get("drives", []))
    expected = fixture["expected"]
    if abs(medium.stored_energy() - expected["stored_energy"]) > 1e-12:
        raise SystemExit("cross-runtime fixture final energy is invalid")
    print("Generation 3 artifacts: valid")


if __name__ == "__main__":
    main()
