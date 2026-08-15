import torch
from token_radiation_unified_world_v1.data import F, R, generate_batch, homograph_mask

def test_homograph_has_two_meanings():
    batch = generate_batch(seed=4, batch_size=128, events=12)
    mask = homograph_mask(batch)
    values = set(batch.operators[mask].tolist())
    assert R in values and F in values
    assert batch.event_byte_ids.shape[:2] == batch.operators.shape
    assert batch.byte_mask.dtype == torch.bool
