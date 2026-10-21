//! Play a score's piano parts on their Ariadne worlds and write what the
//! World itself emits, before the engine's bus and master (for measuring):
//!
//! ```text
//! cargo run --release -p mus-engine --example piano_world -- score.mus out/prefix
//! ```
//!
//! writes `prefix.world.wav` (stereo f32, the listener's observation times the
//! instrument gain, from the score's time zero), `prefix.receipt.json`,
//! `prefix.energy.json` and `prefix.notes.tsv` (onset, midi, velocity,
//! release, contact and peak force of every strike).

use std::io::Write;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (score, prefix) = match args.as_slice() {
        [_, score, prefix] => (Path::new(score), prefix.as_str()),
        _ => {
            eprintln!("usage: piano_world <score.mus> <out/prefix>");
            std::process::exit(2);
        }
    };
    let source = std::fs::read_to_string(score).expect("score");
    let dir = score.parent().unwrap_or(Path::new("."));
    let started = std::time::Instant::now();
    let parts = mus_engine::ariadne_piano::play_score(&source, dir).expect("render");
    let wall = started.elapsed().as_secs_f64();
    let part = parts.first().expect("a piano part");
    let total = part.start + part.left.len();
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48_000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut wav = hound::WavWriter::create(format!("{prefix}.world.wav"), spec).expect("wav");
    let gain = part.gain as f32;
    for k in 0..total {
        let (l, r) = if k >= part.start {
            (
                part.left[k - part.start] * gain,
                part.right[k - part.start] * gain,
            )
        } else {
            (0.0, 0.0)
        };
        wav.write_sample(l).unwrap();
        wav.write_sample(r).unwrap();
    }
    wav.finalize().unwrap();
    let mut receipt = serde_json::to_value(&part.receipt).unwrap();
    receipt["wallSeconds"] = serde_json::json!(wall);
    std::fs::write(
        format!("{prefix}.receipt.json"),
        serde_json::to_string_pretty(&receipt).unwrap(),
    )
    .unwrap();
    let trace =
        serde_json::json!({"schema": "mus.ariadne.energy-trace.v1", "worlds": [&part.trace]});
    std::fs::write(
        format!("{prefix}.energy.json"),
        serde_json::to_string(&trace).unwrap(),
    )
    .unwrap();
    let mut tsv = std::fs::File::create(format!("{prefix}.notes.tsv")).unwrap();
    writeln!(
        tsv,
        "onset_s\tmidi\tname\tvelocity_m_s\trelease_s\tcontact_ms\tpeak_force_n\tstaff"
    )
    .unwrap();
    for s in &part.trace.strikes {
        writeln!(
            tsv,
            "{:.6}\t{}\t{}\t{:.4}\t{:.6}\t{:.4}\t{:.3}\t{}",
            s.seconds,
            s.midi,
            s.name,
            s.hammer_velocity,
            s.release_seconds,
            s.contact_ms,
            s.peak_force_n,
            s.staff
        )
        .unwrap();
    }
    eprintln!(
        "{prefix}: {:.2} s simulated in {wall:.1} s; ledger max frame {:.3e} J, cumulative {:.3e} J",
        part.receipt.simulated_seconds, part.receipt.maximum_frame_residual, part.receipt.cumulative_closure_residual
    );
}
