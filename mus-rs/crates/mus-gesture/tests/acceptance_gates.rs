//! Consensus §7 Phase 1 acceptance gates, verbatim.
//!
//! * dimensions 2–32 over path, star, balanced tree, complete, and random
//!   connected graphs;
//! * every gate on an authored edge;
//! * generic full-target count `n(n-1)/2`;
//! * worst reconstruction error below `1e-11` Frobenius over at least 1,000
//!   random targets;
//! * determinant `-1` and cross-component targets rejected with a typed
//!   error;
//! * constant-speed identity closes analytically and numerically;
//! * search never worsens the baseline;
//! * receipts reproduce under independent replay.

mod support;

use mus_gesture::{
    basis_probes, compile, compile_block_diagonal, measure_control_work, replay,
    search_factorization, thermodynamic_schedule, ControlGraph, ErrorClass, FrictionMetric,
    GestureError, GestureReceipt, Matrix, RotationTarget, SearchConfig, SplitMix64,
};
use np_rand::Pcg64;
use support::{
    dense_givens, family_graph, haar_reflection, haar_target, weighted_family_graph, Family,
};

const DIMENSIONS: std::ops::RangeInclusive<usize> = 2..=32;
const TARGETS_PER_CASE: usize = 7;
const RECONSTRUCTION_GATE: f64 = 1.0e-11;

/// The headline gate: 1,085 Haar targets across every dimension and every
/// named family, each compiled exactly and checked four ways.
#[test]
fn exact_compiler_meets_every_phase_one_numeric_gate() {
    let mut worst_reconstruction = 0.0_f64;
    let mut worst_orthogonality = 0.0_f64;
    let mut worst_determinant = 0.0_f64;
    let mut targets = 0usize;
    let mut gates_checked = 0usize;
    let mut worst_case = String::new();

    for n in DIMENSIONS {
        for family in Family::ALL {
            let mut structure = SplitMix64::new(0x9e37_79b9 ^ ((n as u64) << 8) ^ n as u64);
            let graph = family_graph(family, n, &mut structure);
            assert!(
                graph.is_connected(),
                "{} at n={n} must be connected",
                family.name()
            );
            let mut normals = Pcg64::seeded(0x0a71_ad0c ^ (n as u64) << 16 ^ family as u64);

            for trial in 0..TARGETS_PER_CASE {
                let target = haar_target(n, &mut normals);
                let certificate = compile(&target, &graph).unwrap();
                targets += 1;

                // Generic count.
                assert_eq!(
                    certificate.gate_count(),
                    n * (n - 1) / 2,
                    "{} n={n} trial={trial}",
                    family.name()
                );
                assert_eq!(
                    certificate.gate_count(),
                    certificate.dimension_lower_bound()
                );

                // Every gate on an authored edge, and on the edge it names.
                for gate in certificate.gates() {
                    let edge = graph
                        .edge_between(gate.left, gate.right)
                        .unwrap_or_else(|| {
                            panic!(
                                "{} n={n}: gate on unauthored plane ({}, {})",
                                family.name(),
                                gate.left,
                                gate.right
                            )
                        });
                    assert_eq!(edge.id, gate.edge);
                    gates_checked += 1;
                }

                if certificate.reconstruction_error_fro() > worst_reconstruction {
                    worst_reconstruction = certificate.reconstruction_error_fro();
                    worst_case = format!("{} n={n} trial={trial}", family.name());
                }
                worst_orthogonality =
                    worst_orthogonality.max(certificate.orthogonality_error_fro());
                worst_determinant = worst_determinant.max(certificate.determinant_error());
            }
        }
    }

    assert!(
        targets >= 1_000,
        "the gate asks for at least 1,000 targets, ran {targets}"
    );
    assert!(
        worst_reconstruction < RECONSTRUCTION_GATE,
        "worst reconstruction {worst_reconstruction:e} at {worst_case} \
         is not below {RECONSTRUCTION_GATE:e}"
    );
    assert!(worst_orthogonality < RECONSTRUCTION_GATE);
    assert!(worst_determinant < RECONSTRUCTION_GATE);
    assert!(gates_checked > 100_000);
    println!(
        "targets={targets} gates={gates_checked} \
         worst_reconstruction={worst_reconstruction:e} ({worst_case}) \
         worst_orthogonality={worst_orthogonality:e} worst_determinant={worst_determinant:e}"
    );
}

/// The lowered `mus-watsn` program is the executable artifact, and executing
/// it must agree with the dense target on every state — not merely on the
/// basis vectors the endpoint was reconstructed from.
#[test]
fn the_lowered_program_transports_arbitrary_states_to_the_target() {
    let mut structure = SplitMix64::new(31);
    let mut normals = Pcg64::seeded(77);
    for n in [2usize, 3, 5, 9, 17, 32] {
        let graph = family_graph(Family::RandomConnected, n, &mut structure);
        let target = haar_target(n, &mut normals);
        let certificate = compile(&target, &graph).unwrap();
        for _ in 0..8 {
            let state = normals.standard_normal_vec(n);
            let mut executed = state.clone();
            certificate.program().apply(&mut executed).unwrap();
            for (row, actual) in executed.iter().enumerate() {
                let expected: f64 = (0..n)
                    .map(|column| target.matrix().get(row, column) * state[column])
                    .sum();
                assert!(
                    (actual - expected).abs() <= 1.0e-11 * (1.0 + expected.abs()),
                    "n={n} row={row}: {actual} vs {expected}"
                );
            }
        }
    }
}

/// Inverting the word inverts the transport, and the round trip returns the
/// state exactly enough to bound rather than to hope.
#[test]
fn inverse_replay_returns_the_state() {
    let mut structure = SplitMix64::new(1_009);
    let mut normals = Pcg64::seeded(1_009);
    for n in [3usize, 8, 16, 32] {
        let graph = family_graph(Family::Path, n, &mut structure);
        let target = haar_target(n, &mut normals);
        let certificate = compile(&target, &graph).unwrap();
        let inverse = certificate.program().inverse();
        let original = normals.standard_normal_vec(n);
        let mut state = original.clone();
        certificate.program().apply(&mut state).unwrap();
        inverse.apply(&mut state).unwrap();
        for (actual, expected) in state.iter().zip(original.iter()) {
            assert!((actual - expected).abs() <= 1.0e-12 * (1.0 + expected.abs()));
        }
    }
}

/// Determinant `-1` targets are rejected with a typed, `Unreachable` error at
/// every dimension in the gate range.
#[test]
fn reflections_are_rejected_typed_at_every_dimension() {
    let mut normals = Pcg64::seeded(0xdead_beef);
    for n in DIMENSIONS {
        let reflection = haar_reflection(n, &mut normals);
        let determinant = reflection.determinant();
        assert!(
            (determinant - -1.0).abs() < 1.0e-9,
            "n={n}: constructed reflection has determinant {determinant}"
        );
        let error = RotationTarget::new(reflection, 1.0e-9).unwrap_err();
        match error {
            GestureError::TargetDeterminantNotUnit { determinant, .. } => {
                assert!((determinant - -1.0).abs() < 1.0e-9);
            }
            other => panic!("n={n}: expected a determinant rejection, got {other}"),
        }
        assert_eq!(error_class_of_reflection(), ErrorClass::Unreachable);
    }
}

fn error_class_of_reflection() -> ErrorClass {
    GestureError::TargetDeterminantNotUnit {
        determinant: -1.0,
        tolerance: 1.0e-9,
    }
    .class()
}

/// A disconnected graph rejects a full `SO(n)` target, a cross-component
/// target is rejected typed, and a genuinely block-diagonal target is
/// explicitly decomposed instead.
#[test]
fn cross_component_targets_are_rejected_and_block_targets_decompose() {
    let mut normals = Pcg64::seeded(4_242);
    for n in 4..=16usize {
        let cut = n / 2;
        let mut pairs: Vec<(usize, usize)> =
            (0..cut.saturating_sub(1)).map(|i| (i, i + 1)).collect();
        pairs.extend((cut..n - 1).map(|i| (i, i + 1)));
        let graph = ControlGraph::from_coordinate_edges(n, &pairs).unwrap();
        let report = graph.component_report();
        assert_eq!(report.components.len(), 2);

        // A full Haar target on a disconnected graph.
        let full = haar_target(n, &mut normals);
        assert_eq!(
            compile(&full, &graph).unwrap_err(),
            GestureError::DisconnectedGraph { components: 2 }
        );
        match compile_block_diagonal(&full, &graph).unwrap_err() {
            GestureError::CrossComponentTarget { row, column, .. } => {
                assert_ne!(report.membership[row], report.membership[column]);
            }
            other => panic!("n={n}: expected a cross-component rejection, got {other}"),
        }

        // A target that respects the block structure decomposes exactly.
        let block = dense_givens(n, 0, cut - 1, 0.83).multiply(&dense_givens(n, cut, n - 1, -1.4));
        let block_target = RotationTarget::new(block, 1.0e-9).unwrap();
        let decomposed = compile_block_diagonal(&block_target, &graph).unwrap();
        assert_eq!(decomposed.blocks().len(), 2);
        assert_eq!(
            decomposed.gate_count(),
            report.rotation_dimension(),
            "n={n}: block gate count"
        );
        assert!(decomposed.reconstruction_error_fro() < RECONSTRUCTION_GATE);
        for gate in decomposed.gates() {
            assert!(graph.edge_between(gate.left, gate.right).is_some());
        }
    }
}

/// The identity closes: an all-zero word, zero thermodynamic length, zero
/// predicted cost, an exactly-zero constant-speed residual, and per-edge
/// closure bounded (correction 3) rather than compared to zero.
#[test]
fn constant_speed_identity_closes_analytically_and_numerically() {
    let mut structure = SplitMix64::new(5);
    for n in DIMENSIONS {
        for family in Family::ALL {
            let graph = weighted_family_graph(family, n, &mut structure);
            let target = RotationTarget::identity(n).unwrap();
            let certificate = compile(&target, &graph).unwrap();
            assert_eq!(certificate.gate_count(), n * (n - 1) / 2);
            assert!(certificate.reconstruction_error_fro() <= 0.0);
            for gate in certificate.gates() {
                assert!(gate.theta.abs() <= 0.0);
            }
            for (edge, closure) in certificate.edge_closure() {
                assert!(
                    closure.abs() <= 1.0e-12,
                    "n={n} {} edge {edge:?} closure {closure:e}",
                    family.name()
                );
            }
            let metric = FrictionMetric::resolve(certificate.gates(), &graph).unwrap();
            let schedule = thermodynamic_schedule(certificate.gates(), &metric, 2.0).unwrap();
            assert_eq!(schedule.thermodynamic_length(), 0.0);
            assert_eq!(schedule.minimum_quadratic_cost(), 0.0);
            assert_eq!(schedule.constant_speed_residual(), 0.0);
            assert!(schedule.executable_indices().is_empty());
        }
    }
}

/// Constant thermodynamic speed for a non-trivial word: every executable
/// segment runs at the same speed to float64 rounding, and the closed-form
/// cost equals the cost the segments actually realize.
#[test]
fn constant_speed_timing_is_uniform_and_matches_the_closed_form() {
    let mut structure = SplitMix64::new(606);
    let mut normals = Pcg64::seeded(606);
    for n in [2usize, 3, 6, 11, 20, 32] {
        for family in Family::ALL {
            let graph = weighted_family_graph(family, n, &mut structure);
            let target = haar_target(n, &mut normals);
            let certificate = compile(&target, &graph).unwrap();
            let metric = FrictionMetric::resolve(certificate.gates(), &graph).unwrap();
            let schedule = thermodynamic_schedule(certificate.gates(), &metric, 2.5).unwrap();

            let total: f64 = schedule.segment_durations().iter().sum();
            assert!((total - 2.5).abs() <= 1.0e-12 * 2.5 * n as f64);

            let nominal = schedule.thermodynamic_length() / 2.5;
            assert!(
                schedule.constant_speed_residual() <= 1.0e-12 * nominal.max(1.0),
                "n={n} {}: residual {:e}",
                family.name(),
                schedule.constant_speed_residual()
            );
            let closed = schedule.minimum_quadratic_cost();
            let realized = schedule.realized_quadratic_cost();
            assert!((closed - realized).abs() <= 1.0e-9 * closed.max(1.0));

            // Dropping the zero-angle gates leaves the endpoint untouched.
            let trimmed = schedule.executable_program(certificate.gates(), n).unwrap();
            let mut state = normals.standard_normal_vec(n);
            let mut reference = state.clone();
            trimmed.apply(&mut state).unwrap();
            certificate.program().apply(&mut reference).unwrap();
            assert_eq!(state, reference);
        }
    }
}

/// Search never worsens the baseline, at every dimension and family, across
/// several seeds and budgets — and whatever it returns is still exact.
#[test]
fn search_never_worsens_the_baseline() {
    let mut structure = SplitMix64::new(0xb0a7);
    let mut normals = Pcg64::seeded(0xb0a7);
    let mut improved = 0usize;
    let mut compared = 0usize;
    for n in [3usize, 5, 8, 12, 17, 24, 32] {
        for family in Family::ALL {
            let graph = weighted_family_graph(family, n, &mut structure);
            let target = haar_target(n, &mut normals);
            for seed in [1u64, 0x0a71_ad0c] {
                let outcome =
                    search_factorization(&target, &graph, SearchConfig::new(24, seed, 2.0))
                        .unwrap();
                compared += 1;
                assert!(
                    outcome.best_cost() <= outcome.baseline_cost(),
                    "n={n} {} seed={seed}: {} > {}",
                    family.name(),
                    outcome.best_cost(),
                    outcome.baseline_cost()
                );
                assert!(outcome.improvement_fraction() >= 0.0);
                assert!(outcome.candidates_evaluated() >= 1);
                assert_eq!(outcome.certificate().gate_count(), n * (n - 1) / 2);
                assert!(outcome.certificate().reconstruction_error_fro() < RECONSTRUCTION_GATE);
                for gate in outcome.certificate().gates() {
                    assert!(graph.edge_between(gate.left, gate.right).is_some());
                }
                if outcome.best_cost() < outcome.baseline_cost() {
                    improved += 1;
                }
            }
        }
    }
    assert_eq!(compared, 70);
    // Not a correctness gate, but a search that never finds anything is not
    // being exercised.
    assert!(
        improved > 10,
        "search improved only {improved} of {compared}"
    );
}

/// Receipts reproduce under independent replay across the whole gate range,
/// including the measured work field and the serialized round trip.
#[test]
fn receipts_reproduce_under_independent_replay() {
    let mut structure = SplitMix64::new(0x2ece_1970);
    let mut normals = Pcg64::seeded(0x2ece_1970);
    for n in DIMENSIONS {
        let family = Family::ALL[n % Family::ALL.len()];
        let graph = weighted_family_graph(family, n, &mut structure);
        let target = haar_target(n, &mut normals);
        let certificate = compile(&target, &graph).unwrap();
        let metric = FrictionMetric::resolve(certificate.gates(), &graph).unwrap();
        let schedule = thermodynamic_schedule(certificate.gates(), &metric, 1.75).unwrap();
        let work = measure_control_work(certificate.program(), &basis_probes(n)).unwrap();
        let receipt = GestureReceipt::new(&graph, &target, &certificate, &schedule, Some(&work));

        let text = receipt.to_json().unwrap();
        let parsed = GestureReceipt::from_json(&text).unwrap();
        assert_eq!(parsed, receipt);
        assert_eq!(parsed.to_json().unwrap(), text);

        let report = replay(&parsed, &graph, &target).unwrap();
        assert_eq!(report.max_angle_difference, 0.0, "n={n}");
        assert_eq!(report.endpoint_difference_fro, 0.0, "n={n}");
        assert!(report.reconstruction_error_fro < RECONSTRUCTION_GATE);
        assert_eq!(report.certificate.gates(), certificate.gates());

        let measured = parsed.work_residual.as_ref().unwrap();
        assert_eq!(measured.metric, mus_gesture::EUCLIDEAN_NORMALIZED_METRIC);
        assert_eq!(measured.declared_control_work, 0.0);
        assert!(
            measured.max_abs_residual <= 1.0e-13,
            "n={n}: measured work residual {:e}",
            measured.max_abs_residual
        );
    }
}

/// A word composed with its own inverse is a closed control path: every
/// authored edge integrates to zero. Correction 3 — the assertion is a bound.
#[test]
fn a_closed_word_closes_on_every_edge_within_the_declared_bound() {
    let mut structure = SplitMix64::new(0xc105e);
    let mut normals = Pcg64::seeded(0xc105e);
    for n in [3usize, 7, 13, 21, 32] {
        let graph = family_graph(Family::RandomConnected, n, &mut structure);
        let target = haar_target(n, &mut normals);
        let certificate = compile(&target, &graph).unwrap();

        let mut closed: Vec<_> = certificate.gates().to_vec();
        closed.extend(certificate.gates().iter().rev().map(|gate| gate.inverse()));
        let mut worst = 0.0_f64;
        for (_, closure) in mus_gesture::edge_closure(&closed) {
            worst = worst.max(closure.abs());
        }
        assert!(
            worst <= 1.0e-12,
            "n={n}: worst per-edge closure {worst:e} exceeds 1e-12"
        );

        // And the closed word really is the identity operator.
        let program = mus_watsn::OperatorProgram::new(
            n,
            closed.iter().map(|gate| gate.to_plane_rotation()).collect(),
        )
        .unwrap();
        let endpoint = mus_gesture::endpoint_operator(&program).unwrap();
        assert!(endpoint.frobenius_distance(&Matrix::identity(n)) <= 1.0e-12);
    }
}
