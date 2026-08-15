from __future__ import annotations

import math

import torch
from torch import nn

from token_radiation_paired_operator_v1.lattice import (
    lattice_statistics_from_logits,
    sequence_log_likelihood,
)
from token_radiation_paired_operator_v1.process import ProcessConfig, generate_corpus
from token_radiation_paired_operator_v1.vocabulary import (
    Vocabulary,
    VocabularyEntry,
    train_bpe,
)


def test_lattice_matches_explicit_segmentation_sum() -> None:
    vocabulary = Vocabulary(
        name="tiny",
        alphabet=(ord("a"), ord("b")),
        entries=(
            VocabularyEntry("a", b"a"),
            VocabularyEntry("b", b"b"),
            VocabularyEntry("ab", b"ab"),
            VocabularyEntry("eos", None),
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
    detector = nn.Identity()
    exact = sequence_log_likelihood(logits, b"ab", vocabulary, detector)
    log_probs = torch.log_softmax(logits, dim=-1)
    explicit = torch.logaddexp(
        log_probs[0, 2] + log_probs[2, 3],
        log_probs[0, 0] + log_probs[1, 1] + log_probs[2, 3],
    )
    assert torch.allclose(exact, explicit, atol=1.0e-14, rtol=0.0)
    statistics = lattice_statistics_from_logits(logits.numpy(), b"ab", vocabulary)
    assert math.isclose(statistics.log_likelihood, float(explicit), abs_tol=1.0e-14)
    assert 1.0 <= statistics.expected_tokens <= 2.0
    assert math.isclose(statistics.boundary_probabilities[-1], 1.0)


def test_bpe_preserves_byte_cover_and_target_budget() -> None:
    corpus = generate_corpus(
        ProcessConfig(
            sequence_bytes=12,
            train_sequences=8,
            validation_sequences=3,
            seed=23,
            family="structured",
        )
    )
    vocabulary = train_bpe(corpus, target_size=10)
    vocabulary.validate()
    surfaces = {entry.surface for entry in vocabulary.entries}
    for byte in corpus.alphabet:
        assert bytes((byte,)) in surfaces
    assert vocabulary.size <= 10
