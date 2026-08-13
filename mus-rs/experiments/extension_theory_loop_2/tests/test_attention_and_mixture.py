from __future__ import annotations

import torch

from extension_theory_loop2.attention_phase import RoutedMHA, retrieval_batch
from extension_theory_loop2.core import ObservableHMM, PositiveProjectiveModel, ProcessSpec
from extension_theory_loop2.transport_mixture import FixedCliffordPath, MINUS_LOOP, PLUS_LOOP, TypedTransportBank


def test_attention_shapes_and_invariant_components() -> None:
    tokens, target, positions = retrieval_batch(batch=12, keys=5, routes=3, generator=torch.Generator().manual_seed(3))
    model = RoutedMHA(input_dim=tokens.shape[-1], outputs=3, heads=4, query_dim=6, value_dim=3, collision_noise=1e-3, output_multiplier=1.0)
    output, attention, parts = model(tokens, return_parts=True)
    b, c = model.invariant_components()
    assert output.shape == target.shape == (12, 3)
    assert attention.shape == (12, 4, 5)
    assert parts.shape == (12, 4, 3)
    assert positions.shape == (12, 3)
    assert b.shape[0] == c.shape[0] == 4
    assert torch.isfinite(output).all()


def test_commuting_closed_clifford_loops_collapse() -> None:
    model = FixedCliffordPath(commuting=True)
    moves = torch.stack([PLUS_LOOP, MINUS_LOOP], dim=0)
    state = model(moves)
    vacuum = torch.zeros_like(state)
    vacuum[:, 0] = 1.0
    assert torch.max(torch.abs(state - vacuum)) < 2e-6


def test_typed_transport_bank_forward_is_finite() -> None:
    process = ObservableHMM(ProcessSpec("mix", states=4, symbols=4, persistence=0.9, seed=8))
    projective = PositiveProjectiveModel(symbols=4, rank=4)
    bank = TypedTransportBank(projective, commuting=False, mode="full")
    observations = process.sample(10, 12, torch.Generator().manual_seed(7))
    moves = PLUS_LOOP.repeat(10, 3)
    obs_logits, area_logits, info = bank(observations, moves)
    assert obs_logits.shape == (10, 4)
    assert area_logits.shape == (10, 2)
    assert torch.isfinite(obs_logits).all()
    assert torch.isfinite(area_logits).all()
    assert torch.allclose(info["obs_weight"].sum(), torch.tensor(1.0))
    assert torch.allclose(info["area_weight"].sum(), torch.tensor(1.0))
