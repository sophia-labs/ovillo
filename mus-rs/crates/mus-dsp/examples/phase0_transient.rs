//! Ariadne Phase 0 §C — locate the production 3.24-to-5.38 bend transient.
//!
//! The consensus mandate is to FIND the internal source of the extreme
//! Weave patch's upward-fifth peak rather than attributing it to a stage
//! by assumption. `NEXT-PHASE-REPORT.md` §6.2 establishes that the
//! isolated changing-delay reference model does not reproduce it, so it
//! lives in something that model omits.
//!
//! This harness prints the evidence the report cites:
//!
//! 1. the baseline reproduction (unbent vs upward fifth, both retune
//!    modes), with peak OUTPUT and peak declared STATE energy side by
//!    side — the discriminating pair;
//! 2. a zero-limit bisection, using the switches the existing zero-limit
//!    laws already prove removable;
//! 3. the per-stage energy trace through the peak;
//! 4. the whole-render work terms, allpass and delay-line separately.
//!
//! Run: `cargo run -p mus-dsp --release --example phase0_transient`
//!
//! Consensus rule 7: no master limiter is involved anywhere here. These
//! are raw voice renders.

use std::collections::BTreeMap;

use mus_dsp::pluck::StringNetworkVoice;
use mus_dsp::SR_F64;

/// The fixture the consensus names: the extreme Weave patch from
/// `pluck_invariants::bounded_and_finite_over_extreme_controls`.
const EXTREME: &[(&str, &str)] = &[
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
];

const NOTES: [f64; 3] = [82.4069, 110.0, 146.832];
const SLOT: f64 = 0.45;
const STRIDE: usize = 64;

fn patch_with(extra: &[(&str, &str)]) -> BTreeMap<String, String> {
    let mut map: BTreeMap<String, String> = EXTREME
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    for (key, value) in extra {
        map.insert(key.to_string(), value.to_string());
    }
    map
}

struct Measurement {
    peak_output: f32,
    peak_body: f64,
    peak_line: f64,
    peak_filter: f64,
    peak_output_at_s: f64,
    peak_state: f64,
    peak_state_at_s: f64,
    final_state: f64,
    work_allpass: f64,
    work_line: f64,
    work_scatter: f64,
    work_body_scatter: f64,
    injected: f64,
    lost: f64,
    residual: f64,
    frames: usize,
    peak_frame: usize,
}

fn measure(patch: &BTreeMap<String, String>, gliss: f64) -> Measurement {
    measure_notes(patch, &NOTES, gliss)
}

fn measure_notes(patch: &BTreeMap<String, String>, notes: &[f64], gliss: f64) -> Measurement {
    let voice = StringNetworkVoice::weave(patch, notes, SLOT, gliss);
    let (audio, _, energy) = voice.render_traced_observed(STRIDE, &[]);
    let (peak_index, peak_output) =
        audio
            .iter()
            .enumerate()
            .fold((0_usize, 0.0_f32), |(best_index, best), (index, sample)| {
                if sample.abs() > best {
                    (index, sample.abs())
                } else {
                    (best_index, best)
                }
            });
    let mut peak_state = 0.0_f64;
    let mut peak_state_frame = 0_usize;
    let mut peak_body = 0.0_f64;
    let mut peak_line = 0.0_f64;
    let mut peak_filter = 0.0_f64;
    for frame in 0..energy.frames {
        let value = energy.get(frame, "state_total").unwrap_or(0.0);
        if value > peak_state {
            peak_state = value;
            peak_state_frame = frame;
        }
        peak_body = peak_body.max(energy.get(frame, "body_energy").unwrap_or(0.0));
        peak_line = peak_line.max(energy.get(frame, "line_energy").unwrap_or(0.0));
        peak_filter = peak_filter.max(energy.get(frame, "filter_energy").unwrap_or(0.0));
    }
    let sum = |columns: &[&str]| -> f64 {
        columns
            .iter()
            .map(|column| energy.last(column).unwrap_or(0.0))
            .sum()
    };
    Measurement {
        peak_output,
        peak_body,
        peak_line,
        peak_filter,
        peak_output_at_s: peak_index as f64 / SR_F64,
        peak_state,
        peak_state_at_s: (peak_state_frame * STRIDE) as f64 / SR_F64,
        final_state: energy.last("state_total").unwrap_or(0.0),
        work_allpass: energy.last("work_allpass_retune").unwrap_or(0.0),
        work_line: energy.last("work_line_reinterpretation").unwrap_or(0.0),
        work_scatter: energy.last("work_scatter").unwrap_or(0.0),
        work_body_scatter: energy.last("work_body_scatter").unwrap_or(0.0),
        injected: sum(&[
            "inject_burst",
            "inject_drive",
            "inject_breath",
            "inject_bow",
        ]),
        lost: sum(&[
            "loss_damping",
            "loss_loop_gain",
            "loss_contact",
            "loss_body",
        ]),
        residual: energy.ledger_residual(),
        frames: energy.frames,
        peak_frame: peak_index / STRIDE,
    }
}

fn rule(title: &str) {
    println!("\n=== {title} ===");
}

fn main() {
    println!("Ariadne Phase 0 §C — production bend transient hunt");
    println!("fixture: extreme weave patch, notes {NOTES:?}, slot {SLOT} s, stride {STRIDE}");
    println!("no master limiter is applied anywhere in this harness (consensus rule 7)");

    rule("1. baseline reproduction: output peak vs declared state peak");
    println!(
        "{:<18} {:>7} {:>10} {:>12} {:>10} {:>12} {:>12}",
        "mode/bend", "peak", "peak@s", "peak_state", "state@s", "final_state", "out/state"
    );
    let mut baseline = Vec::new();
    for mode in ["legacy", "neutral_filters"] {
        for (label, gliss) in [("unbent", 1.0), ("fifth", 1.5)] {
            let patch = patch_with(&[("retune", mode)]);
            let measurement = measure(&patch, gliss);
            println!(
                "{:<18} {:>7.4} {:>10.4} {:>12.4} {:>10.4} {:>12.4e} {:>12.4}",
                format!("{mode}/{label}"),
                measurement.peak_output,
                measurement.peak_output_at_s,
                measurement.peak_state,
                measurement.peak_state_at_s,
                measurement.final_state,
                measurement.peak_output as f64 / measurement.peak_state.max(1e-30).sqrt(),
            );
            baseline.push((mode, label, measurement));
        }
    }
    for mode in ["legacy", "neutral_filters"] {
        let unbent = baseline
            .iter()
            .find(|(m, l, _)| *m == mode && *l == "unbent")
            .expect("unbent row");
        let bent = baseline
            .iter()
            .find(|(m, l, _)| *m == mode && *l == "fifth")
            .expect("bent row");
        println!(
            "{mode}: output peak ratio {:.4}, state peak ratio {:.4}, \
             injected ratio {:.4}",
            bent.2.peak_output as f64 / unbent.2.peak_output as f64,
            bent.2.peak_state / unbent.2.peak_state,
            bent.2.injected / unbent.2.injected,
        );
    }

    rule("2. zero-limit bisection (upward fifth, legacy)");
    println!(
        "{:<26} {:>8} {:>8} {:>8} {:>12} {:>12}",
        "removed stage", "unbent", "fifth", "ratio", "state_fifth", "state_ratio"
    );
    let switches: &[(&str, &[(&str, &str)])] = &[
        ("(nothing: baseline)", &[]),
        ("body=0", &[("body", "0")]),
        ("couple=0 (no scattering)", &[("couple", "0")]),
        ("body=0 couple=0", &[("body", "0"), ("couple", "0")]),
        ("curvature=0", &[("curvature", "0")]),
        ("chirality=0", &[("chirality", "0")]),
        ("orbit_depth=0", &[("orbit_depth", "0")]),
        ("courses=3 (played only)", &[("courses", "3")]),
        ("buzz=1 (contact on)", &[("buzz", "1")]),
        ("detune=60", &[("detune", "60")]),
        ("damp=0", &[("damp", "0")]),
        ("stiff=1", &[("stiff", "1")]),
        ("single note", &[]),
    ];
    for (label, extra) in switches {
        let patch = patch_with(extra);
        let (unbent, bent) = if *label == "single note" {
            let one = |gliss: f64| {
                let voice = StringNetworkVoice::weave(&patch, &NOTES[..1], SLOT, gliss);
                let (audio, _, energy) = voice.render_traced_observed(STRIDE, &[]);
                let peak = audio
                    .iter()
                    .fold(0.0_f32, |best, sample| best.max(sample.abs()));
                let state = (0..energy.frames)
                    .filter_map(|frame| energy.get(frame, "state_total"))
                    .fold(0.0_f64, f64::max);
                (peak, state)
            };
            let (pu, su) = one(1.0);
            let (pb, sb) = one(1.5);
            ((pu, su), (pb, sb))
        } else {
            let u = measure(&patch, 1.0);
            let b = measure(&patch, 1.5);
            ((u.peak_output, u.peak_state), (b.peak_output, b.peak_state))
        };
        println!(
            "{:<26} {:>8.4} {:>8.4} {:>8.4} {:>12.4} {:>12.4}",
            label,
            unbent.0,
            bent.0,
            bent.0 as f64 / unbent.0.max(1e-30) as f64,
            bent.1,
            bent.1 / unbent.1.max(1e-30),
        );
    }

    rule("3. per-stage energy trace through the peak (legacy, upward fifth)");
    let patch = patch_with(&[("retune", "legacy")]);
    let voice = StringNetworkVoice::weave(&patch, &NOTES, SLOT, 1.5);
    let (audio, _, energy) = voice.render_traced_observed(STRIDE, &[]);
    let peak_index = audio
        .iter()
        .enumerate()
        .fold((0_usize, 0.0_f32), |(best_index, best), (index, sample)| {
            if sample.abs() > best {
                (index, sample.abs())
            } else {
                (best_index, best)
            }
        })
        .0;
    let peak_frame = peak_index / STRIDE;
    println!(
        "{:>8} {:>8} {:>11} {:>11} {:>11} {:>11} {:>11} {:>11}",
        "frame", "t(s)", "|out|max", "state", "line", "filter", "body", "d_out"
    );
    let low = peak_frame.saturating_sub(12);
    let high = (peak_frame + 13).min(energy.frames);
    let mut previous_output = if low == 0 {
        0.0
    } else {
        energy.get(low - 1, "output_energy").unwrap_or(0.0)
    };
    for frame in low..high {
        let start = frame * STRIDE;
        let end = (start + STRIDE).min(audio.len());
        let window_peak = audio[start..end]
            .iter()
            .fold(0.0_f32, |best, sample| best.max(sample.abs()));
        let output_energy = energy.get(frame, "output_energy").unwrap_or(0.0);
        println!(
            "{:>8} {:>8.4} {:>11.5} {:>11.5} {:>11.5} {:>11.5} {:>11.5} {:>11.3e}{}",
            frame,
            start as f64 / SR_F64,
            window_peak,
            energy.get(frame, "state_total").unwrap_or(0.0),
            energy.get(frame, "line_energy").unwrap_or(0.0),
            energy.get(frame, "filter_energy").unwrap_or(0.0),
            energy.get(frame, "body_energy").unwrap_or(0.0),
            output_energy - previous_output,
            if frame == peak_frame { "  <- peak" } else { "" },
        );
        previous_output = output_energy;
    }

    rule("4. whole-render work terms");
    println!(
        "{:<18} {:>14} {:>14} {:>12} {:>14} {:>12} {:>12} {:>12}",
        "mode/bend",
        "allpass_retune",
        "line_reinterp",
        "scatter",
        "body_scatter",
        "injected",
        "lost",
        "residual"
    );
    for mode in ["legacy", "neutral_filters"] {
        for (label, gliss) in [("unbent", 1.0), ("fifth", 1.5)] {
            let patch = patch_with(&[("retune", mode)]);
            let m = measure(&patch, gliss);
            println!(
                "{:<18} {:>14.6e} {:>14.6e} {:>12.3e} {:>14.3e} {:>12.4} {:>12.4} {:>12.3e}",
                format!("{mode}/{label}"),
                m.work_allpass,
                m.work_line,
                m.work_scatter,
                m.work_body_scatter,
                m.injected,
                m.lost,
                m.residual,
            );
            let _ = (m.frames, m.peak_frame);
        }
    }

    rule("5. output-term decomposition by zero limit (upward fifth, legacy)");
    // The output sum is
    //   0.18*string_direct + (0.92+0.52*body)*body_output
    //   + 0.18*bridge_force + 0.20*contact_output
    // and `body` scales BOTH the body term's gain and the body coupling
    // that fills it, so `body=0` removes the whole branch. `couple=0`
    // empties `bridge_force` of its Weave contribution. Comparing peaks
    // with each branch removed attributes the peak to a branch.
    for (label, extra) in [
        ("all branches", &[][..]),
        ("body branch removed", &[("body", "0")][..]),
        ("weave scattering removed", &[("couple", "0")][..]),
        (
            "both removed (courses alone)",
            &[("body", "0"), ("couple", "0")][..],
        ),
    ] {
        let patch = patch_with(extra);
        let unbent = measure(&patch, 1.0);
        let bent = measure(&patch, 1.5);
        println!(
            "{:<30} unbent {:>7.4}  fifth {:>7.4}  ratio {:>6.4}  \
             transient excess {:>7.4}",
            label,
            unbent.peak_output,
            bent.peak_output,
            bent.peak_output as f64 / unbent.peak_output.max(1e-30) as f64,
            bent.peak_output - unbent.peak_output,
        );
    }

    rule("6. which stored sub-state moves with the bend? (legacy)");
    println!(
        "{:<12} {:>11} {:>11} {:>11} {:>11}",
        "bend", "peak_out", "peak_line", "peak_filter", "peak_body"
    );
    let mut rows = Vec::new();
    for (label, gliss) in [
        ("2/3 down", 2.0 / 3.0),
        ("unbent", 1.0),
        ("4/3 up", 4.0 / 3.0),
        ("fifth up", 1.5),
        ("octave up", 2.0),
    ] {
        let patch = patch_with(&[("retune", "legacy")]);
        let m = measure(&patch, gliss);
        println!(
            "{:<12} {:>11.4} {:>11.4} {:>11.4} {:>11.4}",
            label, m.peak_output, m.peak_line, m.peak_filter, m.peak_body
        );
        rows.push((label, m));
    }
    let unbent = &rows.iter().find(|(l, _)| *l == "unbent").expect("unbent").1;
    println!("ratios against unbent:");
    for (label, m) in &rows {
        println!(
            "  {:<12} out {:>7.4}  line {:>7.4}  filter {:>7.4}  body {:>7.4}",
            label,
            m.peak_output as f64 / unbent.peak_output as f64,
            m.peak_line / unbent.peak_line,
            m.peak_filter / unbent.peak_filter,
            m.peak_body / unbent.peak_body,
        );
    }

    rule("7. moving the body modes moves the transient (falsifiable prediction)");
    // If the peak is a bend sweeping string partials across FIXED body
    // resonances, retuning the body must change it. If the peak were a
    // string-state phenomenon, the body table would be irrelevant.
    println!(
        "{:<24} {:>9} {:>9} {:>8} {:>11}",
        "body variation", "unbent", "fifth", "ratio", "peak_body"
    );
    for (label, extra) in [
        ("bodyp=guitar (default)", &[][..]),
        ("bodyp=violin", &[("bodyp", "violin")][..]),
        ("bodyp=cello", &[("bodyp", "cello")][..]),
        ("bodyp=bass", &[("bodyp", "bass")][..]),
        ("bodyp=mandolin", &[("bodyp", "mandolin")][..]),
        ("body_size=0.45", &[("body_size", "0.45")][..]),
        ("body_size=2.4", &[("body_size", "2.4")][..]),
    ] {
        let patch = patch_with(extra);
        let u = measure(&patch, 1.0);
        let b = measure(&patch, 1.5);
        println!(
            "{:<24} {:>9.4} {:>9.4} {:>8.4} {:>11.4}",
            label,
            u.peak_output,
            b.peak_output,
            b.peak_output as f64 / u.peak_output as f64,
            b.peak_body,
        );
    }

    rule("8. sweep or destination? static notes at the bend endpoints");
    // The decisive control. If the transient were simply "higher notes
    // excite the body harder", a STATIC note at the target pitch would
    // show the same body energy and the same peak. If it is the SWEEP
    // dragging partials across fixed resonances, the static endpoint
    // stays quiet and only the glide is loud.
    println!(
        "{:<34} {:>9} {:>11} {:>11}",
        "fixture", "peak_out", "peak_body", "peak_line"
    );
    let patch = patch_with(&[("retune", "legacy")]);
    let scaled: Vec<f64> = NOTES.iter().map(|note| note * 1.5).collect();
    for (label, notes, gliss) in [
        ("static at origin (unbent)", &NOTES[..], 1.0),
        ("static at target (1.5x, unbent)", &scaled[..], 1.0),
        ("glide origin -> target", &NOTES[..], 1.5),
        ("glide target -> origin (2/3)", &scaled[..], 2.0 / 3.0),
    ] {
        let m = measure_notes(&patch, notes, gliss);
        println!(
            "{:<34} {:>9.4} {:>11.4} {:>11.4}",
            label, m.peak_output, m.peak_body, m.peak_line
        );
    }
}
