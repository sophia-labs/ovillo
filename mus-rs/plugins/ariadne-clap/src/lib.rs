//! Ariadne Powered Body — VST3/CLAP shell around one `mus-dsp` processor.
//!
//! This crate contains parameter plumbing and plugin ABI declarations only.
//! The sound, stability guards, reservoir, stereo radiator, receipts, and
//! sample-rate behavior all live in [`mus_dsp::powered_radiator`].

use std::sync::Arc;

use mus_dsp::powered_radiator::{
    PoweredRadiator, PoweredRadiatorControls, PoweredRadiatorFrame, PoweredRadiatorReceipt,
    PoweredRadiatorSpace, PoweredRadiatorTone,
};
use nice_plug::prelude::*;

const PARAMETER_SMOOTHING_MS: f32 = 20.0;
const SLOW_PARAMETER_SMOOTHING_MS: f32 = 50.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum GuitarTone {
    #[id = "costa_clara"]
    CostaClara,
    #[id = "edge"]
    Edge,
    #[id = "grind"]
    Grind,
    #[id = "wall"]
    Wall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum SpaceProfile {
    #[id = "off"]
    Off,
    #[id = "telarana_plate"]
    Plate,
    #[id = "telarana_spring"]
    Spring,
    #[id = "telarana_sympathique"]
    Sympathique,
    #[id = "telarana_shimmer_lite"]
    ShimmerLite,
}

#[derive(Params)]
pub struct AriadneParams {
    #[id = "character"]
    pub character: EnumParam<GuitarTone>,
    #[id = "space_profile"]
    pub space_profile: EnumParam<SpaceProfile>,
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
    #[id = "motion"]
    pub motion: FloatParam,
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
        let linear = |name: &'static str, default, min, max| {
            FloatParam::new(name, default, FloatRange::Linear { min, max })
                .with_smoother(SmoothingStyle::Linear(PARAMETER_SMOOTHING_MS))
                .with_value_to_string(formatters::v2s_f32_rounded(2))
        };
        Self {
            character: EnumParam::new("Character", GuitarTone::Edge),
            space_profile: EnumParam::new("Telaraña", SpaceProfile::Plate),
            drive: linear("Drive", 0.42, 0.0, 1.5),
            headroom: linear("Headroom", 0.72, 0.05, 1.0),
            bias: linear("Bias", 0.18, -1.0, 1.0),
            tone: linear("Tone", 0.62, 0.0, 1.0),
            bloom: linear("Bloom", 0.36, 0.0, 1.0),
            sustain: linear("Sustain", 0.24, 0.0, 1.0),
            sag: linear("Sag", 0.35, 0.0, 1.0),
            interlace: linear("Interlace", 0.42, 0.0, 1.0),
            chirality: linear("Chirality", 0.36, -1.0, 1.0),
            motion: FloatParam::new(
                "Motion",
                0.28,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 8.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_smoother(SmoothingStyle::Linear(SLOW_PARAMETER_SMOOTHING_MS))
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            curvature: linear("Curvature", 0.30, 0.0, 1.0),
            feedback: FloatParam::new(
                "Feedback",
                0.04,
                FloatRange::Linear {
                    min: 0.0,
                    max: 0.28,
                },
            )
            .with_smoother(SmoothingStyle::Linear(SLOW_PARAMETER_SMOOTHING_MS))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            space: FloatParam::new("Space Mix", 0.45, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(SLOW_PARAMETER_SMOOTHING_MS))
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(1))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            room_decay: FloatParam::new(
                "Room Decay",
                0.42,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(SLOW_PARAMETER_SMOOTHING_MS))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            mix: FloatParam::new("Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(PARAMETER_SMOOTHING_MS))
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(1))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            output: FloatParam::new(
                "Output",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-24.0),
                    max: util::db_to_gain(12.0),
                    factor: FloatRange::gain_skew_factor(-24.0, 12.0),
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
    fn next_controls(&self) -> PoweredRadiatorControls {
        let tone = match self.character.value() {
            GuitarTone::CostaClara => PoweredRadiatorTone::CostaClara,
            GuitarTone::Edge => PoweredRadiatorTone::Edge,
            GuitarTone::Grind => PoweredRadiatorTone::Grind,
            GuitarTone::Wall => PoweredRadiatorTone::Wall,
        };
        let space = match self.space_profile.value() {
            SpaceProfile::Off => PoweredRadiatorSpace::Off,
            SpaceProfile::Plate => PoweredRadiatorSpace::Plate,
            SpaceProfile::Spring => PoweredRadiatorSpace::Spring,
            SpaceProfile::Sympathique => PoweredRadiatorSpace::Sympathique,
            SpaceProfile::ShimmerLite => PoweredRadiatorSpace::Shimmer,
        };
        let controls = PoweredRadiatorControls {
            drive: self.drive.smoothed.next() as f64,
            headroom: self.headroom.smoothed.next() as f64,
            bias: self.bias.smoothed.next() as f64,
            tone: self.tone.smoothed.next() as f64,
            bloom: self.bloom.smoothed.next() as f64,
            sustain: self.sustain.smoothed.next() as f64,
            sag: self.sag.smoothed.next() as f64,
            interlace: self.interlace.smoothed.next() as f64,
            chirality: self.chirality.smoothed.next() as f64,
            motion_hz: self.motion.smoothed.next() as f64,
            curvature: self.curvature.smoothed.next() as f64,
            feedback: self.feedback.smoothed.next() as f64,
            space: 0.0,
            room_character: 0.0,
            room_decay: self.room_decay.smoothed.next() as f64,
            mix: self.mix.smoothed.next() as f64,
            output_gain: self.output.smoothed.next() as f64,
        };
        controls
            .with_tone(tone)
            .with_space(space, self.space.smoothed.next() as f64)
    }
}

/// Host-independent seam used by the plugin, tests, and offline benchmarks.
pub struct AriadneEngine {
    body: PoweredRadiator,
}

impl Default for AriadneEngine {
    fn default() -> Self {
        Self {
            body: PoweredRadiator::new(48_000.0).expect("48 kHz is supported"),
        }
    }
}

impl AriadneEngine {
    /// Construction/host reconfiguration boundary. All source, modal, and room
    /// storage is allocated here, never in [`process_sample`](Self::process_sample).
    pub fn prepare(&mut self, sample_rate: f32) -> bool {
        match PoweredRadiator::new(sample_rate as f64) {
            Ok(body) => {
                self.body = body;
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
        controls: PoweredRadiatorControls,
    ) -> PoweredRadiatorFrame {
        self.body.process_sample(left, right, controls)
    }

    pub fn process_block(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        controls: PoweredRadiatorControls,
    ) {
        self.body.process_block(left, right, controls);
    }

    pub fn reset(&mut self) {
        self.body.reset();
    }

    #[must_use]
    pub fn latency_samples(&self) -> u32 {
        self.body.latency_samples()
    }

    #[must_use]
    pub fn tail_samples(&self) -> u32 {
        self.body.tail_samples()
    }

    #[must_use]
    pub fn sample_rate(&self) -> f64 {
        self.body.sample_rate()
    }

    #[must_use]
    pub fn receipt(&self) -> PoweredRadiatorReceipt {
        self.body.receipt()
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
    const NAME: &'static str = "Ariadne";
    const VENDOR: &'static str = "Sophia Labs";
    const URL: &'static str = "https://github.com/sophia-labs/mus";
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
    const CLAP_ID: &'static str = "com.sophia-labs.ariadne-powered-body";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A work-accounted powered material and causal room field for live guitar");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Reverb,
        ClapFeature::Distortion,
    ];
}

impl Vst3Plugin for AriadneClap {
    // Stable forever once shipped: hosts and saved sessions identify the plugin
    // by these 16 bytes, not by its display name.
    const VST3_CLASS_ID: [u8; 16] = *b"AriadnePwrBody01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Reverb,
        Vst3SubCategory::Distortion,
    ];
}

nice_plug::nice_export_clap!(AriadneClap);
nice_plug::nice_export_vst3!(AriadneClap);

#[cfg(test)]
mod tests {
    use super::*;
    use mus_dsp::powered_radiator::LATENCY_SAMPLES;

    #[test]
    fn engine_is_native_rate_and_reports_declared_low_latency() {
        let mut engine = AriadneEngine::default();
        for rate in [44_100.0, 48_000.0, 96_000.0, 192_000.0] {
            assert!(engine.prepare(rate));
            assert_eq!(engine.sample_rate(), rate as f64);
            assert_eq!(engine.latency_samples(), LATENCY_SAMPLES as u32);
            assert_eq!(engine.tail_samples(), (rate * 20.0) as u32);
        }
    }

    #[test]
    fn stereo_effect_is_finite_audible_and_not_fake_dual_mono() {
        let mut engine = AriadneEngine::default();
        let mut left = [0.0_f32; 4096];
        let mut right = [0.0_f32; 4096];
        left[0] = 1.0;
        right[0] = 1.0;
        engine.process_block(&mut left, &mut right, PoweredRadiatorControls::default());

        let mut peak = 0.0_f32;
        let mut stereo_difference = 0.0_f64;
        for (&left, &right) in left.iter().zip(right.iter()) {
            assert!(left.is_finite() && right.is_finite());
            peak = peak.max(left.abs()).max(right.abs());
            stereo_difference += (left - right).abs() as f64;
        }
        assert!(peak > 1.0e-6);
        assert!(stereo_difference > 1.0e-6);
        assert_eq!(engine.receipt().non_finite_resets, 0);
    }

    #[test]
    fn dry_path_is_sample_exact_at_zero_mix() {
        let mut engine = AriadneEngine::default();
        let mut left = [0.0_f32; 1024];
        let mut right = [0.0_f32; 1024];
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
            PoweredRadiatorControls {
                mix: 0.0,
                output_gain: 1.0,
                ..PoweredRadiatorControls::default()
            },
        );
        assert!(left[..LATENCY_SAMPLES].iter().all(|sample| *sample == 0.0));
        assert!(right[..LATENCY_SAMPLES].iter().all(|sample| *sample == 0.0));
        assert_eq!(
            &left[LATENCY_SAMPLES..],
            &expected_left[..expected_left.len() - LATENCY_SAMPLES]
        );
        assert_eq!(
            &right[LATENCY_SAMPLES..],
            &expected_right[..expected_right.len() - LATENCY_SAMPLES]
        );
    }

    #[test]
    fn parameter_defaults_match_the_profile() {
        let params = AriadneParams::default();
        let defaults = PoweredRadiatorControls::default();
        assert!((params.drive.value() as f64 - defaults.drive).abs() < 1.0e-6);
        assert!((params.headroom.value() as f64 - defaults.headroom).abs() < 1.0e-6);
        assert!((params.feedback.value() as f64 - defaults.feedback).abs() < 1.0e-6);
        assert!((params.mix.value() as f64 - defaults.mix).abs() < 1.0e-6);
        assert_eq!(params.character.value(), GuitarTone::Edge);
        assert_eq!(params.space_profile.value(), SpaceProfile::Plate);
        assert_eq!(params.output.value(), 1.0);
    }
}
