//! One compliant boundary that can be played as a transient contact or a
//! sustained lip-reed-like source.
//!
//! The boundary is not a guitar-pick algorithm beside a brass-source
//! algorithm. It owns one persistent displacement/momentum pair and one finite
//! aperture into a caller-owned wave field. Configuration and gesture determine
//! the regime:
//!
//! - guitar-like: no sustained source, no active boundary work, a short player
//!   pulse, a stiff/damped aperture, and transient sweep into a string-like
//!   field;
//! - trombone-like: a compliant aperture, sustained external source wave,
//!   reciprocal field loading, and localized saturable source work.
//!
//! The source and aperture exchange with the field through norm-preserving
//! Givens factors. The aperture's sweeping component is therefore an internal
//! reciprocal transfer, not hidden gain. Sustained oscillation can draw work
//! only from the declared external source boundary.

#![allow(clippy::needless_range_loop)]

use mus_watsn::givens;
use std::f64::consts::TAU;

const MIN_SAMPLE_RATE_HZ: f64 = 8_000.0;
const MAX_SAMPLE_RATE_HZ: f64 = 384_000.0;
const NEWTON_STEPS: usize = 8;
const DENORMAL_FLOOR: f64 = 1.0e-24;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompliantApertureConfiguration {
    pub natural_frequency_hz: f64,
    pub quartic_stiffness: f64,
    pub asymmetry: f64,
    pub equilibrium_displacement: f64,
    pub opening_bias: f64,
    pub damping_per_second: f64,
    pub source_to_aperture_rate_hz: f64,
    pub source_to_field_rate_hz: f64,
    pub sweep_rate_hz: f64,
    /// Localized pressure/gesture-powered saturable activity.
    pub active_rate_per_second: f64,
    pub active_target_momentum: f64,
}

impl Default for CompliantApertureConfiguration {
    fn default() -> Self {
        Self::guitar()
    }
}

impl CompliantApertureConfiguration {
    #[must_use]
    pub fn guitar() -> Self {
        Self {
            natural_frequency_hz: 540.0,
            quartic_stiffness: 4.0e6,
            asymmetry: 0.012,
            equilibrium_displacement: 0.0,
            opening_bias: 0.025,
            damping_per_second: 48.0,
            source_to_aperture_rate_hz: 1_100.0,
            source_to_field_rate_hz: 1_800.0,
            sweep_rate_hz: 420.0,
            active_rate_per_second: 0.0,
            active_target_momentum: 0.25,
        }
    }

    #[must_use]
    pub fn trombone() -> Self {
        Self {
            natural_frequency_hz: 110.0,
            quartic_stiffness: 4.1e6,
            asymmetry: 0.028,
            equilibrium_displacement: 0.0,
            opening_bias: 0.055,
            damping_per_second: 7.9,
            source_to_aperture_rate_hz: 1_020.0,
            source_to_field_rate_hz: 196.0,
            sweep_rate_hz: 286.0,
            active_rate_per_second: 38.0,
            active_target_momentum: 0.334,
        }
    }

    fn validate(self) -> Result<Self, CompliantApertureError> {
        for value in [
            self.natural_frequency_hz,
            self.quartic_stiffness,
            self.asymmetry,
            self.equilibrium_displacement,
            self.opening_bias,
            self.damping_per_second,
            self.source_to_aperture_rate_hz,
            self.source_to_field_rate_hz,
            self.sweep_rate_hz,
            self.active_rate_per_second,
            self.active_target_momentum,
        ] {
            if !value.is_finite() {
                return Err(CompliantApertureError::InvalidConfiguration);
            }
        }
        if !(5.0..=8_000.0).contains(&self.natural_frequency_hz)
            || self.quartic_stiffness < 0.0
            || self.opening_bias < 0.0
            || self.damping_per_second < 0.0
            || self.source_to_aperture_rate_hz < 0.0
            || self.source_to_field_rate_hz < 0.0
            || self.sweep_rate_hz < 0.0
            || self.active_rate_per_second < 0.0
            || self.active_target_momentum <= 0.0
        {
            return Err(CompliantApertureError::InvalidConfiguration);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CompliantApertureState {
    pub displacement: f64,
    pub momentum: f64,
}

impl CompliantApertureState {
    fn is_finite(self) -> bool {
        self.displacement.is_finite() && self.momentum.is_finite()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ApertureConfigurationWorkFrame {
    pub energy_before: f64,
    pub energy_after: f64,
    pub control_work: f64,
    pub state_displacement: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CompliantApertureFrame {
    pub incoming_source_wave: f64,
    pub returned_source_wave: f64,
    pub source_boundary_work: f64,
    pub active_source_work: f64,
    pub internal_loss: f64,
    pub opening: f64,
    pub field_flow_before: f64,
    pub field_flow_after: f64,
    pub field_effort: f64,
    pub sweep_metric_residual: f64,
    pub boundary_metric_residual: f64,
    pub aperture_equation_residual: f64,
    pub energy_before: f64,
    pub energy_after: f64,
    pub closure_residual: f64,
    pub non_finite_input: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompliantApertureError {
    EmptyField,
    InvalidSampleRate,
    InvalidConfiguration,
    InvalidDirection,
    InvalidState,
}

impl std::fmt::Display for CompliantApertureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField => formatter.write_str("compliant aperture field cannot be empty"),
            Self::InvalidSampleRate => {
                formatter.write_str("invalid compliant-aperture sample rate")
            }
            Self::InvalidConfiguration => {
                formatter.write_str("invalid compliant-aperture configuration")
            }
            Self::InvalidDirection => formatter.write_str("invalid compliant-aperture direction"),
            Self::InvalidState => formatter.write_str("invalid compliant-aperture state"),
        }
    }
}

impl std::error::Error for CompliantApertureError {}

#[derive(Debug, Clone, Copy)]
struct AperturePotential {
    angular_frequency: f64,
    quartic_stiffness: f64,
    asymmetry: f64,
    equilibrium: f64,
}

impl AperturePotential {
    fn from_configuration(configuration: CompliantApertureConfiguration) -> Self {
        Self {
            angular_frequency: TAU * configuration.natural_frequency_hz,
            quartic_stiffness: configuration.quartic_stiffness,
            asymmetry: configuration.asymmetry,
            equilibrium: configuration.equilibrium_displacement,
        }
    }

    fn potential(self, displacement: f64) -> f64 {
        let centered = displacement - self.equilibrium;
        let shifted = centered * (centered + 2.0 * self.asymmetry);
        0.5 * self.angular_frequency * self.angular_frequency * centered * centered
            + 0.25 * self.quartic_stiffness * shifted * shifted
    }

    fn divided_gradient(self, old: f64, new: f64) -> f64 {
        let old = old - self.equilibrium;
        let new = new - self.equilibrium;
        let quadratic = 0.5 * self.angular_frequency * self.angular_frequency
            + self.quartic_stiffness * self.asymmetry * self.asymmetry;
        let cubic = self.quartic_stiffness * self.asymmetry;
        let quartic = 0.25 * self.quartic_stiffness;
        quadratic * (old + new)
            + cubic * (old * old + old * new + new * new)
            + quartic
                * (old * old * old
                    + old * old * new
                    + old * new * new
                    + new * new * new)
    }

    fn divided_gradient_derivative(self, old: f64, new: f64) -> f64 {
        let old = old - self.equilibrium;
        let new = new - self.equilibrium;
        let quadratic = 0.5 * self.angular_frequency * self.angular_frequency
            + self.quartic_stiffness * self.asymmetry * self.asymmetry;
        let cubic = self.quartic_stiffness * self.asymmetry;
        let quartic = 0.25 * self.quartic_stiffness;
        quadratic
            + cubic * (old + 2.0 * new)
            + quartic * (old * old + 2.0 * old * new + 3.0 * new * new)
    }
}

/// One persistent compliant aperture into an `N`-coordinate wave body.
#[derive(Debug, Clone)]
pub struct CompliantApertureBoundary<const N: usize> {
    sample_rate_hz: f64,
    dt: f64,
    configuration: CompliantApertureConfiguration,
    potential: AperturePotential,
    direction: [f64; N],
    state: CompliantApertureState,
}

impl<const N: usize> CompliantApertureBoundary<N> {
    pub fn try_new(
        sample_rate_hz: f64,
        configuration: CompliantApertureConfiguration,
        direction: [f64; N],
    ) -> Result<Self, CompliantApertureError> {
        if N == 0 {
            return Err(CompliantApertureError::EmptyField);
        }
        if !sample_rate_hz.is_finite()
            || !(MIN_SAMPLE_RATE_HZ..=MAX_SAMPLE_RATE_HZ).contains(&sample_rate_hz)
        {
            return Err(CompliantApertureError::InvalidSampleRate);
        }
        let configuration = configuration.validate()?;
        let direction = normalized_direction(direction)?;
        Ok(Self {
            sample_rate_hz,
            dt: sample_rate_hz.recip(),
            configuration,
            potential: AperturePotential::from_configuration(configuration),
            direction,
            state: CompliantApertureState::default(),
        })
    }

    #[must_use]
    pub fn sample_rate_hz(&self) -> f64 {
        self.sample_rate_hz
    }

    #[must_use]
    pub fn configuration(&self) -> CompliantApertureConfiguration {
        self.configuration
    }

    #[must_use]
    pub fn state(&self) -> CompliantApertureState {
        self.state
    }

    #[must_use]
    pub fn direction(&self) -> &[f64; N] {
        &self.direction
    }

    #[must_use]
    pub fn stored_energy(&self) -> f64 {
        self.dt
            * (0.5 * self.state.momentum * self.state.momentum
                + self.potential.potential(self.state.displacement))
    }

    pub fn reset(&mut self) {
        self.state = CompliantApertureState::default();
    }

    pub fn restore_state(
        &mut self,
        state: CompliantApertureState,
    ) -> Result<(), CompliantApertureError> {
        if !state.is_finite() {
            return Err(CompliantApertureError::InvalidState);
        }
        self.state = state;
        Ok(())
    }

    pub fn set_configuration(
        &mut self,
        configuration: CompliantApertureConfiguration,
    ) -> Result<ApertureConfigurationWorkFrame, CompliantApertureError> {
        let configuration = configuration.validate()?;
        let energy_before = self.stored_energy();
        let state_before = self.state;
        self.configuration = configuration;
        self.potential = AperturePotential::from_configuration(configuration);
        let energy_after = self.stored_energy();
        Ok(ApertureConfigurationWorkFrame {
            energy_before,
            energy_after,
            control_work: energy_after - energy_before,
            state_displacement: ((self.state.displacement - state_before.displacement).powi(2)
                + (self.state.momentum - state_before.momentum).powi(2))
            .sqrt(),
        })
    }

    /// Advance one source/contact boundary sample.
    ///
    /// `source_wave` is an external normalized boundary coordinate. Net work
    /// entering through it is measured from incoming minus returned wave
    /// energy. `field_effort` affects synchronization and sweep geometry but is
    /// not itself treated as a source of work.
    pub fn process(
        &mut self,
        field_momenta: &mut [f64; N],
        field_effort: f64,
        source_wave: f64,
    ) -> CompliantApertureFrame {
        let energy_before =
            self.stored_energy() + kinetic_energy(field_momenta, self.dt);
        if field_momenta.iter().any(|value| !value.is_finite())
            || !field_effort.is_finite()
            || !source_wave.is_finite()
        {
            return CompliantApertureFrame {
                energy_before,
                energy_after: energy_before,
                non_finite_input: true,
                ..CompliantApertureFrame::default()
            };
        }

        let opening = (self.configuration.opening_bias + self.state.displacement).max(0.0);
        let opening_fraction = opening / (1.0 + opening);
        let field_flow_before = dot(field_momenta, &self.direction);
        let boundary_kinetic_before = self.dt
            * 0.5
            * (self.state.momentum * self.state.momentum
                + field_momenta.iter().map(|value| value * value).sum::<f64>());

        let incoming_source_wave = source_wave;
        let aperture_angle = TAU
            * self.configuration.source_to_aperture_rate_hz
            * self.dt
            * opening_fraction;
        let (source_wave, aperture_momentum) =
            givens(source_wave, self.state.momentum, aperture_angle);
        self.state.momentum = aperture_momentum;

        let mut field_flow = dot(field_momenta, &self.direction);
        let field_angle = TAU
            * self.configuration.source_to_field_rate_hz
            * self.dt
            * opening_fraction;
        let (returned_source_wave, next_field_flow) =
            givens(source_wave, field_flow, field_angle);
        add_projection(
            field_momenta,
            &self.direction,
            next_field_flow - field_flow,
        );
        field_flow = next_field_flow;

        let boundary_kinetic_after = self.dt
            * 0.5
            * (self.state.momentum * self.state.momentum
                + field_momenta.iter().map(|value| value * value).sum::<f64>());
        let source_boundary_work = 0.5
            * self.dt
            * (incoming_source_wave * incoming_source_wave
                - returned_source_wave * returned_source_wave);
        let boundary_metric_residual =
            boundary_kinetic_after - boundary_kinetic_before - source_boundary_work;

        let sweep_energy_before = self.dt
            * 0.5
            * (self.state.momentum * self.state.momentum
                + field_momenta.iter().map(|value| value * value).sum::<f64>());
        let geometry = (2.5 * self.state.displacement + 0.20 * field_effort).tanh();
        let sweep_angle = TAU
            * self.configuration.sweep_rate_hz
            * self.dt
            * (0.2 + 0.8 * opening_fraction)
            * geometry;
        let (next_aperture_momentum, next_field_flow) =
            givens(self.state.momentum, field_flow, sweep_angle);
        self.state.momentum = next_aperture_momentum;
        add_projection(
            field_momenta,
            &self.direction,
            next_field_flow - field_flow,
        );
        field_flow = next_field_flow;
        let sweep_energy_after = self.dt
            * 0.5
            * (self.state.momentum * self.state.momentum
                + field_momenta.iter().map(|value| value * value).sum::<f64>());
        let sweep_metric_residual = sweep_energy_after - sweep_energy_before;

        let mut active_source_work = 0.0;
        let mut internal_loss = 0.0;
        if self.configuration.active_rate_per_second > 0.0
            && incoming_source_wave != 0.0
            && self.state.momentum.abs() > 1.0e-15
        {
            let target = self.configuration.active_target_momentum;
            let synchronization = 1.0
                + 0.4
                    * (-field_flow * self.state.momentum
                        / (0.01 + self.state.momentum.abs()))
                        .tanh();
            let signed_work = self.dt
                * self.configuration.active_rate_per_second
                * incoming_source_wave.abs()
                * opening_fraction
                * synchronization
                * self.state.momentum
                * self.state.momentum
                * (1.0 - self.state.momentum * self.state.momentum / (target * target));
            if signed_work > 0.0 {
                self.state.momentum = self.state.momentum.signum()
                    * (self.state.momentum * self.state.momentum
                        + 2.0 * signed_work / self.dt)
                        .sqrt();
                active_source_work = signed_work;
            } else if signed_work < 0.0 {
                let available = 0.5 * self.dt * self.state.momentum * self.state.momentum;
                let dissipated = (-signed_work).min(available);
                self.state.momentum = self.state.momentum.signum()
                    * (self.state.momentum * self.state.momentum
                        - 2.0 * dissipated / self.dt)
                        .max(0.0)
                        .sqrt();
                internal_loss += dissipated;
            }
        }

        let aperture_equation_residual = self.propagate_aperture();
        let kinetic_before_damping = 0.5 * self.dt * self.state.momentum * self.state.momentum;
        self.state.momentum *=
            (-self.configuration.damping_per_second * self.dt).exp();
        let kinetic_after_damping = 0.5 * self.dt * self.state.momentum * self.state.momentum;
        internal_loss += (kinetic_before_damping - kinetic_after_damping).max(0.0);
        retire_denormal(&mut self.state.displacement);
        retire_denormal(&mut self.state.momentum);

        let energy_after =
            self.stored_energy() + kinetic_energy(field_momenta, self.dt);
        CompliantApertureFrame {
            incoming_source_wave,
            returned_source_wave,
            source_boundary_work,
            active_source_work,
            internal_loss,
            opening,
            field_flow_before,
            field_flow_after: field_flow,
            field_effort,
            sweep_metric_residual,
            boundary_metric_residual,
            aperture_equation_residual,
            energy_before,
            energy_after,
            closure_residual: energy_after - energy_before - source_boundary_work
                - active_source_work
                + internal_loss,
            non_finite_input: false,
        }
    }

    fn propagate_aperture(&mut self) -> f64 {
        let old_displacement = self.state.displacement;
        let old_momentum = self.state.momentum;
        let mut next_displacement = old_displacement + self.dt * old_momentum;
        for _ in 0..NEWTON_STEPS {
            let gradient = self
                .potential
                .divided_gradient(old_displacement, next_displacement);
            let equation = 2.0 * (next_displacement - old_displacement) / self.dt
                - 2.0 * old_momentum
                + self.dt * gradient;
            let derivative = 2.0 / self.dt
                + self.dt
                    * self
                        .potential
                        .divided_gradient_derivative(old_displacement, next_displacement);
            if !derivative.is_finite() || derivative.abs() < 1.0e-14 {
                break;
            }
            next_displacement -= (equation / derivative).clamp(-4.0, 4.0);
        }
        let next_momentum =
            2.0 * (next_displacement - old_displacement) / self.dt - old_momentum;
        let residual = 2.0 * (next_displacement - old_displacement) / self.dt
            - 2.0 * old_momentum
            + self.dt
                * self
                    .potential
                    .divided_gradient(old_displacement, next_displacement);
        self.state.displacement = next_displacement;
        self.state.momentum = next_momentum;
        residual.abs()
    }
}

fn normalized_direction<const N: usize>(
    direction: [f64; N],
) -> Result<[f64; N], CompliantApertureError> {
    if direction.iter().any(|value| !value.is_finite()) {
        return Err(CompliantApertureError::InvalidDirection);
    }
    let norm = direction.iter().map(|value| value * value).sum::<f64>().sqrt();
    if !norm.is_finite() || norm <= 1.0e-15 {
        return Err(CompliantApertureError::InvalidDirection);
    }
    Ok(direction.map(|value| value / norm))
}

#[inline]
fn dot<const N: usize>(left: &[f64; N], right: &[f64; N]) -> f64 {
    left.iter().zip(right).map(|(left, right)| left * right).sum()
}

#[inline]
fn add_projection<const N: usize>(state: &mut [f64; N], direction: &[f64; N], amount: f64) {
    for index in 0..N {
        state[index] += amount * direction[index];
    }
}

#[inline]
fn kinetic_energy<const N: usize>(state: &[f64; N], dt: f64) -> f64 {
    0.5 * dt * state.iter().map(|value| value * value).sum::<f64>()
}

#[inline]
fn retire_denormal(value: &mut f64) {
    if value.abs() < DENORMAL_FLOOR {
        *value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn direction() -> [f64; 6] {
        [1.0, 0.8, 0.5, 0.25, -0.1, 0.05]
    }

    #[test]
    fn zero_source_and_zero_state_are_exact_fixed_point() {
        let mut boundary = CompliantApertureBoundary::try_new(
            48_000.0,
            CompliantApertureConfiguration::guitar(),
            direction(),
        )
        .unwrap();
        let mut field = [0.0; 6];
        for _ in 0..8_192 {
            let frame = boundary.process(&mut field, 0.0, 0.0);
            assert_eq!(frame.source_boundary_work, 0.0);
            assert_eq!(frame.active_source_work, 0.0);
            assert_eq!(frame.internal_loss, 0.0);
            assert_eq!(frame.closure_residual, 0.0);
        }
        assert_eq!(field, [0.0; 6]);
        assert_eq!(boundary.state(), CompliantApertureState::default());
    }

    #[test]
    fn transient_source_pulse_enters_through_one_boundary_receipt() {
        let mut boundary = CompliantApertureBoundary::try_new(
            96_000.0,
            CompliantApertureConfiguration::guitar(),
            direction(),
        )
        .unwrap();
        let mut field = [0.0; 6];
        let frame = boundary.process(&mut field, 0.0, 0.8);
        assert!(frame.source_boundary_work > 0.0);
        assert!(field.iter().any(|value| value.abs() > 0.0));
        assert!(frame.boundary_metric_residual.abs() < 2.0e-18);
        assert!(frame.closure_residual.abs() < 2.0e-15);
    }

    #[test]
    fn sweep_is_reciprocal_and_zero_work() {
        let mut configuration = CompliantApertureConfiguration::trombone();
        configuration.source_to_aperture_rate_hz = 0.0;
        configuration.source_to_field_rate_hz = 0.0;
        configuration.active_rate_per_second = 0.0;
        configuration.damping_per_second = 0.0;
        let mut boundary =
            CompliantApertureBoundary::try_new(48_000.0, configuration, direction()).unwrap();
        boundary
            .restore_state(CompliantApertureState {
                displacement: 0.02,
                momentum: 0.3,
            })
            .unwrap();
        let mut field = [0.4, -0.2, 0.1, 0.0, 0.3, -0.1];
        let frame = boundary.process(&mut field, 0.4, 0.0);
        assert!(frame.field_flow_after != frame.field_flow_before);
        assert!(frame.sweep_metric_residual.abs() < 2.0e-18);
        assert!(frame.closure_residual.abs() < 3.0e-13);
    }

    #[test]
    fn sustained_source_can_perform_local_saturable_work() {
        let mut boundary = CompliantApertureBoundary::try_new(
            48_000.0,
            CompliantApertureConfiguration::trombone(),
            direction(),
        )
        .unwrap();
        boundary
            .restore_state(CompliantApertureState {
                displacement: 0.005,
                momentum: 0.08,
            })
            .unwrap();
        let mut field = [0.02, 0.0, -0.01, 0.0, 0.0, 0.0];
        let frame = boundary.process(&mut field, 0.01, 0.27);
        assert!(frame.active_source_work > 0.0);
        assert!(frame.closure_residual.abs() < 3.0e-13);
    }

    #[test]
    fn high_boundary_momentum_enters_the_dissipative_side_of_the_same_law() {
        let mut boundary = CompliantApertureBoundary::try_new(
            48_000.0,
            CompliantApertureConfiguration::trombone(),
            direction(),
        )
        .unwrap();
        boundary
            .restore_state(CompliantApertureState {
                displacement: 0.01,
                momentum: 0.8,
            })
            .unwrap();
        let mut field = [0.0; 6];
        let frame = boundary.process(&mut field, 0.0, 0.27);
        assert!(frame.internal_loss > 0.0);
        assert!(frame.closure_residual.abs() < 3.0e-13);
    }

    #[test]
    fn changing_playing_regime_books_configuration_work_without_reset() {
        let mut boundary = CompliantApertureBoundary::try_new(
            96_000.0,
            CompliantApertureConfiguration::guitar(),
            direction(),
        )
        .unwrap();
        let state = CompliantApertureState {
            displacement: 0.013,
            momentum: -0.2,
        };
        boundary.restore_state(state).unwrap();
        let frame = boundary
            .set_configuration(CompliantApertureConfiguration::trombone())
            .unwrap();
        assert!(frame.control_work.abs() > 1.0e-12);
        assert_eq!(frame.state_displacement, 0.0);
        assert_eq!(boundary.state(), state);
    }

    #[test]
    fn invalid_inputs_are_refused_or_contained() {
        assert!(CompliantApertureBoundary::<6>::try_new(
            7_999.0,
            CompliantApertureConfiguration::default(),
            direction(),
        )
        .is_err());
        assert!(CompliantApertureBoundary::<6>::try_new(
            48_000.0,
            CompliantApertureConfiguration::default(),
            [0.0; 6],
        )
        .is_err());
        let mut boundary = CompliantApertureBoundary::try_new(
            48_000.0,
            CompliantApertureConfiguration::default(),
            direction(),
        )
        .unwrap();
        let mut field = [0.0; 6];
        let frame = boundary.process(&mut field, f64::NAN, 0.0);
        assert!(frame.non_finite_input);
        assert_eq!(field, [0.0; 6]);
    }
}