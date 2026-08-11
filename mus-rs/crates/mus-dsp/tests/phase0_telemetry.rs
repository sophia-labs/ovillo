//! Ariadne Phase 0 §B — observer-only telemetry.
//!
//! Two laws and one measurement.
//!
//! **Law 1 (hard).** Observer-enabled and observer-disabled renders are
//! byte-identical. The observer reads state the audio step has already
//! written, through `&self`, and no accumulator is ever read back.
//!
//! **Law 2 (the ledger).** Over the declared state — every course's
//! ACTIVE delay segment under the Euclidean research metric, every
//! course's filter memory (three allpass storages `E_a` plus the one-zero
//! damping storage `E_z = d*x1^2/2`), and every body mode's
//! `(q^2 + p^2)/2` — the accounted flows close:
//!
//! ```text
//! final = initial + sum(inject) + sum(work) - sum(loss)
//! ```
//!
//! The residual is measured here, not assumed, and printed so the report
//! can cite it.
//!
//! **What is deliberately NOT in the ledger.** The audio output. The
//! pickup sums `incoming` and the body radiates `mode.q`; neither removes
//! anything from the state it reads. Booking the output as a loss would
//! double-count, so `output_energy` is recorded as an observation with no
//! ledger role. On bowed notes the bow appears as `inject_bow` — a
//! source, per PW10 §B — and no passivity is claimed.

use std::collections::BTreeMap;

use mus_dsp::pluck::{StringNetworkVoice, ENERGY_TRACE_COLUMNS};

fn params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

/// One render fixture: a patch plus the note list, slot and bend ratio
/// it is played with.
struct Case {
    name: &'static str,
    patch: BTreeMap<String, String>,
    notes: Vec<f64>,
    slot: f64,
    gliss: f64,
}

impl Case {
    fn new(
        name: &'static str,
        pairs: &[(&str, &str)],
        notes: &[f64],
        slot: f64,
        gliss: f64,
    ) -> Self {
        Self {
            name,
            patch: params(pairs),
            notes: notes.to_vec(),
            slot,
            gliss,
        }
    }

    /// Build a fresh voice, dispatching on the patch's own `synth`.
    fn voice(&self) -> StringNetworkVoice {
        if self.patch.get("synth").map(String::as_str) == Some("weave") {
            StringNetworkVoice::weave(&self.patch, &self.notes, self.slot, self.gliss)
        } else {
            StringNetworkVoice::guitar(&self.patch, &self.notes, self.slot, self.gliss)
        }
    }
}

/// The declared Phase 0 §B ledger residual tolerance, absolute. It is a
/// floating-point accumulation budget, not modelling slack: every stage
/// law in the ledger is an exact identity (allpass storage balance,
/// one-zero dissipation, loop-gain attenuation, contact removal, modal
/// decay), so the only thing this covers is summation roundoff over
/// ~10^5 samples.
///
/// Worst residual measured across the fixtures below at the time of
/// writing: `3.635e-12` (`guitar_plain`), against injected/lost totals of
/// order `10^2` — a relative closure near `10^-14`. The tolerance is set
/// two-and-a-half decades above the worst observation so a real
/// regression trips it and a platform's last bit does not.
const LEDGER_RESIDUAL_TOLERANCE: f64 = 1.0e-9;

#[test]
fn observer_on_and_off_are_byte_identical() {
    let cases = [
        Case::new(
            "weave_extreme_bend",
            &[
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
            ],
            &[82.4069, 110.0, 146.832],
            0.45,
            1.5,
        ),
        Case::new(
            "guitar_everything",
            &[
                ("synth", "pluck"),
                ("sus", "1.2"),
                ("stiff", "1"),
                ("tension", "80"),
                ("buzz", "1"),
                ("body", "1"),
                ("symp", "1"),
                ("detune", "60"),
            ],
            &[110.0],
            0.6,
            1.5,
        ),
        Case::new(
            "bowed",
            &[
                ("synth", "pluck"),
                ("sus", "1.5"),
                ("bow", "0.8"),
                ("bow_speed", "0.7"),
                ("body", "0.6"),
            ],
            &[196.0],
            0.8,
            1.0,
        ),
        Case::new(
            "breathed_neutral",
            &[
                ("synth", "weave"),
                ("sus", "1.5"),
                ("breath", "0.7"),
                ("courses", "9"),
                ("couple", "0.2"),
                ("body", "0.5"),
                ("retune", "neutral_filters"),
            ],
            &[110.0, 164.814],
            0.8,
            1.25,
        ),
    ];
    for case in &cases {
        let name = case.name;
        let (quiet_audio, quiet_trace) = case.voice().render_traced_with(256, &[]);
        let (observed_audio, observed_trace, energy) =
            case.voice().render_traced_observed(256, &[]);
        assert_eq!(
            quiet_audio, observed_audio,
            "{name}: enabling the observer moved the audio"
        );
        assert_eq!(
            quiet_trace.data, observed_trace.data,
            "{name}: enabling the observer moved the existing weave trace"
        );
        assert!(energy.frames > 0, "{name}: observer produced no frames");
        assert_eq!(
            energy.data.len(),
            energy.frames * ENERGY_TRACE_COLUMNS.len(),
            "{name}: energy trace shape"
        );
        assert_eq!(
            energy.course_data.len(),
            energy.frames * energy.courses * 2,
            "{name}: per-course trace shape"
        );
        assert!(
            energy.data.iter().all(|value| value.is_finite()),
            "{name}: non-finite telemetry"
        );
    }
}

#[test]
fn a_disabled_observer_records_nothing() {
    let patch = params(&[("synth", "weave"), ("sus", "1.0")]);
    let voice = StringNetworkVoice::weave(&patch, &[110.0], 0.5, 1.0);
    let (_, _) = voice.render_traced_with(256, &[]);
    // The quiet seam returns no EnergyTrace at all; the observed seam is
    // the only way to obtain one. This test exists to pin that the quiet
    // entry point's signature does not grow an observation channel.
    let voice = StringNetworkVoice::weave(&patch, &[110.0], 0.5, 1.0);
    let (_, _, energy) = voice.render_traced_observed(256, &[]);
    assert!(energy.frames > 0);
}

/// Phase 0 §B law 2, the passive regime: no bow, no breath, no sing, no
/// contact. Everything that moves energy is either an exact identity
/// (damping dissipation, loop-gain attenuation, body decay) or an exact
/// rotation (scattering).
#[test]
fn the_passive_ledger_closes_within_the_declared_residual() {
    let cases = [
        Case::new(
            "guitar_plain",
            &[
                ("synth", "pluck"),
                ("sus", "1.5"),
                ("buzz", "0"),
                ("body", "0.5"),
                ("symp", "0.4"),
                ("stiff", "0.4"),
            ],
            &[196.0],
            0.8,
            1.0,
        ),
        Case::new(
            "guitar_bent",
            &[
                ("synth", "pluck"),
                ("sus", "1.5"),
                ("buzz", "0"),
                ("body", "0.7"),
                ("stiff", "0.8"),
                ("tension", "60"),
                ("detune", "25"),
            ],
            &[110.0],
            0.8,
            1.5,
        ),
        Case::new(
            "weave_bent_legacy",
            &[
                ("synth", "weave"),
                ("sus", "2.0"),
                ("buzz", "0"),
                ("courses", "13"),
                ("couple", "0.4"),
                ("chirality", "0.8"),
                ("curvature", "0.9"),
                ("orbit", "9"),
                ("orbit_depth", "0.8"),
                ("body", "1"),
            ],
            &[82.4069, 110.0],
            0.6,
            1.5,
        ),
        Case::new(
            "weave_bent_neutral",
            &[
                ("synth", "weave"),
                ("sus", "2.0"),
                ("buzz", "0"),
                ("courses", "13"),
                ("couple", "0.4"),
                ("chirality", "0.8"),
                ("curvature", "0.9"),
                ("orbit", "9"),
                ("orbit_depth", "0.8"),
                ("body", "1"),
                ("retune", "neutral_filters"),
            ],
            &[82.4069, 110.0],
            0.6,
            1.5,
        ),
        Case::new(
            "guitar_buzzing",
            &[
                ("synth", "pluck"),
                ("sus", "1.2"),
                ("buzz", "1"),
                ("body", "0.6"),
                ("stiff", "0.5"),
            ],
            &[146.832],
            0.7,
            1.25,
        ),
    ];
    for case in &cases {
        let name = case.name;
        let (_, _, energy) = case.voice().render_traced_observed(64, &[]);
        let residual = energy.ledger_residual();
        let injected = [
            "inject_burst",
            "inject_drive",
            "inject_breath",
            "inject_bow",
        ]
        .iter()
        .map(|column| energy.last(column).expect("column"))
        .sum::<f64>();
        let lost = [
            "loss_damping",
            "loss_loop_gain",
            "loss_contact",
            "loss_body",
        ]
        .iter()
        .map(|column| energy.last(column).expect("column"))
        .sum::<f64>();
        println!(
            "{name}: initial={:.6e} injected={injected:.6e} lost={lost:.6e} \
             scatter={:.3e} body_scatter={:.3e} retune={:.3e} line={:.3e} \
             final_state={:.6e} residual={residual:.3e}",
            energy.initial_energy,
            energy.last("work_scatter").expect("column"),
            energy.last("work_body_scatter").expect("column"),
            energy.last("work_allpass_retune").expect("column"),
            energy.last("work_line_reinterpretation").expect("column"),
            energy.last("state_total").expect("column"),
        );
        assert_eq!(
            energy.last("inject_bow"),
            Some(0.0),
            "{name}: the passive regime has no bow"
        );
        assert_eq!(
            energy.last("inject_breath"),
            Some(0.0),
            "{name}: the passive regime has no breath"
        );
        assert_eq!(
            energy.last("inject_drive"),
            Some(0.0),
            "{name}: the passive regime has no sing drive"
        );
        assert!(
            residual.abs() <= LEDGER_RESIDUAL_TOLERANCE,
            "{name}: ledger residual {residual:.6e} exceeds the declared \
             {LEDGER_RESIDUAL_TOLERANCE:.0e}"
        );
    }
}

/// The scattering controls: every Weave and body rotation is an exact
/// Givens, so the measured whole-render scattering work must be roundoff,
/// not merely "small". This is the network-level version of the
/// module's orthogonality argument, measured rather than asserted.
#[test]
fn scattering_work_is_roundoff_over_a_whole_render() {
    let patch = params(&[
        ("synth", "weave"),
        ("sus", "2.0"),
        ("courses", "24"),
        ("couple", "0.45"),
        ("chirality", "1"),
        ("curvature", "1"),
        ("orbit", "20"),
        ("orbit_depth", "1"),
        ("body", "1"),
    ]);
    let voice = StringNetworkVoice::weave(&patch, &[82.4069, 110.0, 146.832], 0.45, 1.5);
    let (_, _, energy) = voice.render_traced_observed(64, &[]);
    let scatter = energy.last("work_scatter").expect("column");
    let body = energy.last("work_body_scatter").expect("column");
    let peak_state = (0..energy.frames)
        .filter_map(|frame| energy.get(frame, "state_total"))
        .fold(0.0_f64, f64::max);
    println!(
        "whole-render scatter work {scatter:.6e}, body-rotation work {body:.6e}, \
         peak state energy {peak_state:.6e}"
    );
    assert!(
        scatter.abs() <= 1e-9 * peak_state.max(1.0),
        "weave scattering did {scatter:.6e} of work on a peak state of {peak_state:.6e}"
    );
    assert!(
        body.abs() <= 1e-9 * peak_state.max(1.0),
        "body rotations did {body:.6e} of work on a peak state of {peak_state:.6e}"
    );
}

/// A bowed note is NOT passive and the telemetry says so out loud: the
/// bow appears as a positive source term. This is the honest counterpart
/// to the passive test, not a failure of it.
#[test]
fn a_bowed_note_books_the_bow_as_a_source() {
    let patch = params(&[
        ("synth", "pluck"),
        ("sus", "1.5"),
        ("bow", "0.9"),
        ("bow_speed", "0.6"),
        ("body", "0.5"),
        ("buzz", "0"),
    ]);
    let voice = StringNetworkVoice::guitar(&patch, &[196.0], 1.0, 1.0);
    let (_, _, energy) = voice.render_traced_observed(64, &[]);
    let bow = energy.last("inject_bow").expect("column");
    let burst = energy.last("inject_burst").expect("column");
    println!("bowed note: inject_bow={bow:.6e} inject_burst={burst:.6e}");
    assert!(bow > 0.0, "the bow pumps the network: inject_bow={bow:.6e}");
    assert!(
        energy.ledger_residual().abs() <= LEDGER_RESIDUAL_TOLERANCE,
        "the ledger still closes with a source in it: {:.6e}",
        energy.ledger_residual()
    );
}

// ---------------------------------------------------------------------
// Phase 0 §C findings, retained as fixtures (consensus rule 9: every
// negative result is kept as a fixture rather than tuned away). Numbers
// in the assertion messages are the measurements from
// `.string-research/PHASE0-TRANSIENT-REPORT.md`; the assertions
// themselves are inequalities with headroom, so an engine change that
// moves the phenomenon fails loudly instead of silently.
// ---------------------------------------------------------------------

/// The fixture the consensus names.
fn extreme(extra: &[(&str, &str)]) -> BTreeMap<String, String> {
    let mut patch = params(&[
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
    ]);
    for (key, value) in extra {
        patch.insert(key.to_string(), value.to_string());
    }
    patch
}

struct Peaks {
    output: f32,
    state: f64,
    body: f64,
    line: f64,
    work_allpass: f64,
    work_line: f64,
}

fn peaks(patch: &BTreeMap<String, String>, notes: &[f64], gliss: f64) -> Peaks {
    let voice = StringNetworkVoice::weave(patch, notes, 0.45, gliss);
    let (audio, _, energy) = voice.render_traced_observed(64, &[]);
    let column_peak = |name: &str| {
        (0..energy.frames)
            .filter_map(|frame| energy.get(frame, name))
            .fold(0.0_f64, f64::max)
    };
    Peaks {
        output: audio
            .iter()
            .fold(0.0_f32, |best, sample| best.max(sample.abs())),
        state: column_peak("state_total"),
        body: column_peak("body_energy"),
        line: column_peak("line_energy"),
        work_allpass: energy.last("work_allpass_retune").unwrap_or(0.0),
        work_line: energy.last("work_line_reinterpretation").unwrap_or(0.0),
    }
}

/// Phase 0 §C, the headline. The production 3.24-to-5.38 upward-fifth
/// transient is NOT stored energy: the output peak rises 66% while the
/// declared state energy of the whole network FALLS.
#[test]
fn the_bend_transient_is_not_stored_energy() {
    let patch = extreme(&[]);
    let unbent = peaks(&patch, &[82.4069, 110.0, 146.832], 1.0);
    let bent = peaks(&patch, &[82.4069, 110.0, 146.832], 1.5);
    let output_ratio = bent.output as f64 / unbent.output as f64;
    let state_ratio = bent.state / unbent.state;
    let line_ratio = bent.line / unbent.line;
    println!(
        "unbent peak {:.4}, fifth peak {:.4}, output ratio {output_ratio:.4}, \
         state ratio {state_ratio:.4}, line ratio {line_ratio:.4}",
        unbent.output, bent.output
    );
    assert!(
        (unbent.output - 3.2364).abs() < 0.02,
        "the unbent reference is still 3.24: {:.4}",
        unbent.output
    );
    assert!(
        (bent.output - 5.3818).abs() < 0.05,
        "the upward-fifth transient is still 5.38: {:.4}",
        bent.output
    );
    assert!(output_ratio > 1.6, "output ratio {output_ratio:.4}");
    assert!(
        state_ratio < 1.0,
        "the bend does not add stored energy — measured {state_ratio:.4}"
    );
    assert!(
        line_ratio > 0.97 && line_ratio < 1.01,
        "the string's own delay-line energy is flat through the bend: {line_ratio:.4}"
    );
}

/// Phase 0 §C, the attribution. Remove the body branch and the transient
/// collapses; the string network alone barely produces one.
#[test]
fn the_bend_transient_lives_in_the_body_branch() {
    let notes = [82.4069, 110.0, 146.832];
    let with_body = (
        peaks(&extreme(&[]), &notes, 1.0),
        peaks(&extreme(&[]), &notes, 1.5),
    );
    let without_body = (
        peaks(&extreme(&[("body", "0")]), &notes, 1.0),
        peaks(&extreme(&[("body", "0")]), &notes, 1.5),
    );
    let excess_with = (with_body.1.output - with_body.0.output) as f64;
    let excess_without = (without_body.1.output - without_body.0.output) as f64;
    let body_ratio = with_body.1.body / with_body.0.body;
    println!(
        "transient excess with body {excess_with:.4}, without body {excess_without:.4}; \
         body-mode energy ratio {body_ratio:.4}"
    );
    assert!(
        excess_without < 0.05 * excess_with,
        "removing the body removes the transient: {excess_without:.4} vs {excess_with:.4}"
    );
    assert!(
        body_ratio > 1.5,
        "the bend pumps the body modes: energy ratio {body_ratio:.4}"
    );
}

/// Phase 0 §C, the decisive control. A STATIC note at the bend's target
/// pitch is quieter than the unbent origin — so the transient is the
/// sweep across the fixed body resonances, not the destination pitch.
#[test]
fn the_transient_is_the_sweep_not_the_destination() {
    let patch = extreme(&[]);
    let origin = [82.4069, 110.0, 146.832];
    let target: Vec<f64> = origin.iter().map(|note| note * 1.5).collect();
    let static_origin = peaks(&patch, &origin, 1.0);
    let static_target = peaks(&patch, &target, 1.0);
    let glide_up = peaks(&patch, &origin, 1.5);
    let glide_down = peaks(&patch, &target, 2.0 / 3.0);
    println!(
        "static origin {:.4}, static target {:.4}, glide up {:.4}, glide down {:.4}",
        static_origin.output, static_target.output, glide_up.output, glide_down.output
    );
    assert!(
        static_target.output < static_origin.output,
        "the higher static note is not louder: {:.4} vs {:.4}",
        static_target.output,
        static_origin.output
    );
    assert!(
        glide_up.output > 1.6 * static_target.output,
        "gliding to a pitch is much louder than sitting on it"
    );
    assert!(
        glide_down.output > 1.5 * static_target.output,
        "the effect is not directional: a downward glide from the same \
         starting state also peaks ({:.4} vs {:.4})",
        glide_down.output,
        static_target.output
    );
}

/// Phase 0 §C, what `neutral_filters` does and does not do. It removes
/// allpass coefficient-update work entirely, leaves the delay-line
/// reinterpretation untouched (Phase 3's problem), and moves the peak by
/// only a few percent. The repair is real; it is not the explanation.
#[test]
fn neutral_filters_repairs_the_allpass_but_not_the_transient() {
    let notes = [82.4069, 110.0, 146.832];
    let legacy = peaks(&extreme(&[("retune", "legacy")]), &notes, 1.5);
    let neutral = peaks(&extreme(&[("retune", "neutral_filters")]), &notes, 1.5);
    println!(
        "legacy: peak {:.4}, allpass work {:.6e}, line work {:.6e}\n\
         neutral: peak {:.4}, allpass work {:.6e}, line work {:.6e}",
        legacy.output,
        legacy.work_allpass,
        legacy.work_line,
        neutral.output,
        neutral.work_allpass,
        neutral.work_line,
    );
    assert!(
        legacy.work_allpass.abs() > 1.0,
        "legacy allpass reinterpretation is large: {:.6e}",
        legacy.work_allpass
    );
    assert!(
        neutral.work_allpass.abs() <= 1e-12,
        "neutral allpass reinterpretation is roundoff: {:.6e}",
        neutral.work_allpass
    );
    assert!(
        neutral.work_line.abs() > 10.0,
        "the delay-line reinterpretation is untouched — Phase 3 still owns it: {:.6e}",
        neutral.work_line
    );
    assert!(
        neutral.work_line.abs() > 3.0 * legacy.work_allpass.abs(),
        "the unrepaired line term dominates the repaired allpass term"
    );
    let change = (neutral.output as f64 / legacy.output as f64 - 1.0).abs();
    assert!(
        change < 0.10,
        "neutral_filters moves the peak by {:.2}% — a few percent, not an \
         explanation",
        change * 100.0
    );
    assert!(
        neutral.output > 4.5,
        "the transient survives the allpass repair: {:.4}",
        neutral.output
    );
}
