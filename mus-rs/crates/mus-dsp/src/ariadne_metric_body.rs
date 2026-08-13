//! V2 same-body runtime: six string courses and one brass bore are metrics on
//! exactly the same 24 persistent coordinates.
//!
//! This keeps the V1 one-ladder composite intact as an ablation. V2 replaces
//! only its field metric; aperture, active matter, resonant transport, room,
//! reservoir, and radiation remain one shared physical graph.

use crate::ariadne_active_inclusion::{
    ActiveInclusionControls, SynchronizedActiveInclusion,
};
use crate::ariadne_compliant_aperture::{
    CompliantApertureBoundary, CompliantApertureConfiguration,
};
use crate::ariadne_metric_field::{
    interpolate_metric, six_course_guitar_metric, trombone_bore_metric,
    MetricFieldState, ModalMetricWaveField,
};
use crate::ariadne_resonant_transport::{
    InternalResonanceTransport, ResonantTransportControls,
};
use crate::ariadne_room_halo::{ReciprocalRoomHalo, RoomHaloControls};
use crate::ariadne_string_brass_profiles::{
    guitar_radiation_direction, guitar_source_direction, trombone_radiation_direction,
    trombone_source_direction, SharedBodyConfiguration, SharedBodyRegime, STRING_BRASS_MODES,
};

const ROOM_COURSES: usize = 8;
const PAIR_COUNT: usize = 12;
const CAPACITY: f64 = 0.002;
const DELAYS: [f64; ROOM_COURSES] = [
    0.0073, 0.0111, 0.0137, 0.0179, 0.0233, 0.0297, 0.0521, 0.0899,
];
const PAIRS: [(usize, usize); PAIR_COUNT] = [
    (0, 2), (1, 3), (2, 5), (3, 7), (4, 9), (5, 11),
    (6, 13), (7, 15), (8, 17), (9, 19), (10, 21), (11, 23),
];
const WEIGHTS: [f64; PAIR_COUNT] = [
    1.0, 0.92, 0.84, 0.77, 0.70, 0.64, 0.59, 0.54, 0.49, 0.44, 0.40, 0.36,
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricBodyControls {
    pub source_wave: f64,
    pub electric_drive: f64,
    pub supply_rate_per_second: f64,
    pub transport: ResonantTransportControls,
    pub room: RoomHaloControls,
    pub radiation_fraction: f64,
}

impl MetricBodyControls {
    #[must_use]
    pub fn clean_guitar(source_wave: f64) -> Self {
        Self {
            source_wave,
            electric_drive: 0.0,
            supply_rate_per_second: 0.0,
            transport: ResonantTransportControls { interlace: 0.18, memory: 0.24 },
            room: RoomHaloControls { coupling: 0.10, decay: 0.38, radiation: 0.12 },
            radiation_fraction: 0.004,
        }
    }

    #[must_use]
    pub fn driven_guitar(source_wave: f64) -> Self {
        Self {
            source_wave,
            electric_drive: 1.0,
            supply_rate_per_second: 3.2,
            transport: ResonantTransportControls { interlace: 0.58, memory: 0.68 },
            room: RoomHaloControls { coupling: 0.24, decay: 0.52, radiation: 0.22 },
            radiation_fraction: 0.006,
        }
    }

    #[must_use]
    pub fn trombone(source_wave: f64) -> Self {
        Self {
            source_wave,
            electric_drive: 0.0,
            supply_rate_per_second: 0.0,
            transport: ResonantTransportControls { interlace: 0.72, memory: 0.74 },
            room: RoomHaloControls { coupling: 0.30, decay: 0.58, radiation: 0.30 },
            radiation_fraction: 0.009,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MetricBodyFrame {
    pub left: f64,
    pub right: f64,
    pub source_work: f64,
    pub active_transfer: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    pub maximum_local_residual: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MetricBodyReceipt {
    pub frames: u64,
    pub source_work: f64,
    pub supply_work: f64,
    pub configuration_work: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    pub active_transfer: f64,
    pub stored_energy: f64,
    pub reservoir: f64,
    pub brassness: f64,
    pub maximum_local_residual: f64,
}

pub struct AriadneMetricBody {
    dt: f64,
    field: ModalMetricWaveField<STRING_BRASS_MODES>,
    aperture: CompliantApertureBoundary<STRING_BRASS_MODES>,
    active: SynchronizedActiveInclusion<STRING_BRASS_MODES>,
    transport: InternalResonanceTransport<STRING_BRASS_MODES, PAIR_COUNT>,
    room: ReciprocalRoomHalo<STRING_BRASS_MODES, ROOM_COURSES>,
    radiation: [f64; STRING_BRASS_MODES],
    reservoir: f64,
    brassness: f64,
    receipt: MetricBodyReceipt,
}

impl AriadneMetricBody {
    pub fn try_new(sample_rate: f64) -> Result<Self, String> {
        let guitar = SharedBodyConfiguration::for_regime(SharedBodyRegime::ElectricGuitarClean);
        let field = ModalMetricWaveField::try_new(sample_rate, six_course_guitar_metric())
            .map_err(str::to_owned)?;
        let aperture = CompliantApertureBoundary::try_new(
            sample_rate,
            guitar.boundary,
            shared_source_direction(),
        ).map_err(|error| error.to_string())?;
        let active = SynchronizedActiveInclusion::try_new(sample_rate, active_directions())
            .map_err(|error| error.to_string())?;
        let transport = InternalResonanceTransport::try_new(sample_rate, PAIRS, WEIGHTS)
            .map_err(|error| error.to_string())?;
        let room = ReciprocalRoomHalo::try_new(
            sample_rate,
            DELAYS,
            room_directions(),
            [1.0, 0.91, 0.82, 0.74, 0.67, 0.60, 0.54, 0.48],
            [1.0, -0.73, 0.88, -0.61, 0.79, -0.52, 0.69, -0.44],
            [1.0, 0.62, 0.18, -0.31, -0.70, -0.54, 0.08, 0.48],
            [0.17, -0.51, -0.72, -0.24, 0.36, 0.61, 0.31, -0.22],
        ).map_err(str::to_owned)?;
        Ok(Self {
            dt: sample_rate.recip(), field, aperture, active, transport, room,
            radiation: guitar.radiation_direction,
            reservoir: CAPACITY, brassness: 0.0,
            receipt: MetricBodyReceipt { reservoir: CAPACITY, ..MetricBodyReceipt::default() },
        })
    }

    #[must_use]
    pub fn state(&self) -> MetricFieldState<STRING_BRASS_MODES> { self.field.state() }

    #[must_use]
    pub fn frequencies_hz(&self) -> [f64; STRING_BRASS_MODES] { self.field.frequencies_hz() }

    #[must_use]
    pub fn brassness(&self) -> f64 { self.brassness }

    pub fn set_brassness(&mut self, amount: f64) -> Result<f64, String> {
        let amount = finite(amount).clamp(0.0, 1.0);
        let guitar = SharedBodyConfiguration::for_regime(SharedBodyRegime::ElectricGuitarClean);
        let brass = SharedBodyConfiguration::for_regime(SharedBodyRegime::TromboneSustain);
        let field = self.field.set_configuration(interpolate_metric(
            six_course_guitar_metric(), trombone_bore_metric(), amount,
        )).map_err(str::to_owned)?;
        let aperture = self.aperture.set_configuration(interpolate_aperture(
            guitar.boundary, brass.boundary, amount,
        )).map_err(|error| error.to_string())?;
        self.radiation = interpolate_direction(
            guitar_radiation_direction(), trombone_radiation_direction(), amount,
        );
        self.brassness = amount;
        let work = field.control_work + aperture.control_work;
        self.receipt.configuration_work += work;
        self.receipt.brassness = amount;
        Ok(work)
    }

    pub fn process_sample(&mut self, controls: MetricBodyControls) -> MetricBodyFrame {
        let supply_rate = finite(controls.supply_rate_per_second).max(0.0);
        let supply = (CAPACITY - self.reservoir).max(0.0)
            * (1.0 - (-supply_rate * self.dt).exp());
        self.reservoir += supply;

        let aperture_direction = *self.aperture.direction();
        let effort = self.field.aperture_effort(&aperture_direction).unwrap_or(0.0);
        let aperture = self.aperture.process(
            self.field.momentum_mut(), effort, finite(controls.source_wave),
        );
        let electric_drive = finite(controls.electric_drive).clamp(0.0, 1.0);
        let room_activity = self.room.activity();
        let active = if electric_drive > 0.0 {
            self.active.process(
                self.field.momentum_mut(),
                electric_drive * (controls.source_wave.abs() + aperture.field_flow_after.abs()),
                room_activity,
                electric_controls(electric_drive),
                &mut self.reservoir,
            )
        } else {
            Default::default()
        };
        let displacement = *self.field.displacement();
        let transport = self.transport.process(
            &displacement, self.field.momentum_mut(), controls.transport,
        );
        let propagation = self.field.propagate();
        let field_loss = self.field.damp();
        let room = self.room.process(self.field.momentum_mut(), controls.room);
        let radiation = self.field.radiate(
            &self.radiation, finite(controls.radiation_fraction).clamp(0.0, 1.0),
        ).expect("stored radiation direction is nondegenerate");

        let internal_loss = aperture.internal_loss + active.dissipated_work
            + field_loss.loss + room.internal_loss;
        let radiated = radiation.radiation + room.radiation;
        let source_work = aperture.source_boundary_work + aperture.active_source_work;
        let maximum_local_residual = [
            aperture.closure_residual,
            active.closure_residual,
            transport.metric_residual,
            propagation.residual,
            field_loss.residual,
            room.coupling_metric_residual,
            room.room_closure_residual,
            radiation.residual,
        ].into_iter().map(f64::abs).fold(0.0, f64::max);

        self.receipt.frames += 1;
        self.receipt.source_work += source_work;
        self.receipt.supply_work += supply;
        self.receipt.internal_loss += internal_loss;
        self.receipt.radiation += radiated;
        self.receipt.active_transfer += active.accepted_positive_work;
        self.receipt.stored_energy = self.field.stored_energy()
            + self.aperture.stored_energy() + self.room.stored_energy() + self.reservoir;
        self.receipt.reservoir = self.reservoir;
        self.receipt.maximum_local_residual = self.receipt.maximum_local_residual.max(maximum_local_residual);

        MetricBodyFrame {
            left: 6.0 * (radiation.emitted_coordinate + room.emitted_left),
            right: 6.0 * (radiation.emitted_coordinate + room.emitted_right),
            source_work,
            active_transfer: active.accepted_positive_work,
            internal_loss,
            radiation: radiated,
            maximum_local_residual,
        }
    }

    #[must_use]
    pub fn receipt(&self) -> MetricBodyReceipt { self.receipt }
}

fn electric_controls(amount: f64) -> ActiveInclusionControls {
    ActiveInclusionControls {
        drive: 1.05 * amount,
        headroom: 0.28,
        sustain: 0.78,
        sag: 0.68,
        bloom: 0.12,
        feedback: 0.10,
        space: 0.24,
    }
}

fn shared_source_direction() -> [f64; STRING_BRASS_MODES] {
    interpolate_direction(guitar_source_direction(), trombone_source_direction(), 0.5)
}

fn interpolate_direction(
    left: [f64; STRING_BRASS_MODES], right: [f64; STRING_BRASS_MODES], amount: f64,
) -> [f64; STRING_BRASS_MODES] {
    let mut result = std::array::from_fn(|i| (1.0 - amount) * left[i] + amount * right[i]);
    let norm = result.iter().map(|value| value * value).sum::<f64>().sqrt();
    for value in &mut result { *value /= norm; }
    result
}

fn interpolate_aperture(
    left: CompliantApertureConfiguration,
    right: CompliantApertureConfiguration,
    amount: f64,
) -> CompliantApertureConfiguration {
    macro_rules! l { ($field:ident) => { left.$field + amount * (right.$field - left.$field) }; }
    CompliantApertureConfiguration {
        natural_frequency_hz: l!(natural_frequency_hz),
        quartic_stiffness: l!(quartic_stiffness),
        asymmetry: l!(asymmetry),
        equilibrium_displacement: l!(equilibrium_displacement),
        opening_bias: l!(opening_bias),
        damping_per_second: l!(damping_per_second),
        source_to_aperture_rate_hz: l!(source_to_aperture_rate_hz),
        source_to_field_rate_hz: l!(source_to_field_rate_hz),
        sweep_rate_hz: l!(sweep_rate_hz),
        active_rate_per_second: l!(active_rate_per_second),
        active_target_momentum: l!(active_target_momentum),
    }
}

fn active_directions() -> [[f64; STRING_BRASS_MODES]; 3] {
    std::array::from_fn(|rank| std::array::from_fn(|index| {
        let x = index as f64 / 23.0;
        match rank {
            0 => 0.35 + 1.65 * x,
            1 => if index % 2 == 0 { 1.0 - 0.35 * x } else { -0.82 + 0.25 * x },
            _ => (std::f64::consts::PI * (2.0 * x + 0.15)).sin(),
        }
    }))
}

fn room_directions() -> [[f64; STRING_BRASS_MODES]; ROOM_COURSES] {
    std::array::from_fn(|course| {
        let mut direction = [0.0; STRING_BRASS_MODES];
        let first = (course * 3 + 1) % STRING_BRASS_MODES;
        direction[first] = 1.0;
        direction[(first + 5) % STRING_BRASS_MODES] = if course % 2 == 0 { 0.35 } else { -0.35 };
        direction
    })
}

fn finite(value: f64) -> f64 { if value.is_finite() { value } else { 0.0 } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_spectra_are_six_courses_then_one_bore_without_state_reset() {
        let mut body = AriadneMetricBody::try_new(96_000.0).unwrap();
        let guitar = body.frequencies_hz();
        assert!((guitar[0] - 82.4075).abs() < 0.01);
        assert!((guitar[4] - 110.0008).abs() < 0.01);
        let before = body.state();
        body.set_brassness(1.0).unwrap();
        assert_eq!(body.state(), before);
        let brass = body.frequencies_hz();
        assert!((brass[0] - 116.5420).abs() < 0.01);
        assert!(brass.windows(2).all(|pair| pair[1] > pair[0]));
    }
}