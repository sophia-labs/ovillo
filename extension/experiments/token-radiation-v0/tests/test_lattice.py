from __future__ import annotations

import numpy as np
import torch
from torch import nn

from token_radiation_v0.lattice import (
    brute_force_log_likelihood,
    lattice_statistics_from_logits,
    sequence_log_likelihood,
)
from token_radiation_v0.vocabulary import Vocabulary, VocabularyEntry


class TableDetector(nn.Module):
    def __init__(self, table: torch.Tensor):
        super().__init__()
        self.table = nn.Parameter(table)

    def forward(self, emissions: torch.Tensor) -> torch.Tensor:
        del emissions
        return self.table


def test_dynamic_program_matches_brute_force_and_has_gradients() -> None:
    vocabulary = Vocabulary(
        "tiny",
        (
            VocabularyEntry(0, b"a", "byte"),
            VocabularyEntry(1, b"b", "byte"),
            VocabularyEntry(2, b"ab", "macro"),
            VocabularyEntry(3, None, "eos"),
        ),
    )
    logits = torch.tensor(
        [
            [0.1, -0.3, 0.7, -1.0],
            [-0.2, 0.6, -0.4, -1.2],
            [0.0, 0.0, 0.0, 0.8],
        ],
        dtype=torch.float64,
    )
    detector = TableDetector(logits.clone())
    emissions = torch.zeros(3, 2, dtype=torch.float64)
    value = sequence_log_likelihood(emissions, b"ab", vocabulary, detector)
    oracle = brute_force_log_likelihood(logits.numpy(), b"ab", vocabulary)
    assert abs(float(value.detach()) - oracle) < 1e-12
    (-value).backward()
    assert detector.table.grad is not None
    assert torch.isfinite(detector.table.grad).all()
    statistics = lattice_statistics_from_logits(logits.numpy(), b"ab", vocabulary)
    assert abs(statistics.log_likelihood - oracle) < 1e-12
    assert 1.0 <= statistics.expected_tokens <= 2.0
