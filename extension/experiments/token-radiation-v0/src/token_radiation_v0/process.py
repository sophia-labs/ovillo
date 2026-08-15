"""Synthetic byte process with latent macro structure and order-sensitive futures."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import json
from typing import Iterable

import numpy as np

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
    sequence_bytes: int = 48
    train_sequences: int = 96
    validation_sequences: int = 32
    seed: int = 20260814

    def validate(self) -> None:
        if self.sequence_bytes < 8:
            raise ValueError("sequence_bytes must be at least eight")
        if self.train_sequences < 1 or self.validation_sequences < 1:
            raise ValueError("corpus partitions must be nonempty")


@dataclass(frozen=True, slots=True)
class Corpus:
    train: tuple[bytes, ...]
    validation: tuple[bytes, ...]
    alphabet: tuple[int, ...]
    config: ProcessConfig

    def encode(self, sequence: bytes) -> list[int]:
        index = {value: position for position, value in enumerate(self.alphabet)}
        return [index[value] for value in sequence]

    def decode(self, ids: Iterable[int]) -> bytes:
        return bytes(self.alphabet[int(value)] for value in ids)

    def to_dict(self) -> dict[str, object]:
        return {
            "format": "sophia.instrument.token-radiation.synthetic-corpus/v1",
            "config": {
                "sequence_bytes": self.config.sequence_bytes,
                "train_sequences": self.config.train_sequences,
                "validation_sequences": self.config.validation_sequences,
                "seed": self.config.seed,
            },
            "alphabet": list(self.alphabet),
            "motifs": [motif.decode("ascii") for motif in MOTIFS],
            "train": [sequence.decode("ascii") for sequence in self.train],
            "validation": [sequence.decode("ascii") for sequence in self.validation],
        }

    def write(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(self.to_dict(), indent=2, sort_keys=True) + "\n")


def _sample_sequence(rng: np.random.Generator, length: int) -> bytes:
    state = int(rng.integers(0, MOTIF_PROBABILITIES.shape[0]))
    result = bytearray()
    while len(result) < length:
        motif_index = int(
            rng.choice(len(MOTIFS), p=MOTIF_PROBABILITIES[state])
        )
        result.extend(MOTIFS[motif_index])
        state = int(NEXT_STATE[state, motif_index])
    return bytes(result[:length])


def generate_corpus(config: ProcessConfig = ProcessConfig()) -> Corpus:
    config.validate()
    rng = np.random.default_rng(config.seed)
    total = config.train_sequences + config.validation_sequences
    sequences = tuple(_sample_sequence(rng, config.sequence_bytes) for _ in range(total))
    alphabet = tuple(sorted({value for sequence in sequences for value in sequence}))
    return Corpus(
        train=sequences[: config.train_sequences],
        validation=sequences[config.train_sequences :],
        alphabet=alphabet,
        config=config,
    )
