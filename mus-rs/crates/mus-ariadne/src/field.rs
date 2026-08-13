//! Arbitrary positive modal metric for the same-body instrument challenge.
//!
//! The earlier morphing field deliberately used one near-harmonic ladder. That
//! is sufficient to prove string/bore reachability, but not to represent a
//! six-course guitar honestly. This field keeps one fixed `(q,p)` state while
//! allowing its positive modal metric to be configured as six four-partial
//! string courses or one 24-partial brass bore. Configuration work is exactly
//! the change in stored metric energy at fixed state.

use std::f64::consts::TAU;

pub const COURSE_BORE_MODES: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModalMetricConfiguration<const N: usize> {
    pub frequency_hz: [f64; N],
    pub t60_seconds: [f64; N],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricFieldState<const N: usize> {
    pub displacement: [f64; N],
    pub momentum: [f64; N],
}

impl<const N: usize> Default for MetricFieldState<N> {
    fn default() -> Self {
        Self {
            displacement: [0.0; N],
            momentum: [0.0; N],
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MetricConfigurationWorkFrame {
    pub energy_before: f64,
    pub energy_after: f64,
    pub control_work: f64,
    pub state_displacement: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MetricPropagationFrame {
    pub energy_before: f64,
    pub energy_after: f64,
    pub residual: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MetricLossFrame {
    pub energy_before: f64,
    pub energy_after: f64,
    pub loss: f64,
    pub residual: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MetricRadiationFrame {
    pub emitted_coordinate: f64,
    pub radiation: f64,
    pub energy_before: f64,
    pub energy_after: f64,
    pub residual: f64,
}

#[derive(Debug, Clone)]
pub struct ModalMetricWaveField<const N: usize> {
    sample_rate: f64,
    dt: f64,
    configuration: ModalMetricConfiguration<N>,
    state: MetricFieldState<N>,
    omega: [f64; N],
    cosine: [f64; N],
    sine: [f64; N],
    retain: [f64; N],
}

impl<const N: usize> ModalMetricWaveField<N> {
    pub fn try_new(
        sample_rate: f64,
        configuration: ModalMetricConfiguration<N>,
    ) -> Result<Self, &'static str> {
        if N == 0 || !sample_rate.is_finite() || !(8_000.0..=384_000.0).contains(&sample_rate) {
            return Err("invalid modal metric dimensions or sample rate");
        }
        validate(configuration, sample_rate)?;
        let mut result = Self {
            sample_rate,
            dt: sample_rate.recip(),
            configuration,
            state: MetricFieldState::default(),
            omega: [0.0; N],
            cosine: [1.0; N],
            sine: [0.0; N],
            retain: [1.0; N],
        };
        result.rebuild();
        Ok(result)
    }

    #[must_use]
    pub fn configuration(&self) -> ModalMetricConfiguration<N> {
        self.configuration
    }

    #[must_use]
    pub fn frequencies_hz(&self) -> [f64; N] {
        self.configuration.frequency_hz
    }

    #[must_use]
    pub fn state(&self) -> MetricFieldState<N> {
        self.state
    }

    pub fn restore_state(&mut self, state: MetricFieldState<N>) -> Result<(), &'static str> {
        if state
            .displacement
            .iter()
            .chain(state.momentum.iter())
            .any(|value| !value.is_finite())
        {
            return Err("modal metric state must be finite");
        }
        self.state = state;
        Ok(())
    }

    #[must_use]
    pub fn stored_energy(&self) -> f64 {
        0.5 * self.dt
            * (0..N)
                .map(|index| {
                    self.state.momentum[index] * self.state.momentum[index]
                        + (self.omega[index] * self.state.displacement[index]).powi(2)
                })
                .sum::<f64>()
    }

    pub fn set_configuration(
        &mut self,
        configuration: ModalMetricConfiguration<N>,
    ) -> Result<MetricConfigurationWorkFrame, &'static str> {
        validate(configuration, self.sample_rate)?;
        let state_before = self.state;
        let energy_before = self.stored_energy();
        self.configuration = configuration;
        self.rebuild();
        let energy_after = self.stored_energy();
        Ok(MetricConfigurationWorkFrame {
            energy_before,
            energy_after,
            control_work: energy_after - energy_before,
            state_displacement: distance(&state_before, &self.state),
        })
    }

    pub fn propagate(&mut self) -> MetricPropagationFrame {
        let energy_before = self.stored_energy();
        for index in 0..N {
            let displacement = self.state.displacement[index];
            let momentum = self.state.momentum[index];
            self.state.displacement[index] =
                self.cosine[index] * displacement + self.sine[index] * momentum / self.omega[index];
            self.state.momentum[index] =
                self.cosine[index] * momentum - self.sine[index] * self.omega[index] * displacement;
        }
        let energy_after = self.stored_energy();
        MetricPropagationFrame {
            energy_before,
            energy_after,
            residual: energy_after - energy_before,
        }
    }

    pub fn damp(&mut self) -> MetricLossFrame {
        let energy_before = self.stored_energy();
        for index in 0..N {
            self.state.displacement[index] *= self.retain[index];
            self.state.momentum[index] *= self.retain[index];
        }
        let energy_after = self.stored_energy();
        let loss = (energy_before - energy_after).max(0.0);
        MetricLossFrame {
            energy_before,
            energy_after,
            loss,
            residual: energy_after - energy_before + loss,
        }
    }

    /// Remove one normalized material-flow projection as radiation. Potential
    /// energy is untouched; the removed kinetic component is booked exactly.
    pub fn radiate(
        &mut self,
        direction: &[f64; N],
        fraction: f64,
    ) -> Result<MetricRadiationFrame, &'static str> {
        if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
            return Err("radiation fraction must be finite in [0,1]");
        }
        let direction = normalized_direction(direction)?;
        let energy_before = self.stored_energy();
        let projection = dot(&self.state.momentum, &direction);
        let emitted_coordinate = fraction.sqrt() * projection;
        let removal = 1.0 - (1.0 - fraction).sqrt();
        for (momentum, direction) in self.state.momentum.iter_mut().zip(direction) {
            *momentum -= removal * direction * projection;
        }
        let energy_after = self.stored_energy();
        let radiation = 0.5 * self.dt * emitted_coordinate * emitted_coordinate;
        Ok(MetricRadiationFrame {
            emitted_coordinate,
            radiation,
            energy_before,
            energy_after,
            residual: energy_after - energy_before + radiation,
        })
    }

    /// Normalized generalized effort conjugate to a finite-aperture momentum
    /// projection. This is used for compliant-boundary synchronization only;
    /// it is not itself an unbooked source of work.
    pub fn aperture_effort(&self, direction: &[f64; N]) -> Result<f64, &'static str> {
        let direction = normalized_direction(direction)?;
        Ok((0..N)
            .map(|index| direction[index] * self.omega[index] * self.state.displacement[index])
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

    fn rebuild(&mut self) {
        for index in 0..N {
            self.omega[index] = TAU * self.configuration.frequency_hz[index];
            let angle = self.omega[index] / self.sample_rate;
            self.cosine[index] = angle.cos();
            self.sine[index] = angle.sin();
            self.retain[index] = (-3.0 * 10.0_f64.ln()
                / (self.configuration.t60_seconds[index] * self.sample_rate))
                .exp();
        }
    }
}

/// Six EADGBE courses, four retained partials per course. The tiny positive
/// inharmonicity is a material coordinate, not an output coloration stage.
#[must_use]
pub fn six_course_guitar_metric() -> ModalMetricConfiguration<COURSE_BORE_MODES> {
    let fundamentals = [
        82.406_889_228,
        110.0,
        146.832_383_959,
        195.997_717_991,
        246.941_650_628,
        329.627_556_913,
    ];
    let mut frequency_hz = [0.0; COURSE_BORE_MODES];
    let mut t60_seconds = [0.0; COURSE_BORE_MODES];
    for (course, fundamental) in fundamentals.iter().enumerate() {
        for partial in 0..4 {
            let index = 4 * course + partial;
            let harmonic = partial as f64 + 1.0;
            frequency_hz[index] =
                fundamental * harmonic * (1.0 + 1.5e-5 * harmonic * harmonic).sqrt();
            t60_seconds[index] =
                (2.8 - 0.30 * course as f64) * (1.0 - 0.18 * partial as f64).max(0.25);
        }
    }
    ModalMetricConfiguration {
        frequency_hz,
        t60_seconds,
    }
}

/// One B-flat trombone-like near-harmonic bore, retaining 24 partials.
#[must_use]
pub fn trombone_bore_metric() -> ModalMetricConfiguration<COURSE_BORE_MODES> {
    let fundamental = 116.540_940_380;
    let frequency_hz = std::array::from_fn(|index| {
        let harmonic = index as f64 + 1.0;
        fundamental * harmonic * (1.0 + 1.9e-5 * harmonic * harmonic).sqrt()
    });
    let t60_seconds = std::array::from_fn(|index| {
        let position = index as f64 / (COURSE_BORE_MODES - 1) as f64;
        3.2 + position * (0.58 - 3.2)
    });
    ModalMetricConfiguration {
        frequency_hz,
        t60_seconds,
    }
}

/// Positive log-frequency interpolation gives a smooth metric path without
/// assigning any audio crossfade semantics to the path itself.
#[must_use]
pub fn interpolate_metric(
    left: ModalMetricConfiguration<COURSE_BORE_MODES>,
    right: ModalMetricConfiguration<COURSE_BORE_MODES>,
    amount: f64,
) -> ModalMetricConfiguration<COURSE_BORE_MODES> {
    let amount = if amount.is_finite() {
        amount.clamp(0.0, 1.0)
    } else {
        0.0
    };
    ModalMetricConfiguration {
        frequency_hz: std::array::from_fn(|index| {
            (left.frequency_hz[index].ln()
                + amount * (right.frequency_hz[index].ln() - left.frequency_hz[index].ln()))
            .exp()
        }),
        t60_seconds: std::array::from_fn(|index| {
            left.t60_seconds[index] + amount * (right.t60_seconds[index] - left.t60_seconds[index])
        }),
    }
}

fn validate<const N: usize>(
    configuration: ModalMetricConfiguration<N>,
    sample_rate: f64,
) -> Result<(), &'static str> {
    for index in 0..N {
        let frequency = configuration.frequency_hz[index];
        let t60 = configuration.t60_seconds[index];
        if !frequency.is_finite() || frequency <= 0.0 || frequency >= 0.48 * sample_rate {
            return Err("invalid modal metric frequency");
        }
        if !t60.is_finite() || t60 <= 0.0 {
            return Err("invalid modal metric loss time");
        }
    }
    Ok(())
}

fn normalized_direction<const N: usize>(direction: &[f64; N]) -> Result<[f64; N], &'static str> {
    if direction.iter().any(|value| !value.is_finite()) {
        return Err("aperture direction must be finite");
    }
    let norm = direction
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm <= 1.0e-12 {
        return Err("aperture direction must be nondegenerate");
    }
    Ok(direction.map(|value| value / norm))
}

fn dot<const N: usize>(left: &[f64; N], right: &[f64; N]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn distance<const N: usize>(left: &MetricFieldState<N>, right: &MetricFieldState<N>) -> f64 {
    (0..N)
        .map(|index| {
            (left.displacement[index] - right.displacement[index]).powi(2)
                + (left.momentum[index] - right.momentum[index]).powi(2)
        })
        .sum::<f64>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guitar_really_contains_six_simultaneous_courses() {
        let metric = six_course_guitar_metric();
        let fundamentals = [
            82.406_889_228,
            110.0,
            146.832_383_959,
            195.997_717_991,
            246.941_650_628,
            329.627_556_913,
        ];
        for (course, fundamental) in fundamentals.iter().enumerate() {
            let measured = metric.frequency_hz[4 * course];
            assert!((measured / fundamental - 1.0).abs() < 2.0e-5);
        }
    }

    #[test]
    fn guitar_and_trombone_are_metrics_on_exactly_the_same_state() {
        let mut body = ModalMetricWaveField::try_new(96_000.0, six_course_guitar_metric()).unwrap();
        let mut state = MetricFieldState::default();
        state.displacement[0] = 0.01;
        state.displacement[7] = -0.004;
        state.momentum[13] = 0.3;
        body.restore_state(state).unwrap();
        let frame = body.set_configuration(trombone_bore_metric()).unwrap();
        assert_eq!(body.state(), state);
        assert_eq!(frame.state_displacement, 0.0);
        assert!((frame.energy_after - frame.energy_before - frame.control_work).abs() < 1.0e-18);
    }

    #[test]
    fn interpolation_never_leaves_positive_metric_cone() {
        for step in 0..=100 {
            let metric = interpolate_metric(
                six_course_guitar_metric(),
                trombone_bore_metric(),
                step as f64 / 100.0,
            );
            assert!(metric.frequency_hz.iter().all(|frequency| *frequency > 0.0));
            assert!(metric.t60_seconds.iter().all(|t60| *t60 > 0.0));
        }
    }

    #[test]
    fn finite_aperture_radiation_closes() {
        let mut body = ModalMetricWaveField::try_new(96_000.0, six_course_guitar_metric()).unwrap();
        let state = MetricFieldState {
            momentum: std::array::from_fn(|index| 0.1 * (index as f64 * 0.31).cos()),
            ..Default::default()
        };
        body.restore_state(state).unwrap();
        let frame = body.radiate(&[1.0; COURSE_BORE_MODES], 0.2).unwrap();
        assert!(frame.radiation > 0.0);
        assert!(frame.residual.abs() < 3.0e-18);
    }
}
