"""Byte-driven local constitutive body with a loading predictive radiator."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

import torch
from torch import nn
import torch.nn.functional as F

from .radiation import metric_radiate_torch, orthonormal_rows

RadiationMode = Literal["passive", "nonloading"]


@dataclass(frozen=True, slots=True)
class BodyConfig:
    alphabet_size: int
    sites: int = 12
    radiation_rank: int = 8
    radiation_fraction: float = 0.04
    radiation_mode: RadiationMode = "passive"
    microsteps_per_byte: int = 1
    edges: tuple[tuple[int, int], ...] | None = None

    def resolved_edges(self) -> tuple[tuple[int, int], ...]:
        if self.edges is not None:
            return self.edges
        ring = [(index, (index + 1) % self.sites) for index in range(self.sites)]
        skips = [(index, (index + 3) % self.sites) for index in range(0, self.sites, 2)]
        return tuple(ring + skips)

    def validate(self) -> None:
        if self.alphabet_size < 2:
            raise ValueError("alphabet_size must be at least two")
        if self.sites < 2:
            raise ValueError("sites must be at least two")
        if not 1 <= self.radiation_rank <= self.sites:
            raise ValueError("radiation_rank must be in [1, sites]")
        if not 0.0 <= self.radiation_fraction <= 1.0:
            raise ValueError("radiation_fraction must be in [0,1]")
        if self.radiation_mode not in {"passive", "nonloading"}:
            raise ValueError("unsupported radiation_mode")
        if self.microsteps_per_byte < 1:
            raise ValueError("microsteps_per_byte must be positive")
        for left, right in self.resolved_edges():
            if left == right or not 0 <= left < self.sites or not 0 <= right < self.sites:
                raise ValueError("invalid local edge")


@dataclass(slots=True)
class BodyTrace:
    emissions: torch.Tensor
    states: torch.Tensor
    radiation: torch.Tensor
    projected_intensity: torch.Tensor
    closure_residual: torch.Tensor


class RadiatingByteBody(nn.Module):
    """A constrained local body driven by canonical byte gestures."""

    def __init__(self, config: BodyConfig):
        super().__init__()
        config.validate()
        self.config = config
        self.edges = config.resolved_edges()
        n = config.sites
        e = len(self.edges)
        inputs = config.alphabet_size + 1

        self.raw_metric = nn.Parameter(torch.linspace(-0.2, 0.2, n, dtype=torch.float64))
        self.raw_wave_decay = nn.Parameter(torch.full((n,), -5.0, dtype=torch.float64))
        self.raw_evidence_decay = nn.Parameter(torch.full((n,), -5.0, dtype=torch.float64))
        self.raw_wave_rate = nn.Parameter(torch.linspace(-0.4, 0.4, e, dtype=torch.float64))
        self.raw_evidence_rate = nn.Parameter(torch.linspace(-0.3, 0.3, e, dtype=torch.float64))

        generator = torch.Generator().manual_seed(20260814)
        source_wave = 0.18 * torch.randn(inputs, n, generator=generator, dtype=torch.float64)
        source_evidence = -2.0 + 0.15 * torch.randn(
            inputs, n, generator=generator, dtype=torch.float64
        )
        self.source_wave = nn.Parameter(source_wave)
        self.raw_source_evidence = nn.Parameter(source_evidence)
        self.raw_aperture_rows = nn.Parameter(
            torch.randn(
                config.radiation_rank, n, generator=generator, dtype=torch.float64
            )
        )

    @property
    def bos_id(self) -> int:
        return self.config.alphabet_size

    def constrained(self) -> dict[str, torch.Tensor]:
        return {
            "metric": F.softplus(self.raw_metric) + 0.35,
            "wave_decay": F.softplus(self.raw_wave_decay),
            "evidence_decay": F.softplus(self.raw_evidence_decay),
            "wave_rate": 20.0 * torch.tanh(self.raw_wave_rate),
            "evidence_rate": 10.0 * torch.sigmoid(self.raw_evidence_rate),
            "source_evidence": F.softplus(self.raw_source_evidence),
            "aperture_rows": orthonormal_rows(self.raw_aperture_rows),
        }

    def _microstep(
        self,
        wave: torch.Tensor,
        evidence: torch.Tensor,
        params: dict[str, torch.Tensor],
    ) -> tuple[torch.Tensor, torch.Tensor]:
        dt = 1.0 / 32.0 / self.config.microsteps_per_byte
        for edge_index, (left, right) in enumerate(self.edges):
            left_metric = params["metric"][left]
            right_metric = params["metric"][right]
            theta = params["wave_rate"][edge_index] * dt
            cosine = torch.cos(theta)
            sine = torch.sin(theta)
            scaled_left = torch.sqrt(left_metric) * wave[:, left]
            scaled_right = torch.sqrt(right_metric) * wave[:, right]
            next_left = (cosine * scaled_left - sine * scaled_right) / torch.sqrt(left_metric)
            next_right = (sine * scaled_left + cosine * scaled_right) / torch.sqrt(right_metric)
            wave = wave.clone()
            wave[:, left] = next_left
            wave[:, right] = next_right

            mix = (params["evidence_rate"][edge_index] * dt).clamp(0.0, 0.5)
            left_evidence = evidence[:, left]
            right_evidence = evidence[:, right]
            evidence = evidence.clone()
            evidence[:, left] = (1.0 - mix) * left_evidence + mix * right_evidence
            evidence[:, right] = mix * left_evidence + (1.0 - mix) * right_evidence

        wave = wave * torch.exp(-0.5 * params["wave_decay"] * dt)
        evidence = evidence * torch.exp(-0.5 * params["evidence_decay"] * dt)
        return wave, evidence

    def forward(self, byte_ids: torch.Tensor) -> BodyTrace:
        if byte_ids.ndim != 2:
            raise ValueError("byte_ids must have shape [batch, time]")
        if byte_ids.dtype != torch.long:
            byte_ids = byte_ids.long()
        if bool(torch.any(byte_ids < 0)) or bool(
            torch.any(byte_ids >= self.config.alphabet_size)
        ):
            raise ValueError("byte id outside configured alphabet")

        params = self.constrained()
        batch = byte_ids.shape[0]
        device = byte_ids.device
        wave = torch.zeros(batch, self.config.sites, dtype=torch.float64, device=device)
        evidence = torch.zeros_like(wave)

        emissions: list[torch.Tensor] = []
        states: list[torch.Tensor] = []
        radiation: list[torch.Tensor] = []
        projected: list[torch.Tensor] = []
        residuals: list[torch.Tensor] = []

        gestures = torch.cat(
            (
                torch.full(
                    (batch, 1), self.bos_id, dtype=torch.long, device=device
                ),
                byte_ids,
            ),
            dim=1,
        )

        for position in range(gestures.shape[1]):
            ids = gestures[:, position]
            wave = wave + self.source_wave[ids]
            evidence = evidence + params["source_evidence"][ids]
            for _ in range(self.config.microsteps_per_byte):
                wave, evidence = self._microstep(wave, evidence, params)

            frame = metric_radiate_torch(
                wave,
                params["metric"],
                params["aperture_rows"],
                self.config.radiation_fraction,
                loading=self.config.radiation_mode == "passive",
            )
            wave = frame.state_after
            emissions.append(frame.emitted)
            states.append(torch.cat((wave, evidence), dim=-1))
            radiation.append(frame.booked_radiation)
            projected.append(frame.projected_intensity)
            residuals.append(frame.residual)

        return BodyTrace(
            emissions=torch.stack(emissions, dim=1),
            states=torch.stack(states, dim=1),
            radiation=torch.stack(radiation, dim=1),
            projected_intensity=torch.stack(projected, dim=1),
            closure_residual=torch.stack(residuals, dim=1),
        )


class BytePredictionModel(nn.Module):
    """Radiating body plus a deliberately simple byte detector."""

    def __init__(self, config: BodyConfig):
        super().__init__()
        self.body = RadiatingByteBody(config)
        self.radiation_head = nn.Linear(
            config.radiation_rank, config.alphabet_size + 1, dtype=torch.float64
        )

    def forward(self, byte_ids: torch.Tensor) -> tuple[torch.Tensor, BodyTrace]:
        trace = self.body(byte_ids)
        return self.radiation_head(trace.emissions), trace
