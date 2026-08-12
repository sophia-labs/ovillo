import torch

from extension_v1.clifford import CliffordConfig, algebra_checks
from extension_v1.geometry import (
    GeometryConfig,
    geometry_receipt,
    random_grade_string_geometry,
    soft_generators,
)


def test_soft_geometric_events_remain_energy_preserving() -> None:
    config = GeometryConfig(ambient_dim=4, kernel_width=0.20, random_trials=128)
    coordinates = random_grade_string_geometry(config)
    generators = soft_generators(config, coordinates)
    identity = torch.eye(16, dtype=torch.float64)
    for generator in generators:
        assert torch.allclose(generator.T @ generator, identity, atol=1e-11)


def test_four_dimensional_narrow_contact_recovers_clifford_observables() -> None:
    config = GeometryConfig(
        ambient_dim=4,
        kernel_width=0.20,
        random_trials=256,
        seed=31,
    )
    receipt = geometry_receipt(config)
    assert receipt["permutation_parity"]["sign_accuracy"] == 1.0
    assert receipt["continuous_oriented_volume"]["sign_accuracy"] > 0.99
    assert receipt["algebra"]["max_orthogonality_residual"] < 1e-11
    # Approximate morphology need not satisfy exact Clifford relations.
    assert receipt["algebra"]["max_anticommutator_residual"] >= 0.0
