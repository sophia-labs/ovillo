//! A bounded, work-accounted powered material for live guitar input.
//!
//! This is deliberately not an amplifier or cabinet circuit model. The input
//! is a normalized source port. Three persistent cells draw gain from one
//! finite reservoir and meet asymmetric, strictly contractive contact stops.
//! The remaining broadband port and eight coupled modes radiate in parallel.
//! Four permanently present Telaraña fields—Plate, Spring, Sympathique, and
//! Shimmer Lite—can return energy to the same source port.
//!
//! Through 96 kHz, the nonlinear source island is oversampled 4x with a fixed
//! linear-phase FIR pair. Above 96 kHz the already-high-rate host stream runs
//! natively through the island and an aligned delay preserves one host-facing
//! latency. That delay is reported to the host and the dry path is aligned.
//! Contact and supply work are exact in the declared normalized port metric.
//! The FIR pair is measured but its internal storage is not yet decomposed in
//! the physical ledger; [`PoweredRadiatorReceipt::source_filter_storage_residual`]
//! keeps that mathematical obligation visible for review.

use serde::{Deserialize, Serialize};
use std::f64::consts::{FRAC_1_SQRT_2, PI};

pub const MODE_COUNT: usize = 8;
/// Activation bounds are deliberately wider than the acoustically calibrated
/// 44.1--192 kHz range. Hosts and validators legitimately probe unusual and
/// fractional rates; the lossless rotations remain finite there, while users
/// get the declared reference response at ordinary production rates.
pub const MIN_SAMPLE_RATE: f64 = 1_000.0;
pub const MAX_SAMPLE_RATE: f64 = 768_000.0;
pub const OVERSAMPLE_FACTOR: usize = 4;
pub const LATENCY_SAMPLES: usize = (FIR_TAPS - 1) / OVERSAMPLE_FACTOR;

// A 385-tap Blackman prototype keeps 20 kHz within 0.9 dB at 44.1 kHz while
// rejecting the first 4x-decimation image by at least ~75 dB. Polyphase
// interpolation and a symmetric decimate-only convolution keep its sample
// path cost practical; the FIR pair contributes 96 host samples of latency.
const FIR_TAPS: usize = 385;
const UPSAMPLER_HISTORY: usize = FIR_TAPS.div_ceil(OVERSAMPLE_FACTOR);
const CONTACT_STAGES: usize = 3;
const CABINET_SECTIONS: usize = 3;
const TAIL_FIELDS: usize = 4;
const PLATE_COURSES: usize = 24;
const SPRING_COURSES: usize = 3;
const SYMPATHETIC_COURSES: usize = 8;
const SHIMMER_COURSES: usize = 15;
const DISPERSION_STAGES: usize = 4;
const CHROMA_BINS: usize = 48;
const RESERVOIR_CAPACITY: f64 = 1.0;
const DENORMAL_FLOOR: f64 = 1.0e-24;

#[derive(Debug, Clone, Copy)]
struct ModeSpec {
    frequency_hz: f64,
    t60_seconds: f64,
    cubic: f64,
    quartic: f64,
    drive_coupling: f64,
    radiation_coupling: f64,
    center_x: f64,
}

// Design-hypothesis values from profiles/ariadne-jazzmaster-v0.json. They are
// not measurements of a particular physical guitar.
const ARIADNE_MODES: [ModeSpec; MODE_COUNT] = [
    ModeSpec {
        frequency_hz: 88.0,
        t60_seconds: 1.20,
        cubic: 0.010,
        quartic: 0.020,
        drive_coupling: 0.34,
        radiation_coupling: 0.58,
        center_x: -0.20,
    },
    ModeSpec {
        frequency_hz: 176.0,
        t60_seconds: 0.95,
        cubic: -0.014,
        quartic: 0.028,
        drive_coupling: 0.40,
        radiation_coupling: 0.66,
        center_x: 0.18,
    },
    ModeSpec {
        frequency_hz: 330.0,
        t60_seconds: 0.70,
        cubic: 0.020,
        quartic: 0.040,
        drive_coupling: 0.48,
        radiation_coupling: 0.72,
        center_x: -0.28,
    },
    ModeSpec {
        frequency_hz: 580.0,
        t60_seconds: 0.50,
        cubic: -0.026,
        quartic: 0.055,
        drive_coupling: 0.54,
        radiation_coupling: 0.78,
        center_x: 0.30,
    },
    ModeSpec {
        frequency_hz: 950.0,
        t60_seconds: 0.35,
        cubic: 0.032,
        quartic: 0.075,
        drive_coupling: 0.58,
        radiation_coupling: 0.82,
        center_x: -0.15,
    },
    ModeSpec {
        frequency_hz: 1500.0,
        t60_seconds: 0.24,
        cubic: -0.040,
        quartic: 0.100,
        drive_coupling: 0.62,
        radiation_coupling: 0.78,
        center_x: 0.16,
    },
    ModeSpec {
        frequency_hz: 2400.0,
        t60_seconds: 0.15,
        cubic: 0.050,
        quartic: 0.140,
        drive_coupling: 0.58,
        radiation_coupling: 0.66,
        center_x: -0.10,
    },
    ModeSpec {
        frequency_hz: 3900.0,
        t60_seconds: 0.09,
        cubic: -0.060,
        quartic: 0.180,
        drive_coupling: 0.48,
        radiation_coupling: 0.50,
        center_x: 0.08,
    },
];

#[derive(Debug, Clone, Copy)]
struct Mode {
    q: f64,
    p: f64,
    cosine: f64,
    sine: f64,
    decay: f64,
    cubic: f64,
    quartic: f64,
    drive_coupling: f64,
    radiation_coupling: f64,
    pan_left: f64,
    pan_right: f64,
}

impl Default for Mode {
    fn default() -> Self {
        Self {
            q: 0.0,
            p: 0.0,
            cosine: 1.0,
            sine: 0.0,
            decay: 1.0,
            cubic: 0.0,
            quartic: 0.0,
            drive_coupling: 0.0,
            radiation_coupling: 0.0,
            pan_left: FRAC_1_SQRT_2,
            pan_right: FRAC_1_SQRT_2,
        }
    }
}

/// Host-rate controls. `drive=0`, `curvature=0`, `space=0`, and `mix=0`
/// are exact zero limits. `motion_hz` is retained for state compatibility,
/// but now controls the response rate of input-driven material motion: there
/// is no free-running LFO.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PoweredRadiatorControls {
    pub drive: f64,
    pub headroom: f64,
    pub bias: f64,
    pub tone: f64,
    pub bloom: f64,
    pub sustain: f64,
    pub sag: f64,
    pub interlace: f64,
    pub chirality: f64,
    pub motion_hz: f64,
    pub curvature: f64,
    pub feedback: f64,
    pub space: f64,
    pub room_character: f64,
    pub room_decay: f64,
    pub mix: f64,
    pub output_gain: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoweredRadiatorTone {
    CostaClara,
    Edge,
    Grind,
    Wall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoweredRadiatorSpace {
    Off,
    Plate,
    Spring,
    Sympathique,
    Shimmer,
}

impl Default for PoweredRadiatorControls {
    fn default() -> Self {
        Self {
            drive: 0.42,
            headroom: 0.72,
            bias: 0.18,
            tone: 0.62,
            bloom: 0.36,
            sustain: 0.24,
            sag: 0.35,
            interlace: 0.42,
            chirality: 0.36,
            motion_hz: 0.28,
            curvature: 0.30,
            feedback: 0.04,
            space: 0.16,
            room_character: 0.0,
            room_decay: 0.42,
            mix: 1.0,
            output_gain: 1.0,
        }
    }
}

impl PoweredRadiatorControls {
    /// Resolve a playable tone macro entirely inside the DSP contract. The
    /// wrapper exposes the enum but owns none of these constitutive mappings.
    #[must_use]
    pub fn with_tone(mut self, tone: PoweredRadiatorTone) -> Self {
        let (drive_scale, bias_offset, tone_offset, body_scale, curvature, chirality) = match tone {
            PoweredRadiatorTone::CostaClara => (0.38, -0.08, 0.22, 0.55, 0.12, 0.18),
            PoweredRadiatorTone::Edge => (1.00, 0.00, 0.00, 1.00, 0.30, 0.36),
            PoweredRadiatorTone::Grind => (1.55, 0.14, -0.10, 1.22, 0.58, -0.28),
            PoweredRadiatorTone::Wall => (2.15, -0.20, -0.22, 1.48, 0.88, -0.62),
        };
        self.drive = (self.drive * drive_scale).clamp(0.0, 1.5);
        self.bias = (self.bias + bias_offset).clamp(-1.0, 1.0);
        self.tone = (self.tone + tone_offset).clamp(0.0, 1.0);
        self.interlace = (self.interlace * body_scale).clamp(0.0, 1.0);
        self.curvature = (0.5 * self.curvature + 0.5 * curvature).clamp(0.0, 1.0);
        self.chirality = (self.chirality + chirality).clamp(-1.0, 1.0);
        self
    }

    /// Resolve one of four permanently present room fields. `amount` changes
    /// only the passive source split; selecting a space never swaps algorithms.
    #[must_use]
    pub fn with_space(mut self, space: PoweredRadiatorSpace, amount: f64) -> Self {
        let (space_base, room_character, decay_offset) = match space {
            PoweredRadiatorSpace::Off => (0.0, 0.0, 0.0),
            PoweredRadiatorSpace::Plate => (0.48, 0.0, 0.05),
            PoweredRadiatorSpace::Spring => (0.52, 1.0 / 3.0, -0.04),
            PoweredRadiatorSpace::Sympathique => (0.44, 2.0 / 3.0, 0.24),
            PoweredRadiatorSpace::Shimmer => (0.56, 1.0, 0.30),
        };
        self.space = (space_base * amount).clamp(0.0, 1.0);
        self.room_character = room_character;
        self.room_decay = (self.room_decay + decay_offset).clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn sanitized(self) -> Self {
        fn finite_or(value: f64, fallback: f64) -> f64 {
            if value.is_finite() {
                value
            } else {
                fallback
            }
        }
        let d = Self::default();
        Self {
            drive: finite_or(self.drive, d.drive).clamp(0.0, 1.5),
            headroom: finite_or(self.headroom, d.headroom).clamp(0.05, 1.0),
            bias: finite_or(self.bias, d.bias).clamp(-1.0, 1.0),
            tone: finite_or(self.tone, d.tone).clamp(0.0, 1.0),
            bloom: finite_or(self.bloom, d.bloom).clamp(0.0, 1.0),
            sustain: finite_or(self.sustain, d.sustain).clamp(0.0, 1.0),
            sag: finite_or(self.sag, d.sag).clamp(0.0, 1.0),
            interlace: finite_or(self.interlace, d.interlace).clamp(0.0, 1.0),
            chirality: finite_or(self.chirality, d.chirality).clamp(-1.0, 1.0),
            motion_hz: finite_or(self.motion_hz, d.motion_hz).clamp(0.0, 8.0),
            curvature: finite_or(self.curvature, d.curvature).clamp(0.0, 1.0),
            feedback: finite_or(self.feedback, d.feedback).clamp(0.0, 0.28),
            space: finite_or(self.space, d.space).clamp(0.0, 1.0),
            room_character: finite_or(self.room_character, d.room_character).clamp(0.0, 1.0),
            room_decay: finite_or(self.room_decay, d.room_decay).clamp(0.0, 1.0),
            mix: finite_or(self.mix, d.mix).clamp(0.0, 1.0),
            output_gain: finite_or(self.output_gain, d.output_gain)
                .clamp(0.063_095_734, 3.981_071_706),
        }
    }
}

/// Cumulative observation of the executed source/body/room graph. Every port
/// and stored amplitude uses the same host-time normalized energy coordinate:
/// `0.5*x^2*dt` and `0.5*(q^2+p^2)*dt`, respectively.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PoweredRadiatorReceipt {
    pub frames: u64,
    pub input_work: f64,
    pub supplied_work: f64,
    pub supply_recharge: f64,
    pub contact_loss: f64,
    pub passive_loss: f64,
    pub protective_loss: f64,
    pub radiator_energy: f64,
    pub modal_closure_residual: f64,
    pub room_radiator_energy: f64,
    pub room_loss: f64,
    pub room_exchange: f64,
    pub room_closure_residual: f64,
    pub source_filter_storage_residual: f64,
    pub tracked_key_class: u8,
    pub tracked_key_is_minor: bool,
    pub key_confidence: f64,
    pub reservoir_energy: f64,
    pub minimum_reservoir_energy: f64,
    pub peak_wet: f64,
    /// Samples retired exactly at the `f32` host boundary because their
    /// magnitude was subnormal. This is a numerical hygiene event, not an
    /// unreported constitutive loss inside the physical graph.
    pub denormal_retirements: u64,
    pub non_finite_resets: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PoweredRadiatorFrame {
    pub left: f32,
    pub right: f32,
    pub wet_left: f32,
    pub wet_right: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoweredRadiatorError {
    UnsupportedSampleRate,
}

impl std::fmt::Display for PoweredRadiatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSampleRate => write!(
                formatter,
                "sample rate must be finite and in {MIN_SAMPLE_RATE}..={MAX_SAMPLE_RATE} Hz"
            ),
        }
    }
}

impl std::error::Error for PoweredRadiatorError {}

#[derive(Clone)]
struct Fir {
    coefficients: [f64; FIR_TAPS],
    // Mirrored ring storage turns the hot convolution into one contiguous dot
    // product. This costs 4 KiB per FIR and removes a branch from every tap.
    state: [f64; FIR_TAPS * 2],
    index: usize,
}

impl Fir {
    fn antialias() -> Self {
        let mut coefficients = [0.0; FIR_TAPS];
        let middle = (FIR_TAPS - 1) as f64 * 0.5;
        let cutoff = 0.1165_f64;
        for (index, coefficient) in coefficients.iter_mut().enumerate() {
            let offset = index as f64 - middle;
            let ideal = if offset == 0.0 {
                2.0 * cutoff
            } else {
                (2.0 * PI * cutoff * offset).sin() / (PI * offset)
            };
            let window = 0.42 - 0.5 * (2.0 * PI * index as f64 / (FIR_TAPS - 1) as f64).cos()
                + 0.08 * (4.0 * PI * index as f64 / (FIR_TAPS - 1) as f64).cos();
            *coefficient = ideal * window;
        }
        let sum: f64 = coefficients.iter().sum();
        for coefficient in &mut coefficients {
            *coefficient /= sum;
        }
        // Make analytic linear-phase symmetry bit-exact so the decimator can
        // fold each mirrored tap pair into one multiply.
        for index in 0..FIR_TAPS / 2 {
            let mirror = FIR_TAPS - 1 - index;
            let average = 0.5 * (coefficients[index] + coefficients[mirror]);
            coefficients[index] = average;
            coefficients[mirror] = average;
        }
        Self {
            coefficients,
            state: [0.0; FIR_TAPS * 2],
            index: 0,
        }
    }

    #[inline]
    fn push(&mut self, input: f64) {
        self.index = if self.index == 0 {
            FIR_TAPS - 1
        } else {
            self.index - 1
        };
        self.state[self.index] = input;
        self.state[self.index + FIR_TAPS] = input;
    }

    #[inline]
    fn output(&self) -> f64 {
        let samples = &self.state[self.index..self.index + FIR_TAPS];
        let middle = FIR_TAPS / 2;
        let mut result = self.coefficients[middle] * samples[middle];
        for index in 0..middle {
            let pair = samples[index] + samples[FIR_TAPS - 1 - index];
            result = self.coefficients[index].mul_add(pair, result);
        }
        result
    }

    fn reset(&mut self) {
        self.state.fill(0.0);
        self.index = 0;
    }
}

struct PolyphaseUpsampler {
    coefficients: [[f64; UPSAMPLER_HISTORY]; OVERSAMPLE_FACTOR],
    state: [f64; UPSAMPLER_HISTORY * 2],
    index: usize,
}

impl PolyphaseUpsampler {
    fn new() -> Self {
        let prototype = Fir::antialias();
        let mut coefficients = [[0.0; UPSAMPLER_HISTORY]; OVERSAMPLE_FACTOR];
        for (phase, phase_coefficients) in coefficients.iter_mut().enumerate() {
            for (tap, coefficient) in phase_coefficients.iter_mut().enumerate() {
                let index = phase + OVERSAMPLE_FACTOR * tap;
                if index < FIR_TAPS {
                    *coefficient = prototype.coefficients[index] * OVERSAMPLE_FACTOR as f64;
                }
            }
        }
        Self {
            coefficients,
            state: [0.0; UPSAMPLER_HISTORY * 2],
            index: 0,
        }
    }

    fn push(&mut self, input: f64) {
        self.index = if self.index == 0 {
            UPSAMPLER_HISTORY - 1
        } else {
            self.index - 1
        };
        self.state[self.index] = input;
        self.state[self.index + UPSAMPLER_HISTORY] = input;
    }

    #[inline]
    fn output(&self, phase: usize) -> f64 {
        let mut result = 0.0;
        let samples = &self.state[self.index..self.index + UPSAMPLER_HISTORY];
        for (&coefficient, &sample) in self.coefficients[phase].iter().zip(samples) {
            result = coefficient.mul_add(sample, result);
        }
        result
    }

    fn reset(&mut self) {
        self.state.fill(0.0);
        self.index = 0;
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct OneZero {
    previous: f64,
}

impl OneZero {
    #[inline]
    fn process(&mut self, input: f64, delay_weight: f64, dt: f64) -> (f64, f64) {
        let d = delay_weight.clamp(0.0, 1.0);
        let output = (1.0 - d).mul_add(input, d * self.previous);
        let loss = (0.5 * input * input + 0.5 * d * self.previous * self.previous
            - 0.5 * output * output
            - 0.5 * d * input * input)
            .max(0.0)
            * dt;
        self.previous = input;
        (output, loss)
    }
}

struct PoweredSource {
    up: PolyphaseUpsampler,
    down: Fir,
    interstage: [OneZero; CONTACT_STAGES],
    cabinet: [OneZero; CABINET_SECTIONS],
    native_delay: [f64; LATENCY_SAMPLES],
    native_delay_index: usize,
}

impl PoweredSource {
    fn new() -> Self {
        Self {
            up: PolyphaseUpsampler::new(),
            down: Fir::antialias(),
            interstage: [OneZero::default(); CONTACT_STAGES],
            cabinet: [OneZero::default(); CABINET_SECTIONS],
            native_delay: [0.0; LATENCY_SAMPLES],
            native_delay_index: 0,
        }
    }

    fn reset(&mut self) {
        self.up.reset();
        self.down.reset();
        self.interstage = [OneZero::default(); CONTACT_STAGES];
        self.cabinet = [OneZero::default(); CABINET_SECTIONS];
        self.native_delay.fill(0.0);
        self.native_delay_index = 0;
    }

    fn process(
        &mut self,
        input: f64,
        controls: &PoweredRadiatorControls,
        reservoir: &mut f64,
        receipt: &mut PoweredRadiatorReceipt,
        sample_rate: f64,
        oversample: bool,
    ) -> f64 {
        let high_rate = if oversample {
            sample_rate * OVERSAMPLE_FACTOR as f64
        } else {
            sample_rate
        };
        let dt = high_rate.recip();
        let host_dt = sample_rate.recip();
        let supplied_before = receipt.supplied_work;
        let contact_before = receipt.contact_loss;
        let passive_before = receipt.passive_loss;
        let selected = if oversample {
            let mut selected = 0.0;
            self.up.push(input);
            for phase in 0..OVERSAMPLE_FACTOR {
                let upsampled = self.up.output(phase);
                let value = self.process_stage_chain(upsampled, controls, reservoir, receipt, dt);
                self.down.push(value);
                if phase == 0 {
                    selected = self.down.output();
                }
            }
            selected
        } else {
            let value = self.process_stage_chain(input, controls, reservoir, receipt, dt);
            let selected = self.native_delay[self.native_delay_index];
            self.native_delay[self.native_delay_index] = value;
            self.native_delay_index = (self.native_delay_index + 1) % LATENCY_SAMPLES;
            selected
        };
        // The gain/contact/filter terms inside the island have exact receipts.
        // What remains is the signed change in the FIR pair's undeclared
        // storage, or in the latency delay above 96 kHz. It is expected to
        // return close to zero after a sufficiently long tail.
        let source_input_energy = 0.5 * input * input * host_dt;
        let source_output_energy = 0.5 * selected * selected * host_dt;
        receipt.source_filter_storage_residual += source_input_energy
            + (receipt.supplied_work - supplied_before)
            - (receipt.contact_loss - contact_before)
            - (receipt.passive_loss - passive_before)
            - source_output_energy;
        selected
    }

    #[inline]
    fn process_stage_chain(
        &mut self,
        mut value: f64,
        controls: &PoweredRadiatorControls,
        reservoir: &mut f64,
        receipt: &mut PoweredRadiatorReceipt,
        dt: f64,
    ) -> f64 {
        let drive = controls.drive / 1.5;
        for stage in 0..CONTACT_STAGES {
            let availability = (*reservoir / RESERVOIR_CAPACITY).clamp(0.0, 1.0);
            let nominal_gain = 1.0 + drive * [7.0, 5.0, 3.2][stage];
            let gain = 1.0 + (nominal_gain - 1.0) * availability;
            let desired = value * gain;
            let desired_work = (0.5 * (desired * desired - value * value)).max(0.0) * dt;
            let work = desired_work.min(*reservoir);
            let powered = if desired_work > work && value != 0.0 {
                value.signum() * (value * value + 2.0 * work / dt).sqrt()
            } else {
                desired
            };
            *reservoir -= work;
            receipt.supplied_work += work;

            let sag_bias = controls.sag * (1.0 - availability) * 0.42;
            let bias = (controls.bias + sag_bias).clamp(-1.0, 1.0);
            let base_threshold = [0.48, 0.38, 0.30][stage]
                * (0.44 + 0.86 * controls.headroom)
                * (1.0 - 0.42 * drive);
            let positive = base_threshold * (1.0 + 0.38 * bias).clamp(0.45, 1.55);
            let negative = base_threshold * (1.0 - 0.38 * bias).clamp(0.45, 1.55);
            let hardness = 1.4 + drive * [5.0, 7.5, 10.0][stage];
            let contacted = contractive_contact(powered, positive, negative, hardness);
            receipt.contact_loss += 0.5 * (powered * powered - contacted * contacted).max(0.0) * dt;

            let interstage_weight =
                (0.06 + 0.16 * stage as f64 + 0.12 * (1.0 - controls.tone)).min(0.48);
            let (filtered, loss) = self.interstage[stage].process(contacted, interstage_weight, dt);
            receipt.passive_loss += loss;
            value = filtered;
        }

        let cabinet_weight = 0.18 + 0.31 * (1.0 - controls.tone);
        for section in &mut self.cabinet {
            let (filtered, loss) = section.process(value, cabinet_weight, dt);
            value = filtered;
            receipt.passive_loss += loss;
        }
        value
    }
}

#[inline]
fn contractive_contact(value: f64, positive: f64, negative: f64, hardness: f64) -> f64 {
    if value > positive {
        let over = value - positive;
        positive + over / (1.0 + hardness * over)
    } else if value < -negative {
        let over = -value - negative;
        -(negative + over / (1.0 + hardness * over))
    } else {
        value
    }
}

struct DelayLine {
    samples: Vec<f64>,
    index: usize,
}

impl DelayLine {
    fn new(length: usize) -> Self {
        Self {
            samples: vec![0.0; length.max(1)],
            index: 0,
        }
    }

    #[inline]
    fn read(&self) -> f64 {
        self.samples[self.index]
    }

    #[inline]
    fn write_advance(&mut self, value: f64) {
        self.samples[self.index] = value;
        self.index = (self.index + 1) % self.samples.len();
    }

    fn reset(&mut self) {
        self.samples.fill(0.0);
        self.index = 0;
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct DispersionStage {
    state: f64,
}

#[derive(Default)]
struct TailResult {
    returned_left: f64,
    returned_right: f64,
    radiated_left: f64,
    radiated_right: f64,
    return_to_body: f64,
}

struct LatticeField<const N: usize> {
    lines: [DelayLine; N],
    retains: [f64; N],
    dispersion: [[DispersionStage; DISPERSION_STAGES]; N],
    dispersion_amount: f64,
    dispersion_curvature: f64,
    dispersion_chirality: f64,
    radiator_left: [f64; N],
    radiator_right: [f64; N],
    last_decay_seconds: f64,
    coefficient_countdown: u8,
    active: bool,
}

impl<const N: usize> LatticeField<N> {
    fn new(
        sample_rate: f64,
        frequencies: [f64; N],
        dispersion_amount: f64,
        dispersion_curvature: f64,
        dispersion_chirality: f64,
    ) -> Self {
        assert!(N >= 2);
        let mut radiator_left = [0.0; N];
        let mut radiator_right = [0.0; N];
        radiator_left.fill((N as f64).sqrt().recip());
        let center = (N - 1) as f64 * 0.5;
        for (index, value) in radiator_right.iter_mut().enumerate() {
            *value = index as f64 - center;
        }
        normalize(&mut radiator_right);
        Self {
            lines: std::array::from_fn(|index| {
                DelayLine::new((sample_rate / frequencies[index].max(20.0)).round() as usize)
            }),
            retains: [1.0; N],
            dispersion: [[DispersionStage::default(); DISPERSION_STAGES]; N],
            dispersion_amount,
            dispersion_curvature,
            dispersion_chirality,
            radiator_left,
            radiator_right,
            last_decay_seconds: f64::NAN,
            coefficient_countdown: 0,
            active: false,
        }
    }

    fn reset(&mut self) {
        for line in &mut self.lines {
            line.reset();
        }
        self.dispersion = [[DispersionStage::default(); DISPERSION_STAGES]; N];
        self.coefficient_countdown = 0;
        self.active = false;
    }

    fn is_active(&self) -> bool {
        self.active
    }

    // These are the explicit constitutive controls of one lattice execution;
    // keeping them named at the call site makes the receipt audit legible.
    #[allow(clippy::too_many_arguments)]
    fn process(
        &mut self,
        input_left: f64,
        input_right: f64,
        injection: f64,
        decay_seconds: f64,
        radiation: f64,
        orbit_angle: f64,
        sample_rate: f64,
        receipt: &mut PoweredRadiatorReceipt,
    ) -> TailResult {
        let dt = sample_rate.recip();
        let mut state = std::array::from_fn::<_, N, _>(|index| self.lines[index].read());
        let dispersion_before = self
            .dispersion
            .iter()
            .flatten()
            .map(|stage| 0.5 * stage.state * stage.state * dt)
            .sum::<f64>();
        let energy_before =
            0.5 * state.iter().map(|value| value * value).sum::<f64>() * dt + dispersion_before;
        let mut local_loss = 0.0;

        if self.coefficient_countdown == 0 || (decay_seconds - self.last_decay_seconds).abs() > 0.02
        {
            for (index, retain) in self.retains.iter_mut().enumerate() {
                let delay = self.lines[index].samples.len() as f64 / sample_rate;
                *retain = (-3.0 * 10.0_f64.ln() * delay / decay_seconds.max(0.05)).exp();
            }
            self.last_decay_seconds = decay_seconds;
            self.coefficient_countdown = 63;
        } else {
            self.coefficient_countdown -= 1;
        }
        if self.dispersion_amount > 0.0 {
            const SHAPE: [f64; DISPERSION_STAGES] = [0.17, 0.31, 0.49, 0.71];
            for (line_index, value) in state.iter_mut().enumerate() {
                for (stage_index, stage) in self.dispersion[line_index].iter_mut().enumerate() {
                    let shape = SHAPE[stage_index];
                    let angle = self.dispersion_chirality
                        * self.dispersion_amount
                        * shape
                        * shape.mul_add(self.dispersion_curvature, 1.0)
                        * (1.0 + 0.025 * line_index as f64);
                    (*value, stage.state) = rotate_values_cayley(*value, stage.state, angle);
                }
            }
        }

        for (index, value) in state.iter_mut().enumerate() {
            let before = *value;
            *value *= self.retains[index];
            local_loss += 0.5 * (before * before - *value * *value).max(0.0) * dt;
        }

        if orbit_angle != 0.0 {
            let start = self.coefficient_countdown as usize & 1;
            for first in (start..N.saturating_sub(1)).step_by(2) {
                let (left, right) = state.split_at_mut(first + 1);
                rotate_pair(&mut left[first], &mut right[0], orbit_angle);
            }
        }

        // O(N) orthogonal Householder circulation.
        let inverse_root_n = (N as f64).sqrt().recip();
        let projection = state
            .iter()
            .enumerate()
            .map(|(index, value)| if index % 2 == 0 { *value } else { -*value })
            .sum::<f64>()
            * inverse_root_n;
        for (index, value) in state.iter_mut().enumerate() {
            let sign = if index % 2 == 0 { 1.0 } else { -1.0 };
            *value -= 2.0 * sign * inverse_root_n * projection;
        }

        let left_projection = dot(&self.radiator_left, &state);
        let right_projection = dot(&self.radiator_right, &state);
        let emit = radiation.clamp(0.0, 0.2).sqrt();
        let retain_projection = (1.0 - radiation.clamp(0.0, 0.2)).sqrt();
        let removal = 1.0 - retain_projection;
        for (index, value) in state.iter_mut().enumerate() {
            *value -= removal
                * (self.radiator_left[index] * left_projection
                    + self.radiator_right[index] * right_projection);
        }
        let radiated_left = emit * left_projection;
        let radiated_right = emit * right_projection;
        let local_radiation =
            0.5 * (radiated_left * radiated_left + radiated_right * radiated_right) * dt;

        let mut returned_left = input_left;
        let mut returned_right = input_right;
        let injection_per_course = injection / ((N as f64 * 0.5).ceil().sqrt());
        for (index, value) in state.iter_mut().enumerate() {
            if index % 2 == 0 {
                (returned_left, *value) =
                    rotate_values_cayley(returned_left, *value, injection_per_course);
            } else {
                (returned_right, *value) =
                    rotate_values_cayley(returned_right, *value, injection_per_course);
            }
        }

        let dispersion_after = self
            .dispersion
            .iter()
            .flatten()
            .map(|stage| 0.5 * stage.state * stage.state * dt)
            .sum::<f64>();
        let energy_after =
            0.5 * state.iter().map(|value| value * value).sum::<f64>() * dt + dispersion_after;
        let port_before = 0.5 * (input_left * input_left + input_right * input_right) * dt;
        let port_after =
            0.5 * (returned_left * returned_left + returned_right * returned_right) * dt;
        let exchange = port_before - port_after;
        self.active =
            energy_after > 1.0e-24 || input_left.abs().max(input_right.abs()) > DENORMAL_FLOOR;
        receipt.room_loss += local_loss;
        receipt.room_radiator_energy += local_radiation;
        receipt.room_exchange += exchange;
        receipt.room_closure_residual +=
            energy_after - (energy_before + exchange - local_loss - local_radiation);

        for (line, value) in self.lines.iter_mut().zip(state) {
            line.write_advance(value);
        }

        TailResult {
            returned_left,
            returned_right,
            radiated_left,
            radiated_right,
            return_to_body: 0.25 * inverse_root_n * state.iter().sum::<f64>(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ChromaBin {
    real: f64,
    imaginary: f64,
    cosine: f64,
    sine: f64,
    retain: f64,
    energy: f64,
}

/// A slow observer, never an audio-energy source. Forty-eight semitone
/// resonators cover E2..D#6 and vote into major/minor key templates.
struct KeyTracker {
    bins: [ChromaBin; CHROMA_BINS],
    previous_input: f64,
    energy_alpha: f64,
    countdown: u16,
    root: u8,
    minor: bool,
    confidence: f64,
}

impl KeyTracker {
    fn new(sample_rate: f64) -> Self {
        let bins = std::array::from_fn(|index| {
            let frequency = midi_frequency(40.0 + index as f64);
            let angle = 2.0 * PI * frequency / sample_rate;
            let bandwidth_hz = 5.0 + 0.006 * frequency;
            ChromaBin {
                cosine: angle.cos(),
                sine: angle.sin(),
                retain: (-2.0 * PI * bandwidth_hz / sample_rate).exp(),
                ..ChromaBin::default()
            }
        });
        Self {
            bins,
            previous_input: 0.0,
            energy_alpha: (0.08 * sample_rate).recip(),
            countdown: 0,
            root: 4,
            minor: false,
            confidence: 0.0,
        }
    }

    fn reset(&mut self) {
        for bin in &mut self.bins {
            bin.real = 0.0;
            bin.imaginary = 0.0;
            bin.energy = 0.0;
        }
        self.previous_input = 0.0;
        self.countdown = 0;
        self.root = 4;
        self.minor = false;
        self.confidence = 0.0;
    }

    fn process(&mut self, input: f64) {
        let observed = input - 0.97 * self.previous_input;
        self.previous_input = input;
        for bin in &mut self.bins {
            let real = bin.cosine.mul_add(bin.real, -bin.sine * bin.imaginary);
            let imaginary = bin.sine.mul_add(bin.real, bin.cosine * bin.imaginary);
            bin.real = bin.retain.mul_add(real, (1.0 - bin.retain) * observed);
            bin.imaginary = bin.retain * imaginary;
            let instantaneous = bin.real.mul_add(bin.real, bin.imaginary * bin.imaginary);
            bin.energy += (instantaneous - bin.energy) * self.energy_alpha;
        }
        if self.countdown == 0 {
            self.estimate();
            self.countdown = 1023;
        } else {
            self.countdown -= 1;
        }
    }

    fn estimate(&mut self) {
        let mut chroma = [0.0_f64; 12];
        for (index, bin) in self.bins.iter().enumerate() {
            chroma[(40 + index) % 12] += bin.energy;
        }
        if chroma.iter().sum::<f64>() < 1.0e-18 {
            self.confidence *= 0.98;
            return;
        }

        let mut best = (f64::NEG_INFINITY, self.root, self.minor);
        let mut second = f64::NEG_INFINITY;
        for root in 0..12_u8 {
            for minor in [false, true] {
                let third = if minor { 3 } else { 4 };
                let root_index = root as usize;
                let score = chroma[root_index]
                    + 0.78 * chroma[(root_index + 7) % 12]
                    + 0.62 * chroma[(root_index + third) % 12]
                    + 0.20 * chroma[(root_index + 2) % 12]
                    + 0.16 * chroma[(root_index + 9) % 12];
                if score > best.0 {
                    second = best.0;
                    best = (score, root, minor);
                } else if score > second {
                    second = score;
                }
            }
        }
        let confidence = ((best.0 - second.max(0.0)) / best.0.max(1.0e-18)).clamp(0.0, 1.0);
        self.confidence += (confidence - self.confidence) * 0.25;
        if confidence > 0.025 {
            self.root = best.1;
            self.minor = best.2;
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SympatheticMode {
    q: f64,
    p: f64,
    midi: f64,
    cosine: f64,
    sine: f64,
    retain: f64,
}

impl Default for SympatheticMode {
    fn default() -> Self {
        Self {
            q: 0.0,
            p: 0.0,
            midi: 40.0,
            cosine: 1.0,
            sine: 0.0,
            retain: 1.0,
        }
    }
}

struct SympatheticField {
    modes: [SympatheticMode; SYMPATHETIC_COURSES],
    radiator_left: [f64; SYMPATHETIC_COURSES * 2],
    radiator_right: [f64; SYMPATHETIC_COURSES * 2],
    coefficient_countdown: u8,
    last_decay_seconds: f64,
    active: bool,
}

impl SympatheticField {
    fn new(sample_rate: f64) -> Self {
        let (radiator_left, radiator_right) = coordinate_radiation_rows();
        let mut result = Self {
            modes: [SympatheticMode::default(); SYMPATHETIC_COURSES],
            radiator_left,
            radiator_right,
            coefficient_countdown: 0,
            last_decay_seconds: f64::NAN,
            active: false,
        };
        result.update_coefficients(4, false, 8.0, sample_rate, true);
        result
    }

    fn reset(&mut self) {
        for mode in &mut self.modes {
            mode.q = 0.0;
            mode.p = 0.0;
        }
        self.coefficient_countdown = 0;
        self.active = false;
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn update_coefficients(
        &mut self,
        root: u8,
        minor: bool,
        decay_seconds: f64,
        sample_rate: f64,
        immediate: bool,
    ) {
        const MAJOR: [f64; SYMPATHETIC_COURSES] = [0.0, 7.0, 12.0, 16.0, 19.0, 24.0, 28.0, 31.0];
        const MINOR: [f64; SYMPATHETIC_COURSES] = [0.0, 7.0, 12.0, 15.0, 19.0, 24.0, 27.0, 31.0];
        let mut base = 36.0 + root as f64;
        while base < 40.0 {
            base += 12.0;
        }
        let degrees = if minor { MINOR } else { MAJOR };
        for (mode, degree) in self.modes.iter_mut().zip(degrees) {
            let target = base + degree;
            mode.midi = if immediate {
                target
            } else {
                mode.midi + 0.08 * (target - mode.midi)
            };
            let angle = 2.0 * PI * midi_frequency(mode.midi) / sample_rate;
            mode.cosine = angle.cos();
            mode.sine = angle.sin();
            mode.retain = (-3.0 * 10.0_f64.ln() / (decay_seconds.max(0.05) * sample_rate)).exp();
        }
        self.last_decay_seconds = decay_seconds;
        self.coefficient_countdown = 63;
    }

    #[allow(clippy::too_many_arguments)]
    fn process(
        &mut self,
        input_left: f64,
        input_right: f64,
        injection: f64,
        decay_seconds: f64,
        radiation: f64,
        root: u8,
        minor: bool,
        sample_rate: f64,
        receipt: &mut PoweredRadiatorReceipt,
    ) -> TailResult {
        let dt = sample_rate.recip();
        if self.coefficient_countdown == 0 || (decay_seconds - self.last_decay_seconds).abs() > 0.02
        {
            self.update_coefficients(root, minor, decay_seconds, sample_rate, false);
        } else {
            self.coefficient_countdown -= 1;
        }
        let energy_before = self
            .modes
            .iter()
            .map(|mode| modal_energy(mode.q, mode.p, dt))
            .sum::<f64>();
        let mut local_loss = 0.0;
        for mode in &mut self.modes {
            let q = mode.cosine.mul_add(mode.q, mode.sine * mode.p);
            let p = (-mode.sine).mul_add(mode.q, mode.cosine * mode.p);
            let before = modal_energy(q, p, dt);
            mode.q = q * mode.retain;
            mode.p = p * mode.retain;
            local_loss += (before - modal_energy(mode.q, mode.p, dt)).max(0.0);
        }

        let mut state = [0.0; SYMPATHETIC_COURSES * 2];
        for (index, mode) in self.modes.iter().enumerate() {
            state[2 * index] = mode.q;
            state[2 * index + 1] = mode.p;
        }
        let left_projection = dot(&self.radiator_left, &state);
        let right_projection = dot(&self.radiator_right, &state);
        let eta = radiation.clamp(0.0, 0.2);
        let emit = eta.sqrt();
        let removal = 1.0 - (1.0 - eta).sqrt();
        for (index, value) in state.iter_mut().enumerate() {
            *value -= removal
                * (self.radiator_left[index] * left_projection
                    + self.radiator_right[index] * right_projection);
        }
        let radiated_left = emit * left_projection;
        let radiated_right = emit * right_projection;
        let local_radiation =
            0.5 * (radiated_left * radiated_left + radiated_right * radiated_right) * dt;
        for (index, mode) in self.modes.iter_mut().enumerate() {
            mode.q = state[2 * index];
            mode.p = state[2 * index + 1];
        }

        let mut returned_left = input_left;
        let mut returned_right = input_right;
        let per_course = injection / 2.0;
        for (index, mode) in self.modes.iter_mut().enumerate() {
            if index % 2 == 0 {
                (returned_left, mode.p) = rotate_values_cayley(returned_left, mode.p, per_course);
            } else {
                (returned_right, mode.p) = rotate_values_cayley(returned_right, mode.p, per_course);
            }
        }
        let energy_after = self
            .modes
            .iter()
            .map(|mode| modal_energy(mode.q, mode.p, dt))
            .sum::<f64>();
        let port_before = 0.5 * (input_left * input_left + input_right * input_right) * dt;
        let port_after =
            0.5 * (returned_left * returned_left + returned_right * returned_right) * dt;
        let exchange = port_before - port_after;
        self.active =
            energy_after > 1.0e-24 || input_left.abs().max(input_right.abs()) > DENORMAL_FLOOR;
        receipt.room_loss += local_loss;
        receipt.room_radiator_energy += local_radiation;
        receipt.room_exchange += exchange;
        receipt.room_closure_residual +=
            energy_after - (energy_before + exchange - local_loss - local_radiation);

        TailResult {
            returned_left,
            returned_right,
            radiated_left,
            radiated_right,
            return_to_body: 0.25
                * (SYMPATHETIC_COURSES as f64).sqrt().recip()
                * self.modes.iter().map(|mode| mode.q).sum::<f64>(),
        }
    }
}

fn midi_frequency(midi: f64) -> f64 {
    440.0 * 2.0_f64.powf((midi - 69.0) / 12.0)
}

fn plate_frequencies() -> [f64; PLATE_COURSES] {
    std::array::from_fn(|index| {
        let cents = (((index * 7) % 5) as f64 - 2.0) * 7.0;
        midi_frequency(40.0 + index as f64 + cents / 100.0)
    })
}

fn spring_frequencies() -> [f64; SPRING_COURSES] {
    [40.0, 47.0, 52.0].map(midi_frequency)
}

fn shimmer_frequencies() -> [f64; SHIMMER_COURSES] {
    std::array::from_fn(|index| {
        let cents = if index % 2 == 0 { 9.0 } else { -9.0 };
        midi_frequency(52.0 + index as f64 + cents / 100.0)
    })
}

fn coordinate_radiation_rows() -> (
    [f64; SYMPATHETIC_COURSES * 2],
    [f64; SYMPATHETIC_COURSES * 2],
) {
    let mut left = [0.0; SYMPATHETIC_COURSES * 2];
    let mut right = [0.0; SYMPATHETIC_COURSES * 2];
    for index in 0..SYMPATHETIC_COURSES {
        left[2 * index] = 1.0;
        right[2 * index] = index as f64 - (SYMPATHETIC_COURSES - 1) as f64 * 0.5;
    }
    normalize(&mut left);
    normalize(&mut right);
    (left, right)
}

#[inline]
fn rotate_values_cayley(port: f64, state: f64, angle: f64) -> (f64, f64) {
    let tangent_half = 0.5 * angle.clamp(-1.0, 1.0);
    let denominator = 1.0 + tangent_half * tangent_half;
    let cos = (1.0 - tangent_half * tangent_half) / denominator;
    let sin = 2.0 * tangent_half / denominator;
    (
        cos.mul_add(port, -sin * state),
        sin.mul_add(port, cos * state),
    )
}

/// Complete powered body and causal room graph. Heap storage is allocated only
/// during construction. The sample path performs no allocation, locking, I/O,
/// FFT, system call, or look-ahead.
pub struct PoweredRadiator {
    sample_rate: f64,
    oversample_source: bool,
    modes: [Mode; MODE_COUNT],
    radiator_left: [f64; MODE_COUNT * 2],
    radiator_right: [f64; MODE_COUNT * 2],
    source: PoweredSource,
    dry_left: [f64; LATENCY_SAMPLES],
    dry_right: [f64; LATENCY_SAMPLES],
    dry_index: usize,
    plate: LatticeField<PLATE_COURSES>,
    spring: LatticeField<SPRING_COURSES>,
    sympathique: SympatheticField,
    shimmer: LatticeField<SHIMMER_COURSES>,
    key_tracker: KeyTracker,
    shimmer_envelope: f64,
    shimmer_envelope_release: f64,
    shimmer_orbit: f64,
    shimmer_orbit_decay: f64,
    shimmer_chirality: f64,
    shimmer_refractory: u32,
    key_accumulator: f64,
    key_stride: u16,
    key_count: u16,
    tail_character: f64,
    tail_character_step: f64,
    reservoir: f64,
    motion: f64,
    feedback_sample: f64,
    frame_index: u64,
    receipt: PoweredRadiatorReceipt,
}

impl PoweredRadiator {
    pub fn new(sample_rate: f64) -> Result<Self, PoweredRadiatorError> {
        if !sample_rate.is_finite() || !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&sample_rate) {
            return Err(PoweredRadiatorError::UnsupportedSampleRate);
        }
        let mut modes = [Mode::default(); MODE_COUNT];
        for (mode, spec) in modes.iter_mut().zip(ARIADNE_MODES) {
            let angle = 2.0 * PI * spec.frequency_hz / sample_rate;
            let pan = ((spec.center_x / 0.65).clamp(-1.0, 1.0) + 1.0) * PI * 0.25;
            *mode = Mode {
                cosine: angle.cos(),
                sine: angle.sin(),
                decay: (-3.0 * 10.0_f64.ln() / (spec.t60_seconds * sample_rate)).exp(),
                cubic: spec.cubic,
                quartic: spec.quartic,
                drive_coupling: spec.drive_coupling,
                radiation_coupling: spec.radiation_coupling,
                pan_left: pan.cos(),
                pan_right: pan.sin(),
                ..Mode::default()
            };
        }

        let (radiator_left, radiator_right) = modal_radiation_rows(&modes);
        let key_stride = (sample_rate / 12_000.0).round().clamp(1.0, u16::MAX as f64) as u16;
        let mut result = Self {
            sample_rate,
            oversample_source: sample_rate <= 96_000.0,
            modes,
            radiator_left,
            radiator_right,
            source: PoweredSource::new(),
            dry_left: [0.0; LATENCY_SAMPLES],
            dry_right: [0.0; LATENCY_SAMPLES],
            dry_index: 0,
            plate: LatticeField::new(sample_rate, plate_frequencies(), 0.0, 0.0, 1.0),
            // The Telaraña note's three-course Spring patch: stiff=.9,
            // curvature=.6, chirality=-.7. All four dispersion cells are
            // lossless Cayley rotations, so the boing is structure, not EQ.
            spring: LatticeField::new(sample_rate, spring_frequencies(), 0.9, 0.6, -0.7),
            sympathique: SympatheticField::new(sample_rate),
            shimmer: LatticeField::new(sample_rate, shimmer_frequencies(), 0.0, 0.0, 1.0),
            key_tracker: KeyTracker::new(sample_rate / key_stride as f64),
            shimmer_envelope: 0.0,
            shimmer_envelope_release: (-1.0 / (0.035 * sample_rate)).exp(),
            shimmer_orbit: 0.0,
            shimmer_orbit_decay: (-1.0 / (1.8 * sample_rate)).exp(),
            shimmer_chirality: 1.0,
            shimmer_refractory: 0,
            key_accumulator: 0.0,
            key_stride,
            key_count: 0,
            tail_character: 0.0,
            tail_character_step: 1.0 / (0.02 * sample_rate + 1.0),
            reservoir: RESERVOIR_CAPACITY,
            motion: 0.0,
            feedback_sample: 0.0,
            frame_index: 0,
            receipt: PoweredRadiatorReceipt::default(),
        };
        result.receipt.reservoir_energy = result.reservoir;
        result.receipt.minimum_reservoir_energy = result.reservoir;
        result.receipt.tracked_key_class = result.key_tracker.root;
        Ok(result)
    }

    #[must_use]
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    #[must_use]
    pub const fn latency_samples(&self) -> u32 {
        LATENCY_SAMPLES as u32
    }

    #[must_use]
    pub fn tail_samples(&self) -> u32 {
        (self.sample_rate * 20.0).round().min(u32::MAX as f64) as u32
    }

    #[must_use]
    pub fn receipt(&self) -> PoweredRadiatorReceipt {
        self.receipt
    }

    pub fn reset(&mut self) {
        for mode in &mut self.modes {
            mode.q = 0.0;
            mode.p = 0.0;
        }
        self.source.reset();
        self.dry_left.fill(0.0);
        self.dry_right.fill(0.0);
        self.dry_index = 0;
        self.plate.reset();
        self.spring.reset();
        self.sympathique.reset();
        self.shimmer.reset();
        self.key_tracker.reset();
        self.shimmer_envelope = 0.0;
        self.shimmer_orbit = 0.0;
        self.shimmer_chirality = 1.0;
        self.shimmer_refractory = 0;
        self.key_accumulator = 0.0;
        self.key_count = 0;
        self.tail_character = 0.0;
        self.reservoir = RESERVOIR_CAPACITY;
        self.motion = 0.0;
        self.feedback_sample = 0.0;
        self.frame_index = 0;
        self.receipt = PoweredRadiatorReceipt {
            reservoir_energy: self.reservoir,
            minimum_reservoir_energy: self.reservoir,
            tracked_key_class: self.key_tracker.root,
            ..PoweredRadiatorReceipt::default()
        };
    }

    pub fn process_sample(
        &mut self,
        input_left: f32,
        input_right: f32,
        controls: PoweredRadiatorControls,
    ) -> PoweredRadiatorFrame {
        let controls = controls.sanitized();
        let host_dt = self.sample_rate.recip();
        let left = sanitize_input(input_left, &mut self.receipt.non_finite_resets);
        let right = sanitize_input(input_right, &mut self.receipt.non_finite_resets);
        let delayed_left = self.dry_left[self.dry_index];
        let delayed_right = self.dry_right[self.dry_index];
        self.dry_left[self.dry_index] = left;
        self.dry_right[self.dry_index] = right;
        self.dry_index = (self.dry_index + 1) % LATENCY_SAMPLES;

        let mono = 0.5 * (left + right);
        self.receipt.input_work += 0.5 * mono * mono / self.sample_rate;

        let recovery_seconds = 0.03 + 1.47 * controls.sag;
        let recovery_step = (recovery_seconds * self.sample_rate).recip();
        let recharge =
            (RESERVOIR_CAPACITY - self.reservoir) * (recovery_step / (1.0 + recovery_step));
        self.reservoir += recharge;
        self.receipt.supply_recharge += recharge;

        let feedback = self.feedback_sample * controls.feedback;
        let source_input = mono + feedback;
        let shaped = self.source.process(
            source_input,
            &controls,
            &mut self.reservoir,
            &mut self.receipt,
            self.sample_rate,
            self.oversample_source,
        );

        let motion_target = if controls.motion_hz == 0.0 {
            0.0
        } else {
            let target = shaped * (0.5 + controls.bloom);
            target / (1.0 + target.abs())
        };
        let motion_rate = if controls.motion_hz == 0.0 {
            1.0
        } else {
            let step = 2.0 * PI * controls.motion_hz / self.sample_rate;
            step / (1.0 + step)
        };
        self.motion += (motion_target - self.motion) * motion_rate;

        let mut broadband = shaped;
        let modal_energy_before = self
            .modes
            .iter()
            .map(|mode| modal_energy(mode.q, mode.p, host_dt))
            .sum::<f64>()
            + 0.5 * broadband * broadband * host_dt;
        let modal_supply_before = self.receipt.supplied_work;
        let modal_loss_before = self.receipt.passive_loss;
        let modal_protection_before = self.receipt.protective_loss;
        let modal_radiation_before = self.receipt.radiator_energy;
        self.weave(&controls);
        for mode in &mut self.modes {
            let angle =
                controls.interlace * mode.drive_coupling * (0.008 + 0.034 * controls.drive / 1.5);
            (broadband, mode.p) = rotate_values_cayley(broadband, mode.p, angle);

            let radius_squared = mode.q.mul_add(mode.q, mode.p * mode.p);
            let nonlinear_angle =
                controls.curvature * (mode.cubic * mode.q + mode.quartic * radius_squared) * 0.22;
            rotate_pair(&mut mode.q, &mut mode.p, nonlinear_angle.clamp(-0.22, 0.22));

            let q_rotated = mode.cosine.mul_add(mode.q, mode.sine * mode.p);
            let p_rotated = (-mode.sine).mul_add(mode.q, mode.cosine * mode.p);
            mode.q = q_rotated;
            mode.p = p_rotated;

            let energy_before = modal_energy(mode.q, mode.p, host_dt);
            mode.q *= mode.decay;
            mode.p *= mode.decay;
            let energy_after = modal_energy(mode.q, mode.p, host_dt);
            self.receipt.passive_loss += (energy_before - energy_after).max(0.0);

            let availability = (self.reservoir / RESERVOIR_CAPACITY).clamp(0.0, 1.0);
            let power = controls.sustain * controls.bloom * shaped.abs().min(1.0) * availability;
            if power > 0.0 && energy_after > 0.0 {
                let desired_multiplier = 1.0 + (mode.decay.recip() - 1.0) * power;
                let desired_work = energy_after * (desired_multiplier * desired_multiplier - 1.0);
                let work = desired_work.min(self.reservoir).max(0.0);
                let multiplier = (1.0 + work / energy_after).sqrt();
                mode.q *= multiplier;
                mode.p *= multiplier;
                self.reservoir -= work;
                self.receipt.supplied_work += work;
            }

            let max_radius = 0.22 + 2.78 * controls.headroom;
            let radius_squared = mode.q.mul_add(mode.q, mode.p * mode.p);
            if radius_squared > max_radius * max_radius {
                let before = 0.5 * radius_squared * host_dt;
                let scale = max_radius / radius_squared.sqrt();
                mode.q *= scale;
                mode.p *= scale;
                self.receipt.protective_loss += before - modal_energy(mode.q, mode.p, host_dt);
            }
            retire_denormal(&mut mode.q);
            retire_denormal(&mut mode.p);
        }

        let (modal_left, modal_right) =
            self.radiate_modes(0.0008 + 0.006 * controls.bloom, host_dt);
        let modal_energy_after = self
            .modes
            .iter()
            .map(|mode| modal_energy(mode.q, mode.p, host_dt))
            .sum::<f64>()
            + 0.5 * broadband * broadband * host_dt;
        self.receipt.modal_closure_residual += modal_energy_after
            - (modal_energy_before + (self.receipt.supplied_work - modal_supply_before)
                - (self.receipt.passive_loss - modal_loss_before)
                - (self.receipt.protective_loss - modal_protection_before)
                - (self.receipt.radiator_energy - modal_radiation_before));
        let body_amount = (0.16 + 0.52 * controls.bloom) * controls.interlace;
        let physical_left = broadband * FRAC_1_SQRT_2 + body_amount * modal_left;
        let physical_right = broadband * FRAC_1_SQRT_2 + body_amount * modal_right;

        let send = controls.space * controls.space;
        let near_gain = (1.0 - send).sqrt();
        let room_gain = send.sqrt();
        let room_input_left = physical_left * room_gain;
        let room_input_right = physical_right * room_gain;
        self.key_accumulator += mono;
        self.key_count += 1;
        if self.key_count == self.key_stride {
            self.key_tracker
                .process(self.key_accumulator / self.key_stride as f64);
            self.key_accumulator = 0.0;
            self.key_count = 0;
        }
        self.receipt.tracked_key_class = self.key_tracker.root;
        self.receipt.tracked_key_is_minor = self.key_tracker.minor;
        self.receipt.key_confidence = self.key_tracker.confidence;

        // Shimmer's orbit is excited by new physical attacks and decays with
        // the tail. Chirality flips only at those re-articulations; silence
        // never advances an oscillator.
        let attack = (mono.abs() - self.shimmer_envelope).max(0.0);
        self.shimmer_envelope = mono
            .abs()
            .max(self.shimmer_envelope * self.shimmer_envelope_release);
        if self.shimmer_refractory > 0 {
            self.shimmer_refractory -= 1;
        }
        if attack > 0.008 && self.shimmer_refractory == 0 {
            self.shimmer_orbit = (self.shimmer_orbit + attack * 14.0).min(1.0);
            self.shimmer_chirality = -self.shimmer_chirality;
            self.shimmer_refractory = (0.035 * self.sample_rate) as u32;
        }
        self.shimmer_orbit *= self.shimmer_orbit_decay;

        self.tail_character +=
            (controls.room_character - self.tail_character) * self.tail_character_step;
        let weights = tail_weights(self.tail_character);
        let mut early_left = 0.0;
        let mut early_right = 0.0;
        let mut room_left = 0.0;
        let mut room_right = 0.0;
        let mut return_to_body = 0.0;
        let decay_scale = 0.35 + 1.65 * controls.room_decay;
        for (index, weight) in weights.into_iter().enumerate() {
            let result = match index {
                0 if weight > 0.0 || self.plate.is_active() => self.plate.process(
                    room_input_left * weight,
                    room_input_right * weight,
                    0.42,
                    9.0 * decay_scale,
                    0.0035,
                    0.0,
                    self.sample_rate,
                    &mut self.receipt,
                ),
                1 if weight > 0.0 || self.spring.is_active() => self.spring.process(
                    room_input_left * weight,
                    room_input_right * weight,
                    0.20,
                    8.0 * decay_scale,
                    0.014,
                    0.0,
                    self.sample_rate,
                    &mut self.receipt,
                ),
                2 if weight > 0.0 || self.sympathique.is_active() => self.sympathique.process(
                    room_input_left * weight,
                    room_input_right * weight,
                    0.16,
                    12.0 * decay_scale,
                    0.006,
                    self.key_tracker.root,
                    self.key_tracker.minor,
                    self.sample_rate,
                    &mut self.receipt,
                ),
                3 if weight > 0.0 || self.shimmer.is_active() => self.shimmer.process(
                    room_input_left * weight,
                    room_input_right * weight,
                    0.34,
                    10.0 * decay_scale,
                    0.005,
                    self.shimmer_chirality
                        * self.shimmer_orbit
                        * 2.6
                        * 0.9
                        * (0.010 + 0.018 * controls.curvature),
                    self.sample_rate,
                    &mut self.receipt,
                ),
                _ => TailResult::default(),
            };
            early_left += weight * result.returned_left;
            early_right += weight * result.returned_right;
            room_left += weight * result.radiated_left;
            room_right += weight * result.radiated_right;
            return_to_body += weight * result.return_to_body;
        }
        self.feedback_sample = return_to_body;

        let wet_left = near_gain * physical_left + early_left + room_left;
        let wet_right = near_gain * physical_right + early_right + room_right;
        if !wet_left.is_finite() || !wet_right.is_finite() || !self.reservoir.is_finite() {
            self.reset_after_fault();
            return PoweredRadiatorFrame::default();
        }

        let wet_peak = wet_left.abs().max(wet_right.abs());
        self.receipt.frames += 1;
        self.receipt.reservoir_energy = self.reservoir;
        self.receipt.minimum_reservoir_energy =
            self.receipt.minimum_reservoir_energy.min(self.reservoir);
        self.receipt.peak_wet = self.receipt.peak_wet.max(wet_peak);
        self.frame_index = self.frame_index.wrapping_add(1);

        let output_left = delayed_left.mul_add(1.0 - controls.mix, wet_left * controls.mix)
            * controls.output_gain;
        let output_right = delayed_right.mul_add(1.0 - controls.mix, wet_right * controls.mix)
            * controls.output_gain;
        PoweredRadiatorFrame {
            left: host_sample(output_left, &mut self.receipt.denormal_retirements),
            right: host_sample(output_right, &mut self.receipt.denormal_retirements),
            wet_left: host_sample(wet_left, &mut self.receipt.denormal_retirements),
            wet_right: host_sample(wet_right, &mut self.receipt.denormal_retirements),
        }
    }

    pub fn process_block(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        controls: PoweredRadiatorControls,
    ) {
        for (left_sample, right_sample) in left.iter_mut().zip(right.iter_mut()) {
            let frame = self.process_sample(*left_sample, *right_sample, controls);
            *left_sample = frame.left;
            *right_sample = frame.right;
        }
    }

    fn weave(&mut self, controls: &PoweredRadiatorControls) {
        if controls.interlace == 0.0 {
            return;
        }
        let handed_motion = controls.chirality * self.motion;
        let angle = controls.interlace
            * (0.004 + 0.012 * controls.curvature)
            * (1.0 + 0.55 * handed_motion);
        let start = self.frame_index as usize & 1;
        for first in (start..MODE_COUNT - 1).step_by(2) {
            let (left, right) = self.modes.split_at_mut(first + 1);
            let a = &mut left[first];
            let b = &mut right[0];
            rotate_pair(&mut a.q, &mut b.q, angle);
            rotate_pair(&mut a.p, &mut b.p, angle);
        }
    }

    fn radiate_modes(&mut self, eta: f64, dt: f64) -> (f64, f64) {
        let mut state = [0.0; MODE_COUNT * 2];
        for (index, mode) in self.modes.iter().enumerate() {
            state[2 * index] = mode.q;
            state[2 * index + 1] = mode.p;
        }
        let left_projection = dot(&self.radiator_left, &state);
        let right_projection = dot(&self.radiator_right, &state);
        let emit = eta.sqrt();
        let removal = 1.0 - (1.0 - eta).sqrt();
        for (index, value) in state.iter_mut().enumerate() {
            *value -= removal
                * (self.radiator_left[index] * left_projection
                    + self.radiator_right[index] * right_projection);
        }
        for (index, mode) in self.modes.iter_mut().enumerate() {
            mode.q = state[2 * index];
            mode.p = state[2 * index + 1];
        }
        let left = emit * left_projection;
        let right = emit * right_projection;
        self.receipt.radiator_energy += 0.5 * (left * left + right * right) * dt;
        (left, right)
    }

    fn reset_after_fault(&mut self) {
        let faults = self.receipt.non_finite_resets.saturating_add(1);
        self.reset();
        self.receipt.non_finite_resets = faults;
    }
}

fn modal_radiation_rows(
    modes: &[Mode; MODE_COUNT],
) -> ([f64; MODE_COUNT * 2], [f64; MODE_COUNT * 2]) {
    let mut left = [0.0; MODE_COUNT * 2];
    let mut right = [0.0; MODE_COUNT * 2];
    for (index, mode) in modes.iter().enumerate() {
        left[2 * index] = 0.35 * mode.radiation_coupling * mode.pan_left;
        left[2 * index + 1] = 0.65 * mode.radiation_coupling * mode.pan_left;
        right[2 * index] = 0.35 * mode.radiation_coupling * mode.pan_right;
        right[2 * index + 1] = 0.65 * mode.radiation_coupling * mode.pan_right;
    }
    normalize(&mut left);
    let projection = dot(&left, &right);
    for index in 0..right.len() {
        right[index] -= projection * left[index];
    }
    normalize(&mut right);
    (left, right)
}

fn tail_weights(character: f64) -> [f64; TAIL_FIELDS] {
    let position = character.clamp(0.0, 1.0) * (TAIL_FIELDS - 1) as f64;
    let mut weights = std::array::from_fn(|index| (1.0 - (position - index as f64).abs()).max(0.0));
    let norm = weights
        .iter()
        .map(|weight| weight * weight)
        .sum::<f64>()
        .sqrt()
        .max(1.0e-12);
    for weight in &mut weights {
        *weight /= norm;
    }
    weights
}

#[inline]
fn dot<const N: usize>(left: &[f64; N], right: &[f64; N]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn normalize<const N: usize>(values: &mut [f64; N]) {
    let norm = values.iter().map(|value| value * value).sum::<f64>().sqrt();
    for value in values {
        *value /= norm;
    }
}

#[inline]
fn modal_energy(q: f64, p: f64, dt: f64) -> f64 {
    0.5 * q.mul_add(q, p * p) * dt
}

#[inline]
fn rotate_pair(a: &mut f64, b: &mut f64, angle: f64) {
    if angle == 0.0 {
        return;
    }
    let tangent_half = 0.5 * angle;
    let denominator = 1.0 + tangent_half * tangent_half;
    let cosine = (1.0 - tangent_half * tangent_half) / denominator;
    let sine = 2.0 * tangent_half / denominator;
    let next_a = cosine.mul_add(*a, sine * *b);
    let next_b = (-sine).mul_add(*a, cosine * *b);
    *a = next_a;
    *b = next_b;
}

#[inline]
fn sanitize_input(sample: f32, fault_count: &mut u64) -> f64 {
    if sample.is_finite() {
        sample as f64
    } else {
        *fault_count = fault_count.saturating_add(1);
        0.0
    }
}

#[inline]
fn retire_denormal(value: &mut f64) {
    if value.abs() < DENORMAL_FLOOR {
        *value = 0.0;
    }
}

/// Convert to the host's sample type without ever handing a subnormal to the
/// DAW. Testing the converted value preserves every normal `f32`, including
/// extremely quiet dry signals, while preventing denormal slow paths in hosts.
#[inline]
fn host_sample(value: f64, retirements: &mut u64) -> f32 {
    let sample = value as f32;
    if sample.is_subnormal() {
        *retirements = retirements.saturating_add(1);
        0.0
    } else {
        sample
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsupported_sample_rates() {
        assert!(PoweredRadiator::new(f64::NAN).is_err());
        assert!(PoweredRadiator::new(999.0).is_err());
        assert!(PoweredRadiator::new(1_000.0).is_ok());
        assert!(PoweredRadiator::new(44_100.0).is_ok());
        assert!(PoweredRadiator::new(192_000.0).is_ok());
        assert!(PoweredRadiator::new(768_000.0).is_ok());
        assert!(PoweredRadiator::new(768_001.0).is_err());
    }

    #[test]
    fn asymmetric_contact_is_contractive_and_has_an_exact_zero_region() {
        for index in -20_000..=20_000 {
            let input = index as f64 / 1000.0;
            let output = contractive_contact(input, 0.3, 0.6, 8.0);
            assert!(output.abs() <= input.abs() + 1.0e-14);
        }
        assert_eq!(contractive_contact(0.2, 0.3, 0.6, 8.0), 0.2);
        assert_ne!(
            contractive_contact(0.8, 0.3, 0.6, 8.0),
            -contractive_contact(-0.8, 0.3, 0.6, 8.0)
        );
    }

    #[test]
    fn antialias_fir_meets_the_declared_passband_and_stopband() {
        let fir = Fir::antialias();
        let magnitude = |normalized_frequency: f64| {
            let middle = FIR_TAPS / 2;
            let angle = 2.0 * PI * normalized_frequency;
            (fir.coefficients[middle]
                + 2.0
                    * (0..middle)
                        .map(|index| {
                            fir.coefficients[index] * (angle * (index as f64 - middle as f64)).cos()
                        })
                        .sum::<f64>())
            .abs()
        };
        let twenty_k_at_44k1_x4 = 20_000.0 / (44_100.0 * OVERSAMPLE_FACTOR as f64);
        let passband_db = 20.0 * magnitude(twenty_k_at_44k1_x4).log10();
        assert!(passband_db >= -1.0, "20 kHz response: {passband_db:.3} dB");

        let stopband_peak = (0..=4096)
            .map(|index| 0.125 + (0.5 - 0.125) * index as f64 / 4096.0)
            .map(magnitude)
            .fold(0.0_f64, f64::max);
        let stopband_db = 20.0 * stopband_peak.log10();
        assert!(stopband_db <= -75.0, "stopband peak: {stopband_db:.3} dB");
    }

    #[test]
    fn modal_radiation_rows_are_orthonormal() {
        let body = PoweredRadiator::new(48_000.0).unwrap();
        assert!((dot(&body.radiator_left, &body.radiator_left) - 1.0).abs() < 1.0e-14);
        assert!((dot(&body.radiator_right, &body.radiator_right) - 1.0).abs() < 1.0e-14);
        assert!(dot(&body.radiator_left, &body.radiator_right).abs() < 1.0e-14);
    }

    #[test]
    fn silence_is_an_exact_fixed_point() {
        let mut body = PoweredRadiator::new(48_000.0).unwrap();
        for _ in 0..100_000 {
            assert_eq!(
                body.process_sample(0.0, 0.0, PoweredRadiatorControls::default()),
                PoweredRadiatorFrame::default()
            );
        }
        assert_eq!(body.receipt().supplied_work, 0.0);
        assert_eq!(body.receipt().non_finite_resets, 0);
    }

    #[test]
    fn zero_mix_is_bit_exact_delayed_dry() {
        let mut body = PoweredRadiator::new(44_100.0).unwrap();
        let controls = PoweredRadiatorControls {
            mix: 0.0,
            output_gain: 1.0,
            ..PoweredRadiatorControls::default()
        };
        let mut history_left = [0.0_f32; LATENCY_SAMPLES];
        let mut history_right = [0.0_f32; LATENCY_SAMPLES];
        for index in 0..20_000 {
            let left = ((index as f64 * 0.019).sin() * 0.7) as f32;
            let right = ((index as f64 * 0.023).cos() * 0.5) as f32;
            let frame = body.process_sample(left, right, controls);
            assert_eq!(
                frame.left.to_bits(),
                history_left[index % LATENCY_SAMPLES].to_bits()
            );
            assert_eq!(
                frame.right.to_bits(),
                history_right[index % LATENCY_SAMPLES].to_bits()
            );
            history_left[index % LATENCY_SAMPLES] = left;
            history_right[index % LATENCY_SAMPLES] = right;
        }
    }

    #[test]
    fn host_boundary_retires_subnormal_samples() {
        let mut body = PoweredRadiator::new(48_000.0).unwrap();
        let controls = PoweredRadiatorControls {
            mix: 0.0,
            output_gain: 1.0,
            ..PoweredRadiatorControls::default()
        };
        let smallest_subnormal = f32::from_bits(1);
        let _ = body.process_sample(smallest_subnormal, -smallest_subnormal, controls);
        let mut delayed = PoweredRadiatorFrame::default();
        for _ in 0..LATENCY_SAMPLES {
            delayed = body.process_sample(0.0, 0.0, controls);
        }
        assert_eq!(delayed.left, 0.0);
        assert_eq!(delayed.right, 0.0);
        assert!(body.receipt().denormal_retirements >= 2);
    }

    #[test]
    fn driven_graph_spends_supply_and_books_contact_loss() {
        let mut body = PoweredRadiator::new(48_000.0).unwrap();
        let controls = PoweredRadiatorControls {
            drive: 1.1,
            space: 0.0,
            ..PoweredRadiatorControls::default()
        };
        for index in 0..48_000 {
            let input = ((index as f64 * 110.0 * 2.0 * PI / 48_000.0).sin() * 0.18) as f32;
            let frame = body.process_sample(input, input, controls);
            assert!(frame.left.is_finite() && frame.right.is_finite());
        }
        let receipt = body.receipt();
        assert!(receipt.supplied_work > 0.0, "{receipt:?}");
        assert!(receipt.contact_loss > 0.0, "{receipt:?}");
        assert!(
            receipt.modal_closure_residual.abs() < 1.0e-12,
            "{receipt:?}"
        );
        assert!(receipt.room_closure_residual.abs() < 1.0e-12, "{receipt:?}");
        assert!((0.0..=RESERVOIR_CAPACITY).contains(&receipt.reservoir_energy));
        assert_eq!(receipt.non_finite_resets, 0);
    }

    #[test]
    fn telarana_has_a_causal_tail_and_zero_space_has_none() {
        let mut dry_room = PoweredRadiator::new(48_000.0).unwrap();
        let mut wet_room = PoweredRadiator::new(48_000.0).unwrap();
        let dry = PoweredRadiatorControls {
            drive: 0.8,
            space: 0.0,
            ..PoweredRadiatorControls::default()
        };
        let wet = PoweredRadiatorControls {
            drive: 0.8,
            space: 0.9,
            room_decay: 0.9,
            ..PoweredRadiatorControls::default()
        };
        let mut dry_tail = 0.0;
        let mut wet_tail = 0.0;
        for index in 0..96_000 {
            let input = if index == 0 { 0.5 } else { 0.0 };
            let a = dry_room.process_sample(input, input, dry);
            let b = wet_room.process_sample(input, input, wet);
            if index > 24_000 {
                dry_tail += a.left.abs() as f64 + a.right.abs() as f64;
                wet_tail += b.left.abs() as f64 + b.right.abs() as f64;
            }
        }
        assert!(
            wet_tail > dry_tail * 4.0 + 1.0e-8,
            "dry={dry_tail} wet={wet_tail}"
        );
    }

    #[test]
    fn all_telarana_profiles_are_finite_distinct_and_receipted() {
        let profiles = [
            PoweredRadiatorSpace::Plate,
            PoweredRadiatorSpace::Spring,
            PoweredRadiatorSpace::Sympathique,
            PoweredRadiatorSpace::Shimmer,
        ];
        let mut checksums = [0.0_f64; TAIL_FIELDS];
        for (profile_index, profile) in profiles.into_iter().enumerate() {
            let mut body = PoweredRadiator::new(48_000.0).unwrap();
            let controls = PoweredRadiatorControls::default().with_space(profile, 0.8);
            for index in 0..96_000 {
                let input = if index < 24_000 {
                    let phase = index as f64 * 2.0 * PI * 110.0 / 48_000.0;
                    (0.11 * phase.sin() + 0.06 * (1.5 * phase).sin()) as f32
                } else {
                    0.0
                };
                let frame = body.process_sample(input, input, controls);
                assert!(frame.left.is_finite() && frame.right.is_finite());
                checksums[profile_index] += frame.left as f64 * 0.37 + frame.right as f64 * 0.63;
            }
            let receipt = body.receipt();
            assert!(
                receipt.room_radiator_energy > 0.0,
                "{profile:?}: {receipt:?}"
            );
            assert!(receipt.room_loss > 0.0, "{profile:?}: {receipt:?}");
            assert!(
                receipt.room_closure_residual.abs() < 1.0e-10,
                "{profile:?}: {receipt:?}"
            );
            assert_eq!(receipt.non_finite_resets, 0);
        }
        for left in 0..checksums.len() {
            for right in left + 1..checksums.len() {
                assert!((checksums[left] - checksums[right]).abs() > 1.0e-3);
            }
        }
    }

    #[test]
    fn key_tracker_finds_sustained_e_minor_evidence() {
        let mut tracker = KeyTracker::new(48_000.0);
        let notes = [52.0, 55.0, 59.0, 64.0];
        for index in 0..(48_000 * 4) {
            let sample = notes
                .iter()
                .map(|midi| {
                    let phase = index as f64 * 2.0 * PI * midi_frequency(*midi) / 48_000.0;
                    phase.sin()
                })
                .sum::<f64>()
                * 0.06;
            tracker.process(sample);
        }
        assert_eq!(
            tracker.root, 4,
            "root={} confidence={}",
            tracker.root, tracker.confidence
        );
        assert!(
            tracker.minor,
            "root={} confidence={}",
            tracker.root, tracker.confidence
        );
        assert!(
            tracker.confidence > 0.01,
            "confidence={}",
            tracker.confidence
        );
    }

    #[test]
    fn non_finite_input_is_retired_without_poisoning_state() {
        let mut body = PoweredRadiator::new(48_000.0).unwrap();
        let frame =
            body.process_sample(f32::NAN, f32::INFINITY, PoweredRadiatorControls::default());
        assert!(frame.left.is_finite() && frame.right.is_finite());
        assert_eq!(body.receipt().non_finite_resets, 2);
    }
}
