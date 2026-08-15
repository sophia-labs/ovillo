"""Differentiable local bodies and exact finite-rank metric radiation.

This module is an Extension-side training mirror. The generic radiation law
already belongs to ``mus-ariadne``; this mirror keeps the toy construction
self-contained and exposes source gestures needed by the lexical interface.
"""

from __future__ import annotations

from dataclasses import dataclass
import math

import torch
from torch import nn
import torch.nn.functional as F


@dataclass(frozen=True, slots=True)
class BodyConfig:
    source_count: int
    sites: int = 16
    radiation_rank: int = 16
    radiation_fraction: float = 0.04
    fixed_full_aperture: bool = True
    seed: int = 20260815

    def validate(self) -> None:
        if self.source_count < 1:
            raise ValueError("source_count must be positive")
        if self.sites < 2:
            raise ValueError("sites must be at least two")
        if not 1 <= self.radiation_rank <= self.sites:
            raise ValueError("radiation_rank must lie in [1, sites]")
        if not 0.0 <= self.radiation_fraction <= 1.0:
            raise ValueError("radiation_fraction must lie in [0,1]")
        if self.fixed_full_aperture and self.radiation_rank != self.sites:
            raise ValueError("fixed_full_aperture requires rank equal to sites")


@dataclass(slots=True)
class BodyTrace:
    states: torch.Tensor
    emissions: torch.Tensor
    storage_before: torch.Tensor
    storage_after: torch.Tensor
    booked_radiation: torch.Tensor
    closure_residual: torch.Tensor


@dataclass(slots=True)
class RadiationStep:
    state_after: torch.Tensor
    emission: torch.Tensor
    storage_before: torch.Tensor
    storage_after: torch.Tensor
    booked_radiation: torch.Tensor
    residual: torch.Tensor


def _storage(state: torch.Tensor, metric: torch.Tensor) -> torch.Tensor:
    return 0.5 * (state.square() * metric).sum(dim=-1)


def _orthonormal_rows(raw: torch.Tensor) -> torch.Tensor:
    q, _ = torch.linalg.qr(raw.transpose(0, 1), mode="reduced")
    return q.transpose(0, 1)


class LocalRadiatingBody(nn.Module):
    """Local source/transport body with a declared radiation aperture."""

    def __init__(self, config: BodyConfig):
        super().__init__()
        config.validate()
        self.config = config
        generator = torch.Generator().manual_seed(config.seed)
        self.source_gestures = nn.Parameter(
            0.15
            * torch.randn(
                config.source_count,
                config.sites,
                generator=generator,
                dtype=torch.float32,
            )
        )
        self.raw_metric = nn.Parameter(torch.zeros(config.sites))
        self.raw_decay = nn.Parameter(torch.full((config.sites,), -4.0))
        self.raw_angles = nn.Parameter(torch.linspace(-0.4, 0.4, config.sites))
        if config.fixed_full_aperture:
            self.register_buffer(
                "fixed_aperture", torch.eye(config.sites, dtype=torch.float32)
            )
            self.raw_aperture = None
        else:
            self.fixed_aperture = None
            self.raw_aperture = nn.Parameter(
                torch.randn(
                    config.radiation_rank,
                    config.sites,
                    generator=generator,
                    dtype=torch.float32,
                )
            )

    def constrained(
        self,
    ) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor]:
        metric = F.softplus(self.raw_metric) + 0.5
        decay = F.softplus(self.raw_decay)
        angles = 0.45 * torch.tanh(self.raw_angles)
        transport = self._transport_matrix(angles)
        if self.fixed_aperture is not None:
            aperture = self.fixed_aperture
        else:
            assert self.raw_aperture is not None
            aperture = _orthonormal_rows(self.raw_aperture)
        return metric, decay, transport, aperture

    def _transport_matrix(self, angles: torch.Tensor) -> torch.Tensor:
        matrix = torch.eye(
            self.config.sites, dtype=angles.dtype, device=angles.device
        )
        for left in range(self.config.sites):
            right = (left + 1) % self.config.sites
            cosine = torch.cos(angles[left])
            sine = torch.sin(angles[left])
            left_row = matrix[left].clone()
            right_row = matrix[right].clone()
            matrix = matrix.clone()
            matrix[left] = cosine * left_row - sine * right_row
            matrix[right] = sine * left_row + cosine * right_row
        return matrix

    def zero_state(
        self, batch: int, *, device: torch.device | str | None = None
    ) -> torch.Tensor:
        return torch.zeros(
            batch,
            self.config.sites,
            device=device,
            dtype=self.source_gestures.dtype,
        )

    def step_with_source(
        self,
        state: torch.Tensor,
        source: torch.Tensor,
        *,
        loading: bool,
        constrained: tuple[
            torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor
        ]
        | None = None,
    ) -> RadiationStep:
        metric, decay, transport, aperture = (
            self.constrained() if constrained is None else constrained
        )
        normalized = (state + source) * metric.sqrt()
        normalized = normalized @ transport.transpose(0, 1)
        state = (normalized / metric.sqrt()) * torch.exp(-0.025 * decay)

        normalized = state * metric.sqrt()
        storage_before = 0.5 * normalized.square().sum(dim=-1)
        coefficients = normalized @ aperture.transpose(0, 1)
        emitted_scale = math.sqrt(self.config.radiation_fraction)
        emission = emitted_scale * coefficients
        booked = 0.5 * emission.square().sum(dim=-1)

        if loading and self.config.radiation_fraction > 0.0:
            projected = coefficients @ aperture
            removal = 1.0 - math.sqrt(1.0 - self.config.radiation_fraction)
            normalized = normalized - removal * projected
            state = normalized / metric.sqrt()
            storage_after = 0.5 * normalized.square().sum(dim=-1)
        else:
            storage_after = storage_before
            booked = torch.zeros_like(booked)

        residual = storage_after - storage_before + booked
        return RadiationStep(
            state_after=state,
            emission=emission,
            storage_before=storage_before,
            storage_after=storage_after,
            booked_radiation=booked,
            residual=residual,
        )

    def step_ids(
        self,
        state: torch.Tensor,
        source_ids: torch.Tensor,
        *,
        loading: bool,
        constrained: tuple[
            torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor
        ]
        | None = None,
    ) -> RadiationStep:
        return self.step_with_source(
            state,
            self.source_gestures[source_ids],
            loading=loading,
            constrained=constrained,
        )

    def forward(
        self,
        source_ids: torch.Tensor,
        *,
        loading: bool,
        bos_source_id: int | None = None,
    ) -> BodyTrace:
        if source_ids.ndim != 2:
            raise ValueError("source_ids must have shape [batch,time]")
        if source_ids.dtype != torch.long:
            source_ids = source_ids.long()
        batch, steps = source_ids.shape
        state = self.zero_state(batch, device=source_ids.device)
        states: list[torch.Tensor] = []
        emissions: list[torch.Tensor] = []
        before: list[torch.Tensor] = []
        after: list[torch.Tensor] = []
        radiation: list[torch.Tensor] = []
        residuals: list[torch.Tensor] = []
        constrained = self.constrained()

        if bos_source_id is not None:
            bos_ids = torch.full(
                (batch,),
                bos_source_id,
                dtype=torch.long,
                device=source_ids.device,
            )
            frame = self.step_ids(
                state,
                bos_ids,
                loading=loading,
                constrained=constrained,
            )
            state = frame.state_after
            states.append(state)
            emissions.append(frame.emission)
            before.append(frame.storage_before)
            after.append(frame.storage_after)
            radiation.append(frame.booked_radiation)
            residuals.append(frame.residual)

        for position in range(steps):
            frame = self.step_ids(
                state,
                source_ids[:, position],
                loading=loading,
                constrained=constrained,
            )
            state = frame.state_after
            states.append(state)
            emissions.append(frame.emission)
            before.append(frame.storage_before)
            after.append(frame.storage_after)
            radiation.append(frame.booked_radiation)
            residuals.append(frame.residual)

        return BodyTrace(
            states=torch.stack(states, dim=1),
            emissions=torch.stack(emissions, dim=1),
            storage_before=torch.stack(before, dim=1),
            storage_after=torch.stack(after, dim=1),
            booked_radiation=torch.stack(radiation, dim=1),
            closure_residual=torch.stack(residuals, dim=1),
        )

    def source_work(self, state: torch.Tensor, source: torch.Tensor) -> torch.Tensor:
        metric, _, _, _ = self.constrained()
        return _storage(state + source, metric) - _storage(state, metric)
