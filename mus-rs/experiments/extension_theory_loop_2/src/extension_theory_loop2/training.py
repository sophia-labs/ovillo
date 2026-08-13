from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Any

import torch
from torch import nn
import torch.nn.functional as F

from .models_clifford import StableCliffordBank
from .models_state import ExactPositiveRealization, LinearStateModel, PositiveProjectiveModel, SmallGRU
from .process import ObservableHMM

@dataclass
class TrainConfig:
    steps: int = 240
    batch: int = 128
    length: int = 24
    learning_rate: float = 0.006
    seed: int = 7
    horizon_aux: int = 0
    horizon_weight: float = 0.0


def train_predictive_model(
    model: nn.Module,
    process: ObservableHMM,
    config: TrainConfig,
) -> dict[str, Any]:
    torch.manual_seed(config.seed)
    parameters = list(model.parameters())
    auxiliary_heads = nn.ModuleList()
    if config.horizon_aux > 0:
        auxiliary_heads = nn.ModuleList(
            [nn.Linear(model.state_size, process.symbols) for _ in range(config.horizon_aux)]
        )
        parameters += list(auxiliary_heads.parameters())
    optimizer = torch.optim.AdamW(parameters, lr=config.learning_rate, weight_decay=1e-5)
    scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=config.steps)
    generator = torch.Generator().manual_seed(config.seed + 1009)
    history: list[dict[str, Any]] = []

    for step in range(1, config.steps + 1):
        model.train()
        tokens = process.sample(
            config.batch, config.length + max(1, config.horizon_aux) + 1, generator
        )
        inputs = tokens[:, : config.length]
        targets = tokens[:, 1 : config.length + 1]
        if hasattr(model, "forward_with_states"):
            logits, states = model.forward_with_states(inputs)
        else:
            logits = model(inputs)
            states = None
        next_loss = F.cross_entropy(logits.reshape(-1, process.symbols), targets.reshape(-1))
        auxiliary = torch.zeros((), device=next_loss.device)
        if config.horizon_aux > 0 and states is not None:
            for horizon, head in enumerate(auxiliary_heads, start=1):
                future = tokens[:, horizon : horizon + config.length]
                auxiliary = auxiliary + F.cross_entropy(
                    head(states).reshape(-1, process.symbols), future.reshape(-1)
                ) / config.horizon_aux
        regularization = (
            model.regularization() if hasattr(model, "regularization") else torch.zeros_like(next_loss)
        )
        loss = next_loss + config.horizon_weight * auxiliary + regularization
        optimizer.zero_grad(set_to_none=True)
        loss.backward()
        gradient = float(nn.utils.clip_grad_norm_(parameters, 5.0))
        optimizer.step()
        scheduler.step()

        if step in {1, 40, 80, 160, config.steps}:
            history.append(
                {
                    "step": step,
                    "loss": float(loss.detach()),
                    "next_loss": float(next_loss.detach()),
                    "auxiliary": float(auxiliary.detach()),
                    "gradient_norm": gradient,
                }
            )

    return {
        "history": history,
        "auxiliary_heads": auxiliary_heads.state_dict() if auxiliary_heads else None,
    }


@torch.no_grad()
def evaluate_model(
    model: nn.Module,
    process: ObservableHMM,
    *,
    seed: int,
    batches: int = 16,
    batch: int = 256,
    length: int = 48,
) -> dict[str, Any]:
    model.eval()
    generator = torch.Generator().manual_seed(seed)
    loss = 0.0
    oracle_loss = 0.0
    correct = 0
    total = 0
    for _ in range(batches):
        tokens = process.sample(batch, length + 1, generator)
        inputs, targets = tokens[:, :-1], tokens[:, 1:]
        logits = model(inputs)
        loss += float(
            F.cross_entropy(
                logits.reshape(-1, process.symbols), targets.reshape(-1), reduction="sum"
            )
        )
        beliefs = process.beliefs(inputs)
        oracle = process.oracle_logits(beliefs)
        oracle_loss += float(
            F.cross_entropy(
                oracle.reshape(-1, process.symbols), targets.reshape(-1), reduction="sum"
            )
        )
        correct += int((logits.argmax(dim=-1) == targets).sum())
        total += targets.numel()
    return {
        "bps": loss / total / math.log(2),
        "oracle_bps": oracle_loss / total / math.log(2),
        "excess_bps": (loss - oracle_loss) / total / math.log(2),
        "accuracy": correct / total,
        "tokens": total,
    }


def build_model(kind: str, process: ObservableHMM, state_size: int = 8) -> nn.Module:
    if kind == "clifford":
        if state_size <= 4:
            return StableCliffordBank(process.symbols, latent_dim=2, fibers=1)
        if state_size <= 8:
            return StableCliffordBank(process.symbols, latent_dim=3, fibers=1)
        return StableCliffordBank(process.symbols, latent_dim=3, fibers=2)
    if kind == "projective":
        return PositiveProjectiveModel(process.symbols, rank=max(2, state_size))
    if kind == "linear":
        return LinearStateModel(process.symbols, hidden=max(2, state_size))
    if kind == "gru":
        return SmallGRU(process.symbols, hidden=max(2, state_size))
    raise ValueError(f"unknown model kind: {kind}")
