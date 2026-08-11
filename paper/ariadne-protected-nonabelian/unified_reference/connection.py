"""Metric-consistent state connections for retuning and rank change.

The protected modal backbone uses a fixed rank and fixed energy metric, so its
connection is the identity in material coordinates.  This module supplies the
separate dense oracle required when a delay/discretization really changes rank.
A raw interpolation is first moved into energy-normalized coordinates and then
replaced by its polar partial isometry.  Rank contraction is never silently
lossless: the discarded source component is measured and requires an explicit
radiate/dissipate policy.
"""

from __future__ import annotations

from dataclasses import dataclass
import math
from typing import Literal

import numpy as np
from numpy.typing import NDArray

FloatMatrix = NDArray[np.float64]
FloatVector = NDArray[np.float64]
DisposalPolicy = Literal["radiate", "dissipate", "reject"]


def _spd_factors(metric: FloatMatrix) -> tuple[FloatMatrix, FloatMatrix, float, float]:
    metric = np.asarray(metric, dtype=np.float64)
    if metric.ndim != 2 or metric.shape[0] != metric.shape[1]:
        raise ValueError("metric must be square")
    if np.linalg.norm(metric - metric.T, ord="fro") > 1.0e-11:
        raise ValueError("metric must be symmetric")
    values, vectors = np.linalg.eigh(metric)
    minimum = float(np.min(values))
    maximum = float(np.max(values))
    if minimum <= 0.0 or not np.all(np.isfinite(values)):
        raise ValueError("metric must be finite positive definite")
    root = vectors @ np.diag(np.sqrt(values)) @ vectors.T
    inverse_root = vectors @ np.diag(1.0 / np.sqrt(values)) @ vectors.T
    return root, inverse_root, minimum, maximum


def weighted_energy(state: FloatVector, metric: FloatMatrix) -> float:
    state = np.asarray(state, dtype=np.float64)
    metric = np.asarray(metric, dtype=np.float64)
    if metric.shape != (state.shape[0], state.shape[0]):
        raise ValueError("metric/state shape mismatch")
    return float(state @ metric @ state)


@dataclass(frozen=True)
class WeightedPolarCertificate:
    source_dimension: int
    target_dimension: int
    normalized_rank: int
    singular_values: tuple[float, ...]
    raw_normalized_operator_norm: float
    closest_partial_isometry_distance_fro: float
    source_isometry_error_fro: float
    target_coisometry_error_fro: float
    source_metric_condition_number: float
    target_metric_condition_number: float
    rank_relation: str
    universal_energy_preservation: bool
    contraction_requires_disposal: bool


@dataclass(frozen=True)
class WeightedPolarTransport:
    transport: FloatMatrix
    normalized_partial_isometry: FloatMatrix
    certificate: WeightedPolarCertificate


def weighted_polar_transport(
    raw_map: FloatMatrix,
    source_metric: FloatMatrix,
    target_metric: FloatMatrix,
) -> WeightedPolarTransport:
    """Closest metric-normalized partial isometry to a declared raw map.

    If target_dimension >= source_dimension, the result is a column isometry
    and preserves every source-state energy exactly.  If the target is smaller,
    no injective energy-preserving linear map exists; the result is a coisometry
    and the source component in its kernel must be retired explicitly.
    """
    raw = np.asarray(raw_map, dtype=np.float64)
    if raw.ndim != 2:
        raise ValueError("raw_map must be a matrix")
    target_dimension, source_dimension = raw.shape
    source_root, source_inverse_root, source_min, source_max = _spd_factors(
        source_metric
    )
    target_root, target_inverse_root, target_min, target_max = _spd_factors(
        target_metric
    )
    if source_metric.shape != (source_dimension, source_dimension):
        raise ValueError("source metric dimension mismatch")
    if target_metric.shape != (target_dimension, target_dimension):
        raise ValueError("target metric dimension mismatch")

    normalized = target_root @ raw @ source_inverse_root
    left, singular_values, right = np.linalg.svd(normalized, full_matrices=False)
    partial = left @ right
    transport = target_inverse_root @ partial @ source_root
    source_law = partial.T @ partial
    target_law = partial @ partial.T
    source_identity = np.eye(source_dimension)
    target_identity = np.eye(target_dimension)
    relation = (
        "equal"
        if target_dimension == source_dimension
        else "expansion"
        if target_dimension > source_dimension
        else "contraction"
    )
    universal = target_dimension >= source_dimension
    return WeightedPolarTransport(
        transport=transport,
        normalized_partial_isometry=partial,
        certificate=WeightedPolarCertificate(
            source_dimension=source_dimension,
            target_dimension=target_dimension,
            normalized_rank=int(np.linalg.matrix_rank(partial)),
            singular_values=tuple(float(value) for value in singular_values),
            raw_normalized_operator_norm=float(np.linalg.norm(normalized, ord=2)),
            closest_partial_isometry_distance_fro=float(
                np.linalg.norm(partial - normalized, ord="fro")
            ),
            source_isometry_error_fro=float(
                np.linalg.norm(source_law - source_identity, ord="fro")
            ),
            target_coisometry_error_fro=float(
                np.linalg.norm(target_law - target_identity, ord="fro")
            ),
            source_metric_condition_number=source_max / source_min,
            target_metric_condition_number=target_max / target_min,
            rank_relation=relation,
            universal_energy_preservation=universal,
            contraction_requires_disposal=target_dimension < source_dimension,
        ),
    )


@dataclass(frozen=True)
class TransportApplicationCertificate:
    initial_energy: float
    final_energy: float
    retired_energy: float
    retirement_radiation: float
    retirement_dissipation: float
    numerical_residual: float
    policy: DisposalPolicy


def apply_weighted_transport(
    state: FloatVector,
    plan: WeightedPolarTransport,
    source_metric: FloatMatrix,
    target_metric: FloatMatrix,
    *,
    policy: DisposalPolicy = "reject",
) -> tuple[FloatVector, TransportApplicationCertificate]:
    state = np.asarray(state, dtype=np.float64)
    if state.shape != (plan.certificate.source_dimension,):
        raise ValueError("state dimension does not match transport")
    if plan.certificate.contraction_requires_disposal and policy == "reject":
        raise ValueError("rank contraction requires radiate or dissipate policy")
    if policy not in ("radiate", "dissipate", "reject"):
        raise ValueError("unknown disposal policy")
    initial = weighted_energy(state, source_metric)
    target = plan.transport @ state
    final = weighted_energy(target, target_metric)
    retired = max(0.0, initial - final)
    residual = final + retired - initial
    return target, TransportApplicationCertificate(
        initial_energy=initial,
        final_energy=final,
        retired_energy=retired,
        retirement_radiation=retired if policy == "radiate" else 0.0,
        retirement_dissipation=retired if policy == "dissipate" else 0.0,
        numerical_residual=residual,
        policy=policy,
    )


def periodic_linear_resampling_matrix(
    source_samples: int, target_samples: int
) -> FloatMatrix:
    """Periodic piecewise-linear state correspondence before polar correction."""
    if source_samples < 2 or target_samples < 2:
        raise ValueError("both grids need at least two samples")
    matrix = np.zeros((target_samples, source_samples), dtype=np.float64)
    for target in range(target_samples):
        position = target * source_samples / target_samples
        left = int(math.floor(position)) % source_samples
        amount = position - math.floor(position)
        right = (left + 1) % source_samples
        matrix[target, left] += 1.0 - amount
        matrix[target, right] += amount
    return matrix


def periodic_cell_metric(samples: int) -> FloatMatrix:
    if samples < 2:
        raise ValueError("samples must be at least two")
    return np.eye(samples, dtype=np.float64) / samples


@dataclass(frozen=True)
class SparseTransportCertificate:
    maximum_nonzeros_per_row: int
    actual_nonzeros: int
    dense_nonzeros: int
    operator_error_fro: float
    worst_sampled_relative_energy_error: float
    worst_sampled_state_error: float
    seed: int
    samples: int


def sparsify_transport_rows(
    dense_transport: FloatMatrix,
    source_metric: FloatMatrix,
    target_metric: FloatMatrix,
    *,
    maximum_nonzeros_per_row: int = 4,
    sample_states: int = 256,
    seed: int = 0xA71AD,
) -> tuple[FloatMatrix, SparseTransportCertificate]:
    """Reference sparse approximation with explicit state/energy errors.

    This is intentionally not promoted as a production connection.  It gives a
    deterministic baseline against which a purpose-built local approximation
    can be judged.
    """
    dense = np.asarray(dense_transport, dtype=np.float64)
    if dense.ndim != 2 or maximum_nonzeros_per_row < 1:
        raise ValueError("invalid sparse approximation request")
    target_dimension, source_dimension = dense.shape
    sparse = np.zeros_like(dense)
    for row in range(target_dimension):
        keep = np.argsort(np.abs(dense[row]))[-maximum_nonzeros_per_row:]
        sparse[row, keep] = dense[row, keep]
    rng = np.random.default_rng(seed)
    worst_energy = 0.0
    worst_state = 0.0
    for _ in range(sample_states):
        state = rng.normal(size=source_dimension)
        energy = weighted_energy(state, source_metric)
        if energy <= np.finfo(float).tiny:
            continue
        state /= math.sqrt(energy)
        dense_state = dense @ state
        sparse_state = sparse @ state
        dense_energy = weighted_energy(dense_state, target_metric)
        sparse_energy = weighted_energy(sparse_state, target_metric)
        worst_energy = max(
            worst_energy,
            abs(sparse_energy - dense_energy) / max(dense_energy, np.finfo(float).tiny),
        )
        worst_state = max(worst_state, float(np.linalg.norm(sparse_state - dense_state)))
    return sparse, SparseTransportCertificate(
        maximum_nonzeros_per_row=maximum_nonzeros_per_row,
        actual_nonzeros=int(np.count_nonzero(sparse)),
        dense_nonzeros=int(np.count_nonzero(dense)),
        operator_error_fro=float(np.linalg.norm(sparse - dense, ord="fro")),
        worst_sampled_relative_energy_error=float(worst_energy),
        worst_sampled_state_error=float(worst_state),
        seed=seed,
        samples=sample_states,
    )


@dataclass(frozen=True)
class FixedCapacityConnectionCertificate:
    capacity_samples: int
    old_active_samples: float
    new_active_samples: float
    state_dimension_constant: bool
    connection_is_identity: bool
    metric_constant: bool
    retirement_required: bool
    interpretation: str


def fixed_capacity_material_connection(
    *,
    capacity_samples: int,
    old_active_samples: float,
    new_active_samples: float,
) -> FixedCapacityConnectionCertificate:
    if capacity_samples < 2:
        raise ValueError("capacity must be at least two")
    if not (
        1.0 <= old_active_samples <= capacity_samples
        and 1.0 <= new_active_samples <= capacity_samples
    ):
        raise ValueError("active delay must remain inside fixed capacity")
    return FixedCapacityConnectionCertificate(
        capacity_samples=capacity_samples,
        old_active_samples=float(old_active_samples),
        new_active_samples=float(new_active_samples),
        state_dimension_constant=True,
        connection_is_identity=True,
        metric_constant=True,
        retirement_required=False,
        interpretation=(
            "Samples retain material identities in a fixed-capacity ring. Retuning changes "
            "the propagation/read interpolation law, not the state fiber."
        ),
    )
