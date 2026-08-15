import torch

from token_radiation_lexical_interface_v1.body import BodyConfig, LocalRadiatingBody


def test_loading_radiation_closes_metric_storage() -> None:
    torch.manual_seed(1)
    body = LocalRadiatingBody(
        BodyConfig(
            source_count=5,
            sites=6,
            radiation_rank=6,
            radiation_fraction=0.25,
            fixed_full_aperture=True,
        )
    )
    state = torch.randn(4, 6)
    frame = body.step_ids(state, torch.tensor([0, 1, 2, 3]), loading=True)
    assert float(frame.residual.detach().abs().max()) < 2.0e-6
    assert torch.allclose(
        frame.storage_before - frame.storage_after,
        frame.booked_radiation,
        atol=2.0e-6,
        rtol=2.0e-6,
    )


def test_nonloading_observation_preserves_post_transport_storage() -> None:
    torch.manual_seed(2)
    body = LocalRadiatingBody(
        BodyConfig(
            source_count=2,
            sites=4,
            radiation_rank=4,
            radiation_fraction=0.5,
            fixed_full_aperture=True,
        )
    )
    state = torch.randn(2, 4)
    params = body.constrained()
    nonloading = body.step_ids(
        state, torch.tensor([0, 1]), loading=False, constrained=params
    )
    loading = body.step_ids(
        state, torch.tensor([0, 1]), loading=True, constrained=params
    )
    assert torch.equal(nonloading.booked_radiation, torch.zeros(2))
    assert torch.all(nonloading.storage_after == nonloading.storage_before)
    assert torch.all(loading.storage_after < loading.storage_before)
