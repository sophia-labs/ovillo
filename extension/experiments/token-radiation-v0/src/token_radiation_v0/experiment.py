"""Controlled Token Radiation v0 training and vocabulary-view campaign."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import hashlib
import json
import math
from pathlib import Path
import random
from typing import Any, Iterable

import numpy as np
import torch
from torch import nn
import torch.nn.functional as F

from .body import BodyConfig, BytePredictionModel
from .detectors import DetectorKind, PositiveFrameDetector, make_detector
from .lattice import lattice_statistics_from_logits, sequence_log_likelihood
from .process import MOTIFS, Corpus, ProcessConfig, generate_corpus
from .radiation import RadiationPlan
from .vocabulary import (
    Vocabulary,
    byte_vocabulary,
    oracle_motif_vocabulary,
    train_bpe,
    train_predictive_vocabulary,
    train_unigram,
)


@dataclass(frozen=True, slots=True)
class CampaignConfig:
    seed: int = 20260814
    sequence_bytes: int = 32
    train_sequences: int = 48
    validation_sequences: int = 16
    body_steps: int = 60
    body_batch_size: int = 16
    detector_steps: int = 24
    detector_batch_size: int = 6
    vocabulary_size: int = 18
    detector_kinds: tuple[DetectorKind, ...] = ("linear", "positive_frame")
    radiation_modes: tuple[str, ...] = ("passive", "nonloading")
    sites: int = 8
    radiation_rank: int = 4
    radiation_fraction: float = 0.04
    cpu_threads: int = 1

    def validate(self) -> None:
        if self.body_steps < 1 or self.detector_steps < 1:
            raise ValueError("training steps must be positive")
        if self.body_batch_size < 1 or self.detector_batch_size < 1:
            raise ValueError("batch sizes must be positive")
        if self.vocabulary_size < 8:
            raise ValueError("vocabulary_size is too small for the synthetic alphabet")
        if not self.radiation_modes:
            raise ValueError("at least one radiation mode is required")
        if any(mode not in {"passive", "nonloading"} for mode in self.radiation_modes):
            raise ValueError("radiation_modes contains an unsupported value")
        if any(
            kind not in {"linear", "metric", "positive_frame"}
            for kind in self.detector_kinds
        ):
            raise ValueError("detector_kinds contains an unsupported value")
        if not 1 <= self.radiation_rank <= self.sites:
            raise ValueError("radiation_rank must be in [1, sites]")
        if not 0.0 <= self.radiation_fraction <= 1.0:
            raise ValueError("radiation_fraction must be in [0,1]")
        if self.cpu_threads < 1:
            raise ValueError("cpu_threads must be positive")


@dataclass(slots=True)
class TrainedBody:
    model: BytePredictionModel
    train_emissions: tuple[torch.Tensor, ...]
    validation_emissions: tuple[torch.Tensor, ...]
    train_states: tuple[torch.Tensor, ...]
    validation_states: tuple[torch.Tensor, ...]
    metrics: dict[str, float]
    radiation_plan: dict[str, Any]


def _sha256_json(value: Any) -> str:
    payload = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode()
    return hashlib.sha256(payload).hexdigest()


def _encode_sequences(corpus: Corpus, sequences: Iterable[bytes]) -> torch.Tensor:
    return torch.tensor(
        [corpus.encode(sequence) for sequence in sequences], dtype=torch.long
    )


def _targets(byte_ids: torch.Tensor, eos_id: int) -> torch.Tensor:
    eos = torch.full(
        (byte_ids.shape[0], 1), eos_id, dtype=torch.long, device=byte_ids.device
    )
    return torch.cat((byte_ids, eos), dim=1)


def _evaluate_byte_model(
    model: BytePredictionModel, byte_ids: torch.Tensor
) -> tuple[float, Any]:
    model.eval()
    with torch.no_grad():
        logits, trace = model(byte_ids)
        targets = _targets(byte_ids, model.body.config.alphabet_size)
        nll = F.cross_entropy(
            logits.reshape(-1, logits.shape[-1]),
            targets.reshape(-1),
            reduction="sum",
        )
        bpb = float(nll / (byte_ids.numel() * math.log(2.0)))
    return bpb, trace


def _fit_state_probe(
    train_states: tuple[torch.Tensor, ...],
    validation_states: tuple[torch.Tensor, ...],
    train_ids: torch.Tensor,
    validation_ids: torch.Tensor,
    alphabet_size: int,
    *,
    seed: int,
    steps: int = 80,
) -> float:
    torch.manual_seed(seed)
    features = torch.cat(train_states, dim=0).reshape(
        -1, train_states[0].shape[-1]
    )
    targets = _targets(train_ids, alphabet_size).reshape(-1)
    probe = nn.Linear(features.shape[-1], alphabet_size + 1, dtype=torch.float64)
    optimizer = torch.optim.Adam(probe.parameters(), lr=0.04)
    for _ in range(steps):
        optimizer.zero_grad(set_to_none=True)
        loss = F.cross_entropy(probe(features), targets)
        loss.backward()
        optimizer.step()
    with torch.no_grad():
        validation_features = torch.cat(validation_states, dim=0).reshape(
            -1, validation_states[0].shape[-1]
        )
        validation_targets = _targets(validation_ids, alphabet_size).reshape(-1)
        nll = F.cross_entropy(
            probe(validation_features), validation_targets, reduction="sum"
        )
    return float(nll / (validation_ids.numel() * math.log(2.0)))


def train_body(
    corpus: Corpus,
    campaign: CampaignConfig,
    *,
    radiation_mode: str,
    seed_offset: int,
) -> TrainedBody:
    seed = campaign.seed + seed_offset
    torch.manual_seed(seed)
    np.random.seed(seed % (2**32))
    random.seed(seed)
    body_config = BodyConfig(
        alphabet_size=len(corpus.alphabet),
        sites=campaign.sites,
        radiation_rank=campaign.radiation_rank,
        radiation_fraction=campaign.radiation_fraction,
        radiation_mode=radiation_mode,  # type: ignore[arg-type]
    )
    model = BytePredictionModel(body_config)
    optimizer = torch.optim.Adam(model.parameters(), lr=0.025)
    train_ids = _encode_sequences(corpus, corpus.train)
    validation_ids = _encode_sequences(corpus, corpus.validation)
    generator = torch.Generator().manual_seed(seed + 1)
    initial_bpb, _ = _evaluate_byte_model(model, validation_ids)

    for _ in range(campaign.body_steps):
        indices = torch.randint(
            0,
            train_ids.shape[0],
            (campaign.body_batch_size,),
            generator=generator,
        )
        batch = train_ids[indices]
        optimizer.zero_grad(set_to_none=True)
        logits, trace = model(batch)
        targets = _targets(batch, len(corpus.alphabet))
        loss = F.cross_entropy(
            logits.reshape(-1, logits.shape[-1]), targets.reshape(-1)
        )
        loss = loss + 1e3 * trace.closure_residual.square().mean()
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 5.0)
        optimizer.step()

    final_bpb, validation_trace = _evaluate_byte_model(model, validation_ids)
    model.eval()
    train_emissions: list[torch.Tensor] = []
    validation_emissions: list[torch.Tensor] = []
    train_states: list[torch.Tensor] = []
    validation_states: list[torch.Tensor] = []
    with torch.no_grad():
        for row in train_ids:
            trace = model.body(row[None, :])
            train_emissions.append(trace.emissions[0].detach())
            train_states.append(trace.states[0].detach())
        for row in validation_ids:
            trace = model.body(row[None, :])
            validation_emissions.append(trace.emissions[0].detach())
            validation_states.append(trace.states[0].detach())

    direct_bpb = _fit_state_probe(
        tuple(train_states),
        tuple(validation_states),
        train_ids,
        validation_ids,
        len(corpus.alphabet),
        seed=seed + 17,
    )
    constrained = model.body.constrained()
    rows = constrained["aperture_rows"].detach().cpu().numpy()
    plan = RadiationPlan(
        id=f"predictive-radiation-{radiation_mode}",
        fraction=campaign.radiation_fraction,
        rows=tuple(tuple(float(value) for value in row) for row in rows),
    )
    radiation_plan = plan.to_dict() | {
        "sha256": plan.sha256,
        "metric": [
            float(value) for value in constrained["metric"].detach().cpu()
        ],
        "loading": radiation_mode == "passive",
    }
    metrics = {
        "initial_validation_bits_per_byte": initial_bpb,
        "final_validation_bits_per_byte": final_bpb,
        "direct_state_probe_bits_per_byte": direct_bpb,
        "mean_booked_radiation_per_byte": float(
            validation_trace.radiation[:, 1:].mean()
        ),
        "mean_projected_intensity_per_byte": float(
            validation_trace.projected_intensity[:, 1:].mean()
        ),
        "maximum_radiation_closure_residual": float(
            validation_trace.closure_residual.abs().max()
        ),
        "mean_final_state_norm": float(
            validation_trace.states[:, -1].norm(dim=-1).mean()
        ),
    }
    return TrainedBody(
        model=model,
        train_emissions=tuple(train_emissions),
        validation_emissions=tuple(validation_emissions),
        train_states=tuple(train_states),
        validation_states=tuple(validation_states),
        metrics=metrics,
        radiation_plan=radiation_plan,
    )


def build_vocabularies(corpus: Corpus, target_size: int) -> tuple[Vocabulary, ...]:
    return (
        byte_vocabulary(corpus.alphabet),
        train_bpe(corpus.train, target_size=target_size),
        train_unigram(corpus.train, target_size=target_size),
        train_predictive_vocabulary(corpus.train, target_size=target_size),
        oracle_motif_vocabulary(corpus.alphabet),
    )


def fit_detector(
    vocabulary: Vocabulary,
    detector_kind: DetectorKind,
    train_emissions: tuple[torch.Tensor, ...],
    validation_emissions: tuple[torch.Tensor, ...],
    corpus: Corpus,
    campaign: CampaignConfig,
    *,
    seed_offset: int,
) -> dict[str, Any]:
    seed = campaign.seed + seed_offset
    torch.manual_seed(seed)
    random.seed(seed)
    radiation_rank = train_emissions[0].shape[-1]
    detector = make_detector(detector_kind, radiation_rank, vocabulary.size)
    optimizer = torch.optim.Adam(detector.parameters(), lr=0.035)
    rng = random.Random(seed + 1)

    for _ in range(campaign.detector_steps):
        indices = [
            rng.randrange(len(corpus.train))
            for _ in range(campaign.detector_batch_size)
        ]
        optimizer.zero_grad(set_to_none=True)
        negative_log_likelihood = train_emissions[0].new_zeros(())
        byte_count = 0
        for index in indices:
            negative_log_likelihood = (
                negative_log_likelihood
                - sequence_log_likelihood(
                    train_emissions[index],
                    corpus.train[index],
                    vocabulary,
                    detector,
                )
            )
            byte_count += len(corpus.train[index])
        loss = negative_log_likelihood / max(byte_count, 1)
        loss.backward()
        torch.nn.utils.clip_grad_norm_(detector.parameters(), 5.0)
        optimizer.step()

    detector.eval()
    total_log_likelihood = 0.0
    total_bytes = 0
    total_expected_tokens = 0.0
    expected_counts = np.zeros(vocabulary.size, dtype=np.float64)
    frame_residual = 0.0
    with torch.no_grad():
        for emissions, sequence in zip(
            validation_emissions, corpus.validation, strict=True
        ):
            logits = detector(emissions).detach().cpu().numpy()
            statistics = lattice_statistics_from_logits(
                logits, sequence, vocabulary
            )
            total_log_likelihood += statistics.log_likelihood
            total_bytes += len(sequence)
            total_expected_tokens += statistics.expected_tokens
            expected_counts += np.asarray(statistics.token_expected_counts)
            if isinstance(detector, PositiveFrameDetector):
                frame_residual = max(
                    frame_residual,
                    float(detector.frame_residual(emissions).abs().max()),
                )
    bits_per_byte = -total_log_likelihood / (
        max(total_bytes, 1) * math.log(2.0)
    )
    events_per_byte = total_expected_tokens / max(total_bytes, 1)
    active_without_eos = int(np.sum(expected_counts[:-1] > 1e-3))
    return {
        "vocabulary": vocabulary.name,
        "vocabulary_size": vocabulary.size,
        "detector": detector_kind,
        "bits_per_byte": bits_per_byte,
        "events_per_byte": events_per_byte,
        "bytes_per_event": 1.0 / events_per_byte
        if events_per_byte > 0
        else math.inf,
        "active_vocabulary_without_eos": active_without_eos,
        "lexicon_surface_bytes": sum(
            len(entry.surface or b"") for entry in vocabulary.entries
        ),
        "maximum_positive_frame_residual": frame_residual,
        "token_expected_counts": [float(value) for value in expected_counts],
    }


def _pareto_frontier(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    frontier = []
    for row in rows:
        dominated = any(
            other is not row
            and other["bits_per_byte"] <= row["bits_per_byte"]
            and other["events_per_byte"] <= row["events_per_byte"]
            and (
                other["bits_per_byte"] < row["bits_per_byte"]
                or other["events_per_byte"] < row["events_per_byte"]
            )
            for other in rows
        )
        if not dominated:
            frontier.append(
                {
                    "radiation_mode": row["radiation_mode"],
                    "vocabulary": row["vocabulary"],
                    "detector": row["detector"],
                    "bits_per_byte": row["bits_per_byte"],
                    "events_per_byte": row["events_per_byte"],
                }
            )
    return sorted(
        frontier,
        key=lambda value: (value["events_per_byte"], value["bits_per_byte"]),
    )


def run_campaign(config: CampaignConfig, output_dir: Path) -> dict[str, Any]:
    config.validate()
    torch.set_num_threads(config.cpu_threads)
    output_dir.mkdir(parents=True, exist_ok=True)
    corpus = generate_corpus(
        ProcessConfig(
            sequence_bytes=config.sequence_bytes,
            train_sequences=config.train_sequences,
            validation_sequences=config.validation_sequences,
            seed=config.seed,
        )
    )
    corpus.write(output_dir / "corpus.json")
    vocabularies = build_vocabularies(corpus, config.vocabulary_size)
    (output_dir / "vocabularies.json").write_text(
        json.dumps(
            [vocabulary.to_dict() for vocabulary in vocabularies],
            indent=2,
            sort_keys=True,
        )
        + "\n"
    )

    bodies: dict[str, dict[str, Any]] = {}
    detector_rows: list[dict[str, Any]] = []
    for mode_index, mode in enumerate(config.radiation_modes):
        trained = train_body(
            corpus,
            config,
            radiation_mode=mode,
            seed_offset=10_000 * mode_index,
        )
        bodies[mode] = {
            "metrics": trained.metrics,
            "radiation_plan": trained.radiation_plan,
        }
        for vocabulary_index, vocabulary in enumerate(vocabularies):
            for detector_index, detector_kind in enumerate(config.detector_kinds):
                result = fit_detector(
                    vocabulary,
                    detector_kind,
                    trained.train_emissions,
                    trained.validation_emissions,
                    corpus,
                    config,
                    seed_offset=(
                        100_000 * mode_index
                        + 1_000 * vocabulary_index
                        + 100 * detector_index
                    ),
                )
                result["radiation_mode"] = mode
                detector_rows.append(result)

    payload: dict[str, Any] = {
        "format": "sophia.instrument.extension.token-radiation-v0.result/v1",
        "campaign": asdict(config),
        "corpus": {
            "alphabet": list(corpus.alphabet),
            "motifs": [value.decode("ascii") for value in MOTIFS],
            "train_sequences": len(corpus.train),
            "validation_sequences": len(corpus.validation),
            "sequence_bytes": config.sequence_bytes,
        },
        "bodies": bodies,
        "detector_results": detector_rows,
        "pareto_frontier": _pareto_frontier(detector_rows),
        "claim_boundary": {
            "exact_rank_k_radiation": True,
            "byte_driven_shared_body": True,
            "segmentation_marginalized_detector_views": True,
            "input_macro_tokens": False,
            "adaptive_event_clock": False,
            "natural_language_utility": False,
            "tokenizer_independent_body": False,
        },
    }
    payload["sha256"] = _sha256_json(payload)
    (output_dir / "summary.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n"
    )
    manifest = {}
    for path in sorted(output_dir.iterdir()):
        if path.is_file():
            manifest[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
    (output_dir / "manifest.sha256.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    )
    return payload


def verify_output(output_dir: Path) -> dict[str, Any]:
    summary = json.loads((output_dir / "summary.json").read_text())
    expected_sha = summary.pop("sha256")
    actual_sha = _sha256_json(summary)
    summary["sha256"] = expected_sha
    if actual_sha != expected_sha:
        raise ValueError("summary scientific digest mismatch")
    manifest = json.loads((output_dir / "manifest.sha256.json").read_text())
    errors = []
    for name, expected in manifest.items():
        actual = hashlib.sha256((output_dir / name).read_bytes()).hexdigest()
        if actual != expected:
            errors.append(f"{name}: expected {expected}, found {actual}")
    if errors:
        raise ValueError("artifact digest errors: " + "; ".join(errors))
    return {
        "valid": True,
        "scientific_sha256": expected_sha,
        "files": len(manifest),
        "detector_results": len(summary["detector_results"]),
    }
