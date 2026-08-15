#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path

import numpy as np

from token_radiation_v0.radiation import RadiationPlan, metric_radiate_numpy


def main() -> None:
    path = (
        Path(__file__).resolve().parents[1]
        / "fixtures"
        / "radiation-cross-runtime.json"
    )
    fixture = json.loads(path.read_text())
    plan_value = fixture["plan"]
    plan = RadiationPlan(
        id=plan_value["id"],
        fraction=float(plan_value["fraction"]),
        rows=tuple(
            tuple(float(value) for value in row) for row in plan_value["rows"]
        ),
        schema_version=plan_value["schema_version"],
    )
    if plan.sha256 != fixture["plan_sha256"]:
        raise SystemExit("radiation plan digest mismatch")
    state = np.asarray(fixture["state"], dtype=np.float64)
    metric = np.asarray(fixture["metric"], dtype=np.float64)
    rows = np.asarray(plan.rows, dtype=np.float64)
    for loading, key in ((True, "loading"), (False, "nonloading")):
        frame = metric_radiate_numpy(
            state, metric, rows, plan.fraction, loading=loading
        )
        expected = fixture[key]
        if not np.allclose(
            frame.state_after, expected["state_after"], atol=1e-14, rtol=1e-14
        ):
            raise SystemExit(f"{key} state mismatch")
        if not np.allclose(
            frame.emitted, expected["emitted"], atol=1e-14, rtol=1e-14
        ):
            raise SystemExit(f"{key} emission mismatch")
        for field in (
            "storage_before",
            "storage_after",
            "projected_intensity",
            "booked_radiation",
            "residual",
        ):
            if abs(float(getattr(frame, field)) - float(expected[field])) > 1e-14:
                raise SystemExit(f"{key} {field} mismatch")
    print("radiation fixture valid")


if __name__ == "__main__":
    main()
