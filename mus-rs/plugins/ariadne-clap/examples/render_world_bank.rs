//! Render several independent canonical Ariadne Worlds as parallel studio
//! returns over a recognizable stereo bed.
//!
//! This is deliberately not a multi-body physical World. Every stem owns a
//! separate World, space, observer, and energy ledger. The final summation,
//! gain staging, and panning are ordinary nonphysical studio operations and
//! are identified as such in the receipt.
//!
//! Usage:
//! `cargo run -p ariadne-clap --release --example render_world_bank -- MANIFEST.json OUTPUT.wav RECEIPT.json`

use std::env;
use std::f64::consts::FRAC_PI_4;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use ariadne_clap::{AriadneControls, AriadneEngine, AriadneEngineReceipt, GuitarCharacter};
use mus_ariadne::WorldSpec;
use serde::Deserialize;
use serde_json::{json, Value};

const RECEIPT_SCHEMA: &str = "mus.ariadne-independent-world-bank-receipt.v1";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    schema: String,
    bed: PathBuf,
    #[serde(default)]
    bed_gain_db: f64,
    #[serde(default = "default_tail_seconds")]
    tail_seconds: f64,
    #[serde(default = "default_target_peak_dbfs")]
    target_peak_dbfs: f64,
    returns: Vec<ReturnSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReturnSpec {
    id: String,
    input: PathBuf,
    character: Character,
    #[serde(default = "unity")]
    frequency_scale: f64,
    return_gain_db: f64,
    #[serde(default)]
    pan: f64,
    #[serde(default)]
    controls: ControlOverrides,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Character {
    CostaClara,
    Edge,
    Grind,
    Wall,
}

impl From<Character> for GuitarCharacter {
    fn from(value: Character) -> Self {
        match value {
            Character::CostaClara => Self::CostaClara,
            Character::Edge => Self::Edge,
            Character::Grind => Self::Grind,
            Character::Wall => Self::Wall,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ControlOverrides {
    drive: Option<f64>,
    headroom: Option<f64>,
    bias: Option<f64>,
    tone: Option<f64>,
    bloom: Option<f64>,
    sustain: Option<f64>,
    sag: Option<f64>,
    interlace: Option<f64>,
    chirality: Option<f64>,
    memory: Option<f64>,
    curvature: Option<f64>,
    feedback: Option<f64>,
    space: Option<f64>,
    room_decay: Option<f64>,
    output_gain_db: Option<f64>,
}

struct RenderedReturn {
    spec: ReturnSpec,
    world_spec_digest: String,
    frames: Vec<[f32; 2]>,
    receipt: AriadneEngineReceipt,
    input_frames: usize,
    source_peak: f64,
    source_rms: f64,
    wet_peak: f64,
    wet_rms: f64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("ariadne-world-bank-render: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let manifest_path = PathBuf::from(args.next().ok_or_else(usage)?);
    let output_path = PathBuf::from(args.next().ok_or_else(usage)?);
    let receipt_path = PathBuf::from(args.next().ok_or_else(usage)?);
    if args.next().is_some() {
        return Err(usage());
    }

    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
    let mut manifest: Manifest = serde_json::from_str(&manifest_text)
        .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
    validate_manifest(&manifest)?;
    manifest.bed = resolve(manifest_dir, &manifest.bed);
    for return_spec in &mut manifest.returns {
        return_spec.input = resolve(manifest_dir, &return_spec.input);
    }

    let (bed, sample_rate) = read_wav(&manifest.bed)?;
    let tail_frames = (manifest.tail_seconds * sample_rate as f64).round() as usize;
    let output_frames = bed.len() + tail_frames;
    let bed_gain = db_amplitude(manifest.bed_gain_db);

    let handles = manifest
        .returns
        .into_iter()
        .map(|return_spec| {
            thread::spawn(move || render_return(return_spec, sample_rate, output_frames))
        })
        .collect::<Vec<_>>();
    let mut rendered = Vec::with_capacity(handles.len());
    for handle in handles {
        rendered.push(
            handle
                .join()
                .map_err(|_| "an Ariadne World render thread panicked".to_owned())??,
        );
    }

    let mut mixed = vec![[0.0_f64; 2]; output_frames];
    for (index, frame) in bed.iter().enumerate() {
        mixed[index][0] = frame[0] as f64 * bed_gain;
        mixed[index][1] = frame[1] as f64 * bed_gain;
    }
    for rendered_return in &rendered {
        let return_gain = db_amplitude(rendered_return.spec.return_gain_db);
        let pan = rendered_return.spec.pan.clamp(-1.0, 1.0);
        // A centered return leaves the World's stereo observation unchanged;
        // a hard pan suppresses one side without manufacturing extra energy.
        let left_pan = ((pan + 1.0) * FRAC_PI_4).cos() * 2.0_f64.sqrt();
        let right_pan = ((pan + 1.0) * FRAC_PI_4).sin() * 2.0_f64.sqrt();
        for (mix, frame) in mixed.iter_mut().zip(&rendered_return.frames) {
            mix[0] += frame[0] as f64 * return_gain * left_pan;
            mix[1] += frame[1] as f64 * return_gain * right_pan;
        }
    }

    let (pre_master_peak, pre_master_rms) = level_stats_f64(&mixed);
    let target_peak = db_amplitude(manifest.target_peak_dbfs);
    let master_gain = if pre_master_peak > target_peak && pre_master_peak > 0.0 {
        target_peak / pre_master_peak
    } else {
        1.0
    };
    let output = mixed
        .iter()
        .map(|frame| {
            [
                (frame[0] * master_gain) as f32,
                (frame[1] * master_gain) as f32,
            ]
        })
        .collect::<Vec<_>>();
    let (output_peak, output_rms) = level_stats(&output);
    write_wav(&output_path, sample_rate, &output)?;

    let return_receipts = rendered.iter().map(return_receipt).collect::<Vec<_>>();
    let receipt = json!({
        "schema": RECEIPT_SCHEMA,
        "architecture": {
            "kind": "independent-worlds-plus-nonphysical-studio-mixer",
            "physicalClaim": "Each return is one canonical World with its own space, observer, and ledger.",
            "nonClaim": "The Worlds do not share bodies, space, coupling, work, loss, radiation, transport, or a global closure ledger.",
            "mixer": "Ordinary gain, stereo balance, summation, and one static peak-preserving master gain."
        },
        "manifest": manifest_path.display().to_string(),
        "bed": manifest.bed.display().to_string(),
        "output": output_path.display().to_string(),
        "sampleRate": sample_rate,
        "bedFrames": bed.len(),
        "tailFrames": tail_frames,
        "bedGainDb": manifest.bed_gain_db,
        "targetPeakDbfs": manifest.target_peak_dbfs,
        "preMasterPeak": pre_master_peak,
        "preMasterPeakDbfs": amplitude_db(pre_master_peak),
        "preMasterRms": pre_master_rms,
        "preMasterRmsDbfs": amplitude_db(pre_master_rms),
        "masterGain": master_gain,
        "masterGainDb": amplitude_db(master_gain),
        "outputPeak": output_peak,
        "outputPeakDbfs": amplitude_db(output_peak),
        "outputRms": output_rms,
        "outputRmsDbfs": amplitude_db(output_rms),
        "returns": return_receipts,
    });
    let receipt_text = serde_json::to_string_pretty(&receipt).map_err(|error| error.to_string())?;
    fs::write(&receipt_path, format!("{receipt_text}\n"))
        .map_err(|error| format!("{}: {error}", receipt_path.display()))?;
    println!("{receipt_text}");
    Ok(())
}

fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    if manifest.schema != "mus.ariadne-independent-world-bank.v1" {
        return Err(format!("unsupported manifest schema {:?}", manifest.schema));
    }
    if manifest.returns.is_empty() {
        return Err("manifest must declare at least one World return".to_owned());
    }
    if !manifest.tail_seconds.is_finite() || !(0.0..=20.0).contains(&manifest.tail_seconds) {
        return Err("tailSeconds must be finite and in [0, 20]".to_owned());
    }
    if !manifest.target_peak_dbfs.is_finite()
        || !(-24.0..=-0.1).contains(&manifest.target_peak_dbfs)
    {
        return Err("targetPeakDbfs must be finite and in [-24, -0.1]".to_owned());
    }
    for return_spec in &manifest.returns {
        if return_spec.id.trim().is_empty()
            || !return_spec.frequency_scale.is_finite()
            || !(0.125..=4.0).contains(&return_spec.frequency_scale)
            || !return_spec.return_gain_db.is_finite()
            || !(-80.0..=12.0).contains(&return_spec.return_gain_db)
            || !return_spec.pan.is_finite()
            || !(-1.0..=1.0).contains(&return_spec.pan)
        {
            return Err(format!(
                "invalid World return specification for {:?}",
                return_spec.id
            ));
        }
    }
    Ok(())
}

fn render_return(
    return_spec: ReturnSpec,
    expected_sample_rate: u32,
    output_frames: usize,
) -> Result<RenderedReturn, String> {
    let (source, sample_rate) = read_wav(&return_spec.input)?;
    if sample_rate != expected_sample_rate {
        return Err(format!(
            "{} has sample rate {sample_rate}, expected {expected_sample_rate}",
            return_spec.input.display()
        ));
    }
    if source.len() > output_frames {
        return Err(format!(
            "{} is longer than bed plus declared tail",
            return_spec.input.display()
        ));
    }
    let (source_peak, source_rms) = level_stats(&source);
    let mut world_spec = WorldSpec::string_brass_v1();
    world_spec.internal_sample_rate_hz = sample_rate as f64;
    scale_world_frequencies(&mut world_spec, return_spec.frequency_scale);
    world_spec.anatomy_name = format!(
        "shared-wave-material-v1-independent-return-{}",
        return_spec.id
    );
    let world_spec_digest = world_spec.digest().map_err(|error| error.to_string())?;
    let mut engine = AriadneEngine::with_spec(world_spec)?;
    let controls = return_spec.controls.apply(return_spec.character.into());
    let mut frames = Vec::with_capacity(output_frames);
    for index in 0..output_frames {
        let frame = source.get(index).copied().unwrap_or([0.0, 0.0]);
        let world_frame = engine.process_sample(frame[0], frame[1], controls);
        frames.push([world_frame.left, world_frame.right]);
    }
    let receipt = engine.receipt();
    let (wet_peak, wet_rms) = level_stats(&frames);
    Ok(RenderedReturn {
        spec: return_spec,
        world_spec_digest,
        frames,
        receipt,
        input_frames: source.len(),
        source_peak,
        source_rms,
        wet_peak,
        wet_rms,
    })
}

impl ControlOverrides {
    fn apply(&self, character: GuitarCharacter) -> AriadneControls {
        let defaults = AriadneControls::default();
        AriadneControls {
            character,
            drive: self.drive.unwrap_or(0.20),
            headroom: self.headroom.unwrap_or(0.90),
            bias: self.bias.unwrap_or(defaults.bias),
            tone: self.tone.unwrap_or(0.42),
            bloom: self.bloom.unwrap_or(0.12),
            sustain: self.sustain.unwrap_or(0.10),
            sag: self.sag.unwrap_or(0.18),
            interlace: self.interlace.unwrap_or(0.18),
            chirality: self.chirality.unwrap_or(defaults.chirality),
            memory: self.memory.unwrap_or(0.10),
            curvature: self.curvature.unwrap_or(defaults.curvature),
            feedback: self.feedback.unwrap_or(0.01),
            space: self.space.unwrap_or(0.08),
            room_decay: self.room_decay.unwrap_or(0.28),
            mix: 1.0,
            output_gain: db_amplitude(self.output_gain_db.unwrap_or(-12.0)),
        }
    }
}

fn scale_world_frequencies(spec: &mut WorldSpec, scale: f64) {
    for frequency in &mut spec.configuration_path.left.metric.frequency_hz {
        *frequency *= scale;
    }
    for frequency in &mut spec.configuration_path.right.metric.frequency_hz {
        *frequency *= scale;
    }
}

fn return_receipt(rendered: &RenderedReturn) -> Value {
    let receipt = &rendered.receipt;
    json!({
        "id": rendered.spec.id,
        "input": rendered.spec.input.display().to_string(),
        "character": format!("{:?}", GuitarCharacter::from(rendered.spec.character)),
        "frequencyScale": rendered.spec.frequency_scale,
        "returnGainDb": rendered.spec.return_gain_db,
        "pan": rendered.spec.pan,
        "inputFrames": rendered.input_frames,
        "sourcePeak": rendered.source_peak,
        "sourcePeakDbfs": amplitude_db(rendered.source_peak),
        "sourceRms": rendered.source_rms,
        "sourceRmsDbfs": amplitude_db(rendered.source_rms),
        "wetPeak": rendered.wet_peak,
        "wetPeakDbfs": amplitude_db(rendered.wet_peak),
        "wetRms": rendered.wet_rms,
        "wetRmsDbfs": amplitude_db(rendered.wet_rms),
        "worldSpecDigest": rendered.world_spec_digest,
        "worldDigest": receipt.world.world_digest,
        "worldFrames": receipt.world.sample_frames,
        "ledgerFrames": receipt.world.ledger_frames,
        "configurationAmount": receipt.world.configuration_amount,
        "globalClosureResidual": receipt.world.global_closure_residual,
        "cumulativeClosureResidual": receipt.world.cumulative_closure_residual,
        "maximumFrameResidual": receipt.world.maximum_frame_residual,
        "maximumLocalResidual": receipt.world.maximum_local_residual,
        "nonFiniteInputFrames": receipt.non_finite_input_frames,
        "worldErrors": receipt.world_errors,
        "safetyLimitedFrames": receipt.safety_limited_frames,
        "peakWetBeforeOutputGain": receipt.peak_wet,
        "peakAfterWorldOutputSafety": receipt.peak_output,
    })
}

fn resolve(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    }
}

fn read_wav(path: &Path) -> Result<(Vec<[f32; 2]>, u32), String> {
    let mut reader =
        hound::WavReader::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let spec = reader.spec();
    let channels = spec.channels as usize;
    if channels == 0 {
        return Err(format!("{} declares zero channels", path.display()));
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
        _ => {
            return Err(format!(
                "{} uses unsupported WAV encoding {:?} {}-bit",
                path.display(),
                spec.sample_format,
                spec.bits_per_sample
            ))
        }
    };
    if samples.len() % channels != 0 {
        return Err(format!("{} has a partial WAV frame", path.display()));
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
    finish_level_stats(peak, sum_squares, count)
}

fn level_stats_f64(frames: &[[f64; 2]]) -> (f64, f64) {
    let mut peak = 0.0_f64;
    let mut sum_squares = 0.0_f64;
    let mut count = 0_u64;
    for sample in frames.iter().flatten() {
        peak = peak.max(sample.abs());
        sum_squares += sample * sample;
        count += 1;
    }
    finish_level_stats(peak, sum_squares, count)
}

fn finish_level_stats(peak: f64, sum_squares: f64, count: u64) -> (f64, f64) {
    let rms = if count == 0 {
        0.0
    } else {
        (sum_squares / count as f64).sqrt()
    };
    (peak, rms)
}

fn db_amplitude(db: f64) -> f64 {
    10.0_f64.powf(db / 20.0)
}

fn amplitude_db(value: f64) -> f64 {
    if value > 0.0 {
        20.0 * value.log10()
    } else {
        f64::NEG_INFINITY
    }
}

const fn default_tail_seconds() -> f64 {
    2.0
}

const fn default_target_peak_dbfs() -> f64 {
    -2.0
}

const fn unity() -> f64 {
    1.0
}

fn usage() -> String {
    "usage: render_world_bank MANIFEST.json OUTPUT.wav RECEIPT.json".to_owned()
}
