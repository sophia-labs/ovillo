"""Vocabulary baselines and state-transition operator covers."""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass, field
import hashlib
import math
import random
from typing import Any, Iterable

import numpy as np
import torch
import torch.nn.functional as F

from .model import BytePredictionModel
from .paired import encode_corpus
from .process import Corpus, MOTIFS


@dataclass(frozen=True, slots=True)
class VocabularyEntry:
    name: str
    surface: bytes | None
    metadata: dict[str, Any] = field(default_factory=dict)


@dataclass(frozen=True, slots=True)
class Vocabulary:
    name: str
    entries: tuple[VocabularyEntry, ...]
    alphabet: tuple[int, ...]
    metadata: dict[str, Any] = field(default_factory=dict)

    @property
    def size(self) -> int:
        return len(self.entries)

    @property
    def eos_index(self) -> int:
        indices = [
            index for index, entry in enumerate(self.entries) if entry.surface is None
        ]
        if len(indices) != 1:
            raise ValueError("vocabulary must contain exactly one EOS entry")
        return indices[0]

    def validate(self) -> None:
        surfaces = [
            entry.surface for entry in self.entries if entry.surface is not None
        ]
        if len(surfaces) != len(set(surfaces)):
            raise ValueError("vocabulary surfaces must be unique")
        for byte in self.alphabet:
            if bytes((byte,)) not in surfaces:
                raise ValueError("vocabulary must preserve complete byte coverage")
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
                    "surface_hex": (
                        entry.surface.hex() if entry.surface is not None else None
                    ),
                    "surface_ascii": (
                        entry.surface.decode("ascii")
                        if entry.surface is not None
                        else None
                    ),
                    "metadata": entry.metadata,
                }
                for entry in self.entries
            ],
        }


def _make_vocabulary(
    name: str,
    alphabet: tuple[int, ...],
    macros: Iterable[bytes],
    *,
    metadata: dict[str, Any] | None = None,
) -> Vocabulary:
    unique_macros: list[bytes] = []
    seen = {bytes((byte,)) for byte in alphabet}
    for surface in macros:
        if len(surface) < 2 or surface in seen:
            continue
        seen.add(surface)
        unique_macros.append(surface)
    entries = [
        VocabularyEntry(name=f"byte-{byte:02x}", surface=bytes((byte,)))
        for byte in alphabet
    ]
    entries.extend(
        VocabularyEntry(name=f"macro-{surface.hex()}", surface=surface)
        for surface in unique_macros
    )
    entries.append(VocabularyEntry(name="eos", surface=None))
    vocabulary = Vocabulary(
        name=name,
        entries=tuple(entries),
        alphabet=alphabet,
        metadata=metadata or {},
    )
    vocabulary.validate()
    return vocabulary


def byte_vocabulary(corpus: Corpus) -> Vocabulary:
    return _make_vocabulary("bytes", corpus.alphabet, ())


def enumerate_candidates(
    surfaces: Iterable[bytes], *, min_length: int = 2, max_length: int = 5
) -> list[tuple[bytes, int]]:
    counts: Counter[bytes] = Counter()
    for surface in surfaces:
        for length in range(min_length, max_length + 1):
            for start in range(0, len(surface) - length + 1):
                counts[surface[start : start + length]] += 1
    return sorted(
        ((surface, count) for surface, count in counts.items() if count >= 2),
        key=lambda item: (-item[1], -len(item[0]), item[0]),
    )


def train_bpe(corpus: Corpus, *, target_size: int) -> Vocabulary:
    macro_budget = max(target_size - len(corpus.alphabet) - 1, 0)
    sequences: list[list[bytes]] = [
        [bytes((value,)) for value in example.surface] for example in corpus.train
    ]
    macros: list[bytes] = []
    while len(macros) < macro_budget:
        pairs: Counter[tuple[bytes, bytes]] = Counter()
        for sequence in sequences:
            pairs.update(zip(sequence, sequence[1:]))
        if not pairs:
            break
        pair, count = min(
            pairs.items(), key=lambda item: (-item[1], item[0][0] + item[0][1])
        )
        if count < 2:
            break
        merged = pair[0] + pair[1]
        if merged not in macros:
            macros.append(merged)
        next_sequences: list[list[bytes]] = []
        for sequence in sequences:
            result: list[bytes] = []
            index = 0
            while index < len(sequence):
                if (
                    index + 1 < len(sequence)
                    and (sequence[index], sequence[index + 1]) == pair
                ):
                    result.append(merged)
                    index += 2
                else:
                    result.append(sequence[index])
                    index += 1
            next_sequences.append(result)
        sequences = next_sequences
    return _make_vocabulary(
        f"bpe-{target_size}",
        corpus.alphabet,
        macros[:macro_budget],
        metadata={"method": "bpe", "target_size": target_size},
    )


def train_frequency_mdl(corpus: Corpus, *, target_size: int) -> Vocabulary:
    macro_budget = max(target_size - len(corpus.alphabet) - 1, 0)
    candidates = enumerate_candidates(corpus.train_surfaces)
    ranked = sorted(
        candidates,
        key=lambda item: (
            -(item[1] * (len(item[0]) - 1) - 0.5 * len(item[0])),
            item[0],
        ),
    )
    return _make_vocabulary(
        f"frequency-mdl-{target_size}",
        corpus.alphabet,
        [surface for surface, _ in ranked[:macro_budget]],
        metadata={"method": "frequency_mdl", "target_size": target_size},
    )


def _next_byte_counts(corpus: Corpus, candidate: bytes) -> Counter[int]:
    counts: Counter[int] = Counter()
    for example in corpus.train:
        surface = example.surface
        start = 0
        while True:
            index = surface.find(candidate, start)
            if index < 0:
                break
            end = index + len(candidate)
            counts[surface[end] if end < len(surface) else 256] += 1
            start = index + 1
    return counts


def train_endpoint_predictive(corpus: Corpus, *, target_size: int) -> Vocabulary:
    macro_budget = max(target_size - len(corpus.alphabet) - 1, 0)
    candidates = enumerate_candidates(corpus.train_surfaces)
    global_counts: Counter[int] = Counter()
    for example in corpus.train:
        global_counts.update(example.surface)
        global_counts[256] += 1
    global_total = sum(global_counts.values())
    support = tuple(sorted(global_counts))
    global_distribution = np.asarray(
        [
            (global_counts[value] + 0.5) / (global_total + 0.5 * len(support))
            for value in support
        ]
    )
    scored: list[tuple[float, bytes]] = []
    for candidate, count in candidates:
        future_counts = _next_byte_counts(corpus, candidate)
        total = sum(future_counts.values())
        distribution = np.asarray(
            [
                (future_counts[value] + 0.5) / (total + 0.5 * len(support))
                for value in support
            ]
        )
        kl = float(
            np.sum(distribution * np.log(distribution / global_distribution))
        )
        scored.append((count * kl * math.sqrt(len(candidate)), candidate))
    scored.sort(key=lambda item: (-item[0], item[1]))
    return _make_vocabulary(
        f"endpoint-predictive-{target_size}",
        corpus.alphabet,
        [surface for _, surface in scored[:macro_budget]],
        metadata={"method": "endpoint_predictive", "target_size": target_size},
    )


def train_random_frequency_matched(
    corpus: Corpus, *, target_size: int, seed: int
) -> Vocabulary:
    macro_budget = max(target_size - len(corpus.alphabet) - 1, 0)
    candidates = enumerate_candidates(corpus.train_surfaces)
    rng = random.Random(seed)
    pool = list(candidates)
    selected: list[bytes] = []
    while pool and len(selected) < macro_budget:
        weights = [max(count * (len(surface) - 1), 1) for surface, count in pool]
        chosen = rng.choices(range(len(pool)), weights=weights, k=1)[0]
        selected.append(pool.pop(chosen)[0])
    return _make_vocabulary(
        f"random-frequency-matched-{target_size}",
        corpus.alphabet,
        selected,
        metadata={
            "method": "random_frequency_matched",
            "target_size": target_size,
            "seed": seed,
        },
    )


def oracle_vocabulary(
    corpus: Corpus, *, target_size: int | None = None
) -> Vocabulary:
    motifs = list(MOTIFS)
    if target_size is not None:
        budget = max(target_size - len(corpus.alphabet) - 1, 0)
        counts: Counter[bytes] = Counter()
        for example in corpus.train:
            counts.update(MOTIFS[index] for index in example.motif_indices)
        motifs = [surface for surface, _ in counts.most_common(budget)]
        name = f"oracle-frequency-budget-{target_size}"
    else:
        name = "oracle-motifs"
    return _make_vocabulary(
        name, corpus.alphabet, motifs, metadata={"oracle": True}
    )


def _operator_feature_matrix(
    corpus: Corpus,
    model: BytePredictionModel,
    candidates: list[tuple[bytes, int]],
    *,
    loading: bool,
    probe_states: int,
    seed: int,
) -> tuple[np.ndarray, dict[str, Any]]:
    train_ids = encode_corpus(corpus)
    model.eval()
    with torch.no_grad():
        trace = model.body(train_ids, loading=loading)
        all_wave = trace.waves[:, :-1].reshape(-1, trace.waves.shape[-1])
        all_evidence = trace.evidence[:, :-1].reshape(
            -1, trace.evidence.shape[-1]
        )
        generator = torch.Generator().manual_seed(seed)
        indices = torch.randperm(all_wave.shape[0], generator=generator)[:probe_states]
        base_wave = all_wave[indices]
        base_evidence = all_evidence[indices]
        rows: list[np.ndarray] = []
        for surface, _ in candidates:
            encoded = torch.tensor(corpus.encode(surface), dtype=torch.long)
            ids = encoded[None, :].repeat(probe_states, 1)
            final_wave, final_evidence, coordinate_sum, intensity_sum = (
                model.body.advance_ids(
                    base_wave.clone(),
                    base_evidence.clone(),
                    ids,
                    loading=loading,
                )
            )
            constrained = model.body.constrained()
            final_coordinates = torch.einsum(
                "bn,kn->bk",
                final_wave * constrained["metric"].sqrt(),
                constrained["aperture_rows"],
            )
            final_log_probs = F.log_softmax(
                model.logits_from_coordinates(final_coordinates), dim=-1
            )
            delta_wave = final_wave - base_wave
            delta_evidence = final_evidence - base_evidence
            feature = torch.cat(
                (
                    delta_wave.mean(dim=0),
                    delta_wave.std(dim=0, unbiased=False),
                    delta_evidence.mean(dim=0),
                    coordinate_sum.mean(dim=0),
                    final_log_probs.mean(dim=0),
                    intensity_sum.mean().reshape(1),
                )
            )
            rows.append(feature.detach().cpu().numpy())
    raw = np.asarray(rows, dtype=np.float64)
    mean = raw.mean(axis=0, keepdims=True)
    scale = raw.std(axis=0, keepdims=True)
    standardized = (raw - mean) / np.maximum(scale, 1.0e-8)
    u, singular, _ = np.linalg.svd(standardized, full_matrices=False)
    rank = min(12, standardized.shape[0], standardized.shape[1])
    conditioned = u[:, :rank] * singular[:rank]
    diagnostics = {
        "probe_states": probe_states,
        "raw_dimension": raw.shape[1],
        "conditioned_dimension": rank,
        "feature_sha256": hashlib.sha256(
            np.ascontiguousarray(raw).tobytes()
        ).hexdigest(),
        "leading_singular_values": [
            float(value) for value in singular[: min(8, len(singular))]
        ],
    }
    return conditioned, diagnostics


def _select_operator_candidates(
    candidates: list[tuple[bytes, int]],
    matrix: np.ndarray,
    *,
    budget: int,
    strategy: str,
) -> tuple[list[int], list[dict[str, Any]]]:
    if budget <= 0 or not candidates:
        return [], []
    norms = np.linalg.norm(matrix, axis=1)
    frequency = np.asarray(
        [count * (len(surface) - 1) for surface, count in candidates], dtype=float
    )
    utility = np.log1p(frequency) * (0.25 + norms)
    selected: list[int] = []
    receipts: list[dict[str, Any]] = []
    remaining = set(range(len(candidates)))
    while remaining and len(selected) < budget:
        scores: list[tuple[float, int, float]] = []
        for index in remaining:
            if selected:
                novelty = min(
                    float(np.linalg.norm(matrix[index] - matrix[other]))
                    for other in selected
                )
            else:
                novelty = float(norms[index])
            if strategy == "utility":
                score = float(utility[index] * (1.0 + 0.15 * novelty))
            elif strategy == "diversity":
                score = float(
                    novelty * (1.0 + 0.05 * math.log1p(frequency[index]))
                )
            else:
                raise ValueError("unsupported operator selection strategy")
            scores.append((score, index, novelty))
        score, chosen, novelty = min(
            scores, key=lambda item: (-item[0], candidates[item[1]][0])
        )
        remaining.remove(chosen)
        selected.append(chosen)
        surface, count = candidates[chosen]
        receipts.append(
            {
                "surface_hex": surface.hex(),
                "surface_ascii": surface.decode("ascii"),
                "count": count,
                "operator_norm": float(norms[chosen]),
                "novelty": novelty,
                "score": score,
            }
        )
    return selected, receipts


def _train_operator(
    corpus: Corpus,
    model: BytePredictionModel,
    *,
    target_size: int,
    loading: bool,
    seed: int,
    probe_states: int,
    strategy: str,
) -> Vocabulary:
    macro_budget = max(target_size - len(corpus.alphabet) - 1, 0)
    candidates = enumerate_candidates(corpus.train_surfaces)
    matrix, diagnostics = _operator_feature_matrix(
        corpus,
        model,
        candidates,
        loading=loading,
        probe_states=probe_states,
        seed=seed,
    )
    selected_indices, receipts = _select_operator_candidates(
        candidates, matrix, budget=macro_budget, strategy=strategy
    )
    selected = [candidates[index][0] for index in selected_indices]
    return _make_vocabulary(
        f"operator-{strategy}-{target_size}",
        corpus.alphabet,
        selected,
        metadata={
            "method": f"operator_{strategy}",
            "target_size": target_size,
            "loading": loading,
            "selection": receipts,
            "operator_features": diagnostics,
        },
    )


def train_operator_cover(
    corpus: Corpus,
    model: BytePredictionModel,
    *,
    target_size: int,
    loading: bool,
    seed: int,
    probe_states: int,
) -> Vocabulary:
    return _train_operator(
        corpus,
        model,
        target_size=target_size,
        loading=loading,
        seed=seed,
        probe_states=probe_states,
        strategy="utility",
    )


def train_operator_diversity_cover(
    corpus: Corpus,
    model: BytePredictionModel,
    *,
    target_size: int,
    loading: bool,
    seed: int,
    probe_states: int,
) -> Vocabulary:
    return _train_operator(
        corpus,
        model,
        target_size=target_size,
        loading=loading,
        seed=seed,
        probe_states=probe_states,
        strategy="diversity",
    )


def vocabulary_diagnostics(vocabulary: Vocabulary) -> dict[str, Any]:
    surfaces = {
        entry.surface for entry in vocabulary.entries if entry.surface is not None
    }
    motif_hits = [motif.decode("ascii") for motif in MOTIFS if motif in surfaces]
    return {
        "vocabulary": vocabulary.name,
        "vocabulary_size": vocabulary.size,
        "lexicon_surface_bytes": sum(len(surface) for surface in surfaces),
        "macro_entries": sum(len(surface) > 1 for surface in surfaces),
        "oracle_motif_hits": motif_hits,
        "oracle_motif_recall": len(motif_hits) / len(MOTIFS),
        "metadata": vocabulary.metadata,
    }
