//! The audio streams: an output device running the rig in its callback and,
//! when asked for, an input device (the audio interface) whose guitar
//! replaces the simulated one. Input and output may be different devices; the
//! rig keeps a short ring between them and drops samples when it runs ahead.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use ovillo_dsp::rig::{Rig, RigEnds, INPUT_MAX_FILL, INPUT_TARGET_FILL};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub struct Audio {
    _stream: cpal::Stream,
    _input_stream: Option<cpal::Stream>,
    pub ends: RigEnds,
    pub sample_rate: u32,
    pub channels: u16,
    pub device: String,
    pub digest: String,
    pub stream_error: Arc<AtomicBool>,
    /// The last error CoreAudio reported (set off the audio callback).
    pub last_error: Arc<Mutex<Option<String>>>,
    pub input: Option<Input>,
}

/// The live input as the screen sees it.
pub struct Input {
    pub device: String,
    pub channels: u16,
    /// The channel being used (0-based); follows the loudest unless pinned.
    pub channel: Arc<AtomicUsize>,
    /// `usize::MAX` = follow the loudest channel.
    pub pinned: Arc<AtomicUsize>,
    /// Samples the input could not hand over because the ring was full.
    pub overflow: Arc<AtomicU64>,
}

#[derive(Debug, Default, Clone)]
pub struct Options {
    /// Substring of the output device name (default: the system output).
    pub output: Option<String>,
    /// Substring of the input device name; `None` = no live input.
    pub input: Option<String>,
    /// Start with the master stage muted: silent from the first sample.
    pub muted: bool,
    /// Seed for the simulated guitarist.
    pub seed: u32,
}

fn name_of(device: &cpal::Device) -> String {
    device.description().map(|d| d.name().to_string()).unwrap_or_else(|_| device.to_string())
}

fn find(devices: impl Iterator<Item = cpal::Device>, want: &str) -> Option<cpal::Device> {
    let want = want.to_lowercase();
    devices.into_iter().find(|d| name_of(d).to_lowercase().contains(&want))
}

/// Every device name, inputs and outputs, for error messages.
pub fn device_names() -> Vec<String> {
    let host = cpal::default_host();
    host.devices().map(|ds| ds.map(|d| name_of(&d)).collect()).unwrap_or_default()
}

pub fn start(opts: &Options) -> Result<Audio, String> {
    let host = cpal::default_host();
    let device = match &opts.output {
        Some(w) => find(host.output_devices().map_err(|e| e.to_string())?, w)
            .ok_or_else(|| format!("no output device matching \"{w}\" (have: {})", device_names().join(", ")))?,
        None => host.default_output_device().ok_or("no output device")?,
    };
    let name = name_of(&device);
    let supported = device.default_output_config().map_err(|e| format!("{name}: {e}"))?;
    if supported.sample_format() != cpal::SampleFormat::F32 {
        return Err(format!("{name}: output format {:?} (Ovillo plays f32)", supported.sample_format()));
    }
    let sample_rate = supported.sample_rate();
    let channels = supported.channels();
    let config = cpal::StreamConfig { channels, sample_rate, buffer_size: cpal::BufferSize::Default };
    let (mut rig, ends) = Rig::new(sample_rate as f64)?;
    rig.set_master_now(1.0, opts.muted);
    rig.seed(opts.seed);
    let digest = rig.plan_digest().to_string();
    let flag = Arc::new(AtomicBool::new(false));
    let last_error = Arc::new(Mutex::new(None));

    let err_sink = |label: &'static str| {
        let flag = flag.clone();
        let text = last_error.clone();
        move |e: cpal::Error| {
            // Runs on a CoreAudio thread, not the IO callback.
            eprintln!("ovillo: {label} stream error: {e}");
            if let Ok(mut t) = text.lock() {
                *t = Some(format!("{label}: {e}"));
            }
            flag.store(true, Ordering::Relaxed);
        }
    };

    // The live input, before the rig moves into the output callback.
    let mut input = None;
    let mut input_stream = None;
    if let Some(w) = &opts.input {
        let idev = find(host.input_devices().map_err(|e| e.to_string())?, w)
            .ok_or_else(|| format!("no input device matching \"{w}\" (have: {})", device_names().join(", ")))?;
        let iname = name_of(&idev);
        let iconf = idev
            .supported_input_configs()
            .map_err(|e| format!("{iname}: {e}"))?
            .find(|c| c.min_sample_rate() <= sample_rate && sample_rate <= c.max_sample_rate())
            .map(|c| c.with_sample_rate(sample_rate))
            .ok_or_else(|| format!("{iname}: cannot run at {sample_rate} Hz like the output ({name})"))?;
        let ich = iconf.channels();
        let (mut tx, rx) = rtrb::RingBuffer::<f32>::new(INPUT_MAX_FILL * 4);
        // Prime the ring so the first output callbacks have something to read.
        for _ in 0..INPUT_TARGET_FILL {
            let _ = tx.push(0.0);
        }
        rig.attach_input(rx);
        let channel = Arc::new(AtomicUsize::new(0));
        let pinned = Arc::new(AtomicUsize::new(usize::MAX));
        let overflow = Arc::new(AtomicU64::new(0));
        let follower = Follower::new(ich as usize, sample_rate as f64, channel.clone(), pinned.clone(), overflow.clone());
        let stream_config = cpal::StreamConfig { channels: ich, sample_rate, buffer_size: cpal::BufferSize::Default };
        let s = match iconf.sample_format() {
            cpal::SampleFormat::F32 => build_input::<f32>(&idev, stream_config, tx, follower, err_sink("input")),
            cpal::SampleFormat::I16 => build_input::<i16>(&idev, stream_config, tx, follower, err_sink("input")),
            cpal::SampleFormat::I32 => build_input::<i32>(&idev, stream_config, tx, follower, err_sink("input")),
            f => return Err(format!("{iname}: input format {f:?} not handled")),
        }
        .map_err(|e| format!("{iname}: {e}"))?;
        input = Some(Input { device: iname, channels: ich, channel, pinned, overflow });
        input_stream = Some(s);
    }

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| rig.process(data, channels as usize),
            err_sink("output"),
            None,
        )
        .map_err(|e| format!("{name}: {e}"))?;
    stream.play().map_err(|e| format!("{name}: {e}"))?;
    if let Some(s) = &input_stream {
        s.play().map_err(|e| format!("input: {e}"))?;
    }
    Ok(Audio {
        _stream: stream,
        _input_stream: input_stream,
        ends,
        sample_rate,
        channels,
        device: name,
        digest,
        stream_error: flag,
        last_error,
        input,
    })
}

/// Picks the input channel: the pinned one, or whichever is loudest (with
/// 6 dB of hysteresis, so an open mic input doesn't steal it from the guitar).
struct Follower {
    ms: Vec<f64>,
    coef: f64,
    channel: Arc<AtomicUsize>,
    pinned: Arc<AtomicUsize>,
    overflow: Arc<AtomicU64>,
}

impl Follower {
    fn new(channels: usize, sr: f64, channel: Arc<AtomicUsize>, pinned: Arc<AtomicUsize>, overflow: Arc<AtomicU64>) -> Self {
        Self { ms: vec![0.0; channels.max(1)], coef: 1.0 - (-1.0 / (0.3 * sr)).exp(), channel, pinned, overflow }
    }

    fn frame(&mut self, frame: &[f32], tx: &mut rtrb::Producer<f32>) {
        for (m, x) in self.ms.iter_mut().zip(frame) {
            *m += ((*x as f64) * (*x as f64) - *m) * self.coef;
        }
        let pinned = self.pinned.load(Ordering::Relaxed);
        let mut ch = self.channel.load(Ordering::Relaxed);
        if pinned < self.ms.len() {
            ch = pinned;
        } else {
            let (loud, m) = self.ms.iter().enumerate().fold((0, 0.0), |b, (i, m)| if *m > b.1 { (i, *m) } else { b });
            if loud != ch && m > 4.0 * self.ms.get(ch).copied().unwrap_or(0.0) && m > 1e-8 {
                ch = loud;
            }
        }
        self.channel.store(ch, Ordering::Relaxed);
        if tx.push(frame.get(ch).copied().unwrap_or(0.0)).is_err() {
            self.overflow.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn build_input<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    mut tx: rtrb::Producer<f32>,
    mut follower: Follower,
    err: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<cpal::Stream, cpal::Error>
where
    T: SizedSample + Send + 'static,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    let mut frame = vec![0.0f32; channels.max(1)];
    device.build_input_stream(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            for chunk in data.chunks(channels.max(1)) {
                for (f, x) in frame.iter_mut().zip(chunk) {
                    *f = <f32 as FromSample<T>>::from_sample_(*x);
                }
                follower.frame(&frame, &mut tx);
            }
        },
        err,
        None,
    )
}
