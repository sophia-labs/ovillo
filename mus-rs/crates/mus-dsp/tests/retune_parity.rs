//! Ariadne Phase 0 §A — the `retune` byte law.
//!
//! The hard law of this round: `retune` absent == `retune=legacy` ==
//! today's render, byte-identical. Rendering with the *pre-change* code
//! path is impossible once the engine has changed, so legacy invariance
//! is asserted by a two-part argument:
//!
//! 1. **Hand-pinned digests.** The constants below were captured from the
//!    engine at `334fcae`, BEFORE the first `retune` edit landed, by
//!    running this file with placeholder digests and recording what the
//!    unmodified engine produced. They are a photograph of the old
//!    renderer; nothing regenerates them. If a future change moves one,
//!    that change moved a legacy render and is wrong by the Phase 0 law.
//! 2. **Structural non-participation.** `retune=legacy` never reaches the
//!    neutral transport at all — see `legacy_mode_never_calls_the_transport`
//!    in `pluck.rs`, which asserts the mode discriminator at the three
//!    `set_coefficient` call sites.
//!
//! Together these say more than a same-process A/B could: (1) fixes the
//! numbers to the pre-change engine, (2) fixes the control flow.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use mus_dsp::pluck::{pluck_note, weave_note, StringNetworkVoice};
use mus_dsp::SR_F64;

fn params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

/// SHA-256 over the raw little-endian f32 bits. Byte-identity, not
/// approximate agreement: one ulp anywhere changes the digest.
fn digest(audio: &[f32]) -> String {
    let mut hasher = Sha256::new();
    hasher.update((audio.len() as u64).to_le_bytes());
    for sample in audio {
        hasher.update(sample.to_bits().to_le_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

/// Parabolically-refined autocorrelation pitch estimate over a window.
/// Same estimator as `pluck_invariants.rs` — deliberately duplicated
/// rather than shared, so the tuning tolerance this file asserts is
/// measured the same way the pre-existing bend test measures it.
fn autocorrelation_f0(values: &[f32], lo_hz: f64, hi_hz: f64) -> f64 {
    let min_lag = (SR_F64 / hi_hz) as usize;
    let max_lag = (SR_F64 / lo_hz) as usize;
    let correlation = |lag: usize| -> f64 {
        values[..values.len() - lag]
            .iter()
            .zip(values[lag..].iter())
            .map(|(&left, &right)| left as f64 * right as f64)
            .sum()
    };
    let mut best = (f64::NEG_INFINITY, min_lag);
    for lag in min_lag..=max_lag.min(values.len() / 2) {
        let value = correlation(lag);
        if value > best.0 {
            best = (value, lag);
        }
    }
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

/// The extreme Weave bend from `pluck_invariants::network_stays_bounded`
/// — the patch whose upward-fifth peak is the 5.38 the consensus sends
/// Phase 0 §C to explain.
fn weave_extreme() -> BTreeMap<String, String> {
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
    ])
}

/// The extreme guitar patch from the same test: every stage lit at once.
fn guitar_extreme() -> BTreeMap<String, String> {
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
    ])
}

/// A tension-decay note: no bend, but `tension` drives the instantaneous
/// frequency (and therefore every allpass coefficient) for the whole
/// relaxation. This is the second retune-active regime.
fn tension_decay() -> BTreeMap<String, String> {
    params(&[
        ("synth", "pluck"),
        ("sus", "2.0"),
        ("tension", "80"),
        ("stiff", "0.6"),
        ("body", "0.4"),
        ("symp", "0.5"),
    ])
}

/// A plain Weave chord at the shipped defaults.
fn weave_chord() -> BTreeMap<String, String> {
    params(&[("synth", "weave"), ("sus", "2.0")])
}

/// The four legacy fixtures, each rendered by name.
fn render(name: &str, patch: &BTreeMap<String, String>) -> Vec<f32> {
    match name {
        "bent_weave_extreme" => weave_note(patch, &[82.4069, 110.0, 146.832], 0.45, 1.5),
        "bent_guitar_extreme" => pluck_note(patch, &[82.4069, 110.0, 146.832], 0.45, 1.5),
        "tension_decay" => pluck_note(patch, &[196.0], 1.0, 1.0),
        "weave_chord" => weave_note(patch, &[110.0, 164.814, 220.0], 1.0, 1.0),
        other => panic!("unknown fixture {other:?}"),
    }
}

fn fixtures() -> Vec<(&'static str, BTreeMap<String, String>)> {
    vec![
        ("bent_weave_extreme", weave_extreme()),
        ("bent_guitar_extreme", guitar_extreme()),
        ("tension_decay", tension_decay()),
        ("weave_chord", weave_chord()),
    ]
}

/// Captured from the engine at commit `334fcae`, before any `retune`
/// code existed. DO NOT regenerate: regenerating destroys the only
/// record of the pre-change renderer.
///
/// The photograph is **release-profile** (the shipped `mus render` binary
/// and the CI battery both build `--release`). Debug-profile floats
/// differ bit-wise from these pins — measured 2026-08-11: the same three
/// digest tests fail identically at every commit from Phase 0 to HEAD
/// under `cargo test` (debug) and pass in 58 s under `--release`, so the
/// difference is profile codegen, not a render regression. The byte law
/// is therefore asserted only in the profile it certifies; comparisons
/// *within one process* (e.g. the static-note A/B below) stay ungated
/// because both sides move together.
const LEGACY_DIGESTS: &[(&str, &str)] = &[
    (
        "bent_weave_extreme",
        "sha256:ad3c074aa360ded1ca6adf039a6c565eb16f499c66f907a26020fbb8dc9dce21",
    ),
    (
        "bent_guitar_extreme",
        "sha256:81e5c9cc36152623be44b5d4d094f675b91af3ad057b6b1afce715ad87bbf359",
    ),
    (
        "tension_decay",
        "sha256:9b017a792f69aef7174e92327090a242e3b897d70fc4f89c765811e622a13a70",
    ),
    (
        "weave_chord",
        "sha256:8167b210afb4c336af9a45c2b8192ef367c8c2e836f0da232521ba29f2078dfb",
    ),
];

fn pinned(name: &str) -> &'static str {
    LEGACY_DIGESTS
        .iter()
        .find(|(fixture, _)| *fixture == name)
        .map(|(_, digest)| *digest)
        .expect("fixture is pinned")
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "byte pins certify the release profile; run with --release"
)]
fn absent_retune_matches_the_pre_change_engine() {
    for (name, patch) in fixtures() {
        let audio = render(name, &patch);
        assert_eq!(
            digest(&audio),
            pinned(name),
            "fixture {name:?} with no `retune` key must render exactly as the pre-change engine did"
        );
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "byte pins certify the release profile; run with --release"
)]
fn explicit_legacy_matches_the_pre_change_engine() {
    for (name, mut patch) in fixtures() {
        patch.insert("retune".to_string(), "legacy".to_string());
        assert_eq!(
            digest(&render(name, &patch)),
            pinned(name),
            "fixture {name:?} with `retune=legacy` must render exactly as the pre-change engine did"
        );
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "byte pins certify the release profile; run with --release"
)]
fn an_unrecognised_retune_value_degrades_to_legacy() {
    for value in ["neutral", "", "NEUTRAL_FILTERS", "1", "garbage"] {
        let mut patch = weave_extreme();
        patch.insert("retune".to_string(), value.to_string());
        assert_eq!(
            digest(&render("bent_weave_extreme", &patch)),
            pinned("bent_weave_extreme"),
            "retune={value:?} must degrade to legacy, not to something new"
        );
    }
}

/// Phase 0 §A law 2. A note with no bend and no onset tension asks for
/// the same frequency forever, so every coefficient update after the
/// first is a no-op — and a no-op is byte-inert in both modes. The
/// equality is exact, not approximate: if the neutral transport ever ran
/// on this fixture, it would not be.
#[test]
fn a_static_note_renders_identically_under_both_modes() {
    let statics = [
        (
            "guitar",
            &[
                ("synth", "pluck"),
                ("sus", "2.0"),
                ("tension", "0"),
                ("stiff", "0.6"),
                ("body", "0.5"),
                ("symp", "0.4"),
            ][..],
            &[196.0_f64][..],
            1.0_f64,
        ),
        (
            "guitar_no_stiffness",
            &[
                ("synth", "pluck"),
                ("sus", "1.5"),
                ("tension", "0"),
                ("stiff", "0"),
                ("buzz", "0.5"),
            ][..],
            &[110.0_f64][..],
            0.8,
        ),
        (
            "weave",
            &[
                ("synth", "weave"),
                ("sus", "2.0"),
                ("tension", "0"),
                ("courses", "13"),
                ("couple", "0.3"),
                ("body", "0.8"),
            ][..],
            &[110.0_f64, 164.814][..],
            1.0,
        ),
    ];
    for (name, pairs, notes, slot) in statics {
        let base = params(pairs);
        let render_one = |mode: &str| -> Vec<f32> {
            let mut patch = base.clone();
            patch.insert("retune".to_string(), mode.to_string());
            if patch.get("synth").map(String::as_str) == Some("weave") {
                weave_note(&patch, notes, slot, 1.0)
            } else {
                pluck_note(&patch, notes, slot, 1.0)
            }
        };
        assert_eq!(
            render_one("legacy"),
            render_one("neutral_filters"),
            "static fixture {name:?}: neutral_filters must be byte-identical to legacy"
        );
    }
}

/// Phase 0 §A law 3, first half. Under `neutral_filters` the bend corner
/// grid stays finite and bounded by the same 8.0 the legacy invariants
/// declare (`pluck_invariants::bounded_and_finite_over_extreme_controls`).
#[test]
fn neutral_filters_stays_bounded_across_the_bend_corner_grid() {
    let mut worst = (0.0_f32, String::new());
    for &f0 in &[82.4069_f64, 110.0, 220.0, 440.0] {
        for &ratio in &[2.0 / 3.0, 1.5_f64, 2.0] {
            for &stiff in &["0", "0.75", "1"] {
                for synth in ["pluck", "weave"] {
                    let mut patch = if synth == "weave" {
                        weave_extreme()
                    } else {
                        guitar_extreme()
                    };
                    patch.insert("stiff".to_string(), stiff.to_string());
                    patch.insert("retune".to_string(), "neutral_filters".to_string());
                    let audio = if synth == "weave" {
                        weave_note(&patch, &[f0], 0.45, ratio)
                    } else {
                        pluck_note(&patch, &[f0], 0.45, ratio)
                    };
                    let label = format!("{synth} f0={f0} ratio={ratio:.4} stiff={stiff}");
                    assert!(
                        audio.iter().all(|sample| sample.is_finite()),
                        "{label}: non-finite sample"
                    );
                    let peak = audio
                        .iter()
                        .fold(0.0_f32, |maximum, sample| maximum.max(sample.abs()));
                    if peak > worst.0 {
                        worst = (peak, label.clone());
                    }
                    assert!(
                        peak <= 8.0,
                        "{label}: peak {peak} exceeds the declared bound"
                    );
                }
            }
        }
    }
    println!(
        "worst neutral_filters corner peak {:.4} at {}",
        worst.0, worst.1
    );
}

/// Phase 0 §A law 3, second half. The rendered fundamental still lands on
/// the bend target within the tolerance
/// `pluck_invariants::bends_retime_the_waveguide_and_land_on_the_target`
/// declares (45 cents), under BOTH modes.
#[test]
fn neutral_filters_keeps_the_bend_landing_in_tune() {
    for mode in ["legacy", "neutral_filters"] {
        let mut patch = params(&[
            ("synth", "pluck"),
            ("body", "0"),
            ("symp", "0"),
            ("stiff", "0"),
            ("tension", "0"),
            ("buzz", "0"),
            ("detune", "0"),
            ("strum", "0"),
            ("sus", "4"),
            ("damp", "0.18"),
        ]);
        patch.insert("retune".to_string(), mode.to_string());
        let audio = pluck_note(&patch, &[220.0], 1.3, 1.5);
        let estimate = estimate_f0(&audio, 1.42, 1.82, 260.0, 380.0);
        let cents = 1200.0 * (estimate / 330.0).log2().abs();
        assert!(
            cents < 45.0,
            "retune={mode}: tail={estimate:.2} Hz, target=330 Hz, error={cents:.1}c"
        );
        println!("retune={mode}: bend lands at {estimate:.2} Hz ({cents:.2}c)");
    }
}

/// Phase 0 §A law 4. Under `neutral_filters` the largest single allpass
/// coefficient-update work over a bent render is at numerical tolerance;
/// under `legacy` the same fixture has a large positive control. Both
/// numbers are printed so the report can cite them.
#[test]
fn allpass_update_work_is_at_tolerance_under_neutral_and_nonzero_under_legacy() {
    let mut peaks = Vec::new();
    for mode in ["legacy", "neutral_filters"] {
        let mut patch = weave_extreme();
        patch.insert("retune".to_string(), mode.to_string());
        let voice = StringNetworkVoice::weave(&patch, &[82.4069, 110.0, 146.832], 0.45, 1.5);
        let (_, _, energy) = voice.render_traced_observed(256, &[]);
        let total = energy.last("work_allpass_retune").expect("column");
        println!(
            "retune={mode}: peak per-update allpass work {:.6e}, |sum| {:.6e}",
            energy.allpass_update_peak,
            total.abs()
        );
        peaks.push((mode, energy.allpass_update_peak, total));
    }
    let neutral = peaks
        .iter()
        .find(|(mode, _, _)| *mode == "neutral_filters")
        .expect("neutral row");
    let legacy = peaks
        .iter()
        .find(|(mode, _, _)| *mode == "legacy")
        .expect("legacy row");
    assert!(
        neutral.1 <= 1e-12,
        "neutral per-update allpass work {:.6e} must be at tolerance",
        neutral.1
    );
    assert!(
        legacy.1 > 1e-6,
        "legacy per-update allpass work {:.6e} must be a live positive control",
        legacy.1
    );
    assert!(
        legacy.1 > neutral.1 * 1e6,
        "the repair must be orders of magnitude, not a nudge"
    );
}
