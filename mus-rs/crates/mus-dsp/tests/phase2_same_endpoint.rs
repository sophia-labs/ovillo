//! Ariadne Phase 2 §C — the laws the same-endpoint experiment rests on.
//!
//! `examples/phase2_same_endpoint.rs` renders the stimulus set and writes the
//! report. This file locks the claims that report makes, on short renders, so
//! that a change to either the compiler or the seam that would invalidate the
//! experiment fails the workspace gate rather than silently producing a
//! different paper.
//!
//! Everything here is constructed independently of the example — the words
//! are built from `mus-gesture`'s public API in this file, and the expected
//! operators are composed here — so agreement between the two is evidence,
//! not shared code.

use std::collections::BTreeMap;

use mus_dsp::pluck::{GestureFrame, GestureTrace, StringNetworkVoice};
use mus_gesture::{
    compile_exact_closed_loop_so3, thermodynamic_schedule, ClosedLoopCertificate, ControlGraph,
    FrictionMetric, GivensGate, Matrix, RotationTarget, DEFAULT_TARGET_TOLERANCE,
};

/// Short, but not quiet: every path-dependent mechanism in the Weave is live.
fn patch() -> BTreeMap<String, String> {
    [
        ("synth", "weave"),
        ("sus", "0.9"),
        ("courses", "6"),
        ("couple", "0.19"),
        ("chirality", "0.4"),
        ("curvature", "0.5"),
        ("body", "0.4"),
        ("orbit", "0.5"),
        ("orbit_depth", "0.7"),
    ]
    .iter()
    .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
    .collect()
}

const NOTES: [f64; 3] = [110.0, 164.813_78, 220.0];
const SLOT_S: f64 = 0.6;
const STRIDE: usize = 256;
const COURSES: usize = 6;
const SR_F64: f64 = 48_000.0;

const WINDOW_START_S: f64 = 0.08;
const WINDOW_DURATION_S: f64 = 0.40;

/// The bound every closure and endpoint law is asserted against. Never
/// `== 0.0`.
const EXACT_TOLERANCE: f64 = 1e-12;

fn graph() -> ControlGraph {
    ControlGraph::from_coordinate_edges(3, &[(0, 1), (1, 2)]).expect("a two-edge path on 3 modes")
}

/// A generic nonidentity SO(3) target: three plane rotations, no coordinate
/// axis, angle far from 0 and pi.
fn target() -> RotationTarget {
    let mut matrix = Matrix::identity(3);
    matrix.rotate_rows(0, 1, 0.62);
    matrix.rotate_rows(1, 2, -0.41);
    matrix.rotate_rows(0, 2, 0.53);
    RotationTarget::new(matrix, DEFAULT_TARGET_TOLERANCE).expect("a product of rotations is SO(3)")
}

fn schedule_word(gates: &[GivensGate], graph: &ControlGraph, duration_s: f64) -> Vec<GestureFrame> {
    let metric = FrictionMetric::resolve(gates, graph).expect("gates run on authored edges");
    let schedule =
        thermodynamic_schedule(gates, &metric, duration_s).expect("a positive finite duration");
    let mut elapsed = 0.0;
    gates
        .iter()
        .zip(schedule.segment_durations())
        .map(|(gate, segment)| {
            elapsed += segment;
            GestureFrame {
                at_sample: ((WINDOW_START_S + elapsed) * SR_F64).round() as usize,
                course_a: gate.left,
                course_b: gate.right,
                theta: gate.theta,
            }
        })
        .collect()
}

/// `B A B^-1 A^-1` from the compiler's `A B A^-1 B^-1`: the same gates, blocks
/// swapped.
fn swap_blocks(certificate: &ClosedLoopCertificate) -> Vec<GivensGate> {
    let (macro_a, macro_b) = certificate
        .macro_certificates()
        .expect("a nonidentity target compiles two macros");
    let mut swapped: Vec<GivensGate> = macro_a.gates().iter().rev().map(|g| g.inverse()).collect();
    swapped.extend(macro_b.gates().iter().rev().map(|g| g.inverse()));
    swapped.extend_from_slice(macro_a.gates());
    swapped.extend_from_slice(macro_b.gates());
    swapped
}

fn render(word: &[GestureFrame]) -> (Vec<f32>, GestureTrace) {
    let (audio, _, trace) = StringNetworkVoice::weave(&patch(), &NOTES, SLOT_S, 1.0)
        .render_traced_gestured(STRIDE, &[], word)
        .expect("every word here names courses this network has");
    (audio, trace)
}

/// The dense operator an executed word realizes, in the seam's front-to-back
/// convention, over the whole network's wave space.
fn word_operator(word: &[GestureFrame]) -> Matrix {
    let mut operator = Matrix::identity(COURSES);
    for frame in word {
        operator.rotate_rows(frame.course_a, frame.course_b, frame.theta);
    }
    operator
}

fn embed(small: &Matrix) -> Matrix {
    let mut big = Matrix::identity(COURSES);
    for row in 0..3 {
        for column in 0..3 {
            big.set(row, column, small.get(row, column));
        }
    }
    big
}

/// Worst per-course-pair integrated control angle over the EXECUTED gates.
fn worst_executed_closure(trace: &GestureTrace) -> f64 {
    let mut closure: BTreeMap<(usize, usize), f64> = BTreeMap::new();
    for gate in &trace.applied {
        let (low, high, sign) = if gate.course_a <= gate.course_b {
            (gate.course_a, gate.course_b, 1.0)
        } else {
            (gate.course_b, gate.course_a, -1.0)
        };
        *closure.entry((low, high)).or_insert(0.0) += sign * gate.theta;
    }
    closure.values().fold(0.0_f64, |w, v| w.max(v.abs()))
}

fn rms(audio: &[f32]) -> f64 {
    (audio
        .iter()
        .map(|&s| f64::from(s) * f64::from(s))
        .sum::<f64>()
        / audio.len() as f64)
        .sqrt()
}

fn relative_distance(left: &[f32], right: &[f32]) -> f64 {
    let norm = |audio: &[f32]| {
        audio
            .iter()
            .map(|&s| f64::from(s) * f64::from(s))
            .sum::<f64>()
            .sqrt()
    };
    let difference = left
        .iter()
        .zip(right)
        .map(|(&a, &b)| {
            let d = f64::from(a) - f64::from(b);
            d * d
        })
        .sum::<f64>()
        .sqrt();
    difference / (0.5 * (norm(left) + norm(right)))
}

/// Phase 2 acceptance gate, first and third lines: *per-edge schedule closure
/// is exact in the compiled representation*, and *direct dense target, Python
/// lowering, Rust operator replay, and live Weave state agree*. The live half
/// is the one this file owns: the word the production voice actually ran
/// composes to the compiled target, and every edge it touched integrates back
/// to zero.
#[test]
fn a_compiled_closed_word_runs_in_the_production_voice_and_closes() {
    let graph = graph();
    let target = target();
    let certificate = compile_exact_closed_loop_so3(&target, &graph, WINDOW_DURATION_S)
        .expect("a connected three-mode graph compiles every SO(3) target");
    assert_eq!(certificate.gate_count(), 12, "twelve authored-edge gates");

    let word = schedule_word(certificate.gates(), &graph, WINDOW_DURATION_S);
    let (_, trace) = render(&word);
    assert_eq!(trace.applied.len(), 12, "every gate landed inside the note");

    let realized = word_operator(&word);
    let expected = embed(target.matrix());
    let endpoint = realized.frobenius_distance(&expected);
    assert!(
        endpoint <= EXACT_TOLERANCE,
        "the executed word missed the compiled target by {endpoint:e}"
    );

    let closure = worst_executed_closure(&trace);
    assert!(
        closure <= EXACT_TOLERANCE,
        "an authored edge did not close: {closure:e}"
    );
    assert!(
        certificate.max_edge_closure() <= EXACT_TOLERANCE,
        "the compiler's own closure exceeded the bound"
    );

    // The localized factor-stage-work gate: every gate is an exact rotation
    // on the pair it touches.
    let drift = trace.worst_pair_energy_drift();
    assert!(
        drift <= 1e-13,
        "a gate moved its pair's energy by a relative {drift:e}"
    );
}

/// Phase 2 acceptance gate: *commuting controls produce the null result*.
/// Disjoint course pairs touch disjoint memory, so this is exact, not close —
/// `assert_eq!` on raw `f32`, no tolerance.
#[test]
fn commuting_controls_produce_the_null_result_byte_identically() {
    let angle = 0.9838;
    let open = ((WINDOW_START_S + 0.05) * SR_F64).round() as usize;
    let close = ((WINDOW_START_S + WINDOW_DURATION_S) * SR_F64).round() as usize;
    let gate = |at_sample: usize, pair: (usize, usize), theta: f64| GestureFrame {
        at_sample,
        course_a: pair.0,
        course_b: pair.1,
        theta,
    };
    let (p, q) = ((0, 1), (3, 4));
    let pq = [
        gate(open, p, angle),
        gate(open, q, -angle),
        gate(close, p, -angle),
        gate(close, q, angle),
    ];
    let qp = [
        gate(open, q, -angle),
        gate(open, p, angle),
        gate(close, q, angle),
        gate(close, p, -angle),
    ];

    let (audio_pq, trace_pq) = render(&pq);
    let (audio_qp, _) = render(&qp);
    assert_eq!(
        audio_pq, audio_qp,
        "commuting controls must render byte-identically in either order"
    );

    // And the control loop really is closed and really did something: the
    // word is not a no-op that would make the law vacuous.
    assert!(worst_executed_closure(&trace_pq) <= EXACT_TOLERANCE);
    let (silent, _) = render(&[]);
    assert_ne!(
        audio_pq, silent,
        "the commuting word must be audible, or the null result is vacuous"
    );
}

/// Phase 2 acceptance gate: *loop inversion produces inverse monodromy*, in
/// its exact state-level form. A word and its reversal-negation applied with
/// no dynamics between them return the network's wave vector to where it
/// started — the trace endpoint returns.
#[test]
fn a_word_and_its_reversal_negation_return_the_trace_endpoint() {
    let graph = graph();
    let certificate = compile_exact_closed_loop_so3(&target(), &graph, WINDOW_DURATION_S)
        .expect("a connected three-mode graph compiles every SO(3) target");
    let at_sample = (WINDOW_START_S * SR_F64).round() as usize;
    let mut word: Vec<GestureFrame> = certificate
        .gates()
        .iter()
        .map(|gate| GestureFrame {
            at_sample,
            course_a: gate.left,
            course_b: gate.right,
            theta: gate.theta,
        })
        .collect();
    word.extend(
        word.clone()
            .iter()
            .rev()
            .map(|frame| GestureFrame {
                theta: -frame.theta,
                ..*frame
            })
            .collect::<Vec<_>>(),
    );
    assert_eq!(word.len(), 24);

    let (_, trace) = render(&word);
    let before = trace.waves_before(0).expect("state before the word");
    let after = trace.waves_after(23).expect("state after the word");
    let norm = before.iter().map(|v| v * v).sum::<f64>().sqrt();
    assert!(norm > 1e-6, "the courses must be ringing, norm {norm}");
    let distance = before
        .iter()
        .zip(after)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt();
    assert!(
        distance <= EXACT_TOLERANCE * norm.max(1.0),
        "the trace endpoint must return, off by {distance:e}"
    );

    // The operator statement, which holds whether or not the gates are
    // coincident: the composed word is the identity.
    let residual = word_operator(&word).frobenius_distance(&Matrix::identity(COURSES));
    assert!(
        residual <= EXACT_TOLERANCE,
        "the composed inverse loop is not the identity, off by {residual:e}"
    );
}

/// The experiment's headline claim, in its testable form: two words with the
/// SAME closed control endpoint render differently, and not because one is
/// louder.
///
/// Two pairs are checked. `A,B,−A,−B` against `B,A,−B,−A` is the round-2
/// contrast with closed per-edge controls — the same twelve gates in a
/// different order. `null` against the spread inverse loop is the stronger
/// one: a twenty-four gate word whose net operator is exactly the identity is
/// still not the same as no word, because the network evolves between gates.
#[test]
fn same_endpoint_words_render_differently_at_matched_level() {
    let graph = graph();
    let certificate = compile_exact_closed_loop_so3(&target(), &graph, WINDOW_DURATION_S)
        .expect("a connected three-mode graph compiles every SO(3) target");

    let forward = schedule_word(certificate.gates(), &graph, WINDOW_DURATION_S);
    let reversed = schedule_word(&swap_blocks(&certificate), &graph, WINDOW_DURATION_S);

    // The two orders are the same twelve gates: same angles, same edges, so
    // the same thermodynamic length. A difference cannot be a difference in
    // effort.
    let sorted = |word: &[GestureFrame]| {
        let mut angles: Vec<f64> = word.iter().map(|frame| frame.theta.abs()).collect();
        angles.sort_by(f64::total_cmp);
        angles
    };
    assert_eq!(sorted(&forward), sorted(&reversed));

    let (audio_ab, _) = render(&forward);
    let (audio_ba, _) = render(&reversed);
    assert_ne!(audio_ab, audio_ba, "the order of a closed word must matter");
    let order_effect = relative_distance(&audio_ab, &audio_ba);
    assert!(
        order_effect > 1e-3,
        "the order effect must be signal, not roundoff: rel_dist {order_effect:e}"
    );
    let level = 20.0 * (rms(&audio_ab) / rms(&audio_ba)).log10();
    assert!(
        level.abs() < 1.0,
        "the two orders differ by {level:.4} dB — a loudness artifact, not a path effect"
    );

    // The block orders realize inverse group elements, so their endpoints are
    // R and R^T: same word, same effort, conjugate endpoints.
    let target_matrix = embed(target().matrix());
    assert!(word_operator(&forward).frobenius_distance(&target_matrix) <= EXACT_TOLERANCE);
    assert!(
        word_operator(&reversed).frobenius_distance(&embed(&target().matrix().transpose()))
            <= EXACT_TOLERANCE
    );

    // The identity-endpoint pair.
    let mut inverse_gates: Vec<GivensGate> = certificate.gates().to_vec();
    inverse_gates.extend(certificate.gates().iter().rev().map(|g| g.inverse()));
    let spread_inverse = schedule_word(&inverse_gates, &graph, WINDOW_DURATION_S);
    let (audio_inverse, trace_inverse) = render(&spread_inverse);
    let (silent, _) = render(&[]);
    assert!(
        word_operator(&spread_inverse).frobenius_distance(&Matrix::identity(COURSES))
            <= EXACT_TOLERANCE,
        "the spread inverse loop must still compose to the identity"
    );
    assert!(worst_executed_closure(&trace_inverse) <= EXACT_TOLERANCE);
    assert_ne!(
        audio_inverse, silent,
        "a 24-gate identity-endpoint word is not the same as no word"
    );
    let identity_effect = relative_distance(&audio_inverse, &silent);
    assert!(
        identity_effect > 1e-3,
        "the identity-endpoint effect must be signal, not roundoff: rel_dist \
         {identity_effect:e}"
    );
    let identity_level = 20.0 * (rms(&audio_inverse) / rms(&silent)).log10();
    assert!(
        identity_level.abs() < 1.0,
        "the identity-endpoint pair differs by {identity_level:.4} dB"
    );
}
