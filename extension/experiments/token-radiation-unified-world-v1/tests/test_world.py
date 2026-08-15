import torch

from token_radiation_unified_world_v1.data import generate_batch
from token_radiation_unified_world_v1.world import UnifiedLexicalWorld, training_loss


def test_world_closes_local_ledger_and_backpropagates_through_field():
    batch = generate_batch(seed=7, batch_size=6, events=3)
    model = UnifiedLexicalWorld(mode="shared", sites=8, courses=2, settle_steps=1, seed=8)
    trace = model(batch)
    assert trace.current_logits.shape == (6, 3, 4)
    assert trace.max_closure_residual.max().item() < 2e-5
    loss = training_loss(trace, batch)
    loss.backward()
    assert model.lexical.source.grad is not None
    assert model.lexical.source.grad.abs().sum().item() > 0.0
    assert model.field.raw_coupling.grad is not None


def test_shared_port_ablation_changes_predictive_state():
    batch = generate_batch(seed=11, batch_size=8, events=4)
    model = UnifiedLexicalWorld(mode="shared", sites=8, courses=2, settle_steps=1, seed=12)
    with torch.no_grad():
        normal = model(batch).predictive_states
        disabled = model(batch, lexical_port_scale=0.0).predictive_states
    assert not torch.allclose(normal, disabled)


def test_analysis_heads_are_not_in_the_world_state_path():
    batch = generate_batch(seed=15, batch_size=8, events=4)
    model = UnifiedLexicalWorld(mode="shared", sites=8, courses=2, settle_steps=1, seed=16)
    with torch.no_grad():
        before = model(batch).predictive_states
        for head in (model.current_head, model.next_head, model.mode_head, model.byte_head):
            head.weight.zero_()
            head.bias.zero_()
        after = model(batch).predictive_states
    torch.testing.assert_close(before, after, rtol=0.0, atol=0.0)


def test_isolated_predictive_body_cannot_receive_surface_sequence():
    first = generate_batch(seed=21, batch_size=8, events=4)
    second = generate_batch(seed=22, batch_size=8, events=4)
    model = UnifiedLexicalWorld(mode="isolated", sites=8, courses=2, settle_steps=1, seed=23)
    with torch.no_grad():
        left = model(first).predictive_states
        right = model(second).predictive_states
    torch.testing.assert_close(left, right, rtol=0.0, atol=0.0)
