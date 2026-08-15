"""Common-random-number loading experiments for Token Radiation."""

from __future__ import annotations

from dataclasses import dataclass
import copy
import hashlib
import math

import numpy as np
import torch
from torch import nn
import torch.nn.functional as F

from .model import BodyConfig, BodyTrace, BytePredictionModel
from .process import Corpus


@dataclass(frozen=True, slots=True)
class PairedConfig:
    radiation_fractions: tuple[float, ...] = (0.0, 0.01, 0.04, 0.16, 0.50)
    seeds: tuple[int, ...] = (20260814, 20260815, 20260816)
    sites: int = 8
    radiation_rank: int = 4
    body_steps: int = 48
    batch_size: int = 14
    learning_rate: float = 0.025
    probe_steps: int = 32
    probe_learning_rate: float = 0.04
    horizons: tuple[int, ...] = (1, 2, 4, 8)
    checkpoint_every: int = 12
    pulse_offsets: tuple[int, ...] = (0, 1, 2, 4, 8)
    equivalence_tolerance: float = 1.0e-11

    def validate(self) -> None:
        if any(not 0.0 <= value <= 1.0 for value in self.radiation_fractions):
            raise ValueError("radiation fractions must lie in [0,1]")
        if not self.seeds:
            raise ValueError("at least one seed is required")
        if self.body_steps < 1 or self.batch_size < 1 or self.probe_steps < 1:
            raise ValueError("training counts must be positive")


@dataclass(slots=True)
class PairedModels:
    passive: BytePredictionModel
    nonloading: BytePredictionModel


def encode_corpus(corpus: Corpus, *, validation: bool = False) -> torch.Tensor:
    examples = corpus.validation if validation else corpus.train
    return torch.tensor(
        [corpus.encode(example.surface) for example in examples], dtype=torch.long
    )


def targets(byte_ids: torch.Tensor, alphabet_size: int) -> torch.Tensor:
    eos = torch.full(
        (byte_ids.shape[0], 1),
        alphabet_size,
        dtype=torch.long,
        device=byte_ids.device,
    )
    return torch.cat((byte_ids, eos), dim=1)


def parameter_digest(model: nn.Module) -> str:
    digest = hashlib.sha256()
    for name, tensor in sorted(model.state_dict().items()):
        digest.update(name.encode())
        digest.update(np.ascontiguousarray(tensor.detach().cpu().numpy()).tobytes())
    return digest.hexdigest()


def evaluate_model(
    model: BytePredictionModel, byte_ids: torch.Tensor, *, loading: bool
) -> tuple[dict[str, float], BodyTrace, torch.Tensor]:
    model.eval()
    with torch.no_grad():
        logits, trace = model(byte_ids, loading=loading)
        expected = targets(byte_ids, model.body.config.alphabet_size)
        token_nll = F.cross_entropy(
            logits.reshape(-1, logits.shape[-1]),
            expected.reshape(-1),
            reduction="none",
        ).reshape_as(expected)
        bpb = float(token_nll.sum() / (byte_ids.numel() * math.log(2.0)))
    return (
        {
            "bits_per_byte": bpb,
            "mean_booked_radiation_per_byte": float(trace.radiation[:, 1:].mean()),
            "mean_projected_intensity_per_byte": float(
                trace.projected_intensity[:, 1:].mean()
            ),
            "maximum_closure_residual": float(trace.closure_residual.abs().max()),
            "mean_final_state_norm": float(trace.states[:, -1].norm(dim=-1).mean()),
        },
        trace,
        token_nll,
    )


def _train_step(
    model: BytePredictionModel,
    optimizer: torch.optim.Optimizer,
    batch: torch.Tensor,
    *,
    loading: bool,
) -> float:
    model.train()
    optimizer.zero_grad(set_to_none=True)
    logits, trace = model(batch, loading=loading)
    expected = targets(batch, model.body.config.alphabet_size)
    loss = F.cross_entropy(logits.reshape(-1, logits.shape[-1]), expected.reshape(-1))
    loss = loss + 1e3 * trace.closure_residual.square().mean()
    loss.backward()
    torch.nn.utils.clip_grad_norm_(model.parameters(), 5.0)
    optimizer.step()
    return float(loss.detach())


def _fit_horizon_probe(
    model: BytePredictionModel,
    train_ids: torch.Tensor,
    validation_ids: torch.Tensor,
    *,
    loading: bool,
    horizon: int,
    seed: int,
    steps: int,
    learning_rate: float,
) -> float:
    model.eval()
    with torch.no_grad():
        train_states = model.body(train_ids, loading=loading).states
        validation_states = model.body(validation_ids, loading=loading).states
    train_count = train_ids.shape[1] - horizon + 1
    validation_count = validation_ids.shape[1] - horizon + 1
    train_features = train_states[:, :train_count].reshape(-1, train_states.shape[-1])
    train_targets = train_ids[:, horizon - 1 :].reshape(-1)
    validation_features = validation_states[:, :validation_count].reshape(
        -1, validation_states.shape[-1]
    )
    validation_targets = validation_ids[:, horizon - 1 :].reshape(-1)
    torch.manual_seed(seed)
    probe = nn.Linear(
        train_features.shape[-1], model.body.config.alphabet_size, dtype=torch.float64
    )
    optimizer = torch.optim.Adam(probe.parameters(), lr=learning_rate)
    for _ in range(steps):
        optimizer.zero_grad(set_to_none=True)
        loss = F.cross_entropy(probe(train_features), train_targets)
        loss.backward()
        optimizer.step()
    with torch.no_grad():
        nll = F.cross_entropy(
            probe(validation_features), validation_targets, reduction="sum"
        )
    return float(nll / (validation_targets.numel() * math.log(2.0)))


def _mean_js(left_logits: torch.Tensor, right_logits: torch.Tensor) -> float:
    left_log = F.log_softmax(left_logits, dim=-1)
    right_log = F.log_softmax(right_logits, dim=-1)
    left = left_log.exp()
    right = right_log.exp()
    mixture = 0.5 * (left + right)
    log_mix = mixture.clamp_min(torch.finfo(mixture.dtype).tiny).log()
    value = 0.5 * (
        (left * (left_log - log_mix)).sum(dim=-1)
        + (right * (right_log - log_mix)).sum(dim=-1)
    )
    return float(value.mean())


def _single_pulse(
    model: BytePredictionModel,
    validation_ids: torch.Tensor,
    *,
    offsets: tuple[int, ...],
    tolerance: float,
) -> dict[str, object]:
    sequence_bytes = validation_ids.shape[1]
    pulse_positions = tuple(
        sorted({sequence_bytes // 4, sequence_bytes // 2, 3 * sequence_bytes // 4})
    )
    event_count = sequence_bytes + 1
    model.eval()
    with torch.no_grad():
        baseline = model.body.forward_with_loading_mask(
            validation_ids,
            loading_mask=tuple(False for _ in range(event_count)),
        )
        baseline_logits = model.logits_from_coordinates(baseline.coordinates)
        expected = targets(validation_ids, model.body.config.alphabet_size)
        baseline_nll = F.cross_entropy(
            baseline_logits.reshape(-1, baseline_logits.shape[-1]),
            expected.reshape(-1),
            reduction="none",
        ).reshape_as(expected)
        metric = model.body.constrained()["metric"]
        pulse_rows: list[dict[str, object]] = []
        for pulse in pulse_positions:
            trace = model.body.forward_with_loading_mask(
                validation_ids,
                loading_mask=tuple(index == pulse for index in range(event_count)),
            )
            logits = model.logits_from_coordinates(trace.coordinates)
            nll = F.cross_entropy(
                logits.reshape(-1, logits.shape[-1]),
                expected.reshape(-1),
                reduction="none",
            ).reshape_as(expected)
            distance = (
                (trace.waves - baseline.waves).square() * metric
            ).sum(dim=-1).sqrt()
            rows: dict[str, object] = {}
            base_distance = float(distance[:, pulse].mean())
            for offset in offsets:
                position = pulse + offset
                if position >= event_count:
                    continue
                value = float(distance[:, position].mean())
                if abs(value) <= tolerance:
                    value = 0.0
                rows[str(offset)] = {
                    "metric_wave_distance": value,
                    "retention_ratio": (
                        value / base_distance if base_distance > tolerance else 0.0
                    ),
                    "excess_nll_nats": float(
                        (nll[:, position] - baseline_nll[:, position]).mean()
                    ),
                    "prediction_js_nats": _mean_js(
                        logits[:, position], baseline_logits[:, position]
                    ),
                }
            pulse_rows.append(
                {
                    "position": pulse,
                    "booked_radiation": float(trace.radiation[:, pulse].mean()),
                    "maximum_closure_residual": float(
                        trace.closure_residual.abs().max()
                    ),
                    "offsets": rows,
                }
            )
    aggregate: dict[str, object] = {}
    for offset in offsets:
        available = [
            row["offsets"][str(offset)]
            for row in pulse_rows
            if str(offset) in row["offsets"]
        ]
        if available:
            aggregate[str(offset)] = {
                key: float(np.mean([float(value[key]) for value in available]))
                for key in (
                    "metric_wave_distance",
                    "retention_ratio",
                    "excess_nll_nats",
                    "prediction_js_nats",
                )
            }
    return {
        "pulse_positions": list(pulse_positions),
        "mean_booked_radiation": float(
            np.mean([float(row["booked_radiation"]) for row in pulse_rows])
        ),
        "maximum_closure_residual": max(
            float(row["maximum_closure_residual"]) for row in pulse_rows
        ),
        "offsets": aggregate,
        "per_pulse": pulse_rows,
    }


def train_paired_condition(
    corpus: Corpus, config: PairedConfig, *, fraction: float, seed: int
) -> tuple[dict[str, object], PairedModels]:
    config.validate()
    train_ids = encode_corpus(corpus)
    validation_ids = encode_corpus(corpus, validation=True)
    torch.manual_seed(seed)
    body_config = BodyConfig(
        alphabet_size=len(corpus.alphabet),
        sites=config.sites,
        radiation_rank=config.radiation_rank,
        radiation_fraction=fraction,
    )
    base = BytePredictionModel(body_config)
    passive = copy.deepcopy(base)
    nonloading = copy.deepcopy(base)
    initial_digest = parameter_digest(base)
    passive_optimizer = torch.optim.Adam(passive.parameters(), lr=config.learning_rate)
    nonloading_optimizer = torch.optim.Adam(
        nonloading.parameters(), lr=config.learning_rate
    )
    generator = torch.Generator().manual_seed(seed + 1)
    schedule = torch.randint(
        0,
        train_ids.shape[0],
        (config.body_steps, config.batch_size),
        generator=generator,
    )
    schedule_digest = hashlib.sha256(
        np.ascontiguousarray(schedule.numpy()).tobytes()
    ).hexdigest()
    checkpoints: list[dict[str, object]] = []
    for step, indices in enumerate(schedule, start=1):
        batch = train_ids[indices]
        passive_loss = _train_step(passive, passive_optimizer, batch, loading=True)
        nonloading_loss = _train_step(
            nonloading, nonloading_optimizer, batch, loading=False
        )
        if step % config.checkpoint_every == 0 or step == config.body_steps:
            passive_metrics, _, _ = evaluate_model(
                passive, validation_ids, loading=True
            )
            nonloading_metrics, _, _ = evaluate_model(
                nonloading, validation_ids, loading=False
            )
            checkpoints.append(
                {
                    "step": step,
                    "passive_train_loss": passive_loss,
                    "nonloading_train_loss": nonloading_loss,
                    "passive_bits_per_byte": passive_metrics["bits_per_byte"],
                    "nonloading_bits_per_byte": nonloading_metrics["bits_per_byte"],
                    "paired_delta_bits_per_byte": passive_metrics["bits_per_byte"]
                    - nonloading_metrics["bits_per_byte"],
                }
            )

    passive_metrics, _, _ = evaluate_model(passive, validation_ids, loading=True)
    nonloading_metrics, _, _ = evaluate_model(
        nonloading, validation_ids, loading=False
    )
    passive_no_load, _, _ = evaluate_model(passive, validation_ids, loading=False)
    nonloading_loaded, _, _ = evaluate_model(
        nonloading, validation_ids, loading=True
    )
    horizon_rows: dict[str, object] = {}
    for horizon in config.horizons:
        probe_seed = seed + 10_000 + horizon
        passive_bpb = _fit_horizon_probe(
            passive,
            train_ids,
            validation_ids,
            loading=True,
            horizon=horizon,
            seed=probe_seed,
            steps=config.probe_steps,
            learning_rate=config.probe_learning_rate,
        )
        nonloading_bpb = _fit_horizon_probe(
            nonloading,
            train_ids,
            validation_ids,
            loading=False,
            horizon=horizon,
            seed=probe_seed,
            steps=config.probe_steps,
            learning_rate=config.probe_learning_rate,
        )
        horizon_rows[str(horizon)] = {
            "passive_bits_per_byte": passive_bpb,
            "nonloading_bits_per_byte": nonloading_bpb,
            "paired_delta_bits_per_byte": passive_bpb - nonloading_bpb,
        }

    passive_digest = parameter_digest(passive)
    nonloading_digest = parameter_digest(nonloading)
    paired_delta = passive_metrics["bits_per_byte"] - nonloading_metrics["bits_per_byte"]
    mean_radiation = passive_metrics["mean_booked_radiation_per_byte"]
    benefit_per_radiation = (
        -paired_delta * math.log(2.0) / mean_radiation
        if mean_radiation > 0
        else 0.0
    )
    row: dict[str, object] = {
        "fraction": fraction,
        "seed": seed,
        "initial_parameter_sha256": initial_digest,
        "batch_schedule_sha256": schedule_digest,
        "final_passive_parameter_sha256": passive_digest,
        "final_nonloading_parameter_sha256": nonloading_digest,
        "zero_fraction_exact_identity": (
            fraction == 0.0 and passive_digest == nonloading_digest
        ),
        "passive": passive_metrics,
        "nonloading": nonloading_metrics,
        "paired_delta_bits_per_byte": paired_delta,
        "acute_all_loading_cost_on_nonloading_checkpoint_bits_per_byte": (
            nonloading_loaded["bits_per_byte"]
            - nonloading_metrics["bits_per_byte"]
        ),
        "adapted_loading_effect_on_passive_checkpoint_bits_per_byte": (
            passive_metrics["bits_per_byte"] - passive_no_load["bits_per_byte"]
        ),
        "predictive_benefit_nats_per_booked_radiation": benefit_per_radiation,
        "horizon_probes": horizon_rows,
        "single_pulse_on_nonloading_checkpoint": _single_pulse(
            nonloading,
            validation_ids,
            offsets=config.pulse_offsets,
            tolerance=config.equivalence_tolerance,
        ),
        "learning_curve": checkpoints,
    }
    if fraction == 0.0:
        if passive_digest != nonloading_digest or paired_delta != 0.0:
            raise AssertionError("rho=0 paired arms must remain exactly identical")
        if any(
            float(value["paired_delta_bits_per_byte"]) != 0.0
            for value in horizon_rows.values()
        ):
            raise AssertionError("rho=0 paired horizon probes must remain identical")
    return row, PairedModels(passive=passive, nonloading=nonloading)


def aggregate_paired_results(
    rows: list[dict[str, object]],
) -> list[dict[str, object]]:
    result: list[dict[str, object]] = []
    for fraction in sorted({float(row["fraction"]) for row in rows}):
        matching = [row for row in rows if float(row["fraction"]) == fraction]
        deltas = np.asarray(
            [float(row["paired_delta_bits_per_byte"]) for row in matching]
        )
        acute = np.asarray(
            [
                float(
                    row[
                        "acute_all_loading_cost_on_nonloading_checkpoint_bits_per_byte"
                    ]
                )
                for row in matching
            ]
        )
        adapted = np.asarray(
            [
                float(
                    row[
                        "adapted_loading_effect_on_passive_checkpoint_bits_per_byte"
                    ]
                )
                for row in matching
            ]
        )
        result.append(
            {
                "fraction": fraction,
                "replicates": len(matching),
                "mean_paired_delta_bits_per_byte": float(deltas.mean()),
                "std_paired_delta_bits_per_byte": float(
                    deltas.std(ddof=1) if len(deltas) > 1 else 0.0
                ),
                "passive_wins": int(np.sum(deltas < 0.0)),
                "mean_acute_all_loading_cost_bits_per_byte": float(acute.mean()),
                "mean_adapted_loading_effect_bits_per_byte": float(adapted.mean()),
                "mean_predictive_benefit_nats_per_booked_radiation": float(
                    np.mean(
                        [
                            float(
                                row[
                                    "predictive_benefit_nats_per_booked_radiation"
                                ]
                            )
                            for row in matching
                        ]
                    )
                ),
                "mean_horizon_paired_deltas_bits_per_byte": {
                    str(horizon): float(
                        np.mean(
                            [
                                float(
                                    row["horizon_probes"][str(horizon)][
                                        "paired_delta_bits_per_byte"
                                    ]
                                )
                                for row in matching
                            ]
                        )
                    )
                    for horizon in (1, 2, 4, 8)
                },
                "mean_single_pulse_excess_nll_nats": {
                    str(offset): float(
                        np.mean(
                            [
                                float(
                                    row[
                                        "single_pulse_on_nonloading_checkpoint"
                                    ]["offsets"][str(offset)]["excess_nll_nats"]
                                )
                                for row in matching
                            ]
                        )
                    )
                    for offset in (0, 1, 2, 4, 8)
                },
            }
        )
    return result
