from __future__ import annotations

from typing import Any

import numpy as np

from .process import ObservableHMM

def participation_rank(values: np.ndarray | list[float], squared: bool = False) -> float:
    x = np.asarray(values, dtype=np.float64)
    if squared:
        x = x**2
    denom = float(x @ x)
    if denom <= 1e-30:
        return 0.0
    return float(x.sum() ** 2 / denom)


def spectrum(matrix: np.ndarray, limit: int = 20) -> dict[str, Any]:
    singular = np.linalg.svd(matrix, compute_uv=False)
    return {
        "singular_values": singular[:limit].tolist(),
        "rank_rel_1e-8": int((singular > singular[0] * 1e-8).sum()),
        "rank_rel_1e-5": int((singular > singular[0] * 1e-5).sum()),
        "participation_rank": participation_rank(singular),
        "condition_active_1e-8": float(
            singular[0] / singular[singular > singular[0] * 1e-8][-1]
        ),
    }


def tangent_basis(n: int) -> np.ndarray:
    a = np.eye(n)[:, :-1] - np.eye(n)[:, -1:]
    q, _ = np.linalg.qr(a)
    return q


def filter_jacobian(process: ObservableHMM, belief: np.ndarray, symbol: int) -> np.ndarray:
    emission = process.E[:, symbol]
    normalizer = belief @ emission
    derivative_posterior = (
        np.diag(emission) / normalizer
        - np.outer(emission, belief * emission) / (normalizer**2)
    )
    return derivative_posterior @ process.P


def filter_lyapunov_spectrum(
    process: ObservableHMM,
    *,
    steps: int = 12_000,
    burn_in: int = 1_000,
    seed: int = 2026,
) -> dict[str, Any]:
    rng = np.random.default_rng(seed)
    belief = process.pi.copy()
    q = tangent_basis(process.states)
    logs = np.zeros(process.states - 1, dtype=np.float64)
    hidden = int(rng.choice(process.states, p=process.pi))
    one_step = []

    for t in range(steps + burn_in):
        symbol = int(rng.choice(process.symbols, p=process.E[hidden]))
        hidden = int(rng.choice(process.states, p=process.P[hidden]))
        jac = filter_jacobian(process, belief, symbol)

        posterior = belief * process.E[:, symbol]
        posterior /= posterior.sum()
        belief = posterior @ process.P

        if t >= burn_in:
            basis = tangent_basis(process.states)
            reduced = basis.T @ jac @ basis
            one_step.append(np.linalg.svd(reduced, compute_uv=False))
            z = jac.T @ q
            q, r = np.linalg.qr(z)
            logs += np.log(np.maximum(np.abs(np.diag(r)), 1e-30))

    exponents = np.exp(logs / steps)
    one = np.asarray(one_step)
    return {
        "retentions": sorted(exponents.tolist(), reverse=True),
        "mean_one_step_singular_values": np.mean(one, axis=0).tolist(),
        "geometric_mean_one_step_singular_values": np.exp(
            np.mean(np.log(np.maximum(one, 1e-30)), axis=0)
        ).tolist(),
        "steps": steps,
    }
