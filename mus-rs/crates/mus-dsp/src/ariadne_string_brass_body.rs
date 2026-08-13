//! Maximal same-body string/brass runtime.
//!
//! One persistent state graph owns the morphing wave field, one compliant
//! source/contact boundary, one thresholded active inclusion, one ordered
//! internal-resonance transport, one reciprocal room halo, and finite-aperture
//! radiation. Guitar, driven electric guitar, trombone, and intermediate
//! states are constitutive/gesture regions of this graph; no renderer changes.

use crate::ariadne_active_inclusion::{
    ActiveInclusionControls, ActiveInclusionFrame, SynchronizedActiveInclusion,
};
use crate::ariadne_compliant_aperture::{
    ApertureConfigurationWorkFrame, CompliantApertureBoundary, CompliantApertureConfiguration,
    CompliantApertureFrame,
};
use crate::ariadne_morphing_field::{
    ConfigurationWorkFrame, MorphingWaveField, WaveFieldConfiguration, WaveFieldState,
};
use crate::ariadne_resonant_transport::{
    InternalResonanceTransport, ResonantTransportControls, ResonantTransportFrame,
};
use crate::ariadne_room_halo::{ReciprocalRoomHalo, RoomHaloControls, RoomHaloFrame};
use crate::ariadne_string_brass_profiles::{
    guitar_radiation_direction, guitar_source_direction, trombone_radiation_direction,
    trombone_source_direction, SharedBodyConfiguration, SharedBodyRegime, STRING_BRASS_MODES,
};

const ROOM_COURSES: usize = 8;
const TRANSPORT_PAIRS: usize = 12;
const RESERVOIR_CAPACITY: f64 = 0.002;
const ROOM_DELAYS: [f64; ROOM_COURSES] = [
    0.0073, 0.0111, 0.0137, 0.0179, 0.0233, 0.0297, 0.0521, 0.0899,
];
const PAIRS: [(usize, usize); TRANSPORT_PAIRS] = [
    (0, 2), (1, 3), (2, 5), (3, 7), (4, 9), (5, 11),
    (6, 13), (7, 15), (8, 17), (9, 19), (10, 21), (11, 23),
];
const PAIR_WEIGHTS: [f64; TRANSPORT_PAIRS] = [
    1.0, 0.92, 0.84, 0.77, 0.70, 0.64, 0.59, 0.54, 0.49, 0.44, 0.40, 0.36,
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SharedBodyPlayControls {
    pub source_wave: f64,
    pub active_coupling: f64,
    pub active: ActiveInclusionControls,
    pub transport: ResonantTransportControls,
    pub room: RoomHaloControls,
    pub field_radiation_fraction: f64,
    pub supply_rate_per_second: f64,
    pub observation_scale: f64,
}

impl SharedBodyPlayControls {
    #[must_use]
    pub fn clean_guitar(source_wave: f64) -> Self {
        let mut active = ActiveInclusionControls::default();
        active.drive = 0.0;
        Self {
            source_wave,
            active_coupling: 0.0,
            active,
            transport: ResonantTransportControls { interlace: 0.18, memory: 0.24 },
            room: RoomHaloControls { coupling: 0.10, decay: 0.38, radiation: 0.12 },
            field_radiation_fraction: 0.004,
            supply_rate_per_second: 0.0,
            observation_scale: 6.0,
        }
    }

    #[must_use]
    pub fn driven_guitar(source_wave: f64) -> Self {
        Self {
            source_wave,
            active_coupling: 1.0,
            active: ActiveInclusionControls {
                drive: 1.05,
                headroom: 0.28,
                sustain: 0.78,
                sag: 0.68,
                bloom: 0.12,
                feedback: 0.10,
                space: 0.24,
            },
            transport: ResonantTransportControls { interlace: 0.58, memory: 0.68 },
            room: RoomHaloControls { coupling: 0.24, decay: 0.52, radiation: 0.22 },
            field_radiation_fraction: 0.006,
            supply_rate_per_second: 3.2,
            observation_scale: 6.0,
        }
    }

    #[must_use]
    pub fn trombone(source_wave: f64) -> Self {
        let mut active = ActiveInclusionControls::default();
        active.drive = 0.0;
        active.sustain = 0.0;
        active.feedback = 0.0;
        active.space = 0.0;
        Self {
            source_wave,
            active_coupling: 0.0,
            active,
            transport: ResonantTransportControls { interlace: 0.72, memory: 0.74 },
            room: RoomHaloControls { coupling: 0.30, decay: 0.58, radiation: 0.30 },
            field_radiation_fraction: 0.009,
            supply_rate_per_second: 0.0,
            observation_scale: 6.0,
        }
    }

    fn sanitized(self) -> Self {
        Self {
            source_wave: finite(self.source_wave),
            active_coupling: finite(self.active_coupling).clamp(0.0, 1.0),
            active: self.active.sanitized(),
            transport: self.transport.sanitized(),
            room: RoomHaloControls {
                coupling: finite(self.room.coupling).clamp(0.0, 1.0),
                decay: finite(self.room.decay).clamp(0.0, 1.0),
                radiation: finite(self.room.radiation).clamp(0.0, 1.0),
            },
            field_radiation_fraction: finite(self.field_radiation_fraction).clamp(0.0, 1.0),
            supply_rate_per_second: finite(self.supply_rate_per_second).max(0.0),
            observation_scale: finite(self.observation_scale).clamp(0.01, 64.0),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SharedBodyFrame {
    pub left: f64,
    pub right: f64,
    pub body: f64,
    pub room_left: f64,
    pub room_right: f64,
    pub aperture: CompliantApertureFrame,
    pub active: ActiveInclusionFrame,
    pub transport: ResonantTransportFrame,
    pub room: RoomHaloFrame,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SharedBodyReconfigurationFrame {
    pub field: ConfigurationWorkFrame,
    pub aperture: ApertureConfigurationWorkFrame,
    pub total_control_work: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SharedBodyReceipt {
    pub frames: u64,
    pub source_work: f64,
    pub external_supply_work: f64,
    pub configuration_work: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    pub active_reservoir_to_material_work: f64,
    pub room_exchange: f64,
    pub initial_stored_energy: f64,
    pub current_stored_energy: f64,
    pub global_closure_residual: f64,
    pub maximum_local_residual: f64,
    pub reservoir_energy: f64,
    pub active_activation: f64,
    pub brassness: f64,
}

#[derive(Debug, Default)]
struct Telemetry {
    frames: u64,
    source_work: f64,
    supply_work: f64,
    configuration_work: f64,
    internal_loss: f64,
    radiation: f64,
    active_transfer: f64,
    room_exchange: f64,
    maximum_local_residual: f64,
}

/// One persistent Ariadne body spanning string/contact and brass/lip regimes.
pub struct AriadneStringBrassBody {
    sample_rate: f64,
    dt: f64,
    field: MorphingWaveField<STRING_BRASS_MODES>,
    aperture: CompliantApertureBoundary<STRING_BRASS_MODES>,
    active: SynchronizedActiveInclusion<STRING_BRASS_MODES>,
    transport: InternalResonanceTransport<STRING_BRASS_MODES, TRANSPORT_PAIRS>,
    room: ReciprocalRoomHalo<STRING_BRASS_MODES, ROOM_COURSES>,
    radiation_direction: [f64; STRING_BRASS_MODES],
    reservoir: f64,
    brassness: f64,
    initial_energy: f64,
    telemetry: Telemetry,
}

impl AriadneStringBrassBody {
    pub fn try_new(sample_rate: f64, reference_frequency_hz: f64) -> Result<Self, String> {
        let guitar = SharedBodyConfiguration::for_regime(SharedBodyRegime::ElectricGuitarClean);
        let source_direction = shared_source_direction();
        let field = MorphingWaveField::try_new(sample_rate, reference_frequency_hz, guitar.field)
            .map_err(|error| error.to_string())?;
        let aperture = CompliantApertureBoundary::try_new(sample_rate, guitar.boundary, source_direction)
            .map_err(|error| error.to_string())?;
        let active = SynchronizedActiveInclusion::try_new(sample_rate, active_directions())
            .map_err(|error| error.to_string())?;
        let transport = InternalResonanceTransport::try_new(sample_rate, PAIRS, PAIR_WEIGHTS)
            .map_err(|error| error.to_string())?;
        let room = ReciprocalRoomHalo::try_new(
            sample_rate,
            ROOM_DELAYS,
            room_body_directions(),
            [1.0, 0.91, 0.82, 0.74, 0.67, 0.60, 0.54, 0.48],
            [1.0, -0.73, 0.88, -0.61, 0.79, -0.52, 0.69, -0.44],
            [1.0, 0.62, 0.18, -0.31, -0.70, -0.54, 0.08, 0.48],
            [0.17, -0.51, -0.72, -0.24, 0.36, 0.61, 0.31, -0.22],
        ).map_err(str::to_owned)?;
        let mut result = Self {
            sample_rate,
            dt: sample_rate.recip(),
            field,
            aperture,
            active,
            transport,
            room,
            radiation_direction: guitar.radiation_direction,
            reservoir: RESERVOIR_CAPACITY,
            brassness: 0.0,
            initial_energy: 0.0,
            telemetry: Telemetry::default(),
        };
        result.initial_energy = result.total_stored_energy();
        Ok(result)
    }

    #[must_use]
    pub fn sample_rate(&self) -> f64 { self.sample_rate }

    #[must_use]
    pub fn brassness(&self) -> f64 { self.brassness }

    #[must_use]
    pub fn field_state(&self) -> WaveFieldState<STRING_BRASS_MODES> { self.field.state() }

    #[must_use]
    pub fn aperture_state(&self) -> crate::ariadne_compliant_aperture::CompliantApertureState {
        self.aperture.state()
    }

    #[must_use]
    pub fn total_stored_energy(&self) -> f64 {
        self.field.stored_energy() + self.aperture.stored_energy() + self.room.stored_energy() + self.reservoir
    }

    /// Move continuously between the measured guitar and trombone constitutive
    /// regions while preserving every live state coordinate. The exact metric
    /// and aperture-potential difference is booked as control work.
    pub fn set_brassness(&mut self, brassness: f64) -> Result<SharedBodyReconfigurationFrame, String> {
        let amount = finite(brassness).clamp(0.0, 1.0);
        let guitar = SharedBodyConfiguration::for_regime(SharedBodyRegime::ElectricGuitarClean);
        let brass = SharedBodyConfiguration::for_regime(SharedBodyRegime::TromboneSustain);
        let field_configuration = interpolate_field(guitar.field, brass.field, amount);
        let aperture_configuration = interpolate_aperture(guitar.boundary, brass.boundary, amount);
        let field = self.field.set_configuration(field_configuration).map_err(|error| error.to_string())?;
        let aperture = self.aperture.set_configuration(aperture_configuration).map_err(|error| error.to_string())?;
        self.radiation_direction = interpolate_direction(
            guitar_radiation_direction(),
            trombone_radiation_direction(),
            amount,
        );
        self.brassness = amount;
        let total_control_work = field.control_work + aperture.control_work;
        self.telemetry.configuration_work += total_control_work;
        Ok(SharedBodyReconfigurationFrame { field, aperture, total_control_work })
    }

    pub fn process_sample(&mut self, controls: SharedBodyPlayControls) -> SharedBodyFrame {
        let controls = controls.sanitized();
        let supply_fraction = 1.0 - (-controls.supply_rate_per_second * self.dt).exp();
        let supply_work = (RESERVOIR_CAPACITY - self.reservoir).max(0.0) * supply_fraction;
        self.reservoir += supply_work;

        let field_effort = self.field.aperture_effort(self.aperture.direction()).unwrap_or(0.0);
        let aperture = self.aperture.process(
            self.field.momentum_mut(),
            field_effort,
            controls.source_wave,
        );
        let boundary_activity = controls.source_wave.abs()
            + aperture.field_flow_after.abs()
            + 0.5 * self.aperture.state().momentum.abs();
        let active = if controls.active_coupling > 0.0 {
            let mut active_controls = controls.active;
            active_controls.drive *= controls.active_coupling;
            self.active.process(
                self.field.momentum_mut(),
                controls.active_coupling * boundary_activity,
                self.room.activity(),
                active_controls,
                &mut self.reservoir,
            )
        } else {
            ActiveInclusionFrame::default()
        };

        let displacement = *self.field.displacement();
        let transport = self.transport.process(
            &displacement,
            self.field.momentum_mut(),
            controls.transport,
        );
        let propagation = self.field.propagate();
        let field_loss = self.field.damp();
        let room = self.room.process(self.field.momentum_mut(), controls.room);
        let body_radiation = self.field.radiate(
            &self.radiation_direction,
            controls.field_radiation_fraction,
        ).expect("stored radiation direction is normalized");

        let internal_loss = aperture.internal_loss
            + active.dissipated_work
            + field_loss.internal_loss
            + room.internal_loss;
        let radiation = body_radiation.radiated_energy + room.radiation;
        let source_work = aperture.source_boundary_work + aperture.active_source_work;
        self.telemetry.frames += 1;
        self.telemetry.source_work += source_work;
        self.telemetry.supply_work += supply_work;
        self.telemetry.internal_loss += internal_loss;
        self.telemetry.radiation += radiation;
        self.telemetry.active_transfer += active.accepted_positive_work;
        self.telemetry.room_exchange += room.exchange;
        for residual in [
            aperture.closure_residual,
            active.closure_residual,
            transport.metric_residual,
            propagation.metric_residual,
            field_loss.closure_residual,
            room.coupling_metric_residual,
            room.room_closure_residual,
            body_radiation.closure_residual,
        ] {
            self.telemetry.maximum_local_residual = self.telemetry.maximum_local_residual.max(residual.abs());
        }

        let body = body_radiation.emitted_coordinate;
        let left = controls.observation_scale * (body + room.emitted_left);
        let right = controls.observation_scale * (body + room.emitted_right);
        SharedBodyFrame {
            left,
            right,
            body,
            room_left: room.emitted_left,
            room_right: room.emitted_right,
            aperture,
            active,
            transport,
            room,
        }
    }

    #[must_use]
    pub fn receipt(&self) -> SharedBodyReceipt {
        let current = self.total_stored_energy();
        let global = current
            - self.initial_energy
            - self.telemetry.source_work
            - self.telemetry.supply_work
            - self.telemetry.configuration_work
            + self.telemetry.internal_loss
            + self.telemetry.radiation;
        SharedBodyReceipt {
            frames: self.telemetry.frames,
            source_work: self.telemetry.source_work,
            external_supply_work: self.telemetry.supply_work,
            configuration_work: self.telemetry.configuration_work,
            internal_loss: self.telemetry.internal_loss,
            radiation: self.telemetry.radiation,
            active_reservoir_to_material_work: self.telemetry.active_transfer,
            room_exchange: self.telemetry.room_exchange,
            initial_stored_energy: self.initial_energy,
            current_stored_energy: current,
            global_closure_residual: global,
            maximum_local_residual: self.telemetry.maximum_local_residual,
            reservoir_energy: self.reservoir,
            active_activation: self.active.activation(),
            brassness: self.brassness,
        }
    }
}

fn shared_source_direction() -> [f64; STRING_BRASS_MODES] {
    interpolate_direction(guitar_source_direction(), trombone_source_direction(), 0.5)
}

fn interpolate_direction(
    left: [f64; STRING_BRASS_MODES],
    right: [f64; STRING_BRASS_MODES],
    amount: f64,
) -> [f64; STRING_BRASS_MODES] {
    let mut result = std::array::from_fn(|i| (1.0 - amount) * left[i] + amount * right[i]);
    let norm = result.iter().map(|value| value * value).sum::<f64>().sqrt();
    for value in &mut result { *value /= norm; }
    result
}

fn interpolate_field(left: WaveFieldConfiguration, right: WaveFieldConfiguration, amount: f64) -> WaveFieldConfiguration {
    WaveFieldConfiguration {
        length_ratio: lerp(left.length_ratio, right.length_ratio, amount),
        boundary_phase: lerp(left.boundary_phase, right.boundary_phase, amount),
        dispersion: lerp(left.dispersion, right.dispersion, amount),
        low_mode_t60_seconds: lerp(left.low_mode_t60_seconds, right.low_mode_t60_seconds, amount),
        high_mode_t60_seconds: lerp(left.high_mode_t60_seconds, right.high_mode_t60_seconds, amount),
    }
}

fn interpolate_aperture(
    left: CompliantApertureConfiguration,
    right: CompliantApertureConfiguration,
    amount: f64,
) -> CompliantApertureConfiguration {
    CompliantApertureConfiguration {
        natural_frequency_hz: lerp(left.natural_frequency_hz, right.natural_frequency_hz, amount),
        quartic_stiffness: lerp(left.quartic_stiffness, right.quartic_stiffness, amount),
        asymmetry: lerp(left.asymmetry, right.asymmetry, amount),
        equilibrium_displacement: lerp(left.equilibrium_displacement, right.equilibrium_displacement, amount),
        opening_bias: lerp(left.opening_bias, right.opening_bias, amount),
        damping_per_second: lerp(left.damping_per_second, right.damping_per_second, amount),
        source_to_aperture_rate_hz: lerp(left.source_to_aperture_rate_hz, right.source_to_aperture_rate_hz, amount),
        source_to_field_rate_hz: lerp(left.source_to_field_rate_hz, right.source_to_field_rate_hz, amount),
        sweep_rate_hz: lerp(left.sweep_rate_hz, right.sweep_rate_hz, amount),
        active_rate_per_second: lerp(left.active_rate_per_second, right.active_rate_per_second, amount),
        active_target_momentum: lerp(left.active_target_momentum, right.active_target_momentum, amount),
    }
}

fn active_directions() -> [[f64; STRING_BRASS_MODES]; 3] {
    std::array::from_fn(|rank| {
        std::array::from_fn(|index| {
            let x = index as f64 / (STRING_BRASS_MODES - 1) as f64;
            match rank {
                0 => 0.35 + 1.65 * x,
                1 => if index % 2 == 0 { 1.0 - 0.35 * x } else { -0.82 + 0.25 * x },
                _ => (std::f64::consts::PI * (2.0 * x + 0.15)).sin(),
            }
        })
    })
}

fn room_body_directions() -> [[f64; STRING_BRASS_MODES]; ROOM_COURSES] {
    std::array::from_fn(|course| {
        let mut direction = [0.0; STRING_BRASS_MODES];
        let first = (course * 3 + 1) % STRING_BRASS_MODES;
        let second = (first + 5) % STRING_BRASS_MODES;
        direction[first] = 1.0;
        direction[second] = if course % 2 == 0 { 0.35 } else { -0.35 };
        direction
    })
}

fn lerp(left: f64, right: f64, amount: f64) -> f64 { left + amount * (right - left) }
fn finite(value: f64) -> f64 { if value.is_finite() { value } else { 0.0 } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morph_changes_configuration_without_resetting_state() {
        let mut body = AriadneStringBrassBody::try_new(96_000.0, 116.5).unwrap();
        for _ in 0..512 {
            body.process_sample(SharedBodyPlayControls::clean_guitar(0.7));
        }
        let field_before = body.field_state();
        let aperture_before = body.aperture_state();
        let frame = body.set_brassness(1.0).unwrap();
        assert_eq!(body.field_state(), field_before);
        assert_eq!(body.aperture_state(), aperture_before);
        assert_eq!(frame.field.state_displacement, 0.0);
        assert_eq!(frame.aperture.state_displacement, 0.0);
    }

    #[test]
    fn driven_guitar_can_draw_from_reservoir_but_trombone_cannot() {
        let mut guitar = AriadneStringBrassBody::try_new(96_000.0, 116.5).unwrap();
        for _ in 0..8_000 {
            guitar.process_sample(SharedBodyPlayControls::driven_guitar(0.9));
        }
        assert!(guitar.receipt().active_reservoir_to_material_work > 0.0);

        let mut brass = AriadneStringBrassBody::try_new(96_000.0, 116.5).unwrap();
        brass.set_brassness(1.0).unwrap();
        for _ in 0..8_000 {
            brass.process_sample(SharedBodyPlayControls::trombone(0.30));
        }
        assert_eq!(brass.receipt().active_reservoir_to_material_work, 0.0);
    }

    #[test]
    fn mixed_regime_stays_finite_and_locally_balanced() {
        let mut body = AriadneStringBrassBody::try_new(96_000.0, 116.5).unwrap();
        for frame in 0..12_000 {
            let amount = frame as f64 / 11_999.0;
            if frame % 32 == 0 { body.set_brassness(amount).unwrap(); }
            let source = 0.25 + 0.45 * (0.017 * frame as f64).sin().abs();
            let controls = if amount < 0.5 {
                SharedBodyPlayControls::driven_guitar(source)
            } else {
                SharedBodyPlayControls::trombone(source)
            };
            let output = body.process_sample(controls);
            assert!(output.left.is_finite() && output.right.is_finite());
        }
        let receipt = body.receipt();
        assert!(receipt.current_stored_energy.is_finite());
        assert!(receipt.maximum_local_residual < 1.0e-8);
    }
}