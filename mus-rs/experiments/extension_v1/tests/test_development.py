import torch

from extension_v1.clifford import CliffordConfig, stack_generators
from extension_v1.development import (
    DevelopmentConfig,
    develop_operator,
    development_receipt,
    inverse_path,
    rectangle_loop,
    split_segments,
)


def test_closed_loop_has_path_dependent_residue_but_no_energy_drift() -> None:
    receipt = development_receipt(DevelopmentConfig(small_loop_points=10))
    loop = receipt["closed_rectangle"]
    assert loop["control_endpoint_norm"] < 1e-15
    assert loop["state_distance_from_vacuum"] > 0.1
    assert loop["operator_distance_from_identity"] > 0.1
    assert abs(loop["energy"] - 1.0) < 1e-12
    assert receipt["inverse_loop"]["operator_inverse_residual"] < 1e-12
    assert receipt["reparameterization"]["operator_residual"] < 1e-12
    exponent = receipt["small_loop_area_law"][
        "bivector_abs_vs_area_loglog_exponent"
    ]
    assert 0.97 < exponent < 1.03


def test_development_is_exact_under_segment_splitting_and_reversal() -> None:
    generators = stack_generators(CliffordConfig())
    path = rectangle_loop(0.4, 0.3)
    refined = split_segments(path, [2, 7, 3, 5])
    operator = develop_operator(path, generators)
    refined_operator = develop_operator(refined, generators)
    inverse = develop_operator(inverse_path(path), generators)
    identity = torch.eye(16, dtype=torch.float64)
    assert torch.allclose(operator, refined_operator, atol=1e-12)
    assert torch.allclose(operator @ inverse, identity, atol=1e-12)
