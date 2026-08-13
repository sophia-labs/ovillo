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
        Self { displacement: [0.0; N], momentum: [0.0; N] }
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
    pub fn configuration(&self) -> ModalMetricConfiguration<N> { self.configuration }

    #[must_use]
    pub fn state(&self) -> MetricFieldState<N> { self.state }

    pub fn restore_state(&mut self, state: MetricFieldState<N>) -> Result<(), &'static str> {
        if state.displacement.iter().chain(state.momentum.iter()).any(|value| !value.is_finite()) {
            return Err("modal metric state must be finite");
        }
        self.state = state;
        Ok(())
    }

    #[must_use]
    pub fn stored_energy(&self) -> f64 {
        0.5 * self.dt * (0..N).map(|i| {
            self.state.momentum[i] * self.state.momentum[i]
                + (self.omega[i] * self.state.displacement[i]).powi(2)
        }).sum::<f64>()
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
        for i in 0..N {
            let q = self.state.displacement[i];
            let p = self.state.momentum[i];
            self.state.displacement[i] = self.cosine[i] * q + self.sine[i] * p / self.omega[i];
            self.state.momentum[i] = self.cosine[i] * p - self.sine[i] * self.omega[i] * q;
        }
        let energy_after = self.stored_energy();
        MetricPropagationFrame { energy_before, energy_after, residual: energy_after - energy_before }
    }

    pub fn damp(&mut self) -> MetricLossFrame {
        let energy_before = self.stored_energy();
        for i in 0..N {
            self.state.displacement[i] *= self.retain[i];
            self.state.momentum[i] *= self.retain[i];
        }
        let energy_after = self.stored_energy();
        let loss = (energy_before - energy_after).max(0.0);
        MetricLossFrame { energy_before, energy_after, loss, residual: energy_after - energy_before + loss }
    }

    #[must_use]
    pub fn displacement(&self) -> &[f64; N] { &self.state.displacement }

    #[must_use]
    pub fn momentum(&self) -> &[f64; N] { &self.state.momentum }

    pub fn momentum_mut(&mut self) -> &mut [f64; N] { &mut self.state.momentum }

    fn rebuild(&mut self) {
        for i in 0..N {
            self.omega[i] = TAU * self.configuration.frequency_hz[i];
            let angle = self.omega[i] / self.sample_rate;
            self.cosine[i] = angle.cos();
            self.sine[i] = angle.sin();
            self.retain[i] = (-3.0 * 10.0_f64.ln()
                / (self.configuration.t60_seconds[i] * self.sample_rate)).exp();
        }
    }
}

/// Six EADGBE courses, four retained partials per course. The tiny positive
/// inharmonicity is a material coordinate, not an output coloration stage.
#[must_use]
pub fn six_course_guitar_metric() -> ModalMetricConfiguration<COURSE_BORE_MODES> {
    let fundamentals = [
        82.406_889_228, 110.0, 146.832_383_959, 195.997_717_991,
        246.941_650_628, 329.627_556_913,
    ];
    let mut frequency_hz = [0.0; COURSE_BORE_MODES];
    let mut t60_seconds = [0.0; COURSE_BORE_MODES];
    for course in 0..6 {
        for partial in 0..4 {
            let index = 4 * course + partial;
            let harmonic = partial as f64 + 1.0;
            frequency_hz[index] = fundamentals[course]
                * harmonic
                * (1.0 + 1.5e-5 * harmonic * harmonic).sqrt();
            t60_seconds[index] = (2.8 - 0.30 * course as f64)
                * (1.0 - 0.18 * partial as f64).max(0.25);
        }
    }
    ModalMetricConfiguration { frequency_hz, t60_seconds }
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
    ModalMetricConfiguration { frequency_hz, t60_seconds }
}

/// Positive log-frequency interpolation gives a smooth metric path without
/// assigning any audio crossfade semantics to the path itself.
#[must_use]
pub fn interpolate_metric(
    left: ModalMetricConfiguration<COURSE_BORE_MODES>,
    right: ModalMetricConfiguration<COURSE_BORE_MODES>,
    amount: f64,
) -> ModalMetricConfiguration<COURSE_BORE_MODES> {
    let amount = if amount.is_finite() { amount.clamp(0.0, 1.0) } else { 0.0 };
    ModalMetricConfiguration {
        frequency_hz: std::array::from_fn(|i| {
            (left.frequency_hz[i].ln() + amount * (right.frequency_hz[i].ln() - left.frequency_hz[i].ln())).exp()
        }),
        t60_seconds: std::array::from_fn(|i| {
            left.t60_seconds[i] + amount * (right.t60_seconds[i] - left.t60_seconds[i])
        }),
    }
}

fn validate<const N: usize>(configuration: ModalMetricConfiguration<N>, sample_rate: f64) -> Result<(), &'static str> {
    for i in 0..N {
        let frequency = configuration.frequency_hz[i];
        let t60 = configuration.t60_seconds[i];
        if !frequency.is_finite() || frequency <= 0.0 || frequency >= 0.48 * sample_rate {
            return Err("invalid modal metric frequency");
        }
        if !t60.is_finite() || t60 <= 0.0 { return Err("invalid modal metric loss time"); }
    }
    Ok(())
}

fn distance<const N: usize>(left: &MetricFieldState<N>, right: &MetricFieldState<N>) -> f64 {
    (0..N).map(|i| {
        (left.displacement[i] - right.displacement[i]).powi(2)
            + (left.momentum[i] - right.momentum[i]).powi(2)
    }).sum::<f64>().sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guitar_really_contains_six_simultaneous_courses() {
        let metric = six_course_guitar_metric();
        let fundamentals = [82.406_889_228, 110.0, 146.832_383_959, 195.997_717_991, 246.941_650_628, 329.627_556_913];
        for course in 0..6 {
            let measured = metric.frequency_hz[4 * course];
            assert!((measured / fundamentals[course] - 1.0).abs() < 2.0e-5);
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
            let metric = interpolate_metric(six_course_guitar_metric(), trombone_bore_metric(), step as f64 / 100.0);
            assert!(metric.frequency_hz.iter().all(|frequency| *frequency > 0.0));
            assert!(metric.t60_seconds.iter().all(|t60| *t60 > 0.0));
        }
    }
}