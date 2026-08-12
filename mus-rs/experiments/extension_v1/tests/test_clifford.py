import torch

from extension_v1.clifford import (
    CliffordConfig,
    clifford_generators,
    continuous_volume_experiment,
    exact_receipt,
    permutation_experiment,
)


def test_exact_clifford_identities_and_parity() -> None:
    config = CliffordConfig(random_trials=256, seed=5)
    receipt = exact_receipt(config)
    assert receipt["state_dimension"] == 16
    assert receipt["grade_string_site_counts"] == [1, 4, 6, 4, 1]
    assert receipt["local_rotations_per_basis_gesture"] == 8
    assert receipt["algebra"]["max_square_residual"] < 1e-12
    assert receipt["algebra"]["max_anticommutator_residual"] < 1e-12
    assert receipt["algebra"]["max_orthogonality_residual"] < 1e-12
    assert receipt["permutation_parity"]["exact_amplitude_accuracy"] == 1.0
    assert receipt["continuous_oriented_volume"]["maximum_volume_residual"] < 1e-12
    assert receipt["continuous_oriented_volume"]["maximum_energy_drift"] < 1e-12


def test_every_generator_is_a_disjoint_quarter_turn_program() -> None:
    config = CliffordConfig()
    generators = clifford_generators(config)
    identity = torch.eye(16, dtype=torch.float64)
    for generator in generators:
        assert torch.allclose(generator.T @ generator, identity, atol=1e-12)
        assert torch.allclose(generator @ generator, -identity, atol=1e-12)
        assert abs(float(torch.linalg.det(generator)) - 1.0) < 1e-12


def test_public_experiments_accept_explicit_generators() -> None:
    config = CliffordConfig(random_trials=64)
    generators = clifford_generators(config)
    assert permutation_experiment(config, generators)["sign_accuracy"] == 1.0
    assert continuous_volume_experiment(config, generators)["sign_accuracy"] == 1.0
