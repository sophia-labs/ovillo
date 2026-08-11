//! Independent replay of the exact closed-control `SO(3)` compiler against the
//! Python float64 oracle.
//!
//! Consensus §5.3: "Rust must replay committed or generated fixtures rather
//! than merely sharing code paths." The Phase 2 spec adds that the closed-loop
//! fixtures must be generated fresh from
//! `reference/ariadne_gesture/closed_loop.py` rather than parsed out of the
//! packet's committed `results.json`, which carries aggregates only.
//!
//! `fixtures/closed_loop_oracle.json` is written by
//! `fixtures/gen_closed_loop_oracle.py`. Rust then does four independent
//! things with it:
//!
//! * **executes the Python word** through `mus_watsn::OperatorProgram` and
//!   checks it reaches the Python target and closes on every authored edge;
//! * **recompiles** the target with [`compile_exact_closed_loop_so3`] and
//!   checks the geometry the two compilers chose — angle, axis, balanced
//!   generator angle, conjugator, branch — agrees;
//! * **compares the words gate for gate** wherever the shared construction is
//!   well conditioned (see `AXIS_CONDITIONED_ANGLE` below);
//! * **re-times** the word and checks the thermodynamic receipt agrees.

use std::path::Path;

use mus_gesture::{
    compile_exact_closed_loop_so3, edge_closure, endpoint_operator, ConjugatorBranch, ControlGraph,
    EdgeControl, EdgeId, Matrix, ModeId, RotationTarget, CLOSED_LOOP_GATE_COUNT,
};
use mus_watsn::OperatorProgram;
use serde::Deserialize;

/// Below this principal angle the *shared* construction is ill conditioned in
/// a way neither implementation can fix, so the two words are allowed to
/// differ while both remaining exact.
///
/// The local commutator `C0(t) = I + O(phi)` is formed by dense multiplication
/// of four rotations, so its off-identity part is computed with absolute error
/// `~eps` and *relative* error `~eps/phi`. Its axis therefore carries relative
/// error `~eps/phi`, and NumPy's BLAS matmul and this crate's row updates do
/// not round identically. At `phi = 1e-13` that is a part in `10^3`: two
/// perfectly valid conjugators, two different twelve-gate words, both landing
/// on the target to `~1e-16` Frobenius because the endpoint error scales as
/// `phi * (axis error)`. The endpoint gate below is asserted on every case
/// including these; only the gate-for-gate comparison is conditioned.
const AXIS_CONDITIONED_ANGLE: f64 = 1.0e-4;

#[derive(Debug, Deserialize)]
struct OracleFixture {
    schema: String,
    case_count: usize,
    duration: f64,
    cases: Vec<OracleCase>,
}

#[derive(Debug, Deserialize)]
struct OracleCase {
    name: String,
    graph: String,
    graph_edges: Vec<[usize; 2]>,
    edge_friction: Vec<(usize, usize, f64)>,
    tree_edges: Vec<[usize; 2]>,
    leaf_order: Vec<usize>,
    target: Vec<Vec<f64>>,
    target_angle_rad: f64,
    target_axis: [f64; 3],
    balanced_generator_angle_rad: f64,
    local_commutator_axis: [f64; 3],
    conjugator: Vec<Vec<f64>>,
    conjugator_branch: String,
    factor_count: usize,
    schedule: Vec<OracleGate>,
    reconstruction_error_fro: f64,
    reconstruction_error_rad: f64,
    orthogonality_error_fro: f64,
    max_edge_control_closure: f64,
    duration: f64,
    unit_thermodynamic_length: f64,
    thermodynamic_length: f64,
    minimum_quadratic_cost: f64,
    segment_durations: Vec<f64>,
    constant_speed_residual: f64,
}

#[derive(Debug, Deserialize)]
struct OracleGate {
    i: usize,
    j: usize,
    theta: f64,
}

fn load_fixture() -> OracleFixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/closed_loop_oracle.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} is not valid JSON: {error}", path.display()))
}

fn graph_of(case: &OracleCase) -> ControlGraph {
    assert_eq!(
        case.edge_friction.len(),
        case.graph_edges.len(),
        "{}: friction and edge lists disagree",
        case.name
    );
    for (authored, &[left, right]) in case.edge_friction.iter().zip(case.graph_edges.iter()) {
        assert_eq!((authored.0, authored.1), (left, right));
    }
    let modes: Vec<ModeId> = (0..3u32).map(ModeId).collect();
    let edges: Vec<EdgeControl> = case
        .edge_friction
        .iter()
        .enumerate()
        .map(|(index, &(left, right, friction))| {
            EdgeControl::with_friction(
                EdgeId(index as u32),
                ModeId(left as u32),
                ModeId(right as u32),
                friction,
            )
        })
        .collect();
    ControlGraph::new(modes, edges).unwrap()
}

fn relative(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs().max(1.0)
}

#[test]
fn the_closed_loop_fixture_covers_the_declared_corpus() {
    let fixture = load_fixture();
    assert_eq!(fixture.schema, "mus-gesture/closed-loop-oracle/1");
    assert_eq!(fixture.case_count, fixture.cases.len());
    assert!(fixture.case_count >= 100);
    assert!(fixture.duration > 0.0);

    let mut graphs: Vec<&str> = fixture
        .cases
        .iter()
        .map(|case| case.graph.as_str())
        .collect();
    graphs.sort_unstable();
    graphs.dedup();
    assert_eq!(graphs, vec!["path_012", "path_021", "triangle"]);

    let mut branches: Vec<&str> = fixture
        .cases
        .iter()
        .map(|case| case.conjugator_branch.as_str())
        .collect();
    branches.sort_unstable();
    branches.dedup();
    assert_eq!(branches, vec!["aligned", "antipodal", "general"]);

    // The corpus the spec names: an identity, near-identity, and near-pi.
    assert!(fixture.cases.iter().any(|case| case.factor_count == 0));
    assert!(fixture
        .cases
        .iter()
        .any(|case| case.factor_count > 0 && case.target_angle_rad < 1.0e-12));
    assert!(fixture
        .cases
        .iter()
        .any(|case| (std::f64::consts::PI - case.target_angle_rad).abs() < 1.0e-12));
    for case in &fixture.cases {
        assert!(
            case.factor_count == 0 || case.factor_count == CLOSED_LOOP_GATE_COUNT,
            "{}: {} factors",
            case.name,
            case.factor_count
        );
        assert_eq!(case.schedule.len(), case.factor_count);
    }
}

/// Rust executes the Python word, lands on the Python target, and finds the
/// same closure the oracle reported.
#[test]
fn rust_executes_the_python_closed_word_onto_the_python_target() {
    let fixture = load_fixture();
    let mut worst = 0.0_f64;
    let mut worst_case = String::new();
    let mut worst_closure = 0.0_f64;
    for case in &fixture.cases {
        let graph = graph_of(case);
        let target = Matrix::from_rows(case.target.clone()).unwrap();
        let program = OperatorProgram::new(
            3,
            case.schedule
                .iter()
                .map(|gate| mus_watsn::PlaneRotation {
                    left: gate.i,
                    right: gate.j,
                    angle: gate.theta,
                })
                .collect(),
        )
        .unwrap();
        let endpoint = endpoint_operator(&program).unwrap();
        let error = endpoint.frobenius_distance(&target);
        assert!(
            error < 1.0e-11,
            "{}: executing the oracle word missed by {error:e}",
            case.name
        );
        if error > worst {
            worst = error;
            worst_case = case.name.clone();
        }
        assert!(case.reconstruction_error_fro < 1.0e-11);
        assert!(case.reconstruction_error_rad < 1.0e-11);
        assert!(case.orthogonality_error_fro < 1.0e-11);

        // Closure of the oracle's own word, recomputed on the authored edges.
        let gates: Vec<mus_gesture::GivensGate> = case
            .schedule
            .iter()
            .map(|gate| mus_gesture::GivensGate {
                edge: graph.edge_between(gate.i, gate.j).unwrap().id,
                left: gate.i,
                right: gate.j,
                theta: gate.theta,
            })
            .collect();
        for (_, closure) in edge_closure(&gates) {
            worst_closure = worst_closure.max(closure.abs());
            // Correction 3: a bound, never equality with zero — even though
            // the oracle's own `max_edge_control_closure` is exactly 0.0 here.
            assert!(
                closure.abs() <= 1.0e-12,
                "{}: oracle word closure {closure:e}",
                case.name
            );
        }
        assert!(case.max_edge_control_closure <= 1.0e-12);
    }
    println!(
        "python closed-word replay: worst endpoint {worst:e} ({worst_case}); \
         worst recomputed closure {worst_closure:e}"
    );
}

/// Rust recompiles each target and agrees with the oracle on the geometry it
/// chose, and — where that geometry is well conditioned — on the word itself.
#[test]
fn rust_recompiles_the_oracle_closed_word() {
    let fixture = load_fixture();
    let mut worst_angle_geometry = 0.0_f64;
    let mut worst_axis = 0.0_f64;
    let mut worst_generator = 0.0_f64;
    let mut worst_conjugator = 0.0_f64;
    let mut worst_gate_angle = 0.0_f64;
    let mut worst_gate_case = String::new();
    let mut compared_gate_for_gate = 0usize;
    let mut endpoint_only = 0usize;

    for case in &fixture.cases {
        let graph = graph_of(case);
        let target = RotationTarget::from_rows(case.target.clone()).unwrap();
        let certificate = compile_exact_closed_loop_so3(&target, &graph, case.duration)
            .unwrap_or_else(|error| {
                panic!("{}: {error}", case.name);
            });

        // The plan the oracle used is the plan this crate reaches on its own.
        let tree: Vec<[usize; 2]> = certificate
            .tree()
            .edges()
            .iter()
            .map(|&(left, right)| [left, right])
            .collect();
        assert_eq!(tree, case.tree_edges, "{}: tree disagrees", case.name);
        assert_eq!(
            certificate.leaf_order().as_slice(),
            case.leaf_order.as_slice(),
            "{}: leaf order disagrees",
            case.name
        );

        assert_eq!(
            certificate.gate_count(),
            case.factor_count,
            "{}: gate count disagrees",
            case.name
        );
        assert!(
            certificate.reconstruction_error_fro() < 1.0e-11,
            "{}: endpoint error {:e}",
            case.name,
            certificate.reconstruction_error_fro()
        );
        assert!(certificate.max_edge_closure() <= 1.0e-12);

        // Geometry: angle and generator angle are well conditioned everywhere.
        worst_angle_geometry = worst_angle_geometry
            .max((certificate.target_angle_rad() - case.target_angle_rad).abs());
        worst_generator = worst_generator.max(
            (certificate.balanced_generator_angle_rad() - case.balanced_generator_angle_rad).abs(),
        );
        assert!(
            (certificate.target_angle_rad() - case.target_angle_rad).abs() <= 1.0e-12,
            "{}: principal angle disagrees",
            case.name
        );
        let axis_delta: f64 = (0..3)
            .map(|k| (certificate.target_axis()[k] - case.target_axis[k]).abs())
            .fold(0.0, f64::max);
        worst_axis = worst_axis.max(axis_delta);
        assert!(
            axis_delta <= 1.0e-12,
            "{}: target axis disagrees by {axis_delta:e}",
            case.name
        );

        if case.factor_count == 0 {
            assert_eq!(certificate.gate_count(), 0);
            continue;
        }

        let branch = match case.conjugator_branch.as_str() {
            "aligned" => ConjugatorBranch::Aligned,
            "antipodal" => ConjugatorBranch::Antipodal,
            "general" => ConjugatorBranch::General,
            other => panic!("{}: unknown branch {other}", case.name),
        };

        // The local commutator axis is the conditioning-sensitive quantity.
        let local_delta: f64 = (0..3)
            .map(|k| (certificate.local_commutator_axis()[k] - case.local_commutator_axis[k]).abs())
            .fold(0.0, f64::max);

        if case.target_angle_rad < AXIS_CONDITIONED_ANGLE {
            // Ill conditioned by construction: both words are exact, they need
            // not be the same word. The endpoint gate above already ran.
            endpoint_only += 1;
            continue;
        }

        compared_gate_for_gate += 1;
        assert_eq!(
            certificate.conjugator_branch(),
            branch,
            "{}: conjugator branch disagrees",
            case.name
        );
        assert!(
            local_delta <= 1.0e-9,
            "{}: local commutator axis disagrees by {local_delta:e}",
            case.name
        );
        let recorded_conjugator = Matrix::from_rows(case.conjugator.clone()).unwrap();
        let conjugator_delta = certificate
            .conjugator()
            .frobenius_distance(&recorded_conjugator);
        worst_conjugator = worst_conjugator.max(conjugator_delta);
        assert!(
            conjugator_delta <= 1.0e-11,
            "{}: conjugator disagrees by {conjugator_delta:e}",
            case.name
        );

        for (index, (mine, theirs)) in certificate.gates().iter().zip(&case.schedule).enumerate() {
            assert_eq!(
                (mine.left, mine.right),
                (theirs.i, theirs.j),
                "{} gate {index}: plane disagrees",
                case.name
            );
            assert!(
                graph.edge_between(mine.left, mine.right).is_some(),
                "{} gate {index}: not on an authored edge",
                case.name
            );
            let delta = (mine.theta - theirs.theta).abs();
            if delta > worst_gate_angle {
                worst_gate_angle = delta;
                worst_gate_case = format!("{} gate {index}", case.name);
            }
        }
    }

    println!(
        "closed-loop recompile: gate-for-gate {compared_gate_for_gate}, \
         endpoint-only {endpoint_only}; worst |dphi| {worst_angle_geometry:e}, \
         worst axis {worst_axis:e}, worst |dt| {worst_generator:e}, \
         worst conjugator {worst_conjugator:e}, \
         worst |dtheta| {worst_gate_angle:e} ({worst_gate_case})"
    );
    assert!(compared_gate_for_gate >= 100);
    assert!(endpoint_only >= 3, "the near-identity corpus is missing");
    // Observed worst over this corpus is ~2e-15: the two implementations are
    // not bit-identical and cannot be, since NumPy's BLAS matmul and this
    // crate's row updates round differently. The bound is set at 1e-11 rather
    // than at the observed worst because a case sitting exactly at
    // `AXIS_CONDITIONED_ANGLE` would admit a conditioning error of ~1e-12 by
    // the estimate above, and the corpus is allowed to grow toward it. It
    // still sits far below anything that could reorder or reshape the word.
    assert!(
        worst_gate_angle <= 1.0e-11,
        "worst closed-word angle disagreement {worst_gate_angle:e} at {worst_gate_case}"
    );
}

/// Rust re-times the closed word and matches the oracle's thermodynamic
/// receipt, including the unit-friction length the packet reports.
#[test]
fn rust_retimes_the_oracle_closed_word() {
    let fixture = load_fixture();
    let mut worst_length = 0.0_f64;
    let mut worst_cost = 0.0_f64;
    let mut worst_segment = 0.0_f64;
    let mut empty_words = 0usize;

    for case in &fixture.cases {
        let graph = graph_of(case);
        let target = RotationTarget::from_rows(case.target.clone()).unwrap();
        let certificate = compile_exact_closed_loop_so3(&target, &graph, case.duration).unwrap();
        let schedule = certificate.schedule();
        assert_eq!(schedule.duration(), case.duration);

        if case.factor_count == 0 {
            empty_words += 1;
            assert_eq!(schedule.thermodynamic_length(), 0.0);
            assert_eq!(schedule.minimum_quadratic_cost(), 0.0);
            assert!(schedule.executable_indices().is_empty());
            assert!(schedule.segment_durations().is_empty());
            assert!(case.segment_durations.is_empty());
            assert_eq!(case.thermodynamic_length, 0.0);
            continue;
        }

        if case.target_angle_rad < AXIS_CONDITIONED_ANGLE {
            // The two words are allowed to differ here, so their lengths are
            // too. Both are still exact; the endpoint gate covers that.
            continue;
        }

        // The unit-friction length the oracle's own certificate reports is the
        // plain sum of |theta| over the word.
        let unit_length: f64 = certificate
            .gates()
            .iter()
            .map(|gate| gate.theta.abs())
            .sum();
        assert!(
            relative(unit_length, case.unit_thermodynamic_length) <= 1.0e-11,
            "{}: unit length {unit_length} vs oracle {}",
            case.name,
            case.unit_thermodynamic_length
        );

        let length_delta = relative(schedule.thermodynamic_length(), case.thermodynamic_length);
        let cost_delta = relative(
            schedule.minimum_quadratic_cost(),
            case.minimum_quadratic_cost,
        );
        worst_length = worst_length.max(length_delta);
        worst_cost = worst_cost.max(cost_delta);
        assert!(
            length_delta <= 1.0e-11,
            "{}: thermodynamic length {} vs oracle {}",
            case.name,
            schedule.thermodynamic_length(),
            case.thermodynamic_length
        );
        assert!(
            cost_delta <= 1.0e-11,
            "{}: predicted cost disagrees",
            case.name
        );

        assert_eq!(
            schedule.segment_durations().len(),
            case.segment_durations.len()
        );
        for (mine, theirs) in schedule
            .segment_durations()
            .iter()
            .zip(case.segment_durations.iter())
        {
            let delta = (mine - theirs).abs() / case.duration;
            worst_segment = worst_segment.max(delta);
            assert!(
                delta <= 1.0e-11,
                "{}: segment duration disagrees",
                case.name
            );
        }
        assert!(case.constant_speed_residual <= 1.0e-12);
        assert!(schedule.constant_speed_residual() <= 1.0e-12);
    }
    assert!(empty_words >= 1, "no identity case in the fixture");
    println!(
        "closed-loop retime: worst relative length {worst_length:e}, \
         cost {worst_cost:e}, segment {worst_segment:e}"
    );
}
