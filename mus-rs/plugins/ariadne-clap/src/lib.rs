//! Ariadne Terminal — stable VST3/CLAP identity around the canonical World.
//!
//! This crate owns host ABI, legacy parameter migration, control lowering,
//! dry/wet mixing, and output safety. All persistent physical state and every
//! acoustic law execute in `mus_ariadne::World`.

use std::sync::Arc;

use mus_ariadne::law::active::ActiveInclusionControls;
use mus_ariadne::law::transport::ResonantTransportControls;
use mus_ariadne::{SpaceControls, World, WorldControls, WorldReceipt, WorldSpec};
use nice_plug::prelude::*;

const PARAMETER_SMOOTHING_MS: f32 = 20.0;
const SLOW_PARAMETER_SMOOTHING_MS: f32 = 50.0;
const DEFAULT_SAMPLE_RATE_HZ: f64 = 48_000.0;
const TAIL_SECONDS: f64 = 20.0;
const OUTPUT_SAFETY_LIMIT: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum GuitarCharacter {
    #[id = "costa_clara"]
    CostaClara,
    #[id = "edge"]
    Edge,
    #[id = "grind"]
    Grind,
    #[id = "wall"]
    Wall,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AriadneControls {
    pub character: GuitarCharacter,
    pub drive: f64,
    pub headroom: f64,
    pub bias: f64,
    pub tone: f64,
    pub bloom: f64,
    pub sustain: f64,
    pub sag: f64,
    pub interlace: f64,
    pub chirality: f64,
    pub memory: f64,
    pub curvature: f64,
    pub feedback: f64,
    pub space: f64,
    pub room_decay: f64,
    pub mix: f64,
    pub output_gain: f64,
}

impl Default for AriadneControls {
    fn default() -> Self {
        Self {
            character: GuitarCharacter::Edge,
            drive: 0.42,
            headroom: 0.72,
            bias: 0.18,
            tone: 0.62,
            bloom: 0.36,
            sustain: 0.24,
            sag: 0.35,
            interlace: 0.42,
            chirality: 0.36,
            memory: 0.28,
            curvature: 0.30,
            feedback: 0.04,
            space: 0.16,
            room_decay: 0.42,
            mix: 1.0,
            output_gain: 1.0,
        }
    }
}

impl AriadneControls {
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            character: self.character,
            drive: finite(self.drive).clamp(0.0, 1.5),
            headroom: finite(self.headroom).clamp(0.02, 1.0),
            bias: finite(self.bias).clamp(-1.0, 1.0),
            tone: finite(self.tone).clamp(0.0, 1.0),
            bloom: finite(self.bloom).clamp(0.0, 1.0),
            sustain: finite(self.sustain).clamp(0.0, 1.0),
            sag: finite(self.sag).clamp(0.0, 1.0),
            interlace: finite(self.interlace).clamp(0.0, 1.0),
            chirality: finite(self.chirality).clamp(-1.0, 1.0),
            memory: finite(self.memory).clamp(0.0, 1.0),
            curvature: finite(self.curvature).clamp(0.0, 1.0),
            feedback: finite(self.feedback).clamp(0.0, 1.0),
            space: finite(self.space).clamp(0.0, 1.0),
            room_decay: finite(self.room_decay).clamp(0.0, 1.0),
            mix: finite(self.mix).clamp(0.0, 1.0),
            output_gain: finite(self.output_gain).clamp(0.0, 8.0),
        }
    }

    fn lower(self, source_wave: f64) -> (f64, WorldControls) {
        let controls = self.sanitized();
        let profile = CharacterProfile::for_character(controls.character);

        // Preserve the old parameter IDs while giving every parameter a
        // declared coordinate in the one canonical world. The four Character
        // values select nearby regions of the string-to-bore configuration
        // path; continuous controls move within that region without reset.
        let configuration_amount = (profile.configuration
            + 0.12 * (controls.tone - 0.62)
            + 0.04 * (controls.bias - 0.18)
            + 0.06 * (controls.curvature - 0.30)
            + 0.02 * (controls.chirality - 0.36))
            .clamp(0.0, 0.46);

        let drive = controls.drive * profile.drive_scale;
        let work_input = (0.000_10
            + 0.012 * drive * controls.sustain * (0.25 + 0.75 * controls.sag))
            .clamp(0.0, 0.05);
        let transport_memory =
            (controls.memory * (0.85 + 0.15 * controls.chirality.abs())).clamp(0.0, 1.0);

        (
            configuration_amount,
            WorldControls {
                source_wave,
                work_input,
                active_coupling: profile.active_coupling,
                active: ActiveInclusionControls {
                    drive,
                    headroom: controls.headroom,
                    sustain: controls.sustain,
                    sag: controls.sag,
                    bloom: controls.bloom,
                    feedback: controls.feedback,
                    space: controls.space,
                },
                transport: ResonantTransportControls {
                    interlace: (controls.interlace * profile.transport_scale).clamp(0.0, 1.0),
                    memory: transport_memory,
                },
                space: SpaceControls {
                    coupling: controls.space,
                    decay: controls.room_decay,
                    radiation: (0.08 + 0.72 * controls.space + 0.20 * controls.feedback)
                        .clamp(0.0, 1.0),
                },
                radiation_fraction: (0.001 + 0.020 * controls.tone * controls.tone)
                    .clamp(0.0, 0.08),
                // Product calibration is an observation coordinate, not work
                // performed on the persistent body. It raises the diagnostic
                // World output into a useful host range while the final guard
                // below prevents accidental full-scale excursions.
                observation_scale: ((0.50 + 1.20 * controls.tone) * profile.observation_scale)
                    .clamp(0.0, 16.0),
            },
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct CharacterProfile {
    configuration: f64,
    active_coupling: f64,
    drive_scale: f64,
    transport_scale: f64,
    observation_scale: f64,
}

impl CharacterProfile {
    fn for_character(character: GuitarCharacter) -> Self {
        match character {
            GuitarCharacter::CostaClara => Self {
                configuration: 0.00,
                active_coupling: 0.08,
                drive_scale: 0.68,
                transport_scale: 0.62,
                observation_scale: 0.88,
            },
            GuitarCharacter::Edge => Self {
                configuration: 0.06,
                active_coupling: 0.46,
                drive_scale: 1.00,
                transport_scale: 0.90,
                observation_scale: 1.00,
            },
            GuitarCharacter::Grind => Self {
                configuration: 0.15,
                active_coupling: 0.76,
                drive_scale: 1.18,
                transport_scale: 1.14,
                observation_scale: 1.05,
            },
            GuitarCharacter::Wall => Self {
                configuration: 0.27,
                active_coupling: 1.00,
                drive_scale: 1.38,
                transport_scale: 1.30,
                observation_scale: 1.10,
            },
        }
    }
}

#[derive(Params)]
pub struct AriadneParams {
    #[id = "character"]
    pub character: EnumParam<GuitarCharacter>,
    #[id = "drive"]
    pub drive: FloatParam,
    #[id = "headroom"]
    pub headroom: FloatParam,
    #[id = "bias"]
    pub bias: FloatParam,
    #[id = "tone"]
    pub tone: FloatParam,
    #[id = "bloom"]
    pub bloom: FloatParam,
    #[id = "sustain"]
    pub sustain: FloatParam,
    #[id = "sag"]
    pub sag: FloatParam,
    #[id = "interlace"]
    pub interlace: FloatParam,
    #[id = "chirality"]
    pub chirality: FloatParam,
    #[id = "memory"]
    pub memory: FloatParam,
    #[id = "curvature"]
    pub curvature: FloatParam,
    #[id = "feedback"]
    pub feedback: FloatParam,
    #[id = "space"]
    pub space: FloatParam,
    #[id = "room_decay"]
    pub room_decay: FloatParam,
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
                .with_value_to_string(formatters::v2s_f32_rounded(2))
        };
        Self {
            character: EnumParam::new("Character", defaults.character),
            drive: linear("Drive", defaults.drive as f32, 0.0, 1.5),
            headroom: linear("Headroom", defaults.headroom as f32, 0.02, 1.0),
            bias: linear("Bias", defaults.bias as f32, -1.0, 1.0),
            tone: linear("Tone", defaults.tone as f32, 0.0, 1.0),
            bloom: linear("Bloom", defaults.bloom as f32, 0.0, 1.0),
            sustain: linear("Sustain", defaults.sustain as f32, 0.0, 1.0),
            sag: linear("Sag", defaults.sag as f32, 0.0, 1.0),
            interlace: linear("Interlace", defaults.interlace as f32, 0.0, 1.0),
            chirality: linear("Chirality", defaults.chirality as f32, -1.0, 1.0),
            memory: FloatParam::new(
                "Path Memory",
                defaults.memory as f32,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(SLOW_PARAMETER_SMOOTHING_MS))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            curvature: linear("Curvature", defaults.curvature as f32, 0.0, 1.0),
            feedback: FloatParam::new(
                "World Return",
                defaults.feedback as f32,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(SLOW_PARAMETER_SMOOTHING_MS))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            space: FloatParam::new(
                "Spatial Coupling",
                defaults.space as f32,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(SLOW_PARAMETER_SMOOTHING_MS))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            room_decay: FloatParam::new(
                "Halo Decay",
                defaults.room_decay as f32,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(SLOW_PARAMETER_SMOOTHING_MS))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),
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
            character: self.character.value(),
            drive: self.drive.smoothed.next() as f64,
            headroom: self.headroom.smoothed.next() as f64,
            bias: self.bias.smoothed.next() as f64,
            tone: self.tone.smoothed.next() as f64,
            bloom: self.bloom.smoothed.next() as f64,
            sustain: self.sustain.smoothed.next() as f64,
            sag: self.sag.smoothed.next() as f64,
            interlace: self.interlace.smoothed.next() as f64,
            chirality: self.chirality.smoothed.next() as f64,
            memory: self.memory.smoothed.next() as f64,
            curvature: self.curvature.smoothed.next() as f64,
            feedback: self.feedback.smoothed.next() as f64,
            space: self.space.smoothed.next() as f64,
            room_decay: self.room_decay.smoothed.next() as f64,
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
    pub wet_left: f32,
    pub wet_right: f32,
    pub world_error: bool,
    pub safety_limited: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AriadneEngineReceipt {
    pub world: WorldReceipt,
    pub non_finite_input_frames: u64,
    pub world_errors: u64,
    pub safety_limited_frames: u64,
    pub peak_wet: f64,
    pub peak_output: f64,
}

/// Host-independent seam shared by the plug-in and offline renderer.
/// It owns exactly one canonical World and never instantiates the compatibility
/// terminal.
pub struct AriadneEngine {
    world: World,
    non_finite_input_frames: u64,
    world_errors: u64,
    safety_limited_frames: u64,
    peak_wet: f64,
    peak_output: f64,
}

impl Default for AriadneEngine {
    fn default() -> Self {
        Self::new(DEFAULT_SAMPLE_RATE_HZ).expect("48 kHz canonical World must construct")
    }
}

impl AriadneEngine {
    pub fn new(sample_rate_hz: f64) -> Result<Self, String> {
        let mut spec = WorldSpec::string_brass_v1();
        spec.internal_sample_rate_hz = sample_rate_hz;
        let world = World::try_new(spec).map_err(|error| error.to_string())?;
        Ok(Self {
            world,
            non_finite_input_frames: 0,
            world_errors: 0,
            safety_limited_frames: 0,
            peak_wet: 0.0,
            peak_output: 0.0,
        })
    }

    pub fn prepare(&mut self, sample_rate: f32) -> bool {
        match Self::new(sample_rate as f64) {
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
        let inputs_finite = left.is_finite() && right.is_finite();
        if !inputs_finite {
            self.non_finite_input_frames += 1;
        }
        let dry_left = if left.is_finite() { left as f64 } else { 0.0 };
        let dry_right = if right.is_finite() { right as f64 } else { 0.0 };
        let source_wave = 0.5 * (dry_left + dry_right);
        let (configuration_amount, world_controls) = controls.lower(source_wave);

        let world_frame = self
            .world
            .set_configuration_amount(configuration_amount)
            .and_then(|_| self.world.process_sample(world_controls));
        let (wet_left, wet_right, world_error) = match world_frame {
            Ok(frame) => (frame.left, frame.right, false),
            Err(_) => {
                self.world_errors += 1;
                // A failed physical frame cannot be reconstructed on the audio
                // thread. Fall back to the finite dry input for this frame.
                (dry_left, dry_right, true)
            }
        };

        self.peak_wet = self.peak_wet.max(wet_left.abs()).max(wet_right.abs());
        let output_left =
            dry_left.mul_add(1.0 - controls.mix, wet_left * controls.mix) * controls.output_gain;
        let output_right =
            dry_right.mul_add(1.0 - controls.mix, wet_right * controls.mix) * controls.output_gain;
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
            wet_left: finite(wet_left) as f32,
            wet_right: finite(wet_right) as f32,
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
        self.non_finite_input_frames = 0;
        self.safety_limited_frames = 0;
        self.peak_wet = 0.0;
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
            non_finite_input_frames: self.non_finite_input_frames,
            world_errors: self.world_errors,
            safety_limited_frames: self.safety_limited_frames,
            peak_wet: self.peak_wet,
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
    // Keep the product identity stable so existing host sessions retain their
    // parameter and plug-in references across the engine migration.
    const NAME: &'static str = "Ariadne Terminal";
    const VENDOR: &'static str = "Sophia Labs";
    const URL: &'static str = "https://github.com/sophia-labs/instrument";
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
    const CLAP_ID: &'static str = "com.sophia-labs.ariadne-terminal";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("The canonical Ariadne World with work-accounted material and reciprocal space");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for AriadneClap {
    const VST3_CLASS_ID: [u8; 16] = *b"AriadneTerminal1";
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
    fn engine_is_one_native_rate_canonical_world_with_a_tail() {
        let mut engine = AriadneEngine::default();
        for rate in [44_100.0, 48_000.0, 96_000.0, 192_000.0] {
            assert!(engine.prepare(rate));
            assert_eq!(engine.sample_rate(), rate as f64);
            assert_eq!(engine.latency_samples(), 0);
            assert_eq!(engine.tail_samples(), (rate * TAIL_SECONDS as f32) as u32);
            assert!(engine.receipt().world.world_digest.starts_with("sha256:"));
        }
    }

    #[test]
    fn stereo_world_is_finite_audible_and_not_fake_dual_mono() {
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
        let receipt = engine.receipt();
        assert_eq!(receipt.world_errors, 0);
        assert_eq!(receipt.world.sample_frames, left.len() as u64);
    }

    #[test]
    fn dry_path_is_sample_exact_at_zero_mix() {
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
                ..AriadneControls::default()
            },
        );
        assert_eq!(left, expected_left);
        assert_eq!(right, expected_right);
    }

    #[test]
    fn characters_move_one_world_without_replacing_its_identity() {
        let mut engine = AriadneEngine::default();
        let digest = engine.receipt().world.world_digest;
        for character in [
            GuitarCharacter::CostaClara,
            GuitarCharacter::Edge,
            GuitarCharacter::Grind,
            GuitarCharacter::Wall,
        ] {
            engine.process_sample(
                0.2,
                0.2,
                AriadneControls {
                    character,
                    ..AriadneControls::default()
                },
            );
        }
        let receipt = engine.receipt();
        assert_eq!(receipt.world.world_digest, digest);
        assert_eq!(receipt.world.sample_frames, 4);
        assert!((receipt.world.configuration_amount - 0.27).abs() < 1.0e-12);
    }

    #[test]
    fn non_finite_input_falls_back_safely_without_poisoning_world() {
        let mut engine = AriadneEngine::default();
        let bad = engine.process_sample(f32::NAN, f32::INFINITY, AriadneControls::default());
        assert!(bad.left.is_finite() && bad.right.is_finite());
        let good = engine.process_sample(0.2, -0.1, AriadneControls::default());
        assert!(good.left.is_finite() && good.right.is_finite());
        let receipt = engine.receipt();
        assert_eq!(receipt.non_finite_input_frames, 1);
        assert_eq!(receipt.world_errors, 0);
    }

    #[test]
    fn parameter_defaults_match_the_world_adapter_contract() {
        let params = AriadneParams::default();
        let defaults = AriadneControls::default();
        assert!((params.drive.value() as f64 - defaults.drive).abs() < 1.0e-6);
        assert!((params.headroom.value() as f64 - defaults.headroom).abs() < 1.0e-6);
        assert!((params.memory.value() as f64 - defaults.memory).abs() < 1.0e-6);
        assert!((params.mix.value() as f64 - defaults.mix).abs() < 1.0e-6);
        assert_eq!(params.character.value(), GuitarCharacter::Edge);
        assert_eq!(params.output.value(), 1.0);
    }
}
