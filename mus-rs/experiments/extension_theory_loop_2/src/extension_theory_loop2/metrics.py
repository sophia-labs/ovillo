from __future__ import annotations

from typing import Any

import numpy as np
import scipy.linalg
import torch
from torch import Tensor, nn

from .core import (
    LinearStateModel,
    ObservableHMM,
    PositiveProjectiveModel,
    SmallGRU,
    StableCliffordBank,
    participation_rank,
)


def _invsqrt(matrix: np.ndarray, ridge: float = 1e-5) -> np.ndarray:
    matrix = (matrix + matrix.T) / 2
    values, vectors = scipy.linalg.eigh(matrix, check_finite=False)
    floor = ridge * (np.trace(matrix) / max(len(matrix), 1) + 1e-12)
    values = np.maximum(values, floor)
    return (vectors * (1.0 / np.sqrt(values))[None, :]) @ vectors.T


def ridge_regression_r2_modes(
    x_train: np.ndarray,
    y_train: np.ndarray,
    x_test: np.ndarray,
    y_test: np.ndarray,
    ridge: float = 1e-3,
) -> dict[str, Any]:
    """Predict future PCA modes from model state and return variance-weighted R²."""

    x_mean = x_train.mean(axis=0)
    x_scale = x_train.std(axis=0) + 1e-6
    x = (x_train - x_mean) / x_scale
    xt = (x_test - x_mean) / x_scale

    y_mean = y_train.mean(axis=0)
    y = y_train - y_mean
    yt = y_test - y_mean
    _, singular, vh = scipy.linalg.svd(
        y / np.sqrt(max(len(y) - 1, 1)), full_matrices=False, check_finite=False
    )
    basis = vh.T
    scores = y @ basis
    score_test = yt @ basis

    design = np.c_[x, np.ones(len(x))]
    gram = design.T @ design + ridge * np.eye(design.shape[1])
    coefficient = scipy.linalg.solve(
        gram, design.T @ scores, assume_a="pos", check_finite=False
    )
    prediction = np.c_[xt, np.ones(len(xt))] @ coefficient

    variances = score_test.var(axis=0)
    residual = ((prediction - score_test) ** 2).mean(axis=0)
    r2 = 1.0 - residual / np.maximum(variances, 1e-12)
    active = singular > singular[0] * 1e-6 if singular[0] > 0 else np.zeros_like(singular, dtype=bool)
    positive = np.maximum(r2, 0.0)
    weighted = float(np.sum(variances[active] * positive[active]) / np.maximum(variances[active].sum(), 1e-12))
    reported = [float(value) if is_active else None for value, is_active in zip(r2, active)]
    return {
        "future_mode_singular_values": singular.tolist(),
        "future_mode_r2": reported,
        "weighted_future_r2": weighted,
        "future_modes_active": int(active.sum()),
        "future_modes_r2_above_0_5": int(((r2 > 0.5) & active).sum()),
        "future_modes_positive": int(((r2 > 0.0) & active).sum()),
    }


def belief_probe(
    x_train: np.ndarray,
    belief_train: np.ndarray,
    x_test: np.ndarray,
    belief_test: np.ndarray,
    ridge: float = 1e-3,
) -> dict[str, float]:
    x_mean = x_train.mean(axis=0)
    x_scale = x_train.std(axis=0) + 1e-6
    x = (x_train - x_mean) / x_scale
    xt = (x_test - x_mean) / x_scale
    eps = 1e-8
    y = np.log(belief_train[:, :-1] + eps) - np.log(belief_train[:, -1:] + eps)
    yt = np.log(belief_test[:, :-1] + eps) - np.log(belief_test[:, -1:] + eps)
    design = np.c_[x, np.ones(len(x))]
    coefficient = scipy.linalg.solve(
        design.T @ design + ridge * np.eye(design.shape[1]),
        design.T @ y,
        assume_a="pos",
        check_finite=False,
    )
    prediction = np.c_[xt, np.ones(len(xt))] @ coefficient
    denominator = ((yt - yt.mean(axis=0)) ** 2).sum()
    return {
        "r2": float(1.0 - ((prediction - yt) ** 2).sum() / max(denominator, 1e-12)),
        "rmse": float(np.sqrt(np.mean((prediction - yt) ** 2))),
    }


def _state_covariance(states: np.ndarray) -> np.ndarray:
    covariance = np.cov(states, rowvar=False, bias=False)
    if np.ndim(covariance) == 0:
        covariance = np.array([[float(covariance)]])
    return covariance + 1e-8 * np.eye(covariance.shape[0])


def _clifford_observability(model: StableCliffordBank) -> np.ndarray:
    operators, _ = model.programs()
    retention = torch.sigmoid(model.raw_retention)
    weight = model.readout.weight.detach()
    n = model.state_size
    current = weight[:, :n]
    transition = weight[:, n : 2 * n] if model.use_transition else torch.zeros_like(current)
    accumulated = np.zeros((n, n), dtype=np.float64)
    for symbol in range(model.symbols):
        blocks = [retention[f] * operators[f, symbol] for f in range(model.fibers)]
        a = torch.block_diag(*blocks)
        # logits = W_current z' + W_delta(z'-z)
        jacobian = (current + transition) @ a.T - transition
        j = jacobian.detach().cpu().numpy()
        accumulated += j.T @ j
    return accumulated / model.symbols


def _projective_observability(
    model: PositiveProjectiveModel, states: np.ndarray, samples: int = 128
) -> np.ndarray:
    accumulated = np.zeros((model.rank, model.rank), dtype=np.float64)
    count = min(samples, len(states))
    for i in range(count):
        state = torch.tensor(states[i], dtype=torch.float32, requires_grad=True).view(1, -1)
        logits = model.next_probabilities(state).clamp_min(1e-9).log()
        rows = []
        for symbol in range(model.symbols):
            rows.append(
                torch.autograd.grad(logits[0, symbol], state, retain_graph=True)[0]
                .reshape(-1)
                .detach()
                .cpu()
                .numpy()
            )
        jacobian = np.stack(rows)
        accumulated += jacobian.T @ jacobian
    return accumulated / max(count, 1)


def output_observability(model: nn.Module, states: np.ndarray) -> np.ndarray:
    if isinstance(model, StableCliffordBank):
        return _clifford_observability(model)
    if isinstance(model, PositiveProjectiveModel):
        return _projective_observability(model, states)
    if isinstance(model, (SmallGRU, LinearStateModel)):
        weight = model.readout.weight.detach().cpu().numpy()
        return weight.T @ weight
    raise TypeError(f"unsupported model for observability: {type(model)!r}")


def balanced_spectrum(controllability: np.ndarray, observability: np.ndarray) -> dict[str, Any]:
    values, vectors = scipy.linalg.eigh(
        (controllability + controllability.T) / 2, check_finite=False
    )
    values = np.maximum(values, 0.0)
    root = (vectors * np.sqrt(values)[None, :]) @ vectors.T
    matrix = root @ observability @ root
    eigenvalues = np.maximum(
        scipy.linalg.eigvalsh((matrix + matrix.T) / 2, check_finite=False), 0.0
    )
    singular = np.sqrt(eigenvalues)[::-1]
    return {
        "singular_values": singular.tolist(),
        "participation_rank": participation_rank(singular),
        "rank_rel_1e-4": int((singular > singular[0] * 1e-4).sum()) if singular[0] else 0,
        "trace": float(singular.sum()),
    }


@torch.no_grad()
def collect_state_dataset(
    model: nn.Module,
    process: ObservableHMM,
    *,
    samples: int,
    history_length: int,
    seed: int,
    future_horizon: int = 4,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    generator = torch.Generator().manual_seed(seed)
    tokens = process.sample(samples, history_length, generator)
    state = model.extract_state(tokens).detach().cpu().numpy()
    belief = process.beliefs(tokens)[:, -1].cpu().numpy()
    future = process.future_marginals(
        torch.tensor(belief, dtype=torch.float64), future_horizon
    ).cpu().numpy()
    return state, belief, future


def analyze_predictive_state(
    model: nn.Module,
    process: ObservableHMM,
    *,
    seed: int,
    train_samples: int = 6000,
    test_samples: int = 3000,
    history_length: int = 28,
    future_horizon: int = 4,
) -> dict[str, Any]:
    model.eval()
    x, belief, future = collect_state_dataset(
        model,
        process,
        samples=train_samples,
        history_length=history_length,
        seed=seed,
        future_horizon=future_horizon,
    )
    xt, belief_t, future_t = collect_state_dataset(
        model,
        process,
        samples=test_samples,
        history_length=history_length,
        seed=seed + 1,
        future_horizon=future_horizon,
    )

    covariance = _state_covariance(x)
    observability = output_observability(model, x)
    state_centered = x - x.mean(axis=0)
    state_singular = np.linalg.svd(
        state_centered / np.sqrt(max(len(x) - 1, 1)), compute_uv=False
    )
    future_result = ridge_regression_r2_modes(x, future, xt, future_t)
    return {
        "state_singular_values": state_singular.tolist(),
        "state_participation_rank": participation_rank(state_singular**2),
        "belief_probe": belief_probe(x, belief, xt, belief_t),
        "future_prediction": future_result,
        "balanced": balanced_spectrum(covariance, observability),
        "controllability_trace": float(np.trace(covariance)),
        "observability_trace": float(np.trace(observability)),
    }


def spectral_overlap(data: list[float], model: list[float]) -> float:
    """Bhattacharyya overlap of normalized nonnegative spectra, zero padded."""
    n = max(len(data), len(model))
    p = np.zeros(n, dtype=np.float64)
    q = np.zeros(n, dtype=np.float64)
    p[: len(data)] = np.maximum(data, 0.0)
    q[: len(model)] = np.maximum(model, 0.0)
    if p.sum() <= 0 or q.sum() <= 0:
        return 0.0
    p /= p.sum()
    q /= q.sum()
    return float(np.sqrt(p * q).sum())
