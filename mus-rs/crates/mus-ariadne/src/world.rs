//! Canonical Ariadne executor.
//!
//! A `World` owns one persistent material field, one compliant boundary, one
//! finite work reservoir, one active inclusion, one ordered transport graph,
//! one reciprocal space, and one radiation aperture. Instrument names never
//! select algorithms; they identify configurations and gestures in this world.

use mus_watsn::{WorkFrame, WorkLedger};

use crate::field::{MetricFieldState, ModalMetricWaveField};
use crate::ir::{
    SpecError, WorldConfiguration, WorldSpec, ACTIVE_RANKS, MODE_COUNT, SPACE_COURSES,
    TRANSPORT_PAIRS,
};
use crate::law::active::{
    ActiveInclusionControls, ActiveInclusionFrame, SynchronizedActiveInclusion,
};
use crate::law::aperture::{
    ApertureConfigurationWorkFrame, CompliantApertureBoundary, CompliantApertureFrame,
};
use crate::law::transport::{
    InternalResonanceTransport, ResonantTransportControls, ResonantTransportFrame,
};
use crate::performance::{CostEstimate, RuntimeLayout};
use crate::space::{ReciprocalSpace, SpaceControls, SpaceFrame};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldControls {
    /// External normalized source coordinate at the compliant boundary.
    pub source_wave: f64,
    /// Work delivered into the finite reservoir per second.
    pub work_input: f64,
    /// Participation of the reservoir-paid active inclusion.
    pub active_coupling: f64,
    pub active: ActiveInclusionControls,
    pub transport: ResonantTransportControls,
    pub space: SpaceControls,
    pub radiation_fraction: f64,
    /// Multiplier over the anatomy's declared observation scale.
    pub observation_scale: f64,
}

impl Default for WorldControls {
    fn default() -> Self {
        let mut active = ActiveInclusionControls::default();
        active.drive = 0.0;
        Self {
            source_wave: 0.0,
            work_input: 0.0,
            active_coupling: 0.0,
            active,
            transport: ResonantTransportControls::default(),
            space: SpaceControls::default(),
            radiation_fraction: 0.004,
            observation_scale: 1.0,
        }
    }
}

impl WorldControls {
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            source_wave: finite(self.source_wave),
            work_input: finite(self.work_input).clamp(0.0, 1.0),
            active_coupling: finite(self.active_coupling).clamp(0.0, 1.0),
            active: self.active.sanitized(),
            transport: self.transport.sanitized(),
            space: self.space.sanitized(),
            radiation_fraction: finite(self.radiation_fraction).clamp(0.0, 1.0),
            observation_scale: finite(self.observation_scale).clamp(0.0, 16.0),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FrameOutput {
    pub left: f64,
    pub right: f64,
    pub material: f64,
    pub space_left: f64,
    pub space_right: f64,
    pub source_work: f64,
    pub control_work: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    pub local_residual: f64,
    pub aperture: CompliantApertureFrame,
    pub active: ActiveInclusionFrame,
    pub transport: ResonantTransportFrame,
    pub space: SpaceFrame,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ReconfigurationFrame {
    pub amount_before: f64,
    pub amount_after: f64,
    pub field_control_work: f64,
    pub aperture_control_work: f64,
    pub total_control_work: f64,
    pub field_state_displacement: f64,
    pub aperture_state_displacement: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldReceipt {
    pub world_digest: String,
    pub sample_frames: u64,
    pub ledger_frames: u64,
    pub source_work: f64,
    pub control_work: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    pub initial_energy: f64,
    pub current_energy: f64,
    pub reservoir_energy: f64,
    pub configuration_amount: f64,
    pub global_closure_residual: f64,
    pub cumulative_closure_residual: f64,
    pub maximum_frame_residual: f64,
    pub maximum_local_residual: f64,
    pub runtime_layout: RuntimeLayout,
    pub structural_cost: CostEstimate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldError {
    InvalidSpec(String),
    Field(String),
    Aperture(String),
    Active(String),
    Transport(String),
    Space(String),
    Radiation(String),
    NonFiniteState,
}

impl std::fmt::Display for WorldError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSpec(message) => write!(formatter, "invalid Ariadne world: {message}"),
            Self::Field(message) => write!(formatter, "Ariadne field error: {message}"),
            Self::Aperture(message) => write!(formatter, "Ariadne aperture error: {message}"),
            Self::Active(message) => write!(formatter, "Ariadne active-material error: {message}"),
            Self::Transport(message) => write!(formatter, "Ariadne transport error: {message}"),
            Self::Space(message) => write!(formatter, "Ariadne space error: {message}"),
            Self::Radiation(message) => write!(formatter, "Ariadne radiation error: {message}"),
            Self::NonFiniteState => formatter.write_str("Ariadne world entered a non-finite state"),
        }
    }
}

impl std::error::Error for WorldError {}

impl From<SpecError> for WorldError {
    fn from(error: SpecError) -> Self {
        Self::InvalidSpec(error.to_string())
    }
}

/// The single canonical persistent world for Ariadne V1.
pub struct World {
    spec: WorldSpec,
    digest: String,
    dt: f64,
    field: ModalMetricWaveField<MODE_COUNT>,
    aperture: CompliantApertureBoundary<MODE_COUNT>,
    active: SynchronizedActiveInclusion<MODE_COUNT>,
    transport: InternalResonanceTransport<MODE_COUNT, TRANSPORT_PAIRS>,
    space: ReciprocalSpace<MODE_COUNT, SPACE_COURSES>,
    radiation_direction: [f64; MODE_COUNT],
    reservoir_energy: f64,
    configuration_amount: f64,
    pending_control_work: f64,
    pending_energy_start: Option<f64>,
    ledger: WorkLedger,
    sample_frames: u64,
    maximum_local_residual: f64,
    runtime_layout: RuntimeLayout,
    structural_cost: CostEstimate,
}

impl World {
    pub fn try_new(spec: WorldSpec) -> Result<Self, WorldError> {
        spec.validate()?;
        let digest = spec.digest()?;
        let sample_rate = spec.internal_sample_rate_hz;
        let amount = spec.configuration_path.initial_amount;
        let configuration = spec.configuration_path.configuration_at(amount);
        let field = ModalMetricWaveField::try_new(sample_rate, configuration.metric.runtime())
            .map_err(|message| WorldError::Field(message.to_owned()))?;
        let aperture = CompliantApertureBoundary::try_new(
            sample_rate,
            configuration.aperture.runtime(),
            spec.source_direction,
        )
        .map_err(|error| WorldError::Aperture(error.to_string()))?;
        let active = SynchronizedActiveInclusion::try_new(sample_rate, spec.active_directions)
            .map_err(|error| WorldError::Active(error.to_string()))?;
        let transport = InternalResonanceTransport::try_new(
            sample_rate,
            spec.transport.pairs,
            spec.transport.weights,
        )
        .map_err(|error| WorldError::Transport(error.to_string()))?;
        let space = ReciprocalSpace::try_new(sample_rate, spec.space)
            .map_err(|error| WorldError::Space(error.to_string()))?;
        let radiation_direction = normalize(configuration.radiation_direction)
            .map_err(|message| WorldError::Radiation(message.to_owned()))?;
        let runtime_layout = RuntimeLayout::from_spec(&spec);
        let structural_cost = CostEstimate::for_spec(&spec);
        let reservoir_energy = spec.reservoir.capacity;

        Ok(Self {
            spec,
            digest,
            dt: sample_rate.recip(),
            field,
            aperture,
            active,
            transport,
            space,
            radiation_direction,
            reservoir_energy,
            configuration_amount: amount,
            pending_control_work: 0.0,
            pending_energy_start: None,
            ledger: WorkLedger::default(),
            sample_frames: 0,
            maximum_local_residual: 0.0,
            runtime_layout,
            structural_cost,
        })
    }

    #[must_use]
    pub fn spec(&self) -> &WorldSpec {
        &self.spec
    }

    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    #[must_use]
    pub fn sample_rate_hz(&self) -> f64 {
        self.spec.internal_sample_rate_hz
    }

    #[must_use]
    pub fn configuration_amount(&self) -> f64 {
        self.configuration_amount
    }

    #[must_use]
    pub fn field_state(&self) -> MetricFieldState<MODE_COUNT> {
        self.field.state()
    }

    #[must_use]
    pub fn total_stored_energy(&self) -> f64 {
        self.field.stored_energy()
            + self.aperture.stored_energy()
            + self.space.stored_energy()
            + self.reservoir_energy
    }

    #[must_use]
    pub fn runtime_layout(&self) -> RuntimeLayout {
        self.runtime_layout
    }

    #[must_use]
    pub fn structural_cost(&self) -> CostEstimate {
        self.structural_cost
    }

    /// Move along the world's declared anatomy path without resetting state.
    /// The exact field-metric and aperture-potential change is queued as control
    /// work and included in the next global ledger frame.
    pub fn set_configuration_amount(
        &mut self,
        amount: f64,
    ) -> Result<ReconfigurationFrame, WorldError> {
        let amount = finite(amount).clamp(0.0, 1.0);
        if amount.to_bits() == self.configuration_amount.to_bits() {
            return Ok(ReconfigurationFrame {
                amount_before: amount,
                amount_after: amount,
                ..ReconfigurationFrame::default()
            });
        }
        let energy_before = self.total_stored_energy();
        if self.pending_energy_start.is_none() {
            self.pending_energy_start = Some(energy_before);
        }
        let configuration = self.spec.configuration_path.configuration_at(amount);
        configuration.validate(self.sample_rate_hz())?;
        let field = self
            .field
            .set_configuration(configuration.metric.runtime())
            .map_err(|message| WorldError::Field(message.to_owned()))?;
        let aperture = self
            .aperture
            .set_configuration(configuration.aperture.runtime())
            .map_err(|error| WorldError::Aperture(error.to_string()))?;
        self.radiation_direction = normalize(configuration.radiation_direction)
            .map_err(|message| WorldError::Radiation(message.to_owned()))?;
        let total_control_work = field.control_work + aperture.control_work;
        self.pending_control_work += total_control_work;
        let amount_before = self.configuration_amount;
        self.configuration_amount = amount;
        Ok(ReconfigurationFrame {
            amount_before,
            amount_after: amount,
            field_control_work: field.control_work,
            aperture_control_work: aperture.control_work,
            total_control_work,
            field_state_displacement: field.state_displacement,
            aperture_state_displacement: aperture.state_displacement,
        })
    }

    /// Advance one material-clock sample. No allocation or locking occurs here.
    pub fn process_sample(&mut self, controls: WorldControls) -> Result<FrameOutput, WorldError> {
        let controls = controls.sanitized();
        let physical_energy_start = self.total_stored_energy();
        let energy_start = self
            .pending_energy_start
            .take()
            .unwrap_or(physical_energy_start);
        let control_work = std::mem::take(&mut self.pending_control_work);

        let reservoir_gap = (self.spec.reservoir.capacity - self.reservoir_energy).max(0.0);
        let work_input = (controls.work_input * self.dt).min(reservoir_gap);
        self.reservoir_energy += work_input;

        let aperture_direction = *self.aperture.direction();
        let field_effort = self
            .field
            .aperture_effort(&aperture_direction)
            .map_err(|message| WorldError::Field(message.to_owned()))?;
        let aperture = self.aperture.process(
            self.field.momentum_mut(),
            field_effort,
            controls.source_wave,
        );
        if aperture.non_finite_input {
            return Err(WorldError::NonFiniteState);
        }

        let boundary_activity = controls.source_wave.abs()
            + aperture.field_flow_after.abs()
            + 0.5 * self.aperture.state().momentum.abs();
        let active = if controls.active_coupling > 0.0 {
            let mut active_controls = controls.active;
            active_controls.drive *= controls.active_coupling;
            self.active.process(
                self.field.momentum_mut(),
                controls.active_coupling * boundary_activity,
                self.space.activity(),
                active_controls,
                &mut self.reservoir_energy,
            )
        } else {
            ActiveInclusionFrame::default()
        };
        if active.non_finite_state {
            return Err(WorldError::NonFiniteState);
        }

        let displacement = *self.field.displacement();
        let transport = self.transport.process(
            &displacement,
            self.field.momentum_mut(),
            controls.transport,
        );
        if transport.non_finite_state {
            return Err(WorldError::NonFiniteState);
        }
        let propagation = self.field.propagate();
        let field_loss = self.field.damp();
        let space = self.space.process(self.field.momentum_mut(), controls.space);
        let material_radiation = self
            .field
            .radiate(&self.radiation_direction, controls.radiation_fraction)
            .map_err(|message| WorldError::Radiation(message.to_owned()))?;

        let source_work =
            work_input + aperture.source_boundary_work + aperture.active_source_work;
        let internal_loss = aperture.internal_loss
            + active.dissipated_work
            + field_loss.loss
            + space.internal_loss;
        let radiation = material_radiation.radiation + space.radiation;
        let energy_end = self.total_stored_energy();
        if !energy_end.is_finite()
            || !self.reservoir_energy.is_finite()
            || self.reservoir_energy < -1.0e-15
        {
            return Err(WorldError::NonFiniteState);
        }

        let frame = WorkFrame {
            energy_start,
            source_work,
            control_work,
            internal_loss,
            radiation_loss: radiation,
            output_work: 0.0,
            energy_end,
        };
        self.ledger.push(frame);
        self.sample_frames += 1;

        let local_residual = [
            aperture.closure_residual,
            active.closure_residual,
            transport.metric_residual,
            propagation.residual,
            field_loss.residual,
            space.coupling_metric_residual,
            space.space_closure_residual,
            material_radiation.residual,
            frame.residual(),
        ]
        .into_iter()
        .map(f64::abs)
        .fold(0.0, f64::max);
        self.maximum_local_residual = self.maximum_local_residual.max(local_residual);

        let scale = self.spec.radiation.observation_scale * controls.observation_scale;
        let material = material_radiation.emitted_coordinate;
        let left = scale * (material + space.emitted_left);
        let right = scale * (material + space.emitted_right);
        if !left.is_finite() || !right.is_finite() {
            return Err(WorldError::NonFiniteState);
        }

        Ok(FrameOutput {
            left,
            right,
            material,
            space_left: space.emitted_left,
            space_right: space.emitted_right,
            source_work,
            control_work,
            internal_loss,
            radiation,
            local_residual,
            aperture,
            active,
            transport,
            space,
        })
    }

    #[must_use]
    pub fn receipt(&self) -> WorldReceipt {
        let initial_energy = self
            .ledger
            .initial_energy
            .unwrap_or_else(|| self.total_stored_energy());
        WorldReceipt {
            world_digest: self.digest.clone(),
            sample_frames: self.sample_frames,
            ledger_frames: self.ledger.frames,
            source_work: self.ledger.source_work,
            control_work: self.ledger.control_work + self.pending_control_work,
            internal_loss: self.ledger.internal_loss,
            radiation: self.ledger.radiation_loss,
            initial_energy,
            current_energy: self.total_stored_energy(),
            reservoir_energy: self.reservoir_energy,
            configuration_amount: self.configuration_amount,
            global_closure_residual: self.ledger.global_residual(),
            cumulative_closure_residual: self.ledger.cumulative_residual,
            maximum_frame_residual: self.ledger.max_abs_residual,
            maximum_local_residual: self.maximum_local_residual,
            runtime_layout: self.runtime_layout,
            structural_cost: self.structural_cost,
        }
    }

    /// Reconstruct the same declared world. This is a host/reset boundary and
    /// intentionally not an audio-thread operation.
    pub fn reset(&mut self) -> Result<(), WorldError> {
        let replacement = Self::try_new(self.spec.clone())?;
        *self = replacement;
        Ok(())
    }
}

#[inline]
fn finite(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

fn normalize<const N: usize>(mut direction: [f64; N]) -> Result<[f64; N], &'static str> {
    let norm = direction
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm <= 1.0e-12 {
        return Err("radiation direction must be finite and nondegenerate");
    }
    for value in &mut direction {
        *value /= norm;
    }
    Ok(direction)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_zero_world_stays_silent() {
        let mut world = World::try_new(WorldSpec::string_brass_v1()).unwrap();
        for _ in 0..4_096 {
            let frame = world.process_sample(WorldControls::default()).unwrap();
            assert_eq!(frame.left, 0.0);
            assert_eq!(frame.right, 0.0);
        }
        let receipt = world.receipt();
        assert!(receipt.global_closure_residual.abs() < 1.0e-15);
    }

    #[test]
    fn configuration_changes_metric_without_resetting_state() {
        let mut world = World::try_new(WorldSpec::string_brass_v1()).unwrap();
        let mut controls = WorldControls::default();
        controls.source_wave = 0.8;
        for _ in 0..128 {
            world.process_sample(controls).unwrap();
            controls.source_wave = 0.0;
        }
        let before = world.field_state();
        let reconfiguration = world.set_configuration_amount(1.0).unwrap();
        let after = world.field_state();
        assert_eq!(before, after);
        assert_eq!(reconfiguration.field_state_displacement, 0.0);
        world.process_sample(WorldControls::default()).unwrap();
        assert!(world.receipt().global_closure_residual.abs() < 2.0e-10);
    }

    #[test]
    fn driven_material_can_only_spend_declared_reservoir_work() {
        let mut world = World::try_new(WorldSpec::string_brass_v1()).unwrap();
        let mut controls = WorldControls::default();
        controls.source_wave = 0.8;
        controls.active_coupling = 1.0;
        controls.active.drive = 1.1;
        controls.active.headroom = 0.2;
        controls.work_input = 0.0;
        for sample in 0..8_192 {
            if sample > 64 {
                controls.source_wave = 0.0;
            }
            world.process_sample(controls).unwrap();
        }
        let receipt = world.receipt();
        assert!(receipt.reservoir_energy >= 0.0);
        assert!(receipt.global_closure_residual.abs() < 2.0e-9);
    }
}
