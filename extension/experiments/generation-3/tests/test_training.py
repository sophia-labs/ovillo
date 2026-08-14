from __future__ import annotations

from extension_generation3.reference import ReferenceMedium, digest_json
from extension_generation3.train import train_construction


def test_training_changes_a_lawful_compilable_constitution() -> None:
    result = train_construction(seed=31, steps=55, batch_size=96)
    training = result.receipt["training"]
    assert training["final_loss"] < training["initial_loss"] * 0.55
    assert training["final_accuracy"]["path_order"] >= 0.95
    assert training["final_accuracy"]["evidence_majority"] >= 0.90
    assert training["absence_controls"]["zero_wave_path_accuracy"] <= 0.60
    assert training["absence_controls"]["zero_evidence_majority_accuracy"] <= 0.60
    assert result.receipt["executable_plan_sha256"] == digest_json(result.plan)
    medium = ReferenceMedium(result.plan)
    assert medium.digest == result.receipt["executable_plan_sha256"]
