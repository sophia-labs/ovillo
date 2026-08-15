from __future__ import annotations

import math

import torch
from torch import nn

from token_radiation_paired_operator_v1.class_codec import (
    EventEntry,
    EventVariant,
    EventVocabulary,
    event_lattice_statistics_from_logits,
    sequence_event_log_likelihood,
)


def test_event_class_lattice_receipts_surface_residual_exactly() -> None:
    vocabulary = EventVocabulary(
        name="tiny-event-classes",
        alphabet=(ord("a"), ord("b")),
        entries=(
            EventEntry("a", (EventVariant(b"a", 0.0),), {"kind": "byte"}),
            EventEntry("b", (EventVariant(b"b", 0.0),), {"kind": "byte"}),
            EventEntry(
                "macro-class",
                (
                    EventVariant(b"ab", math.log(0.75)),
                    EventVariant(b"ba", math.log(0.25)),
                ),
                {"kind": "operator_class"},
            ),
            EventEntry("eos", (), {"kind": "eos"}),
        ),
    )
    vocabulary.validate()
    logits = torch.tensor(
        [
            [0.2, -0.1, 0.7, -0.4],
            [0.3, 0.5, -0.2, -0.8],
            [-0.2, -0.3, -0.4, 0.9],
        ],
        dtype=torch.float64,
    )
    exact = sequence_event_log_likelihood(logits, b"ab", vocabulary, nn.Identity())
    log_probs = torch.log_softmax(logits, dim=-1)
    explicit = torch.logaddexp(
        log_probs[0, 2] + math.log(0.75) + log_probs[2, 3],
        log_probs[0, 0] + log_probs[1, 1] + log_probs[2, 3],
    )
    assert torch.allclose(exact, explicit, atol=1.0e-14, rtol=0.0)
    statistics = event_lattice_statistics_from_logits(
        logits.detach().numpy(), b"ab", vocabulary
    )
    assert math.isclose(statistics.log_likelihood, float(explicit), abs_tol=1.0e-14)
    assert statistics.expected_residual_nats > 0.0
    assert math.isclose(statistics.boundary_probabilities[-1], 1.0)
