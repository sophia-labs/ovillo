//! One fixed-rank Hamiltonian wave body whose configuration spans string- and
//! bore-like regimes.
//!
//! A uniform stretched string and a lossless one-dimensional acoustic bore are
//! both second-order wave systems. After a structure-preserving modal
//! reduction they share the same canonical state
//!
//! ```text
//! H_lambda(q, p) = dt/2 sum_n [p_n^2 + omega_n(lambda)^2 q_n^2].
//! ```
//!
//! `lambda` changes length, boundary phase, weak dispersion, and passive loss;
//! it does not select another renderer. Changing `lambda` while the body stores
//! displacement performs the exact configuration work
//!
//! ```text
//! W_config = H_lambda_new(q,p) - H_lambda_old(q,p).
//! ```
//!
//! A guitar-like configuration uses a nearly harmonic, weakly stiff spectrum,
//! transient contact, and a narrow observation aperture. A trombone-like
//! configuration uses a longer near-harmonic field, a compliant sustained
//! boundary, and a broad radiating aperture. Both retain the same state
//! dimension, solver, energy law, and receipt vocabulary.

#![allow(clippy::needless_range_loop)]

use std::f64::consts::TAU;

const MIN_SAMPLE_RATE_HZ: f64 = 8_000.0;
const MAX_SAMPLE_RATE_HZ: f64 = 384_000.0;
const MIN_REFERENCE_FREQUENCY_HZ: f64 = 5.0;
const MAX_REFERENCE_FREQUENCY_HZ: f64 = 8_000.0;
const MIN_METRIC_FACTOR: f64 = 0.05;
const DENORMAL_FLOOR: f64 = 1.0e-24;

/// Configuration coordinates for one fixed material body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveFieldConfiguration {
    /// Effective material length relative to the reference body.
    pub length_ratio: f64,
    /// Continuous endpoint phase. Zero gives an integer harmonic ladder.
    pub boundary_phase: f64,
    /// Weak stiffness/flare correction in `sqrt(1 + dispersion * n^2)`.
    pub dispersion: f64,
    /// T60 of the lowest retained mode.
    pub low_mode_t60_seconds: f64,
    /// T60 of the highest retained mode.
    pub high_mode_t60_seconds: f64,
}

impl Default for WaveFieldConfiguration {
    fn default() -> Self {
        Self {
            length_ratio: 1.0,
            boundary_phase: 0.0,
            dispersion: 0.0,
            low_mode_t60_seconds: 1.5,
            high_mode_t60_seconds: 0.35,
        }
    }
}

impl WaveFieldConfiguration {
    /// A conservative plucked-string region. This is a configuration of the
    /// shared body, not a specialized string renderer.
    #[must_use]
    pub fn guitar() -> Self {
        Self {
            length_ratio: 1.0,
            boundary_phase: 0.0,
            dispersion: 1.8e-5,
            low_mode_t60_seconds: 2.2,
            high_mode_t60_seconds: 0.28,
        }
    }

    /// A conservative near-harmonic brass-bore region. The compliant aperture
    /// and source gesture live in `ariadne_compliant_aperture`; the field itself
    /// remains the same object used by [`Self::guitar`].
    #[must_use]
    pub fn trombone() -> Self {
        Self {
            length_ratio: 1.015,
            boundary_phase: -0.012,
            dispersion: 1.9e-5,
            low_mode_t60_seconds: 3.1,
            high_mode_t60_seconds: 0.66,
        }
    }

    fn validate_for<const N: usize>(self) -> Result<Self, MorphingFieldError> {
        if N == 0 {
            return Err(MorphingFieldError::EmptyField);
        }
        for value in [
            self.length_ratio,
            self.boundary_phase,
            self.dispersion,
            self.low_mode_t60_seconds,
            self.high_mode_t60_seconds,
        ] {
            if !value.is_finite() {
                return Err(MorphingFieldError::InvalidConfiguration);
            }
        }
        if !(0.1..=12.0).contains(&self.length_ratio)
            || !(-0.45..=0.45).contains(&self.boundary_phase)
            || self.low_mode_t60_seconds <= 0.0
            || self.high_mode_t60_seconds <= 0.0
        {
            return Err(MorphingFieldError::InvalidConfiguration);
        }
        let maximum_mode = N as f64 + self.boundary_phase;
        if maximum_mode <= 0.0
            || 1.0 + self.dispersion * maximum_mode * maximum_mode < MIN_METRIC_FACTOR
        {
            return Err(MorphingFieldError::InvalidConfiguration);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveFieldState<const N: usize> {
    pub displacement: [f64; N],
    pub momentum: [f64; N],
}

impl<const N: usize> Default for WaveFieldState<N> {
    fn default() -> Self {
        Self {
            displacement: [0.0; N],
            momentum: [0.0; N],
        }
    }
}

impl<const N: usize> WaveFieldState<N> {
    fn is_finite(&self) -> bool {
        self.displacement.iter().all(|value| value.is_finite())
            && self.momentum.iter().all(|value| value.is_finite())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ConfigurationWorkFrame {
    pub energy_before: f64,
    pub energy_after: f64,
    pub control_work: f64,
    pub state_displacement: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PropagationFrame {
    pub energy_before: f64,
    pub energy_after: f64,
    pub metric_residual: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FieldLossFrame {
    pub energy_before: f64,
    pub energy_after: f64,
    pub internal_loss: f64,
    pub closure_residual: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FieldRadiationFrame {
    pub emitted_coordinate: f64,
    pub radiated_energy: f64,
    pub energy_before: f64,
    pub energy_after: f64,
    pub closure_residual: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MorphingFieldError {
    EmptyField,
    InvalidSampleRate,
    InvalidReferenceFrequency,
    InvalidConfiguration,
    InvalidDirection,
    InvalidState,
    InvalidRadiationFraction,
}

impl std::fmt::Display for MorphingFieldError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField => formatter.write_str("morphing wave field cannot be empty"),
            Self::InvalidSampleRate => formatter.write_str("invalid morphing-field sample rate"),
            Self::InvalidReferenceFrequency => {
                formatter.write_str("invalid morphing-field reference frequency")
            }
            Self::InvalidConfiguration => {
                formatter.write_str("invalid morphing-field configuration")
            }
            Self::InvalidDirection => formatter.write_str("invalid finite-aperture direction"),
            Self::InvalidState => formatter.write_str("morphing-field state must be finite"),
            Self::InvalidRadiationFraction => {
                formatter.write_str("radiation fraction must be finite in [0, 1]")
            }
        }
    }
}

impl std::error::Error for MorphingFieldError {}

/// Fixed-capacity modal reduction of one configurable Hamiltonian wave body.
#[derive(Debug, Clone)]
pub struct MorphingWaveField<const N: usize> {
    sample_rate_hz: f64,
    dt: f64,
    reference_frequency_hz: f64,
    configuration: WaveFieldConfiguration,
    state: WaveFieldState<N>,
    angular_frequency: [f64; N],
    cosine: [f64; N],
    sine: [f64; N],
    retain: [f64; N],
}

impl<const N: usize> MorphingWaveField<N> {
    pub fn try_new(
        sample_rate_hz: f64,
        reference_frequency_hz: f64,
        configuration: WaveFieldConfiguration,
    ) -> Result<Self, MorphingFieldError> {
        if !sample_rate_hz.is_finite()
            || !(MIN_SAMPLE_RATE_HZ..=MAX_SAMPLE_RATE_HZ).contains(&sample_rate_hz)
        {
            return Err(MorphingFieldError::InvalidSampleRate);
        }
        if !reference_frequency_hz.is_finite()
            || !(MIN_REFERENCE_FREQUENCY_HZ..=MAX_REFERENCE_FREQUENCY_HZ)
                .contains(&reference_frequency_hz)
        {
            return Err(MorphingFieldError::InvalidReferenceFrequency);
        }
        let configuration = configuration.validate_for::<N>()?;
        let mut result = Self {
            sample_rate_hz,
            dt: sample_rate_hz.recip(),
            reference_frequency_hz,
            configuration,
            state: WaveFieldState::default(),
            angular_frequency: [0.0; N],
            cosine: [1.0; N],
            sine: [0.0; N],
            retain: [1.0; N],
        };
        result.rebuild_coefficients();
        Ok(result)
    }

    #[must_use]
    pub fn sample_rate_hz(&self) -> f64 {
        self.sample_rate_hz
    }

    #[must_use]
    pub fn reference_frequency_hz(&self) -> f64 {
        self.reference_frequency_hz
    }

    #[must_use]
    pub fn configuration(&self) -> WaveFieldConfiguration {
        self.configuration
    }

    #[must_use]
    pub fn state(&self) -> WaveFieldState<N> {
        self.state
    }

    #[must_use]
    pub fn frequencies_hz(&self) -> [f64; N] {
        self.angular_frequency.map(|omega| omega / TAU)
    }

    pub fn reset(&mut self) {
        self.state = WaveFieldState::default();
    }

    pub fn restore_state(&mut self, state: WaveFieldState<N>) -> Result<(), MorphingFieldError> {
        if !state.is_finite() {
            return Err(MorphingFieldError::InvalidState);
        }
        self.state = state;
        Ok(())
    }

    #[must_use]
    pub fn stored_energy(&self) -> f64 {
        field_energy(&self.state, &self.angular_frequency, self.dt)
    }

    /// Change geometry/material coordinates while preserving the live state
    /// coordinates. The exact metric-energy difference is control work.
    pub fn set_configuration(
        &mut self,
        configuration: WaveFieldConfiguration,
    ) -> Result<ConfigurationWorkFrame, MorphingFieldError> {
        let configuration = configuration.validate_for::<N>()?;
        let energy_before = self.stored_energy();
        let state_before = self.state;
        self.configuration = configuration;
        self.rebuild_coefficients();
        let energy_after = self.stored_energy();
        Ok(ConfigurationWorkFrame {
            energy_before,
            energy_after,
            control_work: energy_after - energy_before,
            state_displacement: state_distance(&state_before, &self.state),
        })
    }

    /// Exact fixed-configuration modal propagation.
    pub fn propagate(&mut self) -> PropagationFrame {
        let energy_before = self.stored_energy();
        for index in 0..N {
            let displacement = self.state.displacement[index];
            let momentum = self.state.momentum[index];
            let omega = self.angular_frequency[index];
            self.state.displacement[index] = self.cosine[index] * displacement
                + self.sine[index] * momentum / omega;
            self.state.momentum[index] = self.cosine[index] * momentum
                - self.sine[index] * omega * displacement;
            retire_denormal(&mut self.state.displacement[index]);
            retire_denormal(&mut self.state.momentum[index]);
        }
        let energy_after = self.stored_energy();
        PropagationFrame {
            energy_before,
            energy_after,
            metric_residual: energy_after - energy_before,
        }
    }

    /// Apply the configuration's passive per-mode attenuation and book the
    /// exact energy deficit.
    pub fn damp(&mut self) -> FieldLossFrame {
        let energy_before = self.stored_energy();
        for index in 0..N {
            self.state.displacement[index] *= self.retain[index];
            self.state.momentum[index] *= self.retain[index];
            retire_denormal(&mut self.state.displacement[index]);
            retire_denormal(&mut self.state.momentum[index]);
        }
        let energy_after = self.stored_energy();
        let internal_loss = (energy_before - energy_after).max(0.0);
        FieldLossFrame {
            energy_before,
            energy_after,
            internal_loss,
            closure_residual: energy_after - energy_before + internal_loss,
        }
    }

    /// Remove one finite-aperture momentum projection as radiation. The
    /// emitted coordinate and retained state form an exact orthogonal split.
    pub fn radiate(
        &mut self,
        direction: &[f64; N],
        fraction: f64,
    ) -> Result<FieldRadiationFrame, MorphingFieldError> {
        if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
            return Err(MorphingFieldError::InvalidRadiationFraction);
        }
        let direction = normalized_direction(direction)?;
        let energy_before = self.stored_energy();
        let projection = dot(&self.state.momentum, &direction);
        let emitted_coordinate = fraction.sqrt() * projection;
        let removal = 1.0 - (1.0 - fraction).sqrt();
        for index in 0..N {
            self.state.momentum[index] -= removal * direction[index] * projection;
        }
        let energy_after = self.stored_energy();
        let radiated_energy = 0.5 * self.dt * emitted_coordinate * emitted_coordinate;
        Ok(FieldRadiationFrame {
            emitted_coordinate,
            radiated_energy,
            energy_before,
            energy_after,
            closure_residual: energy_after - energy_before + radiated_energy,
        })
    }

    #[must_use]
    pub fn aperture_flow(&self, direction: &[f64; N]) -> Result<f64, MorphingFieldError> {
        let direction = normalized_direction(direction)?;
        Ok(dot(&self.state.momentum, &direction))
    }

    /// Normalized effort coordinate conjugate to the finite-aperture flow.
    #[must_use]
    pub fn aperture_effort(&self, direction: &[f64; N]) -> Result<f64, MorphingFieldError> {
        let direction = normalized_direction(direction)?;
        Ok((0..N)
            .map(|index| {
                direction[index]
                    * self.angular_frequency[index]
                    * self.state.displacement[index]
            })
            .sum())
    }

    #[must_use]
    pub fn displacement(&self) -> &[f64; N] {
        &self.state.displacement
    }

    #[must_use]
    pub fn momentum(&self) -> &[f64; N] {
        &self.state.momentum
    }

    pub fn momentum_mut(&mut self) -> &mut [f64; N] {
        &mut self.state.momentum
    }

    fn rebuild_coefficients(&mut self) {
        let logarithmic_thousand = 3.0 * 10.0_f64.ln();
        for index in 0..N {
            let mode = index as f64 + 1.0 + self.configuration.boundary_phase;
            let metric_factor =
                (1.0 + self.configuration.dispersion * mode * mode).max(MIN_METRIC_FACTOR);
            let frequency_hz = self.reference_frequency_hz * mode * metric_factor.sqrt()
                / self.configuration.length_ratio;
            let omega = TAU * frequency_hz;
            self.angular_frequency[index] = omega;
            let angle = omega / self.sample_rate_hz;
            self.cosine[index] = angle.cos();
            self.sine[index] = angle.sin();
            let position = if N == 1 {
                0.0
            } else {
                index as f64 / (N - 1) as f64
            };
            let t60 = self.configuration.low_mode_t60_seconds
                + position
                    * (self.configuration.high_mode_t60_seconds
                        - self.configuration.low_mode_t60_seconds);
            self.retain[index] =
                (-logarithmic_thousand / (t60.max(0.001) * self.sample_rate_hz)).exp();
        }
    }
}

fn field_energy<const N: usize>(
    state: &WaveFieldState<N>,
    angular_frequency: &[f64; N],
    dt: f64,
) -> f64 {
    0.5 * dt
        * (0..N)
            .map(|index| {
                state.momentum[index] * state.momentum[index]
                    + (angular_frequency[index] * state.displacement[index]).powi(2)
            })
            .sum::<f64>()
}

fn state_distance<const N: usize>(left: &WaveFieldState<N>, right: &WaveFieldState<N>) -> f64 {
    ((0..N)
        .map(|index| {
            (left.displacement[index] - right.displacement[index]).powi(2)
                + (left.momentum[index] - right.momentum[index]).powi(2)
        })
        .sum::<f64>())
    .sqrt()
}

fn normalized_direction<const N: usize>(
    direction: &[f64; N],
) -> Result<[f64; N], MorphingFieldError> {
    if direction.iter().any(|value| !value.is_finite()) {
        return Err(MorphingFieldError::InvalidDirection);
    }
    let norm = direction.iter().map(|value| value * value).sum::<f64>().sqrt();
    if !norm.is_finite() || norm <= 1.0e-15 {
        return Err(MorphingFieldError::InvalidDirection);
    }
    Ok(direction.map(|value| value / norm))
}

#[inline]
fn dot<const N: usize>(left: &[f64; N], right: &[f64; N]) -> f64 {
    left.iter().zip(right).map(|(left, right)| left * right).sum()
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

    #[test]
    fn length_changes_frequency_inversely() {
        let short = MorphingWaveField::<8>::try_new(
            48_000.0,
            110.0,
            WaveFieldConfiguration::guitar(),
        )
        .unwrap();
        let mut long_configuration = WaveFieldConfiguration::guitar();
        long_configuration.length_ratio = 2.0;
        let long = MorphingWaveField::<8>::try_new(48_000.0, 110.0, long_configuration).unwrap();
        for (short_frequency, long_frequency) in
            short.frequencies_hz().iter().zip(long.frequencies_hz())
        {
            assert!((*short_frequency - 2.0 * long_frequency).abs() < 1.0e-11);
        }
    }

    #[test]
    fn configuration_change_books_exact_metric_work_without_moving_state() {
        let mut field = MorphingWaveField::<8>::try_new(
            48_000.0,
            110.0,
            WaveFieldConfiguration::guitar(),
        )
        .unwrap();
        let mut state = WaveFieldState::default();
        state.displacement[0] = 0.4;
        state.displacement[3] = -0.12;
        state.momentum[1] = 0.7;
        field.restore_state(state).unwrap();
        let frame = field
            .set_configuration(WaveFieldConfiguration::trombone())
            .unwrap();
        assert!(frame.control_work.abs() > 1.0e-12);
        assert_eq!(frame.state_displacement, 0.0);
        assert!((frame.energy_after - frame.energy_before - frame.control_work).abs() < 1.0e-18);
        assert_eq!(field.state(), state);
    }

    #[test]
    fn exact_modal_step_preserves_declared_energy() {
        let mut field = MorphingWaveField::<16>::try_new(
            96_000.0,
            82.4,
            WaveFieldConfiguration::guitar(),
        )
        .unwrap();
        let mut state = WaveFieldState::default();
        for index in 0..16 {
            state.displacement[index] = 0.003 * (index as f64 * 0.37).sin();
            state.momentum[index] = 0.2 * (index as f64 * 0.51).cos();
        }
        field.restore_state(state).unwrap();
        let frame = field.propagate();
        assert!(frame.metric_residual.abs() < 2.0e-14 * frame.energy_before.max(1.0));
    }

    #[test]
    fn passive_loss_and_radiation_close() {
        let mut field = MorphingWaveField::<8>::try_new(
            48_000.0,
            110.0,
            WaveFieldConfiguration::trombone(),
        )
        .unwrap();
        let mut state = WaveFieldState::default();
        state.momentum = [0.8, -0.4, 0.2, 0.1, -0.3, 0.5, 0.05, -0.2];
        field.restore_state(state).unwrap();
        let loss = field.damp();
        assert!(loss.internal_loss > 0.0);
        assert!(loss.closure_residual.abs() < 1.0e-18);
        let radiation = field.radiate(&[1.0; 8], 0.2).unwrap();
        assert!(radiation.radiated_energy > 0.0);
        assert!(radiation.closure_residual.abs() < 2.0e-18);
    }

    #[test]
    fn guitar_and_trombone_are_configurations_of_one_state_space() {
        let mut body = MorphingWaveField::<24>::try_new(
            96_000.0,
            116.5,
            WaveFieldConfiguration::guitar(),
        )
        .unwrap();
        let mut state = WaveFieldState::default();
        state.displacement[2] = 0.01;
        state.momentum[0] = 0.2;
        body.restore_state(state).unwrap();
        let guitar_frequencies = body.frequencies_hz();
        body.set_configuration(WaveFieldConfiguration::trombone())
            .unwrap();
        let trombone_frequencies = body.frequencies_hz();
        assert_eq!(body.state(), state);
        assert_ne!(guitar_frequencies, trombone_frequencies);
    }

    #[test]
    fn invalid_configuration_and_direction_are_rejected() {
        let mut invalid = WaveFieldConfiguration::default();
        invalid.length_ratio = 0.0;
        assert!(MorphingWaveField::<8>::try_new(48_000.0, 110.0, invalid).is_err());
        let mut field = MorphingWaveField::<8>::try_new(
            48_000.0,
            110.0,
            WaveFieldConfiguration::default(),
        )
        .unwrap();
        assert!(field.radiate(&[0.0; 8], 0.5).is_err());
        assert!(field.radiate(&[1.0; 8], 1.1).is_err());
    }
}