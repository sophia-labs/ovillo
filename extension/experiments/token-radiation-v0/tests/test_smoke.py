from __future__ import annotations

from pathlib import Path

from token_radiation_v0.experiment import CampaignConfig, run_campaign, verify_output


def test_tiny_campaign_runs_and_verifies(tmp_path: Path) -> None:
    output = tmp_path / "result"
    payload = run_campaign(
        CampaignConfig(
            seed=17,
            sequence_bytes=12,
            train_sequences=10,
            validation_sequences=4,
            body_steps=2,
            body_batch_size=3,
            detector_steps=2,
            detector_batch_size=2,
            vocabulary_size=10,
            detector_kinds=("linear",),
            radiation_modes=("passive",),
            sites=6,
            radiation_rank=3,
        ),
        output,
    )
    assert payload["detector_results"]
    receipt = verify_output(output)
    assert receipt["valid"] is True
