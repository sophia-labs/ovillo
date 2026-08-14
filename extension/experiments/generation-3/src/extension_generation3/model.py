"""Differentiable learner for Extension Generation 3 constitutive media."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable

import torch
from torch import nn
import torch.nn.functional as F


@dataclass(frozen=True, slots=True)
class LearnedMediumConfig:
    sites: int = 4
    sample_rate_hz: float = 32.0
    edges: tuple[tuple[int, int], ...] = ((0, 1), (1, 2), (2, 3), (0, 2))
    sources: tuple[int, ...] = (0, 1, 2, 3)
    maximum_wave_rate: float = 24.0
    maximum_evidence_rate: float = 12.0


@dataclass(slots=True)
class MediumTrajectory:
    wave: torch.Tensor
    evidence: torch.Tensor
    reservoir: torch.Tensor
    history: list[torch.Tensor]

    @property
    def features(self) -> torch.Tensor:
        return torch.cat((self.wave, self.evidence, self.reservoir), dim=-1)


class LearnedConstitutiveMedium(nn.Module):
    """A small differentiable mirror of ``ariadne-local-medium-v1``.

    The law vocabulary is intentionally constrained: weighted orthogonal wave
    transport, positive local diffusion, finite reservoir transfer, and passive
    loss. Training selects a constitution inside that lawful family; compilation
    emits only the constitution, never optimizer or observer state.
    """

    def __init__(self, config: LearnedMediumConfig = LearnedMediumConfig()):
        super().__init__()
        self.config = config
        n = config.sites
        e = len(config.edges)
        s = len(config.sources)

        self.raw_wave_metric = nn.Parameter(torch.zeros(n, dtype=torch.float64))
        self.raw_wave_decay = nn.Parameter(torch.full((n,), -5.0, dtype=torch.float64))
        self.raw_evidence_decay = nn.Parameter(torch.full((n,), -5.0, dtype=torch.float64))
        self.raw_reservoir_capacity = nn.Parameter(torch.zeros(n, dtype=torch.float64))
        self.raw_reservoir_leak = nn.Parameter(torch.full((n,), -6.0, dtype=torch.float64))
        self.raw_active_rate = nn.Parameter(torch.full((n,), -5.0, dtype=torch.float64))
        self.raw_active_threshold = nn.Parameter(torch.zeros(n, dtype=torch.float64))
        self.raw_active_target = nn.Parameter(torch.zeros(n, dtype=torch.float64))

        # Deliberately asymmetric initialization so gradient descent need not
        # first escape an exact edge-permutation symmetry.
        wave_seed = torch.linspace(-0.35, 0.35, e, dtype=torch.float64)
        evidence_seed = torch.linspace(-0.25, 0.25, e, dtype=torch.float64)
        self.raw_wave_rate = nn.Parameter(wave_seed)
        self.raw_evidence_rate = nn.Parameter(evidence_seed)

        self.source_wave_gain = nn.Parameter(
            torch.tensor([1.0, 1.0, 0.0, 0.0][:s], dtype=torch.float64)
        )
        self.source_evidence_gain = nn.Parameter(
            torch.tensor([0.0, 0.0, 1.0, 1.0][:s], dtype=torch.float64)
        )
        self.source_work_gain = nn.Parameter(torch.zeros(s, dtype=torch.float64))

    @property
    def dt(self) -> float:
        return 1.0 / self.config.sample_rate_hz

    def constrained(self) -> dict[str, torch.Tensor]:
        return {
            "wave_metric": F.softplus(self.raw_wave_metric) + 0.25,
            "wave_decay": F.softplus(self.raw_wave_decay),
            "evidence_decay": F.softplus(self.raw_evidence_decay),
            "reservoir_capacity": F.softplus(self.raw_reservoir_capacity) + 0.25,
            "reservoir_leak": F.softplus(self.raw_reservoir_leak),
            "active_rate": F.softplus(self.raw_active_rate),
            "active_threshold": F.softplus(self.raw_active_threshold),
            "active_target": F.softplus(self.raw_active_target) + 0.05,
            "wave_rate": torch.tanh(self.raw_wave_rate) * self.config.maximum_wave_rate,
            "evidence_rate": torch.sigmoid(self.raw_evidence_rate)
            * self.config.maximum_evidence_rate,
        }

    def forward(
        self,
        drives: torch.Tensor,
        *,
        damaged_edge: int | None = None,
        damage_steps: Iterable[int] = (),
        retain_history: bool = False,
    ) -> MediumTrajectory:
        """Advance a batch of source drives.

        ``drives`` has shape ``[batch, time, sources, 3]`` and carries wave,
        positive evidence, and work-rate coordinates in the final dimension.
        """

        if (
            drives.ndim != 4
            or drives.shape[2] != len(self.config.sources)
            or drives.shape[3] != 3
        ):
            raise ValueError("drives must have shape [batch, time, sources, 3]")
        if drives.dtype != torch.float64:
            drives = drives.to(dtype=torch.float64)
        params = self.constrained()
        batch = drives.shape[0]
        device = drives.device
        n = self.config.sites
        wave = torch.zeros(batch, n, dtype=drives.dtype, device=device)
        evidence = torch.zeros_like(wave)
        reservoir = torch.zeros_like(wave)
        history: list[torch.Tensor] = []
        damage_set = set(int(step) for step in damage_steps)

        source_sites = torch.tensor(self.config.sources, device=device)
        wave_gain = self.source_wave_gain.to(device=device)
        evidence_gain = F.softplus(self.source_evidence_gain).to(device=device)
        work_gain = F.softplus(self.source_work_gain).to(device=device)

        for step in range(drives.shape[1]):
            frame = drives[:, step]
            for source_index in range(frame.shape[1]):
                site = int(source_sites[source_index])
                wave[:, site] = (
                    wave[:, site]
                    + wave_gain[source_index] * frame[:, source_index, 0]
                )
                evidence[:, site] = evidence[:, site] + (
                    evidence_gain[source_index] * frame[:, source_index, 1]
                ).clamp_min(0.0)
                requested = (
                    work_gain[source_index] * frame[:, source_index, 2]
                ).clamp_min(0.0) * self.dt
                room = (
                    params["reservoir_capacity"][site] - reservoir[:, site]
                ).clamp_min(0.0)
                reservoir[:, site] = reservoir[:, site] + torch.minimum(room, requested)

            # Reservoir-paid active transfer. It is exactly internal in the
            # declared storage metric: reservoir decreases by the wave-energy gain.
            wave_energy = 0.5 * params["wave_metric"] * wave.square()
            activation = (
                evidence + wave.abs() - params["active_threshold"]
            ).clamp_min(0.0)
            request = params["active_rate"] * self.dt * activation
            gap = (params["active_target"] - wave_energy).clamp_min(0.0)
            transfer = torch.minimum(torch.minimum(reservoir, gap), request)
            next_wave_energy = wave_energy + transfer
            sign = torch.where(
                wave < 0.0,
                -torch.ones_like(wave),
                torch.ones_like(wave),
            )
            candidate_wave = sign * torch.sqrt(
                (2.0 * next_wave_energy / params["wave_metric"]).clamp_min(1e-18)
            )
            wave = torch.where(transfer > 0.0, candidate_wave, wave)
            reservoir = reservoir - transfer

            for edge_index, (left, right) in enumerate(self.config.edges):
                if damaged_edge == edge_index and step in damage_set:
                    continue
                left_metric = params["wave_metric"][left]
                right_metric = params["wave_metric"][right]
                theta = params["wave_rate"][edge_index] * self.dt
                cosine = torch.cos(theta)
                sine = torch.sin(theta)
                scaled_left = torch.sqrt(left_metric) * wave[:, left]
                scaled_right = torch.sqrt(right_metric) * wave[:, right]
                wave_left = (
                    cosine * scaled_left - sine * scaled_right
                ) / torch.sqrt(left_metric)
                wave_right = (
                    sine * scaled_left + cosine * scaled_right
                ) / torch.sqrt(right_metric)
                wave = wave.clone()
                wave[:, left] = wave_left
                wave[:, right] = wave_right

                mix = (
                    params["evidence_rate"][edge_index] * self.dt
                ).clamp(0.0, 0.5)
                evidence_left = evidence[:, left]
                evidence_right = evidence[:, right]
                next_left = (1.0 - mix) * evidence_left + mix * evidence_right
                next_right = mix * evidence_left + (1.0 - mix) * evidence_right
                evidence = evidence.clone()
                evidence[:, left] = next_left
                evidence[:, right] = next_right

            wave = wave * torch.exp(-0.5 * params["wave_decay"] * self.dt)
            evidence = evidence * torch.exp(
                -0.5 * params["evidence_decay"] * self.dt
            )
            reservoir = reservoir * torch.exp(
                -params["reservoir_leak"] * self.dt
            )
            if retain_history:
                history.append(torch.cat((wave, evidence, reservoir), dim=-1))

        return MediumTrajectory(wave, evidence, reservoir, history)


class Generation3Model(nn.Module):
    """Constitutive body plus deliberately weak affine observers."""

    def __init__(self, config: LearnedMediumConfig = LearnedMediumConfig()):
        super().__init__()
        self.medium = LearnedConstitutiveMedium(config)
        self.observer = nn.Linear(3 * config.sites, 2, dtype=torch.float64)

    def forward(
        self, drives: torch.Tensor, **kwargs
    ) -> tuple[torch.Tensor, MediumTrajectory]:
        trajectory = self.medium(drives, **kwargs)
        return self.observer(trajectory.features), trajectory
