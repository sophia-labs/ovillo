from __future__ import annotations

import numpy as np

from extension_theory_loop2.compartmental import CubeCompartmentProcess, ledger_run


def test_cube_local_step_is_positive_stochastic_and_edge_local() -> None:
    process = CubeCompartmentProcess()
    step = process.local_step(8)
    assert np.min(step) >= -1e-14
    assert np.max(np.abs(step.sum(axis=1) - 1.0)) < 1e-12
    edge_pairs = {tuple(sorted((i, j))) for i, j, _, _ in process.edges}
    for i in range(8):
        for j in range(8):
            if i == j or abs(step[i, j]) < 1e-14:
                continue
            assert tuple(sorted((i, j))) in edge_pairs


def test_cube_storage_ledger_closes() -> None:
    process = CubeCompartmentProcess()
    ledger = ledger_run(process, microsteps=16, seed=19)
    assert ledger["minimum_state"] >= 0
    assert ledger["maximum_mass_residual"] < 1e-12
    assert ledger["maximum_storage_ledger_residual"] < 1e-12
    assert ledger["mean_diffusive_dissipation"] > 0
    assert ledger["mean_entropy_diffusion_change"] > 0
