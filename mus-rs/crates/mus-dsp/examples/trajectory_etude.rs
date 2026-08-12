//! Render a small direct-control Weave etude and print its trajectory receipt.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;

use mus_dsp::trajectory::StringTrajectoryVoice;

fn main() -> Result<(), Box<dyn Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("trajectory-etude.wav"));
    let params = BTreeMap::from([
        ("synth".to_string(), "weave".to_string()),
        ("sus".to_string(), "1.4".to_string()),
        ("courses".to_string(), "11".to_string()),
        ("couple".to_string(), "0.04->0.40->0.12".to_string()),
        ("body".to_string(), "0.25->0.72".to_string()),
        ("curvature".to_string(), "0.10->0.90->0.30".to_string()),
        ("orbit".to_string(), "0.4->1.3".to_string()),
        ("orbit_depth".to_string(), "0.65".to_string()),
        ("chirality".to_string(), "0.55".to_string()),
        ("trajectory_curve".to_string(), "smootherstep".to_string()),
    ]);
    let voice = StringTrajectoryVoice::weave(&params, &[110.0, 164.813_778], 1.25, 1.0)?;
    println!("{}", serde_json::to_string_pretty(voice.receipt())?);
    let audio = voice.render();

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48_000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(&output, spec)?;
    for sample in audio {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    eprintln!("wrote {}", output.display());
    Ok(())
}
