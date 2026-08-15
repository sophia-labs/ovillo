"""Exact finite-rank metric radiation used by the paired toy experiments."""

from __future__ import annotations

from dataclasses import dataclass

import torch


@dataclass(slots=True)
class RadiationFrame:
    state_after: torch.Tensor
    coordinates: torch.Tensor
    emitted: torch.Tensor
    storage_before: torch.Tensor
    storage_after: torch.Tensor
    projected_intensity: torch.Tensor
    booked_radiation: torch.Tensor
    residual: torch.Tensor


def orthonormal_rows(raw_rows: torch.Tensor) -> torch.Tensor:
    """Map a full-rank raw matrix to deterministic row-orthonormal coordinates."""

    if raw_rows.ndim != 2 or not 1 <= raw_rows.shape[0] <= raw_rows.shape[1]:
        raise ValueError("raw_rows must have shape [rank, dimension], rank <= dimension")
    q, r = torch.linalg.qr(raw_rows.T, mode="reduced")
    diagonal = torch.diagonal(r)
    signs = torch.where(
        diagonal.detach() < 0, -torch.ones_like(diagonal), torch.ones_like(diagonal)
    )
    return (q * signs).T


def metric_radiate(
    state: torch.Tensor,
    metric: torch.Tensor,
    rows: torch.Tensor,
    fraction: float,
    *,
    loading: bool,
) -> RadiationFrame:
    """Observe or passively load a state through a finite-rank metric aperture.

    ``coordinates = U sqrt(G) x`` are fraction-calibrated detector coordinates.
    ``emitted = sqrt(rho) coordinates`` is the physical emitted amplitude. The
    distinction prevents a detector-scale confound when sweeping ``rho``.
    """

    if state.ndim < 1 or metric.ndim != 1 or state.shape[-1] != metric.shape[0]:
        raise ValueError("state and metric dimensions do not agree")
    if rows.ndim != 2 or rows.shape[1] != state.shape[-1]:
        raise ValueError("rows must have shape [rank, state_dimension]")
    if not 0.0 <= fraction <= 1.0:
        raise ValueError("fraction must lie in [0,1]")
    if bool(torch.any(~torch.isfinite(state))) or bool(torch.any(~torch.isfinite(metric))):
        raise ValueError("state and metric must be finite")
    if bool(torch.any(metric <= 0)):
        raise ValueError("metric must be positive")

    sqrt_metric = metric.sqrt()
    q = state * sqrt_metric
    storage_before = 0.5 * q.square().sum(dim=-1)
    coordinates = torch.einsum("...n,kn->...k", q, rows)
    emitted = float(fraction) ** 0.5 * coordinates
    projected_intensity = 0.5 * emitted.square().sum(dim=-1)

    if loading and fraction > 0.0:
        removal = 1.0 - (1.0 - float(fraction)) ** 0.5
        projection = torch.einsum("...k,kn->...n", coordinates, rows)
        q_after = q - removal * projection
        state_after = q_after / sqrt_metric
        booked = projected_intensity
    else:
        state_after = state
        booked = torch.zeros_like(projected_intensity)

    storage_after = 0.5 * (state_after * sqrt_metric).square().sum(dim=-1)
    residual = storage_after - storage_before + booked
    return RadiationFrame(
        state_after=state_after,
        coordinates=coordinates,
        emitted=emitted,
        storage_before=storage_before,
        storage_after=storage_after,
        projected_intensity=projected_intensity,
        booked_radiation=booked,
        residual=residual,
    )
