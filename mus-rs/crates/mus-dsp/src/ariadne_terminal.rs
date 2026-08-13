//! Ariadne Terminal: a powered metric metamaterial for live guitar input.
//!
//! Electric-guitar behavior is treated as a phenotype of Ariadne's own physics,
//! not as a legacy hardware signal path or fitted post-response. The host
//! signal meets one persistent normalized acoustic state containing nonlinear
//! material cells and a long-delay spatial halo. Character is produced by state
//! evolution, work-accounted positive deformation, path-ordered
//! metric-compatible transport, dissipation, radiation, and reciprocal
//! body/room exchange.
//!
//! The constitutional split is the weighted-polar split already used by Ariadne:
//!
//! ```text
//! A = U H
//! ```
//!
//! `U` is represented by sparse Givens factors from `mus-watsn`. Closed
//! overlapping commutators can return every authored coupling coordinate to its
//! endpoint while leaving nontrivial ordered state memory. `H` is a
//! reservoir-limited rank-one positive deformation. Its exact increase in
//! material energy is removed from the finite reservoir; no scalar sample gain
//! or waveshaping curve exists in the path.
//!
//! Each material cell has the positive asymmetric potential
//!
//! ```text
//! V(q; k, s) = 1/2 q^2 + k/4 [q (q + 2s)]^2,
//! ```
//!
//! advanced with a discrete-gradient solve. Nonlinearity therefore belongs to
//! stored state. Changing `k` or `s` while the cell is displaced performs
//! explicit control work. The room is not a post reverb: delay-course heads
//! exchange energy bidirectionally with the material through lossless rotations
//! before an orthogonal scattering, damping, and radiation step.

#![allow(clippy::needless_range_loop)]

use mus_watsn::{givens, order_defect, WorkFrame, WorkLedger};
use serde::{Deserialize, Serialize};
use std::f64::consts::{FRAC_1_SQRT_2, TAU};

pub const MATERIAL_CELLS: usize = 12;
pub const ROOM_COURSES: usize = 12;
pub const MIN_SAMPLE_RATE: f64 = 8_000.0;
pub const MAX_SAMPLE_RATE: f64 = 384_000.0;

const RESERVOIR_CAPACITY: f64 = 0.002;
const NEWTON_STEPS: usize = 8;
const MATERIAL_COEFFICIENT_PERIOD: u8 = 31;
const ROOM_COEFFICIENT_PERIOD: u8 = 63;
const DENORMAL_FLOOR: f64 = 1.0e-24;
const MAX_PHASE_STEP: f64 = 0.48;

const DEFAULT_FREQUENCIES_HZ: [f64; MATERIAL_CELLS] = [
    52.0, 83.0, 128.0, 191.0, 278.0, 397.0, 557.0, 769.0, 1_049.0, 1_411.0,
    1_871.0, 2_441.0,
];

const DEFAULT_T60_SECONDS: [f64; MATERIAL_CELLS] = [
    0.90, 0.72, 0.58, 0.46, 0.36, 0.28, 0.21, 0.16, 0.12, 0.090, 0.067, 0.050,
];

const DEFAULT_POSITIONS: [f64; MATERIAL_CELLS] = [
    -1.00, -0.82, -0.64, -0.45, -0.27, -0.09, 0.09, 0.27, 0.45, 0.64, 0.82, 1.00,
];

const DEFAULT_CURVATURE_WEIGHTS: [f64; MATERIAL_CELLS] = [
    0.55, 0.68, 0.82, 1.00, 1.12, 1.25, 1.38, 1.50, 1.62, 1.74, 1.86, 2.00,
];

const DEFAULT_BIAS_WEIGHTS: [f64; MATERIAL_CELLS] = [
    1.00, -0.80, 0.65, -0.50, 0.40, -0.30, 0.22, -0.16, 0.11, -0.07, 0.04, -0.02,
];

const DEFAULT_INPUT_WEIGHTS: [f64; MATERIAL_CELLS] = [
    0.72, 0.86, 1.00, 1.10, 1.18, 1.22, 1.20, 1.14, 1.05, 0.94, 0.82, 0.70,
];

const DEFAULT_PUMP_WEIGHTS: [f64; MATERIAL_CELLS] = [
    0.76, 0.91, 1.04, 1.13, 1.19, 1.20, 1.16, 1.08, 0.97, 0.84, 0.70, 0.58,
];

const DEFAULT_RADIATION_WEIGHTS: [f64; MATERIAL_CELLS] = [
    0.58, 0.69, 0.82, 0.96, 1.08, 1.16, 1.20, 1.18, 1.10, 0.98, 0.83, 0.66,
];

const DEFAULT_ROOM_DELAYS_SECONDS: [f64; ROOM_COURSES] = [
    0.0073, 0.0111, 0.0137, 0.0179, 0.0233, 0.0297, 0.0371, 0.0437, 0.0521, 0.0613,
    0.0731, 0.0899,
];

/// One fixed Ariadne anatomy. Profiles change geometry and material constants,
/// never the rendering algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AriadneTerminalProfile {
    pub cell_frequency_hz: [f64; MATERIAL_CELLS],
    pub cell_t60_seconds: [f64; MATERIAL_CELLS],
    pub cell_position: [f64; MATERIAL_CELLS],
    pub curvature_weight: [f64; MATERIAL_CELLS],
    pub bias_weight: [f64; MATERIAL_CELLS],
    pub input_aperture_weight: [f64; MATERIAL_CELLS],
    pub pump_aperture_weight: [f64; MATERIAL_CELLS],
    pub radiation_aperture_weight: [f64; MATERIAL_CELLS],
    pub room_delay_seconds: [f64; ROOM_COURSES],
    /// Unit conversion from normalized emitted-wave coordinates to host full scale.
    pub host_observation_scale: f64,
}

impl Default for AriadneTerminalProfile {
    fn default() -> Self {
        Self {
            cell_frequency_hz: DEFAULT_FREQUENCIES_HZ,
            cell_t60_seconds: DEFAULT_T60_SECONDS,
            cell_position: DEFAULT_POSITIONS,
            curvature_weight: DEFAULT_CURVATURE_WEIGHTS,
            bias_weight: DEFAULT_BIAS_WEIGHTS,
            input_aperture_weight: DEFAULT_INPUT_WEIGHTS,
            pump_aperture_weight: DEFAULT_PUMP_WEIGHTS,
            radiation_aperture_weight: DEFAULT_RADIATION_WEIGHTS,
            room_delay_seconds: DEFAULT_ROOM_DELAYS_SECONDS,
            host_observation_scale: 6.0,
        }
    }
}

impl AriadneTerminalProfile {
    pub fn validate(self) -> Result<Self, AriadneTerminalError> {
        validate_positive_array("cell_frequency_hz", &self.cell_frequency_hz)?;
        validate_positive_array("cell_t60_seconds", &self.cell_t60_seconds)?;
        validate_finite_array("cell_position", &self.cell_position)?;
        validate_nonnegative_array("curvature_weight", &self.curvature_weight)?;
        validate_finite_array("bias_weight", &self.bias_weight)?;
        validate_nonnegative_array("input_aperture_weight", &self.input_aperture_weight)?;
        validate_nonnegative_array("pump_aperture_weight", &self.pump_aperture_weight)?;
        validate_nonnegative_array(
            "radiation_aperture_weight",
            &self.radiation_aperture_weight,
        )?;
        validate_positive_array("room_delay_seconds", &self.room_delay_seconds)?;
        if !self.host_observation_scale.is_finite()
            || !(0.01..=64.0).contains(&self.host_observation_scale)
        {
            return Err(AriadneTerminalError::InvalidProfile(
                "host_observation_scale",
            ));
        }
        validate_nonzero_norm("input_aperture_weight", &self.input_aperture_weight)?;
        validate_nonzero_norm("pump_aperture_weight", &self.pump_aperture_weight)?;
        validate_nonzero_norm(
            "radiation_aperture_weight",
            &self.radiation_aperture_weight,
        )?;
        if self
            .cell_position
            .iter()
            .any(|position| position.abs() > 4.0)
        {
            return Err(AriadneTerminalError::InvalidProfile("cell_position"));
        }
        if self
            .room_delay_seconds
            .iter()
            .any(|delay| *delay < 0.000_25 || *delay > 2.0)
        {
            return Err(AriadneTerminalError::InvalidProfile(
                "room_delay_seconds",
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AriadneTerminalCharacter {
    CostaClara,
    Edge,
    Grind,
    Wall,
}

/// Continuous controls compile into material law, aperture geometry, transport,
/// reservoir conversion, and body/room coupling. None selects a transfer curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AriadneTerminalControls {
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

impl Default for AriadneTerminalControls {
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

impl AriadneTerminalControls {
    #[must_use]
    pub fn with_character(mut self, character: AriadneTerminalCharacter) -> Self {
        let (
            drive_scale,
            headroom_offset,
            bias_offset,
            tone_offset,
            bloom_scale,
            sustain_scale,
            interlace_scale,
            chirality_offset,
            memory_offset,
            curvature_target,
        ) = match character {
            AriadneTerminalCharacter::CostaClara => {
                (0.45, 0.18, -0.12, 0.12, 0.65, 0.55, 0.65, 0.10, -0.12, 0.12)
            }
            AriadneTerminalCharacter::Edge => {
                (1.00, 0.00, 0.00, 0.00, 1.00, 1.00, 1.00, 0.00, 0.00, 0.30)
            }
            AriadneTerminalCharacter::Grind => {
                (1.55, -0.15, 0.16, -0.12, 1.30, 1.35, 1.30, -0.30, 0.25, 0.62)
            }
            AriadneTerminalCharacter::Wall => {
                (2.15, -0.30, -0.40, -0.25, 1.62, 1.70, 1.58, -0.62, 0.48, 0.92)
            }
        };
        self.drive = (self.drive * drive_scale).clamp(0.0, 1.5);
        self.headroom = (self.headroom + headroom_offset).clamp(0.02, 1.0);
        self.bias = (self.bias + bias_offset).clamp(-1.0, 1.0);
        self.tone = (self.tone + tone_offset).clamp(0.0, 1.0);
        self.bloom = (self.bloom * bloom_scale).clamp(0.0, 1.0);
        self.sustain = (self.sustain * sustain_scale).clamp(0.0, 1.0);
        self.interlace = (self.interlace * interlace_scale).clamp(0.0, 1.0);
        self.chirality = (self.chirality + chirality_offset).clamp(-1.0, 1.0);
        self.memory = (self.memory + memory_offset).clamp(0.0, 1.0);
        self.curvature =
            (0.45 * self.curvature + 0.55 * curvature_target).clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn sanitized(self) -> Self {
        let default = Self::default();
        Self {
            drive: finite_or(self.drive, default.drive).clamp(0.0, 1.5),
            headroom: finite_or(self.headroom, default.headroom).clamp(0.02, 1.0),
            bias: finite_or(self.bias, default.bias).clamp(-1.0, 1.0),
            tone: finite_or(self.tone, default.tone).clamp(0.0, 1.0),
            bloom: finite_or(self.bloom, default.bloom).clamp(0.0, 1.0),
            sustain: finite_or(self.sustain, default.sustain).clamp(0.0, 1.0),
            sag: finite_or(self.sag, default.sag).clamp(0.0, 1.0),
            interlace: finite_or(self.interlace, default.interlace).clamp(0.0, 1.0),
            chirality: finite_or(self.chirality, default.chirality).clamp(-1.0, 1.0),
            memory: finite_or(self.memory, default.memory).clamp(0.0, 1.0),
            curvature: finite_or(self.curvature, default.curvature).clamp(0.0, 1.0),
            feedback: finite_or(self.feedback, default.feedback).clamp(0.0, 1.0),
            space: finite_or(self.space, default.space).clamp(0.0, 1.0),
            room_decay: finite_or(self.room_decay, default.room_decay).clamp(0.0, 1.0),
            mix: finite_or(self.mix, default.mix).clamp(0.0, 1.0),
            output_gain: finite_or(self.output_gain, default.output_gain)
                .clamp(0.031_622_776, 7.943_282_347),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AriadneTerminalFrame {
    pub left: f32,
    pub right: f32,
    pub wet_left: f32,
    pub wet_right: f32,
    /// Boundary wave returned by the material before the finite-aperture readout.
    pub returned_left: f32,
    pub returned_right: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct AriadneTerminalReceipt {
    pub frames: u64,
    pub boundary_work: f64,
    pub external_supply_work: f64,
    pub material_control_work: f64,
    pub reservoir_to_material_work: f64,
    pub material_loss: f64,
    pub room_loss: f64,
    pub material_radiation: f64,
    pub room_radiation: f64,
    pub room_exchange: f64,
    pub stored_material_energy: f64,
    pub stored_room_energy: f64,
    pub reservoir_energy: f64,
    pub total_stored_energy: f64,
    pub global_closure_residual: f64,
    pub cumulative_closure_residual: f64,
    pub maximum_abs_closure_residual: f64,
    pub boundary_metric_residual: f64,
    pub pump_metric_residual: f64,
    pub room_coupling_metric_residual: f64,
    pub maximum_suspension_equation_residual: f64,
    pub maximum_closed_transport_chord: f64,
    pub peak_wet: f64,
    pub denormal_retirements: u64,
    pub non_finite_inputs: u64,
    pub non_finite_resets: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AriadneTerminalError {
    UnsupportedSampleRate,
    InvalidProfile(&'static str),
}

impl std::fmt::Display for AriadneTerminalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSampleRate => write!(
                formatter,
                "sample rate must be finite and in {MIN_SAMPLE_RATE}..={MAX_SAMPLE_RATE} Hz"
            ),
            Self::InvalidProfile(field) => {
                write!(formatter, "invalid Ariadne Terminal profile field: {field}")
            }
        }
    }
}

impl std::error::Error for AriadneTerminalError {}

#[derive(Debug, Clone, Copy, Default)]
struct MaterialLaw {
    curvature: f64,
    asymmetry: f64,
}

impl MaterialLaw {
    #[inline]
    fn potential(self, q: f64) -> f64 {
        let shifted = q * (q + 2.0 * self.asymmetry);
        0.5 * q * q + 0.25 * self.curvature * shifted * shifted
    }

    #[inline]
    fn divided_gradient(self, old: f64, new: f64) -> f64 {
        let quadratic = 0.5 + self.curvature * self.asymmetry * self.asymmetry;
        let cubic = self.curvature * self.asymmetry;
        let quartic = 0.25 * self.curvature;
        quadratic * (old + new)
            + cubic * (old * old + old * new + new * new)
            + quartic
                * (old * old * old
                    + old * old * new
                    + old * new * new
                    + new * new * new)
    }

    #[inline]
    fn divided_gradient_derivative(self, old: f64, new: f64) -> f64 {
        let quadratic = 0.5 + self.curvature * self.asymmetry * self.asymmetry;
        let cubic = self.curvature * self.asymmetry;
        let quartic = 0.25 * self.curvature;
        quadratic
            + cubic * (old + 2.0 * new)
            + quartic * (old * old + 2.0 * old * new + 3.0 * new * new)
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct MaterialCell {
    q: f64,
    p: f64,
    law: MaterialLaw,
}

#[derive(Debug)]
struct DelayCourse {
    samples: Vec<f64>,
    index: usize,
}

impl DelayCourse {
    fn new(length: usize) -> Self {
        Self {
            samples: vec![0.0; length.max(8)],
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
struct RoomFrame {
    emitted_left: f64,
    emitted_right: f64,
    loss: f64,
    radiation: f64,
    exchange: f64,
    coupling_metric_residual: f64,
}

struct RoomField {
    lines: [DelayCourse; ROOM_COURSES],
    delay_seconds: [f64; ROOM_COURSES],
    retains: [f64; ROOM_COURSES],
    radiation_left: [f64; ROOM_COURSES],
    radiation_right: [f64; ROOM_COURSES],
    householder: [f64; ROOM_COURSES],
    stored_energy: f64,
    last_decay_seconds: f64,
    coefficient_countdown: u8,
}

impl RoomField {
    fn new(sample_rate: f64, profile: AriadneTerminalProfile) -> Self {
        let lines = std::array::from_fn(|index| {
            DelayCourse::new((sample_rate * profile.room_delay_seconds[index]).round() as usize)
        });
        let mut radiation_left = [1.0; ROOM_COURSES];
        normalize(&mut radiation_left);
        let center = (ROOM_COURSES - 1) as f64 * 0.5;
        let mut radiation_right = std::array::from_fn(|index| index as f64 - center);
        remove_projection(&radiation_left, &mut radiation_right);
        normalize(&mut radiation_right);
        let mut householder = std::array::from_fn(|index| {
            if index % 2 == 0 {
                1.0
            } else {
                -1.0
            }
        });
        normalize(&mut householder);
        Self {
            lines,
            delay_seconds: profile.room_delay_seconds,
            retains: [1.0; ROOM_COURSES],
            radiation_left,
            radiation_right,
            householder,
            stored_energy: 0.0,
            last_decay_seconds: f64::NAN,
            coefficient_countdown: 0,
        }
    }

    fn reset(&mut self) {
        for line in &mut self.lines {
            line.reset();
        }
        self.stored_energy = 0.0;
        self.last_decay_seconds = f64::NAN;
        self.coefficient_countdown = 0;
    }

    #[allow(clippy::too_many_arguments)]
    fn process(
        &mut self,
        momenta: &mut [f64; MATERIAL_CELLS],
        profile: &AriadneTerminalProfile,
        controls: &AriadneTerminalControls,
        sample_rate: f64,
        dt: f64,
    ) -> RoomFrame {
        let mut heads = std::array::from_fn::<_, ROOM_COURSES, _>(|index| {
            self.lines[index].read()
        });
        let head_energy_before = quadratic_energy(&heads, dt);
        let body_energy_before = quadratic_energy(momenta, dt);

        let (bridge_row, _) = aperture_rows(
            &profile.cell_position,
            &profile.radiation_aperture_weight,
            0.25 * (2.0 * controls.space - 1.0),
            0.40 + 0.50 * (1.0 - controls.space),
        );
        let coupling_hz = 280.0
            * controls.space
            * controls.space
            * (0.10 + 0.90 * controls.feedback);
        let base_angle = TAU * coupling_hz / sample_rate;
        let dimension_scale = (MATERIAL_CELLS as f64 / ROOM_COURSES as f64).sqrt();
        for course in 0..ROOM_COURSES {
            let cell = (course * 5 + 1) % MATERIAL_CELLS;
            let angle = base_angle * bridge_row[cell] * dimension_scale;
            let (next_momentum, next_head) = givens(momenta[cell], heads[course], angle);
            momenta[cell] = next_momentum;
            heads[course] = next_head;
        }
        let body_energy_after_coupling = quadratic_energy(momenta, dt);
        let head_energy_after_coupling = quadratic_energy(&heads, dt);
        let exchange = head_energy_after_coupling - head_energy_before;
        let coupling_metric_residual = body_energy_after_coupling - body_energy_before + exchange;

        let projection = dot(&self.householder, &heads);
        for (value, direction) in heads.iter_mut().zip(self.householder) {
            *value -= 2.0 * direction * projection;
        }

        let decay_seconds = 0.12 + 8.0 * controls.room_decay * controls.room_decay;
        if self.coefficient_countdown == 0
            || (decay_seconds - self.last_decay_seconds).abs() > 0.02
        {
            let log_thousand = 3.0 * 10.0_f64.ln();
            for index in 0..ROOM_COURSES {
                self.retains[index] =
                    (-log_thousand * self.delay_seconds[index] / decay_seconds).exp();
            }
            self.last_decay_seconds = decay_seconds;
            self.coefficient_countdown = ROOM_COEFFICIENT_PERIOD;
        } else {
            self.coefficient_countdown -= 1;
        }

        let before_damping = quadratic_energy(&heads, dt);
        for (value, retain) in heads.iter_mut().zip(self.retains) {
            *value *= retain;
        }
        let after_damping = quadratic_energy(&heads, dt);
        let loss = (before_damping - after_damping).max(0.0);

        let eta = (0.001_2 + 0.012 * controls.space * controls.space).clamp(0.0, 0.08);
        let left_projection = dot(&self.radiation_left, &heads);
        let right_projection = dot(&self.radiation_right, &heads);
        let emitted_scale = eta.sqrt();
        let retained_scale = (1.0 - eta).sqrt();
        let removal = 1.0 - retained_scale;
        for index in 0..ROOM_COURSES {
            heads[index] -= removal
                * (self.radiation_left[index] * left_projection
                    + self.radiation_right[index] * right_projection);
        }
        let emitted_left = emitted_scale * left_projection;
        let emitted_right = emitted_scale * right_projection;
        let radiation = 0.5
            * emitted_left.mul_add(emitted_left, emitted_right * emitted_right)
            * dt;

        let head_energy_after = quadratic_energy(&heads, dt);
        self.stored_energy += head_energy_after - head_energy_before;
        if self.stored_energy.abs() < 1.0e-20 {
            self.stored_energy = 0.0;
        }
        for (line, value) in self.lines.iter_mut().zip(heads) {
            line.write_advance(value);
        }

        RoomFrame {
            emitted_left,
            emitted_right,
            loss,
            radiation,
            exchange,
            coupling_metric_residual,
        }
    }
}

#[derive(Default)]
struct TerminalTelemetry {
    boundary_work: f64,
    external_supply_work: f64,
    reservoir_to_material_work: f64,
    material_loss: f64,
    room_loss: f64,
    material_radiation: f64,
    room_radiation: f64,
    room_exchange: f64,
    boundary_metric_residual: f64,
    pump_metric_residual: f64,
    room_coupling_metric_residual: f64,
    maximum_suspension_equation_residual: f64,
    maximum_closed_transport_chord: f64,
    peak_wet: f64,
    denormal_retirements: u64,
    non_finite_inputs: u64,
    non_finite_resets: u64,
}

/// Fixed-capacity material graph. Heap storage is allocated only while the room
/// delay courses are constructed; the sample path allocates and locks nothing.
pub struct AriadneTerminal {
    sample_rate: f64,
    dt: f64,
    profile: AriadneTerminalProfile,
    cells: [MaterialCell; MATERIAL_CELLS],
    material_retains: [f64; MATERIAL_CELLS],
    material_coefficient_countdown: u8,
    last_tone: f64,
    last_sustain: f64,
    room: RoomField,
    reservoir_energy: f64,
    ledger: WorkLedger,
    telemetry: TerminalTelemetry,
}

impl AriadneTerminal {
    pub fn new(sample_rate: f64) -> Result<Self, AriadneTerminalError> {
        Self::with_profile(sample_rate, AriadneTerminalProfile::default())
    }

    pub fn with_profile(
        sample_rate: f64,
        profile: AriadneTerminalProfile,
    ) -> Result<Self, AriadneTerminalError> {
        if !sample_rate.is_finite()
            || !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&sample_rate)
        {
            return Err(AriadneTerminalError::UnsupportedSampleRate);
        }
        let profile = profile.validate()?;
        let default_controls = AriadneTerminalControls::default();
        let cells = std::array::from_fn(|index| MaterialCell {
            law: material_law(&profile, &default_controls, index),
            ..MaterialCell::default()
        });
        let room = RoomField::new(sample_rate, profile);
        let mut result = Self {
            sample_rate,
            dt: sample_rate.recip(),
            profile,
            cells,
            material_retains: [1.0; MATERIAL_CELLS],
            material_coefficient_countdown: 0,
            last_tone: f64::NAN,
            last_sustain: f64::NAN,
            room,
            reservoir_energy: RESERVOIR_CAPACITY,
            ledger: WorkLedger::default(),
            telemetry: TerminalTelemetry::default(),
        };
        result.update_material_retains(&default_controls);
        Ok(result)
    }

    #[must_use]
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    #[must_use]
    pub const fn latency_samples(&self) -> u32 {
        0
    }

    #[must_use]
    pub fn tail_samples(&self) -> u32 {
        (self.sample_rate * 20.0).round().min(u32::MAX as f64) as u32
    }

    #[must_use]
    pub fn profile(&self) -> AriadneTerminalProfile {
        self.profile
    }

    #[must_use]
    pub fn stored_material_energy(&self) -> f64 {
        self.cells
            .iter()
            .map(|cell| self.dt * (0.5 * cell.p * cell.p + cell.law.potential(cell.q)))
            .sum()
    }

    #[must_use]
    pub fn stored_room_energy(&self) -> f64 {
        self.room.stored_energy
    }

    #[must_use]
    pub fn total_stored_energy(&self) -> f64 {
        self.stored_material_energy() + self.room.stored_energy + self.reservoir_energy
    }

    #[must_use]
    pub fn receipt(&self) -> AriadneTerminalReceipt {
        let stored_material_energy = self.stored_material_energy();
        let total_stored_energy =
            stored_material_energy + self.room.stored_energy + self.reservoir_energy;
        AriadneTerminalReceipt {
            frames: self.ledger.frames,
            boundary_work: self.telemetry.boundary_work,
            external_supply_work: self.telemetry.external_supply_work,
            material_control_work: self.ledger.control_work,
            reservoir_to_material_work: self.telemetry.reservoir_to_material_work,
            material_loss: self.telemetry.material_loss,
            room_loss: self.telemetry.room_loss,
            material_radiation: self.telemetry.material_radiation,
            room_radiation: self.telemetry.room_radiation,
            room_exchange: self.telemetry.room_exchange,
            stored_material_energy,
            stored_room_energy: self.room.stored_energy,
            reservoir_energy: self.reservoir_energy,
            total_stored_energy,
            global_closure_residual: self.ledger.global_residual(),
            cumulative_closure_residual: self.ledger.cumulative_residual,
            maximum_abs_closure_residual: self.ledger.max_abs_residual,
            boundary_metric_residual: self.telemetry.boundary_metric_residual,
            pump_metric_residual: self.telemetry.pump_metric_residual,
            room_coupling_metric_residual: self.telemetry.room_coupling_metric_residual,
            maximum_suspension_equation_residual: self
                .telemetry
                .maximum_suspension_equation_residual,
            maximum_closed_transport_chord: self
                .telemetry
                .maximum_closed_transport_chord,
            peak_wet: self.telemetry.peak_wet,
            denormal_retirements: self.telemetry.denormal_retirements,
            non_finite_inputs: self.telemetry.non_finite_inputs,
            non_finite_resets: self.telemetry.non_finite_resets,
        }
    }

    pub fn reset(&mut self) {
        for (index, cell) in self.cells.iter_mut().enumerate() {
            *cell = MaterialCell {
                law: material_law(
                    &self.profile,
                    &AriadneTerminalControls::default(),
                    index,
                ),
                ..MaterialCell::default()
            };
        }
        self.room.reset();
        self.reservoir_energy = RESERVOIR_CAPACITY;
        self.ledger = WorkLedger::default();
        self.telemetry = TerminalTelemetry::default();
        self.material_coefficient_countdown = 0;
        self.last_tone = f64::NAN;
        self.last_sustain = f64::NAN;
        self.update_material_retains(&AriadneTerminalControls::default());
    }

    pub fn process_sample(
        &mut self,
        input_left: f32,
        input_right: f32,
        controls: AriadneTerminalControls,
    ) -> AriadneTerminalFrame {
        let controls = controls.sanitized();
        let left = sanitize_input(input_left, &mut self.telemetry.non_finite_inputs);
        let right = sanitize_input(input_right, &mut self.telemetry.non_finite_inputs);
        let energy_start = self.total_stored_energy();

        let control_work = self.apply_material_law(&controls);
        let external_supply_work = self.recharge_reservoir(&controls);
        let boundary = self.exchange_boundary(left, right, &controls);
        let pump = self.apply_powered_deformation(left.abs().max(right.abs()), &controls);
        let closed_transport_residual = self.apply_closed_transport(&controls);
        let suspension_residual = self.propagate_material(&controls);
        let material_loss = self.damp_material(&controls);

        let mut momenta = std::array::from_fn(|index| self.cells[index].p);
        let room_frame = self.room.process(
            &mut momenta,
            &self.profile,
            &controls,
            self.sample_rate,
            self.dt,
        );
        for (cell, momentum) in self.cells.iter_mut().zip(momenta) {
            cell.p = momentum;
        }
        let material_radiation = self.radiate_material(&controls);

        let wet_left = self.profile.host_observation_scale
            * (material_radiation.0 + room_frame.emitted_left);
        let wet_right = self.profile.host_observation_scale
            * (material_radiation.1 + room_frame.emitted_right);
        if !wet_left.is_finite()
            || !wet_right.is_finite()
            || !self.state_is_finite()
            || !self.reservoir_energy.is_finite()
        {
            self.reset_after_fault();
            return AriadneTerminalFrame::default();
        }

        let energy_end = self.total_stored_energy();
        let internal_loss = material_loss + room_frame.loss;
        let radiation_loss = material_radiation.2 + room_frame.radiation;
        self.ledger.push(WorkFrame {
            energy_start,
            source_work: boundary.work + external_supply_work,
            control_work,
            internal_loss,
            radiation_loss,
            output_work: 0.0,
            energy_end,
        });

        self.telemetry.boundary_work += boundary.work;
        self.telemetry.external_supply_work += external_supply_work;
        self.telemetry.reservoir_to_material_work += pump.work;
        self.telemetry.material_loss += material_loss;
        self.telemetry.room_loss += room_frame.loss;
        self.telemetry.material_radiation += material_radiation.2;
        self.telemetry.room_radiation += room_frame.radiation;
        self.telemetry.room_exchange += room_frame.exchange;
        self.telemetry.boundary_metric_residual += boundary.metric_residual;
        self.telemetry.pump_metric_residual += pump.metric_residual;
        self.telemetry.room_coupling_metric_residual +=
            room_frame.coupling_metric_residual + closed_transport_residual;
        self.telemetry.maximum_suspension_equation_residual = self
            .telemetry
            .maximum_suspension_equation_residual
            .max(suspension_residual.abs());
        self.telemetry.peak_wet = self
            .telemetry
            .peak_wet
            .max(wet_left.abs().max(wet_right.abs()));

        let output_left = left.mul_add(1.0 - controls.mix, wet_left * controls.mix)
            * controls.output_gain;
        let output_right = right.mul_add(1.0 - controls.mix, wet_right * controls.mix)
            * controls.output_gain;
        AriadneTerminalFrame {
            left: host_sample(output_left, &mut self.telemetry.denormal_retirements),
            right: host_sample(output_right, &mut self.telemetry.denormal_retirements),
            wet_left: host_sample(wet_left, &mut self.telemetry.denormal_retirements),
            wet_right: host_sample(wet_right, &mut self.telemetry.denormal_retirements),
            returned_left: host_sample(
                boundary.returned_left,
                &mut self.telemetry.denormal_retirements,
            ),
            returned_right: host_sample(
                boundary.returned_right,
                &mut self.telemetry.denormal_retirements,
            ),
        }
    }

    pub fn process_block(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        controls: AriadneTerminalControls,
    ) {
        for (left_sample, right_sample) in left.iter_mut().zip(right.iter_mut()) {
            let frame = self.process_sample(*left_sample, *right_sample, controls);
            *left_sample = frame.left;
            *right_sample = frame.right;
        }
    }

    fn apply_material_law(&mut self, controls: &AriadneTerminalControls) -> f64 {
        let mut work = 0.0;
        for index in 0..MATERIAL_CELLS {
            let next = material_law(&self.profile, controls, index);
            let cell = &mut self.cells[index];
            work += self.dt * (next.potential(cell.q) - cell.law.potential(cell.q));
            cell.law = next;
        }
        work
    }

    fn recharge_reservoir(&mut self, controls: &AriadneTerminalControls) -> f64 {
        let recovery_seconds = 0.025 + 1.80 * controls.sag * controls.sag;
        let fraction = 1.0 - (-self.dt / recovery_seconds).exp();
        let recharge = (RESERVOIR_CAPACITY - self.reservoir_energy) * fraction;
        self.reservoir_energy += recharge;
        recharge
    }

    fn exchange_boundary(
        &mut self,
        left: f64,
        right: f64,
        controls: &AriadneTerminalControls,
    ) -> BoundaryFrame {
        let mut common = (left + right) * FRAC_1_SQRT_2;
        let mut difference = (left - right) * FRAC_1_SQRT_2;
        let momenta_before = std::array::from_fn::<_, MATERIAL_CELLS, _>(|index| {
            self.cells[index].p
        });
        let (common_row, difference_row) = aperture_rows(
            &self.profile.cell_position,
            &self.profile.input_aperture_weight,
            0.55 * (controls.tone - 0.5),
            0.25 + 0.70 * controls.bloom,
        );
        let base_angle = 0.20 + 0.80 * (0.25 + 0.75 * controls.interlace);
        if controls.chirality >= 0.0 {
            for index in 0..MATERIAL_CELLS {
                let (next_port, next_momentum) = givens(
                    common,
                    self.cells[index].p,
                    base_angle * common_row[index],
                );
                common = next_port;
                self.cells[index].p = next_momentum;
            }
            for index in (0..MATERIAL_CELLS).rev() {
                let (next_port, next_momentum) = givens(
                    difference,
                    self.cells[index].p,
                    base_angle * difference_row[index],
                );
                difference = next_port;
                self.cells[index].p = next_momentum;
            }
        } else {
            for index in (0..MATERIAL_CELLS).rev() {
                let (next_port, next_momentum) = givens(
                    common,
                    self.cells[index].p,
                    base_angle * common_row[index],
                );
                common = next_port;
                self.cells[index].p = next_momentum;
            }
            for index in 0..MATERIAL_CELLS {
                let (next_port, next_momentum) = givens(
                    difference,
                    self.cells[index].p,
                    base_angle * difference_row[index],
                );
                difference = next_port;
                self.cells[index].p = next_momentum;
            }
        }

        let material_change = self.dt
            * 0.5
            * self
                .cells
                .iter()
                .zip(momenta_before)
                .map(|(cell, before)| cell.p * cell.p - before * before)
                .sum::<f64>();
        let incoming = 0.5 * left.mul_add(left, right * right) * self.dt;
        let returned = 0.5 * common.mul_add(common, difference * difference) * self.dt;
        let work = incoming - returned;
        BoundaryFrame {
            returned_left: (common + difference) * FRAC_1_SQRT_2,
            returned_right: (common - difference) * FRAC_1_SQRT_2,
            work,
            metric_residual: material_change - work,
        }
    }

    fn apply_powered_deformation(
        &mut self,
        input_activity: f64,
        controls: &AriadneTerminalControls,
    ) -> PumpFrame {
        if controls.drive == 0.0 || self.reservoir_energy <= 0.0 {
            return PumpFrame::default();
        }
        let mut direction = std::array::from_fn(|index| {
            let q = self.cells[index].q;
            self.profile.pump_aperture_weight[index]
                * (1.0 + 0.35 * controls.bloom * q / (1.0 + q * q).sqrt())
        });
        normalize(&mut direction);
        let projection = self
            .cells
            .iter()
            .zip(direction)
            .map(|(cell, direction)| cell.p * direction)
            .sum::<f64>();
        if projection.abs() < 1.0e-15 {
            return PumpFrame::default();
        }

        let acoustic_energy = self.stored_material_energy();
        let state_activity =
            (2.0 * acoustic_energy / (self.dt * MATERIAL_CELLS as f64)).max(0.0).sqrt();
        let raw_activity = input_activity / (0.03 + 0.50 * controls.headroom)
            + controls.sustain * state_activity;
        let activity = raw_activity / (1.0 + raw_activity);
        let deformation_rate = 80.0 + 3_500.0 * controls.drive * controls.drive;
        let target_multiplier = 1.0 + self.dt * deformation_rate * activity;
        let desired_work = 0.5
            * projection
            * projection
            * self.dt
            * (target_multiplier * target_multiplier - 1.0);
        let accepted_work = desired_work.max(0.0).min(self.reservoir_energy);
        if accepted_work == 0.0 {
            return PumpFrame::default();
        }

        let kinetic_before = self.dt
            * 0.5
            * self.cells.iter().map(|cell| cell.p * cell.p).sum::<f64>();
        let next_projection = projection.signum()
            * (projection * projection + 2.0 * accepted_work / self.dt).sqrt();
        let increment = next_projection - projection;
        for (cell, direction) in self.cells.iter_mut().zip(direction) {
            cell.p += increment * direction;
        }
        self.reservoir_energy -= accepted_work;
        let kinetic_after = self.dt
            * 0.5
            * self.cells.iter().map(|cell| cell.p * cell.p).sum::<f64>();
        PumpFrame {
            work: accepted_work,
            metric_residual: kinetic_after - kinetic_before - accepted_work,
        }
    }

    fn apply_closed_transport(&mut self, controls: &AriadneTerminalControls) -> f64 {
        if controls.interlace == 0.0 || controls.chirality == 0.0 {
            return 0.0;
        }
        let kinetic_before = self.dt
            * 0.5
            * self.cells.iter().map(|cell| cell.p * cell.p).sum::<f64>();
        let transport_hz = 520.0 * controls.interlace * controls.interlace;
        let base_angle = TAU * transport_hz / self.sample_rate;
        for first in (0..MATERIAL_CELLS - 2).step_by(3) {
            let strain = self.cells[first].q.mul_add(
                self.cells[first].q,
                self.cells[first + 1]
                    .q
                    .mul_add(self.cells[first + 1].q, self.cells[first + 2].q.powi(2)),
            );
            let curvature = 1.0 + controls.curvature * strain / (1.0 + strain);
            let angle_a = base_angle * curvature * (0.65 + 0.35 * controls.memory);
            let angle_b = base_angle
                * controls.chirality
                * (0.20 + 0.80 * controls.memory);
            self.apply_momentum_rotation(first + 1, first + 2, -angle_b);
            self.apply_momentum_rotation(first, first + 1, -angle_a);
            self.apply_momentum_rotation(first + 1, first + 2, angle_b);
            self.apply_momentum_rotation(first, first + 1, angle_a);
            self.telemetry.maximum_closed_transport_chord = self
                .telemetry
                .maximum_closed_transport_chord
                .max(order_defect(angle_a, angle_b));
        }
        let kinetic_after = self.dt
            * 0.5
            * self.cells.iter().map(|cell| cell.p * cell.p).sum::<f64>();
        kinetic_after - kinetic_before
    }

    #[inline]
    fn apply_momentum_rotation(&mut self, left: usize, right: usize, angle: f64) {
        let (next_left, next_right) = givens(self.cells[left].p, self.cells[right].p, angle);
        self.cells[left].p = next_left;
        self.cells[right].p = next_right;
    }

    fn propagate_material(&mut self, controls: &AriadneTerminalControls) -> f64 {
        let frequency_scale = 0.72 + 0.65 * controls.tone;
        let mut maximum_residual = 0.0_f64;
        for index in 0..MATERIAL_CELLS {
            let phase_step = (TAU
                * self.profile.cell_frequency_hz[index]
                * frequency_scale
                / self.sample_rate)
                .min(MAX_PHASE_STEP);
            maximum_residual = maximum_residual.max(advance_material_cell(
                &mut self.cells[index],
                phase_step,
            ));
            retire_denormal(&mut self.cells[index].q);
            retire_denormal(&mut self.cells[index].p);
        }
        maximum_residual
    }

    fn update_material_retains(&mut self, controls: &AriadneTerminalControls) {
        let log_thousand = 3.0 * 10.0_f64.ln();
        for index in 0..MATERIAL_CELLS {
            let high_frequency_position = index as f64 / (MATERIAL_CELLS - 1) as f64;
            let t60 = self.profile.cell_t60_seconds[index]
                * (0.65 + 0.70 * controls.sustain)
                * (1.15 - 0.30 * controls.tone * high_frequency_position);
            self.material_retains[index] =
                (-log_thousand / (t60.max(0.01) * self.sample_rate)).exp();
        }
        self.last_tone = controls.tone;
        self.last_sustain = controls.sustain;
        self.material_coefficient_countdown = MATERIAL_COEFFICIENT_PERIOD;
    }

    fn damp_material(&mut self, controls: &AriadneTerminalControls) -> f64 {
        if self.material_coefficient_countdown == 0
            || (controls.tone - self.last_tone).abs() > 0.02
            || (controls.sustain - self.last_sustain).abs() > 0.02
        {
            self.update_material_retains(controls);
        } else {
            self.material_coefficient_countdown -= 1;
        }
        let kinetic_before = self.dt
            * 0.5
            * self.cells.iter().map(|cell| cell.p * cell.p).sum::<f64>();
        for (cell, retain) in self.cells.iter_mut().zip(self.material_retains) {
            cell.p *= retain;
        }
        let kinetic_after = self.dt
            * 0.5
            * self.cells.iter().map(|cell| cell.p * cell.p).sum::<f64>();
        (kinetic_before - kinetic_after).max(0.0)
    }

    fn radiate_material(&mut self, controls: &AriadneTerminalControls) -> (f64, f64, f64) {
        let (radiation_left, radiation_right) = aperture_rows(
            &self.profile.cell_position,
            &self.profile.radiation_aperture_weight,
            0.70 * (controls.tone - 0.5),
            0.20 + 0.55 * controls.bloom,
        );
        let momenta = std::array::from_fn::<_, MATERIAL_CELLS, _>(|index| {
            self.cells[index].p
        });
        let left_projection = dot(&radiation_left, &momenta);
        let right_projection = dot(&radiation_right, &momenta);
        let aperture = 0.15 + 0.85 * controls.bloom;
        let eta = (0.003 + 0.045 * aperture * aperture).clamp(0.0, 0.12);
        let emitted_scale = eta.sqrt();
        let retained_scale = (1.0 - eta).sqrt();
        let removal = 1.0 - retained_scale;
        for index in 0..MATERIAL_CELLS {
            self.cells[index].p -= removal
                * (radiation_left[index] * left_projection
                    + radiation_right[index] * right_projection);
        }
        let emitted_left = emitted_scale * left_projection;
        let emitted_right = emitted_scale * right_projection;
        let radiation = 0.5
            * emitted_left.mul_add(emitted_left, emitted_right * emitted_right)
            * self.dt;
        (emitted_left, emitted_right, radiation)
    }

    fn state_is_finite(&self) -> bool {
        self.cells
            .iter()
            .all(|cell| cell.q.is_finite() && cell.p.is_finite())
            && self.room.stored_energy.is_finite()
    }

    fn reset_after_fault(&mut self) {
        let inputs = self.telemetry.non_finite_inputs;
        let resets = self.telemetry.non_finite_resets.saturating_add(1);
        self.reset();
        self.telemetry.non_finite_inputs = inputs;
        self.telemetry.non_finite_resets = resets;
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct BoundaryFrame {
    returned_left: f64,
    returned_right: f64,
    work: f64,
    metric_residual: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct PumpFrame {
    work: f64,
    metric_residual: f64,
}

fn material_law(
    profile: &AriadneTerminalProfile,
    controls: &AriadneTerminalControls,
    index: usize,
) -> MaterialLaw {
    MaterialLaw {
        curvature: 1.15 * controls.curvature * profile.curvature_weight[index],
        asymmetry: 0.40 * controls.bias * profile.bias_weight[index],
    }
}

/// Average-vector-field update for one positive asymmetric quartic cell.
fn advance_material_cell(cell: &mut MaterialCell, phase_step: f64) -> f64 {
    if phase_step == 0.0 {
        return 0.0;
    }
    let old_q = cell.q;
    let old_p = cell.p;
    let mut next_q = old_q + phase_step * old_p;
    for _ in 0..NEWTON_STEPS {
        let gradient = cell.law.divided_gradient(old_q, next_q);
        let equation = 2.0 * (next_q - old_q) / phase_step - 2.0 * old_p
            + phase_step * gradient;
        let derivative = 2.0 / phase_step
            + phase_step * cell.law.divided_gradient_derivative(old_q, next_q);
        if !derivative.is_finite() || derivative.abs() < 1.0e-14 {
            break;
        }
        let step = (equation / derivative).clamp(-4.0, 4.0);
        next_q -= step;
    }
    let next_p = 2.0 * (next_q - old_q) / phase_step - old_p;
    let residual = 2.0 * (next_q - old_q) / phase_step - 2.0 * old_p
        + phase_step * cell.law.divided_gradient(old_q, next_q);
    cell.q = next_q;
    cell.p = next_p;
    residual.abs()
}

fn aperture_rows<const N: usize>(
    positions: &[f64; N],
    base_weights: &[f64; N],
    center: f64,
    width: f64,
) -> ([f64; N], [f64; N]) {
    let width = width.max(0.08);
    let mut common = std::array::from_fn(|index| {
        let offset = (positions[index] - center) / width;
        let texture = 0.92 + 0.08 * (1.37 * index as f64).cos();
        base_weights[index] * (-offset * offset).exp() * texture
    });
    normalize(&mut common);
    let mut difference = std::array::from_fn(|index| {
        (positions[index] - center) * common[index]
    });
    remove_projection(&common, &mut difference);
    if squared_norm(&difference) < 1.0e-18 {
        difference = std::array::from_fn(|index| {
            if index % 2 == 0 {
                1.0
            } else {
                -1.0
            }
        });
        remove_projection(&common, &mut difference);
    }
    normalize(&mut difference);
    (common, difference)
}

#[inline]
fn quadratic_energy<const N: usize>(state: &[f64; N], dt: f64) -> f64 {
    0.5 * state.iter().map(|value| value * value).sum::<f64>() * dt
}

#[inline]
fn dot<const N: usize>(left: &[f64; N], right: &[f64; N]) -> f64 {
    left.iter().zip(right).map(|(left, right)| left * right).sum()
}

#[inline]
fn squared_norm<const N: usize>(values: &[f64; N]) -> f64 {
    values.iter().map(|value| value * value).sum()
}

fn remove_projection<const N: usize>(basis: &[f64; N], values: &mut [f64; N]) {
    let projection = dot(basis, values);
    for (value, basis_value) in values.iter_mut().zip(basis) {
        *value -= projection * basis_value;
    }
}

fn normalize<const N: usize>(values: &mut [f64; N]) {
    let norm = squared_norm(values).sqrt();
    if norm > 1.0e-15 {
        for value in values {
            *value /= norm;
        }
    }
}

fn validate_finite_array<const N: usize>(
    field: &'static str,
    values: &[f64; N],
) -> Result<(), AriadneTerminalError> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(AriadneTerminalError::InvalidProfile(field))
    }
}

fn validate_positive_array<const N: usize>(
    field: &'static str,
    values: &[f64; N],
) -> Result<(), AriadneTerminalError> {
    if values.iter().all(|value| value.is_finite() && *value > 0.0) {
        Ok(())
    } else {
        Err(AriadneTerminalError::InvalidProfile(field))
    }
}

fn validate_nonnegative_array<const N: usize>(
    field: &'static str,
    values: &[f64; N],
) -> Result<(), AriadneTerminalError> {
    if values
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0)
    {
        Ok(())
    } else {
        Err(AriadneTerminalError::InvalidProfile(field))
    }
}

fn validate_nonzero_norm<const N: usize>(
    field: &'static str,
    values: &[f64; N],
) -> Result<(), AriadneTerminalError> {
    if squared_norm(values) > 1.0e-18 {
        Ok(())
    } else {
        Err(AriadneTerminalError::InvalidProfile(field))
    }
}

#[inline]
fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

#[inline]
fn sanitize_input(value: f32, counter: &mut u64) -> f64 {
    if value.is_finite() {
        value as f64
    } else {
        *counter = counter.saturating_add(1);
        0.0
    }
}

#[inline]
fn retire_denormal(value: &mut f64) {
    if value.abs() < DENORMAL_FLOOR {
        *value = 0.0;
    }
}

#[inline]
fn host_sample(value: f64, counter: &mut u64) -> f32 {
    let sample = value as f32;
    if sample.is_subnormal() {
        *counter = counter.saturating_add(1);
        0.0
    } else {
        sample
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 2.0e-12;

    #[test]
    fn nominal_profile_and_production_rates_are_valid() {
        assert!(AriadneTerminalProfile::default().validate().is_ok());
        for rate in [8_000.0, 44_100.0, 48_000.0, 96_000.0, 192_000.0, 384_000.0] {
            assert!(AriadneTerminal::new(rate).is_ok());
        }
        assert!(AriadneTerminal::new(7_999.0).is_err());
        assert!(AriadneTerminal::new(384_001.0).is_err());
    }

    #[test]
    fn asymmetric_potential_is_positive_and_has_an_exact_linear_limit() {
        for index in -10_000..=10_000 {
            let q = index as f64 / 1000.0;
            let nonlinear = MaterialLaw {
                curvature: 1.4,
                asymmetry: -0.31,
            };
            assert!(nonlinear.potential(q) >= -1.0e-15);
            let linear = MaterialLaw {
                curvature: 0.0,
                asymmetry: 0.9,
            };
            assert_eq!(linear.potential(q), 0.5 * q * q);
        }
    }

    #[test]
    fn discrete_gradient_preserves_nonlinear_cell_energy() {
        let law = MaterialLaw {
            curvature: 1.2,
            asymmetry: -0.28,
        };
        for q in [-2.0, -0.7, 0.0, 0.4, 1.8] {
            for p in [-1.9, -0.3, 0.0, 0.8, 2.1] {
                let mut cell = MaterialCell { q, p, law };
                let before = 0.5 * p * p + law.potential(q);
                let residual = advance_material_cell(&mut cell, 0.37);
                let after = 0.5 * cell.p * cell.p + law.potential(cell.q);
                assert!((after - before).abs() < 2.0e-12, "{q} {p} {residual}");
            }
        }
    }

    #[test]
    fn closed_transport_is_nontrivial_and_zero_work() {
        let mut terminal = AriadneTerminal::new(48_000.0).unwrap();
        terminal.cells[0].p = 1.0;
        let before = terminal.stored_material_energy();
        let controls = AriadneTerminalControls {
            interlace: 1.0,
            chirality: 1.0,
            memory: 1.0,
            ..AriadneTerminalControls::default()
        };
        let residual = terminal.apply_closed_transport(&controls);
        let after = terminal.stored_material_energy();
        assert!(residual.abs() < 1.0e-15);
        assert!((after - before).abs() < 1.0e-15);
        assert!(terminal.cells[1].p.abs() + terminal.cells[2].p.abs() > 1.0e-8);
        assert!(terminal.telemetry.maximum_closed_transport_chord > 0.0);
    }

    #[test]
    fn rank_one_positive_deformation_spends_reservoir_exactly() {
        let mut terminal = AriadneTerminal::new(48_000.0).unwrap();
        terminal.cells[0].p = 0.8;
        terminal.cells[1].p = -0.3;
        let reservoir_before = terminal.reservoir_energy;
        let material_before = terminal.stored_material_energy();
        let pump = terminal.apply_powered_deformation(
            0.8,
            &AriadneTerminalControls {
                drive: 1.5,
                sustain: 1.0,
                ..AriadneTerminalControls::default()
            },
        );
        let material_after = terminal.stored_material_energy();
        assert!(pump.work > 0.0);
        assert!((material_after - material_before - pump.work).abs() < 2.0e-15);
        assert!((reservoir_before - terminal.reservoir_energy - pump.work).abs() < 2.0e-15);
        assert!(pump.metric_residual.abs() < 2.0e-15);
    }

    #[test]
    fn silence_is_an_exact_acoustic_fixed_point() {
        let mut terminal = AriadneTerminal::new(48_000.0).unwrap();
        for _ in 0..16_384 {
            let frame = terminal.process_sample(
                0.0,
                0.0,
                AriadneTerminalControls::default(),
            );
            assert_eq!(frame, AriadneTerminalFrame::default());
        }
        let receipt = terminal.receipt();
        assert_eq!(receipt.boundary_work, 0.0);
        assert_eq!(receipt.external_supply_work, 0.0);
        assert_eq!(receipt.reservoir_to_material_work, 0.0);
        assert_eq!(receipt.stored_material_energy, 0.0);
        assert_eq!(receipt.stored_room_energy, 0.0);
        assert!(receipt.global_closure_residual.abs() < 1.0e-20);
    }

    #[test]
    fn driven_world_closes_one_global_ledger() {
        let mut terminal = AriadneTerminal::new(48_000.0).unwrap();
        let controls = AriadneTerminalControls {
            drive: 1.1,
            curvature: 0.82,
            bias: 0.36,
            sustain: 0.72,
            interlace: 0.78,
            chirality: -0.61,
            memory: 0.74,
            feedback: 0.58,
            space: 0.67,
            room_decay: 0.76,
            ..AriadneTerminalControls::default()
        };
        for frame_index in 0..24_000 {
            let time = frame_index as f64 / 48_000.0;
            let sample = (0.18 * (TAU * 110.0 * time).sin()
                + 0.05 * (TAU * 777.0 * time).sin()) as f32;
            let frame = terminal.process_sample(sample, sample, controls);
            assert!(frame.left.is_finite() && frame.right.is_finite());
        }
        let receipt = terminal.receipt();
        assert!(receipt.boundary_work.abs() > 0.0);
        assert!(receipt.reservoir_to_material_work > 0.0);
        assert!(receipt.material_loss > 0.0);
        assert!(receipt.material_radiation > 0.0);
        assert!(receipt.room_exchange.abs() > 0.0);
        assert!(receipt.maximum_closed_transport_chord > 0.0);
        assert!(receipt.maximum_abs_closure_residual < TOLERANCE, "{receipt:?}");
        assert!(receipt.global_closure_residual.abs() < 2.0e-11, "{receipt:?}");
        assert_eq!(receipt.non_finite_resets, 0);
    }

    #[test]
    fn zero_space_is_an_exact_room_bridge_limit() {
        let mut terminal = AriadneTerminal::new(48_000.0).unwrap();
        let controls = AriadneTerminalControls {
            drive: 0.0,
            space: 0.0,
            feedback: 1.0,
            ..AriadneTerminalControls::default()
        };
        for index in 0..8_192 {
            let sample = if index == 0 { 0.5 } else { 0.0 };
            let _ = terminal.process_sample(sample, sample, controls);
        }
        let receipt = terminal.receipt();
        assert!(receipt.room_exchange.abs() < 1.0e-18, "{receipt:?}");
        assert!(receipt.stored_room_energy.abs() < 1.0e-18, "{receipt:?}");
    }

    #[test]
    fn passive_tail_cannot_grow_without_boundary_or_supply_work() {
        let mut terminal = AriadneTerminal::new(48_000.0).unwrap();
        let excitation = AriadneTerminalControls {
            drive: 0.0,
            space: 0.6,
            feedback: 0.5,
            ..AriadneTerminalControls::default()
        };
        for index in 0..256 {
            let sample = if index < 8 { 0.4 } else { 0.0 };
            let _ = terminal.process_sample(sample, sample, excitation);
        }
        let mut previous = terminal.total_stored_energy();
        for _ in 0..16_384 {
            let before_receipt = terminal.receipt();
            let _ = terminal.process_sample(0.0, 0.0, excitation);
            let after = terminal.total_stored_energy();
            let after_receipt = terminal.receipt();
            let supplied = (after_receipt.external_supply_work
                - before_receipt.external_supply_work)
                + (after_receipt.boundary_work - before_receipt.boundary_work);
            assert!(after <= previous + supplied + 2.0e-13);
            previous = after;
        }
    }

    #[test]
    fn changing_material_law_books_control_work() {
        let mut terminal = AriadneTerminal::new(48_000.0).unwrap();
        terminal.cells[0].q = 0.7;
        terminal.cells[3].q = -0.4;
        let before = terminal.stored_material_energy();
        let work = terminal.apply_material_law(&AriadneTerminalControls {
            curvature: 1.0,
            bias: -0.7,
            ..AriadneTerminalControls::default()
        });
        let after = terminal.stored_material_energy();
        assert!((after - before - work).abs() < 2.0e-15);
        assert!(work.abs() > 1.0e-12);
    }

    #[test]
    fn zero_mix_is_bit_exact_dry() {
        let mut terminal = AriadneTerminal::new(48_000.0).unwrap();
        let controls = AriadneTerminalControls {
            mix: 0.0,
            output_gain: 1.0,
            ..AriadneTerminalControls::default()
        };
        for index in 0..8_192 {
            let left = ((index as f64 * 0.019).sin() * 0.7) as f32;
            let right = ((index as f64 * 0.023).cos() * 0.5) as f32;
            let frame = terminal.process_sample(left, right, controls);
            assert_eq!(frame.left.to_bits(), left.to_bits());
            assert_eq!(frame.right.to_bits(), right.to_bits());
        }
    }

    #[test]
    fn block_segmentation_does_not_change_state_or_audio() {
        let controls = AriadneTerminalControls::default();
        let mut sample_engine = AriadneTerminal::new(48_000.0).unwrap();
        let mut block_engine = AriadneTerminal::new(48_000.0).unwrap();
        let mut left = [0.0_f32; 4_096];
        let mut right = [0.0_f32; 4_096];
        for index in 0..left.len() {
            left[index] = ((index as f64 * 0.017).sin() * 0.2) as f32;
            right[index] = ((index as f64 * 0.013).cos() * 0.15) as f32;
        }
        let mut expected_left = left;
        let mut expected_right = right;
        for index in 0..expected_left.len() {
            let frame = sample_engine.process_sample(
                expected_left[index],
                expected_right[index],
                controls,
            );
            expected_left[index] = frame.left;
            expected_right[index] = frame.right;
        }
        block_engine.process_block(&mut left, &mut right, controls);
        assert_eq!(left, expected_left);
        assert_eq!(right, expected_right);
        assert_eq!(block_engine.receipt(), sample_engine.receipt());
    }

    #[test]
    fn non_finite_input_is_retired_without_poisoning_state() {
        let mut terminal = AriadneTerminal::new(48_000.0).unwrap();
        let frame = terminal.process_sample(
            f32::NAN,
            f32::INFINITY,
            AriadneTerminalControls::default(),
        );
        assert!(frame.left.is_finite() && frame.right.is_finite());
        assert_eq!(terminal.receipt().non_finite_inputs, 2);
        assert_eq!(terminal.receipt().non_finite_resets, 0);
    }
}
