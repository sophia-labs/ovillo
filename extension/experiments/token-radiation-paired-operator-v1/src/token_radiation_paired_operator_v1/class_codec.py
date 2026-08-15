"""Lossless operator-class event codecs with separately receipted surface residuals.

An event class is not identified with one surface string. The detector predicts
an event class from the current radiation coordinates, while a normalized
within-class residual code identifies the exact byte span. Exact dynamic
programming marginalizes over every compatible segmentation and surface
variant, so the codec remains lossless and byte-normalized.
"""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass, field
import math
import random
from typing import Any, Iterable

import numpy as np
import torch
from torch import nn
import torch.nn.functional as F

from .model import BytePredictionModel
from .process import Corpus, Example
from .vocabulary import _operator_feature_matrix, enumerate_candidates


@dataclass(frozen=True, slots=True)
class EventVariant:
    surface: bytes
    log_probability: float = 0.0
    metadata: dict[str, Any] = field(default_factory=dict)


@dataclass(frozen=True, slots=True)
class EventEntry:
    name: str
    variants: tuple[EventVariant, ...]
    metadata: dict[str, Any] = field(default_factory=dict)

    def validate(self) -> None:
        if not self.variants:
            return
        surfaces = [variant.surface for variant in self.variants]
        if len(surfaces) != len(set(surfaces)):
            raise ValueError("event variants must have unique surfaces")
        probabilities = np.exp(
            np.asarray([variant.log_probability for variant in self.variants])
        )
        if not np.isfinite(probabilities).all():
            raise ValueError("event residual probabilities must be finite")
        if abs(float(probabilities.sum()) - 1.0) > 1.0e-9:
            raise ValueError("event residual probabilities must sum to one")


@dataclass(frozen=True, slots=True)
class EventVocabulary:
    name: str
    entries: tuple[EventEntry, ...]
    alphabet: tuple[int, ...]
    metadata: dict[str, Any] = field(default_factory=dict)

    @property
    def size(self) -> int:
        return len(self.entries)

    @property
    def eos_index(self) -> int:
        indices = [index for index, entry in enumerate(self.entries) if not entry.variants]
        if len(indices) != 1:
            raise ValueError("event vocabulary must contain exactly one EOS event")
        return indices[0]

    def validate(self) -> None:
        for entry in self.entries:
            entry.validate()
        byte_surfaces = {
            variant.surface
            for entry in self.entries
            for variant in entry.variants
            if len(variant.surface) == 1
        }
        required = {bytes((byte,)) for byte in self.alphabet}
        if not required.issubset(byte_surfaces):
            raise ValueError("event vocabulary must preserve complete byte coverage")
        _ = self.eos_index

    def to_dict(self) -> dict[str, Any]:
        self.validate()
        return {
            "name": self.name,
            "alphabet": list(self.alphabet),
            "metadata": self.metadata,
            "entries": [
                {
                    "name": entry.name,
                    "metadata": entry.metadata,
                    "variants": [
                        {
                            "surface_ascii": variant.surface.decode("ascii"),
                            "surface_hex": variant.surface.hex(),
                            "log_probability": variant.log_probability,
                            "probability": math.exp(variant.log_probability),
                            "metadata": variant.metadata,
                        }
                        for variant in entry.variants
                    ],
                }
                for entry in self.entries
            ],
        }


def _surface_counts(corpus: Corpus, candidates: Iterable[bytes]) -> Counter[bytes]:
    result: Counter[bytes] = Counter()
    candidate_list = tuple(candidates)
    for example in corpus.train:
        for candidate in candidate_list:
            start = 0
            while True:
                index = example.surface.find(candidate, start)
                if index < 0:
                    break
                result[candidate] += 1
                start = index + 1
    return result


def _residual_variants(
    surfaces: list[bytes], counts: Counter[bytes], *, smoothing: float = 0.5
) -> tuple[EventVariant, ...]:
    weights = np.asarray([counts[surface] + smoothing for surface in surfaces], dtype=float)
    weights /= weights.sum()
    return tuple(
        EventVariant(
            surface=surface,
            log_probability=float(math.log(probability)),
            metadata={"training_occurrences": int(counts[surface])},
        )
        for surface, probability in zip(surfaces, weights, strict=True)
    )


def _weighted_kmeans(
    matrix: np.ndarray,
    weights: np.ndarray,
    *,
    clusters: int,
    iterations: int = 30,
) -> np.ndarray:
    """Deterministic weighted Euclidean clustering with farthest-first seeds."""

    if matrix.ndim != 2 or matrix.shape[0] < clusters or clusters < 1:
        raise ValueError("invalid operator clustering dimensions")
    weights = np.asarray(weights, dtype=np.float64)
    weights = np.maximum(weights, 1.0e-12)
    first = int(np.argmax(weights * (1.0 + np.linalg.norm(matrix, axis=1))))
    centers = [matrix[first].copy()]
    chosen = {first}
    while len(centers) < clusters:
        distances = np.min(
            np.stack(
                [np.sum((matrix - center) ** 2, axis=1) for center in centers],
                axis=1,
            ),
            axis=1,
        )
        score = weights * distances
        for index in chosen:
            score[index] = -np.inf
        selected = int(np.argmax(score))
        chosen.add(selected)
        centers.append(matrix[selected].copy())
    centers_array = np.asarray(centers)
    assignments = np.zeros(matrix.shape[0], dtype=np.int64)
    for _ in range(iterations):
        distances = np.stack(
            [np.sum((matrix - center) ** 2, axis=1) for center in centers_array],
            axis=1,
        )
        next_assignments = np.argmin(distances, axis=1)
        next_centers = centers_array.copy()
        for cluster in range(clusters):
            members = np.flatnonzero(next_assignments == cluster)
            if members.size == 0:
                nearest = np.min(distances, axis=1)
                selected = int(np.argmax(weights * nearest))
                next_assignments[selected] = cluster
                members = np.asarray([selected])
            local_weights = weights[members]
            next_centers[cluster] = np.average(
                matrix[members], axis=0, weights=local_weights
            )
        if np.array_equal(assignments, next_assignments):
            assignments = next_assignments
            centers_array = next_centers
            break
        assignments = next_assignments
        centers_array = next_centers
    return assignments


def _balanced_random_assignments(count: int, clusters: int, seed: int) -> np.ndarray:
    order = list(range(count))
    random.Random(seed).shuffle(order)
    assignments = np.empty(count, dtype=np.int64)
    for position, index in enumerate(order):
        assignments[index] = position % clusters
    return assignments


def _length_assignments(surfaces: list[bytes], clusters: int) -> np.ndarray:
    lengths = sorted(set(len(surface) for surface in surfaces))
    length_to_cluster = {
        length: min(position * clusters // max(len(lengths), 1), clusters - 1)
        for position, length in enumerate(lengths)
    }
    assignments = np.asarray([length_to_cluster[len(surface)] for surface in surfaces])
    # Reassign empty labels deterministically so every detector channel has support.
    present = set(int(value) for value in assignments)
    for missing in sorted(set(range(clusters)) - present):
        largest = max(
            present,
            key=lambda label: int(np.sum(assignments == label)),
        )
        members = np.flatnonzero(assignments == largest)
        assignments[members[-1]] = missing
        present.add(missing)
    return assignments


def _event_vocabulary(
    corpus: Corpus,
    surfaces: list[bytes],
    counts: Counter[bytes],
    assignments: np.ndarray,
    *,
    name: str,
    metadata: dict[str, Any],
) -> EventVocabulary:
    entries: list[EventEntry] = [
        EventEntry(
            name=f"byte-{byte:02x}",
            variants=(EventVariant(bytes((byte,)), 0.0),),
            metadata={"kind": "byte"},
        )
        for byte in corpus.alphabet
    ]
    for cluster in sorted(set(int(value) for value in assignments)):
        members = [
            surface
            for surface, assignment in zip(surfaces, assignments, strict=True)
            if int(assignment) == cluster
        ]
        entries.append(
            EventEntry(
                name=f"class-{cluster:02d}",
                variants=_residual_variants(members, counts),
                metadata={
                    "kind": "operator_class",
                    "surface_count": len(members),
                },
            )
        )
    entries.append(EventEntry(name="eos", variants=(), metadata={"kind": "eos"}))
    vocabulary = EventVocabulary(
        name=name,
        entries=tuple(entries),
        alphabet=corpus.alphabet,
        metadata=metadata,
    )
    vocabulary.validate()
    return vocabulary


def build_event_vocabularies(
    corpus: Corpus,
    model: BytePredictionModel,
    *,
    loading: bool,
    class_count: int,
    surface_budget: int,
    probe_states: int,
    seed: int,
) -> tuple[EventVocabulary, ...]:
    candidates = enumerate_candidates(corpus.train_surfaces)
    ranked = sorted(
        candidates,
        key=lambda item: (-(item[1] * (len(item[0]) - 1)), -item[1], item[0]),
    )[:surface_budget]
    surfaces = [surface for surface, _ in ranked]
    if len(surfaces) < class_count:
        raise ValueError("surface budget must cover every event class")
    counts = _surface_counts(corpus, surfaces)
    matrix, feature_diagnostics = _operator_feature_matrix(
        corpus,
        model,
        ranked,
        loading=loading,
        probe_states=probe_states,
        seed=seed,
    )
    weights = np.asarray(
        [max(counts[surface] * (len(surface) - 1), 1) for surface in surfaces],
        dtype=np.float64,
    )
    operator_assignments = _weighted_kmeans(matrix, weights, clusters=class_count)
    random_assignments = _balanced_random_assignments(
        len(surfaces), class_count, seed + 101
    )
    length_assignments = _length_assignments(surfaces, class_count)
    exact_surfaces = surfaces[:class_count]
    exact_assignments = np.arange(class_count, dtype=np.int64)

    common = {
        "class_count": class_count,
        "surface_budget": surface_budget,
        "candidate_surfaces": [surface.decode("ascii") for surface in surfaces],
        "candidate_counts": {
            surface.decode("ascii"): int(counts[surface]) for surface in surfaces
        },
        "operator_features": feature_diagnostics,
        "loading": loading,
    }
    return (
        _event_vocabulary(
            corpus,
            surfaces,
            counts,
            operator_assignments,
            name=f"operator-classes-{class_count}x{surface_budget}",
            metadata=common | {"method": "operator_weighted_kmeans"},
        ),
        _event_vocabulary(
            corpus,
            surfaces,
            counts,
            random_assignments,
            name=f"random-classes-{class_count}x{surface_budget}",
            metadata=common | {"method": "balanced_random", "seed": seed + 101},
        ),
        _event_vocabulary(
            corpus,
            surfaces,
            counts,
            length_assignments,
            name=f"length-classes-{class_count}x{surface_budget}",
            metadata=common | {"method": "surface_length"},
        ),
        _event_vocabulary(
            corpus,
            exact_surfaces,
            counts,
            exact_assignments,
            name=f"exact-frequency-{class_count}",
            metadata={
                "method": "exact_frequency",
                "class_count": class_count,
                "surface_budget": class_count,
                "candidate_surfaces": [
                    surface.decode("ascii") for surface in exact_surfaces
                ],
                "loading": loading,
            },
        ),
    )


def event_transitions(
    sequence: bytes, vocabulary: EventVocabulary
) -> tuple[tuple[tuple[int, int, float], ...], ...]:
    """Compile the sparse lossless lattice for one surface once."""

    rows: list[tuple[tuple[int, int, float], ...]] = []
    for position in range(len(sequence)):
        transitions: list[tuple[int, int, float]] = []
        for event_index, entry in enumerate(vocabulary.entries):
            for variant in entry.variants:
                if sequence.startswith(variant.surface, position):
                    transitions.append(
                        (
                            event_index,
                            position + len(variant.surface),
                            variant.log_probability,
                        )
                    )
        rows.append(tuple(transitions))
    return tuple(rows)


def sequence_event_log_likelihood_from_transitions(
    features: torch.Tensor,
    transitions: tuple[tuple[tuple[int, int, float], ...], ...],
    vocabulary: EventVocabulary,
    detector: nn.Module,
) -> torch.Tensor:
    logits = detector(features)
    log_probs = F.log_softmax(logits, dim=-1)
    length = len(transitions)
    negative_infinity = logits.new_tensor(float("-inf"))
    alpha = [negative_infinity for _ in range(length + 1)]
    alpha[0] = logits.new_zeros(())
    for position, local in enumerate(transitions):
        if not bool(torch.isfinite(alpha[position])):
            continue
        for event_index, end, residual_log_probability in local:
            value = (
                alpha[position]
                + log_probs[position, event_index]
                + residual_log_probability
            )
            alpha[end] = torch.logaddexp(alpha[end], value)
    return alpha[length] + log_probs[length, vocabulary.eos_index]


def sequence_event_log_likelihood(
    features: torch.Tensor,
    sequence: bytes,
    vocabulary: EventVocabulary,
    detector: nn.Module,
) -> torch.Tensor:
    return sequence_event_log_likelihood_from_transitions(
        features, event_transitions(sequence, vocabulary), vocabulary, detector
    )


@dataclass(frozen=True, slots=True)
class EventLatticeStatistics:
    log_likelihood: float
    expected_events: float
    expected_residual_nats: float
    event_expected_counts: tuple[float, ...]
    boundary_probabilities: tuple[float, ...]


def event_lattice_statistics_from_logits(
    logits: np.ndarray,
    sequence: bytes,
    vocabulary: EventVocabulary,
    *,
    transitions: tuple[tuple[tuple[int, int, float], ...], ...] | None = None,
) -> EventLatticeStatistics:
    logits = np.asarray(logits, dtype=np.float64)
    logits = logits - logits.max(axis=-1, keepdims=True)
    log_probs = logits - np.log(np.exp(logits).sum(axis=-1, keepdims=True))
    length = len(sequence)
    compiled = transitions or event_transitions(sequence, vocabulary)
    alpha = np.full(length + 1, -np.inf, dtype=np.float64)
    alpha[0] = 0.0
    for position, local in enumerate(compiled):
        for event_index, end, residual_log_probability in local:
            alpha[end] = np.logaddexp(
                alpha[end],
                alpha[position]
                + log_probs[position, event_index]
                + residual_log_probability,
            )
    total = alpha[length] + log_probs[length, vocabulary.eos_index]
    beta = np.full(length + 1, -np.inf, dtype=np.float64)
    beta[length] = log_probs[length, vocabulary.eos_index]
    for position in range(length - 1, -1, -1):
        values: list[float] = []
        for event_index, end, residual_log_probability in compiled[position]:
            values.append(
                log_probs[position, event_index]
                + residual_log_probability
                + beta[end]
            )
        if values:
            beta[position] = np.logaddexp.reduce(values)
    expected_counts = np.zeros(vocabulary.size, dtype=np.float64)
    boundaries = np.zeros(length + 1, dtype=np.float64)
    expected_events = 0.0
    expected_residual = 0.0
    for position, local in enumerate(compiled):
        for event_index, end, residual_log_probability in local:
            posterior = math.exp(
                alpha[position]
                + log_probs[position, event_index]
                + residual_log_probability
                + beta[end]
                - total
            )
            expected_counts[event_index] += posterior
            expected_events += posterior
            expected_residual += posterior * -residual_log_probability
            boundaries[end] += posterior
    expected_counts[vocabulary.eos_index] = 1.0
    boundaries[0] = 1.0
    boundaries[length] = 1.0
    return EventLatticeStatistics(
        log_likelihood=float(total),
        expected_events=float(expected_events),
        expected_residual_nats=float(expected_residual),
        event_expected_counts=tuple(float(value) for value in expected_counts),
        boundary_probabilities=tuple(float(value) for value in boundaries),
    )


def _soft_boundary_f1(
    statistics: EventLatticeStatistics, example: Example
) -> tuple[float, float, float]:
    predicted = np.asarray(statistics.boundary_probabilities, dtype=np.float64)
    truth = np.zeros_like(predicted)
    for boundary in example.motif_boundaries[1:-1]:
        truth[boundary] = 1.0
    predicted[0] = 0.0
    predicted[-1] = 0.0
    true_positive = float((predicted * truth).sum())
    precision = true_positive / max(float(predicted.sum()), 1.0e-12)
    recall = true_positive / max(float(truth.sum()), 1.0e-12)
    f1 = 2.0 * precision * recall / max(precision + recall, 1.0e-12)
    return precision, recall, f1


def fit_event_detector(
    vocabulary: EventVocabulary,
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
    train_transitions = tuple(
        event_transitions(example.surface, vocabulary) for example in corpus.train
    )
    validation_transitions = tuple(
        event_transitions(example.surface, vocabulary) for example in corpus.validation
    )
    for _ in range(steps):
        indices = [random_source.randrange(len(corpus.train)) for _ in range(batch_size)]
        optimizer.zero_grad(set_to_none=True)
        loss = train_features[0].new_zeros(())
        bytes_seen = 0
        for index in indices:
            loss = loss - sequence_event_log_likelihood_from_transitions(
                train_features[index],
                train_transitions[index],
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
    total_events = 0.0
    total_residual_nats = 0.0
    expected_counts = np.zeros(vocabulary.size, dtype=np.float64)
    f1s: list[float] = []
    precisions: list[float] = []
    recalls: list[float] = []
    with torch.no_grad():
        for features, example, compiled in zip(
            validation_features, corpus.validation, validation_transitions, strict=True
        ):
            logits = detector(features).cpu().numpy()
            statistics = event_lattice_statistics_from_logits(
                logits, example.surface, vocabulary, transitions=compiled
            )
            total_log_likelihood += statistics.log_likelihood
            total_bytes += len(example.surface)
            total_events += statistics.expected_events
            total_residual_nats += statistics.expected_residual_nats
            expected_counts += np.asarray(statistics.event_expected_counts)
            precision, recall, f1 = _soft_boundary_f1(statistics, example)
            precisions.append(precision)
            recalls.append(recall)
            f1s.append(f1)
    bits_per_byte = -total_log_likelihood / (total_bytes * math.log(2.0))
    events_per_byte = total_events / total_bytes
    residual_bits_per_byte = total_residual_nats / (total_bytes * math.log(2.0))
    return {
        "vocabulary": vocabulary.name,
        "event_vocabulary_size": vocabulary.size,
        "macro_event_classes": vocabulary.size - len(corpus.alphabet) - 1,
        "covered_macro_surfaces": sum(
            len(entry.variants)
            for entry in vocabulary.entries
            if entry.metadata.get("kind") == "operator_class"
        ),
        "bits_per_byte": bits_per_byte,
        "expected_lexical_events_per_byte": events_per_byte,
        "bytes_per_lexical_event": (
            1.0 / events_per_byte if events_per_byte else math.inf
        ),
        "surface_residual_bits_per_byte": residual_bits_per_byte,
        "detector_and_segmentation_bits_per_byte": bits_per_byte
        - residual_bits_per_byte,
        "soft_oracle_boundary_precision": float(np.mean(precisions)),
        "soft_oracle_boundary_recall": float(np.mean(recalls)),
        "soft_oracle_boundary_f1": float(np.mean(f1s)),
        "active_event_classes_without_eos": int(
            np.sum(expected_counts[:-1] > 1.0e-3)
        ),
        "event_expected_counts": [float(value) for value in expected_counts],
    }
