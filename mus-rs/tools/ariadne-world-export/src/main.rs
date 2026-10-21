use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use mus_ariadne::law::active::ActiveInclusionControls;
use mus_ariadne::law::transport::ResonantTransportControls;
use mus_ariadne::{
    BodyControls, BoundObservationProgram, SharedSpaceControls, StereoListener, StereoListenerSpec,
    World, WorldControls, WorldPlan,
};
use mus_ariadne_webgpu_bridge::{canonical_optical_observation_program, OpticalConstitutionPlan};
use mus_observation::{
    digest_json, encode_trace, ObservationFrame, ObservationTrace, ObservationValue, RunIdentity,
    ScalarType,
};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

const DEFAULT_TAIL_SECONDS: f64 = 4.0;
const TARGET_OPTICAL_RATE_HZ: u32 = 120;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunControls {
    material_organization: f64,
    source_coupling: f64,
    work_input: f64,
    active_coupling: f64,
    active_drive: f64,
    headroom: f64,
    sustain: f64,
    sag: f64,
    bloom: f64,
    feedback: f64,
    active_space: f64,
    interlace: f64,
    memory: f64,
    shared_space_coupling: f64,
    shared_space_decay: f64,
    shared_space_radiation: f64,
    body_radiation: f64,
    listener_scale: f64,
}

impl Default for RunControls {
    fn default() -> Self {
        Self {
            material_organization: 0.06,
            source_coupling: 1.0,
            work_input: 0.004,
            active_coupling: 0.46,
            active_drive: 0.42,
            headroom: 0.72,
            sustain: 0.24,
            sag: 0.35,
            bloom: 0.36,
            feedback: 0.04,
            active_space: 0.16,
            interlace: 0.42,
            memory: 0.28,
            shared_space_coupling: 0.16,
            shared_space_decay: 0.42,
            shared_space_radiation: 0.20,
            body_radiation: 0.008,
            listener_scale: 6.0,
        }
    }
}

impl RunControls {
    fn body(self, source: f64) -> BodyControls {
        BodyControls {
            source_wave: source * self.source_coupling,
            work_input: self.work_input,
            active_coupling: self.active_coupling,
            active: ActiveInclusionControls {
                drive: self.active_drive,
                headroom: self.headroom,
                sustain: self.sustain,
                sag: self.sag,
                bloom: self.bloom,
                feedback: self.feedback,
                space: self.active_space,
            },
            transport: ResonantTransportControls {
                interlace: self.interlace,
                memory: self.memory,
            },
            radiation_fraction: self.body_radiation,
        }
    }

    fn world(self) -> WorldControls {
        WorldControls {
            space: SharedSpaceControls {
                coupling: self.shared_space_coupling,
                decay: self.shared_space_decay,
                radiation: self.shared_space_radiation,
            },
            direct_coupling: Default::default(),
        }
    }
}

struct Config {
    input: PathBuf,
    output: PathBuf,
    tail_seconds: f64,
    controls: RunControls,
}

struct Source {
    sample_rate_hz: u32,
    frames: Vec<[f32; 2]>,
    channels: u16,
    sha256: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("ariadne-world-export: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = parse_args()?;
    fs::create_dir_all(&config.output).map_err(|error| error.to_string())?;
    let source = read_wav(&config.input)?;
    let sample_rate = source.sample_rate_hz;
    let tail_frames = (config.tail_seconds * sample_rate as f64).round() as usize;
    let horizon = source
        .frames
        .len()
        .checked_add(tail_frames)
        .ok_or_else(|| "render horizon overflows addressable memory".to_owned())?;
    let (observation_rate_hz, cadence_ticks) = exact_rate(sample_rate, TARGET_OPTICAL_RATE_HZ)?;

    let plan = WorldPlan::string_brass_v1(sample_rate as f64);
    let physical_world_digest = plan.digest().map_err(|error| error.to_string())?;
    let listener_spec = StereoListenerSpec::all_bodies(&plan, config.controls.listener_scale)
        .map_err(|error| error.to_string())?;
    let listener =
        StereoListener::bind(&listener_spec, &plan).map_err(|error| error.to_string())?;
    let observation_program = canonical_optical_observation_program(&plan, cadence_ticks)
        .map_err(|error| error.to_string())?;
    let observation_schema = observation_program
        .schema()
        .map_err(|error| error.to_string())?;
    let observation_program_digest = observation_program
        .digest()
        .map_err(|error| error.to_string())?;
    let observation_schema_digest = observation_schema
        .digest()
        .map_err(|error| error.to_string())?;
    let mut optical_plan = OpticalConstitutionPlan::canonical(&plan, &observation_program)
        .map_err(|error| error.to_string())?;
    optical_plan.surface.fixed_rate_hz = observation_rate_hz;
    optical_plan
        .validate(&plan, &observation_schema)
        .map_err(|error| error.to_string())?;
    let optical_constitution_digest = optical_plan.digest().map_err(|error| error.to_string())?;

    let source_adapter = json!({
        "schema": "ariadne-source-adapter-v1",
        "kind": "wav-mid-boundary",
        "leftWeight": 0.5,
        "rightWeight": 0.5,
        "sourceCoupling": config.controls.source_coupling,
    });
    let intervention = json!({
        "schema": "ariadne-wav-intervention-v1",
        "source": {
            "sha256": source.sha256.clone(),
            "channels": source.channels,
            "adapter": source_adapter.clone(),
        },
        "controls": config.controls,
    });
    let intervention_digest = digest_json(&intervention).map_err(|error| error.to_string())?;
    let source_frames = u64::try_from(source.frames.len())
        .map_err(|_| "source frame count exceeds u64".to_owned())?;
    let tail_frames_u64 =
        u64::try_from(tail_frames).map_err(|_| "tail frame count exceeds u64".to_owned())?;
    let run_identity = RunIdentity::new(
        physical_world_digest.clone(),
        listener.listener_digest().to_owned(),
        intervention_digest.clone(),
        observation_program_digest.clone(),
        observation_schema_digest.clone(),
        optical_constitution_digest.clone(),
        sample_rate,
        observation_rate_hz,
        source_frames,
        tail_frames_u64,
    )
    .map_err(|error| error.to_string())?;
    let run_digest = run_identity.digest().map_err(|error| error.to_string())?;

    let mut world = World::from_plan(plan.clone()).map_err(|error| error.to_string())?;
    world
        .set_body_configuration_index(0, config.controls.material_organization)
        .map_err(|error| error.to_string())?;
    let mut observer = BoundObservationProgram::compile(&observation_program, &plan)
        .map_err(|error| error.to_string())?;
    let mut observation_values = zero_values(&observation_schema);
    observer
        .snapshot_initial_into(&world, &mut observation_values)
        .map_err(|error| error.to_string())?;
    let mut observation_frames = vec![ObservationFrame::new(0, observation_values.clone())];

    let mut audio = Vec::with_capacity(horizon);
    let world_controls = config.controls.world();
    let mut body_controls = vec![BodyControls::default(); world.body_count()];
    for index in 0..horizon {
        let input = source.frames.get(index).copied().unwrap_or([0.0, 0.0]);
        let source_coordinate = 0.5 * (input[0] as f64 + input[1] as f64);
        body_controls[0] = config.controls.body(source_coordinate);
        let frame = world
            .process_sample(&body_controls, world_controls)
            .map_err(|error| error.to_string())?;
        let stereo = listener
            .observe(&world, &frame)
            .map_err(|error| error.to_string())?;
        audio.push([stereo.left as f32, stereo.right as f32]);
        let tick = (index + 1) as u64;
        if observer
            .observe_sample_into(tick, &world, &frame, &mut observation_values)
            .map_err(|error| error.to_string())?
        {
            observation_frames.push(ObservationFrame::new(tick, observation_values.clone()));
        }
    }
    if observation_frames
        .last()
        .is_none_or(|frame| frame.tick != horizon as u64)
    {
        observation_frames.push(
            observer
                .flush_into(horizon as u64, &mut observation_values)
                .map_err(|error| error.to_string())?,
        );
    }

    let trace = ObservationTrace::new(
        observation_schema.clone(),
        run_digest.clone(),
        run_identity.horizon_ticks,
        observation_frames,
    )
    .map_err(|error| error.to_string())?;

    let audio_path = config.output.join("audio.wav");
    let trace_path = config.output.join("observation.aotr");
    let plan_path = config.output.join("world.plan.json");
    let listener_path = config.output.join("listener.json");
    let program_path = config.output.join("observation.program.json");
    let schema_path = config.output.join("observation.schema.json");
    let optical_path = config.output.join("optical.plan.json");
    let intervention_path = config.output.join("intervention.json");
    let identity_path = config.output.join("run.identity.json");
    write_wav(&audio_path, sample_rate, &audio)?;
    fs::write(
        &trace_path,
        encode_trace(&trace).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    write_json(&plan_path, &plan)?;
    write_json(&listener_path, &listener_spec)?;
    write_json(&program_path, &observation_program)?;
    write_json(&schema_path, &observation_schema)?;
    write_json(&optical_path, &optical_plan)?;
    write_json(&intervention_path, &intervention)?;
    write_json(&identity_path, &run_identity)?;

    let receipt = world.receipt();
    let manifest = json!({
        "schema": "ariadne-world-audiovisual-bundle-v1",
        "claim": "one physical World; sound and WebGPU data are non-loading observation programs",
        "runDigest": run_digest,
        "physicalWorldDigest": physical_world_digest,
        "listenerDigest": listener.listener_digest(),
        "interventionDigest": intervention_digest,
        "observationProgramDigest": observation_program_digest,
        "observationSchemaDigest": observation_schema_digest,
        "opticalConstitutionDigest": optical_constitution_digest,
        "sampleRateHz": sample_rate,
        "observationRateHz": observation_rate_hz,
        "sourceFrames": run_identity.source_frames,
        "tailFrames": run_identity.tail_frames,
        "horizonTicks": run_identity.horizon_ticks,
        "source": {
            "path": config.input.display().to_string(),
            "sha256": source.sha256,
            "channels": source.channels,
            "adapter": source_adapter,
        },
        "controls": config.controls,
        "files": {
            "audio": "audio.wav",
            "trace": "observation.aotr",
            "worldPlan": "world.plan.json",
            "listener": "listener.json",
            "observationProgram": "observation.program.json",
            "observationSchema": "observation.schema.json",
            "opticalPlan": "optical.plan.json",
            "intervention": "intervention.json",
            "runIdentity": "run.identity.json",
        },
        "digests": {
            "audio": file_digest(&audio_path)?,
            "trace": file_digest(&trace_path)?,
            "worldPlan": file_digest(&plan_path)?,
            "listener": file_digest(&listener_path)?,
            "observationProgram": file_digest(&program_path)?,
            "observationSchema": file_digest(&schema_path)?,
            "opticalPlan": file_digest(&optical_path)?,
            "intervention": file_digest(&intervention_path)?,
            "runIdentity": file_digest(&identity_path)?,
        },
        "finalReceipt": {
            "sourceWork": receipt.source_work,
            "controlWork": receipt.control_work,
            "internalLoss": receipt.internal_loss,
            "radiation": receipt.radiation,
            "initialEnergy": receipt.initial_energy,
            "currentEnergy": receipt.current_energy,
            "globalClosureResidual": receipt.global_closure_residual,
            "maximumLocalResidual": receipt.maximum_local_residual,
        },
    });
    write_json(&config.output.join("manifest.json"), &manifest)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&manifest).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn parse_args() -> Result<Config, String> {
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
        .unwrap_or(DEFAULT_TAIL_SECONDS);
    let material_organization = args
        .next()
        .map(|value| {
            value
                .parse::<f64>()
                .map_err(|_| format!("invalid organization {value:?}"))
        })
        .transpose()?
        .unwrap_or(RunControls::default().material_organization);
    if args.next().is_some()
        || !tail_seconds.is_finite()
        || !(0.0..=60.0).contains(&tail_seconds)
        || !material_organization.is_finite()
        || !(0.0..=1.0).contains(&material_organization)
    {
        return Err(usage());
    }
    Ok(Config {
        input,
        output,
        tail_seconds,
        controls: RunControls {
            material_organization,
            ..RunControls::default()
        },
    })
}

fn usage() -> String {
    "usage: ariadne-world-export INPUT.wav OUTPUT_DIR [TAIL_SECONDS] [ORGANIZATION]".to_owned()
}

fn exact_rate(sample_rate_hz: u32, target_hz: u32) -> Result<(u32, u64), String> {
    let rate = (1..=target_hz)
        .rev()
        .find(|candidate| sample_rate_hz % candidate == 0)
        .ok_or_else(|| "material clock has no positive observation-rate divisor".to_owned())?;
    Ok((rate, u64::from(sample_rate_hz / rate)))
}

fn zero_values(schema: &mus_observation::ObservationSchema) -> Vec<ObservationValue> {
    schema
        .channels
        .iter()
        .map(|channel| match channel.scalar_type {
            ScalarType::F32 => ObservationValue::F32(0.0),
            ScalarType::F64 => ObservationValue::F64(0.0),
            ScalarType::U64 => ObservationValue::U64(0),
        })
        .collect()
}

fn read_wav(path: &Path) -> Result<Source, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let sha256 = format!("sha256:{:x}", Sha256::digest(&bytes));
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
    Ok(Source {
        sample_rate_hz: spec.sample_rate,
        frames,
        channels: spec.channels,
        sha256,
    })
}

fn write_wav(path: &Path, sample_rate_hz: u32, frames: &[[f32; 2]]) -> Result<(), String> {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 2,
            sample_rate: sample_rate_hz,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )
    .map_err(|error| error.to_string())?;
    for frame in frames {
        writer
            .write_sample(frame[0])
            .map_err(|error| error.to_string())?;
        writer
            .write_sample(frame[1])
            .map_err(|error| error.to_string())?;
    }
    writer.finalize().map_err(|error| error.to_string())
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn file_digest(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}
