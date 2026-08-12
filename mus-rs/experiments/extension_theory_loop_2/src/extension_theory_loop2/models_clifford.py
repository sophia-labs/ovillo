from __future__ import annotations

from typing import Any, Protocol

import numpy as np
import torch
from torch import Tensor, nn
import torch.nn.functional as F

def _right_wedge_sign(mask_without_axis: int, axis: int) -> float:
    above = mask_without_axis >> (axis + 1)
    return -1.0 if above.bit_count() % 2 else 1.0


def clifford_generators(latent_dim: int, dtype: torch.dtype = torch.float32) -> Tensor:
    state_dim = 1 << latent_dim
    generators = torch.zeros(latent_dim, state_dim, state_dim, dtype=dtype)
    for axis in range(latent_dim):
        bit = 1 << axis
        for mask in range(state_dim):
            if mask & bit:
                continue
            paired = mask | bit
            sign = _right_wedge_sign(mask, axis)
            # Row-state convention: e_mask D_axis = sign e_paired and
            # e_paired D_axis = -sign e_mask.
            generators[axis, mask, paired] = sign
            generators[axis, paired, mask] = -sign
    return generators


class PredictiveModel(Protocol):
    state_size: int

    def forward(self, tokens: Tensor) -> Tensor: ...

    def extract_state(self, tokens: Tensor) -> Tensor: ...


class StableCliffordBank(nn.Module):
    """Driven dissipative bank of exact closed Clifford gesture fibers."""

    def __init__(
        self,
        symbols: int,
        latent_dim: int = 3,
        fibers: int = 1,
        segments: int = 4,
        loop_scale: float = 0.18,
        retention_init: list[float] | Tensor | None = None,
        learn_retention: bool = True,
        use_source: bool = True,
        use_transition: bool = True,
    ):
        super().__init__()
        self.symbols = symbols
        self.latent_dim = latent_dim
        self.fibers = fibers
        self.segments = segments
        self.state_dim = 1 << latent_dim
        self.use_source = use_source
        self.use_transition = use_transition
        self.register_buffer("generators", clifford_generators(latent_dim))
        self.free = nn.Parameter(
            torch.randn(fibers, symbols, segments - 1, latent_dim) * loop_scale
        )
        if retention_init is None:
            retention = (
                torch.linspace(0.55, 0.94, fibers)
                if fibers > 1
                else torch.tensor([0.78])
            )
        else:
            retention = torch.as_tensor(retention_init, dtype=torch.float32)
        self.raw_retention = nn.Parameter(
            torch.logit(retention.clamp(0.02, 0.995)), requires_grad=learn_retention
        )
        self.raw_source = nn.Parameter(torch.full((fibers,), -0.2))
        feature_multiple = 2 if use_transition else 1
        self.readout = nn.Linear(fibers * self.state_dim * feature_multiple, symbols)

    @property
    def state_size(self) -> int:
        return self.fibers * self.state_dim

    def loops(self) -> Tensor:
        return torch.cat([self.free, -self.free.sum(dim=-2, keepdim=True)], dim=-2)

    def segment_operator(self, delta: Tensor) -> Tensor:
        generator = torch.einsum("...a,aij->...ij", delta, self.generators)
        radius = delta.square().sum(dim=-1).sqrt()
        safe = radius.clamp_min(1e-7)
        sinc = torch.sin(radius) / safe
        sinc = torch.where(radius < 1e-4, 1 - radius.square() / 6, sinc)
        identity = torch.eye(self.state_dim, device=delta.device, dtype=delta.dtype)
        return (
            torch.cos(radius)[..., None, None] * identity
            + sinc[..., None, None] * generator
        )

    def programs(self) -> tuple[Tensor, Tensor]:
        loops = self.loops()
        identity = torch.eye(self.state_dim, device=loops.device).expand(
            self.fibers, self.symbols, self.state_dim, self.state_dim
        )
        operator = identity.clone()
        vacuum = torch.zeros(
            self.fibers, self.symbols, self.state_dim, device=loops.device
        )
        vacuum[..., 0] = 1.0
        path_state = vacuum
        trace = []
        for segment in range(self.segments):
            step = self.segment_operator(loops[:, :, segment])
            operator = operator @ step
            path_state = torch.einsum("fvi,fvij->fvj", path_state, step)
            trace.append(path_state)
        source = torch.stack(trace, dim=-2).mean(dim=-2) - vacuum
        return operator, source

    def initial_state(self, batch: int) -> Tensor:
        state = torch.zeros(batch, self.fibers, self.state_dim, device=self.free.device)
        state[:, :, 0] = 1.0
        return state

    def step(
        self,
        state: Tensor,
        symbols: Tensor,
        programs: tuple[Tensor, Tensor] | None = None,
    ) -> tuple[Tensor, Tensor, Tensor]:
        operators, sources = self.programs() if programs is None else programs
        selected = operators[:, symbols].permute(1, 0, 2, 3)
        transported = torch.einsum("bfi,bfij->bfj", state, selected)
        source = sources[:, symbols].permute(1, 0, 2)
        retention = torch.sigmoid(self.raw_retention)[None, :, None]
        vacuum = torch.zeros_like(state)
        vacuum[:, :, 0] = 1.0
        attractor = vacuum
        if self.use_source:
            attractor = attractor + F.softplus(self.raw_source)[None, :, None] * source
        new_state = retention * transported + (1.0 - retention) * attractor
        transition = new_state - state
        features = [new_state.reshape(state.shape[0], -1)]
        if self.use_transition:
            features.append(transition.reshape(state.shape[0], -1))
        logits = self.readout(torch.cat(features, dim=-1))
        return new_state, logits, transition

    def forward_with_states(self, tokens: Tensor) -> tuple[Tensor, Tensor]:
        programs = self.programs()
        state = self.initial_state(tokens.shape[0])
        logits, states = [], []
        for t in range(tokens.shape[1]):
            state, output, _ = self.step(state, tokens[:, t], programs)
            logits.append(output)
            states.append(state.reshape(tokens.shape[0], -1))
        return torch.stack(logits, dim=1), torch.stack(states, dim=1)

    def forward(self, tokens: Tensor) -> Tensor:
        return self.forward_with_states(tokens)[0]

    def extract_state(self, tokens: Tensor) -> Tensor:
        return self.forward_with_states(tokens)[1][:, -1]

    def regularization(self) -> Tensor:
        length = self.loops().norm(dim=-1).sum(dim=-1)
        return 1e-4 * (length - 1.5).square().mean() + 1e-5 * F.softplus(
            self.raw_source
        ).square().mean()

    @torch.no_grad()
    def diagnostics(self) -> dict[str, Any]:
        operators, sources = self.programs()
        identity = torch.eye(self.state_dim, device=operators.device)
        return {
            "kind": "dissipative_clifford",
            "latent_dim": self.latent_dim,
            "fibers": self.fibers,
            "state_size": self.state_size,
            "retention": torch.sigmoid(self.raw_retention).tolist(),
            "source_gain": F.softplus(self.raw_source).tolist(),
            "loop_closure": float(self.loops().sum(dim=-2).abs().max()),
            "orthogonality": float(
                (operators.transpose(-2, -1) @ operators - identity).abs().max()
            ),
            "mean_source_rms": float(sources.square().mean().sqrt()),
            "parameters": sum(parameter.numel() for parameter in self.parameters()),
        }
