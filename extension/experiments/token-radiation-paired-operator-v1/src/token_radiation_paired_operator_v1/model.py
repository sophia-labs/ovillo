"""Byte-driven local body with calibrated predictive radiation coordinates."""

from __future__ import annotations

from dataclasses import dataclass

import torch
from torch import nn
import torch.nn.functional as F

from .radiation import metric_radiate, orthonormal_rows


@dataclass(frozen=True, slots=True)
class BodyConfig:
    alphabet_size: int
    sites: int = 8
    radiation_rank: int = 4
    radiation_fraction: float = 0.04

    def validate(self) -> None:
        if self.alphabet_size < 2:
            raise ValueError("alphabet_size must be at least two")
        if self.sites < 2:
            raise ValueError("sites must be at least two")
        if not 1 <= self.radiation_rank <= self.sites:
            raise ValueError("radiation_rank must be in [1, sites]")
        if not 0.0 <= self.radiation_fraction <= 1.0:
            raise ValueError("radiation_fraction must lie in [0,1]")


@dataclass(slots=True)
class BodyTrace:
    coordinates: torch.Tensor
    emissions: torch.Tensor
    states: torch.Tensor
    waves: torch.Tensor
    evidence: torch.Tensor
    radiation: torch.Tensor
    projected_intensity: torch.Tensor
    closure_residual: torch.Tensor


class RadiatingByteBody(nn.Module):
    """A small local constitutive body; no macro-token enters its input."""

    def __init__(self, config: BodyConfig):
        super().__init__()
        config.validate()
        self.config = config
        n = config.sites
        inputs = config.alphabet_size + 1
        edge_pairs = [(i, (i + 1) % n) for i in range(n)]
        edge_pairs += [(i, (i + 3) % n) for i in range(0, n, 2)]
        self.edges = tuple(edge_pairs)

        self.raw_metric = nn.Parameter(torch.linspace(-0.2, 0.2, n, dtype=torch.float64))
        self.raw_decay = nn.Parameter(torch.full((n,), -3.5, dtype=torch.float64))
        self.raw_evidence_decay = nn.Parameter(torch.full((n,), -2.5, dtype=torch.float64))
        self.raw_edge_angles = nn.Parameter(
            torch.linspace(-0.35, 0.35, len(self.edges), dtype=torch.float64)
        )
        generator = torch.Generator().manual_seed(20260814)
        self.source_wave = nn.Parameter(
            0.16 * torch.randn(inputs, n, generator=generator, dtype=torch.float64)
        )
        self.raw_source_evidence = nn.Parameter(
            -1.5 + 0.18 * torch.randn(inputs, n, generator=generator, dtype=torch.float64)
        )
        self.raw_aperture_rows = nn.Parameter(
            torch.randn(config.radiation_rank, n, generator=generator, dtype=torch.float64)
        )

    @property
    def bos_id(self) -> int:
        return self.config.alphabet_size

    def constrained(self) -> dict[str, torch.Tensor]:
        return {
            "metric": F.softplus(self.raw_metric) + 0.4,
            "decay": F.softplus(self.raw_decay),
            "evidence_decay": F.softplus(self.raw_evidence_decay),
            "angles": 0.35 * torch.tanh(self.raw_edge_angles),
            "source_evidence": F.softplus(self.raw_source_evidence),
            "aperture_rows": orthonormal_rows(self.raw_aperture_rows),
        }

    def _transport(
        self,
        wave: torch.Tensor,
        evidence: torch.Tensor,
        params: dict[str, torch.Tensor],
    ) -> tuple[torch.Tensor, torch.Tensor]:
        metric = params["metric"]
        q = wave * metric.sqrt()
        for edge_index, (left, right) in enumerate(self.edges):
            theta = params["angles"][edge_index]
            cosine = torch.cos(theta)
            sine = torch.sin(theta)
            q_left = q[:, left]
            q_right = q[:, right]
            next_left = cosine * q_left - sine * q_right
            next_right = sine * q_left + cosine * q_right
            q = q.clone()
            q[:, left] = next_left
            q[:, right] = next_right
        wave = q / metric.sqrt()
        wave = wave * torch.exp(-0.04 * params["decay"])
        evidence = evidence * torch.exp(-0.04 * params["evidence_decay"])
        evidence = (
            0.82 * evidence
            + 0.09 * torch.roll(evidence, 1, dims=-1)
            + 0.09 * torch.roll(evidence, -1, dims=-1)
        )
        return wave, evidence

    def _step(
        self,
        wave: torch.Tensor,
        evidence: torch.Tensor,
        ids: torch.Tensor,
        params: dict[str, torch.Tensor],
        *,
        loading: bool,
    ) -> tuple[torch.Tensor, torch.Tensor, dict[str, torch.Tensor]]:
        wave = wave + self.source_wave[ids]
        evidence = evidence + params["source_evidence"][ids]
        wave, evidence = self._transport(wave, evidence, params)
        frame = metric_radiate(
            wave,
            params["metric"],
            params["aperture_rows"],
            self.config.radiation_fraction,
            loading=loading,
        )
        wave = frame.state_after
        values = {
            "coordinates": frame.coordinates,
            "emitted": frame.emitted,
            "radiation": frame.booked_radiation,
            "projected": frame.projected_intensity,
            "residual": frame.residual,
        }
        return wave, evidence, values

    def forward(self, byte_ids: torch.Tensor, *, loading: bool) -> BodyTrace:
        event_count = byte_ids.shape[1] + 1
        return self.forward_with_loading_mask(
            byte_ids, loading_mask=tuple(loading for _ in range(event_count))
        )

    def forward_with_loading_mask(
        self, byte_ids: torch.Tensor, *, loading_mask: tuple[bool, ...]
    ) -> BodyTrace:
        if byte_ids.ndim != 2:
            raise ValueError("byte_ids must have shape [batch,time]")
        if byte_ids.dtype != torch.long:
            byte_ids = byte_ids.long()
        if len(loading_mask) != byte_ids.shape[1] + 1:
            raise ValueError("loading_mask must cover BOS and every byte boundary")
        params = self.constrained()
        batch = byte_ids.shape[0]
        wave = torch.zeros(
            batch, self.config.sites, dtype=torch.float64, device=byte_ids.device
        )
        evidence = torch.zeros_like(wave)
        gestures = torch.cat(
            (
                torch.full(
                    (batch, 1), self.bos_id, dtype=torch.long, device=byte_ids.device
                ),
                byte_ids,
            ),
            dim=1,
        )
        coordinates: list[torch.Tensor] = []
        emissions: list[torch.Tensor] = []
        states: list[torch.Tensor] = []
        waves: list[torch.Tensor] = []
        evidences: list[torch.Tensor] = []
        radiation: list[torch.Tensor] = []
        projected: list[torch.Tensor] = []
        residuals: list[torch.Tensor] = []
        for position in range(gestures.shape[1]):
            wave, evidence, values = self._step(
                wave,
                evidence,
                gestures[:, position],
                params,
                loading=loading_mask[position],
            )
            coordinates.append(values["coordinates"])
            emissions.append(values["emitted"])
            waves.append(wave)
            evidences.append(evidence)
            states.append(torch.cat((wave, evidence), dim=-1))
            radiation.append(values["radiation"])
            projected.append(values["projected"])
            residuals.append(values["residual"])
        return BodyTrace(
            coordinates=torch.stack(coordinates, dim=1),
            emissions=torch.stack(emissions, dim=1),
            states=torch.stack(states, dim=1),
            waves=torch.stack(waves, dim=1),
            evidence=torch.stack(evidences, dim=1),
            radiation=torch.stack(radiation, dim=1),
            projected_intensity=torch.stack(projected, dim=1),
            closure_residual=torch.stack(residuals, dim=1),
        )

    def advance_ids(
        self,
        wave: torch.Tensor,
        evidence: torch.Tensor,
        ids: torch.Tensor,
        *,
        loading: bool,
    ) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor]:
        """Apply a byte span to supplied body states for operator studies."""

        params = self.constrained()
        coordinate_sum = torch.zeros(
            wave.shape[0],
            self.config.radiation_rank,
            dtype=wave.dtype,
            device=wave.device,
        )
        intensity_sum = torch.zeros(
            wave.shape[0], dtype=wave.dtype, device=wave.device
        )
        for position in range(ids.shape[1]):
            wave, evidence, values = self._step(
                wave, evidence, ids[:, position], params, loading=loading
            )
            coordinate_sum = coordinate_sum + values["coordinates"]
            intensity_sum = intensity_sum + values["projected"]
        return wave, evidence, coordinate_sum, intensity_sum


class BytePredictionModel(nn.Module):
    def __init__(self, config: BodyConfig):
        super().__init__()
        self.body = RadiatingByteBody(config)
        self.head = nn.Linear(
            config.radiation_rank, config.alphabet_size + 1, dtype=torch.float64
        )

    def logits_from_coordinates(self, coordinates: torch.Tensor) -> torch.Tensor:
        return self.head(coordinates)

    def forward(
        self, byte_ids: torch.Tensor, *, loading: bool
    ) -> tuple[torch.Tensor, BodyTrace]:
        trace = self.body(byte_ids, loading=loading)
        return self.logits_from_coordinates(trace.coordinates), trace
