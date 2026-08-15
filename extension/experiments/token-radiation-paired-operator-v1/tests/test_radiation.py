from __future__ import annotations

import torch

from token_radiation_paired_operator_v1.radiation import metric_radiate, orthonormal_rows


def test_metric_radiation_closes_and_nonloading_preserves_state() -> None:
    state = torch.tensor([[0.8, -0.4, 1.2]], dtype=torch.float64)
    metric = torch.tensor([0.75, 1.5, 2.25], dtype=torch.float64)
    rows = orthonormal_rows(
        torch.tensor([[1.0, 0.2, -0.1], [0.1, 0.8, 0.3]], dtype=torch.float64)
    )
    loaded = metric_radiate(state, metric, rows, 0.37, loading=True)
    observed = metric_radiate(state, metric, rows, 0.37, loading=False)
    assert loaded.booked_radiation.item() > 0.0
    assert loaded.residual.abs().max().item() < 1.0e-12
    assert torch.equal(observed.state_after, state)
    assert observed.booked_radiation.item() == 0.0
    assert observed.residual.item() == 0.0
    assert torch.allclose(loaded.coordinates, observed.coordinates, atol=0.0, rtol=0.0)


def test_zero_fraction_is_exact_identity_and_coordinates_are_calibrated() -> None:
    state = torch.tensor([[0.8, -0.4, 1.2]], dtype=torch.float64)
    metric = torch.tensor([0.75, 1.5, 2.25], dtype=torch.float64)
    rows = orthonormal_rows(
        torch.tensor([[1.0, 0.2, -0.1], [0.1, 0.8, 0.3]], dtype=torch.float64)
    )
    zero = metric_radiate(state, metric, rows, 0.0, loading=True)
    low = metric_radiate(state, metric, rows, 0.04, loading=False)
    high = metric_radiate(state, metric, rows, 0.50, loading=False)
    assert torch.equal(zero.state_after, state)
    assert zero.booked_radiation.item() == 0.0
    assert torch.equal(low.coordinates, high.coordinates)
    assert torch.allclose(low.emitted / (0.04**0.5), low.coordinates)
    assert torch.allclose(high.emitted / (0.50**0.5), high.coordinates)
