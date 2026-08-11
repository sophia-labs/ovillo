//! Invariants for the post-Karplus string network.
//!
//! There is no external golden vector for original synthesis work, so the
//! tests assert the promises that matter: deterministic identity, tuning,
//! T60, spatial pluck physics, dispersion, tension relaxation, physical
//! bends, block/stream parity, audible topology, and boundedness.
//!
//! The bow (PW10 §B) is the one excitation that is an energy SOURCE, so
//! its laws are stated differently: sustain, release and boundedness are
//! measured, the numbers are recorded in the assertions, and the region
//! where the v1 friction curve does NOT hold is asserted as a refusal
//! rather than quietly avoided.

use std::collections::BTreeMap;

use mus_dsp::body_profiles;
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

/// How periodic a window is at `f0`: power summed over the first ten
/// harmonics against power summed at the ten half-way points between them.
/// A plucked onset measures ~4.4e3; broadband chatter measures ~1.
fn tonality(values: &[f32], f0: f64) -> f64 {
    let harmonic: f64 = (1..=10).map(|k| goertzel(values, k as f64 * f0)).sum();
    let between: f64 = (1..=10)
        .map(|k| goertzel(values, (k as f64 + 0.5) * f0))
        .sum::<f64>()
        .max(1e-30);
    harmonic / between
}

fn peak(values: &[f32]) -> f32 {
    values
        .iter()
        .fold(0.0_f32, |maximum, value| maximum.max(value.abs()))
}

fn window(values: &[f32], start_s: f64, end_s: f64) -> &[f32] {
    let start = ((start_s * SR_F64) as usize).min(values.len());
    let end = ((end_s * SR_F64) as usize).min(values.len());
    &values[start..end.max(start)]
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
        // This exact weave patch peaks 3.24 unbent and 5.38 during an
        // upward-fifth bend — a bounded ~0.3 s transient that then decays
        // to silence. The bound is measurement + headroom, not a
        // passivity claim.
        //
        // CORRECTED (Ariadne Phase 0 §C,
        // `.string-research/PHASE0-TRANSIENT-REPORT.md`). This comment
        // used to attribute the transient to retuning work — "the
        // fretting hand doing work on a shortening string". Instrumenting
        // the network falsified that: over this render the retuning work
        // is NEGATIVE (-3.32 allpass, -20.44 delay line, declared
        // Euclidean metric), total stored energy FALLS 1.3% across the
        // peak, and the delay-line energy is flat to 1.2%. The transient
        // is a body-resonance sweep — the glide drags the partials across
        // the fixed modal body, roughly doubling body-mode energy, and
        // `body=0` removes 98.9% of the excess. Repairing the allpass
        // transport (`retune=neutral_filters`) moves the peak by -3.6%.
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

// ---------------------------------------------------------------- the bow
//
// PW10 §B. The bow is an energy SOURCE: none of the rotation/passivity
// arguments above reach it, and nothing below claims they do. These laws
// are empirical, and the domain they hold on is declared.

#[test]
fn bow_zero_limit_is_byte_identical() {
    // Hard zero limit: with bow = 0 the friction port must not exist, at
    // any bow_speed, in either voice. Byte identity, not a tolerance.
    let held = &[("sus", "1.5")];
    let reference = pluck_note(&isolated(held), &[196.0], 0.6, 1.0);
    for (key, value) in [
        ("bow", "0"),
        ("bow_speed", "0"),
        ("bow_speed", "0.5"),
        ("bow_speed", "1"),
    ] {
        let mut patch = isolated(held);
        patch.insert(key.to_string(), value.to_string());
        assert_eq!(
            reference,
            pluck_note(&patch, &[196.0], 0.6, 1.0),
            "{key}={value} must be inert while bow = 0"
        );
    }
    let woven = params(&[
        ("synth", "weave"),
        ("sus", "1.2"),
        ("courses", "9"),
        ("couple", "0.2"),
        ("body", "0.4"),
    ]);
    let mut bowless = woven.clone();
    bowless.insert("bow".to_string(), "0".to_string());
    bowless.insert("bow_speed".to_string(), "0.9".to_string());
    assert_eq!(
        weave_note(&woven, &[110.0, 165.0], 0.5, 1.0),
        weave_note(&bowless, &[110.0, 165.0], 0.5, 1.0),
        "the weave voice keeps the same zero limit"
    );
}

#[test]
fn bow_holds_a_tone_the_pluck_has_already_lost() {
    // 196 Hz, T60 = 1.2 s, a 2 s slot: by 1.6 s the plucked string is 60 dB
    // down and still falling, while the bow is still driving.
    let held = &[("sus", "1.2")];
    let dry = pluck_note(&isolated(held), &[196.0], 2.0, 1.0);
    let bowed = pluck_note(
        &isolated(&[("sus", "1.2"), ("bow", "0.6"), ("bow_speed", "0.5")]),
        &[196.0],
        2.0,
        1.0,
    );
    let late_dry = rms(window(&dry, 1.6, 1.95));
    let late_bowed = rms(window(&bowed, 1.6, 1.95));
    let early_bowed = rms(window(&bowed, 0.6, 0.95));
    let factor = late_bowed / late_dry.max(1e-12);
    println!(
        "bow sustain: late bowed={late_bowed:.6}, late dry={late_dry:.6}, \
         factor={factor:.1}, early bowed={early_bowed:.6}"
    );
    // MEASURED: dry 5.02e-6, bowed 2.60e-2 — a factor of 5179.
    assert!(
        factor > 1_000.0,
        "bowed/un-bowed late RMS factor {factor:.1} (measured 5179)"
    );
    // Held, not merely louder: the late window is no quieter than the
    // early one (measured late/early = 1.00 — a flat sustain).
    assert!(
        late_bowed > early_bowed * 0.7,
        "late={late_bowed:.6} early={early_bowed:.6} — the tone must hold"
    );
    // And it is a TONE, not sustained noise: harmonic power dominates.
    let periodicity = tonality(window(&bowed, 1.5, 1.95), 196.0);
    println!("bow tonality: {periodicity:.3e}");
    assert!(
        periodicity > 1.0e3,
        "harmonic/between-harmonic power {periodicity:.3e} (measured 3.2e4)"
    );

    // DECLARED DOMAIN, refused rather than smoothed over: below the
    // minimum bow force the string does not speak. A light bow drawn fast
    // only slows the pluck's decay — Schelleng's lower bound, emergent.
    let too_light = pluck_note(
        &isolated(&[("sus", "1.2"), ("bow", "0.1"), ("bow_speed", "1")]),
        &[196.0],
        2.0,
        1.0,
    );
    let light_factor = rms(window(&too_light, 1.6, 1.95)) / late_dry.max(1e-12);
    println!("below minimum bow force: factor={light_factor:.2}");
    assert!(
        light_factor < 100.0,
        "bow=0.1 at bow_speed=1 must NOT be claimed to sustain \
         (factor {light_factor:.2}, measured 2.11)"
    );
}

#[test]
fn bow_releases_at_the_slot_edge() {
    // The friction port stops with the slot, and what is left is an
    // ordinary decaying string: 1.5 s later it must be far quieter.
    let held = pluck_note(
        &isolated(&[("bow", "0.7"), ("bow_speed", "0.5"), ("sus", "3")]),
        &[196.0],
        1.0,
        1.0,
    );
    let at_slot = rms(window(&held, 0.8, 1.0));
    let after = rms(window(&held, 2.4, 2.6));
    let ratio = after / at_slot.max(1e-12);
    println!("bow release: at slot={at_slot:.6}, slot+1.5 s={after:.6}, ratio={ratio:.4}");
    assert!(
        at_slot > 1e-3,
        "the bowed slot must be audible: {at_slot:.6}"
    );
    // MEASURED 0.0209 (-33.6 dB), consistent with the string's own
    // T60 = 3 s once the source is removed.
    assert!(
        ratio < 0.10,
        "post-slot tail must decay: {ratio:.4} of the held level"
    );
}

#[test]
fn bow_stays_bounded_over_the_corner_grid() {
    // Empirical boundedness over the (bow, bow_speed, sus, damp) corners,
    // with body/symp/buzz at maximum and a 1.2 s slot — long enough for
    // the friction port to reach steady state at every sus.
    let mut worst = 0.0_f32;
    let mut worst_case = String::new();
    for bow in ["0", "0.5", "1"] {
        for bow_speed in ["0", "0.5", "1"] {
            for sus in ["0.05", "2.5", "20"] {
                for damp in ["0", "0.35", "0.95"] {
                    let audio = pluck_note(
                        &isolated(&[
                            ("bow", bow),
                            ("bow_speed", bow_speed),
                            ("sus", sus),
                            ("damp", damp),
                            ("body", "1"),
                            ("symp", "1"),
                            ("buzz", "1"),
                        ]),
                        &[82.4069, 110.0, 146.832],
                        1.2,
                        1.0,
                    );
                    assert!(
                        audio.iter().all(|sample| sample.is_finite()),
                        "non-finite at bow={bow} bow_speed={bow_speed} sus={sus} damp={damp}"
                    );
                    let corner = peak(&audio);
                    if corner > worst {
                        worst = corner;
                        worst_case =
                            format!("bow={bow} bow_speed={bow_speed} sus={sus} damp={damp}");
                    }
                }
            }
        }
    }
    println!("bow corner grid: worst peak {worst:.4} at {worst_case}");
    // MEASURED: 2.13, and it belongs to the UN-bowed pluck (bow=0,
    // sus=20, damp=0); the worst bowed corner is 1.78. The bound is the
    // module's existing 8.0 measurement bound and is NOT widened here.
    assert!(worst <= 8.0, "bowed corner grid peak {worst} is bounded");

    // The exotic neighbours: full bow on the extreme weave patch, and on
    // a bent extreme guitar (delay-line retuning does work on the state,
    // so this is the case that historically peaks highest).
    let woven = params(&[
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
        ("bow", "1"),
        ("bow_speed", "1"),
    ]);
    let woven_audio = weave_note(&woven, &[82.4069, 110.0, 146.832], 1.2, 1.0);
    let extreme = params(&[
        ("synth", "pluck"),
        ("sus", "20"),
        ("damp", "0"),
        ("pos", "0.02"),
        ("pick", "1"),
        ("stiff", "1"),
        ("tension", "80"),
        ("buzz", "1"),
        ("body", "1"),
        ("symp", "1"),
        ("detune", "60"),
        ("bow", "1"),
        ("bow_speed", "0.5"),
    ]);
    let bent_audio = pluck_note(&extreme, &[82.4069, 110.0, 146.832], 1.2, 1.5);
    for (name, audio, measured) in [
        ("weave extreme + bow", &woven_audio, 0.73),
        ("bent extreme + bow", &bent_audio, 0.35),
    ] {
        assert!(audio.iter().all(|sample| sample.is_finite()), "{name}");
        let corner = peak(audio);
        println!("{name}: peak {corner:.4} (measured {measured})");
        assert!(corner <= 8.0, "{name} peak {corner} is bounded");
    }
}

#[test]
fn bow_render_is_deterministic() {
    let patch = isolated(&[("sus", "1.5"), ("bow", "0.6"), ("bow_speed", "0.5")]);
    let a = pluck_note(&patch, &[196.0], 1.0, 1.0);
    let b = pluck_note(&patch, &[196.0], 1.0, 1.0);
    assert_eq!(a, b, "a bowed note is a pure function of its patch");
    let faster = isolated(&[("sus", "1.5"), ("bow", "0.6"), ("bow_speed", "0.9")]);
    assert_ne!(
        a,
        pluck_note(&faster, &[196.0], 1.0, 1.0),
        "bow_speed is audible once the bow is down"
    );
}

#[test]
fn bow_crescendo_rises_through_the_note() {
    // The trajectory law for a source: 40 frames walk `bow` from 0 to 0.8
    // across a 4 s slot, and the energy envelope follows. Sampled in three
    // windows the note grows monotonically — a real crescendo, played by
    // control frames rather than by an amplitude envelope.
    let patch = isolated(&[("sus", "1.2"), ("bow_speed", "0.35")]);
    let at = |seconds: f64| (seconds * SR_F64) as usize;
    let trajectory: Vec<ControlFrame> = (0..=40)
        .map(|step| ControlFrame {
            at_sample: at(0.1 * step as f64),
            sets: vec![(ControlTarget::Bow, 0.8 * step as f64 / 40.0)],
        })
        .collect();
    let (audio, _) =
        StringNetworkVoice::guitar(&patch, &[196.0], 4.0, 1.0).render_traced_with(256, &trajectory);
    assert!(audio.iter().all(|sample| sample.is_finite()));
    let early = rms(window(&audio, 1.2, 1.5));
    let middle = rms(window(&audio, 2.4, 2.7));
    let late = rms(window(&audio, 3.6, 3.9));
    println!("bow crescendo: {early:.6} -> {middle:.6} -> {late:.6}");
    // MEASURED: 1.48e-3 -> 1.19e-2 -> 1.92e-2 (x8.0, then x1.61).
    assert!(
        middle > early * 2.0,
        "crescendo must rise: {early:.6} -> {middle:.6}"
    );
    assert!(
        late > middle * 1.15,
        "crescendo must keep rising: {middle:.6} -> {late:.6}"
    );
    let crescendo_peak = peak(&audio);
    println!("bow crescendo peak: {crescendo_peak:.4}");
    assert!(crescendo_peak <= 8.0, "peak {crescendo_peak} is bounded");
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

// ---------------------------------------------------------------- PW10 §A
// Body profiles (`bodyp`). The compat law is the hard one: `bodyp` absent
// or `"guitar"` must render byte-identically to the pre-PW10 single-table
// engine, for both pluck and weave. Since there is no pre-change binary to
// diff against, the law is split into two independently-checkable halves:
// (1) absent == "guitar" (below), and (2) the guitar table itself matches
// the historical `BODY_MODE_SPECS` literal verbatim (next test). Together
// they leave no room for the registry to have silently perturbed the
// guitar sound while still passing (1) alone.

#[test]
fn bodyp_absent_equals_guitar_identity_pluck_and_weave() {
    let pluck_absent = isolated(&[("sus", "2.0"), ("body", "0.6"), ("body_size", "1.1")]);
    let mut pluck_guitar = pluck_absent.clone();
    pluck_guitar.insert("bodyp".to_string(), "guitar".to_string());
    let a = pluck_note(&pluck_absent, &[110.0, 164.8138], 1.0, 1.0);
    let b = pluck_note(&pluck_guitar, &[110.0, 164.8138], 1.0, 1.0);
    assert_eq!(a, b, "bodyp absent == bodyp=guitar (pluck)");
    let a_again = pluck_note(&pluck_absent, &[110.0, 164.8138], 1.0, 1.0);
    assert_eq!(a, a_again, "deterministic (pluck)");

    let weave_absent = params(&[
        ("synth", "weave"),
        ("sus", "1.5"),
        ("courses", "9"),
        ("couple", "0.16"),
        ("body", "0.5"),
        ("body_size", "1.0"),
    ]);
    let mut weave_guitar = weave_absent.clone();
    weave_guitar.insert("bodyp".to_string(), "guitar".to_string());
    let wa = weave_note(&weave_absent, &[110.0, 165.0], 1.0, 1.0);
    let wb = weave_note(&weave_guitar, &[110.0, 165.0], 1.0, 1.0);
    assert_eq!(wa, wb, "bodyp absent == bodyp=guitar (weave)");
    let wa_again = weave_note(&weave_absent, &[110.0, 165.0], 1.0, 1.0);
    assert_eq!(wa, wa_again, "deterministic (weave)");

    // An unrecognised name also falls back to guitar (the enum clamp),
    // not just the absent-key path.
    let mut pluck_bogus = pluck_absent.clone();
    pluck_bogus.insert("bodyp".to_string(), "banjo".to_string());
    let c = pluck_note(&pluck_bogus, &[110.0, 164.8138], 1.0, 1.0);
    assert_eq!(a, c, "unrecognised bodyp falls back to guitar (pluck)");
}

#[test]
fn guitar_body_profile_matches_the_historical_table_verbatim() {
    // The pre-PW10 `BODY_MODE_SPECS` literal, transcribed here
    // independently of `body_profiles.rs` so this test would catch a
    // silent drift in the registry file even if the compat-identity test
    // above somehow still passed.
    const HISTORICAL_GUITAR_MODES: [(f64, f64, f64, f64); 10] = [
        (98.0, 1.25, 0.0065, 0.78),
        (186.0, 0.95, 0.0058, 0.72),
        (221.0, 0.82, 0.0053, 0.66),
        (286.0, 0.72, 0.0047, 0.58),
        (365.0, 0.58, 0.0042, 0.50),
        (455.0, 0.50, 0.0038, 0.43),
        (610.0, 0.40, 0.0032, 0.34),
        (835.0, 0.32, 0.0027, 0.27),
        (1175.0, 0.25, 0.0022, 0.20),
        (1580.0, 0.18, 0.0018, 0.14),
    ];
    assert_eq!(body_profiles::GUITAR_MODES, HISTORICAL_GUITAR_MODES);
}

#[test]
fn every_body_profile_is_bounded_under_the_extreme_patch() {
    // Measured peaks at this exact corner grid (unbent, gliss_ratio=1.0):
    // pluck maxes out around 0.28 (bass); weave — which stacks 24 courses
    // against body=1 and every scattering control maxed — maxes out
    // around 5.29 (bass, whose 0.0070 coupling is the registry's
    // highest). Bounds below carry headroom over that measurement, the
    // same discipline `bounded_and_finite_over_extreme_controls` uses;
    // this is not a passivity claim for the source-shaping params (buzz,
    // tension, detune) that are already outside the rotation proof.
    let profiles = [
        "guitar", "violin", "cello", "bass", "mandolin", "a", "e", "i", "o", "u",
    ];
    for profile in profiles {
        let pluck_patch = params(&[
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
            ("bodyp", profile),
        ]);
        let audio = pluck_note(&pluck_patch, &[82.4069, 110.0, 146.832], 0.45, 1.0);
        assert!(
            audio.iter().all(|s| s.is_finite()),
            "{profile}: pluck finite"
        );
        let peak = audio.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!(peak <= 1.5, "{profile}: pluck extreme peak {peak} bounded");

        let weave_patch = params(&[
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
            ("bodyp", profile),
        ]);
        let waudio = weave_note(&weave_patch, &[82.4069, 110.0, 146.832], 0.45, 1.0);
        assert!(
            waudio.iter().all(|s| s.is_finite()),
            "{profile}: weave finite"
        );
        let wpeak = waudio.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!(
            wpeak <= 7.0,
            "{profile}: weave extreme peak {wpeak} bounded"
        );
    }
}

fn spectrum_vector(audio: &[f32], freqs: &[f64]) -> Vec<f64> {
    freqs
        .iter()
        .map(|&f| goertzel(audio, f).max(0.0).sqrt())
        .collect()
}

/// Relative RMS distance between two equal-length vectors, normalised by
/// their combined magnitude so it is comparable across profile pairs of
/// different overall loudness.
fn rel_rms_distance(a: &[f64], b: &[f64]) -> f64 {
    let diff = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt();
    let scale = ((a.iter().map(|v| v * v).sum::<f64>() + b.iter().map(|v| v * v).sum::<f64>())
        / 2.0)
        .sqrt();
    diff / scale.max(1e-18)
}

#[test]
fn body_profiles_are_pairwise_spectrally_distinct() {
    let profiles = [
        "guitar", "violin", "cello", "bass", "mandolin", "a", "e", "i", "o", "u",
    ];
    // Log-spaced band centers spanning every profile's modal range: the
    // bass table anchors near 60 Hz, the violin/mandolin tables climb
    // past 3 kHz.
    let bands: Vec<f64> = (0..48)
        .map(|i| 60.0 * (4000.0_f64 / 60.0).powf(i as f64 / 47.0))
        .collect();
    let spectra: Vec<Vec<f64>> = profiles
        .iter()
        .map(|&profile| {
            let patch = params(&[
                ("synth", "pluck"),
                ("sus", "1.6"),
                ("damp", "0.15"),
                ("pos", "0.13"),
                ("pick", "0.6"),
                ("body", "1.0"),
                ("body_size", "1.0"),
                ("symp", "0"),
                ("stiff", "0"),
                ("tension", "0"),
                ("buzz", "0"),
                ("detune", "0"),
                ("strum", "0"),
                ("bodyp", profile),
            ]);
            let audio = pluck_note(&patch, &[110.0], 1.4, 1.0);
            spectrum_vector(&audio, &bands)
        })
        .collect();
    // PW10 measured 0.50-1.39 across the five instrument bodies (guitar
    // vs cello closest, bass vs mandolin farthest). GARGANTA's five vowel
    // bodies (45 pairs total) pull the measured minimum to 0.29 — the
    // closest pair now sits in the vowel family, as it should (/o/ vs
    // /u/ differ mainly by F1). The 0.25 floor holds, now snugly —
    // strengthening formant radiation for rung-1 audibility spent most
    // of the old headroom; if a future table lands under the floor,
    // differentiate the tables rather than lowering it.
    let floor = 0.25;
    let mut minimum = f64::INFINITY;
    for i in 0..profiles.len() {
        for j in (i + 1)..profiles.len() {
            let distance = rel_rms_distance(&spectra[i], &spectra[j]);
            minimum = minimum.min(distance);
            assert!(
                distance > floor,
                "{} vs {}: relative spectral distance {distance:.4} <= floor {floor}",
                profiles[i],
                profiles[j]
            );
        }
    }
    println!("minimum pairwise relative spectral distance: {minimum:.4}");
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

// ------------------------------------------------ PW10 composition laws
// Each PW10 line proved its own zero limit and boundedness in an isolated
// worktree that did not contain the other two features. These pin the
// pairings none of the per-line suites could structurally see.

#[test]
fn pw10_explicit_zeros_match_the_unspecified_patch() {
    // bodyp=guitar, bow=0, breath=0 spelled out == none of them present,
    // byte-for-byte, pluck and weave. Together with each line's own
    // zero-limit law this closes the combined limit without a captured
    // pre-PW10 render (none exists for the string-network engine — see
    // the PW10 spec's landing status).
    let pluck_plain = isolated(&[("sus", "1.8"), ("body", "0.5"), ("buzz", "0.2")]);
    let mut pluck_explicit = pluck_plain.clone();
    pluck_explicit.insert("bodyp".to_string(), "guitar".to_string());
    pluck_explicit.insert("bow".to_string(), "0".to_string());
    pluck_explicit.insert("bow_speed".to_string(), "0.5".to_string());
    pluck_explicit.insert("breath".to_string(), "0".to_string());
    let a = pluck_note(&pluck_plain, &[110.0, 164.8138], 1.0, 1.0);
    let b = pluck_note(&pluck_explicit, &[110.0, 164.8138], 1.0, 1.0);
    assert_eq!(a, b, "explicit PW10 zeros == unspecified (pluck)");

    let weave_plain = params(&[
        ("synth", "weave"),
        ("sus", "1.5"),
        ("courses", "9"),
        ("couple", "0.16"),
        ("chirality", "0.6"),
        ("body", "0.5"),
    ]);
    let mut weave_explicit = weave_plain.clone();
    weave_explicit.insert("bodyp".to_string(), "guitar".to_string());
    weave_explicit.insert("bow".to_string(), "0".to_string());
    weave_explicit.insert("bow_speed".to_string(), "0.2".to_string());
    weave_explicit.insert("breath".to_string(), "0".to_string());
    let wa = weave_note(&weave_plain, &[110.0, 165.0], 1.0, 1.0);
    let wb = weave_note(&weave_explicit, &[110.0, 165.0], 1.0, 1.0);
    assert_eq!(wa, wb, "explicit PW10 zeros == unspecified (weave)");
}

#[test]
fn bow_and_breath_together_stay_bounded() {
    // The two sustained sources sum into one excitation node. B's corner
    // grid maxed at 2.13 and C's stayed under 8.0, but neither ran the
    // pair. The house 8.0 bound applies, not widened.
    let pluck = params(&[
        ("synth", "pluck"),
        ("bow", "0.9"),
        ("bow_speed", "0.5"),
        ("breath", "0.8"),
        ("sus", "8"),
        ("damp", "0.05"),
        ("body", "1"),
        ("symp", "0.6"),
        ("buzz", "0.5"),
    ]);
    let audio = pluck_note(&pluck, &[110.0, 164.8138], 1.5, 1.0);
    assert!(audio.iter().all(|sample| sample.is_finite()));
    let peak = audio
        .iter()
        .fold(0.0_f32, |maximum, sample| maximum.max(sample.abs()));
    assert!(peak <= 8.0, "bow+breath pluck peak {peak} bounded");

    let weave = params(&[
        ("synth", "weave"),
        ("bow", "0.8"),
        ("bow_speed", "0.5"),
        ("breath", "0.7"),
        ("sus", "6"),
        ("damp", "0.1"),
        ("courses", "13"),
        ("couple", "0.25"),
        ("chirality", "0.8"),
        ("orbit", "4"),
        ("orbit_depth", "0.6"),
        ("curvature", "0.5"),
        ("body", "1"),
    ]);
    let weave_audio = weave_note(&weave, &[110.0, 165.0, 220.0], 1.5, 1.0);
    assert!(weave_audio.iter().all(|sample| sample.is_finite()));
    let weave_peak = weave_audio
        .iter()
        .fold(0.0_f32, |maximum, sample| maximum.max(sample.abs()));
    assert!(
        weave_peak <= 8.0,
        "bow+breath weave peak {weave_peak} bounded"
    );
    println!("bow+breath peaks: pluck {peak:.4}, weave {weave_peak:.4}");
}

#[test]
fn nonguitar_bodies_stay_bounded_under_bow_and_breath() {
    // A's boundedness grid ran every profile un-bowed and un-breathed;
    // the sustained sources drive a body harder than any single pluck.
    for profile in ["violin", "cello", "bass", "mandolin"] {
        let patch = params(&[
            ("synth", "pluck"),
            ("bodyp", profile),
            ("bow", "0.8"),
            ("bow_speed", "0.5"),
            ("breath", "0.6"),
            ("sus", "6"),
            ("damp", "0.1"),
            ("body", "1"),
            ("body_size", "1.2"),
        ]);
        let audio = pluck_note(&patch, &[110.0, 164.8138], 1.5, 1.0);
        assert!(
            audio.iter().all(|sample| sample.is_finite()),
            "{profile} stays finite under bow+breath"
        );
        let peak = audio
            .iter()
            .fold(0.0_f32, |maximum, sample| maximum.max(sample.abs()));
        assert!(
            peak <= 8.0,
            "{profile} under bow+breath peak {peak} bounded"
        );
        println!("{profile} under bow+breath: peak {peak:.4}");
    }
}
