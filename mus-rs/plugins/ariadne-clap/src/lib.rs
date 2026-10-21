//! Ariadne World — VST3/CLAP boundary around the sole canonical runtime.
//!
//! The wrapper owns host ABI, smoothing, dry/wet presentation, and safety. All
//! persistent physical state executes in `mus_ariadne::World`; stereo sound is
//! produced by a separate non-loading `StereoListener`.

use std::sync::Arc;

use mus_ariadne::law::active::ActiveInclusionControls;
use mus_ariadne::law::transport::ResonantTransportControls;
use mus_ariadne::{
    BodyControls, SharedSpaceControls, StereoListener, StereoListenerSpec, World, WorldControls,
    WorldPlan, WorldReceipt,
};
use nice_plug::prelude::*;

const PARAMETER_SMOOTHING_MS: f32 = 20.0;
const SLOW_PARAMETER_SMOOTHING_MS: f32 = 60.0;
const DEFAULT_SAMPLE_RATE_HZ: f64 = 48_000.0;
const TAIL_SECONDS: f64 = 20.0;
const OUTPUT_SAFETY_LIMIT: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AriadneControls {
    pub material_organization: f64,
    pub source_coupling: f64,
    pub work_input: f64,
    pub active_coupling: f64,
    pub active_drive: f64,
    pub headroom: f64,
    pub sustain: f64,
    pub sag: f64,
    pub bloom: f64,
    pub feedback: f64,
    pub active_space: f64,
    pub interlace: f64,
    pub memory: f64,
    pub space_coupling: f64,
    pub space_decay: f64,
    pub space_radiation: f64,
    pub body_radiation: f64,
    pub listener_scale: f64,
    pub mix: f64,
    pub output_gain: f64,
}

impl Default for AriadneControls {
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
            space_coupling: 0.16,
            space_decay: 0.42,
            space_radiation: 0.20,
            body_radiation: 0.008,
            listener_scale: 6.0,
            mix: 1.0,
            output_gain: 1.0,
        }
    }
}

impl AriadneControls {
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            material_organization: finite(self.material_organization).clamp(0.0, 1.0),
            source_coupling: finite(self.source_coupling).clamp(0.0, 4.0),
            work_input: finite(self.work_input).clamp(0.0, 1.0),
            active_coupling: finite(self.active_coupling).clamp(0.0, 1.0),
            active_drive: finite(self.active_drive).clamp(0.0, 1.5),
            headroom: finite(self.headroom).clamp(0.02, 1.0),
            sustain: finite(self.sustain).clamp(0.0, 1.0),
            sag: finite(self.sag).clamp(0.0, 1.0),
            bloom: finite(self.bloom).clamp(0.0, 1.0),
            feedback: finite(self.feedback).clamp(0.0, 1.0),
            active_space: finite(self.active_space).clamp(0.0, 1.0),
            interlace: finite(self.interlace).clamp(0.0, 1.0),
            memory: finite(self.memory).clamp(0.0, 1.0),
            space_coupling: finite(self.space_coupling).clamp(0.0, 1.0),
            space_decay: finite(self.space_decay).clamp(0.0, 1.0),
            space_radiation: finite(self.space_radiation).clamp(0.0, 1.0),
            body_radiation: finite(self.body_radiation).clamp(0.0, 1.0),
            listener_scale: finite(self.listener_scale).clamp(0.0, 64.0),
            mix: finite(self.mix).clamp(0.0, 1.0),
            output_gain: finite(self.output_gain).clamp(0.0, 8.0),
        }
    }
}

#[derive(Params)]
pub struct AriadneParams {
    #[id = "organization"]
    pub organization: FloatParam,
    #[id = "source_coupling"]
    pub source_coupling: FloatParam,
    #[id = "work_input"]
    pub work_input: FloatParam,
    #[id = "active_coupling"]
    pub active_coupling: FloatParam,
    #[id = "active_drive"]
    pub active_drive: FloatParam,
    #[id = "headroom"]
    pub headroom: FloatParam,
    #[id = "sustain"]
    pub sustain: FloatParam,
    #[id = "sag"]
    pub sag: FloatParam,
    #[id = "bloom"]
    pub bloom: FloatParam,
    #[id = "feedback"]
    pub feedback: FloatParam,
    #[id = "active_space"]
    pub active_space: FloatParam,
    #[id = "interlace"]
    pub interlace: FloatParam,
    #[id = "memory"]
    pub memory: FloatParam,
    #[id = "space_coupling"]
    pub space_coupling: FloatParam,
    #[id = "space_decay"]
    pub space_decay: FloatParam,
    #[id = "space_radiation"]
    pub space_radiation: FloatParam,
    #[id = "body_radiation"]
    pub body_radiation: FloatParam,
    #[id = "listener_scale"]
    pub listener_scale: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
    #[id = "output"]
    pub output: FloatParam,
}

impl Default for AriadneParams {
    fn default() -> Self {
        let defaults = AriadneControls::default();
        let linear = |name: &'static str, default, min, max| {
            FloatParam::new(name, default, FloatRange::Linear { min, max })
                .with_smoother(SmoothingStyle::Linear(PARAMETER_SMOOTHING_MS))
                .with_value_to_string(formatters::v2s_f32_rounded(3))
        };
        let slow = |name: &'static str, default, min, max| {
            FloatParam::new(name, default, FloatRange::Linear { min, max })
                .with_smoother(SmoothingStyle::Linear(SLOW_PARAMETER_SMOOTHING_MS))
                .with_value_to_string(formatters::v2s_f32_rounded(3))
        };
        Self {
            organization: slow(
                "Material Organization",
                defaults.material_organization as f32,
                0.0,
                1.0,
            ),
            source_coupling: linear("Source Coupling", defaults.source_coupling as f32, 0.0, 4.0),
            work_input: linear("Work Input", defaults.work_input as f32, 0.0, 1.0),
            active_coupling: linear("Active Material", defaults.active_coupling as f32, 0.0, 1.0),
            active_drive: linear("Active Drive", defaults.active_drive as f32, 0.0, 1.5),
            headroom: linear("Headroom", defaults.headroom as f32, 0.02, 1.0),
            sustain: linear("Sustain", defaults.sustain as f32, 0.0, 1.0),
            sag: linear("Hunger", defaults.sag as f32, 0.0, 1.0),
            bloom: linear("Metabolism", defaults.bloom as f32, 0.0, 1.0),
            feedback: slow("Reciprocity", defaults.feedback as f32, 0.0, 1.0),
            active_space: slow("Active Space", defaults.active_space as f32, 0.0, 1.0),
            interlace: linear("Interlace", defaults.interlace as f32, 0.0, 1.0),
            memory: slow("Path Memory", defaults.memory as f32, 0.0, 1.0),
            space_coupling: slow("Shared Space", defaults.space_coupling as f32, 0.0, 1.0),
            space_decay: slow("Space Persistence", defaults.space_decay as f32, 0.0, 1.0),
            space_radiation: slow("Space Radiation", defaults.space_radiation as f32, 0.0, 1.0),
            body_radiation: linear("Body Radiation", defaults.body_radiation as f32, 0.0, 1.0),
            listener_scale: linear("Listener Scale", defaults.listener_scale as f32, 0.0, 64.0),
            mix: FloatParam::new(
                "Mix",
                defaults.mix as f32,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(PARAMETER_SMOOTHING_MS))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            output: FloatParam::new(
                "Output",
                defaults.output_gain as f32,
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(18.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 18.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(PARAMETER_SMOOTHING_MS))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

impl AriadneParams {
    #[inline]
    fn next_controls(&self) -> AriadneControls {
        AriadneControls {
            material_organization: self.organization.smoothed.next() as f64,
            source_coupling: self.source_coupling.smoothed.next() as f64,
            work_input: self.work_input.smoothed.next() as f64,
            active_coupling: self.active_coupling.smoothed.next() as f64,
            active_drive: self.active_drive.smoothed.next() as f64,
            headroom: self.headroom.smoothed.next() as f64,
            sustain: self.sustain.smoothed.next() as f64,
            sag: self.sag.smoothed.next() as f64,
            bloom: self.bloom.smoothed.next() as f64,
            feedback: self.feedback.smoothed.next() as f64,
            active_space: self.active_space.smoothed.next() as f64,
            interlace: self.interlace.smoothed.next() as f64,
            memory: self.memory.smoothed.next() as f64,
            space_coupling: self.space_coupling.smoothed.next() as f64,
            space_decay: self.space_decay.smoothed.next() as f64,
            space_radiation: self.space_radiation.smoothed.next() as f64,
            body_radiation: self.body_radiation.smoothed.next() as f64,
            listener_scale: self.listener_scale.smoothed.next() as f64,
            mix: self.mix.smoothed.next() as f64,
            output_gain: self.output.smoothed.next() as f64,
        }
        .sanitized()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AriadneFrame {
    pub left: f32,
    pub right: f32,
    pub observed_left: f32,
    pub observed_right: f32,
    pub world_error: bool,
    pub safety_limited: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AriadneEngineReceipt {
    pub world: WorldReceipt,
    pub listener_digest: String,
    pub non_finite_input_frames: u64,
    pub world_errors: u64,
    pub safety_limited_frames: u64,
    pub peak_observed: f64,
    pub peak_output: f64,
}

pub struct AriadneEngine {
    world: World,
    listener: StereoListener,
    body_controls: Vec<BodyControls>,
    non_finite_input_frames: u64,
    world_errors: u64,
    safety_limited_frames: u64,
    peak_observed: f64,
    peak_output: f64,
}

impl Default for AriadneEngine {
    fn default() -> Self {
        Self::new(DEFAULT_SAMPLE_RATE_HZ).expect("48 kHz Ariadne World must construct")
    }
}

impl AriadneEngine {
    pub fn new(sample_rate_hz: f64) -> Result<Self, String> {
        let plan = WorldPlan::string_brass_v1(sample_rate_hz);
        let listener_spec =
            StereoListenerSpec::all_bodies(&plan, 1.0).map_err(|error| error.to_string())?;
        let listener =
            StereoListener::bind(&listener_spec, &plan).map_err(|error| error.to_string())?;
        let world = World::from_plan(plan).map_err(|error| error.to_string())?;
        Ok(Self {
            body_controls: vec![BodyControls::default(); world.body_count()],
            world,
            listener,
            non_finite_input_frames: 0,
            world_errors: 0,
            safety_limited_frames: 0,
            peak_observed: 0.0,
            peak_output: 0.0,
        })
    }

    pub fn prepare(&mut self, sample_rate_hz: f32) -> bool {
        match Self::new(sample_rate_hz as f64) {
            Ok(replacement) => {
                *self = replacement;
                true
            }
            Err(_) => false,
        }
    }

    #[inline]
    pub fn process_sample(
        &mut self,
        left: f32,
        right: f32,
        controls: AriadneControls,
    ) -> AriadneFrame {
        let controls = controls.sanitized();
        if !left.is_finite() || !right.is_finite() {
            self.non_finite_input_frames += 1;
        }
        let dry_left = if left.is_finite() { left as f64 } else { 0.0 };
        let dry_right = if right.is_finite() { right as f64 } else { 0.0 };
        let source = 0.5 * (dry_left + dry_right) * controls.source_coupling;

        let body = &mut self.body_controls[0];
        body.source_wave = source;
        body.work_input = controls.work_input;
        body.active_coupling = controls.active_coupling;
        body.active = ActiveInclusionControls {
            drive: controls.active_drive,
            headroom: controls.headroom,
            sustain: controls.sustain,
            sag: controls.sag,
            bloom: controls.bloom,
            feedback: controls.feedback,
            space: controls.active_space,
        };
        body.transport = ResonantTransportControls {
            interlace: controls.interlace,
            memory: controls.memory,
        };
        body.radiation_fraction = controls.body_radiation;
        let world_controls = WorldControls {
            space: SharedSpaceControls {
                coupling: controls.space_coupling,
                decay: controls.space_decay,
                radiation: controls.space_radiation,
            },
            direct_coupling: Default::default(),
        };

        let observed = self
            .world
            .set_body_configuration_index(0, controls.material_organization)
            .and_then(|_| {
                self.world
                    .process_sample(&self.body_controls, world_controls)
            })
            .and_then(|frame| {
                self.listener
                    .observe(&self.world, &frame)
                    .map_err(|error| mus_ariadne::WorldError::Plan(error.to_string()))
            });
        let (observed_left, observed_right, world_error) = match observed {
            Ok(frame) => (
                frame.left * controls.listener_scale,
                frame.right * controls.listener_scale,
                false,
            ),
            Err(_) => {
                self.world_errors += 1;
                (0.0, 0.0, true)
            }
        };
        self.peak_observed = self
            .peak_observed
            .max(observed_left.abs())
            .max(observed_right.abs());

        let output_left = dry_left.mul_add(1.0 - controls.mix, observed_left * controls.mix)
            * controls.output_gain;
        let output_right = dry_right.mul_add(1.0 - controls.mix, observed_right * controls.mix)
            * controls.output_gain;
        let (left, left_limited) = safe_output(output_left);
        let (right, right_limited) = safe_output(output_right);
        let safety_limited = left_limited || right_limited;
        if safety_limited {
            self.safety_limited_frames += 1;
        }
        self.peak_output = self
            .peak_output
            .max((left as f64).abs())
            .max((right as f64).abs());

        AriadneFrame {
            left,
            right,
            observed_left: finite(observed_left) as f32,
            observed_right: finite(observed_right) as f32,
            world_error,
            safety_limited,
        }
    }

    pub fn process_block(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        controls: AriadneControls,
    ) {
        for (left, right) in left.iter_mut().zip(right.iter_mut()) {
            let frame = self.process_sample(*left, *right, controls);
            *left = frame.left;
            *right = frame.right;
        }
    }

    pub fn reset(&mut self) {
        if self.world.reset().is_err() {
            self.world_errors += 1;
        }
        self.body_controls.fill(BodyControls::default());
        self.non_finite_input_frames = 0;
        self.world_errors = 0;
        self.safety_limited_frames = 0;
        self.peak_observed = 0.0;
        self.peak_output = 0.0;
    }

    #[must_use]
    pub fn latency_samples(&self) -> u32 {
        0
    }

    #[must_use]
    pub fn tail_samples(&self) -> u32 {
        (self.sample_rate() * TAIL_SECONDS).round() as u32
    }

    #[must_use]
    pub fn sample_rate(&self) -> f64 {
        self.world.sample_rate_hz()
    }

    #[must_use]
    pub fn receipt(&self) -> AriadneEngineReceipt {
        AriadneEngineReceipt {
            world: self.world.receipt(),
            listener_digest: self.listener.listener_digest().to_owned(),
            non_finite_input_frames: self.non_finite_input_frames,
            world_errors: self.world_errors,
            safety_limited_frames: self.safety_limited_frames,
            peak_observed: self.peak_observed,
            peak_output: self.peak_output,
        }
    }
}

pub struct AriadneClap {
    params: Arc<AriadneParams>,
    engine: AriadneEngine,
}

impl Default for AriadneClap {
    fn default() -> Self {
        Self {
            params: Arc::new(AriadneParams::default()),
            engine: AriadneEngine::default(),
        }
    }
}

impl Plugin for AriadneClap {
    const NAME: &'static str = "Ariadne World";
    const VENDOR: &'static str = "Sophia Labs";
    const URL: &'static str = "https://github.com/sophia-labs/ovillo";
    const EMAIL: &'static str = "noreply@sophia-labs.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        if !self.engine.prepare(buffer_config.sample_rate) {
            return false;
        }
        context.set_latency_samples(self.engine.latency_samples());
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channels in buffer.iter_samples() {
            let mut channels = channels.into_iter();
            let Some(left) = channels.next() else {
                continue;
            };
            let input_left = *left;
            if let Some(right) = channels.next() {
                let frame =
                    self.engine
                        .process_sample(input_left, *right, self.params.next_controls());
                *left = frame.left;
                *right = frame.right;
            } else {
                let frame =
                    self.engine
                        .process_sample(input_left, input_left, self.params.next_controls());
                *left = 0.5 * (frame.left + frame.right);
            }
        }
        ProcessStatus::Tail(self.engine.tail_samples())
    }
}

impl ClapPlugin for AriadneClap {
    const CLAP_ID: &'static str = "com.sophia-labs.ariadne-world";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("One persistent multi-body Ariadne metamaterial World");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for AriadneClap {
    const VST3_CLASS_ID: [u8; 16] = *b"AriadneWorld0001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx];
}

nice_plug::nice_export_clap!(AriadneClap);
nice_plug::nice_export_vst3!(AriadneClap);

#[inline]
fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

#[inline]
fn safe_output(value: f64) -> (f32, bool) {
    let value = finite(value);
    let limited = value.abs() > OUTPUT_SAFETY_LIMIT;
    (
        value.clamp(-OUTPUT_SAFETY_LIMIT, OUTPUT_SAFETY_LIMIT) as f32,
        limited,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_runs_one_canonical_world_and_listener() {
        let mut engine = AriadneEngine::default();
        for rate in [44_100.0, 48_000.0, 96_000.0, 192_000.0] {
            assert!(engine.prepare(rate));
            assert_eq!(engine.sample_rate(), rate as f64);
            assert_eq!(engine.latency_samples(), 0);
            assert_eq!(engine.receipt().world.body_count, 1);
            assert!(engine.receipt().listener_digest.starts_with("sha256:"));
        }
    }

    #[test]
    fn observed_path_is_finite_and_stereo() {
        let mut engine = AriadneEngine::default();
        let mut left = [0.0_f32; 16_384];
        let mut right = [0.0_f32; 16_384];
        left[0] = 0.6;
        right[0] = 0.6;
        engine.process_block(&mut left, &mut right, AriadneControls::default());
        let mut peak = 0.0_f32;
        let mut stereo_difference = 0.0_f64;
        for (&left, &right) in left.iter().zip(right.iter()) {
            assert!(left.is_finite() && right.is_finite());
            peak = peak.max(left.abs()).max(right.abs());
            stereo_difference += (left - right).abs() as f64;
        }
        assert!(peak > 1.0e-5);
        assert!(stereo_difference > 1.0e-5);
        assert_eq!(engine.receipt().world_errors, 0);
    }

    #[test]
    fn dry_path_is_exact_at_zero_mix() {
        let mut engine = AriadneEngine::default();
        let mut left = [0.0_f32; 1_024];
        let mut right = [0.0_f32; 1_024];
        for (index, sample) in left.iter_mut().enumerate() {
            *sample = (index as f32 * 0.013).sin();
        }
        for (index, sample) in right.iter_mut().enumerate() {
            *sample = (index as f32 * 0.021).cos();
        }
        let expected_left = left;
        let expected_right = right;
        engine.process_block(
            &mut left,
            &mut right,
            AriadneControls {
                mix: 0.0,
                output_gain: 1.0,
                ..Default::default()
            },
        );
        assert_eq!(left, expected_left);
        assert_eq!(right, expected_right);
    }
}
