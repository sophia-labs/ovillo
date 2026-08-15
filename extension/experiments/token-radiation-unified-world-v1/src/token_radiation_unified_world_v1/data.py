"""Controlled surface/operator world for shared-Ariadne communication tests."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Final

import numpy as np
import torch

R: Final[int] = 0
L: Final[int] = 1
F: Final[int] = 2
T: Final[int] = 3
OP_NAMES: Final[tuple[str, ...]] = ("R", "L", "F", "T")
PAD_BYTE: Final[int] = 256
BOS_BYTE: Final[int] = 257
BYTE_ALPHABET: Final[int] = 258

# The exact bytes b"ba" are deliberately ambiguous: R in mode 0, F in mode 1.
SURFACES: Final[dict[tuple[int, int], tuple[bytes, ...]]] = {
    (0, R): (b"ba", b"ra", b"arx"),
    (0, L): (b"la", b"al", b"lex"),
    (0, F): (b"fa", b"af", b"fox"),
    (0, T): (b"ta", b"at", b"tog"),
    (1, R): (b"ri", b"ir", b"rox"),
    (1, L): (b"li", b"il", b"lox"),
    (1, F): (b"ba", b"fe", b"fex"),
    (1, T): (b"ti", b"it", b"tig"),
}


@dataclass(frozen=True, slots=True)
class LatentState:
    rotation: int
    reflected: int
    mode: int

    def packed(self) -> int:
        return self.rotation + 4 * self.reflected + 8 * self.mode


@dataclass(slots=True)
class Batch:
    byte_ids: torch.Tensor
    byte_mask: torch.Tensor
    event_byte_ids: torch.Tensor
    event_byte_mask: torch.Tensor
    event_end: torch.Tensor
    operators: torch.Tensor
    next_operators: torch.Tensor
    modes: torch.Tensor
    initial_states: torch.Tensor
    surface_variant: torch.Tensor

    @property
    def batch_size(self) -> int:
        return int(self.byte_ids.shape[0])

    @property
    def event_count(self) -> int:
        return int(self.operators.shape[1])


def apply_operator(state: LatentState, operator: int) -> LatentState:
    r, reflected, mode = state.rotation, state.reflected, state.mode
    if operator == R:
        r = (r + (-1 if reflected else 1)) % 4
    elif operator == L:
        r = (r + (1 if reflected else -1)) % 4
    elif operator == F:
        reflected ^= 1
        r = (-r) % 4
    elif operator == T:
        mode ^= 1
    else:
        raise ValueError(f"unknown operator {operator}")
    return LatentState(r, reflected, mode)


def policy(state: LatentState, event_index: int) -> int:
    # Uses the non-commutative group state, persistent lexical mode, and event phase.
    value = (
        state.rotation
        + 2 * state.reflected
        + 3 * state.mode
        + (event_index % 3)
        + (state.rotation * state.mode)
    ) % 4
    return (R, F, L, T)[value]


def generate_batch(
    *,
    seed: int,
    batch_size: int,
    events: int,
    device: torch.device | str = "cpu",
) -> Batch:
    rng = np.random.default_rng(seed)
    sequences: list[list[int]] = []
    event_surfaces: list[list[list[int]]] = []
    event_ends: list[list[int]] = []
    operators: list[list[int]] = []
    next_operators: list[list[int]] = []
    modes: list[list[int]] = []
    initial_states: list[int] = []
    variants: list[list[int]] = []

    for _ in range(batch_size):
        state = LatentState(
            rotation=int(rng.integers(0, 4)),
            reflected=int(rng.integers(0, 2)),
            mode=int(rng.integers(0, 2)),
        )
        initial_states.append(state.packed())
        byte_sequence: list[int] = []
        ends: list[int] = []
        ops: list[int] = []
        next_ops: list[int] = []
        event_modes: list[int] = []
        event_variants: list[int] = []
        event_surface_row: list[list[int]] = []
        for event_index in range(events):
            op = policy(state, event_index)
            forms = SURFACES[(state.mode, op)]
            variant = int(rng.integers(0, len(forms)))
            surface = forms[variant]
            event_modes.append(state.mode)
            ops.append(op)
            event_variants.append(variant)
            surface_values = list(surface)
            event_surface_row.append(surface_values)
            byte_sequence.extend(surface_values)
            ends.append(len(byte_sequence) - 1)
            state = apply_operator(state, op)
            next_ops.append(policy(state, event_index + 1))
        sequences.append(byte_sequence)
        event_surfaces.append(event_surface_row)
        event_ends.append(ends)
        operators.append(ops)
        next_operators.append(next_ops)
        modes.append(event_modes)
        variants.append(event_variants)

    max_bytes = max(map(len, sequences))
    max_surface = max(len(surface) for row in event_surfaces for surface in row)
    byte_ids = np.full((batch_size, max_bytes), PAD_BYTE, dtype=np.int64)
    byte_mask = np.zeros((batch_size, max_bytes), dtype=np.bool_)
    event_byte_ids = np.full((batch_size, events, max_surface), PAD_BYTE, dtype=np.int64)
    event_byte_mask = np.zeros((batch_size, events, max_surface), dtype=np.bool_)
    for index, sequence in enumerate(sequences):
        byte_ids[index, : len(sequence)] = sequence
        byte_mask[index, : len(sequence)] = True
        for event_index, surface in enumerate(event_surfaces[index]):
            event_byte_ids[index, event_index, : len(surface)] = surface
            event_byte_mask[index, event_index, : len(surface)] = True

    return Batch(
        byte_ids=torch.as_tensor(byte_ids, device=device),
        byte_mask=torch.as_tensor(byte_mask, device=device),
        event_byte_ids=torch.as_tensor(event_byte_ids, device=device),
        event_byte_mask=torch.as_tensor(event_byte_mask, device=device),
        event_end=torch.as_tensor(event_ends, dtype=torch.long, device=device),
        operators=torch.as_tensor(operators, dtype=torch.long, device=device),
        next_operators=torch.as_tensor(next_operators, dtype=torch.long, device=device),
        modes=torch.as_tensor(modes, dtype=torch.long, device=device),
        initial_states=torch.as_tensor(initial_states, dtype=torch.long, device=device),
        surface_variant=torch.as_tensor(variants, dtype=torch.long, device=device),
    )


def homograph_mask(batch: Batch) -> torch.Tensor:
    """Events whose observed surface is the ambiguous bytes b'ba'."""

    rows: list[list[bool]] = []
    for sequence_index in range(batch.batch_size):
        row: list[bool] = []
        start = 0
        for end in batch.event_end[sequence_index].tolist():
            surface = bytes(batch.byte_ids[sequence_index, start : end + 1].tolist())
            row.append(surface == b"ba")
            start = end + 1
        rows.append(row)
    return torch.tensor(rows, dtype=torch.bool, device=batch.byte_ids.device)
