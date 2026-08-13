from __future__ import annotations

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]


def load(name: str) -> dict[str, object]:
    return json.loads((ROOT / "schemas" / name).read_text("utf-8"))


def runtime_receipt() -> dict[str, object]:
    return {
        "adapter_version": "mus-dsp-string-trajectory/0.1.0",
        "trajectory": {
            "schema_version": "trajectory-program-v1",
            "compiler_version": "mus-trajectory/0.1.0",
            "program_digest": "sha256:" + "0" * 64,
            "frame_digest": "sha256:" + "1" * 64,
            "sample_rate_hz": 48_000,
            "duration_samples": 192_000,
            "control_stride_samples": 240,
            "frame_count": 801,
            "tracks": [],
            "claim_level": "score-spanning-work-receipted-rearticulation",
            "evidence_class": (
                "validated-numerics+engine-execution+single-construction+"
                "physical-gain-lowering+exact-attack-work+"
                "fully-unobserved-course-kernel"
            ),
        },
        "native_pitch_transport": "score-gliss-or-static",
        "pitch_start_ratio": 1.0,
        "pitch_end_ratio": 1.0,
        "control_frame_count": 801,
        "unresolved_obligations": [],
        "strict_transport": True,
        "rearticulation": {
            "schema_version": "string-rearticulation-receipt-v1",
            "work_digest": "sha256:" + "2" * 64,
            "schedule": {
                "attack_count": 2,
                "active_courses": 1,
                "attack_span_samples": 437,
                "drive_samples": 192_000,
                "first_attack_sample": 0,
                "last_attack_sample": 48_000,
                "maximum_gain": 0.7079457843841379,
                "overlap_policy": "refuse-v1",
                "excitation_identity": "constructor-content-keyed-template",
                "execution_strategy": (
                    "prevalidated-monotone-attack-cursor+"
                    "fully-unobserved-course-kernel-v3"
                ),
            },
            "work_law": (
                "delta_E = wave_before * excitation + 0.5 * excitation^2; "
                "ordered before drive, breath, bow, and contact"
            ),
            "work_unit": "declared-string-wave-energy",
            "total_signed_work": 0.83,
            "total_positive_work": 0.91,
            "total_extracted_work": 0.08,
            "peak_abs_course_sample_work": 0.12,
            "attacks": [
                {
                    "index": 0,
                    "at_sample": 0,
                    "gain": 0.1,
                    "active_courses": 1,
                    "nonzero_course_samples": 437,
                    "signed_work": 0.31,
                    "positive_work": 0.34,
                    "extracted_work": 0.03,
                    "peak_abs_course_sample_work": 0.08,
                },
                {
                    "index": 1,
                    "at_sample": 48_000,
                    "gain": 0.7079457843841379,
                    "active_courses": 1,
                    "nonzero_course_samples": 437,
                    "signed_work": 0.52,
                    "positive_work": 0.57,
                    "extracted_work": 0.05,
                    "peak_abs_course_sample_work": 0.12,
                },
            ],
        },
    }


def resolved_string_schema() -> dict[str, object]:
    schema = load("string-trajectory-receipt.schema.json")
    properties = schema["properties"]
    assert isinstance(properties, dict)
    properties["trajectory"] = load("trajectory-receipt.schema.json")
    properties["rearticulation"] = load("rearticulation-receipt.schema.json")
    return schema


def test_work_receipted_runtime_receipt_conforms() -> None:
    errors = list(
        Draft202012Validator(resolved_string_schema()).iter_errors(runtime_receipt())
    )
    assert errors == [], "\n".join(error.message for error in errors)


def test_event_local_receipt_may_structurally_omit_rearticulation() -> None:
    receipt = runtime_receipt()
    receipt.pop("rearticulation")
    trajectory = receipt["trajectory"]
    assert isinstance(trajectory, dict)
    trajectory["claim_level"] = "authored-control-path"
    trajectory["evidence_class"] = "validated-numerics"
    errors = list(Draft202012Validator(resolved_string_schema()).iter_errors(receipt))
    assert errors == [], "\n".join(error.message for error in errors)
