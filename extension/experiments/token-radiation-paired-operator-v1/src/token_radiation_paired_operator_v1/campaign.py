"""End-to-end paired loading and operator-vocabulary CPU campaign."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import hashlib
import json
from pathlib import Path
from typing import Any

import numpy as np
import torch

from .lattice import extract_boundary_features, fit_detector
from .paired import (
    PairedConfig,
    PairedModels,
    aggregate_paired_results,
    train_paired_condition,
)
from .process import ProcessConfig, ProcessFamily, generate_corpus
from .vocabulary import (
    Vocabulary,
    byte_vocabulary,
    oracle_vocabulary,
    train_bpe,
    train_endpoint_predictive,
    train_frequency_mdl,
    train_operator_cover,
    train_operator_diversity_cover,
    train_random_frequency_matched,
    vocabulary_diagnostics,
)


@dataclass(frozen=True, slots=True)
class CampaignConfig:
    seed: int = 20260814
    sequence_bytes: int = 32
    train_sequences: int = 48
    validation_sequences: int = 16
    radiation_fractions: tuple[float, ...] = (0.0, 0.01, 0.04, 0.16, 0.50)
    paired_seeds: tuple[int, ...] = (20260814, 20260815, 20260816)
    sites: int = 8
    radiation_rank: int = 4
    body_steps: int = 48
    body_batch_size: int = 14
    probe_steps: int = 32
    vocabulary_size: int = 18
    operator_probe_states: int = 18
    operator_fraction: float = 0.16
    operator_families: tuple[ProcessFamily, ...] = ("structured", "iid_motifs")
    detector_steps: int = 40
    detector_batch_size: int = 7
    detector_replicates: int = 1
    cpu_threads: int = 1

    def validate(self) -> None:
        if self.sequence_bytes < 8:
            raise ValueError("sequence_bytes must be at least eight")
        if self.train_sequences < 1 or self.validation_sequences < 1:
            raise ValueError("corpus partitions must be nonempty")
        if self.body_steps < 1 or self.detector_steps < 1:
            raise ValueError("training steps must be positive")
        if self.vocabulary_size < 8:
            raise ValueError("vocabulary_size is too small")
        if self.operator_probe_states < 2:
            raise ValueError("operator_probe_states must be at least two")
        if self.operator_fraction not in self.radiation_fractions:
            raise ValueError("operator_fraction must be present in radiation_fractions")
        if self.detector_replicates < 1 or self.cpu_threads < 1:
            raise ValueError("replicate and CPU counts must be positive")
        if not self.paired_seeds:
            raise ValueError("paired_seeds cannot be empty")


def canonical_sha256(value: Any) -> str:
    encoded = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode()
    return hashlib.sha256(encoded).hexdigest()


def _paired_config(config: CampaignConfig) -> PairedConfig:
    return PairedConfig(
        radiation_fractions=config.radiation_fractions,
        seeds=config.paired_seeds,
        sites=config.sites,
        radiation_rank=config.radiation_rank,
        body_steps=config.body_steps,
        batch_size=config.body_batch_size,
        probe_steps=config.probe_steps,
    )


def _build_vocabularies(
    corpus,
    model,
    *,
    body_mode: str,
    target_size: int,
    seed: int,
    probe_states: int,
) -> tuple[Vocabulary, ...]:
    loading = body_mode == "passive"
    return (
        byte_vocabulary(corpus),
        train_bpe(corpus, target_size=target_size),
        train_frequency_mdl(corpus, target_size=target_size),
        train_endpoint_predictive(corpus, target_size=target_size),
        train_operator_cover(
            corpus,
            model,
            target_size=target_size,
            loading=loading,
            seed=seed + 31,
            probe_states=probe_states,
        ),
        train_operator_diversity_cover(
            corpus,
            model,
            target_size=target_size,
            loading=loading,
            seed=seed + 37,
            probe_states=probe_states,
        ),
        train_random_frequency_matched(
            corpus, target_size=target_size, seed=seed + 47
        ),
        oracle_vocabulary(corpus),
    )


def _aggregate_detector_replicates(
    rows: list[dict[str, object]],
) -> list[dict[str, object]]:
    keys = sorted(
        {
            (
                str(row["process_family"]),
                str(row["body_mode"]),
                str(row["vocabulary"]),
            )
            for row in rows
        }
    )
    result: list[dict[str, object]] = []
    for process_family, body_mode, vocabulary in keys:
        matching = [
            row
            for row in rows
            if row["process_family"] == process_family
            and row["body_mode"] == body_mode
            and row["vocabulary"] == vocabulary
        ]
        bpb = np.asarray([float(row["bits_per_byte"]) for row in matching])
        event_rate = np.asarray(
            [float(row["expected_lexical_events_per_byte"]) for row in matching]
        )
        boundary_f1 = np.asarray(
            [float(row["soft_oracle_boundary_f1"]) for row in matching]
        )
        result.append(
            {
                "process_family": process_family,
                "body_mode": body_mode,
                "vocabulary": vocabulary,
                "replicates": len(matching),
                "mean_bits_per_byte": float(bpb.mean()),
                "std_bits_per_byte": float(
                    bpb.std(ddof=1) if bpb.size > 1 else 0.0
                ),
                "mean_expected_lexical_events_per_byte": float(event_rate.mean()),
                "mean_soft_oracle_boundary_f1": float(boundary_f1.mean()),
            }
        )
    return result


def _pareto_frontier(rows: list[dict[str, object]]) -> list[dict[str, object]]:
    frontier: list[dict[str, object]] = []
    groups = sorted(
        {(str(row["process_family"]), str(row["body_mode"])) for row in rows}
    )
    for process_family, body_mode in groups:
        matching = [
            row
            for row in rows
            if row["process_family"] == process_family
            and row["body_mode"] == body_mode
        ]
        for row in matching:
            dominated = any(
                other is not row
                and float(other["mean_bits_per_byte"])
                <= float(row["mean_bits_per_byte"])
                and float(other["mean_expected_lexical_events_per_byte"])
                <= float(row["mean_expected_lexical_events_per_byte"])
                and (
                    float(other["mean_bits_per_byte"])
                    < float(row["mean_bits_per_byte"])
                    or float(other["mean_expected_lexical_events_per_byte"])
                    < float(row["mean_expected_lexical_events_per_byte"])
                )
                for other in matching
            )
            if not dominated:
                frontier.append(row)
    return sorted(
        frontier,
        key=lambda row: (
            str(row["process_family"]),
            str(row["body_mode"]),
            float(row["mean_expected_lexical_events_per_byte"]),
        ),
    )


def run_campaign(config: CampaignConfig, output_dir: Path) -> dict[str, Any]:
    config.validate()
    torch.set_num_threads(config.cpu_threads)
    output_dir.mkdir(parents=True, exist_ok=True)
    paired_config = _paired_config(config)

    structured_corpus = generate_corpus(
        ProcessConfig(
            sequence_bytes=config.sequence_bytes,
            train_sequences=config.train_sequences,
            validation_sequences=config.validation_sequences,
            seed=config.seed,
            family="structured",
        )
    )
    structured_corpus.write(output_dir / "corpus-structured.json")

    paired_rows: list[dict[str, object]] = []
    retained_models: dict[tuple[float, int], PairedModels] = {}
    for fraction in config.radiation_fractions:
        for seed in config.paired_seeds:
            row, models = train_paired_condition(
                structured_corpus,
                paired_config,
                fraction=fraction,
                seed=seed,
            )
            paired_rows.append(row)
            retained_models[(fraction, seed)] = models

    operator_fraction = config.operator_fraction
    primary_seed = config.paired_seeds[0]
    detector_rows: list[dict[str, object]] = []
    vocabulary_rows: list[dict[str, object]] = []

    for family_index, family in enumerate(config.operator_families):
        corpus = generate_corpus(
            ProcessConfig(
                sequence_bytes=config.sequence_bytes,
                train_sequences=config.train_sequences,
                validation_sequences=config.validation_sequences,
                seed=config.seed + family_index * 100_000,
                family=family,
            )
        )
        corpus.write(output_dir / f"corpus-{family}.json")
        if family == "structured":
            models = retained_models[(operator_fraction, primary_seed)]
        else:
            _, models = train_paired_condition(
                corpus,
                paired_config,
                fraction=operator_fraction,
                seed=primary_seed,
            )
        for mode_index, (body_mode, model, loading) in enumerate(
            (
                ("passive", models.passive, True),
                ("nonloading", models.nonloading, False),
            )
        ):
            train_features, validation_features = extract_boundary_features(
                model, corpus, loading=loading
            )
            vocabularies = _build_vocabularies(
                corpus,
                model,
                body_mode=body_mode,
                target_size=config.vocabulary_size,
                seed=config.seed + family_index * 10_000 + mode_index * 1_000,
                probe_states=config.operator_probe_states,
            )
            for vocabulary in vocabularies:
                diagnostics = vocabulary_diagnostics(vocabulary)
                diagnostics.update(
                    {"process_family": family, "body_mode": body_mode}
                )
                vocabulary_rows.append(diagnostics)
                for replicate in range(config.detector_replicates):
                    result = fit_detector(
                        vocabulary,
                        train_features,
                        validation_features,
                        corpus,
                        seed=(
                            config.seed
                            + family_index * 1_000_000
                            + mode_index * 100_000
                            + replicate * 10_000
                            + sum(vocabulary.name.encode())
                        ),
                        steps=config.detector_steps,
                        batch_size=config.detector_batch_size,
                    )
                    result.update(
                        {
                            "process_family": family,
                            "body_mode": body_mode,
                            "replicate": replicate,
                        }
                    )
                    detector_rows.append(result)

    detector_aggregates = _aggregate_detector_replicates(detector_rows)
    summary: dict[str, Any] = {
        "format": "sophia.instrument.extension.token-radiation-paired-operator-v1.result/v1",
        "campaign": asdict(config),
        "claim_boundary": {
            "gpu_required": False,
            "paired_common_random_numbers": True,
            "same_initial_parameters": True,
            "multi_seed_loading_sweep": True,
            "counterfactual_loading_evaluation": True,
            "operator_vocabulary_over_common_probe_states": True,
            "exact_segmentation_marginalization": True,
            "structured_and_iid_motif_control": True,
            "input_macro_tokens": False,
            "natural_language_utility": False,
        },
        "paired_loading": {
            "rows": paired_rows,
            "aggregates": aggregate_paired_results(paired_rows),
        },
        "operator_vocabulary": {
            "operator_fraction": operator_fraction,
            "primary_seed": primary_seed,
            "vocabularies": vocabulary_rows,
            "detector_rows": detector_rows,
            "aggregates": detector_aggregates,
            "pareto_frontier": _pareto_frontier(detector_aggregates),
        },
    }
    summary["sha256"] = canonical_sha256(summary)
    summary_path = output_dir / "summary.json"
    summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    manifest = {
        path.name: hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(output_dir.glob("*.json"))
        if path.name != "manifest.sha256.json"
    }
    (output_dir / "manifest.sha256.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    )
    return summary


def verify_output(output_dir: Path) -> dict[str, Any]:
    manifest_path = output_dir / "manifest.sha256.json"
    if not manifest_path.exists():
        raise ValueError("result manifest is missing")
    manifest = json.loads(manifest_path.read_text())
    for name, expected in manifest.items():
        path = output_dir / name
        if not path.exists():
            raise ValueError(f"manifested file is missing: {name}")
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != expected:
            raise ValueError(f"digest mismatch for {name}")
    summary = json.loads((output_dir / "summary.json").read_text())
    expected_scientific = summary.pop("sha256")
    actual_scientific = canonical_sha256(summary)
    if actual_scientific != expected_scientific:
        raise ValueError("scientific summary digest mismatch")
    return {
        "valid": True,
        "scientific_sha256": expected_scientific,
        "manifested_files": len(manifest),
        "paired_rows": len(summary["paired_loading"]["rows"]),
        "detector_rows": len(summary["operator_vocabulary"]["detector_rows"]),
    }
