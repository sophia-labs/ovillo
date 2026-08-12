from __future__ import annotations

import argparse
import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np
import scipy.linalg


@dataclass(frozen=True)
class CubeSpec:
    dt: float = 0.70
    edge_rates: tuple[float, float, float] = (0.55, 0.35, 0.20)
    emission_anchor: float = 2.5
    emission_background: float = 0.45
    seed: int = 77


class CubeCompartmentProcess:
    """Eight positive compartments on a physical cube with local diffusion."""

    def __init__(self, spec: CubeSpec = CubeSpec()):
        self.spec = spec
        self.coordinates = np.array(
            [[(index >> axis) & 1 for axis in range(3)] for index in range(8)],
            dtype=np.float64,
        )
        q = np.zeros((8, 8), dtype=np.float64)
        self.edges: list[tuple[int, int, int, float]] = []
        for i in range(8):
            for axis, rate in enumerate(spec.edge_rates):
                j = i ^ (1 << axis)
                if i < j:
                    q[i, j] = q[j, i] = rate
                    self.edges.append((i, j, axis, rate))
        np.fill_diagonal(q, -q.sum(axis=1))
        self.Q = q
        self.P = scipy.linalg.expm(spec.dt * q)
        self.pi = np.full(8, 1.0 / 8)

        rng = np.random.default_rng(spec.seed)
        emission = rng.gamma(spec.emission_background, 1.0, size=(8, 4))
        for state in range(8):
            # Each symbol identifies a face-pair pattern, not a unique compartment.
            preferred = ((state & 1) + 2 * ((state >> 1) & 1)) % 4
            emission[state, preferred] += spec.emission_anchor
        self.E = emission / emission.sum(axis=1, keepdims=True)

    def local_step(self, microsteps: int) -> np.ndarray:
        step = np.eye(8) + (self.spec.dt / microsteps) * self.Q
        if step.min() < -1e-12:
            raise ValueError("microstep is not positivity preserving")
        return step

    def approximate_transition(self, microsteps: int) -> np.ndarray:
        return np.linalg.matrix_power(self.local_step(microsteps), microsteps)

    def sample(self, batch: int, length: int, seed: int) -> np.ndarray:
        rng = np.random.default_rng(seed)
        hidden = rng.choice(8, size=batch, p=self.pi)
        tokens = np.empty((batch, length), dtype=np.int16)
        for t in range(length):
            for b in range(batch):
                tokens[b, t] = rng.choice(4, p=self.E[hidden[b]])
                hidden[b] = rng.choice(8, p=self.P[hidden[b]])
        return tokens

    def filter(self, tokens: np.ndarray, transition: np.ndarray) -> tuple[np.ndarray, float]:
        belief = np.repeat(self.pi[None, :], len(tokens), axis=0)
        nll = 0.0
        for t in range(tokens.shape[1] - 1):
            likelihood = self.E[:, tokens[:, t]].T
            posterior = belief * likelihood
            posterior /= posterior.sum(axis=1, keepdims=True)
            belief = posterior @ transition
            probability = belief @ self.E
            nll -= np.log(np.maximum(probability[np.arange(len(tokens)), tokens[:, t + 1]], 1e-15)).sum()
        return belief, nll / (tokens.shape[0] * (tokens.shape[1] - 1))


def entropy(probability: np.ndarray) -> np.ndarray:
    return -(probability * np.log(np.maximum(probability, 1e-15))).sum(axis=-1)


def storage(probability: np.ndarray) -> np.ndarray:
    return 0.5 * (probability**2).sum(axis=-1)


def ledger_run(process: CubeCompartmentProcess, microsteps: int, seed: int = 11) -> dict[str, Any]:
    rng = np.random.default_rng(seed)
    step = process.local_step(microsteps)
    belief = rng.dirichlet(np.ones(8), size=512)
    symbols = rng.integers(0, 4, size=len(belief))

    energy_start = storage(belief)
    entropy_start = entropy(belief)
    likelihood = process.E[:, symbols].T
    unnormalized = belief * likelihood
    evidence = unnormalized.sum(axis=1)
    posterior = unnormalized / evidence[:, None]
    energy_post = storage(posterior)
    entropy_post = entropy(posterior)
    evidence_work = energy_post - energy_start

    state = posterior.copy()
    dissipation = np.zeros(len(state))
    maximum_mass_residual = 0.0
    minimum_state = float(state.min())
    dirichlet_integral = np.zeros(len(state))
    correction = np.zeros(len(state))
    h = process.spec.dt / microsteps
    for _ in range(microsteps):
        before = storage(state)
        q_velocity = state @ process.Q
        # For symmetric Q, -b Q b^T is the graph Dirichlet form.
        dirichlet_integral += -h * np.einsum("bi,bi->b", state, q_velocity)
        correction += 0.5 * h * h * (q_velocity**2).sum(axis=1)
        state = state @ step
        after = storage(state)
        dissipation += before - after
        maximum_mass_residual = max(
            maximum_mass_residual, float(np.max(np.abs(state.sum(axis=1) - 1.0)))
        )
        minimum_state = min(minimum_state, float(state.min()))

    energy_end = storage(state)
    entropy_end = entropy(state)
    closure = energy_end - energy_start - evidence_work + dissipation
    local_dense = posterior @ np.linalg.matrix_power(step, microsteps)
    return {
        "microsteps": microsteps,
        "minimum_state": minimum_state,
        "maximum_mass_residual": maximum_mass_residual,
        "mean_evidence_surprisal": float(np.mean(-np.log(np.maximum(evidence, 1e-15)))),
        "mean_evidence_storage_work": float(evidence_work.mean()),
        "mean_diffusive_dissipation": float(dissipation.mean()),
        "mean_dirichlet_term": float(dirichlet_integral.mean()),
        "mean_discrete_correction": float(correction.mean()),
        "mean_entropy_evidence_change": float((entropy_post - entropy_start).mean()),
        "mean_entropy_diffusion_change": float((entropy_end - entropy_post).mean()),
        "maximum_storage_ledger_residual": float(np.max(np.abs(closure))),
        "maximum_local_dense_state_residual": float(np.max(np.abs(state - local_dense))),
    }


def run(output: Path) -> dict[str, Any]:
    process = CubeCompartmentProcess()
    tokens = process.sample(4096, 64, seed=2026)
    _, oracle_nll = process.filter(tokens, process.P)
    rows = []
    for microsteps in [1, 2, 4, 8, 16, 32, 64]:
        approximate = process.approximate_transition(microsteps)
        _, nll = process.filter(tokens, approximate)
        ledger = ledger_run(process, microsteps)
        rows.append(
            {
                "microsteps": microsteps,
                "transition_frobenius_error": float(np.linalg.norm(approximate - process.P)),
                "transition_max_error": float(np.max(np.abs(approximate - process.P))),
                "nll_nats": nll,
                "excess_bps": (nll - oracle_nll) / math.log(2),
                "nonzero_off_diagonal_local_edges": int(
                    np.count_nonzero(process.local_step(microsteps) - np.diag(np.diag(process.local_step(microsteps))))
                ),
                "dense_transition_nonzeros": int(np.count_nonzero(np.abs(approximate) > 1e-14)),
                "ledger": ledger,
            }
        )
    payload = {
        "spec": {
            "dt": process.spec.dt,
            "edge_rates": process.spec.edge_rates,
            "compartments": 8,
            "local_undirected_edges": len(process.edges),
            "symbols": 4,
        },
        "oracle_nll_nats": oracle_nll,
        "oracle_bps": oracle_nll / math.log(2),
        "transition": process.P.tolist(),
        "generator": process.Q.tolist(),
        "emission": process.E.tolist(),
        "rows": rows,
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    print(json.dumps(payload, indent=2))
    return payload


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    run(args.output)


if __name__ == "__main__":
    main()
