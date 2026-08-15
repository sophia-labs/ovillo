from __future__ import annotations

from token_radiation_v0.process import ProcessConfig, generate_corpus
from token_radiation_v0.vocabulary import (
    byte_vocabulary,
    oracle_motif_vocabulary,
    train_bpe,
    train_predictive_vocabulary,
    train_unigram,
)


def _segmentable(sequence: bytes, vocabulary) -> bool:
    reachable = [False] * (len(sequence) + 1)
    reachable[0] = True
    for start in range(len(sequence)):
        if not reachable[start]:
            continue
        for _, end in vocabulary.matches(sequence, start):
            reachable[end] = True
    return reachable[-1]


def test_all_vocabulary_views_remain_lossless() -> None:
    corpus = generate_corpus(
        ProcessConfig(
            sequence_bytes=24,
            train_sequences=24,
            validation_sequences=8,
            seed=11,
        )
    )
    vocabularies = (
        byte_vocabulary(corpus.alphabet),
        train_bpe(corpus.train, target_size=14),
        train_unigram(corpus.train, target_size=14, iterations=3),
        train_predictive_vocabulary(
            corpus.train, target_size=14, minimum_count=2
        ),
        oracle_motif_vocabulary(corpus.alphabet),
    )
    for vocabulary in vocabularies:
        assert all(_segmentable(sequence, vocabulary) for sequence in corpus.validation)
        assert vocabulary.entries[vocabulary.eos_id].surface is None
