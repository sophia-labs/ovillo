from __future__ import annotations

from typing import Any

import numpy as np
import torch
from torch import Tensor, nn
import torch.nn.functional as F

class PositiveProjectiveModel(nn.Module):
    """Positive weighted-automaton realization in projective coordinates."""

    def __init__(self, symbols: int, rank: int):
        super().__init__()
        self.symbols = symbols
        self.rank = rank
        raw = torch.randn(symbols, rank, rank) * 0.15
        raw += torch.eye(rank)[None]
        self.raw_ops = nn.Parameter(raw)
        self.raw_terminal = nn.Parameter(torch.zeros(rank))
        self.raw_initial = nn.Parameter(torch.zeros(rank))

    @property
    def state_size(self) -> int:
        return self.rank

    def operators(self) -> Tensor:
        operators = F.softplus(self.raw_ops) + 1e-6
        # A single global scale is projectively irrelevant, while symbol-specific
        # scales carry predictive probability and must not be normalized away.
        return operators / operators.mean()

    def terminal(self) -> Tensor:
        return F.softplus(self.raw_terminal) + 1e-5

    def initial_state(self, batch: int) -> Tensor:
        state = F.softplus(self.raw_initial) + 1e-5
        terminal = self.terminal()
        state = state / (state @ terminal).clamp_min(1e-8)
        return state.expand(batch, -1)

    def next_probabilities(self, state: Tensor, operators: Tensor | None = None) -> Tensor:
        operators = self.operators() if operators is None else operators
        terminal = self.terminal()
        scores = torch.einsum("bi,xij,j->bx", state, operators, terminal)
        return scores / scores.sum(dim=-1, keepdim=True).clamp_min(1e-8)

    def step(
        self, state: Tensor, symbol: Tensor, operators: Tensor | None = None
    ) -> tuple[Tensor, Tensor]:
        operators = self.operators() if operators is None else operators
        selected = operators[symbol]
        state = torch.einsum("bi,bij->bj", state, selected)
        terminal = self.terminal()
        state = state / (state @ terminal).unsqueeze(-1).clamp_min(1e-8)
        probabilities = self.next_probabilities(state, operators)
        return state, probabilities.clamp_min(1e-9).log()

    def forward_with_states(self, tokens: Tensor) -> tuple[Tensor, Tensor]:
        operators = self.operators()
        state = self.initial_state(tokens.shape[0])
        logits, states = [], []
        for t in range(tokens.shape[1]):
            state, output = self.step(state, tokens[:, t], operators)
            logits.append(output)
            states.append(state)
        return torch.stack(logits, dim=1), torch.stack(states, dim=1)

    def forward(self, tokens: Tensor) -> Tensor:
        return self.forward_with_states(tokens)[0]

    def extract_state(self, tokens: Tensor) -> Tensor:
        return self.forward_with_states(tokens)[1][:, -1]

    @torch.no_grad()
    def diagnostics(self) -> dict[str, Any]:
        operators = self.operators()
        commutators = []
        for i in range(self.symbols):
            for j in range(i + 1, self.symbols):
                commutators.append(
                    float(
                        torch.linalg.matrix_norm(
                            operators[i] @ operators[j] - operators[j] @ operators[i]
                        )
                    )
                )
        return {
            "kind": "positive_projective_wfa",
            "rank": self.rank,
            "state_size": self.state_size,
            "parameters": sum(parameter.numel() for parameter in self.parameters()),
            "mean_commutator": float(np.mean(commutators)),
        }


class LinearStateModel(nn.Module):
    """Unconstrained symbol-conditioned affine linear recurrence."""

    def __init__(self, symbols: int, hidden: int):
        super().__init__()
        self.symbols = symbols
        self.hidden = hidden
        self.raw_a = nn.Parameter(torch.randn(symbols, hidden, hidden) * 0.08)
        self.bias = nn.Parameter(torch.randn(symbols, hidden) * 0.05)
        self.initial = nn.Parameter(torch.zeros(hidden))
        self.readout = nn.Linear(hidden, symbols)

    @property
    def state_size(self) -> int:
        return self.hidden

    def matrices(self) -> Tensor:
        # Stable residual parameterization with bounded operator norm in practice.
        return 0.88 * torch.eye(self.hidden, device=self.raw_a.device)[None] + 0.12 * torch.tanh(
            self.raw_a
        )

    def forward_with_states(self, tokens: Tensor) -> tuple[Tensor, Tensor]:
        matrices = self.matrices()
        state = self.initial.expand(tokens.shape[0], -1)
        logits, states = [], []
        for t in range(tokens.shape[1]):
            state = torch.einsum("bi,bij->bj", state, matrices[tokens[:, t]]) + self.bias[
                tokens[:, t]
            ]
            state = torch.tanh(state)
            logits.append(self.readout(state))
            states.append(state)
        return torch.stack(logits, dim=1), torch.stack(states, dim=1)

    def forward(self, tokens: Tensor) -> Tensor:
        return self.forward_with_states(tokens)[0]

    def extract_state(self, tokens: Tensor) -> Tensor:
        return self.forward_with_states(tokens)[1][:, -1]

    @torch.no_grad()
    def diagnostics(self) -> dict[str, Any]:
        return {
            "kind": "affine_linear_state",
            "state_size": self.state_size,
            "parameters": sum(parameter.numel() for parameter in self.parameters()),
        }


class SmallGRU(nn.Module):
    def __init__(self, symbols: int, hidden: int, embedding: int = 8):
        super().__init__()
        self.hidden = hidden
        self.embedding = nn.Embedding(symbols, embedding)
        self.gru = nn.GRU(embedding, hidden, batch_first=True)
        self.readout = nn.Linear(hidden, symbols)

    @property
    def state_size(self) -> int:
        return self.hidden

    def forward_with_states(self, tokens: Tensor) -> tuple[Tensor, Tensor]:
        hidden, _ = self.gru(self.embedding(tokens))
        return self.readout(hidden), hidden

    def forward(self, tokens: Tensor) -> Tensor:
        return self.forward_with_states(tokens)[0]

    def extract_state(self, tokens: Tensor) -> Tensor:
        return self.forward_with_states(tokens)[1][:, -1]

    @torch.no_grad()
    def diagnostics(self) -> dict[str, Any]:
        return {
            "kind": "gru",
            "state_size": self.state_size,
            "parameters": sum(parameter.numel() for parameter in self.parameters()),
        }


class ExactPositiveRealization(nn.Module):
    """Exact positive observable operator model for a known HMM."""

    def __init__(self, process: ObservableHMM):
        super().__init__()
        self.rank = process.states
        self.symbols = process.symbols
        self.register_buffer("ops", torch.tensor(process.ops, dtype=torch.float32))
        self.register_buffer("terminal", torch.ones(process.states, dtype=torch.float32))
        self.register_buffer("initial", torch.tensor(process.pi, dtype=torch.float32))

    @property
    def state_size(self) -> int:
        return self.rank

    def forward_with_states(self, tokens: Tensor) -> tuple[Tensor, Tensor]:
        state = self.initial.expand(tokens.shape[0], -1)
        logits, states = [], []
        for t in range(tokens.shape[1]):
            state = torch.einsum("bi,bij->bj", state, self.ops[tokens[:, t]])
            state = state / state.sum(dim=-1, keepdim=True).clamp_min(1e-9)
            scores = torch.einsum("bi,xij,j->bx", state, self.ops, self.terminal)
            probabilities = scores / scores.sum(dim=-1, keepdim=True)
            logits.append(probabilities.clamp_min(1e-9).log())
            states.append(state)
        return torch.stack(logits, dim=1), torch.stack(states, dim=1)

    def forward(self, tokens: Tensor) -> Tensor:
        return self.forward_with_states(tokens)[0]

    def extract_state(self, tokens: Tensor) -> Tensor:
        return self.forward_with_states(tokens)[1][:, -1]
