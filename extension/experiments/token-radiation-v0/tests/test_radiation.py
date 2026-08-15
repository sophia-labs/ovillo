from __future__ import annotations

import numpy as np
import torch

from token_radiation_v0.detectors import PositiveFrameDetector
from token_radiation_v0.radiation import (
    RadiationPlan,
    metric_radiate_numpy,
    metric_radiate_torch,
    orthonormal_rows,
)


def test_rank_k_passive_radiation_closes_and_matches_numpy() -> None:
    generator = torch.Generator().manual_seed(7)
    state = torch.randn(5, 7, generator=generator, dtype=torch.float64)
    metric = torch.linspace(0.5, 2.0, 7, dtype=torch.float64)
    rows = orthonormal_rows(
        torch.randn(3, 7, generator=generator, dtype=torch.float64)
    )
    frame = metric_radiate_torch(state, metric, rows, 0.37, loading=True)
    assert float(frame.residual.abs().max()) < 2e-14
    assert torch.allclose(
        frame.storage_before - frame.storage_after,
        frame.booked_radiation,
        atol=2e-14,
        rtol=2e-14,
    )
    oracle = metric_radiate_numpy(
        state[0].numpy(), metric.numpy(), rows.detach().numpy(), 0.37, loading=True
    )
    assert np.allclose(
        frame.emitted[0].detach().numpy(), oracle.emitted, atol=1e-14
    )
    assert np.allclose(
        frame.state_after[0].detach().numpy(), oracle.state_after, atol=1e-14
    )


def test_nonloading_is_observation_not_radiation() -> None:
    state = torch.tensor([[1.0, -0.5, 0.25]], dtype=torch.float64)
    metric = torch.tensor([1.0, 2.0, 3.0], dtype=torch.float64)
    rows = torch.tensor([[1.0, 0.0, 0.0]], dtype=torch.float64)
    frame = metric_radiate_torch(state, metric, rows, 0.5, loading=False)
    assert torch.equal(frame.state_after, state)
    assert float(frame.booked_radiation) == 0.0
    assert float(frame.projected_intensity) > 0.0
    assert float(frame.residual) == 0.0


def test_radiation_plan_digest_and_validation() -> None:
    plan = RadiationPlan("test", 0.25, ((1.0, 0.0), (0.0, 1.0)))
    plan.validate()
    assert len(plan.sha256) == 64


def test_positive_frame_contextual_intensity_closes() -> None:
    detector = PositiveFrameDetector(4, 9)
    radiation = torch.randn(11, 4, dtype=torch.float64)
    assert float(detector.frame_residual(radiation).detach().abs().max()) < 1e-12
