from extension_v1.closed_gestures import (
    ClosedGestureConfig,
    commuting_control,
    train_closed_gestures,
)


def test_closed_token_gestures_learn_permutation_parity() -> None:
    model, receipt = train_closed_gestures(
        ClosedGestureConfig(
            seed=101,
            fibers=1,
            segments=6,
            train_steps=300,
            trace_every=50,
        ),
        save_checkpoint=False,
    )
    assert receipt["accuracy"] == 1.0
    assert receipt["parameter_count"] == 130
    assert receipt["max_loop_closure"] < 1e-12
    assert receipt["max_energy_drift"] < 1e-11

    control = commuting_control(model)
    assert control["clifford_noncommuting"]["linear_probe_accuracy"] == 1.0
    assert control["commuting_collapse"]["linear_probe_accuracy"] == 0.5
    assert control["commuting_collapse"]["maximum_pairwise_state_distance"] < 1e-12
