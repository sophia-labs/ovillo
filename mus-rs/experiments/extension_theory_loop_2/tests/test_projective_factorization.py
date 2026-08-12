from __future__ import annotations

import numpy as np
import torch

from extension_theory_loop2.core import ObservableHMM, PositiveProjectiveModel, ProcessSpec
from extension_theory_loop2.projective_init import empirical_hankel, initialize_from_factorization, positive_hankel_factorization


def test_observable_positive_factorization_is_positive_and_executable() -> None:
    process = ObservableHMM(ProcessSpec("factor", states=4, symbols=3, persistence=0.91, emission_concentration=0.8, anchor=2.0, seed=22))
    _, hankel, shifted, _ = empirical_hankel(process, stream_length=12_000, word_depth=1, seed=101)
    factor = positive_hankel_factorization(hankel, shifted, rank=4, seed=102, max_iter=600)
    assert factor["operators"].shape == (3, 4, 4)
    assert factor["initial"].shape == (4,)
    assert factor["terminal"].shape == (4,)
    assert np.all(factor["operators"] >= 0)
    assert np.all(np.isfinite(factor["operators"]))

    model = PositiveProjectiveModel(symbols=3, rank=4)
    initialize_from_factorization(model, factor)
    tokens = process.sample(8, 7, torch.Generator().manual_seed(103))
    logits, states = model.forward_with_states(tokens)
    assert logits.shape == (8, 7, 3)
    assert states.shape == (8, 7, 4)
    assert torch.isfinite(logits).all()
    assert torch.isfinite(states).all()
    assert torch.all(states > 0)
