"""Differentiable mirror of a multi-body Ariadne world with one shared field."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

import torch
from torch import nn
import torch.nn.functional as F

from .data import BOS_BYTE, BYTE_ALPHABET, PAD_BYTE, Batch

WorldMode = Literal["oracle_handoff", "isolated", "direct", "shared"]


def orthonormal_rows(raw: torch.Tensor) -> torch.Tensor:
    if raw.ndim != 2 or raw.shape[0] > raw.shape[1]:
        raise ValueError("raw aperture must have shape [rank, dimension] with rank <= dimension")
    q, _ = torch.linalg.qr(raw.transpose(0, 1), mode="reduced")
    return q.transpose(0, 1)


def givens(left: torch.Tensor, right: torch.Tensor, angle: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
    cosine = torch.cos(angle)
    sine = torch.sin(angle)
    return cosine * left - sine * right, sine * left + cosine * right


@dataclass(slots=True)
class BodyStepReceipt:
    source_work: torch.Tensor
    internal_loss: torch.Tensor


class LocalWaveBody(nn.Module):
    """Small metric-normalized local body with ordered transport and finite ports."""

    def __init__(self, *, source_count: int, sites: int, courses: int, seed: int):
        super().__init__()
        generator = torch.Generator().manual_seed(seed)
        self.sites = sites
        self.courses = courses
        self.source = nn.Parameter(0.08 * torch.randn(source_count, sites, generator=generator))
        if sites % 2:
            raise ValueError("sites must be even for the vectorized local transport")
        layer_pairs = [
            (tuple(range(0, sites, 2)), tuple(range(1, sites, 2))),
            (tuple(range(1, sites, 2)), tuple(list(range(2, sites, 2)) + [0])),
            (tuple(range(0, sites // 2)), tuple(range(sites // 2, sites))),
        ]
        self.layers = layer_pairs
        self.raw_angles = nn.Parameter(torch.linspace(-0.25, 0.25, 3 * (sites // 2)))
        self.raw_port = nn.Parameter(torch.randn(courses, sites, generator=generator))
        self.raw_decay = nn.Parameter(torch.tensor(3.8))

    def port_rows(self) -> torch.Tensor:
        return orthonormal_rows(self.raw_port)

    def apply_sources(self, state: torch.Tensor, source_ids: torch.Tensor, mask: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        before = 0.5 * state.square().sum(dim=-1)
        source = self.source[source_ids.clamp(min=0)] * mask.unsqueeze(-1)
        state = state + source
        after = 0.5 * state.square().sum(dim=-1)
        return state, after - before

    def transport_and_loss(self, state: torch.Tensor, mask: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        angle_offset = 0
        for left_indices, right_indices in self.layers:
            width = len(left_indices)
            angles = 0.32 * torch.tanh(self.raw_angles[angle_offset : angle_offset + width])
            angle_offset += width
            left_index = torch.as_tensor(left_indices, device=state.device)
            right_index = torch.as_tensor(right_indices, device=state.device)
            left = state.index_select(1, left_index)
            right = state.index_select(1, right_index)
            next_left, next_right = givens(left, right, angles)
            updated = state.clone()
            updated[:, left_index] = next_left
            updated[:, right_index] = next_right
            state = torch.where(mask[:, None], updated, state)
        before = 0.5 * state.square().sum(dim=-1)
        retention = 0.985 + 0.014 * torch.sigmoid(self.raw_decay)
        damped = state * retention
        state = torch.where(mask[:, None], damped, state)
        after = 0.5 * state.square().sum(dim=-1)
        return state, before - after

    def step(self, state: torch.Tensor, source_ids: torch.Tensor, mask: torch.Tensor) -> tuple[torch.Tensor, BodyStepReceipt]:
        state, source_work = self.apply_sources(state, source_ids, mask)
        state, internal_loss = self.transport_and_loss(state, mask)
        return state, BodyStepReceipt(source_work, internal_loss)


class DelayedSharedField(nn.Module):
    """Four-course delayed field with orthogonal scattering and explicit loss."""

    def __init__(self, *, courses: int, delays: tuple[int, ...], seed: int):
        super().__init__()
        if len(delays) != courses or min(delays) < 1:
            raise ValueError("one positive delay is required per course")
        generator = torch.Generator().manual_seed(seed)
        self.courses = courses
        self.delays = delays
        self.raw_scatter = nn.Parameter(torch.randn(courses, courses, generator=generator))
        self.raw_decay = nn.Parameter(torch.tensor(4.0))
        self.raw_coupling = nn.Parameter(torch.full((courses,), -0.2))

    def scattering(self) -> torch.Tensor:
        q, _ = torch.linalg.qr(self.raw_scatter)
        return q

    def initial_buffers(self, batch: int, *, device: torch.device, dtype: torch.dtype) -> list[torch.Tensor]:
        return [torch.zeros(batch, delay, device=device, dtype=dtype) for delay in self.delays]

    @staticmethod
    def energy(buffers: list[torch.Tensor]) -> torch.Tensor:
        return 0.5 * sum(buffer.square().sum(dim=-1) for buffer in buffers)

    def heads(self, buffers: list[torch.Tensor]) -> torch.Tensor:
        return torch.stack([buffer[:, 0] for buffer in buffers], dim=-1)

    def write(self, buffers: list[torch.Tensor], heads: torch.Tensor) -> list[torch.Tensor]:
        output: list[torch.Tensor] = []
        for course, buffer in enumerate(buffers):
            if buffer.shape[1] == 1:
                output.append(heads[:, course : course + 1])
            else:
                output.append(torch.cat((buffer[:, 1:], heads[:, course : course + 1]), dim=1))
        return output

    def finish(
        self,
        buffers: list[torch.Tensor],
        heads: torch.Tensor,
        scattering: torch.Tensor,
        retention: torch.Tensor,
    ) -> tuple[list[torch.Tensor], torch.Tensor]:
        scattered = heads @ scattering.transpose(0, 1)
        before = 0.5 * scattered.square().sum(dim=-1)
        after_heads = retention * scattered
        after = 0.5 * after_heads.square().sum(dim=-1)
        return self.write(buffers, after_heads), before - after

    def coupling_angles(self) -> torch.Tensor:
        return 0.45 * torch.sigmoid(self.raw_coupling)


@dataclass(slots=True)
class WorldTrace:
    current_logits: torch.Tensor
    next_logits: torch.Tensor
    mode_logits: torch.Tensor
    lexical_states: torch.Tensor
    predictive_states: torch.Tensor
    field_heads: torch.Tensor
    max_closure_residual: torch.Tensor
    source_work: torch.Tensor
    internal_loss: torch.Tensor
    final_energy: torch.Tensor


class UnifiedLexicalWorld(nn.Module):
    """Two local bodies, one field, one material clock, and no token handoff."""

    def __init__(
        self,
        *,
        mode: WorldMode,
        sites: int = 16,
        courses: int = 4,
        settle_steps: int = 3,
        seed: int = 20260815,
    ):
        super().__init__()
        self.mode = mode
        self.sites = sites
        self.courses = courses
        self.settle_steps = settle_steps
        self.lexical = LocalWaveBody(source_count=BYTE_ALPHABET, sites=sites, courses=courses, seed=seed)
        # Predictive sources: no-op/BOS plus four oracle operator gestures. Shared-world mode uses only no-op.
        self.predictive = LocalWaveBody(source_count=5, sites=sites, courses=courses, seed=seed + 1)
        self.field = DelayedSharedField(courses=courses, delays=(1, 2, 3, 4)[:courses], seed=seed + 2)
        self.current_head = nn.Linear(sites, 4)
        self.next_head = nn.Linear(sites, 4)
        self.mode_head = nn.Linear(sites, 2)
        self.byte_head = nn.Linear(sites, 256)
        self.raw_direct_angle = nn.Parameter(torch.tensor(-0.2))

    @staticmethod
    def _energy(lexical: torch.Tensor, predictive: torch.Tensor, buffers: list[torch.Tensor]) -> torch.Tensor:
        return 0.5 * lexical.square().sum(dim=-1) + 0.5 * predictive.square().sum(dim=-1) + DelayedSharedField.energy(buffers)

    @staticmethod
    def _project(state: torch.Tensor, row: torch.Tensor) -> torch.Tensor:
        return (state * row).sum(dim=-1)

    @staticmethod
    def _add_projection(state: torch.Tensor, row: torch.Tensor, amount: torch.Tensor) -> torch.Tensor:
        return state + amount.unsqueeze(-1) * row

    def _direct_exchange(
        self,
        lexical: torch.Tensor,
        predictive: torch.Tensor,
        lexical_rows: torch.Tensor,
        predictive_rows: torch.Tensor,
    ) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        left_row = lexical_rows[0]
        right_row = predictive_rows[0]
        left = self._project(lexical, left_row)
        right = self._project(predictive, right_row)
        before = 0.5 * (left.square() + right.square())
        angle = 0.45 * torch.sigmoid(self.raw_direct_angle)
        next_left, next_right = givens(left, right, angle)
        lexical = self._add_projection(lexical, left_row, next_left - left)
        predictive = self._add_projection(predictive, right_row, next_right - right)
        residual = 0.5 * (next_left.square() + next_right.square()) - before
        return lexical, predictive, residual

    def _shared_exchange(
        self,
        lexical: torch.Tensor,
        predictive: torch.Tensor,
        buffers: list[torch.Tensor],
        *,
        shuffle_field: bool,
        lexical_port_scale: float,
        predictive_port_scale: float,
        lexical_rows: torch.Tensor,
        predictive_rows: torch.Tensor,
        coupling_angles: torch.Tensor,
        scattering: torch.Tensor,
        field_retention: torch.Tensor,
    ) -> tuple[torch.Tensor, torch.Tensor, list[torch.Tensor], torch.Tensor, torch.Tensor]:
        heads = self.field.heads(buffers)
        energy_before = 0.5 * lexical.square().sum(dim=-1) + 0.5 * predictive.square().sum(dim=-1) + 0.5 * heads.square().sum(dim=-1)
        angles = coupling_angles
        for course in range(self.courses):
            if lexical_port_scale != 0.0:
                coordinate = self._project(lexical, lexical_rows[course])
                next_coordinate, next_head = givens(coordinate, heads[:, course], angles[course] * lexical_port_scale)
                lexical = self._add_projection(lexical, lexical_rows[course], next_coordinate - coordinate)
                heads = heads.clone()
                heads[:, course] = next_head
        if shuffle_field:
            permutation = torch.arange(heads.shape[0] - 1, -1, -1, device=heads.device)
            heads = heads[permutation]
        for course in range(self.courses):
            if predictive_port_scale != 0.0:
                coordinate = self._project(predictive, predictive_rows[course])
                next_coordinate, next_head = givens(coordinate, heads[:, course], angles[course] * predictive_port_scale)
                predictive = self._add_projection(predictive, predictive_rows[course], next_coordinate - coordinate)
                heads = heads.clone()
                heads[:, course] = next_head
        energy_after_exchange = 0.5 * lexical.square().sum(dim=-1) + 0.5 * predictive.square().sum(dim=-1) + 0.5 * heads.square().sum(dim=-1)
        coupling_residual = energy_after_exchange - energy_before
        buffers, field_loss = self.field.finish(buffers, heads, scattering, field_retention)
        return lexical, predictive, buffers, coupling_residual, field_loss

    def forward(
        self,
        batch: Batch,
        *,
        shuffle_field: bool = False,
        lexical_port_scale: float = 1.0,
        predictive_port_scale: float = 1.0,
    ) -> WorldTrace:
        device = batch.byte_ids.device
        dtype = self.lexical.source.dtype
        lexical = torch.zeros(batch.batch_size, self.sites, device=device, dtype=dtype)
        predictive = torch.zeros_like(lexical)
        buffers = self.field.initial_buffers(batch.batch_size, device=device, dtype=dtype)
        lexical_rows = self.lexical.port_rows()
        predictive_rows = self.predictive.port_rows()
        coupling_angles = self.field.coupling_angles()
        scattering = self.field.scattering()
        field_retention = 0.985 + 0.014 * torch.sigmoid(self.field.raw_decay)
        source_work_total = torch.zeros(batch.batch_size, device=device, dtype=dtype)
        loss_total = torch.zeros_like(source_work_total)
        residual_max = torch.zeros_like(source_work_total)

        current_states: list[torch.Tensor] = []
        lexical_states: list[torch.Tensor] = []
        field_states: list[torch.Tensor] = []

        # BOS is a real source gesture on the lexical body. The predictive body receives only no-op.
        lexical, receipt = self.lexical.step(
            lexical,
            torch.full((batch.batch_size,), BOS_BYTE, dtype=torch.long, device=device),
            torch.ones(batch.batch_size, dtype=torch.bool, device=device),
        )
        source_work_total += receipt.source_work
        loss_total += receipt.internal_loss

        for event in range(batch.event_count):
            for position in range(batch.event_byte_ids.shape[2]):
                mask = batch.event_byte_mask[:, event, position]
                source_ids = batch.event_byte_ids[:, event, position]
                energy_before = self._energy(lexical, predictive, buffers)
                lexical, lexical_receipt = self.lexical.step(lexical, source_ids, mask)
                predictive_ids = torch.zeros(batch.batch_size, dtype=torch.long, device=device)
                predictive, predictive_receipt = self.predictive.step(
                    predictive,
                    predictive_ids,
                    torch.ones_like(mask),
                )
                source_work = lexical_receipt.source_work + predictive_receipt.source_work
                internal_loss = lexical_receipt.internal_loss + predictive_receipt.internal_loss
                coupling_residual = torch.zeros_like(source_work)
                field_loss = torch.zeros_like(source_work)
                if self.mode == "direct":
                    lexical, predictive, coupling_residual = self._direct_exchange(
                        lexical, predictive, lexical_rows, predictive_rows
                    )
                elif self.mode == "shared":
                    lexical, predictive, buffers, coupling_residual, field_loss = self._shared_exchange(
                        lexical,
                        predictive,
                        buffers,
                        shuffle_field=shuffle_field,
                        lexical_port_scale=lexical_port_scale,
                        predictive_port_scale=predictive_port_scale,
                        lexical_rows=lexical_rows,
                        predictive_rows=predictive_rows,
                        coupling_angles=coupling_angles,
                        scattering=scattering,
                        field_retention=field_retention,
                    )
                energy_after = self._energy(lexical, predictive, buffers)
                residual = energy_after - energy_before - source_work + internal_loss + field_loss
                residual_max = torch.maximum(residual_max, torch.maximum(residual.abs(), coupling_residual.abs()))
                source_work_total += source_work
                loss_total += internal_loss + field_loss

            # Let the common world evolve between surface event and observation.
            for settle_index in range(self.settle_steps):
                energy_before = self._energy(lexical, predictive, buffers)
                no_ids = torch.zeros(batch.batch_size, dtype=torch.long, device=device)
                active = torch.ones(batch.batch_size, dtype=torch.bool, device=device)
                lexical, lexical_receipt = self.lexical.step(lexical, no_ids, torch.zeros_like(active))
                predictive_ids = (
                    batch.operators[:, event] + 1
                    if self.mode == "oracle_handoff" and settle_index == 0
                    else no_ids
                )
                predictive, predictive_receipt = self.predictive.step(predictive, predictive_ids, active)
                source_work = lexical_receipt.source_work + predictive_receipt.source_work
                internal_loss = lexical_receipt.internal_loss + predictive_receipt.internal_loss
                coupling_residual = torch.zeros_like(source_work)
                field_loss = torch.zeros_like(source_work)
                if self.mode == "direct":
                    lexical, predictive, coupling_residual = self._direct_exchange(
                        lexical, predictive, lexical_rows, predictive_rows
                    )
                elif self.mode == "shared":
                    lexical, predictive, buffers, coupling_residual, field_loss = self._shared_exchange(
                        lexical,
                        predictive,
                        buffers,
                        shuffle_field=shuffle_field,
                        lexical_port_scale=lexical_port_scale,
                        predictive_port_scale=predictive_port_scale,
                        lexical_rows=lexical_rows,
                        predictive_rows=predictive_rows,
                        coupling_angles=coupling_angles,
                        scattering=scattering,
                        field_retention=field_retention,
                    )
                energy_after = self._energy(lexical, predictive, buffers)
                residual = energy_after - energy_before - source_work + internal_loss + field_loss
                residual_max = torch.maximum(residual_max, torch.maximum(residual.abs(), coupling_residual.abs()))
                source_work_total += source_work
                loss_total += internal_loss + field_loss

            current_states.append(predictive)
            lexical_states.append(lexical)
            field_states.append(self.field.heads(buffers))

        predictive_stack = torch.stack(current_states, dim=1)
        lexical_stack = torch.stack(lexical_states, dim=1)
        field_stack = torch.stack(field_states, dim=1)
        return WorldTrace(
            current_logits=self.current_head(predictive_stack),
            next_logits=self.next_head(predictive_stack),
            mode_logits=self.mode_head(predictive_stack),
            lexical_states=lexical_stack,
            predictive_states=predictive_stack,
            field_heads=field_stack,
            max_closure_residual=residual_max,
            source_work=source_work_total,
            internal_loss=loss_total,
            final_energy=self._energy(lexical, predictive, buffers),
        )


def training_loss(trace: WorldTrace, batch: Batch) -> torch.Tensor:
    current = F.cross_entropy(trace.current_logits.flatten(0, 1), batch.operators.flatten())
    next_loss = F.cross_entropy(trace.next_logits.flatten(0, 1), batch.next_operators.flatten())
    mode = F.cross_entropy(trace.mode_logits.flatten(0, 1), batch.modes.flatten())
    return current + 0.5 * next_loss + 0.25 * mode
