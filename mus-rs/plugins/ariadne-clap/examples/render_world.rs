//! Offline WAV face for the exact same World + listener engine as the plug-in.
//!
//! Usage:
//! `cargo run -p ariadne-clap --release --example render_world -- INPUT OUTPUT [TAIL_SECONDS] [ORGANIZATION]`

use std::env;
use std::path::{Path, PathBuf};

use ariadne_clap::{AriadneControls, AriadneEngine};

fn main() {
    if let Err(error) = run() {
        eprintln!("ariadne-world-render: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or_else(usage)?);
    let output = PathBuf::from(args.next().ok_or_else(usage)?);
    let tail_seconds = args
        .next()
        .map(|value| {
            value
                .parse::<f64>()
                .map_err(|_| format!("invalid tail {value:?}"))
        })
        .transpose()?
        .unwrap_or(4.0);
    let organization = args
        .next()
        .map(|value| {
            value
                .parse::<f64>()
                .map_err(|_| format!("invalid organization {value:?}"))
        })
        .transpose()?
        .unwrap_or(AriadneControls::default().material_organization);
    if args.next().is_some()
        || !tail_seconds.is_finite()
        || !(0.0..=60.0).contains(&tail_seconds)
        || !organization.is_finite()
        || !(0.0..=1.0).contains(&organization)
    {
        return Err(usage());
    }

    let (input_frames, sample_rate) = read_wav(&input)?;
    let mut engine = AriadneEngine::new(sample_rate as f64)?;
    let controls = AriadneControls {
        material_organization: organization,
        ..AriadneControls::default()
    };
    let tail_frames = (tail_seconds * sample_rate as f64).round() as usize;
    let mut output_frames = Vec::with_capacity(input_frames.len() + tail_frames);
    for [left, right] in input_frames {
        let frame = engine.process_sample(left, right, controls);
        output_frames.push([frame.left, frame.right]);
    }
    for _ in 0..tail_frames {
        let frame = engine.process_sample(0.0, 0.0, controls);
        output_frames.push([frame.left, frame.right]);
    }
    write_wav(&output, sample_rate, &output_frames)?;

    let receipt = engine.receipt();
    let (peak, rms) = level_stats(&output_frames);
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": "mus.ariadne-world-render-receipt.v2",
            "input": input.display().to_string(),
            "output": output.display().to_string(),
            "sampleRate": sample_rate,
            "tailFrames": tail_frames,
            "materialOrganization": organization,
            "physicalWorldDigest": receipt.world.plan_digest,
            "listenerDigest": receipt.listener_digest,
            "worldFrames": receipt.world.sample_frames,
            "bodyCount": receipt.world.body_count,
            "globalClosureResidual": receipt.world.global_closure_residual,
            "maximumLocalResidual": receipt.world.maximum_local_residual,
            "nonFiniteInputFrames": receipt.non_finite_input_frames,
            "worldErrors": receipt.world_errors,
            "safetyLimitedFrames": receipt.safety_limited_frames,
            "peakObservedBeforeMix": receipt.peak_observed,
            "peakOutputAfterSafety": receipt.peak_output,
            "peak": peak,
            "rms": rms,
            "peakDbfs": amplitude_db(peak),
            "rmsDbfs": amplitude_db(rms),
        }))
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn read_wav(path: &Path) -> Result<(Vec<[f32; 2]>, u32), String> {
    let mut reader =
        hound::WavReader::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let spec = reader.spec();
    let channels = spec.channels as usize;
    if channels == 0 {
        return Err("WAV declares zero channels".to_owned());
    }
    let samples = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?,
        (hound::SampleFormat::Int, bits @ 1..=16) => {
            let scale = 2.0_f32.powi(bits as i32 - 1);
            reader
                .samples::<i16>()
                .map(|sample| sample.map(|value| value as f32 / scale))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?
        }
        (hound::SampleFormat::Int, bits @ 17..=32) => {
            let scale = 2.0_f64.powi(bits as i32 - 1);
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|value| (value as f64 / scale) as f32))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?
        }
        _ => return Err("unsupported WAV encoding".to_owned()),
    };
    if samples.len() % channels != 0 {
        return Err("WAV sample count is not a whole number of frames".to_owned());
    }
    let frames = samples
        .chunks_exact(channels)
        .map(|frame| match channels {
            1 => [frame[0], frame[0]],
            2 => [frame[0], frame[1]],
            _ => {
                let mono = frame.iter().copied().sum::<f32>() / channels as f32;
                [mono, mono]
            }
        })
        .collect();
    Ok((frames, spec.sample_rate))
}

fn write_wav(path: &Path, sample_rate: u32, frames: &[[f32; 2]]) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    for frame in frames {
        for sample in frame {
            writer
                .write_sample(*sample)
                .map_err(|error| error.to_string())?;
        }
    }
    writer.finalize().map_err(|error| error.to_string())
}

fn level_stats(frames: &[[f32; 2]]) -> (f64, f64) {
    let mut peak = 0.0_f64;
    let mut sum_squares = 0.0_f64;
    let mut count = 0_u64;
    for sample in frames.iter().flatten() {
        let sample = *sample as f64;
        peak = peak.max(sample.abs());
        sum_squares += sample * sample;
        count += 1;
    }
    let rms = if count == 0 {
        0.0
    } else {
        (sum_squares / count as f64).sqrt()
    };
    (peak, rms)
}

fn amplitude_db(value: f64) -> f64 {
    if value > 0.0 {
        20.0 * value.log10()
    } else {
        f64::NEG_INFINITY
    }
}

fn usage() -> String {
    "usage: render_world INPUT.wav OUTPUT.wav [TAIL_SECONDS] [ORGANIZATION]".to_owned()
}
