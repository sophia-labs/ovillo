from __future__ import annotations

from token_radiation_paired_operator_v1.paired import PairedConfig, train_paired_condition
from token_radiation_paired_operator_v1.process import ProcessConfig, generate_corpus


def _corpus():
    return generate_corpus(
        ProcessConfig(
            sequence_bytes=12,
            train_sequences=8,
            validation_sequences=3,
            seed=17,
            family="structured",
        )
    )


def test_zero_fraction_paired_training_is_exact() -> None:
    config = PairedConfig(
        radiation_fractions=(0.0,),
        seeds=(91,),
        sites=4,
        radiation_rank=2,
        body_steps=2,
        batch_size=3,
        probe_steps=2,
        checkpoint_every=1,
    )
    row, _ = train_paired_condition(_corpus(), config, fraction=0.0, seed=91)
    assert row["zero_fraction_exact_identity"] is True
    assert row["paired_delta_bits_per_byte"] == 0.0
    assert row["final_passive_parameter_sha256"] == row["final_nonloading_parameter_sha256"]
    for probe in row["horizon_probes"].values():
        assert probe["paired_delta_bits_per_byte"] == 0.0
    for pulse in row["single_pulse_on_nonloading_checkpoint"]["offsets"].values():
        assert pulse["metric_wave_distance"] == 0.0
        assert pulse["excess_nll_nats"] == 0.0


def test_nonzero_single_pulse_is_delayed_and_receipted() -> None:
    config = PairedConfig(
        radiation_fractions=(0.2,),
        seeds=(92,),
        sites=4,
        radiation_rank=2,
        body_steps=2,
        batch_size=3,
        probe_steps=2,
        checkpoint_every=1,
    )
    row, _ = train_paired_condition(_corpus(), config, fraction=0.2, seed=92)
    pulse = row["single_pulse_on_nonloading_checkpoint"]
    assert pulse["mean_booked_radiation"] > 0.0
    assert pulse["maximum_closure_residual"] < 1.0e-10
    assert abs(pulse["offsets"]["0"]["excess_nll_nats"]) < 1.0e-14
    assert pulse["offsets"]["1"]["metric_wave_distance"] > 0.0
