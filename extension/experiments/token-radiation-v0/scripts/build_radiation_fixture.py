#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path

import numpy as np

from token_radiation_v0.radiation import RadiationPlan, metric_radiate_numpy


def main() -> None:
    rows = np.asarray(
        [
            [1.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, 0.6, 0.8, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0, 0.0],
        ],
        dtype=np.float64,
    )
    plan = RadiationPlan(
        id="token-radiation-cross-runtime",
        fraction=0.37,
        rows=tuple(tuple(float(value) for value in row) for row in rows),
    )
    state = np.asarray([0.7, -0.4, 0.9, 0.25, -1.1], dtype=np.float64)
    metric = np.asarray([0.75, 1.25, 2.0, 1.5, 3.0], dtype=np.float64)
    loading = metric_radiate_numpy(
        state, metric, rows, plan.fraction, loading=True
    )
    observed = metric_radiate_numpy(
        state, metric, rows, plan.fraction, loading=False
    )
    payload = {
        "format": "sophia.instrument.ariadne.metric-radiation-fixture/v1",
        "plan": plan.to_dict(),
        "plan_sha256": plan.sha256,
        "state": state.tolist(),
        "metric": metric.tolist(),
        "loading": {
            "state_after": loading.state_after.tolist(),
            "emitted": loading.emitted.tolist(),
            "storage_before": loading.storage_before,
            "storage_after": loading.storage_after,
            "projected_intensity": loading.projected_intensity,
            "booked_radiation": loading.booked_radiation,
            "residual": loading.residual,
        },
        "nonloading": {
            "state_after": observed.state_after.tolist(),
            "emitted": observed.emitted.tolist(),
            "storage_before": observed.storage_before,
            "storage_after": observed.storage_after,
            "projected_intensity": observed.projected_intensity,
            "booked_radiation": observed.booked_radiation,
            "residual": observed.residual,
        },
    }
    output = (
        Path(__file__).resolve().parents[1]
        / "fixtures"
        / "radiation-cross-runtime.json"
    )
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    print(output)


if __name__ == "__main__":
    main()
