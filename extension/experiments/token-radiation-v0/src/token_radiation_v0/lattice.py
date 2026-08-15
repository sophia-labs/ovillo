"""Exact segmentation marginalization over one byte-driven body trajectory."""

from __future__ import annotations

from dataclasses import dataclass
import math
from typing import Protocol

import numpy as np
import torch

from .vocabulary import Vocabulary


class Detector(Protocol):
    def __call__(self, emissions: torch.Tensor) -> torch.Tensor: ...


@dataclass(frozen=True, slots=True)
class LatticeStatistics:
    log_likelihood: float
    expected_tokens: float
    events_per_byte: float
    token_expected_counts: tuple[float, ...]


def sequence_log_likelihood(
    emissions: torch.Tensor,
    sequence: bytes,
    vocabulary: Vocabulary,
    detector: Detector,
) -> torch.Tensor:
    if emissions.ndim != 2 or emissions.shape[0] != len(sequence) + 1:
        raise ValueError(
            "emissions must have shape [len(sequence)+1, radiation_rank]"
        )
    logits = detector(emissions)
    if logits.shape != (len(sequence) + 1, vocabulary.size):
        raise ValueError("detector output does not match lattice dimensions")
    log_probabilities = torch.log_softmax(logits, dim=-1)
    negative_infinity = emissions.new_tensor(-torch.inf)
    alpha: list[torch.Tensor] = [
        negative_infinity for _ in range(len(sequence) + 1)
    ]
    alpha[0] = emissions.new_zeros(())
    for start in range(len(sequence)):
        for token_id, end in vocabulary.matches(sequence, start):
            candidate = alpha[start] + log_probabilities[start, token_id]
            alpha[end] = torch.logaddexp(alpha[end], candidate)
    return alpha[len(sequence)] + log_probabilities[len(sequence), vocabulary.eos_id]


def _logaddexp(left: float, right: float) -> float:
    if left == -math.inf:
        return right
    if right == -math.inf:
        return left
    if left < right:
        left, right = right, left
    return left + math.log1p(math.exp(right - left))


def lattice_statistics_from_logits(
    logits: np.ndarray, sequence: bytes, vocabulary: Vocabulary
) -> LatticeStatistics:
    logits = np.asarray(logits, dtype=np.float64)
    if logits.shape != (len(sequence) + 1, vocabulary.size):
        raise ValueError("logit table shape mismatch")
    maximum = logits.max(axis=1, keepdims=True)
    log_probabilities = (
        logits
        - maximum
        - np.log(np.exp(logits - maximum).sum(axis=1, keepdims=True))
    )
    length = len(sequence)
    matches = [vocabulary.matches(sequence, start) for start in range(length)]

    alpha = [-math.inf] * (length + 1)
    alpha[0] = 0.0
    for start in range(length):
        for token_id, end in matches[start]:
            alpha[end] = _logaddexp(
                alpha[end],
                alpha[start] + float(log_probabilities[start, token_id]),
            )

    beta = [-math.inf] * (length + 1)
    beta[length] = float(log_probabilities[length, vocabulary.eos_id])
    for start in range(length - 1, -1, -1):
        value = -math.inf
        for token_id, end in matches[start]:
            value = _logaddexp(
                value, float(log_probabilities[start, token_id]) + beta[end]
            )
        beta[start] = value

    log_z = alpha[length] + float(log_probabilities[length, vocabulary.eos_id])
    if not math.isfinite(log_z) or abs(beta[0] - log_z) > 1e-8:
        raise RuntimeError("segmentation lattice is disconnected or inconsistent")

    expected = np.zeros(vocabulary.size, dtype=np.float64)
    for start in range(length):
        for token_id, end in matches[start]:
            log_posterior = (
                alpha[start]
                + float(log_probabilities[start, token_id])
                + beta[end]
                - log_z
            )
            expected[token_id] += math.exp(log_posterior)
    expected[vocabulary.eos_id] = 1.0
    lexical_events = float(expected.sum() - 1.0)
    return LatticeStatistics(
        log_likelihood=log_z,
        expected_tokens=lexical_events,
        events_per_byte=lexical_events / max(length, 1),
        token_expected_counts=tuple(float(value) for value in expected),
    )


def brute_force_log_likelihood(
    logits: np.ndarray, sequence: bytes, vocabulary: Vocabulary
) -> float:
    logits = np.asarray(logits, dtype=np.float64)
    maximum = logits.max(axis=1, keepdims=True)
    log_probabilities = (
        logits
        - maximum
        - np.log(np.exp(logits - maximum).sum(axis=1, keepdims=True))
    )
    paths: list[float] = []

    def visit(position: int, total: float) -> None:
        if position == len(sequence):
            paths.append(total + float(log_probabilities[position, vocabulary.eos_id]))
            return
        for token_id, end in vocabulary.matches(sequence, position):
            visit(end, total + float(log_probabilities[position, token_id]))

    visit(0, 0.0)
    if not paths:
        return -math.inf
    result = paths[0]
    for value in paths[1:]:
        result = _logaddexp(result, value)
    return result


def implied_next_byte_distribution(
    logits: np.ndarray,
    vocabulary: Vocabulary,
    alphabet: tuple[int, ...],
) -> tuple[np.ndarray, float]:
    logits = np.asarray(logits, dtype=np.float64)
    maximum = float(logits.max())
    probabilities = np.exp(logits - maximum)
    probabilities /= probabilities.sum()
    byte_to_index = {value: index for index, value in enumerate(alphabet)}
    output = np.zeros(len(alphabet), dtype=np.float64)
    for entry in vocabulary.entries:
        if entry.surface:
            output[byte_to_index[entry.surface[0]]] += probabilities[entry.id]
    return output, float(probabilities[vocabulary.eos_id])
