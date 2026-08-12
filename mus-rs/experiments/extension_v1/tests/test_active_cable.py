import torch

from extension_v1.active_cable import ActiveCableConfig, ActiveCableMedium, dataset


def test_active_cable_forward_is_finite_and_shape_correct() -> None:
    config = ActiveCableConfig(
        string_count=3,
        nodes_per_string=6,
        channels=3,
        reaction_rank=4,
        microsteps_per_token=2,
        read_paths=1,
        read_steps=3,
        contact_neighbors=2,
        train_steps=2,
    )
    torch.manual_seed(config.seed)
    model = ActiveCableMedium(config)
    tokens, _ = dataset(config.vocab_size)
    logits, audit = model(tokens[:4], audit=True)
    assert logits.shape == (4, 2)
    assert torch.isfinite(logits).all()
    assert torch.isfinite(audit["final_energy"]).all()
    assert model.geometry().shape == (
        config.string_count,
        config.nodes_per_string,
        config.ambient_dim,
    )
