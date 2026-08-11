"""Explicit claim boundary for bounded nonlinear stages."""

from __future__ import annotations

from dataclasses import dataclass
import math


@dataclass(frozen=True)
class NonlinearEnvelopeCertificate:
    duration_seconds: float
    spectral_gap: float
    jacobian_operator_norm_bound: float
    forcing_norm_bound: float
    linearized_cluster_preserved: bool
    trajectory_deviation_upper_bound: float
    accepted_for_linear_topological_claim: bool
    refusal_reason: str | None


def certify_nonlinear_envelope(
    *,
    duration_seconds: float,
    spectral_gap: float,
    jacobian_operator_norm_bound: float,
    forcing_norm_bound: float,
    trajectory_error_budget: float,
) -> NonlinearEnvelopeCertificate:
    """Weyl plus Gronwall receipt for z_dot=G(t)z+N(z,t).

    The C2 statement belongs to the linear conservative family.  This function
    only certifies that a bounded nonlinear Jacobian cannot close its isolated
    rank-two linearized cluster and that the nonlinear trajectory remains in a
    declared tube around the linear trajectory.
    """
    values = (
        duration_seconds,
        spectral_gap,
        jacobian_operator_norm_bound,
        forcing_norm_bound,
        trajectory_error_budget,
    )
    if any(not math.isfinite(value) for value in values):
        raise ValueError("all bounds must be finite")
    if duration_seconds < 0.0 or spectral_gap <= 0.0:
        raise ValueError("duration must be nonnegative and gap positive")
    if jacobian_operator_norm_bound < 0.0 or forcing_norm_bound < 0.0:
        raise ValueError("nonlinear bounds must be nonnegative")
    if trajectory_error_budget < 0.0:
        raise ValueError("trajectory budget must be nonnegative")

    lipschitz = jacobian_operator_norm_bound
    if lipschitz <= 1.0e-15:
        deviation = forcing_norm_bound * duration_seconds
    else:
        deviation = forcing_norm_bound * math.expm1(lipschitz * duration_seconds) / lipschitz
    cluster_preserved = 2.0 * jacobian_operator_norm_bound < spectral_gap
    accepted = cluster_preserved and deviation <= trajectory_error_budget
    reason = None
    if not cluster_preserved:
        reason = "nonlinear Jacobian radius can close the declared linearized gap"
    elif deviation > trajectory_error_budget:
        reason = "Gronwall trajectory tube exceeds the requested budget"
    return NonlinearEnvelopeCertificate(
        duration_seconds=duration_seconds,
        spectral_gap=spectral_gap,
        jacobian_operator_norm_bound=jacobian_operator_norm_bound,
        forcing_norm_bound=forcing_norm_bound,
        linearized_cluster_preserved=cluster_preserved,
        trajectory_deviation_upper_bound=deviation,
        accepted_for_linear_topological_claim=accepted,
        refusal_reason=reason,
    )
