//! Phase 2 Commit F acceptance gates for the exact closed-control `SO(3)`
//! compiler, verbatim from `SPECS/ariadne/PHASE2-same-endpoint.md` §A:
//!
//! * 1,000 random targets, endpoint error below `1e-11` Frobenius;
//! * the direct dense commutator and the lowered word agree;
//! * the identity target compiles to the empty word;
//! * near-identity and near-pi targets are in the corpus;
//! * `work_residual` is `None` or ledger-measured (Phase 1 rule 4);
//!
//! plus the structural claims the same section makes: exactly twelve
//! authored-edge gates for every nonidentity target, and per-edge closure
//! asserted `<= 1e-12` and never `== 0.0`.

mod support;

use std::f64::consts::PI;

use mus_gesture::{
    basis_probes, compile_exact_closed_loop_so3, measure_control_work, replay_closed_loop,
    ClosedLoopReceipt, ConjugatorBranch, ControlGraph, ErrorClass, GestureError, Matrix,
    RotationTarget, SplitMix64, CLOSED_LOOP_GATE_COUNT, EUCLIDEAN_NORMALIZED_METRIC,
};
use np_rand::Pcg64;
use support::{haar_special_orthogonal, uniform01};

const RANDOM_TARGETS: usize = 1_000;
const ENDPOINT_GATE: f64 = 1.0e-11;
const CLOSURE_GATE: f64 = 1.0e-12;

/// The three connected three-mode authored graphs, with heterogeneous
/// friction so the timing gate has something to weigh.
fn graphs(rng: &mut SplitMix64) -> Vec<(&'static str, ControlGraph)> {
    let shapes: [(&'static str, &[(usize, usize)]); 3] = [
        ("path_012", &[(0, 1), (1, 2)]),
        ("path_021", &[(0, 2), (1, 2)]),
        ("triangle", &[(0, 1), (1, 2), (0, 2)]),
    ];
    shapes
        .iter()
        .map(|(name, pairs)| {
            let modes = (0..3u32).map(mus_gesture::ModeId).collect();
            let edges = pairs
                .iter()
                .enumerate()
                .map(|(index, &(left, right))| {
                    mus_gesture::EdgeControl::with_friction(
                        mus_gesture::EdgeId(index as u32),
                        mus_gesture::ModeId(left as u32),
                        mus_gesture::ModeId(right as u32),
                        (-1.2 + 2.4 * uniform01(rng)).exp(),
                    )
                })
                .collect();
            (*name, ControlGraph::new(modes, edges).unwrap())
        })
        .collect()
}

/// Rodrigues, written out independently of the compiler.
fn axis_angle(axis: [f64; 3], angle: f64) -> Matrix {
    let norm = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    let axis = [axis[0] / norm, axis[1] / norm, axis[2] / norm];
    let (sin, cos) = angle.sin_cos();
    let mut out = Matrix::zeros(3, 3);
    for row in 0..3 {
        for column in 0..3 {
            let kronecker = if row == column { 1.0 } else { 0.0 };
            out.set(
                row,
                column,
                kronecker * cos + (1.0 - cos) * axis[row] * axis[column],
            );
        }
    }
    out.set(0, 1, out.get(0, 1) - sin * axis[2]);
    out.set(0, 2, out.get(0, 2) + sin * axis[1]);
    out.set(1, 0, out.get(1, 0) + sin * axis[2]);
    out.set(1, 2, out.get(1, 2) - sin * axis[0]);
    out.set(2, 0, out.get(2, 0) - sin * axis[1]);
    out.set(2, 1, out.get(2, 1) + sin * axis[0]);
    out
}

/// The headline gate: 1,000 Haar `SO(3)` targets across all three connected
/// three-mode graphs, each compiled closed and checked six ways.
#[test]
fn the_closed_compiler_meets_every_phase_two_numeric_gate() {
    let mut structure = SplitMix64::new(0xc105_ed10_0f11);
    let all = graphs(&mut structure);
    let mut normals = Pcg64::seeded(0xc105_ed10_0a11);

    let mut worst_endpoint = 0.0_f64;
    let mut worst_endpoint_case = String::new();
    let mut worst_endpoint_rad = 0.0_f64;
    let mut worst_closure = 0.0_f64;
    let mut worst_commutator = 0.0_f64;
    let mut worst_orthogonality = 0.0_f64;
    let mut worst_determinant = 0.0_f64;
    let mut worst_speed = 0.0_f64;
    let mut branches = [0usize; 3];
    let mut compiled = 0usize;

    for trial in 0..RANDOM_TARGETS {
        let (name, graph) = &all[trial % all.len()];
        let matrix = haar_special_orthogonal(3, &mut normals);
        let target = RotationTarget::new(matrix, 1.0e-9).unwrap();
        let duration = 0.5 + 2.0 * uniform01(&mut structure);
        let certificate = compile_exact_closed_loop_so3(&target, graph, duration)
            .unwrap_or_else(|error| panic!("{name} trial {trial}: {error}"));
        compiled += 1;

        // Exactly twelve authored-edge gates.
        assert_eq!(
            certificate.gate_count(),
            CLOSED_LOOP_GATE_COUNT,
            "{name} trial {trial}"
        );
        for gate in certificate.gates() {
            let edge = graph
                .edge_between(gate.left, gate.right)
                .unwrap_or_else(|| panic!("{name} trial {trial}: gate off the authored graph"));
            assert_eq!(edge.id, gate.edge);
        }

        // Endpoint.
        let endpoint_error = certificate.reconstruction_error_fro();
        assert!(
            endpoint_error < ENDPOINT_GATE,
            "{name} trial {trial}: endpoint error {endpoint_error:e}"
        );
        if endpoint_error > worst_endpoint {
            worst_endpoint = endpoint_error;
            worst_endpoint_case = format!("{name} trial {trial}");
        }
        worst_endpoint_rad = worst_endpoint_rad.max(certificate.reconstruction_error_rad());
        worst_orthogonality = worst_orthogonality.max(certificate.orthogonality_error_fro());
        worst_determinant = worst_determinant.max(certificate.determinant_error());
        assert!(certificate.reconstruction_error_rad() < ENDPOINT_GATE);

        // The direct dense commutator and the lowered word agree.
        let commutator_gap = certificate
            .endpoint()
            .frobenius_distance(certificate.dense_commutator());
        worst_commutator = worst_commutator.max(commutator_gap);
        assert!(
            commutator_gap <= 1.0e-13,
            "{name} trial {trial}: dense commutator and lowered word differ by {commutator_gap:e}"
        );
        // And that commutator really is A B A^-1 B^-1 of the reported macros.
        let a = certificate.macro_a();
        let b = certificate.macro_b();
        let by_hand = a
            .multiply(b)
            .multiply(&a.transpose())
            .multiply(&b.transpose());
        assert!(certificate.dense_commutator().frobenius_distance(&by_hand) <= 1.0e-15);

        // Closure: a bound, never equality with zero (correction 3).
        for (_, closure) in certificate.edge_closure() {
            worst_closure = worst_closure.max(closure.abs());
            assert!(
                closure.abs() <= CLOSURE_GATE,
                "{name} trial {trial}: closure {closure:e}"
            );
        }

        // Thermodynamic timing of the twelve-gate word.
        let schedule = certificate.schedule();
        assert_eq!(schedule.segment_durations().len(), CLOSED_LOOP_GATE_COUNT);
        assert!(
            (schedule.segment_durations().iter().sum::<f64>() - duration).abs() <= 1.0e-12,
            "{name} trial {trial}: segments do not sum to the duration"
        );
        assert!(schedule.thermodynamic_length() > 0.0);
        let realized = schedule.realized_quadratic_cost();
        let predicted = schedule.minimum_quadratic_cost();
        assert!(
            (realized - predicted).abs() <= 1.0e-9 * predicted.max(1.0),
            "{name} trial {trial}: closed form {predicted} vs realized {realized}"
        );
        worst_speed = worst_speed.max(schedule.constant_speed_residual());
        assert!(
            schedule.constant_speed_residual()
                <= 1.0e-12 * (schedule.thermodynamic_length() / duration).max(1.0),
            "{name} trial {trial}: speed residual {:e}",
            schedule.constant_speed_residual()
        );

        match certificate.conjugator_branch() {
            ConjugatorBranch::Aligned => branches[0] += 1,
            ConjugatorBranch::Antipodal => branches[1] += 1,
            ConjugatorBranch::General => branches[2] += 1,
        }
    }

    assert_eq!(compiled, RANDOM_TARGETS);
    // Haar targets essentially never land on the degenerate branches.
    assert_eq!(branches[2], RANDOM_TARGETS);
    println!(
        "closed-loop gate: {compiled} targets; worst endpoint {worst_endpoint:e} \
         ({worst_endpoint_case}), worst endpoint-rad {worst_endpoint_rad:e}, \
         worst commutator gap {worst_commutator:e}, worst closure {worst_closure:e}, \
         worst orthogonality {worst_orthogonality:e}, worst |det-1| {worst_determinant:e}, \
         worst speed residual {worst_speed:e}"
    );
}

/// Identity in, empty word out — and nothing downstream trips over it.
#[test]
fn the_identity_target_compiles_to_the_empty_word_on_every_graph() {
    let mut structure = SplitMix64::new(0xf00d);
    for (name, graph) in graphs(&mut structure) {
        let target = RotationTarget::identity(3).unwrap();
        let certificate = compile_exact_closed_loop_so3(&target, &graph, 1.5).unwrap();
        assert_eq!(certificate.gate_count(), 0, "{name}");
        assert!(certificate.program().factors().is_empty(), "{name}");
        assert!(certificate.macro_certificates().is_none(), "{name}");
        assert_eq!(certificate.target_angle_rad(), 0.0, "{name}");
        assert_eq!(certificate.balanced_generator_angle_rad(), 0.0, "{name}");
        assert!(certificate.edge_closure().is_empty(), "{name}");
        assert!(certificate.max_edge_closure() <= CLOSURE_GATE, "{name}");
        assert_eq!(certificate.schedule().thermodynamic_length(), 0.0, "{name}");
        assert_eq!(certificate.schedule().duration(), 1.5, "{name}");
        assert!(
            certificate.schedule().executable_indices().is_empty(),
            "{name}"
        );
        assert!(
            certificate
                .endpoint()
                .frobenius_distance(&Matrix::identity(3))
                <= 0.0,
            "{name}"
        );

        // The receipt of an empty word is still a receipt, and still replays.
        let receipt = ClosedLoopReceipt::new(&graph, &target, &certificate, None);
        assert_eq!(receipt.gate_count, 0);
        assert_eq!(receipt.expected_gate_count, 0);
        assert_eq!(receipt.max_edge_closure, 0.0);
        let parsed = ClosedLoopReceipt::from_json(&receipt.to_json().unwrap()).unwrap();
        let report = replay_closed_loop(&parsed, &graph, &target).unwrap();
        assert_eq!(report.max_angle_difference, 0.0);
    }
}

/// The near-identity and near-pi corpus the spec asks for, over the whole
/// authored family. These are the two ends where an `acos((tr-1)/2)` angle
/// extraction would lose half its digits.
#[test]
fn near_identity_and_near_pi_targets_compile_closed_and_exact() {
    let mut structure = SplitMix64::new(0xbead);
    let axes: [[f64; 3]; 4] = [
        [0.37, -0.82, 0.44],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
    ];
    let angles: [f64; 10] = [
        1.0e-13,
        1.0e-11,
        1.0e-9,
        1.0e-6,
        1.0e-3,
        PI - 1.0e-3,
        PI - 1.0e-6,
        PI - 1.0e-9,
        PI - 1.0e-11,
        PI,
    ];
    let mut worst = 0.0_f64;
    let mut worst_case = String::new();
    let mut worst_closure = 0.0_f64;
    let mut checked = 0usize;

    for (name, graph) in graphs(&mut structure) {
        for axis in axes {
            for angle in angles {
                let target = RotationTarget::new(axis_angle(axis, angle), 1.0e-9).unwrap();
                let certificate = compile_exact_closed_loop_so3(&target, &graph, 1.0)
                    .unwrap_or_else(|error| panic!("{name} at angle {angle}: {error}"));
                checked += 1;
                assert_eq!(
                    certificate.gate_count(),
                    CLOSED_LOOP_GATE_COUNT,
                    "{name} at angle {angle}"
                );
                let error = certificate.reconstruction_error_fro();
                assert!(
                    error < ENDPOINT_GATE,
                    "{name} at angle {angle}, axis {axis:?}: endpoint error {error:e}"
                );
                if error > worst {
                    worst = error;
                    worst_case = format!("{name} angle {angle} axis {axis:?}");
                }
                // The recovered principal angle is the requested one.
                assert!(
                    (certificate.target_angle_rad() - angle).abs() <= 1.0e-8 * angle.max(1.0e-8),
                    "{name}: angle {angle} recovered as {}",
                    certificate.target_angle_rad()
                );
                let closure = certificate.max_edge_closure();
                worst_closure = worst_closure.max(closure);
                assert!(closure <= CLOSURE_GATE, "{name} at angle {angle}");
                let gap = certificate
                    .endpoint()
                    .frobenius_distance(certificate.dense_commutator());
                assert!(
                    gap <= 1.0e-13,
                    "{name} at angle {angle}: commutator gap {gap:e}"
                );
            }
        }
    }
    assert_eq!(checked, 3 * axes.len() * angles.len());
    println!(
        "near-identity / near-pi: {checked} targets; worst endpoint {worst:e} ({worst_case}), \
         worst closure {worst_closure:e}"
    );
}

/// Phase 1 rule 4 carries over unchanged: the work field is absent unless a
/// caller actually pushed the lowered program through the `mus-watsn` ledger.
#[test]
fn the_work_field_is_none_or_ledger_measured() {
    let mut structure = SplitMix64::new(0x5eed);
    let (_, graph) = graphs(&mut structure).remove(0);
    let target = RotationTarget::new(axis_angle([0.2, 0.5, -0.9], 1.7), 1.0e-9).unwrap();
    let certificate = compile_exact_closed_loop_so3(&target, &graph, 2.0).unwrap();

    let without = ClosedLoopReceipt::new(&graph, &target, &certificate, None);
    assert!(without.work_residual.is_none());
    assert!(without
        .to_json()
        .unwrap()
        .contains("\"work_residual\": null"));

    let measurement = measure_control_work(certificate.program(), &basis_probes(3)).unwrap();
    let with = ClosedLoopReceipt::new(&graph, &target, &certificate, Some(&measurement));
    let work = with.work_residual.clone().unwrap();
    assert_eq!(work.metric, EUCLIDEAN_NORMALIZED_METRIC);
    assert_eq!(work.probe_states, 3);
    assert_eq!(work.frames, (CLOSED_LOOP_GATE_COUNT * 3) as u64);
    // The model's prediction is declared, not measured; the discrepancy is.
    assert_eq!(work.declared_control_work, 0.0);
    assert!(work.max_abs_residual <= 1.0e-14);
    assert!(work.global_residual.abs() <= 1.0e-13);

    // And the receipt round-trips and replays with the measurement attached.
    let parsed = ClosedLoopReceipt::from_json(&with.to_json().unwrap()).unwrap();
    assert_eq!(parsed, with);
    let report = replay_closed_loop(&parsed, &graph, &target).unwrap();
    assert_eq!(report.max_angle_difference, 0.0);
    assert_eq!(report.endpoint_difference_fro, 0.0);
    assert!(report.reconstruction_error_fro < ENDPOINT_GATE);
}

/// The compiler is deterministic: same inputs, same word, bit for bit.
#[test]
fn compilation_is_deterministic() {
    let mut structure = SplitMix64::new(0xd37);
    let (_, graph) = graphs(&mut structure).remove(2);
    let mut normals = Pcg64::seeded(0xd37);
    for _ in 0..64 {
        let target = RotationTarget::new(haar_special_orthogonal(3, &mut normals), 1.0e-9).unwrap();
        let first = compile_exact_closed_loop_so3(&target, &graph, 1.25).unwrap();
        let second = compile_exact_closed_loop_so3(&target, &graph, 1.25).unwrap();
        assert_eq!(first.gates(), second.gates());
        assert_eq!(first.endpoint(), second.endpoint());
        assert_eq!(first.conjugator(), second.conjugator());
        let left = ClosedLoopReceipt::new(&graph, &target, &first, None);
        let right = ClosedLoopReceipt::new(&graph, &target, &second, None);
        assert_eq!(left.to_json().unwrap(), right.to_json().unwrap());
    }
}

/// Requests this phase does not serve are typed, not approximated.
#[test]
fn out_of_scope_requests_are_typed() {
    let mut structure = SplitMix64::new(0xbad);
    let (_, graph) = graphs(&mut structure).remove(0);
    let target = RotationTarget::new(axis_angle([0.0, 1.0, 0.0], 1.0), 1.0e-9).unwrap();

    // SO(n > 3): Gotô guarantees existence, this phase does not construct it.
    let four = ControlGraph::from_coordinate_edges(4, &[(0, 1), (1, 2), (2, 3)]).unwrap();
    let error = compile_exact_closed_loop_so3(&RotationTarget::identity(4).unwrap(), &four, 1.0)
        .unwrap_err();
    assert_eq!(error, GestureError::ClosedLoopDimension { dimension: 4 });
    assert_eq!(error.class(), ErrorClass::Unsupported);

    // A disconnected three-mode graph cannot generate SO(3) at all.
    let split = ControlGraph::from_coordinate_edges(3, &[(1, 2)]).unwrap();
    let error = compile_exact_closed_loop_so3(&target, &split, 1.0).unwrap_err();
    assert!(matches!(error, GestureError::DisconnectedGraph { .. }));
    assert_eq!(error.class(), ErrorClass::Unreachable);

    // Reflections never become a target in the first place.
    let mut reflection = Matrix::identity(3);
    reflection.set(0, 0, -1.0);
    let error = RotationTarget::new(reflection, 1.0e-9).unwrap_err();
    assert_eq!(error.class(), ErrorClass::Unreachable);

    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            compile_exact_closed_loop_so3(&target, &graph, bad).unwrap_err(),
            GestureError::InvalidDuration { .. }
        ));
    }
}
