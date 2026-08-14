//! Persistent local Ariadne body.
//!
//! A `Body` owns local stored state and local constitutive laws. It does not
//! own a room, listener, studio placement, global ledger, or another hidden
//! world. Several bodies are composed by [`crate::world::World`] through shared
//! fields and explicit reciprocal coupling edges.

use crate::field::{
    MetricFieldState, MetricLossFrame, MetricPropagationFrame, MetricRadiationFrame,
    ModalMetricWaveField,
};
use crate::ir::MODE_COUNT;
use crate::law::active::{
    ActiveInclusionControls, ActiveInclusionFrame, SynchronizedActiveInclusion,
};
use crate::law::aperture::{
    CompliantApertureBoundary, CompliantApertureFrame,
};
use crate::law::transport::{
    InternalResonanceTransport, ResonantTransportControls, ResonantTransportFrame,
};
use crate::plan::{BodySpec, PlanError};
use crate::ir::TRANSPORT_PAIRS;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyControls {
    /// External normalized source coordinate at the body's compliant boundary.
    pub source_wave: f64,
    /// Work delivered into this body's finite reservoir per second.
    pub work_input: f64,
    /// Participation of the reservoir-paid active inclusion.
    pub active_coupling: f64,
    pub active: ActiveInclusionControls,
    pub transport: ResonantTransportControls,
    /// Fraction of the body's declared radiation aperture removed this frame.
    pub radiation_fraction: f64,
}

impl Default for BodyControls {
    fn default() -> Self {
        Self {
            source_wave: 0.0,
            work_input: 0.0,
            active_coupling: 0.0,
            active: ActiveInclusionControls {
                drive: 0.0,
                ..Default::default()
            },
            transport: ResonantTransportControls::default(),
            radiation_fraction: 0.004,
        }
    }
}

impl BodyControls {
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            source_wave: finite(self.source_wave),
            work_input: finite(self.work_input).clamp(0.0, 1.0),
            active_coupling: finite(self.active_coupling).clamp(0.0, 1.0),
            active: self.active.sanitized(),
            transport: self.transport.sanitized(),
            radiation_fraction: finite(self.radiation_fraction).clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BodyReconfigurationFrame {
    pub amount_before: f64,
    pub amount_after: f64,
    pub field_control_work: f64,
    pub aperture_control_work: f64,
    pub total_control_work: f64,
    pub field_state_displacement: f64,
    pub aperture_state_displacement: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BodyLocalFrame {
    pub source_work: f64,
    pub internal_loss: f64,
    pub energy_before: f64,
    pub energy_after: f64,
    pub closure_residual: f64,
    pub local_residual: f64,
    pub aperture: CompliantApertureFrame,
    pub active: ActiveInclusionFrame,
    pub transport: ResonantTransportFrame,
    pub propagation: MetricPropagationFrame,
    pub field_loss: MetricLossFrame,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyError {
    InvalidSpec(String),
    Field(String),
    Aperture(String),
    Active(String),
    Transport(String),
    Radiation(String),
    NonFiniteState,
}

impl std::fmt::Display for BodyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSpec(message) => write!(formatter, "invalid Ariadne body: {message}"),
            Self::Field(message) => write!(formatter, "Ariadne body field error: {message}"),
            Self::Aperture(message) => write!(formatter, "Ariadne body aperture error: {message}"),
            Self::Active(message) => write!(formatter, "Ariadne body active-material error: {message}"),
            Self::Transport(message) => write!(formatter, "Ariadne body transport error: {message}"),
            Self::Radiation(message) => write!(formatter, "Ariadne body radiation error: {message}"),
            Self::NonFiniteState => formatter.write_str("Ariadne body entered a non-finite state"),
        }
    }
}

impl std::error::Error for BodyError {}

impl From<PlanError> for BodyError {
    fn from(error: PlanError) -> Self {
        Self::InvalidSpec(error.to_string())
    }
}

/// One persistent local metamaterial assembly.
pub struct Body {
    spec: BodySpec,
    sample_rate_hz: f64,
    dt: f64,
    field: ModalMetricWaveField<MODE_COUNT>,
    aperture: CompliantApertureBoundary<MODE_COUNT>,
    active: SynchronizedActiveInclusion<MODE_COUNT>,
    transport: InternalResonanceTransport<MODE_COUNT, TRANSPORT_PAIRS>,
    radiation_direction: [f64; MODE_COUNT],
    reservoir_energy: f64,
    configuration_amount: f64,
}

impl Body {
    pub fn try_new(sample_rate_hz: f64, spec: BodySpec) -> Result<Self, BodyError> {
        spec.validate(sample_rate_hz)?;
        let amount = spec.configuration_path.initial_amount;
        let configuration = spec.configuration_path.configuration_at(amount);
        let field = ModalMetricWaveField::try_new(sample_rate_hz, configuration.metric.runtime())
            .map_err(|message| BodyError::Field(message.to_owned()))?;
        let aperture = CompliantApertureBoundary::try_new(
            sample_rate_hz,
            configuration.aperture.runtime(),
            spec.source_direction,
        )
        .map_err(|error| BodyError::Aperture(error.to_string()))?;
        let active = SynchronizedActiveInclusion::try_new(sample_rate_hz, spec.active_directions)
            .map_err(|error| BodyError::Active(error.to_string()))?;
        let transport = InternalResonanceTransport::try_new(
            sample_rate_hz,
            spec.transport.pairs,
            spec.transport.weights,
        )
        .map_err(|error| BodyError::Transport(error.to_string()))?;
        let radiation_direction = normalize(configuration.radiation_direction)
            .map_err(|message| BodyError::Radiation(message.to_owned()))?;
        let reservoir_energy = spec.reservoir.capacity;
        Ok(Self {
            spec,
            sample_rate_hz,
            dt: sample_rate_hz.recip(),
            field,
            aperture,
            active,
            transport,
            radiation_direction,
            reservoir_energy,
            configuration_amount: amount,
        })
    }

    #[must_use]
    pub fn spec(&self) -> &BodySpec {
        &self.spec
    }

    #[must_use]
    pub fn sample_rate_hz(&self) -> f64 {
        self.sample_rate_hz
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
    pub fn material_stored_energy(&self) -> f64 {
        self.field.stored_energy() + self.aperture.stored_energy()
    }

    #[must_use]
    pub fn reservoir_energy(&self) -> f64 {
        self.reservoir_energy
    }

    #[must_use]
    pub fn total_stored_energy(&self) -> f64 {
        self.material_stored_energy() + self.reservoir_energy
    }

    #[must_use]
    pub fn momentum(&self) -> &[f64; MODE_COUNT] {
        self.field.momentum()
    }

    pub(crate) fn momentum_mut(&mut self) -> &mut [f64; MODE_COUNT] {
        self.field.momentum_mut()
    }

    pub fn set_configuration_amount(
        &mut self,
        amount: f64,
    ) -> Result<BodyReconfigurationFrame, BodyError> {
        let amount = finite(amount).clamp(0.0, 1.0);
        if amount.to_bits() == self.configuration_amount.to_bits() {
            return Ok(BodyReconfigurationFrame {
                amount_before: amount,
                amount_after: amount,
                ..BodyReconfigurationFrame::default()
            });
        }
        let configuration = self.spec.configuration_path.configuration_at(amount);
        configuration
            .validate(self.sample_rate_hz)
            .map_err(|error| BodyError::InvalidSpec(error.to_string()))?;
        let field = self
            .field
            .set_configuration(configuration.metric.runtime())
            .map_err(|message| BodyError::Field(message.to_owned()))?;
        let aperture = self
            .aperture
            .set_configuration(configuration.aperture.runtime())
            .map_err(|error| BodyError::Aperture(error.to_string()))?;
        self.radiation_direction = normalize(configuration.radiation_direction)
            .map_err(|message| BodyError::Radiation(message.to_owned()))?;
        let amount_before = self.configuration_amount;
        self.configuration_amount = amount;
        Ok(BodyReconfigurationFrame {
            amount_before,
            amount_after: amount,
            field_control_work: field.control_work,
            aperture_control_work: aperture.control_work,
            total_control_work: field.control_work + aperture.control_work,
            field_state_displacement: field.state_displacement,
            aperture_state_displacement: aperture.state_displacement,
        })
    }

    /// Advance every body-local law except shared-space exchange, direct
    /// body-to-body coupling, and radiation.
    pub fn process_local(
        &mut self,
        controls: BodyControls,
        returning_space_activity: f64,
    ) -> Result<BodyLocalFrame, BodyError> {
        let controls = controls.sanitized();
        let energy_before = self.total_stored_energy();

        let reservoir_gap = (self.spec.reservoir.capacity - self.reservoir_energy).max(0.0);
        let work_input = (controls.work_input * self.dt).min(reservoir_gap);
        self.reservoir_energy += work_input;

        let aperture_direction = *self.aperture.direction();
        let field_effort = self
            .field
            .aperture_effort(&aperture_direction)
            .map_err(|message| BodyError::Field(message.to_owned()))?;
        let aperture = self.aperture.process(
            self.field.momentum_mut(),
            field_effort,
            controls.source_wave,
        );
        if aperture.non_finite_input {
            return Err(BodyError::NonFiniteState);
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
                finite(returning_space_activity).abs(),
                active_controls,
                &mut self.reservoir_energy,
            )
        } else {
            ActiveInclusionFrame::default()
        };
        if active.non_finite_state {
            return Err(BodyError::NonFiniteState);
        }

        let displacement = *self.field.displacement();
        let transport = self.transport.process(
            &displacement,
            self.field.momentum_mut(),
            controls.transport,
        );
        if transport.non_finite_state {
            return Err(BodyError::NonFiniteState);
        }
        let propagation = self.field.propagate();
        let field_loss = self.field.damp();

        let source_work = work_input + aperture.source_boundary_work + aperture.active_source_work;
        let internal_loss = aperture.internal_loss + active.dissipated_work + field_loss.loss;
        let energy_after = self.total_stored_energy();
        if !energy_after.is_finite()
            || !self.reservoir_energy.is_finite()
            || self.reservoir_energy < -1.0e-15
        {
            return Err(BodyError::NonFiniteState);
        }
        let closure_residual = energy_after - energy_before - source_work + internal_loss;
        let local_residual = [
            aperture.closure_residual,
            active.closure_residual,
            transport.metric_residual,
            propagation.residual,
            field_loss.residual,
        ]
        .into_iter()
        .map(f64::abs)
        .fold(0.0, f64::max);

        Ok(BodyLocalFrame {
            source_work,
            internal_loss,
            energy_before,
            energy_after,
            closure_residual,
            local_residual,
            aperture,
            active,
            transport,
            propagation,
            field_loss,
        })
    }

    pub fn radiate(&mut self, fraction: f64) -> Result<MetricRadiationFrame, BodyError> {
        self.field
            .radiate(&self.radiation_direction, finite(fraction).clamp(0.0, 1.0))
            .map_err(|message| BodyError::Radiation(message.to_owned()))
    }

    pub fn reset(&mut self) -> Result<(), BodyError> {
        let replacement = Self::try_new(self.sample_rate_hz, self.spec.clone())?;
        *self = replacement;
        Ok(())
    }
}

#[inline]
fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
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
    use crate::ir::WorldSpec;

    fn body_spec() -> BodySpec {
        BodySpec::from_world_spec(&WorldSpec::string_brass_v1())
    }

    #[test]
    fn zero_body_is_an_exact_fixed_point() {
        let mut body = Body::try_new(96_000.0, body_spec()).unwrap();
        for _ in 0..4_096 {
            let frame = body.process_local(BodyControls::default(), 0.0).unwrap();
            assert_eq!(frame.source_work, 0.0);
            assert_eq!(frame.internal_loss, 0.0);
            assert!(frame.local_residual < 1.0e-15);
        }
        assert_eq!(body.field_state(), MetricFieldState::default());
    }

    #[test]
    fn configuration_preserves_live_state_and_books_work() {
        let mut body = Body::try_new(96_000.0, body_spec()).unwrap();
        let controls = BodyControls {
            source_wave: 0.8,
            ..Default::default()
        };
        body.process_local(controls, 0.0).unwrap();
        let before = body.field_state();
        let frame = body.set_configuration_amount(1.0).unwrap();
        assert_eq!(before, body.field_state());
        assert_eq!(frame.field_state_displacement, 0.0);
        assert_eq!(
            frame.total_control_work,
            frame.field_control_work + frame.aperture_control_work
        );
    }
}
