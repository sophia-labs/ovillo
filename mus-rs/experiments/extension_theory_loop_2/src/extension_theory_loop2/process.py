from __future__ import annotations

import itertools
from dataclasses import asdict, dataclass
from typing import Any, Iterable

import numpy as np
import torch
from torch import Tensor

@dataclass(frozen=True)
class ProcessSpec:
    """Finite observable process used as a solvable predictive system.

    ``emission_concentration`` controls how peaky state-conditioned emissions are:
    smaller values make the random background sparse, while ``anchor`` adds a
    state-specific preferred symbol. ``persistence`` blends a random transition
    matrix with the identity and therefore changes both memory and operator
    commutators.
    """

    name: str
    states: int = 6
    symbols: int = 4
    persistence: float = 0.94
    emission_concentration: float = 0.65
    anchor: float = 2.0
    transition_kind: str = "random"  # random | cycle | reversible | local_ring
    seed: int = 41


class ObservableHMM:
    """Stationary finite-state hidden Markov process with exact Bayesian filter."""

    def __init__(self, spec: ProcessSpec):
        self.spec = spec
        rng = np.random.default_rng(spec.seed)
        n = spec.states

        if spec.transition_kind == "random":
            raw = rng.gamma(1.2, 1.0, size=(n, n))
            raw /= raw.sum(axis=1, keepdims=True)
        elif spec.transition_kind == "cycle":
            raw = np.full((n, n), 0.03 / max(n - 2, 1), dtype=np.float64)
            for i in range(n):
                raw[i, (i + 1) % n] = 0.72
                raw[i, (i - 1) % n] = 0.25
            raw /= raw.sum(axis=1, keepdims=True)
        elif spec.transition_kind == "reversible":
            w = rng.gamma(1.3, 1.0, size=(n, n))
            w = (w + w.T) / 2
            raw = w / w.sum(axis=1, keepdims=True)
        elif spec.transition_kind == "local_ring":
            raw = np.zeros((n, n), dtype=np.float64)
            for i in range(n):
                raw[i, i] = 0.25
                raw[i, (i + 1) % n] = 0.45
                raw[i, (i - 1) % n] = 0.30
        else:
            raise ValueError(f"unknown transition kind: {spec.transition_kind}")

        self.P = spec.persistence * np.eye(n) + (1.0 - spec.persistence) * raw
        self.P /= self.P.sum(axis=1, keepdims=True)

        emission = rng.gamma(
            spec.emission_concentration,
            1.0,
            size=(spec.states, spec.symbols),
        )
        for i in range(spec.states):
            emission[i, i % spec.symbols] += spec.anchor
        self.E = emission / emission.sum(axis=1, keepdims=True)

        values, vectors = np.linalg.eig(self.P.T)
        stationary = np.real(vectors[:, np.argmin(np.abs(values - 1.0))])
        stationary = np.abs(stationary)
        self.pi = stationary / stationary.sum()
        self.ops = np.stack([np.diag(self.E[:, x]) @ self.P for x in range(spec.symbols)])

    @property
    def states(self) -> int:
        return self.spec.states

    @property
    def symbols(self) -> int:
        return self.spec.symbols

    def sample(self, batch: int, length: int, generator: torch.Generator) -> Tensor:
        p = torch.tensor(self.P, dtype=torch.float32)
        e = torch.tensor(self.E, dtype=torch.float32)
        pi = torch.tensor(self.pi, dtype=torch.float32)
        hidden = torch.multinomial(pi.expand(batch, -1), 1, generator=generator).squeeze(1)
        tokens: list[Tensor] = []
        for _ in range(length):
            x = torch.multinomial(e[hidden], 1, generator=generator).squeeze(1)
            tokens.append(x)
            hidden = torch.multinomial(p[hidden], 1, generator=generator).squeeze(1)
        return torch.stack(tokens, dim=1)

    def sample_stream(self, length: int, seed: int) -> np.ndarray:
        rng = np.random.default_rng(seed)
        hidden = int(rng.choice(self.states, p=self.pi))
        out = np.empty(length, dtype=np.int16)
        for t in range(length):
            x = int(rng.choice(self.symbols, p=self.E[hidden]))
            out[t] = x
            hidden = int(rng.choice(self.states, p=self.P[hidden]))
        return out

    def beliefs(self, tokens: Tensor) -> Tensor:
        belief = torch.tensor(self.pi, dtype=torch.float64).expand(tokens.shape[0], -1).clone()
        e = torch.tensor(self.E, dtype=torch.float64)
        p = torch.tensor(self.P, dtype=torch.float64)
        rows: list[Tensor] = []
        for t in range(tokens.shape[1]):
            likelihood = e[:, tokens[:, t]].T
            posterior = belief * likelihood
            posterior /= posterior.sum(dim=1, keepdim=True)
            belief = posterior @ p
            rows.append(belief.clone())
        return torch.stack(rows, dim=1)

    def oracle_logits(self, beliefs: Tensor) -> Tensor:
        emission = torch.tensor(self.E, dtype=beliefs.dtype, device=beliefs.device)
        return (beliefs @ emission).clamp_min(1e-12).log()

    def future_marginals(self, beliefs: Tensor, horizon: int = 4) -> Tensor:
        """Return next-symbol marginals for horizons 1..horizon."""
        p = torch.tensor(self.P, dtype=beliefs.dtype, device=beliefs.device)
        e = torch.tensor(self.E, dtype=beliefs.dtype, device=beliefs.device)
        b = beliefs
        rows = []
        for _ in range(horizon):
            rows.append(b @ e)
            b = b @ p
        return torch.cat(rows, dim=-1)

    def word_probability(self, word: Iterable[int]) -> float:
        row = self.pi.copy()
        for symbol in word:
            row = row @ self.ops[int(symbol)]
        return float(row.sum())

    def diagnostics(self) -> dict[str, Any]:
        commutators = []
        for i in range(self.symbols):
            for j in range(i + 1, self.symbols):
                commutators.append(
                    np.linalg.norm(self.ops[i] @ self.ops[j] - self.ops[j] @ self.ops[i])
                )
        return {
            "spec": asdict(self.spec),
            "transition_eigenvalues_abs": sorted(
                np.abs(np.linalg.eigvals(self.P)).tolist(), reverse=True
            ),
            "mean_operator_commutator": float(np.mean(commutators)),
            "max_operator_commutator": float(np.max(commutators)),
            "transition": self.P.tolist(),
            "emission": self.E.tolist(),
            "stationary": self.pi.tolist(),
        }


def all_words(symbols: int, max_length: int) -> list[tuple[int, ...]]:
    words: list[tuple[int, ...]] = [()]
    for length in range(1, max_length + 1):
        words.extend(itertools.product(range(symbols), repeat=length))
    return words


def teacher_hankel(process: ObservableHMM, max_length: int = 3) -> np.ndarray:
    words = all_words(process.symbols, max_length)
    return np.array(
        [[process.word_probability(u + v) for v in words] for u in words],
        dtype=np.float64,
    )
