use mus_dsp::powered_radiator::{
    PoweredRadiator, PoweredRadiatorControls, PoweredRadiatorSpace, PoweredRadiatorTone,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct SourceReceipt {
    path: String,
    sha256: String,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    sample_format: String,
    frames: usize,
    duration_seconds: f64,
    peak: f32,
    rms: f64,
}

#[derive(Serialize)]
struct SegmentReceipt {
    name: &'static str,
    controls: Option<PoweredRadiatorControls>,
    output_peak_before_monitor_gain: f32,
    output_rms_before_monitor_gain: f64,
    dsp: Option<mus_dsp::powered_radiator::PoweredRadiatorReceipt>,
}

struct ProcessedSegment {
    left: Vec<f32>,
    right: Vec<f32>,
    receipt: SegmentReceipt,
}

#[derive(Serialize)]
struct AuditionReceipt {
    schema: &'static str,
    source: SourceReceipt,
    tail_frames: usize,
    monitor_gain: f32,
    output_peak: f32,
    segments: Vec<SegmentReceipt>,
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn file_sha256(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn read_wav(path: &Path) -> Result<(Vec<f32>, SourceReceipt), Box<dyn Error>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    if spec.channels == 0 {
        return Err(invalid_data("WAV declares zero channels").into());
    }

    let interleaved = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Float, 32) => {
            reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?
        }
        (hound::SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .map(|sample| sample.map(|sample| sample as f32 / 32_768.0))
            .collect::<Result<Vec<_>, _>>()?,
        (hound::SampleFormat::Int, bits @ 17..=32) => {
            let scale = (1_u64 << (bits - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|sample| sample as f32 / scale))
                .collect::<Result<Vec<_>, _>>()?
        }
        (format, bits) => {
            return Err(invalid_data(format!(
                "unsupported WAV encoding: {format:?}, {bits} bits; expected PCM16/24/32 or float32"
            ))
            .into())
        }
    };

    let channels = usize::from(spec.channels);
    if interleaved.len() % channels != 0 {
        return Err(invalid_data("WAV ends in a partial interleaved frame").into());
    }

    let mut mono = Vec::with_capacity(interleaved.len() / channels);
    for frame in interleaved.chunks_exact(channels) {
        let sample = frame.iter().map(|sample| *sample as f64).sum::<f64>() / channels as f64;
        if !sample.is_finite() {
            return Err(invalid_data("WAV contains a non-finite sample").into());
        }
        mono.push(sample as f32);
    }
    if mono.is_empty() {
        return Err(invalid_data("WAV contains no audio frames").into());
    }

    let receipt = SourceReceipt {
        path: path.display().to_string(),
        sha256: file_sha256(path)?,
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        bits_per_sample: spec.bits_per_sample,
        sample_format: format!("{:?}", spec.sample_format).to_lowercase(),
        frames: mono.len(),
        duration_seconds: mono.len() as f64 / spec.sample_rate as f64,
        peak: mono_peak(&mono),
        rms: mono_rms(&mono),
    };
    Ok((mono, receipt))
}

fn process(
    source: &[f32],
    sample_rate: u32,
    tail_frames: usize,
    controls: PoweredRadiatorControls,
) -> Result<ProcessedSegment, Box<dyn Error>> {
    let mut body = PoweredRadiator::new(sample_rate as f64)?;
    let mut left = Vec::with_capacity(source.len() + tail_frames);
    left.extend_from_slice(source);
    left.resize(source.len() + tail_frames, 0.0);
    let mut right = left.clone();
    body.process_block(&mut left, &mut right, controls);
    let receipt = SegmentReceipt {
        name: "",
        controls: Some(controls),
        output_peak_before_monitor_gain: stereo_peak(&left, &right),
        output_rms_before_monitor_gain: stereo_rms(&left, &right),
        dsp: Some(body.receipt()),
    };
    Ok(ProcessedSegment {
        left,
        right,
        receipt,
    })
}

fn mono_peak(audio: &[f32]) -> f32 {
    audio
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

fn mono_rms(audio: &[f32]) -> f64 {
    let energy: f64 = audio.iter().map(|sample| (*sample as f64).powi(2)).sum();
    (energy / audio.len().max(1) as f64).sqrt()
}

fn stereo_peak(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .chain(right)
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

fn stereo_rms(left: &[f32], right: &[f32]) -> f64 {
    let energy: f64 = left
        .iter()
        .chain(right)
        .map(|sample| (*sample as f64).powi(2))
        .sum();
    (energy / (left.len() + right.len()).max(1) as f64).sqrt()
}

fn write_wav(
    path: &Path,
    left: &[f32],
    right: &[f32],
    sample_rate: u32,
    gain: f32,
) -> hound::Result<()> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 24,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    let scale = 8_388_607.0_f32;
    for (&left, &right) in left.iter().zip(right) {
        writer.write_sample((left * gain).clamp(-1.0, 1.0).mul_add(scale, 0.0).round() as i32)?;
        writer.write_sample((right * gain).clamp(-1.0, 1.0).mul_add(scale, 0.0).round() as i32)?;
    }
    writer.finalize()
}

fn usage(program: &str) -> String {
    format!(
        "usage: {program} INPUT.wav OUTPUT.wav\n\
         example: {program} artifacts/guitarjam/source/050.wav artifacts/guitarjam/050-audition.wav"
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args();
    let program = args
        .next()
        .unwrap_or_else(|| "powered_radiator_audition".to_owned());
    let Some(input) = args.next().map(PathBuf::from) else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, usage(&program)).into());
    };
    let Some(output) = args.next().map(PathBuf::from) else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, usage(&program)).into());
    };
    if args.next().is_some() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, usage(&program)).into());
    }
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }

    let (source, source_receipt) = read_wav(&input)?;
    let probe = PoweredRadiator::new(source_receipt.sample_rate as f64)?;
    // Five seconds is enough to hear the audition tails without turning the
    // five-segment montage into a multi-minute file. The plugin itself still
    // reports the full conservative tail returned by `tail_samples()`.
    let tail_frames = (probe.tail_samples() as usize).min(source_receipt.sample_rate as usize * 5);

    let mut dry_left = vec![0.0; probe.latency_samples() as usize];
    dry_left.extend_from_slice(&source);
    dry_left.resize(source.len() + tail_frames, 0.0);
    let dry_right = dry_left.clone();
    let base = PoweredRadiatorControls {
        drive: 0.58,
        headroom: 0.70,
        bias: 0.18,
        tone: 0.62,
        bloom: 0.52,
        sustain: 0.55,
        sag: 0.42,
        interlace: 0.50,
        chirality: 0.38,
        motion_hz: 0.32,
        curvature: 0.38,
        feedback: 0.09,
        space: 0.0,
        room_character: 0.0,
        room_decay: 0.42,
        mix: 1.0,
        output_gain: 1.0,
    };
    let profiles = [
        (
            "costa-clara-plate",
            PoweredRadiatorControls {
                drive: 0.28,
                headroom: 0.92,
                bloom: 0.32,
                sustain: 0.36,
                interlace: 0.36,
                feedback: 0.03,
                ..base
            }
            .with_tone(PoweredRadiatorTone::CostaClara)
            .with_space(PoweredRadiatorSpace::Plate, 0.42),
        ),
        (
            "edge-spring",
            base.with_tone(PoweredRadiatorTone::Edge)
                .with_space(PoweredRadiatorSpace::Spring, 0.58),
        ),
        (
            "grind-sympathique",
            PoweredRadiatorControls {
                drive: 0.82,
                headroom: 0.58,
                bloom: 0.68,
                sustain: 0.72,
                interlace: 0.68,
                curvature: 0.64,
                feedback: 0.15,
                ..base
            }
            .with_tone(PoweredRadiatorTone::Grind)
            .with_space(PoweredRadiatorSpace::Sympathique, 0.62),
        ),
        (
            "wall-shimmer-lite",
            PoweredRadiatorControls {
                drive: 1.50,
                headroom: 0.36,
                bloom: 1.00,
                sustain: 0.95,
                sag: 0.76,
                interlace: 0.90,
                chirality: -0.74,
                motion_hz: 0.88,
                curvature: 1.00,
                feedback: 0.26,
                room_decay: 0.70,
                ..base
            }
            .with_tone(PoweredRadiatorTone::Wall)
            .with_space(PoweredRadiatorSpace::Shimmer, 0.72),
        ),
    ];
    let mut processed = Vec::with_capacity(profiles.len());
    for (name, controls) in profiles {
        let mut segment = process(&source, source_receipt.sample_rate, tail_frames, controls)?;
        segment.receipt.name = name;
        processed.push(segment);
    }

    let gap = vec![0.0_f32; source_receipt.sample_rate as usize];
    let mut montage_left = Vec::new();
    let mut montage_right = Vec::new();
    montage_left.extend_from_slice(&dry_left);
    montage_right.extend_from_slice(&dry_right);
    montage_left.extend_from_slice(&gap);
    montage_right.extend_from_slice(&gap);
    for segment in &processed {
        montage_left.extend_from_slice(&segment.left);
        montage_right.extend_from_slice(&segment.right);
        montage_left.extend_from_slice(&gap);
        montage_right.extend_from_slice(&gap);
    }

    // This shared post-DSP gain exists only for comfortable listening. It is
    // never part of the input to any processor, and it cannot boost more than
    // 20 dB. All measurements in the receipt are taken before it.
    let peak_before_monitor_gain = stereo_peak(&montage_left, &montage_right);
    let monitor_gain = (0.95 / peak_before_monitor_gain.max(1.0e-9)).min(10.0);
    write_wav(
        &output,
        &montage_left,
        &montage_right,
        source_receipt.sample_rate,
        monitor_gain,
    )?;

    let stem = output
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("audition");
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    write_wav(
        &parent.join(format!("{stem}-dry.wav")),
        &dry_left,
        &dry_right,
        source_receipt.sample_rate,
        monitor_gain,
    )?;
    for segment in &processed {
        write_wav(
            &parent.join(format!("{stem}-{}.wav", segment.receipt.name)),
            &segment.left,
            &segment.right,
            source_receipt.sample_rate,
            monitor_gain,
        )?;
    }

    let dry_receipt = SegmentReceipt {
        name: "dry",
        controls: None,
        output_peak_before_monitor_gain: mono_peak(&source),
        output_rms_before_monitor_gain: mono_rms(&source),
        dsp: None,
    };
    let mut segment_receipts = Vec::with_capacity(processed.len() + 1);
    segment_receipts.push(dry_receipt);
    segment_receipts.extend(processed.into_iter().map(|segment| segment.receipt));
    let receipt = AuditionReceipt {
        schema: "mus.ariadne-powered-body-audition.v3",
        source: source_receipt,
        tail_frames,
        monitor_gain,
        output_peak: stereo_peak(&montage_left, &montage_right) * monitor_gain,
        segments: segment_receipts,
    };
    let receipt_path = output.with_extension("receipt.json");
    std::fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt)?)?;

    println!("wrote {}", output.display());
    println!("wrote {}", receipt_path.display());
    println!(
        "source: {} Hz, {} frame(s), {:.3} s, sha256 {}",
        receipt.source.sample_rate,
        receipt.source.frames,
        receipt.source.duration_seconds,
        receipt.source.sha256
    );
    println!(
        "shared monitor gain {monitor_gain:.6}, delivered peak {:.3}",
        receipt.output_peak
    );
    for segment in &receipt.segments {
        println!(
            "{}: peak {:.6}, rms {:.6}",
            segment.name,
            segment.output_peak_before_monitor_gain,
            segment.output_rms_before_monitor_gain
        );
    }
    Ok(())
}
