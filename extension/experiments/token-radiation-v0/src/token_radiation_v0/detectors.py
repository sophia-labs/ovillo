"""Vocabulary detector families over a shared predictive radiation field."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

import torch
from torch import nn
import torch.nn.functional as F

DetectorKind = Literal["linear", "metric", "positive_frame"]


class LinearDetector(nn.Module):
    def __init__(self, radiation_rank: int, vocabulary_size: int):
        super().__init__()
        self.projection = nn.Linear(
            radiation_rank, vocabulary_size, dtype=torch.float64
        )

    def forward(self, radiation: torch.Tensor) -> torch.Tensor:
        return self.projection(radiation)


class MetricDetector(nn.Module):
    def __init__(self, radiation_rank: int, vocabulary_size: int):
        super().__init__()
        generator = torch.Generator().manual_seed(3407 + vocabulary_size)
        self.centers = nn.Parameter(
            0.2
            * torch.randn(
                vocabulary_size,
                radiation_rank,
                generator=generator,
                dtype=torch.float64,
            )
        )
        self.bias = nn.Parameter(torch.zeros(vocabulary_size, dtype=torch.float64))
        self.raw_temperature = nn.Parameter(torch.tensor(-0.5, dtype=torch.float64))

    def forward(self, radiation: torch.Tensor) -> torch.Tensor:
        difference = radiation[..., None, :] - self.centers
        temperature = F.softplus(self.raw_temperature) + 0.05
        return -0.5 * difference.square().sum(dim=-1) / temperature + self.bias


class PositiveFrameDetector(nn.Module):
    """Positive token intensities from an overcomplete tight frame."""

    def __init__(self, radiation_rank: int, vocabulary_size: int):
        super().__init__()
        if vocabulary_size < radiation_rank:
            raise ValueError("positive frame requires vocabulary_size >= radiation_rank")
        generator = torch.Generator().manual_seed(7817 + vocabulary_size)
        self.raw_frame = nn.Parameter(
            torch.randn(
                vocabulary_size,
                radiation_rank,
                generator=generator,
                dtype=torch.float64,
            )
        )
        self.log_prior = nn.Parameter(torch.zeros(vocabulary_size, dtype=torch.float64))

    def frame(self) -> torch.Tensor:
        q, r = torch.linalg.qr(self.raw_frame, mode="reduced")
        diagonal = torch.diagonal(r)
        signs = torch.where(
            diagonal.detach() < 0,
            -torch.ones_like(diagonal),
            torch.ones_like(diagonal),
        )
        return q * signs

    def contextual_intensity(self, radiation: torch.Tensor) -> torch.Tensor:
        return torch.einsum("...k,vk->...v", radiation, self.frame()).square()

    def frame_residual(self, radiation: torch.Tensor) -> torch.Tensor:
        contextual = self.contextual_intensity(radiation).sum(dim=-1)
        return contextual - radiation.square().sum(dim=-1)

    def forward(self, radiation: torch.Tensor) -> torch.Tensor:
        intensity = self.contextual_intensity(radiation)
        return torch.log(intensity + 1e-10) + self.log_prior


@dataclass(frozen=True, slots=True)
class DetectorDescription:
    kind: DetectorKind
    radiation_rank: int
    vocabulary_size: int


def make_detector(
    kind: DetectorKind, radiation_rank: int, vocabulary_size: int
) -> nn.Module:
    if kind == "linear":
        return LinearDetector(radiation_rank, vocabulary_size)
    if kind == "metric":
        return MetricDetector(radiation_rank, vocabulary_size)
    if kind == "positive_frame":
        return PositiveFrameDetector(radiation_rank, vocabulary_size)
    raise ValueError(f"unknown detector kind {kind!r}")
