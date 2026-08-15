"""Exact finite-rank metric radiation for Token Radiation v0.

The law mirrors the Ariadne rank-k radiation primitive. Rows are expressed in
metric-normalized coordinates and must be orthonormal. Passive radiation removes
exactly the storage carried by the emitted amplitudes. Non-loading observation
uses the same aperture but leaves the state untouched and books no radiation.
"""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import json
from typing import Any

import numpy as np
import torch

SCHEMA_VERSION = "ariadne-metric-radiation-v1"


@dataclass(frozen=True, slots=True)
class RadiationPlan:
    id: str
    fraction: float
    rows: tuple[tuple[float, ...], ...]
    schema_version: str = SCHEMA_VERSION

    @property
    def rank(self) -> int:
        return len(self.rows)

    @property
    def dimension(self) -> int:
        return len(self.rows[0]) if self.rows else 0

    def validate(self, *, tolerance: float = 1e-8) -> None:
        if self.schema_version != SCHEMA_VERSION:
            raise ValueError(f"unsupported radiation schema {self.schema_version!r}")
        if not self.id.strip():
            raise ValueError("radiation aperture id must be nonempty")
        if not np.isfinite(self.fraction) or not 0.0 <= self.fraction <= 1.0:
            raise ValueError("radiation fraction must be finite in [0,1]")
        if not self.rows or not self.rows[0]:
            raise ValueError("radiation aperture requires at least one nonempty row")
        dimension = len(self.rows[0])
        if len(self.rows) > dimension:
            raise ValueError("radiation rank cannot exceed state dimension")
        matrix = np.asarray(self.rows, dtype=np.float64)
        if matrix.shape != (len(self.rows), dimension) or not np.isfinite(matrix).all():
            raise ValueError("radiation rows must form a finite rectangular matrix")
        gram = matrix @ matrix.T
        error = float(np.max(np.abs(gram - np.eye(len(self.rows)))))
        if error > tolerance:
            raise ValueError(f"radiation rows are not orthonormal; residual={error:.3e}")

    def to_dict(self) -> dict[str, Any]:
        self.validate()
        return {
            "schema_version": self.schema_version,
            "id": self.id,
            "fraction": self.fraction,
            "rows": [list(row) for row in self.rows],
        }

    @property
    def sha256(self) -> str:
        payload = json.dumps(
            self.to_dict(), separators=(",", ":"), ensure_ascii=False
        ).encode()
        return hashlib.sha256(payload).hexdigest()


@dataclass(slots=True)
class TorchRadiationFrame:
    state_after: torch.Tensor
    emitted: torch.Tensor
    storage_before: torch.Tensor
    storage_after: torch.Tensor
    projected_intensity: torch.Tensor
    booked_radiation: torch.Tensor
    residual: torch.Tensor


@dataclass(frozen=True, slots=True)
class NumpyRadiationFrame:
    state_after: np.ndarray
    emitted: np.ndarray
    storage_before: float
    storage_after: float
    projected_intensity: float
    booked_radiation: float
    residual: float


def orthonormal_rows(raw_rows: torch.Tensor) -> torch.Tensor:
    """Return row-orthonormal coordinates with differentiable QR."""

    if raw_rows.ndim != 2 or raw_rows.shape[0] < 1 or raw_rows.shape[0] > raw_rows.shape[1]:
        raise ValueError("raw_rows must have shape [rank, dimension], rank <= dimension")
    q, r = torch.linalg.qr(raw_rows.T, mode="reduced")
    diagonal = torch.diagonal(r)
    signs = torch.where(
        diagonal.detach() < 0, -torch.ones_like(diagonal), torch.ones_like(diagonal)
    )
    return (q * signs).T


def _validate_torch_inputs(
    state: torch.Tensor, metric: torch.Tensor, rows: torch.Tensor, fraction: float
) -> None:
    if state.ndim < 1:
        raise ValueError("state must have a final coordinate dimension")
    if metric.ndim != 1 or metric.shape[0] != state.shape[-1]:
        raise ValueError("metric must be a vector matching the state dimension")
    if rows.ndim != 2 or rows.shape[1] != state.shape[-1]:
        raise ValueError("rows must have shape [rank, state_dimension]")
    if rows.shape[0] > rows.shape[1] or rows.shape[0] < 1:
        raise ValueError("radiation rank must be in [1, state_dimension]")
    if not 0.0 <= fraction <= 1.0:
        raise ValueError("fraction must be in [0,1]")
    if bool(torch.any(~torch.isfinite(metric))) or bool(torch.any(metric <= 0)):
        raise ValueError("metric entries must be finite and positive")


def metric_radiate_torch(
    state: torch.Tensor,
    metric: torch.Tensor,
    rows: torch.Tensor,
    fraction: float,
    *,
    loading: bool = True,
) -> TorchRadiationFrame:
    """Project a state through a finite-rank metric aperture."""

    _validate_torch_inputs(state, metric, rows, fraction)
    sqrt_metric = torch.sqrt(metric)
    q = state * sqrt_metric
    storage_before = 0.5 * q.square().sum(dim=-1)
    coefficients = torch.einsum("...n,kn->...k", q, rows)
    emitted = float(fraction) ** 0.5 * coefficients
    projected_intensity = 0.5 * emitted.square().sum(dim=-1)

    if loading:
        removal = 1.0 - (1.0 - float(fraction)) ** 0.5
        q_after = q - removal * torch.einsum("...k,kn->...n", coefficients, rows)
        state_after = q_after / sqrt_metric
        booked_radiation = projected_intensity
    else:
        state_after = state
        booked_radiation = torch.zeros_like(projected_intensity)

    storage_after = 0.5 * (state_after * sqrt_metric).square().sum(dim=-1)
    residual = storage_after - storage_before + booked_radiation
    return TorchRadiationFrame(
        state_after=state_after,
        emitted=emitted,
        storage_before=storage_before,
        storage_after=storage_after,
        projected_intensity=projected_intensity,
        booked_radiation=booked_radiation,
        residual=residual,
    )


def metric_radiate_numpy(
    state: np.ndarray,
    metric: np.ndarray,
    rows: np.ndarray,
    fraction: float,
    *,
    loading: bool = True,
) -> NumpyRadiationFrame:
    state = np.asarray(state, dtype=np.float64)
    metric = np.asarray(metric, dtype=np.float64)
    rows = np.asarray(rows, dtype=np.float64)
    if (
        state.ndim != 1
        or metric.shape != state.shape
        or rows.ndim != 2
        or rows.shape[1] != state.size
    ):
        raise ValueError("invalid radiation shapes")
    if rows.shape[0] < 1 or rows.shape[0] > state.size:
        raise ValueError("invalid radiation rank")
    if not np.isfinite(state).all() or not np.isfinite(metric).all() or np.any(metric <= 0):
        raise ValueError("state and metric must be finite; metric must be positive")
    if not np.isfinite(rows).all() or not np.isfinite(fraction) or not 0 <= fraction <= 1:
        raise ValueError("rows and fraction must be finite")
    gram_error = np.max(np.abs(rows @ rows.T - np.eye(rows.shape[0])))
    if gram_error > 1e-8:
        raise ValueError(f"rows are not orthonormal; residual={gram_error:.3e}")

    q = np.sqrt(metric) * state
    storage_before = 0.5 * float(q @ q)
    coefficients = rows @ q
    emitted = np.sqrt(fraction) * coefficients
    projected_intensity = 0.5 * float(emitted @ emitted)
    if loading:
        removal = 1.0 - np.sqrt(1.0 - fraction)
        q_after = q - removal * (rows.T @ coefficients)
        state_after = q_after / np.sqrt(metric)
        booked = projected_intensity
    else:
        state_after = state.copy()
        booked = 0.0
    normalized_after = np.sqrt(metric) * state_after
    storage_after = 0.5 * float(normalized_after @ normalized_after)
    residual = storage_after - storage_before + booked
    return NumpyRadiationFrame(
        state_after=state_after,
        emitted=emitted,
        storage_before=storage_before,
        storage_after=storage_after,
        projected_intensity=projected_intensity,
        booked_radiation=booked,
        residual=residual,
    )
