"""Lossless detector vocabularies over a canonical byte substrate."""

from __future__ import annotations

from collections import Counter, defaultdict
from dataclasses import dataclass
import math
from typing import Iterable

import numpy as np

from .process import MOTIFS

EOS_NAME = "<eos>"


@dataclass(frozen=True, slots=True)
class VocabularyEntry:
    id: int
    surface: bytes | None
    kind: str
    score: float = 0.0

    @property
    def display(self) -> str:
        if self.surface is None:
            return EOS_NAME
        try:
            return self.surface.decode("utf-8")
        except UnicodeDecodeError:
            return self.surface.hex()


@dataclass(frozen=True, slots=True)
class Vocabulary:
    name: str
    entries: tuple[VocabularyEntry, ...]

    def __post_init__(self) -> None:
        if not self.entries:
            raise ValueError("vocabulary must be nonempty")
        if tuple(entry.id for entry in self.entries) != tuple(range(len(self.entries))):
            raise ValueError("vocabulary ids must be dense and ordered")
        surfaces = [entry.surface for entry in self.entries if entry.surface is not None]
        if len(surfaces) != len(set(surfaces)):
            raise ValueError("non-EOS surfaces must be unique")
        if sum(entry.surface is None for entry in self.entries) != 1:
            raise ValueError("vocabulary requires exactly one EOS entry")
        if any(entry.surface == b"" for entry in self.entries if entry.surface is not None):
            raise ValueError("ordinary vocabulary entries may not be empty")

    @property
    def eos_id(self) -> int:
        return next(entry.id for entry in self.entries if entry.surface is None)

    @property
    def size(self) -> int:
        return len(self.entries)

    def matches(self, sequence: bytes, start: int) -> tuple[tuple[int, int], ...]:
        result: list[tuple[int, int]] = []
        for entry in self.entries:
            if entry.surface is None:
                continue
            end = start + len(entry.surface)
            if sequence[start:end] == entry.surface:
                result.append((entry.id, end))
        return tuple(result)

    def to_dict(self) -> dict[str, object]:
        return {
            "format": "sophia.instrument.token-radiation.vocabulary/v1",
            "name": self.name,
            "entries": [
                {
                    "id": entry.id,
                    "surface_hex": None if entry.surface is None else entry.surface.hex(),
                    "display": entry.display,
                    "kind": entry.kind,
                    "score": entry.score,
                }
                for entry in self.entries
            ],
        }


def _assemble(name: str, surfaces: Iterable[tuple[bytes, str, float]]) -> Vocabulary:
    unique: dict[bytes, tuple[str, float]] = {}
    for surface, kind, score in surfaces:
        if surface and surface not in unique:
            unique[surface] = (kind, float(score))
    ordered = sorted(unique.items(), key=lambda item: (len(item[0]), item[0]))
    entries = [
        VocabularyEntry(index, surface, kind, score)
        for index, (surface, (kind, score)) in enumerate(ordered)
    ]
    entries.append(VocabularyEntry(len(entries), None, "eos", 0.0))
    return Vocabulary(name, tuple(entries))


def byte_vocabulary(alphabet: Iterable[int]) -> Vocabulary:
    return _assemble(
        "bytes", ((bytes([value]), "byte", 0.0) for value in sorted(alphabet))
    )


def train_bpe(corpus: Iterable[bytes], *, target_size: int) -> Vocabulary:
    sequences = [[bytes([value]) for value in sequence] for sequence in corpus]
    alphabet = sorted({symbol for sequence in sequences for symbol in sequence})
    if target_size < len(alphabet) + 1:
        raise ValueError("target_size is smaller than byte alphabet plus EOS")
    learned: list[tuple[bytes, float]] = []
    vocab = set(alphabet)
    target_without_eos = target_size - 1

    while len(vocab) < target_without_eos:
        pairs: Counter[tuple[bytes, bytes]] = Counter()
        for sequence in sequences:
            pairs.update(zip(sequence, sequence[1:]))
        if not pairs:
            break
        ranked = sorted(
            pairs.items(), key=lambda item: (-item[1], item[0][0], item[0][1])
        )
        pair = next(
            (
                candidate
                for candidate, _ in ranked
                if candidate[0] + candidate[1] not in vocab
            ),
            None,
        )
        if pair is None:
            break
        count = pairs[pair]
        merged = pair[0] + pair[1]
        vocab.add(merged)
        learned.append((merged, float(count)))
        next_sequences: list[list[bytes]] = []
        for sequence in sequences:
            output: list[bytes] = []
            index = 0
            while index < len(sequence):
                if (
                    index + 1 < len(sequence)
                    and (sequence[index], sequence[index + 1]) == pair
                ):
                    output.append(merged)
                    index += 2
                else:
                    output.append(sequence[index])
                    index += 1
            next_sequences.append(output)
        sequences = next_sequences

    rows = [(surface, "byte", 0.0) for surface in alphabet]
    rows.extend((surface, "bpe_merge", score) for surface, score in learned)
    return _assemble(f"bpe-{target_size}", rows[:target_without_eos])


def _candidate_counts(corpus: Iterable[bytes], max_length: int) -> Counter[bytes]:
    counts: Counter[bytes] = Counter()
    for sequence in corpus:
        for start in range(len(sequence)):
            for length in range(1, max_length + 1):
                end = start + length
                if end <= len(sequence):
                    counts[sequence[start:end]] += 1
    return counts


def _logaddexp(left: float, right: float) -> float:
    if left == -math.inf:
        return right
    if right == -math.inf:
        return left
    if left < right:
        left, right = right, left
    return left + math.log1p(math.exp(right - left))


def _unigram_em(
    corpus: tuple[bytes, ...], candidates: tuple[bytes, ...], *, iterations: int
) -> tuple[dict[bytes, float], dict[bytes, float]]:
    surface_to_id = {surface: index for index, surface in enumerate(candidates)}
    frequencies = _candidate_counts(corpus, max(len(surface) for surface in candidates))
    weights = np.asarray(
        [
            max(float(frequencies[surface]), 0.1) * math.sqrt(len(surface))
            for surface in candidates
        ],
        dtype=np.float64,
    )
    probabilities = weights / weights.sum()
    expected: dict[bytes, float] = {surface: 0.0 for surface in candidates}

    for _ in range(iterations):
        log_prob = np.log(np.maximum(probabilities, 1e-300))
        counts = np.full(len(candidates), 0.05, dtype=np.float64)
        for sequence in corpus:
            length = len(sequence)
            alpha = [-math.inf] * (length + 1)
            alpha[0] = 0.0
            matches: list[list[tuple[int, int]]] = [[] for _ in range(length)]
            for start in range(length):
                for surface, token_id in surface_to_id.items():
                    end = start + len(surface)
                    if end <= length and sequence[start:end] == surface:
                        matches[start].append((token_id, end))
                for token_id, end in matches[start]:
                    alpha[end] = _logaddexp(
                        alpha[end], alpha[start] + float(log_prob[token_id])
                    )
            log_z = alpha[length]
            if not math.isfinite(log_z):
                raise RuntimeError("unigram candidates cannot segment a sequence")
            beta = [-math.inf] * (length + 1)
            beta[length] = 0.0
            for start in range(length - 1, -1, -1):
                value = -math.inf
                for token_id, end in matches[start]:
                    value = _logaddexp(
                        value, float(log_prob[token_id]) + beta[end]
                    )
                beta[start] = value
            for start in range(length):
                for token_id, end in matches[start]:
                    posterior = math.exp(
                        alpha[start]
                        + float(log_prob[token_id])
                        + beta[end]
                        - log_z
                    )
                    counts[token_id] += posterior
        probabilities = counts / counts.sum()
        expected = {
            surface: float(counts[index])
            for index, surface in enumerate(candidates)
        }
    probability_map = {
        surface: float(probabilities[index])
        for index, surface in enumerate(candidates)
    }
    return probability_map, expected


def train_unigram(
    corpus: Iterable[bytes],
    *,
    target_size: int,
    max_length: int = 5,
    iterations: int = 6,
) -> Vocabulary:
    corpus_tuple = tuple(corpus)
    counts = _candidate_counts(corpus_tuple, max_length)
    alphabet = sorted(surface for surface in counts if len(surface) == 1)
    if target_size < len(alphabet) + 1:
        raise ValueError("target_size is smaller than byte alphabet plus EOS")
    candidate_limit = max(target_size * 6, len(alphabet) + 8)
    nonbytes = sorted(
        (surface for surface in counts if len(surface) > 1),
        key=lambda surface: (-counts[surface] * math.sqrt(len(surface)), surface),
    )[: max(candidate_limit - len(alphabet), 0)]
    candidates = tuple(alphabet + nonbytes)
    probabilities, expected = _unigram_em(
        corpus_tuple, candidates, iterations=iterations
    )
    keep = set(alphabet)
    ranked = sorted(
        nonbytes,
        key=lambda surface: (
            -(expected.get(surface, 0.0) * max(len(surface) - 1, 1)),
            -probabilities.get(surface, 0.0),
            surface,
        ),
    )
    keep.update(ranked[: max(target_size - 1 - len(alphabet), 0)])
    rows = []
    for surface in keep:
        kind = "byte" if len(surface) == 1 else "unigram_piece"
        rows.append((surface, kind, expected.get(surface, 0.0)))
    return _assemble(f"unigram-{target_size}", rows)


def _future_signature(
    sequence: bytes, boundary: int, horizon: int
) -> tuple[int, ...]:
    values = list(sequence[boundary : boundary + horizon])
    values.extend([256] * (horizon - len(values)))
    return tuple(values)


def train_predictive_vocabulary(
    corpus: Iterable[bytes],
    *,
    target_size: int,
    max_length: int = 5,
    future_horizon: int = 3,
    minimum_count: int = 6,
) -> Vocabulary:
    corpus_tuple = tuple(corpus)
    counts = _candidate_counts(corpus_tuple, max_length)
    alphabet = sorted(surface for surface in counts if len(surface) == 1)
    if target_size < len(alphabet) + 1:
        raise ValueError("target_size is smaller than byte alphabet plus EOS")

    global_future: Counter[tuple[int, ...]] = Counter()
    conditional: dict[bytes, Counter[tuple[int, ...]]] = defaultdict(Counter)
    for sequence in corpus_tuple:
        for boundary in range(len(sequence) + 1):
            global_future[_future_signature(sequence, boundary, future_horizon)] += 1
        for start in range(len(sequence)):
            for length in range(2, max_length + 1):
                end = start + length
                if end <= len(sequence):
                    surface = sequence[start:end]
                    conditional[surface][
                        _future_signature(sequence, end, future_horizon)
                    ] += 1

    signatures = sorted(global_future)
    smoothing = 0.25
    global_total = sum(global_future.values()) + smoothing * len(signatures)
    global_probability = {
        signature: (global_future[signature] + smoothing) / global_total
        for signature in signatures
    }
    scored: list[tuple[float, bytes]] = []
    for surface, future_counts in conditional.items():
        count = counts[surface]
        if count < minimum_count:
            continue
        total = sum(future_counts.values()) + smoothing * len(signatures)
        divergence = 0.0
        for signature in signatures:
            probability = (future_counts[signature] + smoothing) / total
            divergence += probability * math.log(
                probability / global_probability[signature]
            )
        score = count * divergence * math.sqrt(max(len(surface) - 1, 1))
        scored.append((score, surface))
    scored.sort(key=lambda item: (-item[0], item[1]))
    selected = scored[: max(target_size - 1 - len(alphabet), 0)]
    rows = [(surface, "byte", 0.0) for surface in alphabet]
    rows.extend(
        (surface, "predictive_piece", score) for score, surface in selected
    )
    return _assemble(f"predictive-{target_size}", rows)


def oracle_motif_vocabulary(alphabet: Iterable[int]) -> Vocabulary:
    rows = [(bytes([value]), "byte", 0.0) for value in sorted(alphabet)]
    rows.extend((motif, "oracle_motif", 1.0) for motif in MOTIFS)
    return _assemble("oracle-motifs", rows)
