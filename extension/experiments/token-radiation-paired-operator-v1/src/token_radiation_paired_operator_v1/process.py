"""Synthetic byte processes with explicit latent motif boundaries."""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
from typing import Iterable, Literal

import numpy as np

ProcessFamily = Literal["structured", "iid_motifs"]

MOTIFS: tuple[bytes, ...] = (
    b"ab",
    b"ba",
    b"aca",
    b"bdb",
    b"ce",
    b"ec",
    b"dad",
    b"cbc",
)

MOTIF_PROBABILITIES = np.asarray(
    [
        [0.38, 0.08, 0.24, 0.04, 0.14, 0.04, 0.05, 0.03],
        [0.08, 0.38, 0.04, 0.24, 0.04, 0.14, 0.03, 0.05],
        [0.10, 0.06, 0.12, 0.05, 0.34, 0.12, 0.15, 0.06],
        [0.06, 0.10, 0.05, 0.12, 0.12, 0.34, 0.06, 0.15],
    ],
    dtype=np.float64,
)
IID_MOTIF_PROBABILITIES = MOTIF_PROBABILITIES.mean(axis=0)
IID_MOTIF_PROBABILITIES /= IID_MOTIF_PROBABILITIES.sum()
NEXT_STATE = np.asarray(
    [
        [1, 3, 2, 1, 3, 2, 0, 1],
        [2, 0, 3, 3, 1, 2, 0, 1],
        [3, 1, 2, 0, 3, 1, 0, 2],
        [0, 2, 1, 3, 2, 0, 1, 3],
    ],
    dtype=np.int64,
)


@dataclass(frozen=True, slots=True)
class ProcessConfig:
    sequence_bytes: int = 32
    train_sequences: int = 48
    validation_sequences: int = 16
    seed: int = 20260814
    family: ProcessFamily = "structured"

    def validate(self) -> None:
        if self.sequence_bytes < 8:
            raise ValueError("sequence_bytes must be at least eight")
        if self.train_sequences < 1 or self.validation_sequences < 1:
            raise ValueError("corpus partitions must be nonempty")
        if self.family not in {"structured", "iid_motifs"}:
            raise ValueError("unsupported process family")


@dataclass(frozen=True, slots=True)
class Example:
    surface: bytes
    motif_indices: tuple[int, ...]
    motif_boundaries: tuple[int, ...]


@dataclass(frozen=True, slots=True)
class Corpus:
    train: tuple[Example, ...]
    validation: tuple[Example, ...]
    alphabet: tuple[int, ...]
    config: ProcessConfig

    @property
    def train_surfaces(self) -> tuple[bytes, ...]:
        return tuple(example.surface for example in self.train)

    @property
    def validation_surfaces(self) -> tuple[bytes, ...]:
        return tuple(example.surface for example in self.validation)

    def encode(self, sequence: bytes) -> list[int]:
        index = {value: position for position, value in enumerate(self.alphabet)}
        return [index[value] for value in sequence]

    def decode(self, ids: Iterable[int]) -> bytes:
        return bytes(self.alphabet[int(value)] for value in ids)

    def to_dict(self) -> dict[str, object]:
        def example_dict(example: Example) -> dict[str, object]:
            return {
                "surface": example.surface.decode("ascii"),
                "motif_indices": list(example.motif_indices),
                "motif_boundaries": list(example.motif_boundaries),
            }

        return {
            "format": "sophia.instrument.token-radiation.synthetic-corpus/v2",
            "config": {
                "sequence_bytes": self.config.sequence_bytes,
                "train_sequences": self.config.train_sequences,
                "validation_sequences": self.config.validation_sequences,
                "seed": self.config.seed,
                "family": self.config.family,
            },
            "alphabet": list(self.alphabet),
            "motifs": [motif.decode("ascii") for motif in MOTIFS],
            "train": [example_dict(example) for example in self.train],
            "validation": [example_dict(example) for example in self.validation],
        }

    def write(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(self.to_dict(), indent=2, sort_keys=True) + "\n")


def _sample_example(rng: np.random.Generator, config: ProcessConfig) -> Example:
    state = int(rng.integers(0, MOTIF_PROBABILITIES.shape[0]))
    result = bytearray()
    motif_indices: list[int] = []
    boundaries = [0]
    while len(result) < config.sequence_bytes:
        probabilities = (
            MOTIF_PROBABILITIES[state]
            if config.family == "structured"
            else IID_MOTIF_PROBABILITIES
        )
        motif_index = int(rng.choice(len(MOTIFS), p=probabilities))
        motif = MOTIFS[motif_index]
        remaining = config.sequence_bytes - len(result)
        result.extend(motif[:remaining])
        motif_indices.append(motif_index)
        boundaries.append(len(result))
        if config.family == "structured":
            state = int(NEXT_STATE[state, motif_index])
    return Example(
        surface=bytes(result),
        motif_indices=tuple(motif_indices),
        motif_boundaries=tuple(boundaries),
    )


def generate_corpus(config: ProcessConfig = ProcessConfig()) -> Corpus:
    config.validate()
    rng = np.random.default_rng(config.seed)
    total = config.train_sequences + config.validation_sequences
    examples = tuple(_sample_example(rng, config) for _ in range(total))
    alphabet = tuple(sorted({value for example in examples for value in example.surface}))
    return Corpus(
        train=examples[: config.train_sequences],
        validation=examples[config.train_sequences :],
        alphabet=alphabet,
        config=config,
    )
