"""Dependency-light reference semantics for Ariadne local constitutive media.

This module mirrors ``mus_ariadne::law::medium``. It is intentionally written
with ordinary Python floats so fixtures can be generated and checked without a
machine-learning runtime. Training code lives in :mod:`extension_generation3.model`.
"""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import json
import math
from typing import Any, Iterable

SCHEMA_VERSION = "ariadne-local-medium-v1"


def _finite(value: float) -> float:
    value = float(value)
    return value if math.isfinite(value) else 0.0


def _canonical_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")


def digest_json(value: Any) -> str:
    return hashlib.sha256(_canonical_bytes(value)).hexdigest()


class PlanError(ValueError):
    """Raised when a local-medium construction plan is outside the law domain."""


@dataclass(slots=True)
class StepReceipt:
    energy_before: float
    energy_after: float
    source_work: float
    active_transfer: float
    transport_residual: float
    positive_mass_residual: float
    transport_loss: float
    passive_loss: float
    internal_loss: float
    closure_residual: float
    observation: list[float]

    def to_dict(self) -> dict[str, Any]:
        return {
            "energy_before": self.energy_before,
            "energy_after": self.energy_after,
            "source_work": self.source_work,
            "active_transfer": self.active_transfer,
            "transport_residual": self.transport_residual,
            "positive_mass_residual": self.positive_mass_residual,
            "transport_loss": self.transport_loss,
            "passive_loss": self.passive_loss,
            "internal_loss": self.internal_loss,
            "closure_residual": self.closure_residual,
            "observation": self.observation,
        }


class ReferenceMedium:
    """Exact scalar reference for a validated ``ariadne-local-medium-v1`` plan."""

    def __init__(self, plan: dict[str, Any]):
        self.plan = validate_plan(plan)
        self.dt = 1.0 / float(self.plan["sample_rate_hz"])
        self.wave = [float(site.get("initial_wave", 0.0)) for site in self.plan["sites"]]
        self.evidence = [
            float(site.get("initial_evidence", 0.0)) for site in self.plan["sites"]
        ]
        self.reservoir = [
            min(
                float(site.get("initial_reservoir", 0.0)),
                float(site["reservoir_capacity"]),
            )
            for site in self.plan["sites"]
        ]
        self.edge_enabled = [bool(edge.get("enabled", True)) for edge in self.plan["edges"]]

    @property
    def digest(self) -> str:
        return digest_json(self.plan)

    def state_dict(self) -> dict[str, list[float]]:
        return {
            "wave": list(self.wave),
            "evidence": list(self.evidence),
            "reservoir": list(self.reservoir),
        }

    def stored_energy(self) -> float:
        total = 0.0
        for index, site in enumerate(self.plan["sites"]):
            total += 0.5 * float(site["wave_metric"]) * self.wave[index] ** 2
            total += 0.5 * self.evidence[index] ** 2
            total += self.reservoir[index]
        return total

    def positive_mass(self) -> float:
        return sum(self.evidence)

    def observe(self) -> list[float]:
        result: list[float] = []
        for observer in self.plan.get("observers", []):
            value = float(observer.get("bias", 0.0))
            for index in range(len(self.wave)):
                value += float(observer["wave_weights"][index]) * self.wave[index]
                value += float(observer["evidence_weights"][index]) * self.evidence[index]
                value += float(observer["reservoir_weights"][index]) * self.reservoir[index]
            result.append(value)
        return result

    def set_edge_enabled(self, edge_index: int, enabled: bool) -> dict[str, Any]:
        if edge_index < 0 or edge_index >= len(self.edge_enabled):
            raise IndexError(edge_index)
        energy_before = self.stored_energy()
        previous = self.edge_enabled[edge_index]
        self.edge_enabled[edge_index] = bool(enabled)
        energy_after = self.stored_energy()
        return {
            "edge_index": edge_index,
            "enabled_before": previous,
            "enabled_after": bool(enabled),
            "state_energy_before": energy_before,
            "state_energy_after": energy_after,
            "state_energy_delta": energy_after - energy_before,
        }

    def reconfigure_site(self, site_index: int, replacement: dict[str, Any]) -> dict[str, Any]:
        if site_index < 0 or site_index >= len(self.wave):
            raise IndexError(site_index)
        current = self.plan["sites"][site_index]
        replacement = dict(replacement)
        if replacement.get("id") != current.get("id"):
            raise PlanError("reconfiguration must preserve the site id")
        _validate_site(replacement, site_index)
        if self.reservoir[site_index] > float(replacement["reservoir_capacity"]) + 1e-15:
            raise PlanError("reconfiguration would silently discard reservoir state")
        energy_before = self.stored_energy()
        wave_before = self.wave[site_index]
        current_metric = float(current["wave_metric"])
        replacement_metric = float(replacement["wave_metric"])
        configuration_work = 0.5 * (replacement_metric - current_metric) * wave_before**2
        self.plan["sites"][site_index] = replacement
        energy_after = self.stored_energy()
        return {
            "site_index": site_index,
            "site_id": replacement["id"],
            "configuration_work": configuration_work,
            "energy_before": energy_before,
            "energy_after": energy_after,
            "closure_residual": energy_after - energy_before - configuration_work,
        }

    def step(self, drives: Iterable[dict[str, Any]] = ()) -> StepReceipt:
        energy_before = self.stored_energy()
        source_work = 0.0

        sources = self.plan.get("sources", [])
        for drive in drives:
            source_index = int(drive["source"])
            if source_index < 0 or source_index >= len(sources):
                raise PlanError(f"unknown source index {source_index}")
            aperture = sources[source_index]
            site_index = int(aperture["site"])
            site = self.plan["sites"][site_index]

            delta_wave = float(aperture["wave_gain"]) * _finite(drive.get("wave", 0.0))
            old_wave_energy = 0.5 * float(site["wave_metric"]) * self.wave[site_index] ** 2
            self.wave[site_index] += delta_wave
            new_wave_energy = 0.5 * float(site["wave_metric"]) * self.wave[site_index] ** 2
            source_work += new_wave_energy - old_wave_energy

            delta_evidence = max(
                0.0,
                float(aperture["evidence_gain"]) * _finite(drive.get("evidence", 0.0)),
            )
            old_evidence_energy = 0.5 * self.evidence[site_index] ** 2
            self.evidence[site_index] += delta_evidence
            new_evidence_energy = 0.5 * self.evidence[site_index] ** 2
            source_work += new_evidence_energy - old_evidence_energy

            work_rate = max(
                0.0,
                float(aperture["work_gain"]) * _finite(drive.get("work_rate", 0.0)),
            )
            room = float(site["reservoir_capacity"]) - self.reservoir[site_index]
            reservoir_add = min(max(room, 0.0), work_rate * self.dt)
            self.reservoir[site_index] += reservoir_add
            source_work += reservoir_add

        active_transfer = 0.0
        for index, site in enumerate(self.plan["sites"]):
            rate = float(site["active_rate_per_second"])
            threshold = float(site["active_threshold"])
            target = float(site["active_target_wave_energy"])
            if rate <= 0.0 or self.reservoir[index] <= 0.0:
                continue
            activation = max(self.evidence[index] + abs(self.wave[index]) - threshold, 0.0)
            current_wave_energy = 0.5 * float(site["wave_metric"]) * self.wave[index] ** 2
            gap = max(target - current_wave_energy, 0.0)
            request = rate * self.dt * activation
            transfer = min(self.reservoir[index], gap, request)
            if transfer <= 0.0:
                continue
            sign = -1.0 if self.wave[index] < 0.0 else 1.0
            next_energy = current_wave_energy + transfer
            self.wave[index] = sign * math.sqrt(
                2.0 * next_energy / float(site["wave_metric"])
            )
            self.reservoir[index] -= transfer
            active_transfer += transfer

        transport_residual = 0.0
        positive_mass_residual = 0.0
        transport_loss = 0.0
        for edge_index, edge in enumerate(self.plan["edges"]):
            if not self.edge_enabled[edge_index]:
                continue
            left = int(edge["left"])
            right = int(edge["right"])
            left_metric = float(self.plan["sites"][left]["wave_metric"])
            right_metric = float(self.plan["sites"][right]["wave_metric"])

            before_wave_energy = (
                0.5 * left_metric * self.wave[left] ** 2
                + 0.5 * right_metric * self.wave[right] ** 2
            )
            theta = float(edge["wave_rate_radians_per_second"]) * self.dt
            cosine = math.cos(theta)
            sine = math.sin(theta)
            scaled_left = math.sqrt(left_metric) * self.wave[left]
            scaled_right = math.sqrt(right_metric) * self.wave[right]
            rotated_left = cosine * scaled_left - sine * scaled_right
            rotated_right = sine * scaled_left + cosine * scaled_right
            self.wave[left] = rotated_left / math.sqrt(left_metric)
            self.wave[right] = rotated_right / math.sqrt(right_metric)
            after_wave_energy = (
                0.5 * left_metric * self.wave[left] ** 2
                + 0.5 * right_metric * self.wave[right] ** 2
            )
            transport_residual = max(
                transport_residual, abs(after_wave_energy - before_wave_energy)
            )

            mass_before = self.evidence[left] + self.evidence[right]
            evidence_energy_before = 0.5 * (
                self.evidence[left] ** 2 + self.evidence[right] ** 2
            )
            mix = min(
                0.5,
                max(0.0, float(edge["evidence_rate_per_second"]) * self.dt),
            )
            left_before = self.evidence[left]
            right_before = self.evidence[right]
            self.evidence[left] = (1.0 - mix) * left_before + mix * right_before
            self.evidence[right] = mix * left_before + (1.0 - mix) * right_before
            mass_after = self.evidence[left] + self.evidence[right]
            evidence_energy_after = 0.5 * (
                self.evidence[left] ** 2 + self.evidence[right] ** 2
            )
            positive_mass_residual = max(
                positive_mass_residual, abs(mass_after - mass_before)
            )
            transport_loss += max(evidence_energy_before - evidence_energy_after, 0.0)

        passive_loss = 0.0
        for index, site in enumerate(self.plan["sites"]):
            wave_before = 0.5 * float(site["wave_metric"]) * self.wave[index] ** 2
            evidence_before = 0.5 * self.evidence[index] ** 2
            reservoir_before = self.reservoir[index]

            self.wave[index] *= math.exp(
                -0.5 * float(site["wave_decay_per_second"]) * self.dt
            )
            self.evidence[index] *= math.exp(
                -0.5 * float(site["evidence_decay_per_second"]) * self.dt
            )
            self.reservoir[index] *= math.exp(
                -float(site["reservoir_leak_per_second"]) * self.dt
            )

            wave_after = 0.5 * float(site["wave_metric"]) * self.wave[index] ** 2
            evidence_after = 0.5 * self.evidence[index] ** 2
            reservoir_after = self.reservoir[index]
            passive_loss += max(
                wave_before + evidence_before + reservoir_before
                - wave_after
                - evidence_after
                - reservoir_after,
                0.0,
            )

        internal_loss = transport_loss + passive_loss
        energy_after = self.stored_energy()
        closure_residual = energy_after - energy_before - source_work + internal_loss
        return StepReceipt(
            energy_before=energy_before,
            energy_after=energy_after,
            source_work=source_work,
            active_transfer=active_transfer,
            transport_residual=transport_residual,
            positive_mass_residual=positive_mass_residual,
            transport_loss=transport_loss,
            passive_loss=passive_loss,
            internal_loss=internal_loss,
            closure_residual=closure_residual,
            observation=self.observe(),
        )


def validate_plan(plan: dict[str, Any]) -> dict[str, Any]:
    """Return a normalized deep copy or raise :class:`PlanError`."""

    normalized = json.loads(json.dumps(plan))
    if normalized.get("schema_version") != SCHEMA_VERSION:
        raise PlanError(f"expected schema_version={SCHEMA_VERSION!r}")
    if not str(normalized.get("name", "")).strip():
        raise PlanError("plan name must be nonempty")
    sample_rate = float(normalized.get("sample_rate_hz", 0.0))
    if not math.isfinite(sample_rate) or not 1.0 <= sample_rate <= 1_000_000.0:
        raise PlanError("sample_rate_hz must be finite in [1, 1_000_000]")
    sites = normalized.get("sites")
    if not isinstance(sites, list) or not sites:
        raise PlanError("at least one site is required")
    site_ids: set[str] = set()
    for index, site in enumerate(sites):
        _validate_site(site, index)
        identifier = str(site["id"])
        if identifier in site_ids:
            raise PlanError(f"duplicate site id {identifier!r}")
        site_ids.add(identifier)
    edge_ids: set[str] = set()
    for index, edge in enumerate(normalized.get("edges", [])):
        identifier = str(edge.get("id", ""))
        if not identifier or identifier in edge_ids:
            raise PlanError(f"invalid or duplicate edge id at index {index}")
        edge_ids.add(identifier)
        left = int(edge.get("left", -1))
        right = int(edge.get("right", -1))
        if left == right or left < 0 or right < 0 or left >= len(sites) or right >= len(sites):
            raise PlanError(f"edge {identifier!r} has invalid endpoints")
        for key in ("wave_rate_radians_per_second", "evidence_rate_per_second"):
            value = float(edge.get(key, math.nan))
            if not math.isfinite(value):
                raise PlanError(f"edge {identifier!r} has nonfinite {key}")
        if float(edge["evidence_rate_per_second"]) < 0.0:
            raise PlanError(f"edge {identifier!r} has negative evidence rate")
        edge["enabled"] = bool(edge.get("enabled", True))
    source_ids: set[str] = set()
    for index, source in enumerate(normalized.get("sources", [])):
        identifier = str(source.get("id", ""))
        if not identifier or identifier in source_ids:
            raise PlanError(f"invalid or duplicate source id at index {index}")
        source_ids.add(identifier)
        site = int(source.get("site", -1))
        if site < 0 or site >= len(sites):
            raise PlanError(f"source {identifier!r} references an unknown site")
        for key in ("wave_gain", "evidence_gain", "work_gain"):
            value = float(source.get(key, math.nan))
            if not math.isfinite(value):
                raise PlanError(f"source {identifier!r} has nonfinite {key}")
    observer_ids: set[str] = set()
    for index, observer in enumerate(normalized.get("observers", [])):
        identifier = str(observer.get("id", ""))
        if not identifier or identifier in observer_ids:
            raise PlanError(f"invalid or duplicate observer id at index {index}")
        observer_ids.add(identifier)
        for key in ("wave_weights", "evidence_weights", "reservoir_weights"):
            values = observer.get(key)
            if not isinstance(values, list) or len(values) != len(sites):
                raise PlanError(f"observer {identifier!r} has invalid {key}")
            if any(not math.isfinite(float(value)) for value in values):
                raise PlanError(f"observer {identifier!r} has nonfinite {key}")
        if not math.isfinite(float(observer.get("bias", 0.0))):
            raise PlanError(f"observer {identifier!r} has nonfinite bias")
    return normalized


def _validate_site(site: dict[str, Any], index: int) -> None:
    identifier = str(site.get("id", ""))
    if not identifier:
        raise PlanError(f"site {index} id must be nonempty")
    positive_keys = (
        "wave_metric",
        "reservoir_capacity",
    )
    nonnegative_keys = (
        "wave_decay_per_second",
        "evidence_decay_per_second",
        "reservoir_leak_per_second",
        "active_rate_per_second",
        "active_threshold",
        "active_target_wave_energy",
        "initial_evidence",
        "initial_reservoir",
    )
    for key in positive_keys:
        value = float(site.get(key, math.nan))
        if not math.isfinite(value) or value <= 0.0:
            raise PlanError(f"site {identifier!r} requires positive finite {key}")
    for key in nonnegative_keys:
        value = float(site.get(key, 0.0))
        if not math.isfinite(value) or value < 0.0:
            raise PlanError(f"site {identifier!r} requires nonnegative finite {key}")
    initial_wave = float(site.get("initial_wave", 0.0))
    if not math.isfinite(initial_wave):
        raise PlanError(f"site {identifier!r} has nonfinite initial_wave")
    if float(site.get("initial_reservoir", 0.0)) > float(site["reservoir_capacity"]):
        raise PlanError(f"site {identifier!r} initial reservoir exceeds capacity")
