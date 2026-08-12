use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use mus_dsp::trajectory::{program_from_inline_string_params, render_weave_program};

fn patch() -> BTreeMap<String, String> {
    [
        ("synth", "weave"),
        ("sus", "5.0"),
        ("damp", "0.24"),
        ("courses", "13"),
        ("couple", "0.12@0->0.42@0.60->0.18@1"),
        ("chirality", "0.55"),
        ("orbit", "0.22->1.40->0.31"),
        ("orbit_depth", "0.35"),
        ("curvature", "0.20->0.82->0.28"),
        ("body", "0.30->0.76->0.38"),
        ("pitch_ratio", "1->1.0594630943592953"),
        ("retune", "neutral_filters"),
        ("traj_curve", "cosine"),
        ("traj_hz", "200"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect()
}

fn receipt_path(audio_path: &Path) -> PathBuf {
    let mut path = audio_path.as_os_str().to_os_string();
    path.push(".trajectory.json");
    PathBuf::from(path)
}

fn main() -> Result<(), Box<dyn Error>> {
    let output = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("trajectory-etude.wav"));
    let (static_patch, program) = program_from_inline_string_params(&patch(), 2.0)?;
    let program = program.expect("the example carries registered trajectories");
    let rendered = render_weave_program(
        &static_patch,
        &[110.0, 165.0],
        2.0,
        1.0,
        &program,
        256,
    )?;

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: mus_dsp::SR as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(&output, spec)?;
    for sample in &rendered.audio {
        writer.write_sample(*sample)?;
    }
    writer.finalize()?;

    let receipt = receipt_path(&output);
    std::fs::write(&receipt, serde_json::to_vec_pretty(&rendered.receipt)?)?;
    println!("audio: {}", output.display());
    println!("trajectory receipt: {}", receipt.display());
    println!("program digest: {}", rendered.receipt.trajectory.program_digest);
    println!(
        "unresolved obligations: {}",
        rendered.receipt.unresolved_obligations.join(", ")
    );
    Ok(())
}
