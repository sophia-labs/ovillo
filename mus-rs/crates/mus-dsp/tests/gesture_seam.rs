//! Ariadne Phase 2 §B — the gesture seam in the production voice.
//!
//! A compiled control word becomes executable inside a live note: each
//! gate is an explicit plane rotation, at a declared sample, on the same
//! wave variables the Weave ring's own Givens rotations act on. The seven
//! spec laws are the test list below, in order.
//!
//! 1. Empty gesture list == today's render, byte-identical (pluck AND
//!    weave).
//! 2. `theta = 0` frames are byte-inert; `[theta, -theta]` at the SAME
//!    sample composes to the identity.
//! 3. Energy-neutral at the application site: each gate preserves its two
//!    courses' wave sum-of-squares to roundoff.
//! 4. Rotations on DISJOINT course pairs at the same sample commute
//!    byte-identically (they touch disjoint memory — exact).
//! 5. Executor agreement: on a lossless configuration, a 12-gate word
//!    applied all at one sample rotates the course wave-vector by the
//!    dense product of its gates.
//! 6. Host-block invariance with gestures armed.
//! 7. Determinism; out-of-range course indices are a typed error, not a
//!    clamp.
//!
//! Plus one interaction the phase's shared rules demand: the Phase 0 §B
//! energy ledger still closes with a word armed.

use std::collections::BTreeMap;

use mus_dsp::pluck::{
    pluck_note, weave_note, ControlFrame, ControlTarget, GestureError, GestureFrame,
    StringNetworkVoice,
};

fn params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

/// The working Weave patch for the byte laws: coupled, curved, orbiting —
/// every path-dependent mechanism live, so a law that only holds on a
/// quiet network cannot pass here by accident.
fn weave_patch() -> BTreeMap<String, String> {
    params(&[
        ("synth", "weave"),
        ("sus", "1.4"),
        ("courses", "9"),
        ("couple", "0.19"),
        ("chirality", "0.4"),
        ("curvature", "0.5"),
        ("body", "0.4"),
        ("orbit", "0.5"),
        ("orbit_depth", "0.7"),
    ])
}

const NOTES: [f64; 2] = [110.0, 165.0];
const SLOT: f64 = 0.5;
const STRIDE: usize = 256;

fn weave_voice() -> StringNetworkVoice {
    StringNetworkVoice::weave(&weave_patch(), &NOTES, SLOT, 1.0)
}

fn gesture(at_sample: usize, course_a: usize, course_b: usize, theta: f64) -> GestureFrame {
    GestureFrame {
        at_sample,
        course_a,
        course_b,
        theta,
    }
}

/// Law 1, the hard zero limit: an empty word is exactly no word — on the
/// plain entry points, under a trajectory, and on both voices.
#[test]
fn an_empty_gesture_list_is_byte_identical() {
    let patch = weave_patch();
    let plain = weave_note(&patch, &NOTES, SLOT, 1.0);
    let (gestured, _, trace) = weave_voice()
        .render_traced_gestured(STRIDE, &[], &[])
        .expect("empty word cannot refuse");
    assert_eq!(plain, gestured, "weave: no gestures is exactly no gestures");
    assert!(trace.applied.is_empty(), "nothing executed");
    assert!(trace.waves.is_empty(), "and nothing was recorded");

    // Under a live trajectory, too: the seam must not perturb automation.
    let trajectory = [ControlFrame {
        at_sample: 9_600,
        sets: vec![
            (ControlTarget::Couple, 0.31),
            (ControlTarget::Curvature, 0.9),
        ],
    }];
    let (traced, _) = weave_voice().render_traced_with(STRIDE, &trajectory);
    let (traced_gestured, _, _) = weave_voice()
        .render_traced_gestured(STRIDE, &trajectory, &[])
        .expect("empty word cannot refuse");
    assert_eq!(traced, traced_gestured, "weave under trajectory");

    // Pluck: the guitar has no ring, and the seam still owes it the law.
    let guitar = params(&[("synth", "pluck"), ("sus", "1.2"), ("body", "0.45")]);
    let plucked = pluck_note(&guitar, &NOTES, SLOT, 1.0);
    let (plucked_gestured, _, _) = StringNetworkVoice::guitar(&guitar, &NOTES, SLOT, 1.0)
        .render_traced_gestured(STRIDE, &[], &[])
        .expect("empty word cannot refuse");
    assert_eq!(
        plucked, plucked_gestured,
        "pluck: no gestures is no gestures"
    );
}

/// Law 2a: a zero angle performs no state arithmetic. `cos 0` and `sin 0`
/// are exact, but `0.0 * a + 1.0 * b` maps `-0.0` to `+0.0`, so the
/// executor skips the rotation outright rather than trusting it. `-0.0`
/// is an identity rotation too, and is skipped by the same test.
#[test]
fn zero_angle_gestures_are_byte_inert() {
    let plain = weave_note(&weave_patch(), &NOTES, SLOT, 1.0);
    let word = [
        gesture(0, 0, 1, 0.0),
        gesture(1, 3, 7, -0.0),
        gesture(4_800, 2, 5, 0.0),
        gesture(4_800, 5, 2, -0.0),
        gesture(19_200, 8, 0, 0.0),
    ];
    let (audio, _, trace) = weave_voice()
        .render_traced_gestured(STRIDE, &[], &word)
        .expect("in-range word");
    assert_eq!(plain, audio, "a zero angle is byte-inert");
    assert_eq!(trace.applied.len(), word.len(), "but it is still recorded");
    for (index, gate) in trace.applied.iter().enumerate() {
        assert_eq!(
            trace.waves_before(index),
            trace.waves_after(index),
            "no wave moved"
        );
        assert_eq!(
            gate.pair_energy_before.to_bits(),
            gate.pair_energy_after.to_bits(),
            "and no energy moved"
        );
    }
}

/// Law 2b: `[theta, -theta]` at the SAME sample composes to the identity.
///
/// The composition is the identity in exact arithmetic and identity to
/// roundoff in floating point — `R(-t) R(t) v = v (c^2 + s^2)` reassociated
/// through four roundings, so it returns the pair to within a few ulp
/// rather than to the bit. What is asserted here is both halves of that
/// honestly: the state returns to <= 8 ulp, and the rendered audio is
/// byte-identical because a perturbation eight decades below the f32
/// mantissa cannot survive the cast.
#[test]
fn an_inverse_pair_at_one_sample_composes_to_the_identity() {
    let plain = weave_note(&weave_patch(), &NOTES, SLOT, 1.0);
    let theta = 0.61;
    let word = [
        gesture(6_000, 1, 4, theta),
        gesture(6_000, 1, 4, -theta),
        gesture(15_000, 0, 7, -0.23),
        gesture(15_000, 0, 7, 0.23),
    ];
    let (audio, _, trace) = weave_voice()
        .render_traced_gestured(STRIDE, &[], &word)
        .expect("in-range word");
    assert_eq!(trace.applied.len(), 4);
    // Non-vacuity: the courses under the gates are ringing, and the
    // FIRST gate of each pair really moved them. A composition that
    // returns nothing to nothing would prove nothing.
    for pair in 0..2 {
        let opening = trace.applied[pair * 2];
        assert!(
            opening.pair_energy_before > 1e-12,
            "pair {pair} is silent: {}",
            opening.pair_energy_before
        );
        assert_ne!(
            trace.waves_before(pair * 2),
            trace.waves_after(pair * 2),
            "pair {pair}'s opening gate moved the state"
        );
    }
    for pair in 0..2 {
        let before = trace.waves_before(pair * 2).expect("snapshot");
        let after = trace.waves_after(pair * 2 + 1).expect("snapshot");
        for (course, (start, end)) in before.iter().zip(after.iter()).enumerate() {
            let ulps = (start - end).abs() / f64::EPSILON.max(start.abs() * f64::EPSILON);
            assert!(
                ulps <= 8.0,
                "course {course} returned to {end} from {start} ({ulps:.2} ulp)"
            );
        }
    }
    assert_eq!(
        plain, audio,
        "an inverse pair at one sample leaves the render where it was"
    );
}

/// Law 3, the localized "factor-stage work is zero" gate: every executed
/// gate preserves its two courses' wave sum-of-squares. Measured here,
/// asserted at the site by a `debug_assert!` in the executor.
#[test]
fn every_gate_is_energy_neutral_at_the_application_site() {
    // A long, ugly word: many angles, many pairs, over the whole note.
    let word: Vec<GestureFrame> = (0..96)
        .map(|step: usize| {
            let theta = 0.9 * ((step as f64) * 0.37).sin() - 0.4;
            gesture(step * 211, step % 9, (step * 5 + 3) % 9, theta)
        })
        .filter(|frame| frame.course_a != frame.course_b)
        .collect();
    let (audio, _, trace) = weave_voice()
        .render_traced_gestured(STRIDE, &[], &word)
        .expect("in-range word");
    assert_eq!(trace.applied.len(), word.len(), "every gate executed");
    let drift = trace.worst_pair_energy_drift();
    assert!(
        drift <= 1e-13,
        "worst relative pair-energy drift {drift:e} is not roundoff"
    );
    assert!(audio.iter().all(|sample| sample.is_finite()));
    // The seam is not a no-op dressed as a law: a real word is audible.
    let plain = weave_note(&weave_patch(), &NOTES, SLOT, 1.0);
    assert_ne!(plain, audio, "a nonzero word changes the render");
}

/// Law 4: rotations on disjoint course pairs at the same sample touch
/// disjoint memory, so the two orders are byte-identical — asserted
/// EXACTLY, not to a tolerance.
///
/// The positive control is the point of the whole instrument: swap the
/// orders of two OVERLAPPING pairs and the render changes.
#[test]
fn disjoint_pairs_at_one_sample_commute_byte_identically() {
    let render = |word: &[GestureFrame]| -> Vec<f32> {
        weave_voice()
            .render_traced_gestured(STRIDE, &[], word)
            .expect("in-range word")
            .0
    };
    let left = gesture(7_200, 0, 1, 0.41);
    let right = gesture(7_200, 2, 3, -0.27);
    assert_eq!(
        render(&[left, right]),
        render(&[right, left]),
        "disjoint pairs commute exactly"
    );

    let overlapping = gesture(7_200, 1, 2, -0.27);
    assert_ne!(
        render(&[left, overlapping]),
        render(&[overlapping, left]),
        "overlapping pairs do not — that is the holonomy the paper is about"
    );
}

type Mat3 = [[f64; 3]; 3];

/// The dense plane rotation matching [`givens`]'s convention exactly:
/// `a' = cos t * a - sin t * b`, `b' = sin t * a + cos t * b`.
fn plane_rotation(course_a: usize, course_b: usize, theta: f64) -> Mat3 {
    let (sin, cos) = theta.sin_cos();
    let mut matrix = [[0.0; 3]; 3];
    for (index, row) in matrix.iter_mut().enumerate() {
        row[index] = 1.0;
    }
    matrix[course_a][course_a] = cos;
    matrix[course_a][course_b] = -sin;
    matrix[course_b][course_a] = sin;
    matrix[course_b][course_b] = cos;
    matrix
}

fn matmul(left: Mat3, right: Mat3) -> Mat3 {
    let mut out = [[0.0; 3]; 3];
    for (row, slot) in out.iter_mut().enumerate() {
        for (column, value) in slot.iter_mut().enumerate() {
            *value = (0..3).map(|k| left[row][k] * right[k][column]).sum();
        }
    }
    out
}

fn apply(matrix: Mat3, vector: [f64; 3]) -> [f64; 3] {
    let mut out = [0.0; 3];
    for (row, slot) in out.iter_mut().enumerate() {
        *slot = (0..3).map(|k| matrix[row][k] * vector[k]).sum();
    }
    out
}

/// A local generator: three gates on the (0,1) and (1,2) planes.
fn generator(at_sample: usize, angles: [f64; 3], flip: bool) -> Vec<GestureFrame> {
    let planes: [(usize, usize); 3] = if flip {
        [(1, 2), (0, 1), (1, 2)]
    } else {
        [(0, 1), (1, 2), (0, 1)]
    };
    planes
        .iter()
        .zip(angles.iter())
        .map(|(&(a, b), &theta)| gesture(at_sample, a, b, theta))
        .collect()
}

/// The exact inverse of a gate sequence: reversed order, negated angles.
fn inverted(word: &[GestureFrame]) -> Vec<GestureFrame> {
    word.iter()
        .rev()
        .map(|frame| GestureFrame {
            theta: -frame.theta,
            ..*frame
        })
        .collect()
}

/// Law 5, executor agreement in its honest form. On a lossless
/// configuration (damp 0, huge sus, couple 0, no dispersion, no body) a
/// 12-gate balanced-commutator word `A B A^-1 B^-1`, applied all at ONE
/// sample, rotates the live course wave-vector by the dense product of
/// its gates.
///
/// All at one sample is the whole point: between frames the state evolves
/// through the waveguides, and that evolution is the music, not an error
/// — so the operator claim is made exactly where the operator applies.
#[test]
fn a_twelve_gate_word_agrees_with_its_dense_operator() {
    let lossless = params(&[
        ("synth", "weave"),
        ("sus", "20"),
        ("damp", "0"),
        ("stiff", "0"),
        ("couple", "0"),
        ("body", "0"),
        ("buzz", "0"),
        ("tension", "0"),
        ("strum", "0"),
        ("courses", "3"),
    ]);
    let notes = [110.0, 164.8138, 220.0];
    let at = 9_600;
    let a = generator(at, [0.37, -0.52, 0.19], false);
    let b = generator(at, [0.61, 0.24, -0.44], true);
    let mut word = a.clone();
    word.extend(b.iter().copied());
    word.extend(inverted(&a));
    word.extend(inverted(&b));
    assert_eq!(word.len(), 12, "twelve authored gates");

    let dense = word.iter().fold(
        {
            let mut identity = [[0.0; 3]; 3];
            for (index, row) in identity.iter_mut().enumerate() {
                row[index] = 1.0;
            }
            identity
        },
        |accumulated, frame| {
            matmul(
                plane_rotation(frame.course_a, frame.course_b, frame.theta),
                accumulated,
            )
        },
    );
    // A balanced commutator of non-commuting generators is not the
    // identity — otherwise this test would pass on an empty word.
    let off_identity: f64 = (0..3)
        .map(|row| {
            (0..3)
                .map(|column| {
                    let expected = if row == column { 1.0 } else { 0.0 };
                    (dense[row][column] - expected).powi(2)
                })
                .sum::<f64>()
        })
        .sum();
    assert!(
        off_identity.sqrt() > 0.1,
        "the word must be a real rotation, Fro distance {}",
        off_identity.sqrt()
    );

    let (_, _, trace) = StringNetworkVoice::weave(&lossless, &notes, 0.6, 1.0)
        .render_traced_gestured(STRIDE, &[], &word)
        .expect("in-range word");
    assert_eq!(trace.applied.len(), 12);
    assert_eq!(trace.courses, 3, "three courses, all of them played");
    let before = trace.waves_before(0).expect("state before the word");
    let after = trace.waves_after(11).expect("state after the word");
    let start = [before[0], before[1], before[2]];
    let norm = (start.iter().map(|value| value * value).sum::<f64>()).sqrt();
    assert!(norm > 1e-6, "the courses must be ringing, norm {norm}");
    let predicted = apply(dense, start);
    let residual = (0..3)
        .map(|index| (predicted[index] - after[index]).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(
        residual <= 1e-8,
        "live state disagrees with the dense operator by {residual:e} (norm {norm})"
    );
    // The relative statement, which is the one that scales: the residual
    // is roundoff against the vector it rotates.
    assert!(
        residual / norm <= 1e-12,
        "relative residual {:e}",
        residual / norm
    );

    // The word's exact reversal-negation returns the state: the closed
    // loop the same-endpoint experiment leans on.
    let mut round_trip = word.clone();
    round_trip.extend(inverted(&word));
    let (_, _, closed) = StringNetworkVoice::weave(&lossless, &notes, 0.6, 1.0)
        .render_traced_gestured(STRIDE, &[], &round_trip)
        .expect("in-range word");
    let opened = closed.waves_before(0).expect("state before");
    let returned = closed.waves_after(23).expect("state after");
    let endpoint = (0..3)
        .map(|index| (opened[index] - returned[index]).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(
        endpoint <= 1e-12 * norm.max(1.0),
        "word then its reversal-negation must return the state, off by {endpoint:e}"
    );
}

/// Law 6: a host driving arbitrary block sizes gets the same render as
/// the offline one, gestures included. The seam lives inside
/// `next_sample`, so this is a law about where it lives.
#[test]
fn gestures_are_host_block_invariant() {
    let word = [
        gesture(0, 0, 1, 0.31),
        gesture(1, 2, 5, -0.42),
        gesture(4_801, 3, 8, 0.77),
        gesture(4_801, 8, 3, -0.15),
        gesture(12_345, 1, 6, 0.53),
        gesture(23_999, 4, 7, -0.61),
    ];
    let (expected, _, expected_trace) = weave_voice()
        .render_traced_gestured(STRIDE, &[], &word)
        .expect("in-range word");

    let mut voice = weave_voice();
    voice.arm_gestures(&word).expect("in-range word");
    let mut actual = Vec::new();
    for block_len in [1, 31, 64, 127, 509] {
        while !voice.is_finished() && actual.len() % 1_023 < 800 {
            let mut block = vec![0.0; block_len];
            let written = voice.render_block(&mut block);
            actual.extend_from_slice(&block[..written]);
        }
    }
    while !voice.is_finished() {
        let mut block = [0.0_f32; 113];
        let written = voice.render_block(&mut block);
        actual.extend_from_slice(&block[..written]);
    }
    assert_eq!(expected, actual, "the gesture seam is host-block invariant");
    assert_eq!(
        &expected_trace,
        voice.gesture_trace(),
        "and so is its receipt"
    );
}

/// Law 7a: out-of-range courses, a course rotated against itself, and a
/// non-finite angle are typed refusals — the word is rejected whole,
/// before any audio exists, and nothing is clamped into range.
#[test]
fn a_bad_gate_is_a_typed_refusal_not_a_clamp() {
    let courses = 9;
    let refuse = |frame: GestureFrame| -> GestureError {
        match weave_voice().render_traced_gestured(STRIDE, &[], &[gesture(0, 0, 1, 0.2), frame]) {
            Ok(_) => panic!("a bad gate must refuse, not render"),
            Err(error) => error,
        }
    };
    assert_eq!(
        refuse(gesture(100, courses, 1, 0.3)),
        GestureError::CourseOutOfRange {
            at_sample: 100,
            course: courses,
            courses,
        }
    );
    assert_eq!(
        refuse(gesture(100, 0, 9_999, 0.3)),
        GestureError::CourseOutOfRange {
            at_sample: 100,
            course: 9_999,
            courses,
        }
    );
    assert_eq!(
        refuse(gesture(7, 4, 4, 0.3)),
        GestureError::DegeneratePair {
            at_sample: 7,
            course: 4,
        }
    );
    assert_eq!(
        refuse(gesture(7, 4, 5, f64::NAN)).code(),
        "gesture-nonfinite-angle"
    );
    assert_eq!(
        refuse(gesture(7, 4, 5, f64::INFINITY)).code(),
        "gesture-nonfinite-angle"
    );
    assert_eq!(
        refuse(gesture(100, courses, 1, 0.3)).code(),
        "gesture-course-out-of-range"
    );
    // A refusal is a refusal at arming time: the voice renders nothing.
    let mut voice = weave_voice();
    assert!(voice.arm_gestures(&[gesture(0, 0, 40, 0.1)]).is_err());
    assert!(
        voice.gesture_trace().applied.is_empty(),
        "a refused word arms nothing"
    );
}

/// Law 7b: determinism, and the ordering contract that makes a word a
/// word — frames sort by sample, and frames sharing a sample keep the
/// order they were written in.
#[test]
fn gestured_renders_are_deterministic_and_order_faithful() {
    let word = [
        gesture(12_000, 0, 1, 0.44),
        gesture(3_000, 2, 3, -0.31),
        gesture(12_000, 1, 2, 0.28),
        gesture(3_000, 3, 4, 0.17),
    ];
    let render = |word: &[GestureFrame]| {
        weave_voice()
            .render_traced_gestured(STRIDE, &[], word)
            .expect("in-range word")
    };
    let (first, _, first_trace) = render(&word);
    let (again, _, again_trace) = render(&word);
    assert_eq!(first, again, "same word, same bytes");
    assert_eq!(first_trace, again_trace, "same word, same receipt");

    // Sorted by sample, stable within a sample.
    let executed: Vec<(usize, usize, usize)> = first_trace
        .applied
        .iter()
        .map(|gate| (gate.at_sample, gate.course_a, gate.course_b))
        .collect();
    assert_eq!(
        executed,
        vec![(3_000, 2, 3), (3_000, 3, 4), (12_000, 0, 1), (12_000, 1, 2)]
    );

    // Writing the word already sorted changes nothing.
    let mut sorted = word;
    sorted.sort_by_key(|frame| frame.at_sample);
    assert_eq!(render(&sorted).0, first, "sorting is not a transformation");

    // Swapping two OVERLAPPING gates inside one sample does change it —
    // the stable sort is load-bearing, not decorative.
    let swapped = [word[1], word[3], word[2], word[0]];
    assert_ne!(
        render(&swapped).0,
        first,
        "order within a sample is meaning"
    );
}

/// The phase's shared rule, checked rather than assumed: a gesture word
/// does not open a hole in the Phase 0 §B energy ledger. Gates are exact
/// rotations on the scattering state, so their work lands in
/// `work_scatter` at roundoff and the ledger still closes.
#[test]
fn the_energy_ledger_still_closes_with_a_word_armed() {
    let word: Vec<GestureFrame> = (0..24)
        .map(|step: usize| {
            gesture(
                step * 700 + 100,
                step % 9,
                (step * 4 + 1) % 9,
                0.7 * ((step as f64) * 0.9).cos(),
            )
        })
        .filter(|frame| frame.course_a != frame.course_b)
        .collect();
    let mut voice = weave_voice();
    voice.arm_gestures(&word).expect("in-range word");
    let (audio, _, energy) = voice.render_traced_observed(STRIDE, &[]);
    assert!(audio.iter().all(|sample| sample.is_finite()));
    let residual = energy.ledger_residual();
    assert!(
        residual.abs() <= 1.0e-9,
        "ledger residual {residual:e} with a gesture word armed"
    );
    let scatter_work = energy.last("work_scatter").expect("work_scatter");
    assert!(
        scatter_work.abs() <= 1.0e-9,
        "gesture work is roundoff, measured {scatter_work:e}"
    );
}
