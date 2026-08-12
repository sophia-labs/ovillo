from extension_v1.morphology import MorphologyConfig, optimize_morphology


def test_short_morphology_optimization_reduces_operator_error() -> None:
    _, receipt = optimize_morphology(
        MorphologyConfig(
            ambient_dim=4,
            kernel_width=0.50,
            seed=41,
            train_steps=30,
            eval_trials=64,
        ),
        save_checkpoint=False,
    )
    assert receipt["final"]["operator_mse"] < receipt["initial"]["operator_mse"]
    assert receipt["trainable_parameters"] == 44
