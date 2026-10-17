//! Render one `synth=weave` string-network note offline, as a proof that
//! ovillo-dsp can reach the aigua/telar palette through `mus_dsp::pluck`.
//!
//! ```text
//! cargo run -p ovillo-dsp --example weave_probe -- [out.wav] [slot_s]
//! ```
//! Writes a stereo WAV (the mono voice on both channels) and prints the
//! peak, so a silent output is obvious.

use std::collections::BTreeMap;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "weave_probe.wav".into());
    let slot_s: f64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2.0);

    // The "ml" (stated tune) instrument from the aigua/Gaily scores: a bowed,
    // violin-ish string network. Every key is a plain string; unknown keys are
    // ignored by the voice.
    let params: BTreeMap<String, String> = [
        ("synth", "weave"),
        ("bodyp", "violin"),
        ("courses", "7"),
        ("dimension", "1.72"),
        ("couple", "0.065"),
        ("chirality", "0.52"),
        ("orbit", "0.13"),
        ("orbit_depth", "0.30"),
        ("curvature", "0.10"),
        ("bow", "0.18"),
        ("bow_speed", "0.42"),
        ("sus", "3.4"),
        ("damp", "0.25"),
        ("pos", "0.34"),
        ("pick", "0.34"),
        ("body", "0.68"),
        ("body_size", "0.96"),
        ("stiff", "0.08"),
        ("tension", "4"),
        ("detune", "3"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

    // G3 Bb3 D4 F4 — the tune's home chord, held `slot_s` seconds.
    let freqs = vec![196.0, 233.08, 293.66, 349.23];
    let note = mus_dsp::pluck::weave_note(&params, &freqs, slot_s, 1.0);

    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48_000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(&out, spec).expect("create WAV");
    for &s in &note {
        w.write_sample(s).expect("write L");
        w.write_sample(s).expect("write R");
    }
    w.finalize().expect("finalize WAV");

    let peak = note.iter().map(|v| v.abs()).fold(0.0_f32, f32::max);
    println!(
        "wrote {out}: {} samples ({slot_s} s), peak {peak:.4}",
        note.len()
    );
}
