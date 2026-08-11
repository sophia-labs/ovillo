//! Invariants for the post-Karplus string network.
//!
//! There is no external golden vector for original synthesis work, so the
//! tests assert the promises that matter: deterministic identity, tuning,
//! T60, spatial pluck physics, dispersion, tension relaxation, physical
//! bends, block/stream parity, audible topology, and boundedness.

use std::collections::BTreeMap;

use mus_dsp::pluck::{
    pluck_note, pluck_note_sung, weave_note, ControlFrame, ControlTarget, StringNetworkVoice,
};
use mus_dsp::SR_F64;

fn params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn isolated(extra: &[(&str, &str)]) -> BTreeMap<String, String> {
    let mut patch = params(&[
        ("synth", "pluck"),
        ("body", "0"),
        ("symp", "0"),
        ("stiff", "0"),
        ("tension", "0"),
        ("buzz", "0"),
        ("detune", "0"),
        ("strum", "0"),
    ]);
    for (key, value) in extra {
        patch.insert((*key).to_string(), (*value).to_string());
    }
    patch
}

fn rms(values: &[f32]) -> f64 {
    (values
        .iter()
        .map(|&value| (value as f64) * (value as f64))
        .sum::<f64>()
        / values.len().max(1) as f64)
        .sqrt()
}

fn autocorrelation_f0(values: &[f32], lo_hz: f64, hi_hz: f64) -> f64 {
    let min_lag = (SR_F64 / hi_hz) as usize;
    let max_lag = (SR_F64 / lo_hz) as usize;
    let mut best = (f64::NEG_INFINITY, min_lag);
    for lag in min_lag..=max_lag.min(values.len() / 2) {
        let correlation = values[..values.len() - lag]
            .iter()
            .zip(values[lag..].iter())
            .map(|(&left, &right)| left as f64 * right as f64)
            .sum::<f64>();
        if correlation > best.0 {
            best = (correlation, lag);
        }
    }
    let correlation = |lag: usize| -> f64 {
        values[..values.len() - lag]
            .iter()
            .zip(values[lag..].iter())
            .map(|(&left, &right)| left as f64 * right as f64)
            .sum()
    };
    let lag = best.1;
    if lag <= min_lag || lag + 1 >= max_lag {
        return SR_F64 / lag as f64;
    }
    let (a, b, c) = (correlation(lag - 1), correlation(lag), correlation(lag + 1));
    let denominator = a - 2.0 * b + c;
    let refinement = if denominator.abs() > 1e-12 {
        0.5 * (a - c) / denominator
    } else {
        0.0
    };
    SR_F64 / (lag as f64 + refinement)
}

fn estimate_f0(values: &[f32], start_s: f64, end_s: f64, lo_hz: f64, hi_hz: f64) -> f64 {
    let start = (start_s * SR_F64) as usize;
    let end = ((end_s * SR_F64) as usize).min(values.len());
    autocorrelation_f0(&values[start..end], lo_hz, hi_hz)
}

fn goertzel(values: &[f32], frequency: f64) -> f64 {
    let omega = 2.0 * std::f64::consts::PI * frequency / SR_F64;
    let coefficient = 2.0 * omega.cos();
    let (mut state_1, mut state_2) = (0.0_f64, 0.0_f64);
    for &value in values {
        let state_0 = value as f64 + coefficient * state_1 - state_2;
        state_2 = state_1;
        state_1 = state_0;
    }
    state_1 * state_1 + state_2 * state_2 - coefficient * state_1 * state_2
}

fn spectral_peak(values: &[f32], center_hz: f64, fractional_span: f64) -> f64 {
    let steps = 120;
    let mut best = (f64::NEG_INFINITY, center_hz);
    for step in 0..=steps {
        let position = step as f64 / steps as f64;
        let frequency = center_hz * (1.0 - fractional_span + 2.0 * fractional_span * position);
        let power = goertzel(values, frequency);
        if power > best.0 {
            best = (power, frequency);
        }
    }
    best.1
}

#[test]
fn deterministic_and_content_keyed() {
    let patch = isolated(&[]);
    let a = pluck_note(&patch, &[110.0], 1.0, 1.0);
    let b = pluck_note(&patch, &[110.0], 1.0, 1.0);
    assert_eq!(a, b, "same note renders byte-identically");
    let c = pluck_note(&patch, &[110.5], 1.0, 1.0);
    assert_ne!(
        a, c,
        "different note content gets a different contact state"
    );
}

#[test]
fn streaming_blocks_equal_offline_render() {
    let patch = params(&[
        ("synth", "pluck"),
        ("sus", "1.2"),
        ("body", "0.45"),
        ("symp", "0.2"),
        ("stiff", "0.13"),
        ("tension", "8"),
    ]);
    let expected = pluck_note(&patch, &[110.0, 164.8138], 0.7, 1.0);
    let mut voice = StringNetworkVoice::guitar(&patch, &[110.0, 164.8138], 0.7, 1.0);
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
    assert_eq!(expected, actual, "the DSP kernel is host-block invariant");
}

#[test]
fn in_tune_across_the_guitar_neck() {
    let patch = isolated(&[("sus", "3.0"), ("damp", "0.2")]);
    for target in [82.4069, 110.0, 220.0, 440.0, 880.0, 1318.51] {
        let audio = pluck_note(&patch, &[target], 1.2, 1.0);
        let estimate = estimate_f0(&audio, 0.16, 0.56, target * 0.80, target * 1.25);
        let cents = 1200.0 * (estimate / target).log2().abs();
        assert!(
            cents < 8.0,
            "{target} Hz estimated {estimate:.3} Hz: {cents:.2} cents"
        );
    }
}

#[test]
fn sus_remains_an_honest_fundamental_t60() {
    let patch = isolated(&[("sus", "0.8"), ("damp", "0")]);
    let audio = pluck_note(&patch, &[196.0], 1.6, 1.0);
    let early = rms(&audio[(0.06 * SR_F64) as usize..(0.16 * SR_F64) as usize]);
    let at_t60 = rms(&audio[(0.78 * SR_F64) as usize..(0.88 * SR_F64) as usize]);
    let decibels = 20.0 * (at_t60 / early).log10();
    assert!(
        (-76.0..=-40.0).contains(&decibels),
        "T60=0.8s measured {decibels:.1} dB relative to early"
    );
}

#[test]
fn midpoint_pluck_suppresses_the_second_harmonic() {
    let f0 = 220.0;
    let near_bridge = pluck_note(
        &isolated(&[("pos", "0.07"), ("pick", "0")]),
        &[f0],
        1.0,
        1.0,
    );
    let midpoint = pluck_note(&isolated(&[("pos", "0.5"), ("pick", "0")]), &[f0], 1.0, 1.0);
    let start = (0.04 * SR_F64) as usize;
    let end = (0.55 * SR_F64) as usize;
    let ratio_bridge = goertzel(&near_bridge[start..end], 2.0 * f0)
        / goertzel(&near_bridge[start..end], f0).max(1e-12);
    let ratio_midpoint =
        goertzel(&midpoint[start..end], 2.0 * f0) / goertzel(&midpoint[start..end], f0).max(1e-12);
    assert!(
        ratio_midpoint < ratio_bridge * 0.12,
        "2nd/1st power: midpoint {ratio_midpoint:.5}, bridge {ratio_bridge:.5}"
    );
}

#[test]
fn stiff_string_spreads_upper_partials_without_detuning_the_fundamental() {
    let plain = pluck_note(
        &isolated(&[("sus", "3"), ("damp", "0.12"), ("stiff", "0")]),
        &[110.0],
        1.4,
        1.0,
    );
    let stiff = pluck_note(
        &isolated(&[("sus", "3"), ("damp", "0.12"), ("stiff", "0.75")]),
        &[110.0],
        1.4,
        1.0,
    );
    let start = (0.12 * SR_F64) as usize;
    let end = (0.95 * SR_F64) as usize;
    let plain_window = &plain[start..end];
    let stiff_window = &stiff[start..end];
    let plain_1 = spectral_peak(plain_window, 110.0, 0.02);
    let plain_5 = spectral_peak(plain_window, 550.0, 0.025);
    let stiff_1 = spectral_peak(stiff_window, 110.0, 0.02);
    let stiff_5 = spectral_peak(stiff_window, 550.0, 0.04);
    let plain_ratio = plain_5 / (5.0 * plain_1);
    let stiff_ratio = stiff_5 / (5.0 * stiff_1);
    assert!(
        stiff_ratio > plain_ratio + 0.00035,
        "upper-partial ratio plain={plain_ratio:.6}, stiff={stiff_ratio:.6}"
    );
    let fundamental_cents = 1200.0 * (stiff_1 / 110.0).log2().abs();
    assert!(
        fundamental_cents < 10.0,
        "fundamental moved {fundamental_cents:.2} cents"
    );
}

#[test]
fn nonlinear_tension_glide_relaxes_toward_nominal_pitch() {
    let patch = isolated(&[("sus", "3.0"), ("damp", "0.18"), ("tension", "32")]);
    let audio = pluck_note(&patch, &[110.0], 1.5, 1.0);
    let early = estimate_f0(&audio, 0.035, 0.19, 105.0, 116.0);
    let late = estimate_f0(&audio, 0.85, 1.25, 105.0, 116.0);
    let glide_cents = 1200.0 * (early / late).log2();
    let late_error = 1200.0 * (late / 110.0).log2().abs();
    assert!(
        glide_cents > 5.0,
        "early={early:.3}, late={late:.3}, glide={glide_cents:.2}c"
    );
    assert!(
        late_error < 9.0,
        "late pitch remains {late_error:.2} cents from nominal"
    );
}

#[test]
fn body_is_feedback_coupling_not_a_post_eq_alias() {
    let dry = pluck_note(
        &isolated(&[("body", "0"), ("sus", "2.5"), ("pos", "0.19")]),
        &[137.0],
        1.0,
        1.0,
    );
    let coupled = pluck_note(
        &isolated(&[
            ("body", "0.9"),
            ("body_size", "1.0"),
            ("sus", "2.5"),
            ("pos", "0.19"),
        ]),
        &[137.0],
        1.0,
        1.0,
    );
    assert_ne!(dry, coupled);
    let start = (0.04 * SR_F64) as usize;
    let end = (0.55 * SR_F64) as usize;
    let dry_body_mode = goertzel(&dry[start..end], 186.0);
    let coupled_body_mode = goertzel(&coupled[start..end], 186.0);
    assert!(
        coupled_body_mode > dry_body_mode * 1.15,
        "186 Hz body mode dry={dry_body_mode:.3e}, coupled={coupled_body_mode:.3e}"
    );
}

#[test]
fn bends_retime_the_waveguide_and_land_on_the_target() {
    let patch = isolated(&[("sus", "4"), ("damp", "0.18")]);
    let audio = pluck_note(&patch, &[220.0], 1.3, 1.5);
    let estimate = estimate_f0(&audio, 1.42, 1.82, 260.0, 380.0);
    let cents = 1200.0 * (estimate / 330.0).log2().abs();
    assert!(
        cents < 45.0,
        "tail={estimate:.2} Hz, target=330 Hz, error={cents:.1}c"
    );
}

#[test]
fn weave_is_path_sensitive_but_deterministic() {
    let clockwise_patch = params(&[
        ("synth", "weave"),
        ("sus", "2.2"),
        ("courses", "9"),
        ("couple", "0.16"),
        ("chirality", "0.9"),
        ("orbit", "0.47"),
        ("orbit_depth", "0.7"),
        ("curvature", "0.55"),
        ("body", "0.55"),
    ]);
    let counter_patch = {
        let mut patch = clockwise_patch.clone();
        patch.insert("chirality".to_string(), "-0.9".to_string());
        patch
    };
    let a = weave_note(&clockwise_patch, &[110.0, 165.0], 1.1, 1.0);
    let b = weave_note(&clockwise_patch, &[110.0, 165.0], 1.1, 1.0);
    let reversed = weave_note(&counter_patch, &[110.0, 165.0], 1.1, 1.0);
    assert_eq!(a, b);
    assert_ne!(
        a, reversed,
        "reversing the ordered coupling path changes the state"
    );
    let difference = a
        .iter()
        .zip(reversed.iter())
        .map(|(&left, &right)| (left as f64 - right as f64).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(difference > 0.05, "path difference norm={difference}");
}

#[test]
fn bounded_and_finite_over_extreme_controls() {
    let cases = [
        params(&[
            ("synth", "pluck"),
            ("sus", "0.05"),
            ("damp", "0"),
            ("pos", "0.02"),
            ("pick", "1"),
            ("stiff", "1"),
            ("tension", "80"),
            ("buzz", "1"),
            ("body", "1"),
            ("symp", "1"),
            ("detune", "60"),
        ]),
        params(&[
            ("synth", "weave"),
            ("sus", "5"),
            ("damp", "0.9"),
            ("courses", "24"),
            ("couple", "0.45"),
            ("chirality", "1"),
            ("orbit", "20"),
            ("orbit_depth", "1"),
            ("curvature", "1"),
            ("body", "1"),
        ]),
    ];
    for patch in &cases {
        let audio = if patch.get("synth").map(String::as_str) == Some("weave") {
            weave_note(patch, &[82.4069, 110.0, 146.832], 0.45, 1.0)
        } else {
            pluck_note(patch, &[82.4069, 110.0, 146.832], 0.45, 1.0)
        };
        assert!(audio.iter().all(|sample| sample.is_finite()));
        let peak = audio
            .iter()
            .fold(0.0_f32, |maximum, sample| maximum.max(sample.abs()));
        assert!(peak <= 4.0, "network peak {peak} is bounded");
        let bent = if patch.get("synth").map(String::as_str) == Some("weave") {
            weave_note(patch, &[82.4069, 110.0, 146.832], 0.45, 1.5)
        } else {
            pluck_note(patch, &[82.4069, 110.0, 146.832], 0.45, 1.5)
        };
        assert!(bent.iter().all(|sample| sample.is_finite()));
        let bent_peak = bent
            .iter()
            .fold(0.0_f32, |maximum, sample| maximum.max(sample.abs()));
        // Delay-line retuning during a bend performs work on the state —
        // the predicted gap in the rotation proof, measured: this exact
        // weave patch peaks 3.24 unbent and 5.38 during an upward-fifth
        // bend (a bounded ~0.3 s transient that then decays to silence;
        // the fretting hand doing work on a shortening string is real
        // physics). Bound is measurement + headroom, not a passivity
        // claim; energy-compensated retuning is the P0 follow-up.
        assert!(bent_peak <= 8.0, "bent network peak {bent_peak} is bounded");
    }
}

#[test]
fn zero_limit_laws_decouple_the_network() {
    // couple=0 with body=0: every scattering-path control is inert — the
    // decoupled network is nothing but independent courses.
    let base = params(&[
        ("synth", "weave"),
        ("body", "0"),
        ("couple", "0"),
        ("courses", "9"),
        ("sus", "1.0"),
    ]);
    let reference = weave_note(&base, &[110.0, 165.0], 0.5, 1.0);
    for (key, value) in [
        ("chirality", "-0.9"),
        ("orbit", "7.0"),
        ("orbit_depth", "0.9"),
        ("curvature", "0.9"),
        ("dimension", "2.4"),
    ] {
        let mut patch = base.clone();
        patch.insert(key.to_string(), value.to_string());
        let altered = weave_note(&patch, &[110.0, 165.0], 0.5, 1.0);
        assert_eq!(
            reference, altered,
            "{key} must be inert when couple=0 and body=0"
        );
    }

    // orbit_depth=0: the travelling field's rate is inert even while the
    // network is coupled and the body is live.
    let still = params(&[
        ("synth", "weave"),
        ("body", "0.4"),
        ("couple", "0.2"),
        ("orbit_depth", "0"),
        ("orbit", "0.1"),
        ("courses", "9"),
        ("sus", "1.0"),
    ]);
    let mut spinning = still.clone();
    spinning.insert("orbit".to_string(), "19.0".to_string());
    assert_eq!(
        weave_note(&still, &[110.0, 165.0], 0.5, 1.0),
        weave_note(&spinning, &[110.0, 165.0], 0.5, 1.0),
        "orbit must be inert when orbit_depth=0"
    );
}

#[test]
fn traced_render_is_byte_identical_and_shaped() {
    let patch = params(&[
        ("synth", "weave"),
        ("sus", "1.2"),
        ("courses", "9"),
        ("couple", "0.2"),
        ("body", "0.4"),
        ("orbit", "0.5"),
    ]);
    let plain = weave_note(&patch, &[110.0, 165.0], 0.6, 1.0);
    let (traced, trace) =
        StringNetworkVoice::weave(&patch, &[110.0, 165.0], 0.6, 1.0).render_traced(256);
    assert_eq!(plain, traced, "the trace is a tap, never a participant");
    assert_eq!(trace.courses, 9);
    assert_eq!(trace.played, 2);
    assert_eq!(trace.frames, plain.len().div_ceil(256));
    assert_eq!(trace.data.len(), trace.frames * (2 * trace.courses + 1));
    assert_eq!(trace.course_freqs.len(), trace.courses);
    assert!(trace.data.iter().all(|value| value.is_finite()));
    assert!(
        trace.data.iter().any(|value| *value > 0.0),
        "energy telemetry is live"
    );
}

#[test]
fn sing_zero_limit_and_offset_law() {
    let external: Vec<f32> = (0..48_000)
        .map(|i| (0.4 * (2.0 * std::f64::consts::PI * 220.0 * i as f64 / SR_F64).sin()) as f32)
        .collect();
    // sing=0: byte-identical whether or not a sample is attached.
    let patch = isolated(&[("sus", "1.5")]);
    assert_eq!(
        pluck_note(&patch, &[220.0], 0.5, 1.0),
        pluck_note_sung(&patch, &[220.0], 0.5, 1.0, &external),
        "sing=0 must not touch the render"
    );
    // sing_at offset equals pre-slicing the take by the same amount.
    let sung = isolated(&[("sus", "1.5"), ("sing", "1")]);
    let mut offset = sung.clone();
    offset.insert("sing_at".to_string(), "0.25".to_string());
    assert_eq!(
        pluck_note_sung(&offset, &[220.0], 0.5, 1.0, &external),
        pluck_note_sung(&sung, &[220.0], 0.5, 1.0, &external[12_000..]),
        "sing_at is exactly a read offset"
    );
}

#[test]
fn sung_drive_is_selective_and_silent_safe() {
    // sing=1 removes the pluck burst entirely: silence in, silence out.
    let sung = isolated(&[("sing", "1"), ("sus", "2")]);
    let zeros = vec![0.0_f32; 48_000];
    let silent = pluck_note_sung(&sung, &[220.0], 0.5, 1.0, &zeros);
    assert!(silent.iter().all(|sample| sample.abs() < 1e-9));

    // Fretting works: driving AT the course frequency rings the post-
    // drive tail far more than driving between courses.
    let tone = |frequency: f64| -> Vec<f32> {
        (0..24_000)
            .map(|i| {
                (0.3 * (2.0 * std::f64::consts::PI * frequency * i as f64 / SR_F64).sin()) as f32
            })
            .collect()
    };
    let on = pluck_note_sung(&sung, &[220.0], 0.5, 1.0, &tone(220.0));
    let off = pluck_note_sung(&sung, &[220.0], 0.5, 1.0, &tone(303.0));
    assert!(on.iter().chain(off.iter()).all(|sample| sample.is_finite()));
    let tail = |y: &[f32]| rms(&y[(0.55 * SR_F64) as usize..(0.9 * SR_F64) as usize]);
    assert!(
        tail(&on) > tail(&off) * 4.0,
        "resonant tail on={:.6} off={:.6}",
        tail(&on),
        tail(&off)
    );
}

#[test]
fn trajectory_laws() {
    let patch = params(&[
        ("synth", "weave"),
        ("sus", "1.4"),
        ("courses", "9"),
        ("couple", "0.11"),
        ("chirality", "0.4"),
        ("body", "0.4"),
        ("orbit", "0.5"),
    ]);
    // Law 1: no trajectory is exactly an empty trajectory.
    let plain = weave_note(&patch, &[110.0, 165.0], 0.6, 1.0);
    let (empty, _) =
        StringNetworkVoice::weave(&patch, &[110.0, 165.0], 0.6, 1.0).render_traced_with(256, &[]);
    assert_eq!(plain, empty);

    // Law 2: a frame at sample 0 equals constructing with those values,
    // for the pure-patch controls.
    let mut retuned = patch.clone();
    retuned.insert("couple".to_string(), "0.22".to_string());
    retuned.insert("chirality".to_string(), "-0.6".to_string());
    let constructed = weave_note(&retuned, &[110.0, 165.0], 0.6, 1.0);
    let frame = ControlFrame {
        at_sample: 0,
        sets: vec![
            (ControlTarget::Couple, 0.22),
            (ControlTarget::Chirality, -0.6),
        ],
    };
    let (framed, _) = StringNetworkVoice::weave(&patch, &[110.0, 165.0], 0.6, 1.0)
        .render_traced_with(256, std::slice::from_ref(&frame));
    assert_eq!(constructed, framed, "frame-at-zero equals construction");

    // Law 3: trajectory renders are deterministic.
    let (again, _) = StringNetworkVoice::weave(&patch, &[110.0, 165.0], 0.6, 1.0)
        .render_traced_with(256, &[frame]);
    assert_eq!(framed, again);
}

#[test]
fn closed_loop_commutator_is_path_dependent() {
    // The E15 primitive: two control loops with IDENTICAL endpoints —
    // every retargeted scalar returns to its initial value — differing
    // only in the order of operations. The vibrating state need not
    // return: discrete ordered-product holonomy, inside one note.
    let patch = params(&[
        ("synth", "weave"),
        ("sus", "2.0"),
        ("courses", "9"),
        ("couple", "0.12"),
        ("chirality", "0.0"),
        ("body", "0.45"),
        ("orbit", "0.4"),
        ("orbit_depth", "0.5"),
    ]);
    let s = |t: f64| (t * SR_F64) as usize;
    let a_then_b = vec![
        ControlFrame {
            at_sample: s(0.20),
            sets: vec![(ControlTarget::Couple, 0.34)],
        },
        ControlFrame {
            at_sample: s(0.45),
            sets: vec![(ControlTarget::Chirality, 0.85)],
        },
        ControlFrame {
            at_sample: s(0.70),
            sets: vec![(ControlTarget::Couple, 0.12)],
        },
        ControlFrame {
            at_sample: s(0.95),
            sets: vec![(ControlTarget::Chirality, 0.0)],
        },
    ];
    let b_then_a = vec![
        ControlFrame {
            at_sample: s(0.20),
            sets: vec![(ControlTarget::Chirality, 0.85)],
        },
        ControlFrame {
            at_sample: s(0.45),
            sets: vec![(ControlTarget::Couple, 0.34)],
        },
        ControlFrame {
            at_sample: s(0.70),
            sets: vec![(ControlTarget::Chirality, 0.0)],
        },
        ControlFrame {
            at_sample: s(0.95),
            sets: vec![(ControlTarget::Couple, 0.12)],
        },
    ];
    let render = |trajectory: &[ControlFrame]| {
        StringNetworkVoice::weave(&patch, &[110.0, 165.0], 1.2, 1.0)
            .render_traced_with(256, trajectory)
            .0
    };
    let ab = render(&a_then_b);
    let ba = render(&b_then_a);
    assert!(ab.iter().chain(ba.iter()).all(|v| v.is_finite()));
    let peak = ab
        .iter()
        .chain(ba.iter())
        .fold(0.0_f32, |m, v| m.max(v.abs()));
    assert!(peak <= 8.0, "closed loops stay bounded: {peak}");
    let diff = ab
        .iter()
        .zip(ba.iter())
        .map(|(x, y)| (x - y) as f64 * (x - y) as f64)
        .sum::<f64>()
        .sqrt();
    let scale = ab.iter().map(|v| *v as f64 * *v as f64).sum::<f64>().sqrt();
    println!(
        "closed-loop path distance: {:.4} (signal norm {:.4})",
        diff, scale
    );
    assert!(
        diff > scale * 0.01,
        "equal-endpoint loops must differ: diff={diff}, scale={scale}"
    );
}

// ---------------------------------------------------------- PW10 §C: breath

#[test]
fn breath_zero_limit_is_byte_identical() {
    // A busy patch (body, buzz, sing all engaged) so the zero-limit law is
    // tested alongside the rest of the excitation stack, not in isolation.
    let external: Vec<f32> = (0..24_000)
        .map(|i| (0.3 * (2.0 * std::f64::consts::PI * 220.0 * i as f64 / SR_F64).sin()) as f32)
        .collect();
    let patch = isolated(&[
        ("sus", "1.5"),
        ("body", "0.4"),
        ("buzz", "0.3"),
        ("sing", "0.2"),
    ]);
    let baseline = pluck_note_sung(&patch, &[220.0], 0.5, 1.0, &external);
    let mut with_key = patch.clone();
    with_key.insert("breath".to_string(), "0".to_string());
    assert_eq!(
        baseline,
        pluck_note_sung(&with_key, &[220.0], 0.5, 1.0, &external),
        "breath=0 must not touch the render"
    );
}

#[test]
fn breath_is_deterministic_and_content_keyed() {
    let patch = isolated(&[("breath", "0.8"), ("sus", "1.2")]);
    let a = pluck_note(&patch, &[220.0], 0.6, 1.0);
    let b = pluck_note(&patch, &[220.0], 0.6, 1.0);
    assert_eq!(a, b, "same breathed note renders byte-identically");
    let c = pluck_note(&patch, &[220.5], 0.6, 1.0);
    assert_ne!(
        a, c,
        "different note content changes the breath noise stream too"
    );
}

#[test]
fn breath_sustains_energy_through_the_slot() {
    // sus=0.4 makes the unbreathed string nearly silent well before the
    // 1.0s slot ends; breath keeps injecting fresh noise the whole time.
    let slot = 1.0;
    let unbreathed = pluck_note(
        &isolated(&[("sus", "0.4"), ("damp", "0.3")]),
        &[220.0],
        slot,
        1.0,
    );
    let breathed = pluck_note(
        &isolated(&[("sus", "0.4"), ("damp", "0.3"), ("breath", "0.8")]),
        &[220.0],
        slot,
        1.0,
    );
    let late = |audio: &[f32]| rms(&audio[(0.75 * SR_F64) as usize..(0.95 * SR_F64) as usize]);
    let unbreathed_late = late(&unbreathed);
    let breathed_late = late(&breathed);
    assert!(
        breathed_late > unbreathed_late * 3.0,
        "late-slot RMS unbreathed={unbreathed_late:.8}, breathed={breathed_late:.8}"
    );
}

#[test]
fn breath_stops_at_the_slot_and_decays_to_silence() {
    let slot = 0.3;
    let patch = isolated(&[("sus", "1.5"), ("damp", "0.2"), ("breath", "0.9")]);
    let audio = pluck_note(&patch, &[220.0], slot, 1.0);
    let near_slot_end = rms(&audio[(0.20 * SR_F64) as usize..(0.29 * SR_F64) as usize]);
    let tail_start = audio.len() - (0.08 * SR_F64) as usize;
    let final_tail = rms(&audio[tail_start..]);
    let decibels = 20.0 * (final_tail / near_slot_end.max(1e-12)).log10();
    assert!(
        decibels < -20.0,
        "post-slot tail should be well below the sustained in-slot level: {decibels:.1} dB \
         (near_slot_end={near_slot_end:.6}, final_tail={final_tail:.6})"
    );
}

#[test]
fn breath_stays_bounded_at_extremes() {
    let patch = isolated(&[
        ("sus", "0.1"),
        ("damp", "0"),
        ("breath", "1"),
        ("pick", "1"),
        ("buzz", "1"),
        ("body", "1"),
    ]);
    let audio = pluck_note(&patch, &[82.4069, 110.0, 146.832], 0.5, 1.0);
    assert!(audio.iter().all(|sample| sample.is_finite()));
    let peak = audio
        .iter()
        .fold(0.0_f32, |maximum, sample| maximum.max(sample.abs()));
    assert!(peak <= 8.0, "breath extreme peak {peak} is bounded");
}

#[test]
fn breath_composes_with_sing_and_pluck_and_stays_bounded() {
    // No hidden priority: sing's external drive, breath's noise, and the
    // (scaled-down but nonzero) pluck burst all sum at the same
    // excitation point.
    let external: Vec<f32> = (0..48_000)
        .map(|i| (0.4 * (2.0 * std::f64::consts::PI * 220.0 * i as f64 / SR_F64).sin()) as f32)
        .collect();
    let patch = isolated(&[("breath", "0.6"), ("sing", "0.5"), ("sus", "1.0")]);
    let audio = pluck_note_sung(&patch, &[220.0], 0.6, 1.0, &external);
    assert!(audio.iter().all(|sample| sample.is_finite()));
    let peak = audio
        .iter()
        .fold(0.0_f32, |maximum, sample| maximum.max(sample.abs()));
    assert!(
        peak <= 8.0,
        "breath+sing+pluck composition peak {peak} is bounded"
    );
}
