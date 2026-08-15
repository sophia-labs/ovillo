"""Batch construction for byte and event trajectories."""

from __future__ import annotations

from dataclasses import dataclass

import torch

from .world import ByteCodec, LexicalExample, NEXT_TABLE, OP_TO_ID, OPS, apply_operator


@dataclass(slots=True)
class ByteBatch:
    ids: torch.Tensor
    mask: torch.Tensor
    next_byte: torch.Tensor
    boundary: torch.Tensor
    operator: torch.Tensor
    next_operator: torch.Tensor
    mode: torch.Tensor
    group: torch.Tensor


@dataclass(slots=True)
class EventBatch:
    initial_ids: torch.Tensor
    operator_ids: torch.Tensor
    next_operator: torch.Tensor
    mode: torch.Tensor
    group: torch.Tensor


def collate_bytes(examples: list[LexicalExample], codec: ByteCodec) -> ByteBatch:
    maximum = max(len(example.surface) for example in examples)
    batch = len(examples)
    ids = torch.full((batch, maximum), codec.pad_id, dtype=torch.long)
    mask = torch.zeros((batch, maximum), dtype=torch.bool)
    next_byte = torch.full((batch, maximum + 1), -100, dtype=torch.long)
    boundary = torch.zeros((batch, maximum + 1), dtype=torch.float32)
    operator = torch.full((batch, maximum + 1), -100, dtype=torch.long)
    next_operator = torch.full_like(operator, -100)
    mode = torch.full_like(operator, -100)
    group = torch.full_like(operator, -100)

    for row, example in enumerate(examples):
        encoded = codec.encode(example.surface)
        length = len(encoded)
        ids[row, :length] = torch.tensor(encoded, dtype=torch.long)
        mask[row, :length] = True
        next_byte[row, 0] = encoded[0]
        for position in range(length):
            next_byte[row, position + 1] = (
                encoded[position + 1] if position + 1 < length else codec.eos_id
            )
            boundary[row, position + 1] = float(example.boundaries[position])
            operator[row, position + 1] = example.operator_labels[position]
            next_operator[row, position + 1] = example.next_operator_labels[position]
            mode[row, position + 1] = example.mode_labels[position]
            group[row, position + 1] = example.group_labels[position]
        initial_rotation, initial_reflected, initial_mode = example.initial_state
        mode[row, 0] = initial_mode
        group[row, 0] = initial_rotation + 4 * initial_reflected

    return ByteBatch(
        ids=ids,
        mask=mask,
        next_byte=next_byte,
        boundary=boundary,
        operator=operator,
        next_operator=next_operator,
        mode=mode,
        group=group,
    )


def collate_events(examples: list[LexicalExample]) -> EventBatch:
    event_count = len(examples[0].latent_operators)
    if any(len(example.latent_operators) != event_count for example in examples):
        raise ValueError("event batches require equal event counts")
    batch = len(examples)
    operator_ids = torch.empty((batch, event_count), dtype=torch.long)
    next_operator = torch.empty((batch, event_count + 1), dtype=torch.long)
    mode = torch.empty_like(next_operator)
    group = torch.empty_like(next_operator)
    initial_ids = torch.empty(batch, dtype=torch.long)

    for row, example in enumerate(examples):
        state = example.initial_state
        initial_ids[row] = state[0] + 4 * state[1] + 8 * state[2]
        operator_ids[row] = torch.tensor(example.latent_operators)
        states = [state]
        for operator_id in example.latent_operators:
            state = apply_operator(state, OPS[operator_id])
            states.append(state)
        next_operator[row] = torch.tensor(
            [OP_TO_ID[NEXT_TABLE[item]] for item in states]
        )
        mode[row] = torch.tensor([item[2] for item in states])
        group[row] = torch.tensor([item[0] + 4 * item[1] for item in states])

    return EventBatch(
        initial_ids=initial_ids,
        operator_ids=operator_ids,
        next_operator=next_operator,
        mode=mode,
        group=group,
    )
