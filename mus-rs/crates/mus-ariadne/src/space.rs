//! Reciprocal delayed space for the canonical Ariadne world.
//!
//! Space is another persistent body. Delay-course heads exchange momentum with
//! the live material through metric-compatible Givens factors, scatter through
//! a Householder reflection, lose energy explicitly, and radiate through two
//! finite apertures. Construction allocates the delay storage; the sample path
//! allocates and locks nothing.

use mus_watsn::givens;
use std::f64::consts::TAU;

pub use crate::ir::SpaceSpec;

const MIN_SAMPLE_RATE_HZ: f64 = 8_000.0;
const MAX_SAMPLE_RATE_HZ: f64 = 384_000.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpaceControls {
    pub coupling: f64,
    pub decay: f64,
    pub radiation: f64,
}

impl Default for SpaceControls {
    fn default() -> Self {
        Self {
            coupling: 0.18,
            decay: 0.42,
            radiation: 0.16,
        }
    }
}

impl SpaceControls {
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            coupling: finite(self.coupling).clamp(0.0, 1.0),
            decay: finite(self.decay).clamp(0.0, 1.0),
            radiation: finite(self.radiation).clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpaceFrame {
    pub emitted_left: f64,
    pub emitted_right: f64,
    pub exchange: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    pub coupling_metric_residual: f64,
    pub space_closure_residual: f64,
    pub activity_before: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpaceError {
    EmptyWorld,
    InvalidSampleRate,
    InvalidDelay { course: usize },
    InvalidDirection { name: &'static str, index: usize },
}

impl std::fmt::Display for SpaceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyWorld => formatter.write_str("Ariadne space needs body and delay coordinates"),
            Self::InvalidSampleRate => formatter.write_str("invalid Ariadne space sample rate"),
            Self::InvalidDelay { course } => write!(formatter, "invalid delay for space course {course}"),
            Self::InvalidDirection { name, index } => {
                write!(formatter, "invalid {name} direction at index {index}")
            }
        }
    }
}

impl std::error::Error for SpaceError {}

#[derive(Debug, Clone)]
struct DelayCourse {
    data: Vec<f64>,
    index: usize,
}

impl DelayCourse {
    fn new(length: usize) -> Self {
        Self {
            data: vec![0.0; length.max(1)],
            index: 0,
        }
    }

    #[inline]
    fn read(&self) -> f64 {
        self.data[self.index]
    }

    #[inline]
    fn write_advance(&mut self, value: f64) {
        self.data[self.index] = value;
        self.index += 1;
        if self.index == self.data.len() {
            self.index = 0;
        }
    }

    fn reset(&mut self) {
        self.data.fill(0.0);
        self.index = 0;
    }
}

/// Fixed-capacity reciprocal delayed field coupled to an `N`-coordinate body.
#[derive(Debug, Clone)]
pub struct ReciprocalSpace<const N: usize, const C: usize> {
    sample_rate_hz: f64,
    dt: f64,
    spec: SpaceSpec,
    body_directions: [[f64; N]; C],
    coupling_weights: [f64; C],
    scattering_direction: [f64; C],
    radiation_left: [f64; C],
    radiation_right: [f64; C],
    lines: [DelayCourse; C],
    stored_energy: f64,
    delay_cell_count: usize,
}

impl<const N: usize, const C: usize> ReciprocalSpace<N, C> {
    pub fn try_new(sample_rate_hz: f64, spec: SpaceSpec) -> Result<Self, SpaceError> {
        if N == 0 || C == 0 {
            return Err(SpaceError::EmptyWorld);
        }
        if !sample_rate_hz.is_finite()
            || !(MIN_SAMPLE_RATE_HZ..=MAX_SAMPLE_RATE_HZ).contains(&sample_rate_hz)
        {
            return Err(SpaceError::InvalidSampleRate);
        }
        if C != crate::ir::SPACE_COURSES || N != crate::ir::MODE_COUNT {
            return Err(SpaceError::EmptyWorld);
        }

        let mut body_directions = std::array::from_fn(|course| {
            std::array::from_fn(|mode| spec.body_directions[course][mode])
        });
        for (course, direction) in body_directions.iter_mut().enumerate() {
            normalize(direction).map_err(|_| SpaceError::InvalidDirection {
                name: "body",
                index: course,
            })?;
        }

        let coupling_weights = std::array::from_fn(|index| finite(spec.coupling_weights[index]));
        let mut scattering_direction = std::array::from_fn(|index| spec.scattering_direction[index]);
        normalize(&mut scattering_direction).map_err(|_| SpaceError::InvalidDirection {
            name: "scattering",
            index: 0,
        })?;
        let mut radiation_left = std::array::from_fn(|index| spec.radiation_left[index]);
        normalize(&mut radiation_left).map_err(|_| SpaceError::InvalidDirection {
            name: "left radiation",
            index: 0,
        })?;
        let mut radiation_right = std::array::from_fn(|index| spec.radiation_right[index]);
        let left_projection = dot(&radiation_right, &radiation_left);
        for index in 0..C {
            radiation_right[index] -= left_projection * radiation_left[index];
        }
        normalize(&mut radiation_right).map_err(|_| SpaceError::InvalidDirection {
            name: "right radiation",
            index: 0,
        })?;

        let lines = std::array::from_fn(|course| {
            let delay = spec.delays_seconds[course];
            DelayCourse::new((delay * sample_rate_hz).round().max(1.0) as usize)
        });
        for (course, delay) in spec.delays_seconds.iter().copied().enumerate() {
            if !delay.is_finite() || !(0.000_25..=2.0).contains(&delay) {
                return Err(SpaceError::InvalidDelay { course });
            }
        }
        let delay_cell_count = lines.iter().map(|line| line.data.len()).sum();

        Ok(Self {
            sample_rate_hz,
            dt: sample_rate_hz.recip(),
            spec,
            body_directions,
            coupling_weights,
            scattering_direction,
            radiation_left,
            radiation_right,
            lines,
            stored_energy: 0.0,
            delay_cell_count,
        })
    }

    #[must_use]
    pub fn sample_rate_hz(&self) -> f64 {
        self.sample_rate_hz
    }

    #[must_use]
    pub fn spec(&self) -> SpaceSpec {
        self.spec
    }

    #[must_use]
    pub fn stored_energy(&self) -> f64 {
        self.stored_energy
    }

    #[must_use]
    pub fn delay_cell_count(&self) -> usize {
        self.delay_cell_count
    }

    #[must_use]
    pub fn heap_bytes(&self) -> usize {
        self.delay_cell_count * std::mem::size_of::<f64>()
    }

    #[must_use]
    pub fn activity(&self) -> f64 {
        if self.delay_cell_count == 0 {
            0.0
        } else {
            (2.0 * self.stored_energy / (self.dt * self.delay_cell_count as f64))
                .max(0.0)
                .sqrt()
        }
    }

    pub fn reset(&mut self) {
        for line in &mut self.lines {
            line.reset();
        }
        self.stored_energy = 0.0;
    }

    pub fn process(&mut self, body_momentum: &mut [f64; N], controls: SpaceControls) -> SpaceFrame {
        let controls = controls.sanitized();
        let space_before = self.stored_energy;
        let activity_before = self.activity();
        let mut heads = std::array::from_fn::<_, C, _>(|index| self.lines[index].read());
        let heads_before = heads;
        let body_before = quadratic_energy(body_momentum, self.dt);
        let head_before = quadratic_energy(&heads, self.dt);

        let coupling_hz = 2.0 + 118.0 * controls.coupling * controls.coupling;
        let base_angle = TAU * coupling_hz / self.sample_rate_hz;
        for course in 0..C {
            let projection = dot(body_momentum, &self.body_directions[course]);
            let (next_projection, next_head) = givens(
                projection,
                heads[course],
                base_angle * self.coupling_weights[course],
            );
            add_projection(
                body_momentum,
                &self.body_directions[course],
                next_projection - projection,
            );
            heads[course] = next_head;
        }
        let exchange = quadratic_energy(&heads, self.dt) - head_before;
        let coupling_metric_residual =
            quadratic_energy(body_momentum, self.dt) - body_before + exchange;

        let scatter_projection = dot(&heads, &self.scattering_direction);
        for index in 0..C {
            heads[index] -= 2.0 * self.scattering_direction[index] * scatter_projection;
        }

        let before_loss = quadratic_energy(&heads, self.dt);
        let decay_seconds = 0.12 + 7.88 * controls.decay * controls.decay;
        for index in 0..C {
            let retain = (-3.0 * 10.0_f64.ln() * self.spec.delays_seconds[index]
                / decay_seconds)
                .exp();
            heads[index] *= retain;
        }
        let internal_loss =
            (before_loss - quadratic_energy(&heads, self.dt)).max(0.0);

        let fraction = (0.000_5 + 0.024 * controls.radiation * controls.radiation).min(0.08);
        let left_projection = dot(&heads, &self.radiation_left);
        let right_projection = dot(&heads, &self.radiation_right);
        let emitted_scale = fraction.sqrt();
        let emitted_left = emitted_scale * left_projection;
        let emitted_right = emitted_scale * right_projection;
        let removal = 1.0 - (1.0 - fraction).sqrt();
        for index in 0..C {
            heads[index] -= removal
                * (self.radiation_left[index] * left_projection
                    + self.radiation_right[index] * right_projection);
        }
        let radiation = 0.5
            * self.dt
            * (emitted_left * emitted_left + emitted_right * emitted_right);

        self.stored_energy +=
            quadratic_energy(&heads, self.dt) - quadratic_energy(&heads_before, self.dt);
        if self.stored_energy.abs() < 1.0e-20 {
            self.stored_energy = 0.0;
        }
        for index in 0..C {
            self.lines[index].write_advance(heads[index]);
        }
        let space_closure_residual =
            self.stored_energy - space_before - exchange + internal_loss + radiation;

        SpaceFrame {
            emitted_left,
            emitted_right,
            exchange,
            internal_loss,
            radiation,
            coupling_metric_residual,
            space_closure_residual,
            activity_before,
        }
    }
}

#[inline]
fn quadratic_energy<const N: usize>(state: &[f64; N], dt: f64) -> f64 {
    0.5 * dt * state.iter().map(|value| value * value).sum::<f64>()
}

#[inline]
fn dot<const N: usize>(left: &[f64; N], right: &[f64; N]) -> f64 {
    left.iter().zip(right).map(|(left, right)| left * right).sum()
}

#[inline]
fn add_projection<const N: usize>(state: &mut [f64; N], direction: &[f64; N], amount: f64) {
    for (value, basis) in state.iter_mut().zip(direction) {
        *value += amount * basis;
    }
}

fn normalize<const N: usize>(values: &mut [f64; N]) -> Result<(), ()> {
    let norm = values.iter().map(|value| value * value).sum::<f64>().sqrt();
    if !norm.is_finite() || norm <= 1.0e-12 {
        return Err(());
    }
    for value in values {
        *value /= norm;
    }
    Ok(())
}

#[inline]
fn finite(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::WorldSpec;

    #[test]
    fn reciprocal_exchange_and_space_balance_close() {
        let spec = WorldSpec::string_brass_v1();
        let mut space = ReciprocalSpace::<{ crate::ir::MODE_COUNT }, { crate::ir::SPACE_COURSES }>::try_new(
            spec.internal_sample_rate_hz,
            spec.space,
        )
        .unwrap();
        let mut body = [0.0; crate::ir::MODE_COUNT];
        body[0] = 0.8;
        body[3] = -0.3;
        body[8] = 0.2;
        let frame = space.process(
            &mut body,
            SpaceControls {
                coupling: 0.9,
                decay: 0.8,
                radiation: 0.7,
            },
        );
        assert!(frame.exchange.abs() > 0.0);
        assert!(frame.coupling_metric_residual.abs() < 5.0e-17);
        assert!(frame.space_closure_residual.abs() < 5.0e-17);
    }

    #[test]
    fn sample_path_keeps_delay_capacity_fixed() {
        let spec = WorldSpec::string_brass_v1();
        let mut space = ReciprocalSpace::<{ crate::ir::MODE_COUNT }, { crate::ir::SPACE_COURSES }>::try_new(
            spec.internal_sample_rate_hz,
            spec.space,
        )
        .unwrap();
        let cells = space.delay_cell_count();
        let bytes = space.heap_bytes();
        let mut body = [0.0; crate::ir::MODE_COUNT];
        for _ in 0..1_000 {
            space.process(&mut body, SpaceControls::default());
        }
        assert_eq!(space.delay_cell_count(), cells);
        assert_eq!(space.heap_bytes(), bytes);
    }
}
