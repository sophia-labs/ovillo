"""Compile learned constitutions into the Ariadne local-medium IR."""

from __future__ import annotations

import copy
from dataclasses import asdict
from typing import Any

import torch

from .model import Generation3Model
from .reference import SCHEMA_VERSION, digest_json, validate_plan


def compile_plan(
    model: Generation3Model, *, name: str = "extension-generation-3"
) -> dict[str, Any]:
    medium = model.medium
    config = medium.config
    params = {key: value.detach().cpu() for key, value in medium.constrained().items()}

    sites = []
    for index in range(config.sites):
        sites.append(
            {
                "id": f"site-{index}",
                "wave_metric": float(params["wave_metric"][index]),
                "wave_decay_per_second": float(params["wave_decay"][index]),
                "evidence_decay_per_second": float(params["evidence_decay"][index]),
                "reservoir_capacity": float(params["reservoir_capacity"][index]),
                "reservoir_leak_per_second": float(params["reservoir_leak"][index]),
                "active_rate_per_second": float(params["active_rate"][index]),
                "active_threshold": float(params["active_threshold"][index]),
                "active_target_wave_energy": float(params["active_target"][index]),
                "initial_wave": 0.0,
                "initial_evidence": 0.0,
                "initial_reservoir": 0.0,
            }
        )

    edges = []
    for index, (left, right) in enumerate(config.edges):
        edges.append(
            {
                "id": f"edge-{index}",
                "left": left,
                "right": right,
                "wave_rate_radians_per_second": float(params["wave_rate"][index]),
                "evidence_rate_per_second": float(params["evidence_rate"][index]),
                "enabled": True,
            }
        )

    wave_gains = medium.source_wave_gain.detach().cpu()
    evidence_gains = torch.nn.functional.softplus(medium.source_evidence_gain).detach().cpu()
    work_gains = torch.nn.functional.softplus(medium.source_work_gain).detach().cpu()
    sources = [
        {
            "id": f"source-{index}",
            "site": int(site),
            "wave_gain": float(wave_gains[index]),
            "evidence_gain": float(evidence_gains[index]),
            "work_gain": float(work_gains[index]),
        }
        for index, site in enumerate(config.sources)
    ]

    weights = model.observer.weight.detach().cpu()
    bias = model.observer.bias.detach().cpu()
    observers = []
    for output in range(weights.shape[0]):
        row = weights[output]
        n = config.sites
        observers.append(
            {
                "id": "path-order" if output == 0 else "evidence-majority",
                "wave_weights": [float(value) for value in row[:n]],
                "evidence_weights": [float(value) for value in row[n : 2 * n]],
                "reservoir_weights": [float(value) for value in row[2 * n :]],
                "bias": float(bias[output]),
            }
        )

    plan = {
        "schema_version": SCHEMA_VERSION,
        "name": name,
        "sample_rate_hz": config.sample_rate_hz,
        "sites": sites,
        "edges": edges,
        "sources": sources,
        "observers": observers,
    }
    return validate_plan(plan)


def physical_digest(plan: dict[str, Any]) -> str:
    physical = copy.deepcopy(plan)
    physical["observers"] = []
    return digest_json(physical)


def compiler_receipt(
    model: Generation3Model,
    plan: dict[str, Any],
    *,
    training_receipt: dict[str, Any],
) -> dict[str, Any]:
    return {
        "format": "sophia.instrument.extension.generation3-compiler-receipt/v1",
        "model_config": asdict(model.medium.config),
        "physical_plan_sha256": physical_digest(plan),
        "executable_plan_sha256": digest_json(plan),
        "training": training_receipt,
        "claim_boundary": {
            "learned_local_constitution": True,
            "observer_is_non_loading": True,
            "continuum_limit": False,
            "natural_language_utility": False,
            "superiority_to_sequence_baselines": False,
            "canonical_world_body_integration": False,
        },
    }
