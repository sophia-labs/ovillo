"""Controlled task generators for Extension Generation 3."""

from __future__ import annotations

from dataclasses import dataclass

import torch


@dataclass(slots=True)
class TaskBatch:
    drives: torch.Tensor
    labels: torch.Tensor


def generate_joint_batch(
    batch_size: int,
    *,
    generator: torch.Generator,
    device: torch.device | str = "cpu",
    sequence_length: int = 5,
) -> TaskBatch:
    """Generate a joint ordered-path and positive-evidence task.

    Path examples are ``A,B,_,_,A`` versus ``B,A,_,_,A``. They have the same
    token multiset and terminal token, so a bag statistic or last-token shortcut
    cannot solve the label. Evidence examples inject a positive token into one
    of two sites at every step; the second label is the five-bit majority.
    """

    if sequence_length != 5:
        raise ValueError("the registered construction fixture uses five steps")
    device = torch.device(device)
    drives = torch.zeros(batch_size, sequence_length, 4, 3, dtype=torch.float64, device=device)

    order = torch.randint(0, 2, (batch_size,), generator=generator, device=device)
    rows = torch.arange(batch_size, device=device)
    first_source = torch.where(order == 0, torch.zeros_like(order), torch.ones_like(order))
    second_source = 1 - first_source
    drives[rows, 0, first_source, 0] = 0.8
    drives[rows, 1, second_source, 0] = 0.8
    drives[:, 4, 0, 0] = 0.8

    evidence_bits = torch.randint(
        0,
        2,
        (batch_size, sequence_length),
        generator=generator,
        device=device,
    )
    for step in range(sequence_length):
        zero_rows = rows[evidence_bits[:, step] == 0]
        one_rows = rows[evidence_bits[:, step] == 1]
        drives[zero_rows, step, 2, 1] = 0.35
        drives[one_rows, step, 3, 1] = 0.35
    majority = (evidence_bits.sum(dim=1) >= 3).to(torch.float64)
    labels = torch.stack((order.to(torch.float64), majority), dim=-1)
    return TaskBatch(drives, labels)
