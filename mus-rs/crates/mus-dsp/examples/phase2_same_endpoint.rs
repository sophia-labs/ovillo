//! Ariadne Phase 2 §C — the same-endpoint production experiment.
//!
//! The render the paper's limitation 5 has demanded since round 2. Round 2
//! (`.string-research/E15-REPORT.md`) drove two *scalar* controls — `couple`
//! and `chirality` — around A,B,−A,−B and B,A,−B,−A and measured an audible
//! difference. Its honest weakness: the per-edge control history was closed
//! only in the sense that the two knobs returned to their starting values.
//! Nothing certified that the network's own scattering angles integrated back
//! to zero, so the difference could not be attributed to the ordering of a
//! *closed* control loop.
//!
//! This harness removes that weakness. Every word below is compiled by
//! [`mus_gesture::compile_exact_closed_loop_so3`] (Phase 2 §A), so every
//! authored edge's integrated control angle returns to zero to float64
//! roundoff, and it is *executed* by the production Weave voice through the
//! gesture seam (Phase 2 §B) — the same wave variables the ring's own Givens
//! rotations act on, inside a played chord, with coupling, curvature, orbit
//! and body all live.
//!
//! Run: `cargo run -p mus-dsp --release --example phase2_same_endpoint`
//! Optional argument: the output directory (default
//! `<repo>/.string-research/phase2-stimuli`).
//!
//! # Terminology discipline (consensus rules 7, 8, 10)
//!
//! These are **closed-control monodromy renders**. The word "holonomy" is not
//! used for them: `ARIADNE-MATHEMATICS.md` §6 lists five conditions for a
//! Berry/Wilczek–Zee-style geometric claim and this experiment establishes
//! condition (1) — a control cycle with `λ_T = λ_0` — *exactly and per edge*,
//! which is strictly more than E15 established, and still only condition (1).
//! No modal subspace is identified or transported, so (2)–(5) remain open.
//! **No master limiter and no normalization** is applied anywhere here
//! (consensus rule 7): the loudness columns are reported so the reader can
//! check that the orders are level-matched, not achieved by making them so.
//! Listener validation remains future work; nothing below is a perceptual
//! claim.
//!
//! # The stimulus set
//!
//! | id | word | endpoint operator |
//! |---|---|---|
//! | `null` | empty | `I` |
//! | `commute-pq` | `P Q P⁻¹ Q⁻¹` on disjoint course pairs | `I` |
//! | `commute-qp` | `Q P Q⁻¹ P⁻¹`, same samples | `I` |
//! | `loop-ab` | compiled 12-gate `A B A⁻¹ B⁻¹` | `R` |
//! | `loop-ba` | the same twelve gates, blocks swapped | `Rᵀ` |
//! | `mirror` | 12 gates compiled for the axis-mirrored target | `Rᵀ` |
//! | `inverse` | `loop-ab` followed by its exact reversal-negation | `I` |
//!
//! Two same-endpoint pairs fall out of that table, and they are the point:
//!
//! * `(null, inverse)` — both close on the identity operator, one with no
//!   gates and one with twenty-four. The network evolves *between* the gates,
//!   so the audio differs while the control endpoint does not.
//! * `(loop-ba, mirror)` — two structurally different closed words with the
//!   same endpoint operator `Rᵀ`, compiled from different targets through
//!   different conjugators. Reported, not controlled: `R` and `Rᵀ` conjugate
//!   through opposite axes, so the two words do NOT share a thermodynamic
//!   length. `(loop-ab, loop-ba)` is the effort-matched contrast — literally
//!   the same twelve gates in a different order.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use mus_dsp::pluck::{GestureFrame, GestureTrace, StringNetworkVoice};
use mus_dsp::scene::write_wav_f32;
use mus_dsp::{SR, SR_F64};
use mus_gesture::{
    basis_probes, compile_exact_closed_loop_so3, measure_control_work, principal_angle_axis,
    thermodynamic_schedule, ClosedLoopCertificate, ClosedLoopReceipt, ControlGraph, FrictionMetric,
    GivensGate, Matrix, RotationTarget, DEFAULT_TARGET_TOLERANCE,
};

// ------------------------------------------------------------------ fixture

/// A representative, fully-live Weave patch: coupled, chiral, curved,
/// orbiting, with a body. Nothing is switched off to make the laws easier —
/// the same family E15 measured and `tests/gesture_seam.rs` proves the seam
/// laws on.
const PATCH: &[(&str, &str)] = &[
    ("synth", "weave"),
    ("sus", "2.4"),
    ("courses", "9"),
    ("couple", "0.19"),
    ("chirality", "0.4"),
    ("curvature", "0.5"),
    ("body", "0.4"),
    ("orbit", "0.5"),
    ("orbit_depth", "0.7"),
];

/// A played chord: A2, E3, A3. `detune` is 0, so course `k` is note `k` and
/// courses 0,1,2 are exactly the three played courses the SO(3) target acts
/// on. Courses 3..8 are the weave's virtual courses.
const NOTES: [f64; 3] = [110.0, 164.813_78, 220.0];

const SLOT_S: f64 = 2.4;
const STRIDE: usize = 256;
const COURSES: usize = 9;

/// The gesture window inside the sustain: the word starts after the attack
/// has settled and closes with ring-out left, so the consequence of the
/// closed loop is audible *after* the last gate as well as between them.
const WINDOW_START_S: f64 = 0.20;
const WINDOW_DURATION_S: f64 = 1.60;

/// The two courses each commuting generator acts on. Disjoint from each
/// other by construction — that is what makes them commute, and the pair
/// `(0,1)`/`(3,4)` keeps one generator inside the played chord and one
/// outside it, so neither order is trivially inaudible.
const COMMUTE_P: (usize, usize) = (0, 1);
const COMMUTE_Q: (usize, usize) = (3, 4);

/// Bound every closure, endpoint, and energy law in this file is asserted
/// against. Never `== 0.0` (consensus §5.2 correction 3).
const EXACT_TOLERANCE: f64 = 1e-12;

// ------------------------------------------------------------------- target

/// The two sweep axes, unnormalized. Both generic: neither lies along a
/// coordinate axis, neither is parallel or antiparallel to the local
/// commutator axis, so the conjugator takes its `general` branch throughout
/// and no grid point is a branch artifact.
const AXES: [(&str, [f64; 3]); 2] = [("n1", [1.0, -2.0, 3.0]), ("n2", [2.0, 1.0, -1.0])];

/// The principal angles the amplitude sweep visits, in radians. Chosen to
/// span the domain of [`mus_gesture::balanced_commutator_angle`] — small,
/// mid, and approaching `pi` — on a round decimal grid, BEFORE any of them
/// was rendered. Consensus rule 9: nothing here is dropped for being
/// inconvenient.
const SWEEP_ANGLES: [f64; 7] = [0.15, 0.35, 0.6, 0.9, 1.4, 2.0, 2.7];

/// The headline target: `AXES[0]` at `0.9` rad, one interior point of the
/// grid above. It carries no distinguished status — the stimulus set is
/// rendered at one grid point because a stimulus set has to be, and the sweep
/// exists so the reader can see what the neighbouring points do.
const HEADLINE_ANGLE: f64 = 0.9;

/// `R(axis, angle)` by Rodrigues, built through the same
/// [`Matrix::rotate_rows`] kernel discipline the rest of the stack uses for
/// its dense bookkeeping: `I + sin(a) K + (1 - cos(a)) K^2`.
fn rotation_about(axis: [f64; 3], angle: f64) -> Matrix {
    let norm = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    let unit = [axis[0] / norm, axis[1] / norm, axis[2] / norm];
    let generator = Matrix::from_rows(vec![
        vec![0.0, -unit[2], unit[1]],
        vec![unit[2], 0.0, -unit[0]],
        vec![-unit[1], unit[0], 0.0],
    ])
    .expect("a hand-written 3x3 is rectangular");
    let square = generator.multiply(&generator);
    let (sine, cosine) = angle.sin_cos();
    let mut matrix = Matrix::identity(3);
    for row in 0..3 {
        for column in 0..3 {
            matrix.set(
                row,
                column,
                matrix.get(row, column)
                    + sine * generator.get(row, column)
                    + (1.0 - cosine) * square.get(row, column),
            );
        }
    }
    matrix
}

fn target_rotation() -> Matrix {
    rotation_about(AXES[0].1, HEADLINE_ANGLE)
}

/// Embed a `3x3` operator on courses 0,1,2 into the network's `COURSES`
/// dimensional wave space, identity elsewhere — the space every stimulus's
/// executed word is compared in, including the commuting control, which
/// touches course 3 and 4.
fn embed(small: &Matrix) -> Matrix {
    let mut big = Matrix::identity(COURSES);
    for row in 0..3 {
        for column in 0..3 {
            big.set(row, column, small.get(row, column));
        }
    }
    big
}

// -------------------------------------------------------------- word timing

/// Lower a compiled word onto the production seam at its own constant
/// thermodynamic speed.
///
/// Gate `k` fires at `start + sum_{j<=k} tau_j`: the control has swept
/// through `theta_k` by the *end* of its segment, which is the only reading
/// under which the word's final gate coincides with the end of the declared
/// duration. The last gate therefore lands exactly at `start + duration`.
///
/// Rounding to a sample can collide two gates on one sample when a segment is
/// shorter than `1/SR`; that is not a problem, because the seam executes
/// frames sharing a sample in list order, and list order is the word's order.
fn schedule_word(
    gates: &[GivensGate],
    graph: &ControlGraph,
    start_s: f64,
    duration_s: f64,
) -> Vec<GestureFrame> {
    let metric = FrictionMetric::resolve(gates, graph).expect("gates run on authored edges");
    let schedule =
        thermodynamic_schedule(gates, &metric, duration_s).expect("a positive finite duration");
    let mut elapsed = 0.0;
    let mut word = Vec::with_capacity(gates.len());
    for (gate, segment) in gates.iter().zip(schedule.segment_durations()) {
        elapsed += segment;
        word.push(GestureFrame {
            at_sample: ((start_s + elapsed) * SR_F64).round() as usize,
            course_a: gate.left,
            course_b: gate.right,
            theta: gate.theta,
        });
    }
    word
}

/// The exact reversal-negation of a word: reversed order, negated angles,
/// same course pairs. Its product with the original is the identity by
/// construction, not by cancellation of accumulated error.
fn reversal_negation(word: &[GestureFrame]) -> Vec<GestureFrame> {
    word.iter()
        .rev()
        .map(|frame| GestureFrame {
            theta: -frame.theta,
            ..*frame
        })
        .collect()
}

// -------------------------------------------------------------- measurement

/// The dense operator the executed word actually realizes, composed in the
/// seam's own convention: frames apply front to back, so frame `k` left
/// multiplies what came before it.
fn word_operator(word: &[GestureFrame]) -> Matrix {
    let mut operator = Matrix::identity(COURSES);
    for frame in word {
        operator.rotate_rows(frame.course_a, frame.course_b, frame.theta);
    }
    operator
}

/// Per-course-pair integrated control angle of an executed word, in canonical
/// `(min, max)` orientation. A closed control loop returns every one of these
/// to zero; this is the production-side restatement of the compiler's
/// `edge_closure`, computed from what the voice actually ran rather than from
/// what the compiler intended.
fn executed_closure(trace: &GestureTrace) -> BTreeMap<(usize, usize), f64> {
    let mut closure: BTreeMap<(usize, usize), f64> = BTreeMap::new();
    for gate in &trace.applied {
        let (low, high, sign) = if gate.course_a <= gate.course_b {
            (gate.course_a, gate.course_b, 1.0)
        } else {
            (gate.course_b, gate.course_a, -1.0)
        };
        *closure.entry((low, high)).or_insert(0.0) += sign * gate.theta;
    }
    closure
}

fn rms(audio: &[f32]) -> f64 {
    if audio.is_empty() {
        return 0.0;
    }
    let sum: f64 = audio.iter().map(|&s| f64::from(s) * f64::from(s)).sum();
    (sum / audio.len() as f64).sqrt()
}

fn peak(audio: &[f32]) -> f64 {
    audio
        .iter()
        .fold(0.0_f64, |worst, &s| worst.max(f64::from(s).abs()))
}

fn l2(audio: &[f32]) -> f64 {
    audio
        .iter()
        .map(|&s| f64::from(s) * f64::from(s))
        .sum::<f64>()
        .sqrt()
}

fn l2_distance(left: &[f32], right: &[f32]) -> f64 {
    assert_eq!(left.len(), right.len(), "stimuli must be the same length");
    left.iter()
        .zip(right)
        .map(|(&a, &b)| {
            let difference = f64::from(a) - f64::from(b);
            difference * difference
        })
        .sum::<f64>()
        .sqrt()
}

/// The dimensionless waveform-shape distance E15 reported, unchanged so the
/// two rounds' numbers are comparable in kind.
fn relative_distance(left: &[f32], right: &[f32]) -> f64 {
    let scale = 0.5 * (l2(left) + l2(right));
    if scale <= 0.0 {
        0.0
    } else {
        l2_distance(left, right) / scale
    }
}

fn decibels(ratio: f64) -> f64 {
    20.0 * ratio.max(f64::MIN_POSITIVE).log10()
}

fn vector_distance(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
}

fn vector_norm(values: &[f64]) -> f64 {
    values.iter().map(|v| v * v).sum::<f64>().sqrt()
}

// --------------------------------------------------------------- experiment

struct Stimulus {
    id: &'static str,
    what: &'static str,
    word: Vec<GestureFrame>,
    /// What the composed product of the executed gates must equal.
    expected: Matrix,
    audio: Vec<f32>,
    trace: GestureTrace,
}

impl Stimulus {
    fn render(
        id: &'static str,
        what: &'static str,
        word: Vec<GestureFrame>,
        expected: Matrix,
    ) -> Self {
        let patch: BTreeMap<String, String> = PATCH
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect();
        let (audio, _, trace) = StringNetworkVoice::weave(&patch, &NOTES, SLOT_S, 1.0)
            .render_traced_gestured(STRIDE, &[], &word)
            .expect("every word here names courses this network has");
        Self {
            id,
            what,
            word,
            expected,
            audio,
            trace,
        }
    }

    /// `‖product of executed gates − expected‖_F`.
    fn endpoint_error(&self) -> f64 {
        word_operator(&self.word).frobenius_distance(&self.expected)
    }

    /// The wave vector immediately after the word's last gate — the trace
    /// endpoint state. `None` for the empty word.
    fn endpoint_state(&self) -> Option<&[f64]> {
        self.trace
            .applied
            .len()
            .checked_sub(1)
            .and_then(|last| self.trace.waves_after(last))
    }

    fn worst_closure(&self) -> f64 {
        executed_closure(&self.trace)
            .values()
            .fold(0.0_f64, |worst, value| worst.max(value.abs()))
    }
}

/// The block-swapped word.
///
/// The compiler emits `inv(W_B) ++ inv(W_A) ++ W_B ++ W_A`, which realizes
/// `A B A⁻¹ B⁻¹` in the pinned front-to-back convention; swapping the roles of
/// A and B gives `B A B⁻¹ A⁻¹`, the inverse group element. Same twelve gates,
/// same authored edges, same angle multiset, same thermodynamic length — only
/// the order differs, so a difference in the render cannot be a difference in
/// effort. That is the round-2 contrast, now with each block individually
/// closed.
fn swap_blocks(certificate: &ClosedLoopCertificate) -> Vec<GivensGate> {
    let (macro_a, macro_b) = certificate
        .macro_certificates()
        .expect("a nonidentity target compiles two macros");
    let mut swapped: Vec<GivensGate> = macro_a.gates().iter().rev().map(|g| g.inverse()).collect();
    swapped.extend(macro_b.gates().iter().rev().map(|g| g.inverse()));
    swapped.extend_from_slice(macro_a.gates());
    swapped.extend_from_slice(macro_b.gates());
    assert_eq!(
        swapped.len(),
        certificate.gate_count(),
        "the swapped word is the same gates"
    );
    swapped
}

/// The three modes and two authored edges every word in this file compiles
/// on: exactly the planes the balanced commutator construction uses (`R01`,
/// `R12`). The graph is a path, so its spanning tree is itself and every
/// emitted gate is on an authored edge by construction.
fn control_graph() -> ControlGraph {
    ControlGraph::from_coordinate_edges(3, &[(0, 1), (1, 2)]).expect("a two-edge path on 3 modes")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = output_directory();
    fs::create_dir_all(&out_dir)?;

    // ---- the control graph and the target ----------------------------
    let graph = control_graph();
    let target_matrix = target_rotation();
    let target = RotationTarget::new(target_matrix.clone(), DEFAULT_TARGET_TOLERANCE)?;
    let mirror_matrix = target_matrix.transpose();
    let mirror_target = RotationTarget::new(mirror_matrix.clone(), DEFAULT_TARGET_TOLERANCE)?;

    let certificate = compile_exact_closed_loop_so3(&target, &graph, WINDOW_DURATION_S)?;
    let mirror_certificate =
        compile_exact_closed_loop_so3(&mirror_target, &graph, WINDOW_DURATION_S)?;

    let swapped = swap_blocks(&certificate);

    // ---- words -------------------------------------------------------
    let loop_ab = schedule_word(
        certificate.gates(),
        &graph,
        WINDOW_START_S,
        WINDOW_DURATION_S,
    );
    let loop_ba = schedule_word(&swapped, &graph, WINDOW_START_S, WINDOW_DURATION_S);
    let mirror = schedule_word(
        mirror_certificate.gates(),
        &graph,
        WINDOW_START_S,
        WINDOW_DURATION_S,
    );

    // The inverse loop is a 24-gate word, timed by ONE constant-speed
    // schedule over the whole window rather than two half-window schedules
    // glued together — the schedule is a property of the word, and the word
    // here is the concatenation.
    let mut inverse_gates: Vec<GivensGate> = certificate.gates().to_vec();
    inverse_gates.extend(certificate.gates().iter().rev().map(|g| g.inverse()));
    let inverse = schedule_word(&inverse_gates, &graph, WINDOW_START_S, WINDOW_DURATION_S);

    // The commuting control. `P` and `Q` act on disjoint course pairs, so
    // they commute exactly — they touch disjoint memory. Both generators
    // fire at the window's first gate time and both inverses at its last, so
    // the loop is genuinely spread across the sustain (the network evolves
    // between the opening and the closing) while the two ORDERS remain
    // byte-identical. The angle is the compiled word's own balanced
    // generator angle, so the effort is comparable to the loop words'.
    let commuting_angle = certificate.balanced_generator_angle_rad();
    let opened_at = loop_ab.first().expect("twelve gates").at_sample;
    let closed_at = loop_ab.last().expect("twelve gates").at_sample;
    let gesture = |at_sample: usize, pair: (usize, usize), theta: f64| GestureFrame {
        at_sample,
        course_a: pair.0,
        course_b: pair.1,
        theta,
    };
    let commute_pq = vec![
        gesture(opened_at, COMMUTE_P, commuting_angle),
        gesture(opened_at, COMMUTE_Q, -commuting_angle),
        gesture(closed_at, COMMUTE_P, -commuting_angle),
        gesture(closed_at, COMMUTE_Q, commuting_angle),
    ];
    let commute_qp = vec![
        gesture(opened_at, COMMUTE_Q, -commuting_angle),
        gesture(opened_at, COMMUTE_P, commuting_angle),
        gesture(closed_at, COMMUTE_Q, commuting_angle),
        gesture(closed_at, COMMUTE_P, -commuting_angle),
    ];

    // ---- render ------------------------------------------------------
    let identity = Matrix::identity(COURSES);
    let stimuli = vec![
        Stimulus::render("null", "empty word", Vec::new(), identity.clone()),
        Stimulus::render(
            "commute-pq",
            "P Q P^-1 Q^-1 on disjoint course pairs",
            commute_pq,
            identity.clone(),
        ),
        Stimulus::render(
            "commute-qp",
            "Q P Q^-1 P^-1, same samples",
            commute_qp,
            identity.clone(),
        ),
        Stimulus::render(
            "loop-ab",
            "compiled 12-gate A B A^-1 B^-1",
            loop_ab.clone(),
            embed(&target_matrix),
        ),
        Stimulus::render(
            "loop-ba",
            "the same twelve gates, blocks swapped",
            loop_ba,
            embed(&mirror_matrix),
        ),
        Stimulus::render(
            "mirror",
            "12 gates compiled for the axis-mirrored target",
            mirror,
            embed(&mirror_matrix),
        ),
        Stimulus::render(
            "inverse",
            "loop-ab then its exact reversal-negation",
            inverse,
            identity.clone(),
        ),
    ];

    // ---- report ------------------------------------------------------
    println!("Ariadne Phase 2 §C — same-endpoint production experiment");
    println!("=======================================================");
    println!();
    println!("patch: {PATCH:?}");
    println!("notes: {NOTES:?} Hz   slot: {SLOT_S} s   stride: {STRIDE}   courses: {COURSES}");
    println!(
        "gesture window: [{WINDOW_START_S}, {}] s at {SR} Hz",
        WINDOW_START_S + WINDOW_DURATION_S
    );
    println!("no master limiter, no normalization, no matched-gain stage (consensus rule 7)");
    println!();

    report_compilation(
        &certificate,
        &mirror_certificate,
        &graph,
        &target,
        &mirror_target,
    )?;
    report_schedule(&stimuli);
    report_endpoints(&stimuli);
    report_closure(&stimuli);
    report_loudness(&stimuli);
    report_audible_distances(&stimuli);
    report_state_distances(&stimuli);
    report_coincident_inverse(&certificate);
    report_amplitude_sweep(&graph)?;

    // ---- laws --------------------------------------------------------
    check_laws(&stimuli);

    // ---- artifacts ---------------------------------------------------
    for stimulus in &stimuli {
        let path = out_dir.join(format!("phase2-{}.wav", stimulus.id));
        write_wav_f32(&stimulus.audio, 1, SR as u32, &path)?;
        println!("wrote {}", path.display());
    }
    let receipt = ClosedLoopReceipt::new(
        &graph,
        &target,
        &certificate,
        Some(&measure_control_work(
            certificate.program(),
            &basis_probes(3),
        )?),
    );
    let mirror_receipt = ClosedLoopReceipt::new(
        &graph,
        &mirror_target,
        &mirror_certificate,
        Some(&measure_control_work(
            mirror_certificate.program(),
            &basis_probes(3),
        )?),
    );
    let receipt_path = out_dir.join("phase2-closed-loop-receipt.json");
    fs::write(&receipt_path, receipt.to_json()?)?;
    println!("wrote {}", receipt_path.display());
    let mirror_path = out_dir.join("phase2-mirror-receipt.json");
    fs::write(&mirror_path, mirror_receipt.to_json()?)?;
    println!("wrote {}", mirror_path.display());

    Ok(())
}

fn output_directory() -> PathBuf {
    if let Some(argument) = std::env::args().nth(1) {
        return PathBuf::from(argument);
    }
    // `<repo>/mus-rs/crates/mus-dsp` -> `<repo>`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("the crate lives three levels below the repository root")
        .join(".string-research")
        .join("phase2-stimuli")
}

fn report_compilation(
    certificate: &ClosedLoopCertificate,
    mirror: &ClosedLoopCertificate,
    graph: &ControlGraph,
    target: &RotationTarget,
    mirror_target: &RotationTarget,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("-- compilation (mus-gesture, Phase 2 §A) --");
    println!(
        "control graph: 3 modes, authored edges {:?}",
        graph.edge_coordinates()
    );
    for (name, certificate, target) in [
        ("target R  ", certificate, target),
        ("mirror R^T", mirror, mirror_target),
    ] {
        let (angle, axis) = principal_angle_axis(target.matrix())?;
        println!(
            "{name}: phi={angle:.15} rad  axis=[{:.15}, {:.15}, {:.15}]",
            axis[0], axis[1], axis[2]
        );
        println!(
            "{name}: balanced generator t={:.15} rad  conjugator branch={}  gates={}",
            certificate.balanced_generator_angle_rad(),
            certificate.conjugator_branch().name(),
            certificate.gate_count()
        );
        println!(
            "{name}: endpoint error {:.6e} Fro / {:.6e} rad   orthogonality {:.6e}   |det-1| \
             {:.6e}",
            certificate.reconstruction_error_fro(),
            certificate.reconstruction_error_rad(),
            certificate.orthogonality_error_fro(),
            certificate.determinant_error()
        );
        println!(
            "{name}: compiler per-edge closure {:?}  max {:.6e}",
            certificate
                .edge_closure()
                .into_iter()
                .map(|(edge, closure)| (edge.0, closure))
                .collect::<Vec<_>>(),
            certificate.max_edge_closure()
        );
        let schedule = certificate.schedule();
        println!(
            "{name}: thermodynamic length {:.15}  D_min {:.15}  constant-speed residual {:.6e}",
            schedule.thermodynamic_length(),
            schedule.minimum_quadratic_cost(),
            schedule.constant_speed_residual()
        );
        let work = measure_control_work(certificate.program(), &basis_probes(3))?;
        println!(
            "{name}: measured control work — metric {}, frames {}, declared {:.3e}, max |residual| \
             {:.6e}",
            work.metric(),
            work.frames(),
            work.declared_control_work(),
            work.max_abs_residual()
        );
    }
    println!();
    Ok(())
}

fn report_schedule(stimuli: &[Stimulus]) {
    println!("-- schedule (constant thermodynamic speed, gate at the END of its segment) --");
    for stimulus in stimuli {
        if stimulus.word.is_empty() {
            println!("{:>11}: no gates", stimulus.id);
            continue;
        }
        let times: Vec<String> = stimulus
            .word
            .iter()
            .map(|frame| format!("{:.4}", frame.at_sample as f64 / SR_F64))
            .collect();
        println!(
            "{:>11}: {} gates at t = [{}] s",
            stimulus.id,
            stimulus.word.len(),
            times.join(", ")
        );
        let planes: Vec<String> = stimulus
            .word
            .iter()
            .map(|frame| format!("({},{}){:+.4}", frame.course_a, frame.course_b, frame.theta))
            .collect();
        println!("{:>11}  {}", "", planes.join(" "));
    }
    println!();
}

fn report_endpoints(stimuli: &[Stimulus]) {
    println!("-- executed-word endpoint operators (composed from the frames the voice ran) --");
    println!("{:>11}  {:>14}  expected", "stimulus", "‖W−expected‖_F");
    for stimulus in stimuli {
        println!(
            "{:>11}  {:>14.6e}  {}",
            stimulus.id,
            stimulus.endpoint_error(),
            stimulus.what
        );
    }
    println!();
}

fn report_closure(stimuli: &[Stimulus]) {
    println!("-- per-edge closure receipts, computed from EXECUTED gates --");
    for stimulus in stimuli {
        let closure = executed_closure(&stimulus.trace);
        if closure.is_empty() {
            println!("{:>11}: no edges touched", stimulus.id);
            continue;
        }
        let entries: Vec<String> = closure
            .iter()
            .map(|((low, high), sum)| format!("({low},{high})={sum:+.3e}"))
            .collect();
        println!(
            "{:>11}: max |closure| {:.6e}   {}",
            stimulus.id,
            stimulus.worst_closure(),
            entries.join(" ")
        );
    }
    println!();
    println!("-- localized factor-stage work (law 3): worst relative pair-energy drift --");
    for stimulus in stimuli {
        println!(
            "{:>11}: {:.6e} over {} executed gates",
            stimulus.id,
            stimulus.trace.worst_pair_energy_drift(),
            stimulus.trace.applied.len()
        );
    }
    println!();
}

fn report_loudness(stimuli: &[Stimulus]) {
    let reference = rms(&stimuli[0].audio);
    println!("-- matched loudness (reported, NOT imposed: no limiter, no normalization) --");
    println!(
        "{:>11}  {:>10}  {:>10}  {:>10}  {:>12}",
        "stimulus", "rms", "peak", "rms dBFS", "dB vs null"
    );
    for stimulus in stimuli {
        let level = rms(&stimulus.audio);
        println!(
            "{:>11}  {:>10.7}  {:>10.7}  {:>10.4}  {:>12.5}",
            stimulus.id,
            level,
            peak(&stimulus.audio),
            decibels(level),
            decibels(level / reference)
        );
    }
    println!();
}

fn report_audible_distances(stimuli: &[Stimulus]) {
    let find = |id: &str| {
        stimuli
            .iter()
            .find(|stimulus| stimulus.id == id)
            .expect("every reported id is rendered")
    };
    println!("-- audible distances between stimuli (rendered f32, no post-processing) --");
    println!(
        "{:>24}  {:>12}  {:>12}  {:>10}  note",
        "pair", "rel_dist", "abs_dist", "dB_diff"
    );
    let pairs: [(&str, &str, &str); 6] = [
        (
            "loop-ab",
            "loop-ba",
            "order of the SAME twelve closed gates",
        ),
        (
            "commute-pq",
            "commute-qp",
            "commuting order: the null result",
        ),
        ("null", "inverse", "same endpoint I, 0 vs 24 gates"),
        ("loop-ba", "mirror", "same endpoint R^T, different words"),
        ("null", "loop-ab", "the word against no word"),
        ("loop-ab", "mirror", "R against R^T"),
    ];
    for (left, right, note) in pairs {
        let a = find(left);
        let b = find(right);
        println!(
            "{:>24}  {:>12.6}  {:>12.4}  {:>10.5}  {}",
            format!("{left} vs {right}"),
            relative_distance(&a.audio, &b.audio),
            l2_distance(&a.audio, &b.audio),
            decibels(rms(&a.audio) / rms(&b.audio)),
            note
        );
    }
    println!();
}

fn report_state_distances(stimuli: &[Stimulus]) {
    println!("-- trace-endpoint state: the wave vector immediately after the word's last gate --");
    println!("{:>11}  {:>12}  note", "stimulus", "‖state‖");
    for stimulus in stimuli {
        match stimulus.endpoint_state() {
            Some(state) => println!(
                "{:>11}  {:>12.7}  after gate {}",
                stimulus.id,
                vector_norm(state),
                stimulus.trace.applied.len() - 1
            ),
            None => println!("{:>11}  {:>12}  no gates executed", stimulus.id, "—"),
        }
    }
    println!();
    let find = |id: &str| {
        stimuli
            .iter()
            .find(|stimulus| stimulus.id == id)
            .expect("every reported id is rendered")
    };
    println!("{:>24}  {:>14}  {:>14}", "pair", "‖Δstate‖", "relative");
    for (left, right) in [
        ("loop-ab", "loop-ba"),
        ("commute-pq", "commute-qp"),
        ("loop-ba", "mirror"),
        ("loop-ab", "mirror"),
    ] {
        let (Some(a), Some(b)) = (find(left).endpoint_state(), find(right).endpoint_state()) else {
            continue;
        };
        let scale = 0.5 * (vector_norm(a) + vector_norm(b));
        println!(
            "{:>24}  {:>14.6e}  {:>14.6e}",
            format!("{left} vs {right}"),
            vector_distance(a, b),
            if scale > 0.0 {
                vector_distance(a, b) / scale
            } else {
                0.0
            }
        );
    }
    println!();
}

/// The literal "trace endpoint returns" check, which only has a state-level
/// meaning when the network is not allowed to evolve between the word and its
/// reversal-negation: all 24 gates on ONE sample. Spread across the sustain
/// (the `inverse` stimulus above) the operator still closes on the identity,
/// but the state does not return — that difference is the dynamics, which is
/// the music, not an error.
fn report_coincident_inverse(certificate: &ClosedLoopCertificate) {
    let mut word: Vec<GestureFrame> = certificate
        .gates()
        .iter()
        .map(|gate| GestureFrame {
            at_sample: (WINDOW_START_S * SR_F64).round() as usize,
            course_a: gate.left,
            course_b: gate.right,
            theta: gate.theta,
        })
        .collect();
    word.extend(reversal_negation(&word));
    let patch: BTreeMap<String, String> = PATCH
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    let (_, _, trace) = StringNetworkVoice::weave(&patch, &NOTES, SLOT_S, 1.0)
        .render_traced_gestured(STRIDE, &[], &word)
        .expect("in-range word");
    let before = trace.waves_before(0).expect("state before the word");
    let after = trace
        .waves_after(trace.applied.len() - 1)
        .expect("state after the word");
    let distance = vector_distance(before, after);
    let norm = vector_norm(before);
    println!("-- coincident inverse loop (all 24 gates at one sample) --");
    println!(
        "trace endpoint returns: ‖state_after − state_before‖ = {distance:.6e}  (‖state‖ = \
         {norm:.7}, relative {:.6e})",
        distance / norm.max(f64::MIN_POSITIVE)
    );
    assert!(
        distance <= EXACT_TOLERANCE * norm.max(1.0),
        "the coincident inverse loop must return the state, off by {distance:e}"
    );
    println!();
}

/// The amplitude sweep, in E15 §3's shape and for E15's reason: one grid
/// point is an anecdote. For each `(axis, phi)` the target is recompiled, the
/// two block orders are rendered, and the order effect is measured — with
/// the closure and endpoint laws re-checked at every point, so a large
/// measured difference can never come from a word that failed to close.
///
/// Reported, not fitted. The grid was fixed before it was rendered and no
/// point is dropped.
fn report_amplitude_sweep(graph: &ControlGraph) -> Result<(), Box<dyn std::error::Error>> {
    println!("-- amplitude sweep: the order effect against the target's principal angle --");
    println!(
        "{:>4} {:>7} {:>9} {:>9} {:>12} {:>12} {:>10} {:>12} {:>12}",
        "axis",
        "phi",
        "t_bal",
        "length L",
        "rel_dist",
        "abs_dist",
        "dB_diff",
        "‖Δstate‖",
        "closure"
    );
    for (axis_name, axis) in AXES {
        for angle in SWEEP_ANGLES {
            let matrix = rotation_about(axis, angle);
            let target = RotationTarget::new(matrix.clone(), DEFAULT_TARGET_TOLERANCE)?;
            let certificate = compile_exact_closed_loop_so3(&target, graph, WINDOW_DURATION_S)?;
            let forward = schedule_word(
                certificate.gates(),
                graph,
                WINDOW_START_S,
                WINDOW_DURATION_S,
            );
            let reversed = schedule_word(
                &swap_blocks(&certificate),
                graph,
                WINDOW_START_S,
                WINDOW_DURATION_S,
            );
            let ab = Stimulus::render("sweep-ab", "A B A^-1 B^-1", forward, embed(&matrix));
            let ba = Stimulus::render(
                "sweep-ba",
                "B A B^-1 A^-1",
                reversed,
                embed(&matrix.transpose()),
            );
            let worst_closure = ab.worst_closure().max(ba.worst_closure());
            let worst_endpoint = ab.endpoint_error().max(ba.endpoint_error());
            assert!(
                worst_closure <= EXACT_TOLERANCE && worst_endpoint <= EXACT_TOLERANCE,
                "sweep point ({axis_name}, {angle}) did not close: closure {worst_closure:e}, \
                 endpoint {worst_endpoint:e}"
            );
            let state_distance = match (ab.endpoint_state(), ba.endpoint_state()) {
                (Some(left), Some(right)) => vector_distance(left, right),
                _ => 0.0,
            };
            println!(
                "{axis_name:>4} {angle:>7.2} {:>9.4} {:>9.4} {:>12.6} {:>12.4} {:>10.5} \
                 {:>12.6e} {:>12.3e}",
                certificate.balanced_generator_angle_rad(),
                certificate.schedule().thermodynamic_length(),
                relative_distance(&ab.audio, &ba.audio),
                l2_distance(&ab.audio, &ba.audio),
                decibels(rms(&ab.audio) / rms(&ba.audio)),
                state_distance,
                worst_closure
            );
        }
    }
    println!();
    Ok(())
}

fn check_laws(stimuli: &[Stimulus]) {
    let find = |id: &str| {
        stimuli
            .iter()
            .find(|stimulus| stimulus.id == id)
            .expect("every checked id is rendered")
    };
    println!("-- laws --");

    for stimulus in stimuli {
        assert!(
            stimulus.endpoint_error() <= EXACT_TOLERANCE,
            "{}: executed word missed its endpoint operator by {:e}",
            stimulus.id,
            stimulus.endpoint_error()
        );
        assert!(
            stimulus.worst_closure() <= EXACT_TOLERANCE,
            "{}: per-edge control did not close, worst {:e}",
            stimulus.id,
            stimulus.worst_closure()
        );
        assert!(
            stimulus.trace.worst_pair_energy_drift() <= 1e-13,
            "{}: a gate moved pair energy by {:e}",
            stimulus.id,
            stimulus.trace.worst_pair_energy_drift()
        );
    }
    println!("  every executed word hits its endpoint operator and closes every edge");

    assert_eq!(
        find("commute-pq").audio,
        find("commute-qp").audio,
        "commuting controls must produce the null result, byte-identically"
    );
    println!("  commuting controls: both orders byte-identical (assert_eq! on raw f32)");

    let ab = find("loop-ab");
    let ba = find("loop-ba");
    assert_ne!(
        ab.audio, ba.audio,
        "the two orders of a noncommuting closed word must differ"
    );
    let order_distance = relative_distance(&ab.audio, &ba.audio);
    assert!(
        order_distance > 1e-3,
        "the order effect must be a signal, not roundoff: rel_dist {order_distance:e}"
    );
    println!("  A,B,−A,−B vs B,A,−B,−A: rel_dist {order_distance:.6}, and not byte-identical");

    // Effort matching, which is what makes the line above a statement about
    // ORDER: the two words are the same twelve gates on the same authored
    // edges, so their angle multisets — and therefore their thermodynamic
    // lengths, and therefore their predicted minimum quadratic costs — are
    // identical, not merely close. Asserted on the sorted multiset.
    let sorted_angles = |stimulus: &Stimulus| {
        let mut angles: Vec<f64> = stimulus
            .word
            .iter()
            .map(|frame| frame.theta.abs())
            .collect();
        angles.sort_by(f64::total_cmp);
        angles
    };
    assert_eq!(
        sorted_angles(ab),
        sorted_angles(ba),
        "the two orders must be the same twelve gates, or the contrast is about effort"
    );
    println!(
        "  and they are the same twelve angles: thermodynamic length {:.6} both ways",
        sorted_angles(ab).iter().sum::<f64>()
    );

    let null = find("null");
    let inverse = find("inverse");
    assert_ne!(
        null.audio, inverse.audio,
        "a 24-gate word whose operator is the identity is still not no word"
    );
    println!(
        "  same endpoint I, different audio: rel_dist {:.6}",
        relative_distance(&null.audio, &inverse.audio)
    );

    let mirror = find("mirror");
    assert_ne!(
        ba.audio, mirror.audio,
        "two different words with the same endpoint must not render identically"
    );
    println!(
        "  same endpoint R^T, different words: rel_dist {:.6}",
        relative_distance(&ba.audio, &mirror.audio)
    );

    // Level matching is REPORTED, not imposed — so it is also asserted, to
    // catch the failure mode E15 ruled out by hand: "one order is just
    // louder".
    let level_difference = decibels(rms(&ab.audio) / rms(&ba.audio)).abs();
    assert!(
        level_difference < 1.0,
        "the two orders differ by {level_difference:.4} dB — that is a loudness artifact, not a \
         path effect"
    );
    println!("  the two orders are level-matched to {level_difference:.5} dB, unaided");
    println!();
}
