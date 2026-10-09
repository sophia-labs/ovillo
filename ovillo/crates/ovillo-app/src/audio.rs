//! The output stream: the default CoreAudio output device, running the rig in
//! its callback. (When the interface arrives, the guitar comes in on the
//! same device and the simulated guitar becomes one source among others.)

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ovillo_dsp::rig::{Rig, RigEnds};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct Audio {
    _stream: cpal::Stream,
    pub ends: RigEnds,
    pub sample_rate: u32,
    pub channels: u16,
    pub device: String,
    pub digest: String,
    pub stream_error: Arc<AtomicBool>,
    /// The last error CoreAudio reported (set off the audio callback).
    pub last_error: Arc<Mutex<Option<String>>>,
}

/// Open the default output and start the rig. `muted` starts the master
/// stage at silence from the first sample; `seed` seeds the guitarist.
pub fn start(muted: bool, seed: u32) -> Result<Audio, String> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("no output device")?;
    let name = device.description().map(|d| d.name().to_string()).unwrap_or_else(|_| device.to_string());
    let supported = device.default_output_config().map_err(|e| format!("{name}: {e}"))?;
    if supported.sample_format() != cpal::SampleFormat::F32 {
        return Err(format!("{name}: output format {:?} (Ovillo plays f32)", supported.sample_format()));
    }
    let sample_rate = supported.sample_rate();
    let channels = supported.channels();
    let config = cpal::StreamConfig { channels, sample_rate, buffer_size: cpal::BufferSize::Default };
    let (mut rig, ends) = Rig::new(sample_rate as f64)?;
    rig.set_master_now(1.0, muted);
    rig.seed(seed);
    let digest = rig.plan_digest().to_string();
    let flag = Arc::new(AtomicBool::new(false));
    let err_flag = flag.clone();
    let last_error = Arc::new(Mutex::new(None));
    let err_text = last_error.clone();
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| rig.process(data, channels as usize),
            move |e: cpal::Error| {
                // Runs on a CoreAudio thread, not the IO callback.
                eprintln!("ovillo: audio stream error: {e}");
                if let Ok(mut t) = err_text.lock() {
                    *t = Some(e.to_string());
                }
                err_flag.store(true, Ordering::Relaxed);
            },
            None,
        )
        .map_err(|e| format!("{name}: {e}"))?;
    stream.play().map_err(|e| format!("{name}: {e}"))?;
    Ok(Audio { _stream: stream, ends, sample_rate, channels, device: name, digest, stream_error: flag, last_error })
}
