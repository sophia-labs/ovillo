from __future__ import annotations

import json
import math
from collections.abc import Iterator
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "schemas" / "ariadne-jazzmaster-profile.schema.json"
PROFILE_PATH = ROOT / "profiles" / "ariadne-jazzmaster-v0.json"

EVIDENCE_CLASSES = {
    "manufacturer",
    "patent",
    "converted",
    "physics-derived",
    "fit",
    "design-hypothesis",
}


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def estimates(value: Any, path: str = "$.") -> Iterator[tuple[str, dict[str, Any]]]:
    """Yield every provenance-bearing numeric estimate in the profile."""
    if isinstance(value, dict):
        required = {"initial", "minimum", "maximum", "unit", "evidence"}
        if required.issubset(value):
            yield path, value
        for key, child in value.items():
            yield from estimates(child, f"{path}{key}.")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from estimates(child, f"{path}[{index}].")


def test_profile_validates_against_draft_2020_12_schema() -> None:
    schema = load_json(SCHEMA_PATH)
    profile = load_json(PROFILE_PATH)
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(profile)


def test_every_estimate_is_finite_ordered_and_provenanced() -> None:
    profile = load_json(PROFILE_PATH)
    found = list(estimates(profile))
    assert found, "the design seed must contain provenance-bearing estimates"
    for path, estimate in found:
        initial = estimate["initial"]
        minimum = estimate["minimum"]
        maximum = estimate["maximum"]
        assert all(
            isinstance(value, (int, float)) and math.isfinite(float(value))
            for value in (initial, minimum, maximum)
        ), f"{path}: estimate bounds must be finite numbers"
        assert minimum <= initial <= maximum, (
            f"{path}: expected minimum <= initial <= maximum, got "
            f"{minimum} <= {initial} <= {maximum}"
        )
        assert isinstance(estimate["unit"], str) and estimate["unit"].strip(), (
            f"{path}: estimate unit must be explicit"
        )
        assert estimate["evidence"] in EVIDENCE_CLASSES, (
            f"{path}: unknown evidence class {estimate['evidence']!r}"
        )


def test_seed_keeps_the_declared_six_course_offset_constitution() -> None:
    profile = load_json(PROFILE_PATH)
    topology = profile["courseTopology"]
    assert topology["polarizations"] == 2
    courses = topology["courses"]
    assert len(courses) == 6
    assert [course["openMidi"] for course in courses] == [40, 45, 50, 55, 59, 64]
    assert all(course["afterlengthRatio"]["minimum"] > 0 for course in courses)


def test_powered_body_and_spatial_radiator_have_bounded_seed_structure() -> None:
    profile = load_json(PROFILE_PATH)
    modes = profile["poweredBody"]["modes"]
    assert len(modes) >= 6
    assert all(mode["quartic"]["minimum"] >= 0 for mode in modes)
    assert all(mode["quartic"]["initial"] >= 0 for mode in modes)

    radiator = profile["radiator"]
    assert radiator["maximumHoaOrder"] >= 1
    assert len(radiator["modes"]) >= 4
    source_ids = {mode["id"] for mode in modes}
    assert all(
        set(radiator_mode["sourceModeIds"]).issubset(source_ids)
        for radiator_mode in radiator["modes"]
    )


def test_seed_is_explicitly_uncalibrated_and_delivery_bound() -> None:
    profile = load_json(PROFILE_PATH)
    assert profile["status"] == "design-seed"
    assert "Uncalibrated" in profile["claim"]
    assert profile["metadata"]["calibrationRequired"] is True
    assert profile["metadata"]["forPlaybackOnlyWithDeliveryReceipt"] is True
    assert profile["deliveryProfileId"]
