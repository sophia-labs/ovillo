from __future__ import annotations

import math
import unittest

import numpy as np

from unified_reference.compiler import (
    compile_unitary_on_tree,
    path_graph,
    random_unitary,
)
from unified_reference.claims import ClaimEvidence, ClaimLevel, evaluate_claims
from unified_reference.connection import (
    apply_weighted_transport,
    fixed_capacity_material_connection,
    periodic_cell_metric,
    periodic_linear_resampling_matrix,
    sparsify_transport_rows,
    weighted_polar_transport,
)
from unified_reference.instrument import (
    causal_feedback_certificate,
    coupling_sweep,
    design_order_observable,
    verify_persistence,
)
from unified_reference.model import (
    COMPLEX_DIMENSION,
    GAMMAS,
    FamilyParameters,
    complex_structure,
    exact_base_radius_squared,
    ordinary_real_givens_symplectic_residual,
    physical_mixer,
    protected_projector,
    pseudo_time_reversal_residual,
    pseudo_time_reversal_square_residual,
    real_generator_from_hamiltonian,
    region_participation,
    relative_hamiltonian,
    unitary_law_residuals,
)
from unified_reference.nonlinear import certify_nonlinear_envelope
from unified_reference.open_system import (
    PassiveRadiationPort,
    inject_source,
    retire_modes,
)
from unified_reference.perturbations import run_perturbation_sweep
from unified_reference.topology import (
    analytic_second_chern,
    certify_perturbation_box,
    exact_gap_certificate,
    pure_delay_spectral_crowding,
    second_chern_degree,
    second_chern_lattice_gauge,
)
from unified_reference.transport import (
    DEFAULT_BASE,
    LOOP_A,
    LOOP_B,
    compile_holonomy,
    jrs_adiabatic_bound,
    kato_path,
    minimum_duration_for_jrs_budget,
    nonabelian_certificate,
    simulate_adiabatic_loop,
    verify_gauge_covariance,
    verify_path_laws,
)


class ModelTests(unittest.TestCase):
    def test_clifford_algebra(self) -> None:
        identity = np.eye(4)
        for i, left in enumerate(GAMMAS):
            for j, right in enumerate(GAMMAS):
                expected = 2.0 * identity if i == j else np.zeros((4, 4))
                self.assertLess(
                    np.linalg.norm(left @ right + right @ left - expected), 1.0e-12
                )

    def test_exact_gap_identity(self) -> None:
        rng = np.random.default_rng(42)
        for _ in range(64):
            k = rng.uniform(-math.pi, math.pi, size=4)
            self.assertAlmostEqual(
                exact_base_radius_squared(k),
                float(np.linalg.norm(
                    np.concatenate((np.sin(k), [-3.0 + np.sum(np.cos(k))]))
                ) ** 2),
                places=12,
            )
            self.assertGreaterEqual(exact_base_radius_squared(k), 1.0 - 1.0e-12)
        certificate = exact_gap_certificate()
        self.assertEqual(certificate.protected_cluster_gap, 2.0)

    def test_coupled_region_participation_and_symmetry(self) -> None:
        projector = protected_projector(DEFAULT_BASE)
        np.testing.assert_allclose(region_participation(projector), (0.25,) * 4, atol=1.0e-12)
        self.assertLess(pseudo_time_reversal_residual(DEFAULT_BASE), 1.0e-12)
        self.assertLess(pseudo_time_reversal_square_residual(), 1.0e-12)
        sweep = coupling_sweep(DEFAULT_BASE)
        gaps = [point.protected_gap for point in sweep]
        self.assertLess(max(gaps) - min(gaps), 1.0e-11)
        self.assertEqual(sweep[0].participation, (1.0, 0.0, 0.0, 0.0))

    def test_complex_structure_law(self) -> None:
        generator = real_generator_from_hamiltonian(relative_hamiltonian(DEFAULT_BASE))
        self.assertLess(np.linalg.norm(generator + generator.T), 1.0e-12)
        laws = unitary_law_residuals(physical_mixer(1.0))
        self.assertLess(laws["real_symplectic_fro"], 1.0e-12)
        self.assertGreater(ordinary_real_givens_symplectic_residual(), 0.1)


class TopologyTests(unittest.TestCase):
    def test_two_independent_second_chern_estimators(self) -> None:
        self.assertEqual(analytic_second_chern(-3.0), 1)
        self.assertEqual(analytic_second_chern(-5.0), 0)
        degree = second_chern_degree(grid=8)
        lattice = second_chern_lattice_gauge(grid=6)
        self.assertEqual(degree.nearest_integer, 1)
        self.assertEqual(lattice.nearest_integer, 1)
        self.assertTrue(lattice.branch_safe)
        trivial = second_chern_lattice_gauge(mass=-5.0, grid=6)
        self.assertEqual(trivial.nearest_integer, 0)

    def test_analytic_perturbation_box(self) -> None:
        certificate = certify_perturbation_box()
        self.assertTrue(certificate.rank_preserved)
        self.assertTrue(certificate.c2_preserved)
        self.assertGreater(certificate.protected_gap_lower_bound, 1.5)

    def test_empirical_perturbation_corpus(self) -> None:
        certificate = run_perturbation_sweep(
            trials=4, degree_grid=6, path_points=65
        )
        self.assertGreater(certificate.minimum_commutator_fro, 1.0)
        self.assertGreater(certificate.minimum_sampled_global_gap, 1.5)
        self.assertLess(certificate.maximum_second_chern_integer_error, 0.3)

    def test_delay_spectral_crowding(self) -> None:
        one_second = pure_delay_spectral_crowding(48_000)
        self.assertEqual(one_second.modal_spacing_hz, 1.0)
        two_seconds = pure_delay_spectral_crowding(96_000)
        self.assertEqual(two_seconds.modal_spacing_hz, 0.5)


class CompilerTests(unittest.TestCase):
    def test_complex_tree_qr(self) -> None:
        rng = np.random.default_rng(1234)
        for n in range(2, 9):
            certificate = compile_unitary_on_tree(
                random_unitary(n, rng), path_graph(n)
            )
            self.assertLess(certificate.reconstruction_error_fro, 1.0e-11)
            self.assertLess(certificate.real_symplectic_error_fro, 1.0e-11)
            self.assertTrue(certificate.all_two_mode_gates_on_graph)
            self.assertEqual(certificate.two_mode_gate_count, n * (n - 1) // 2)


class TransportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.loop_a = kato_path(LOOP_A, points=129)
        cls.loop_b = kato_path(LOOP_B, points=129)

    def test_nonabelian_and_gauge_receipts(self) -> None:
        nonabelian = nonabelian_certificate(self.loop_a, self.loop_b)
        self.assertGreater(nonabelian.commutator_fro, 1.5)
        gauge = verify_gauge_covariance(self.loop_a, self.loop_b)
        self.assertLess(gauge.loop_matrix_covariance_error_fro, 1.0e-12)
        self.assertLess(gauge.commutator_norm_error, 1.0e-12)

    def test_path_laws_and_complex_compilation(self) -> None:
        laws = verify_path_laws(LOOP_A, coarse_points=65)
        self.assertLess(laws.reversal_error_fro, 1.0e-10)
        self.assertLess(laws.concatenation_error_fro, 1.0e-10)
        self.assertLess(laws.identity_error_fro, 1.0e-10)
        self.assertLess(laws.coarse_fine_error_fro, 0.02)
        compiled = compile_holonomy(self.loop_a)
        self.assertLess(compiled.reconstruction_error_fro, 1.0e-10)
        self.assertLess(compiled.real_symplectic_error_fro, 1.0e-10)

    def test_persistence_and_causal_room_feedback(self) -> None:
        design = design_order_observable(self.loop_a, self.loop_b)
        self.assertGreater(design.certificate.raw_top_transfer_gain, 0.2)
        self.assertLess(design.certificate.raw_blind_transfer_gain, 1.0e-12)
        self.assertGreater(design.certificate.calibrated_transfer_gain, 1.0)
        self.assertLess(
            design.certificate.calibrated_prepared_state_outside_protected_fro,
            1.0e-10,
        )
        self.assertLess(
            design.certificate.calibrated_difference_outside_room_fro,
            1.0e-10,
        )
        persistence = verify_persistence(
            self.loop_a, self.loop_b, design.prepared_vector
        )
        self.assertLess(persistence.segmented_vs_concatenated_error, 1.0e-13)
        self.assertLess(persistence.direct_vs_step_replay_error, 1.0e-12)
        self.assertGreater(persistence.reset_between_loops_difference, 0.2)
        feedback = causal_feedback_certificate(DEFAULT_BASE, time_points=101)
        self.assertGreater(feedback.intervention_string_response, 0.1)
        self.assertLess(feedback.decoupled_room_to_string_transfer_fro, 1.0e-12)

    def test_finite_time_and_refusal(self) -> None:
        minimum = minimum_duration_for_jrs_budget(
            LOOP_A, requested_projector_error=0.05
        )
        self.assertGreater(minimum, 1.0)
        rejected = jrs_adiabatic_bound(
            LOOP_A, physical_duration_seconds=1.0, requested_projector_error=0.05
        )
        accepted = jrs_adiabatic_bound(
            LOOP_A,
            physical_duration_seconds=minimum * 1.001,
            requested_projector_error=0.05,
        )
        self.assertFalse(rejected.accepted)
        self.assertTrue(accepted.accepted)
        replay = simulate_adiabatic_loop(
            LOOP_B,
            self.loop_b,
            duration_seconds=0.1,
            sample_rate_hz=2_000.0,
        )
        self.assertLess(replay.active_unitarity_error_fro, 1.0e-10)
        self.assertEqual(replay.host_block_invariance_error_fro, 0.0)


class OpenSystemTests(unittest.TestCase):
    def test_source_radiation_and_retirement_ledgers(self) -> None:
        state = np.zeros(COMPLEX_DIMENSION, dtype=np.complex128)
        source = np.zeros_like(state)
        source[0] = 0.5
        state, source_frame = inject_source(state, source)
        self.assertAlmostEqual(source_frame.source_work, 0.25)
        rows = np.zeros((2, COMPLEX_DIMENSION), dtype=np.complex128)
        rows[0, 12] = 1.0
        rows[1, 13] = 1.0
        port = PassiveRadiationPort(rows, 0.1)
        self.assertLess(port.law_residual(), 1.0e-12)
        retained, outgoing, frame = port.scatter(state)
        self.assertLess(abs(frame.closure_error), 1.0e-12)
        with self.assertRaises(ValueError):
            retire_modes(state, [0], policy="reject")
        retired, receipt = retire_modes(state, [0], policy="radiate")
        self.assertAlmostEqual(receipt.retirement_radiation, 0.25)
        self.assertAlmostEqual(np.linalg.norm(retired), 0.0)

    def test_nonlinear_claim_boundary(self) -> None:
        accepted = certify_nonlinear_envelope(
            duration_seconds=0.1,
            spectral_gap=2.0,
            jacobian_operator_norm_bound=0.1,
            forcing_norm_bound=0.01,
            trajectory_error_budget=0.01,
        )
        self.assertTrue(accepted.accepted_for_linear_topological_claim)
        rejected = certify_nonlinear_envelope(
            duration_seconds=1.0,
            spectral_gap=2.0,
            jacobian_operator_norm_bound=1.1,
            forcing_norm_bound=0.01,
            trajectory_error_budget=1.0,
        )
        self.assertFalse(rejected.accepted_for_linear_topological_claim)


class ConnectionTests(unittest.TestCase):
    def test_weighted_polar_expansion_is_energy_preserving(self) -> None:
        raw = periodic_linear_resampling_matrix(8, 11)
        source_metric = periodic_cell_metric(8)
        target_metric = periodic_cell_metric(11)
        plan = weighted_polar_transport(raw, source_metric, target_metric)
        self.assertTrue(plan.certificate.universal_energy_preservation)
        self.assertLess(plan.certificate.source_isometry_error_fro, 1.0e-11)
        rng = np.random.default_rng(17)
        state = rng.normal(size=8)
        target, receipt = apply_weighted_transport(
            state, plan, source_metric, target_metric
        )
        self.assertEqual(target.shape, (11,))
        self.assertLess(abs(receipt.initial_energy - receipt.final_energy), 1.0e-11)

    def test_weighted_polar_contraction_requires_disposal(self) -> None:
        raw = periodic_linear_resampling_matrix(11, 8)
        source_metric = periodic_cell_metric(11)
        target_metric = periodic_cell_metric(8)
        plan = weighted_polar_transport(raw, source_metric, target_metric)
        self.assertTrue(plan.certificate.contraction_requires_disposal)
        state = np.arange(11, dtype=np.float64)
        with self.assertRaises(ValueError):
            apply_weighted_transport(state, plan, source_metric, target_metric)
        _, receipt = apply_weighted_transport(
            state, plan, source_metric, target_metric, policy="radiate"
        )
        self.assertGreaterEqual(receipt.retirement_radiation, 0.0)
        self.assertLess(abs(receipt.numerical_residual), 1.0e-10)

    def test_sparse_connection_is_a_measured_approximation(self) -> None:
        raw = periodic_linear_resampling_matrix(16, 23)
        source_metric = periodic_cell_metric(16)
        target_metric = periodic_cell_metric(23)
        plan = weighted_polar_transport(raw, source_metric, target_metric)
        sparse, certificate = sparsify_transport_rows(
            plan.transport,
            source_metric,
            target_metric,
            maximum_nonzeros_per_row=4,
            sample_states=32,
        )
        self.assertEqual(sparse.shape, plan.transport.shape)
        self.assertLessEqual(certificate.actual_nonzeros, 23 * 4)
        self.assertGreater(certificate.operator_error_fro, 0.0)

    def test_fixed_capacity_connection_is_identity(self) -> None:
        certificate = fixed_capacity_material_connection(
            capacity_samples=512,
            old_active_samples=221.5,
            new_active_samples=317.25,
        )
        self.assertTrue(certificate.connection_is_identity)
        self.assertFalse(certificate.retirement_required)


class ClaimLatticeTests(unittest.TestCase):
    def test_reference_and_production_claims_do_not_collapse(self) -> None:
        decision = evaluate_claims(
            ClaimEvidence(
                same_endpoint_paths=True,
                persistent_state_order_defect=True,
                projector_transport=True,
                globally_isolated_rank_two_family=True,
                nonzero_second_chern=True,
                finite_time_replay=True,
                theorem_budget_accepted=False,
                lawful_sparse_lowering=True,
                protected_modes_acoustically_participating=True,
                passive_radiation_from_state=True,
                one_whole_instrument_ledger=True,
                production_cross_replay=False,
                listener_discrimination=False,
                correction_or_fault_tolerance=False,
            )
        )
        self.assertEqual(
            decision.maximum_reference_claim, ClaimLevel.PROTECTED_ACOUSTIC
        )
        self.assertIsNone(decision.maximum_production_claim)
        self.assertNotIn(ClaimLevel.FAULT_TOLERANT, decision.promoted)


if __name__ == "__main__":
    unittest.main()
