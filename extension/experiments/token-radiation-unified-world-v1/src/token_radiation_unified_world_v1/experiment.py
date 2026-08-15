"""CPU construction campaign for the unified shared-world lexical interface."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import hashlib
import json
from pathlib import Path
import random
import time

import numpy as np
import torch

from .data import Batch, generate_batch, homograph_mask
from .world import UnifiedLexicalWorld, WorldMode, training_loss


@dataclass(frozen=True, slots=True)
class ExperimentConfig:
    seed: int = 20260815
    train_steps: int = 300
    batch_size: int = 48
    train_events: int = 6
    test_events: int = 8
    learning_rate: float = 0.01
    sites: int = 8
    courses: int = 2
    settle_steps: int = 1
    replicates: int = 3

    def validate(self) -> None:
        if self.train_steps < 1 or self.batch_size < 1 or self.replicates < 1:
            raise ValueError("training steps, batch size, and replicates must be positive")
        if self.train_events < 1 or self.test_events < 1:
            raise ValueError("event counts must be positive")
        if self.learning_rate <= 0.0 or not np.isfinite(self.learning_rate):
            raise ValueError("learning_rate must be finite and positive")
        if self.sites < 2 or self.sites % 2:
            raise ValueError("sites must be an even integer >= 2")
        if not 1 <= self.courses <= 4:
            raise ValueError("courses must be in [1,4]")
        if self.settle_steps < 0:
            raise ValueError("settle_steps must be non-negative")


@dataclass(slots=True)
class Metrics:
    current_accuracy: float
    next_accuracy: float
    mode_accuracy: float
    homograph_accuracy: float
    exact_sequence_rate: float
    max_closure_residual: float
    mean_final_energy: float
    mean_source_work: float
    mean_internal_loss: float


def set_seed(seed: int) -> None:
    random.seed(seed)
    np.random.seed(seed)
    torch.manual_seed(seed)


def evaluate(
    model: UnifiedLexicalWorld,
    batch: Batch,
    *,
    shuffle_field: bool = False,
    lexical_port_scale: float = 1.0,
    predictive_port_scale: float = 1.0,
) -> Metrics:
    model.eval()
    with torch.no_grad():
        trace = model(
            batch,
            shuffle_field=shuffle_field,
            lexical_port_scale=lexical_port_scale,
            predictive_port_scale=predictive_port_scale,
        )
        current = trace.current_logits.argmax(dim=-1)
        next_op = trace.next_logits.argmax(dim=-1)
        mode = trace.mode_logits.argmax(dim=-1)
        current_correct = current.eq(batch.operators)
        mask = homograph_mask(batch)
        homograph = current_correct[mask].float().mean().item() if bool(mask.any()) else float("nan")
        return Metrics(
            current_accuracy=float(current_correct.float().mean().item()),
            next_accuracy=float(next_op.eq(batch.next_operators).float().mean().item()),
            mode_accuracy=float(mode.eq(batch.modes).float().mean().item()),
            homograph_accuracy=float(homograph),
            exact_sequence_rate=float(current_correct.all(dim=1).float().mean().item()),
            max_closure_residual=float(trace.max_closure_residual.max().item()),
            mean_final_energy=float(trace.final_energy.mean().item()),
            mean_source_work=float(trace.source_work.mean().item()),
            mean_internal_loss=float(trace.internal_loss.mean().item()),
        )


def train_one(config: ExperimentConfig, *, mode: WorldMode, seed: int) -> tuple[UnifiedLexicalWorld, dict[str, object]]:
    set_seed(seed)
    model = UnifiedLexicalWorld(
        mode=mode,
        sites=config.sites,
        courses=config.courses,
        settle_steps=config.settle_steps,
        seed=seed,
    )
    optimizer = torch.optim.Adam(model.parameters(), lr=config.learning_rate)
    started = time.perf_counter()
    losses: list[float] = []
    for step in range(config.train_steps):
        batch = generate_batch(
            seed=seed * 1_000_003 + step,
            batch_size=config.batch_size,
            events=config.train_events,
        )
        model.train()
        optimizer.zero_grad(set_to_none=True)
        trace = model(batch)
        loss = training_loss(trace, batch)
        if not torch.isfinite(loss):
            raise RuntimeError(f"non-finite loss for {mode} at step {step}")
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 5.0)
        optimizer.step()
        losses.append(float(loss.detach()))
    elapsed = time.perf_counter() - started
    test = generate_batch(seed=seed + 50_000_000, batch_size=384, events=config.test_events)
    metrics = evaluate(model, test)
    controls: dict[str, dict[str, object]] = {}
    if mode == "shared":
        # This is a cross-world causal intervention: complete live field heads are
        # reassigned between independently generated sequences. It preserves the
        # batch distribution but is not one lawful per-world trajectory, so no
        # per-world ledger claim is attached to it.
        permuted = asdict(evaluate(model, test, shuffle_field=True))
        controls["cross_world_field_permutation"] = {
            key: value
            for key, value in permuted.items()
            if key
            in {
                "current_accuracy",
                "next_accuracy",
                "mode_accuracy",
                "homograph_accuracy",
                "exact_sequence_rate",
            }
        }
        controls["cross_world_field_permutation"]["physical_ledger_applicable"] = False
        controls["lexical_port_disabled"] = {
            **asdict(evaluate(model, test, lexical_port_scale=0.0)),
            "physical_ledger_applicable": True,
        }
        controls["predictive_port_disabled"] = {
            **asdict(evaluate(model, test, predictive_port_scale=0.0)),
            "physical_ledger_applicable": True,
        }
    return model, {
        "mode": mode,
        "seed": seed,
        "initial_loss": losses[0],
        "final_loss": losses[-1],
        "elapsed_seconds": elapsed,
        "metrics": asdict(metrics),
        "controls": controls,
    }


def aggregate(rows: list[dict[str, object]]) -> dict[str, object]:
    by_mode: dict[str, list[dict[str, object]]] = {}
    for row in rows:
        by_mode.setdefault(str(row["mode"]), []).append(row)
    summary: dict[str, object] = {}
    for mode, group in by_mode.items():
        metrics = [row["metrics"] for row in group]
        assert all(isinstance(item, dict) for item in metrics)
        keys = list(metrics[0])  # type: ignore[index]
        summary[mode] = {
            "replicates": len(group),
            "mean": {
                key: float(np.mean([float(item[key]) for item in metrics]))  # type: ignore[index]
                for key in keys
            },
            "std": {
                key: float(np.std([float(item[key]) for item in metrics]))  # type: ignore[index]
                for key in keys
            },
        }
    return summary


def scientific_digest(payload: dict[str, object]) -> str:
    encoded = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def run_campaign(config: ExperimentConfig, output: Path) -> dict[str, object]:
    config.validate()
    output.mkdir(parents=True, exist_ok=True)
    rows: list[dict[str, object]] = []
    for replicate in range(config.replicates):
        seed = config.seed + 10_007 * replicate
        for mode in ("oracle_handoff", "isolated", "direct", "shared"):
            _, row = train_one(config, mode=mode, seed=seed)
            rows.append(row)
    summary = {
        "format": "sophia.instrument.token-radiation-unified-world-v1/result-v1",
        "config": asdict(config),
        "runs": rows,
        "aggregate": aggregate(rows),
        "decision": {},
    }
    aggregate_rows = summary["aggregate"]
    assert isinstance(aggregate_rows, dict)
    shared = aggregate_rows["shared"]["mean"]  # type: ignore[index]
    isolated = aggregate_rows["isolated"]["mean"]  # type: ignore[index]
    direct = aggregate_rows["direct"]["mean"]  # type: ignore[index]
    shared_controls = [row["controls"] for row in rows if row["mode"] == "shared"]
    shuffled_accuracy = float(np.mean([control["cross_world_field_permutation"]["current_accuracy"] for control in shared_controls]))  # type: ignore[index]
    no_lexical = float(np.mean([control["lexical_port_disabled"]["current_accuracy"] for control in shared_controls]))  # type: ignore[index]
    no_predictive = float(np.mean([control["predictive_port_disabled"]["current_accuracy"] for control in shared_controls]))  # type: ignore[index]
    chance = 0.25
    shared_gain = float(shared["current_accuracy"] - isolated["current_accuracy"])
    strongest_disabled = max(shuffled_accuracy, no_lexical, no_predictive)
    decision = {
        "shared_field_signal_causally_supported": bool(
            shared_gain >= 0.20
            and shared["current_accuracy"] - strongest_disabled >= 0.20
        ),
        "shared_world_performance_gate_passed": bool(
            shared["current_accuracy"] >= 0.90
            and shared["exact_sequence_rate"] >= 0.80
        ),
        "shared_beats_isolated_by": shared_gain,
        "shared_minus_direct": float(shared["current_accuracy"] - direct["current_accuracy"]),
        "cross_world_field_permutation_accuracy": shuffled_accuracy,
        "lexical_port_disabled_accuracy": no_lexical,
        "predictive_port_disabled_accuracy": no_predictive,
        "chance_current_accuracy": chance,
        "observer_not_in_causal_path": True,
        "privileged_operator_handoff_removed": True,
        "canonical_rust_world_execution_confirmed": False,
        "gpu_scale_warranted": False,
    }
    summary["decision"] = decision
    summary["scientific_digest"] = scientific_digest({key: value for key, value in summary.items() if key != "scientific_digest"})
    (output / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    (output / "config.json").write_text(json.dumps(asdict(config), indent=2, sort_keys=True) + "\n")
    manifest = {
        "summary_sha256": hashlib.sha256((output / "summary.json").read_bytes()).hexdigest(),
        "config_sha256": hashlib.sha256((output / "config.json").read_bytes()).hexdigest(),
        "scientific_digest": summary["scientific_digest"],
    }
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    return summary


def verify_result(output: Path) -> None:
    summary = json.loads((output / "summary.json").read_text())
    manifest = json.loads((output / "manifest.json").read_text())
    assert hashlib.sha256((output / "summary.json").read_bytes()).hexdigest() == manifest["summary_sha256"]
    assert hashlib.sha256((output / "config.json").read_bytes()).hexdigest() == manifest["config_sha256"]
    expected = scientific_digest({key: value for key, value in summary.items() if key != "scientific_digest"})
    assert expected == summary["scientific_digest"] == manifest["scientific_digest"]
