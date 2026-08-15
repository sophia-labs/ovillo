"""Lexical, predictive, event, and articulation models."""

from __future__ import annotations

from dataclasses import dataclass

import torch
from torch import nn

from .body import BodyConfig, BodyTrace, LocalRadiatingBody, RadiationStep
from .world import OPS, SURFACES, ByteCodec


class MultiTaskHead(nn.Module):
    def __init__(self, input_dim: int, byte_classes: int, hidden: int = 64):
        super().__init__()
        self.trunk = nn.Sequential(nn.Linear(input_dim, hidden), nn.Tanh())
        self.byte = nn.Linear(hidden, byte_classes)
        self.boundary = nn.Linear(hidden, 1)
        self.operator = nn.Linear(hidden, len(OPS))
        self.next_operator = nn.Linear(hidden, len(OPS))
        self.mode = nn.Linear(hidden, 2)
        self.group = nn.Linear(hidden, 8)

    def forward(self, values: torch.Tensor) -> dict[str, torch.Tensor]:
        hidden = self.trunk(values)
        return {
            "byte": self.byte(hidden),
            "boundary": self.boundary(hidden).squeeze(-1),
            "operator": self.operator(hidden),
            "next_operator": self.next_operator(hidden),
            "mode": self.mode(hidden),
            "group": self.group(hidden),
        }


@dataclass(slots=True)
class LexicalForward:
    outputs: dict[str, torch.Tensor]
    trace: BodyTrace


class LexicalRadiationModel(nn.Module):
    """Byte surface body trained through a radiation-only interface."""

    def __init__(
        self,
        codec: ByteCodec,
        *,
        sites: int = 16,
        radiation_fraction: float = 0.04,
        seed: int = 20260815,
    ):
        super().__init__()
        self.codec = codec
        self.body = LocalRadiatingBody(
            BodyConfig(
                source_count=codec.source_classes,
                sites=sites,
                radiation_rank=sites,
                radiation_fraction=radiation_fraction,
                fixed_full_aperture=True,
                seed=seed,
            )
        )
        self.head = MultiTaskHead(sites, codec.model_classes)

    def forward(self, byte_ids: torch.Tensor, *, loading: bool = True) -> LexicalForward:
        trace = self.body(
            byte_ids, loading=loading, bos_source_id=self.codec.bos_id
        )
        return LexicalForward(self.head(trace.emissions), trace)


class TraceClassifier(nn.Module):
    """Small detector over a fixed radiation/surface trace window."""

    def __init__(self, input_dim: int, output_dim: int, hidden: int = 96):
        super().__init__()
        self.network = nn.Sequential(
            nn.Linear(input_dim, hidden),
            nn.Tanh(),
            nn.Linear(hidden, output_dim),
        )

    def forward(self, values: torch.Tensor) -> torch.Tensor:
        return self.network(values)


@dataclass(slots=True)
class PredictiveForward:
    outputs: dict[str, torch.Tensor]
    trace: BodyTrace


class PredictiveOperatorBody(nn.Module):
    """Surface-neutral body whose source alphabet is latent operators."""

    OPERATOR_COUNT = len(OPS)
    INITIAL_STATE_COUNT = 16

    def __init__(
        self,
        *,
        sites: int = 16,
        radiation_fraction: float = 0.04,
        seed: int = 20260816,
    ):
        super().__init__()
        self.body = LocalRadiatingBody(
            BodyConfig(
                source_count=self.OPERATOR_COUNT + self.INITIAL_STATE_COUNT,
                sites=sites,
                radiation_rank=sites,
                radiation_fraction=radiation_fraction,
                fixed_full_aperture=True,
                seed=seed,
            )
        )
        self.head = MultiTaskHead(sites, byte_classes=1)

    @staticmethod
    def initial_source_id(
        rotation: torch.Tensor,
        reflected: torch.Tensor,
        mode: torch.Tensor,
    ) -> torch.Tensor:
        return 4 + rotation + 4 * reflected + 8 * mode

    def forward(
        self,
        initial_ids: torch.Tensor,
        operator_ids: torch.Tensor,
        *,
        loading: bool = True,
    ) -> PredictiveForward:
        if operator_ids.ndim != 2:
            raise ValueError("operator_ids must have shape [batch,event]")
        batch = operator_ids.shape[0]
        state = self.body.zero_state(batch, device=operator_ids.device)
        constrained = self.body.constrained()
        frames: list[RadiationStep] = []
        frame = self.body.step_ids(
            state, 4 + initial_ids, loading=loading, constrained=constrained
        )
        frames.append(frame)
        state = frame.state_after
        for position in range(operator_ids.shape[1]):
            frame = self.body.step_ids(
                state,
                operator_ids[:, position],
                loading=loading,
                constrained=constrained,
            )
            frames.append(frame)
            state = frame.state_after
        trace = BodyTrace(
            states=torch.stack([frame.state_after for frame in frames], dim=1),
            emissions=torch.stack([frame.emission for frame in frames], dim=1),
            storage_before=torch.stack(
                [frame.storage_before for frame in frames], dim=1
            ),
            storage_after=torch.stack(
                [frame.storage_after for frame in frames], dim=1
            ),
            booked_radiation=torch.stack(
                [frame.booked_radiation for frame in frames], dim=1
            ),
            closure_residual=torch.stack(
                [frame.residual for frame in frames], dim=1
            ),
        )
        return PredictiveForward(self.head(trace.emissions), trace)

    def start(self, initial_id: torch.Tensor, *, loading: bool = True) -> RadiationStep:
        state = self.body.zero_state(len(initial_id), device=initial_id.device)
        return self.body.step_ids(state, 4 + initial_id, loading=loading)

    def feedback(
        self,
        state: torch.Tensor,
        operator_id: torch.Tensor,
        *,
        loading: bool = True,
    ) -> RadiationStep:
        return self.body.step_ids(state, operator_id, loading=loading)


class DeclaredArticulator:
    """Lossless surface codec used to exercise the closed loop."""

    @staticmethod
    def render(
        operator_id: int,
        state: tuple[int, int, int],
        event_index: int,
    ) -> bytes:
        operator = OPS[operator_id]
        variants = SURFACES[state[2]][operator]
        if (
            operator in ("R", "F")
            and b"ba" in variants
            and (state[0] + event_index) % 3 == 2
        ):
            return b"ba"
        return variants[(3 * state[0] + state[1] + event_index) % len(variants)]
