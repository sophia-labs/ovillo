"""Exact segmentation marginalization and linear detector fitting."""

from __future__ import annotations

from dataclasses import dataclass
import math
import random

import numpy as np
import torch
from torch import nn
import torch.nn.functional as F

from .model import BytePredictionModel
from .paired import encode_corpus
from .process import Corpus, Example
from .vocabulary import Vocabulary


@dataclass(frozen=True, slots=True)
class LatticeStatistics:
    log_likelihood: float
    expected_tokens: float
    token_expected_counts: tuple[float, ...]
    boundary_probabilities: tuple[float, ...]


def _matches(sequence: bytes, position: int, surface: bytes | None) -> bool:
    return surface is not None and sequence.startswith(surface, position)


def sequence_log_likelihood(
    features: torch.Tensor,
    sequence: bytes,
    vocabulary: Vocabulary,
    detector: nn.Module,
) -> torch.Tensor:
    logits = detector(features)
    log_probs = F.log_softmax(logits, dim=-1)
    length = len(sequence)
    negative_infinity = logits.new_tensor(float("-inf"))
    alpha = [negative_infinity for _ in range(length + 1)]
    alpha[0] = logits.new_zeros(())
    for position in range(length):
        if not bool(torch.isfinite(alpha[position])):
            continue
        for token_index, entry in enumerate(vocabulary.entries):
            if _matches(sequence, position, entry.surface):
                end = position + len(entry.surface or b"")
                value = alpha[position] + log_probs[position, token_index]
                alpha[end] = torch.logaddexp(alpha[end], value)
    return alpha[length] + log_probs[length, vocabulary.eos_index]


def lattice_statistics_from_logits(
    logits: np.ndarray, sequence: bytes, vocabulary: Vocabulary
) -> LatticeStatistics:
    logits = np.asarray(logits, dtype=np.float64)
    logits = logits - logits.max(axis=-1, keepdims=True)
    log_probs = logits - np.log(np.exp(logits).sum(axis=-1, keepdims=True))
    length = len(sequence)
    alpha = np.full(length + 1, -np.inf, dtype=np.float64)
    alpha[0] = 0.0
    for position in range(length):
        for token_index, entry in enumerate(vocabulary.entries):
            if _matches(sequence, position, entry.surface):
                end = position + len(entry.surface or b"")
                alpha[end] = np.logaddexp(
                    alpha[end], alpha[position] + log_probs[position, token_index]
                )
    total = alpha[length] + log_probs[length, vocabulary.eos_index]
    beta = np.full(length + 1, -np.inf, dtype=np.float64)
    beta[length] = log_probs[length, vocabulary.eos_index]
    for position in range(length - 1, -1, -1):
        values: list[float] = []
        for token_index, entry in enumerate(vocabulary.entries):
            if _matches(sequence, position, entry.surface):
                end = position + len(entry.surface or b"")
                values.append(log_probs[position, token_index] + beta[end])
        if values:
            beta[position] = np.logaddexp.reduce(values)
    expected_counts = np.zeros(vocabulary.size, dtype=np.float64)
    boundaries = np.zeros(length + 1, dtype=np.float64)
    expected_tokens = 0.0
    for position in range(length):
        for token_index, entry in enumerate(vocabulary.entries):
            if _matches(sequence, position, entry.surface):
                end = position + len(entry.surface or b"")
                posterior = math.exp(
                    alpha[position]
                    + log_probs[position, token_index]
                    + beta[end]
                    - total
                )
                expected_counts[token_index] += posterior
                expected_tokens += posterior
                boundaries[end] += posterior
    expected_counts[vocabulary.eos_index] = 1.0
    boundaries[0] = 1.0
    boundaries[length] = 1.0
    return LatticeStatistics(
        log_likelihood=float(total),
        expected_tokens=float(expected_tokens),
        token_expected_counts=tuple(float(value) for value in expected_counts),
        boundary_probabilities=tuple(float(value) for value in boundaries),
    )


def extract_boundary_features(
    model: BytePredictionModel, corpus: Corpus, *, loading: bool
) -> tuple[tuple[torch.Tensor, ...], tuple[torch.Tensor, ...]]:
    model.eval()
    train_ids = encode_corpus(corpus)
    validation_ids = encode_corpus(corpus, validation=True)
    with torch.no_grad():
        train = model.body(train_ids, loading=loading).coordinates.detach()
        validation = model.body(validation_ids, loading=loading).coordinates.detach()
    return tuple(row for row in train), tuple(row for row in validation)


def _soft_boundary_f1(
    statistics: LatticeStatistics, example: Example
) -> tuple[float, float, float]:
    predicted = np.asarray(statistics.boundary_probabilities, dtype=np.float64)
    internal = np.zeros_like(predicted)
    for boundary in example.motif_boundaries[1:-1]:
        internal[boundary] = 1.0
    predicted[0] = 0.0
    predicted[-1] = 0.0
    true_positive = float((predicted * internal).sum())
    precision = true_positive / max(float(predicted.sum()), 1.0e-12)
    recall = true_positive / max(float(internal.sum()), 1.0e-12)
    f1 = 2.0 * precision * recall / max(precision + recall, 1.0e-12)
    return precision, recall, f1


def fit_detector(
    vocabulary: Vocabulary,
    train_features: tuple[torch.Tensor, ...],
    validation_features: tuple[torch.Tensor, ...],
    corpus: Corpus,
    *,
    seed: int,
    steps: int,
    batch_size: int,
) -> dict[str, object]:
    torch.manual_seed(seed)
    random_source = random.Random(seed + 1)
    detector = nn.Linear(
        train_features[0].shape[-1], vocabulary.size, dtype=torch.float64
    )
    optimizer = torch.optim.Adam(detector.parameters(), lr=0.04)
    for _ in range(steps):
        indices = [
            random_source.randrange(len(corpus.train)) for _ in range(batch_size)
        ]
        optimizer.zero_grad(set_to_none=True)
        loss = train_features[0].new_zeros(())
        bytes_seen = 0
        for index in indices:
            loss = loss - sequence_log_likelihood(
                train_features[index],
                corpus.train[index].surface,
                vocabulary,
                detector,
            )
            bytes_seen += len(corpus.train[index].surface)
        loss = loss / max(bytes_seen, 1)
        loss.backward()
        torch.nn.utils.clip_grad_norm_(detector.parameters(), 5.0)
        optimizer.step()

    detector.eval()
    total_log_likelihood = 0.0
    total_bytes = 0
    total_tokens = 0.0
    expected_counts = np.zeros(vocabulary.size, dtype=np.float64)
    precisions: list[float] = []
    recalls: list[float] = []
    f1s: list[float] = []
    with torch.no_grad():
        for features, example in zip(
            validation_features, corpus.validation, strict=True
        ):
            logits = detector(features).cpu().numpy()
            statistics = lattice_statistics_from_logits(
                logits, example.surface, vocabulary
            )
            total_log_likelihood += statistics.log_likelihood
            total_bytes += len(example.surface)
            total_tokens += statistics.expected_tokens
            expected_counts += np.asarray(statistics.token_expected_counts)
            precision, recall, f1 = _soft_boundary_f1(statistics, example)
            precisions.append(precision)
            recalls.append(recall)
            f1s.append(f1)
    bits_per_byte = -total_log_likelihood / (
        max(total_bytes, 1) * math.log(2.0)
    )
    events_per_byte = total_tokens / max(total_bytes, 1)
    return {
        "vocabulary": vocabulary.name,
        "vocabulary_size": vocabulary.size,
        "bits_per_byte": bits_per_byte,
        "expected_lexical_events_per_byte": events_per_byte,
        "bytes_per_lexical_event": (
            1.0 / events_per_byte if events_per_byte else math.inf
        ),
        "soft_oracle_boundary_precision": float(np.mean(precisions)),
        "soft_oracle_boundary_recall": float(np.mean(recalls)),
        "soft_oracle_boundary_f1": float(np.mean(f1s)),
        "active_vocabulary_without_eos": int(np.sum(expected_counts[:-1] > 1.0e-3)),
        "token_expected_counts": [float(value) for value in expected_counts],
    }
