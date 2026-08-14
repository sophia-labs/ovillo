"""Deterministic construction proof for Extension Generation 3."""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
import random
from typing import Any

import numpy as np
import torch
import torch.nn.functional as F

from .compiler import compile_plan, compiler_receipt
from .model import Generation3Model, LearnedMediumConfig
from .reference import digest_json
from .tasks import generate_joint_batch


@dataclass(slots=True)
class TrainingResult:
    model: Generation3Model
    plan: dict[str, Any]
    receipt: dict[str, Any]


def _accuracy(logits: torch.Tensor, labels: torch.Tensor) -> list[float]:
    predictions = (torch.sigmoid(logits) >= 0.5).to(labels.dtype)
    return [
        float((predictions[:, index] == labels[:, index]).double().mean())
        for index in range(2)
    ]


def train_construction(
    *,
    seed: int = 20260814,
    steps: int = 240,
    batch_size: int = 256,
    learning_rate: float = 0.025,
    device: str | torch.device = "cpu",
) -> TrainingResult:
    random.seed(seed)
    np.random.seed(seed % (2**32 - 1))
    torch.manual_seed(seed)
    device = torch.device(device)
    model = Generation3Model(LearnedMediumConfig()).to(device)
    optimizer = torch.optim.Adam(model.parameters(), lr=learning_rate)
    generator = torch.Generator(device=device).manual_seed(seed + 1)

    initial_batch = generate_joint_batch(2048, generator=generator, device=device)
    with torch.no_grad():
        initial_logits, _ = model(initial_batch.drives)
        initial_loss = float(
            F.binary_cross_entropy_with_logits(initial_logits, initial_batch.labels)
        )
        initial_accuracy = _accuracy(initial_logits, initial_batch.labels)

    losses: list[float] = []
    for step in range(steps):
        batch = generate_joint_batch(batch_size, generator=generator, device=device)
        optimizer.zero_grad(set_to_none=True)
        logits, trajectory = model(batch.drives)
        clean_loss = F.binary_cross_entropy_with_logits(logits, batch.labels)
        damaged_logits, _ = model(
            batch.drives,
            damaged_edge=(step % len(model.medium.config.edges)),
            damage_steps=(1, 2),
        )
        damage_loss = F.binary_cross_entropy_with_logits(damaged_logits, batch.labels)
        task_loss = 0.6 * clean_loss + 0.4 * damage_loss
        # A small constitution prior keeps transport distributed and keeps the
        # body, rather than only the affine observer, involved in the solution.
        params = model.medium.constrained()
        transport_use = params["wave_rate"].square().mean() + 0.05 * params[
            "evidence_rate"
        ].square().mean()
        observer_penalty = 1e-4 * model.observer.weight.square().mean()
        active_penalty = 1e-5 * trajectory.reservoir.square().mean()
        loss = task_loss + observer_penalty + active_penalty - 1e-5 * transport_use
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 10.0)
        optimizer.step()
        losses.append(float(task_loss.detach()))

    evaluation_generator = torch.Generator(device=device).manual_seed(seed + 2)
    evaluation = generate_joint_batch(8192, generator=evaluation_generator, device=device)
    with torch.no_grad():
        logits, _ = model(evaluation.drives)
        final_loss = float(F.binary_cross_entropy_with_logits(logits, evaluation.labels))
        final_accuracy = _accuracy(logits, evaluation.labels)
        damage_accuracy_by_edge: dict[str, list[float]] = {}
        for edge_index in range(len(model.medium.config.edges)):
            damaged_logits, _ = model(
                evaluation.drives,
                damaged_edge=edge_index,
                damage_steps=(1, 2),
            )
            damage_accuracy_by_edge[str(edge_index)] = _accuracy(
                damaged_logits, evaluation.labels
            )
        damaged_accuracy = [
            min(values[task] for values in damage_accuracy_by_edge.values())
            for task in range(2)
        ]

        zero_wave = evaluation.drives.clone()
        zero_wave[..., 0] = 0.0
        zero_wave_logits, _ = model(zero_wave)
        zero_wave_accuracy = _accuracy(zero_wave_logits, evaluation.labels)

        zero_evidence = evaluation.drives.clone()
        zero_evidence[..., 1] = 0.0
        zero_evidence_logits, _ = model(zero_evidence)
        zero_evidence_accuracy = _accuracy(zero_evidence_logits, evaluation.labels)

    training_receipt = {
        "format": "sophia.instrument.extension.generation3-training/v1",
        "seed": seed,
        "steps": steps,
        "batch_size": batch_size,
        "learning_rate": learning_rate,
        "initial_loss": initial_loss,
        "final_loss": final_loss,
        "initial_accuracy": {
            "path_order": initial_accuracy[0],
            "evidence_majority": initial_accuracy[1],
        },
        "final_accuracy": {
            "path_order": final_accuracy[0],
            "evidence_majority": final_accuracy[1],
        },
        "damage_accuracy": {
            "path_order_minimum": damaged_accuracy[0],
            "evidence_majority_minimum": damaged_accuracy[1],
            "by_edge": {
                edge: {"path_order": value[0], "evidence_majority": value[1]}
                for edge, value in damage_accuracy_by_edge.items()
            },
        },
        "absence_controls": {
            "zero_wave_path_accuracy": zero_wave_accuracy[0],
            "zero_evidence_majority_accuracy": zero_evidence_accuracy[1],
        },
        "loss_trace_sha256": digest_json(losses),
    }
    plan = compile_plan(model, name=f"extension-generation-3-seed-{seed}")
    receipt = compiler_receipt(model, plan, training_receipt=training_receipt)
    return TrainingResult(model=model, plan=plan, receipt=receipt)


def write_result(result: TrainingResult, output: Path) -> None:
    output.mkdir(parents=True, exist_ok=True)
    (output / "local-medium.plan.json").write_text(
        json.dumps(result.plan, indent=2, sort_keys=True) + "\n"
    )
    (output / "compiler.receipt.json").write_text(
        json.dumps(result.receipt, indent=2, sort_keys=True) + "\n"
    )
