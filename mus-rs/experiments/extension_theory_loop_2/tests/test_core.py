from __future__ import annotations

import numpy as np
import torch

from extension_theory_loop2.core import ExactPositiveRealization, ObservableHMM, PositiveProjectiveModel, ProcessSpec, clifford_generators, filter_lyapunov_spectrum


def tiny_process(states: int = 4) -> ObservableHMM:
    return ObservableHMM(ProcessSpec("test", states=states, symbols=4, persistence=0.9, emission_concentration=0.8, anchor=1.8, seed=13))


def test_clifford_generators_satisfy_relations() -> None:
    generators = clifford_generators(3, dtype=torch.float64)
    identity = torch.eye(8, dtype=torch.float64)
    for i in range(3):
        assert torch.allclose(generators[i].T, -generators[i], atol=1e-12)
        assert torch.allclose(generators[i] @ generators[i], -identity, atol=1e-12)
        for j in range(i + 1, 3):
            anti = generators[i] @ generators[j] + generators[j] @ generators[i]
            assert torch.max(torch.abs(anti)) < 1e-12


def test_exact_positive_realization_matches_bayesian_oracle() -> None:
    process = tiny_process(states=4)
    model = ExactPositiveRealization(process)
    tokens = process.sample(32, 15, torch.Generator().manual_seed(9))
    model_logits = model(tokens)
    oracle_logits = process.oracle_logits(process.beliefs(tokens)).float()
    assert torch.max(torch.abs(model_logits - oracle_logits)) < 2e-6


def test_projective_global_operator_scale_is_gauge() -> None:
    torch.manual_seed(5)
    model = PositiveProjectiveModel(symbols=4, rank=5)
    state = model.initial_state(7)
    symbols = torch.tensor([0, 1, 2, 3, 0, 2, 1])
    operators = model.operators().detach()
    next_a, logits_a = model.step(state, symbols, operators)
    next_b, logits_b = model.step(state, symbols, 17.0 * operators)
    assert torch.allclose(next_a, next_b, atol=2e-6, rtol=2e-6)
    assert torch.allclose(logits_a, logits_b, atol=2e-6, rtol=2e-6)


def test_filter_lyapunov_spectrum_is_finite_and_contracting() -> None:
    process = tiny_process(states=4)
    result = filter_lyapunov_spectrum(process, steps=400, burn_in=50, seed=7)
    retentions = np.asarray(result["retentions"])
    assert retentions.shape == (3,)
    assert np.all(np.isfinite(retentions))
    assert np.all(retentions > 0)
    assert np.max(retentions) < 1.05
