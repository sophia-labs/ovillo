//! Ariadne Terminal — VST3/CLAP shell around one pure Ariadne material graph.
//!
//! This crate contains only host ABI and parameter plumbing. The current
//! product engine is quarantined as a compatibility implementation owned by
//! `mus-ariadne`; new bodies lower to `mus_ariadne::World`.

use std::sync::Arc;

use mus_ariadne::compat::terminal::{
    AriadneTerminal, AriadneTerminalCharacter, AriadneTerminalControls,
    AriadneTerminalFrame, AriadneTerminalReceipt,
};
use nice_plug::prelude::*;

const PARAMETER_SMOOTHING_MS: f32 = 20.0;
const SLOW_PARAMETER_SMOOTHING_MS: f32 = 50.0;

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
        let linear = |name: &'static str, default, min, max| {
            FloatParam::new(name, default, FloatRange::Linear { min, max })
                .with_smoother(SmoothingStyle::Linear(PARAMETER_SMOOTHING_MS))
                .with_value_to_string(formatters::v2s_f32_rounded(2))
        };
        Self {
            character: EnumParam::new("Character", GuitarCharacter::Edge),
            drive: linear("Drive", 0.42, 0.0, 1.5),
            headroom: linear("Headroom", 0.72, 0.02, 1.0),
            bias: linear("Bias", 0.18, -1.0, 1.0),
            tone: linear("Tone", 0.62, 0.0, 1.0),
            bloom: linear("Bloom", 0.36, 0.0, 1.0),
            sustain: linear("Sustain", 0.24, 0.0, 1.0),
            sag: linear("Sag", 0.35, 0.0, 1.0),
            interlace: linear("Interlace", 0.42, 0.0, 1.0),
            chirality: linear("Chirality", 0.36, -1.0, 1.0),
            memory: FloatParam::new(
                "Path Memory",
                0.28,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(SLOW_PARAMETER_SMOOTHING_MS))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            curvature: linear("Curvature", 0.30, 0.0, 1.0),
            feedback: FloatParam::new(
                "World Return",
                0.04,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(SLOW_PARAMETER_SMOOTHING_MS))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            space: FloatParam::new(
                "Spatial Coupling",
                0.16,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(SLOW_PARAMETER_SMOOTHING_MS))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(1))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            room_decay: FloatParam::new(
                "Halo Decay",
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
    fn next_controls(&self) -> AriadneTerminalControls {
        let character = match self.character.value() {
            GuitarCharacter::CostaClara => AriadneTerminalCharacter::CostaClara,
            GuitarCharacter::Edge => AriadneTerminalCharacter::Edge,
            GuitarCharacter::Grind => AriadneTerminalCharacter::Grind,
            GuitarCharacter::Wall => AriadneTerminalCharacter::Wall,
        };
        AriadneTerminalControls {
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
        .with_character(character)
        .sanitized()
    }
}

/// Host-independent seam used by the plug-in, tests, and offline benchmarks.
pub struct AriadneEngine {
    terminal: AriadneTerminal,
}

impl Default for AriadneEngine {
    fn default() -> Self {
        Self {
            terminal: AriadneTerminal::new(48_000.0).expect("48 kHz is supported"),
        }
    }
}

impl AriadneEngine {
    pub fn prepare(&mut self, sample_rate: f32) -> bool {
        match AriadneTerminal::new(sample_rate as f64) {
            Ok(terminal) => {
                self.terminal = terminal;
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
        controls: AriadneTerminalControls,
    ) -> AriadneTerminalFrame {
        self.terminal.process_sample(left, right, controls)
    }

    pub fn process_block(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        controls: AriadneTerminalControls,
    ) {
        self.terminal.process_block(left, right, controls);
    }

    pub fn reset(&mut self) {
        self.terminal.reset();
    }

    #[must_use]
    pub fn latency_samples(&self) -> u32 {
        self.terminal.latency_samples()
    }

    #[must_use]
    pub fn tail_samples(&self) -> u32 {
        self.terminal.tail_samples()
    }

    #[must_use]
    pub fn sample_rate(&self) -> f64 {
        self.terminal.sample_rate()
    }

    #[must_use]
    pub fn receipt(&self) -> AriadneTerminalReceipt {
        self.terminal.receipt()
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
                let frame = self.engine.process_sample(
                    input_left,
                    *right,
                    self.params.next_controls(),
                );
                *left = frame.left;
                *right = frame.right;
            } else {
                let frame = self.engine.process_sample(
                    input_left,
                    input_left,
                    self.params.next_controls(),
                );
                *left = 0.5 * (frame.left + frame.right);
            }
        }
        ProcessStatus::Tail(self.engine.tail_samples())
    }
}

impl ClapPlugin for AriadneClap {
    const CLAP_ID: &'static str = "com.sophia-labs.ariadne-terminal";
    const CLAP_DESCRIPTION: Option<&'static str> = Some(
        "A powered metric metamaterial with work-accounted path memory and spatial return",
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_is_native_rate_zero_latency_and_has_a_tail() {
        let mut engine = AriadneEngine::default();
        for rate in [44_100.0, 48_000.0, 96_000.0, 192_000.0] {
            assert!(engine.prepare(rate));
            assert_eq!(engine.sample_rate(), rate as f64);
            assert_eq!(engine.latency_samples(), 0);
            assert_eq!(engine.tail_samples(), (rate * 20.0) as u32);
        }
    }

    #[test]
    fn stereo_material_is_finite_audible_and_not_fake_dual_mono() {
        let mut engine = AriadneEngine::default();
        let mut left = [0.0_f32; 8_192];
        let mut right = [0.0_f32; 8_192];
        left[0] = 0.6;
        right[0] = 0.6;
        engine.process_block(
            &mut left,
            &mut right,
            AriadneTerminalControls::default(),
        );

        let mut peak = 0.0_f32;
        let mut stereo_difference = 0.0_f64;
        for (&left, &right) in left.iter().zip(right.iter()) {
            assert!(left.is_finite() && right.is_finite());
            peak = peak.max(left.abs()).max(right.abs());
            stereo_difference += (left - right).abs() as f64;
        }
        assert!(peak > 1.0e-7);
        assert!(stereo_difference > 1.0e-7);
        assert_eq!(engine.receipt().non_finite_resets, 0);
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
            AriadneTerminalControls {
                mix: 0.0,
                output_gain: 1.0,
                ..AriadneTerminalControls::default()
            },
        );
        assert_eq!(left, expected_left);
        assert_eq!(right, expected_right);
    }

    #[test]
    fn parameter_defaults_match_the_core_contract() {
        let params = AriadneParams::default();
        let defaults = AriadneTerminalControls::default();
        assert!((params.drive.value() as f64 - defaults.drive).abs() < 1.0e-6);
        assert!((params.headroom.value() as f64 - defaults.headroom).abs() < 1.0e-6);
        assert!((params.memory.value() as f64 - defaults.memory).abs() < 1.0e-6);
        assert!((params.mix.value() as f64 - defaults.mix).abs() < 1.0e-6);
        assert_eq!(params.character.value(), GuitarCharacter::Edge);
        assert_eq!(params.output.value(), 1.0);
    }
}
