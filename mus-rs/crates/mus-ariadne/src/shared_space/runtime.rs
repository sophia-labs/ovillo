use std::f64::consts::TAU;

#[cfg(test)]
use mus_watsn::givens;

use crate::ir::{MODE_COUNT, SPACE_COURSES};
use crate::plan::{SharedSpaceSpec, SpacePortSpec};

use super::types::finite;
use super::{SharedSpaceControls, SharedSpaceError, SharedSpaceExchange, SharedSpaceFrame};

const MIN_SAMPLE_RATE_HZ: f64 = 8_000.0;
const MAX_SAMPLE_RATE_HZ: f64 = 384_000.0;

#[derive(Debug, Clone)]
struct DelayCourse {
    data: Vec<f64>,
    index: usize,
    /// Sum of squares of `data`, kept as cells are written and re-summed
    /// exactly once per lap, so the ledger reads the space's stored energy in
    /// O(1) instead of walking every delay cell twice per sample.
    sum_sq: f64,
}

impl DelayCourse {
    fn new(length: usize) -> Self {
        Self {
            data: vec![0.0; length.max(1)],
            index: 0,
            sum_sq: 0.0,
        }
    }

    #[inline]
    fn read(&self) -> f64 {
        self.data[self.index]
    }

    #[inline]
    fn write_advance(&mut self, value: f64) {
        let old = self.data[self.index];
        self.data[self.index] = value;
        self.sum_sq += value * value - old * old;
        self.index += 1;
        if self.index == self.data.len() {
            self.index = 0;
            self.sum_sq = self.data.iter().map(|value| value * value).sum();
        }
    }

    #[inline]
    fn sum_sq(&self) -> f64 {
        self.sum_sq.max(0.0)
    }
}

/// A body's port with its unit directions taken once at construction, and
/// each course's last rotation `(angle bits, sin, cos)`.
#[derive(Debug, Clone)]
pub(crate) struct CompiledSpacePort {
    directions: [[f64; MODE_COUNT]; SPACE_COURSES],
    weights: [f64; SPACE_COURSES],
    rotations: [(u64, f64, f64); SPACE_COURSES],
}

impl CompiledSpacePort {
    pub(crate) fn compile(port: &SpacePortSpec) -> Result<Self, SharedSpaceError> {
        let mut directions = [[0.0; MODE_COUNT]; SPACE_COURSES];
        for (course, (direction, &body)) in
            directions.iter_mut().zip(&port.body_directions).enumerate()
        {
            *direction = normalized(body).map_err(|_| SharedSpaceError::InvalidDirection {
                name: "body port",
                index: course,
            })?;
        }
        // NaN bits never equal a computed angle's: the first exchange computes every rotation.
        Ok(Self {
            directions,
            weights: port.coupling_weights,
            rotations: [(f64::NAN.to_bits(), 0.0, 1.0); SPACE_COURSES],
        })
    }
}

/// One fixed-capacity delayed field shared by all body ports in a `World`.
#[derive(Debug, Clone)]
pub(crate) struct SharedSpace {
    sample_rate_hz: f64,
    dt: f64,
    scattering_direction: [f64; SPACE_COURSES],
    radiation_left: [f64; SPACE_COURSES],
    radiation_right: [f64; SPACE_COURSES],
    lines: [DelayCourse; SPACE_COURSES],
    heads: [f64; SPACE_COURSES],
    frame_open: bool,
}

impl SharedSpace {
    pub(crate) fn try_new(
        sample_rate_hz: f64,
        spec: SharedSpaceSpec,
    ) -> Result<Self, SharedSpaceError> {
        if !sample_rate_hz.is_finite()
            || !(MIN_SAMPLE_RATE_HZ..=MAX_SAMPLE_RATE_HZ).contains(&sample_rate_hz)
        {
            return Err(SharedSpaceError::InvalidSampleRate);
        }
        let mut scattering_direction = spec.scattering_direction;
        normalize(&mut scattering_direction).map_err(|_| SharedSpaceError::InvalidDirection {
            name: "scattering",
            index: 0,
        })?;
        let mut radiation_left = spec.radiation_left;
        normalize(&mut radiation_left).map_err(|_| SharedSpaceError::InvalidDirection {
            name: "left radiation",
            index: 0,
        })?;
        let mut radiation_right = spec.radiation_right;
        let left_projection = dot(&radiation_right, &radiation_left);
        for course in 0..SPACE_COURSES {
            radiation_right[course] -= left_projection * radiation_left[course];
        }
        normalize(&mut radiation_right).map_err(|_| SharedSpaceError::InvalidDirection {
            name: "right radiation",
            index: 0,
        })?;

        for (course, delay) in spec.delays_seconds.iter().copied().enumerate() {
            if !delay.is_finite() || !(0.000_25..=2.0).contains(&delay) {
                return Err(SharedSpaceError::InvalidDelay { course });
            }
        }
        let lines = std::array::from_fn(|course| {
            DelayCourse::new((spec.delays_seconds[course] * sample_rate_hz).round() as usize)
        });
        Ok(Self {
            sample_rate_hz,
            dt: sample_rate_hz.recip(),
            scattering_direction,
            radiation_left,
            radiation_right,
            lines,
            heads: [0.0; SPACE_COURSES],
            frame_open: false,
        })
    }

    #[must_use]
    pub(crate) fn stored_energy(&self) -> f64 {
        0.5 * self.dt * self.lines.iter().map(DelayCourse::sum_sq).sum::<f64>()
    }

    #[cfg(test)]
    pub(crate) fn longest_line(&self) -> usize {
        self.lines
            .iter()
            .map(|line| line.data.len())
            .max()
            .unwrap_or(0)
    }

    /// The stored energy re-summed cell by cell, as it was computed before the
    /// running sums; tests hold [`Self::stored_energy`] to it.
    #[cfg(test)]
    pub(crate) fn stored_energy_exact(&self) -> f64 {
        0.5 * self.dt
            * self
                .lines
                .iter()
                .flat_map(|line| line.data.iter())
                .map(|value| value * value)
                .sum::<f64>()
    }

    #[must_use]
    pub(crate) fn activity(&self) -> f64 {
        (2.0 * self.stored_energy() / self.dt).sqrt()
    }

    #[must_use]
    pub(crate) fn delay_cell_count(&self) -> usize {
        self.lines.iter().map(|line| line.data.len()).sum()
    }

    /// Current live head of one delayed course. This is a non-loading readout;
    /// observation programs resolve the course index at construction.
    #[must_use]
    pub(crate) fn course_head(&self, index: usize) -> Option<f64> {
        self.heads.get(index).copied()
    }

    pub(crate) fn begin_frame(&mut self) -> Result<f64, SharedSpaceError> {
        if self.frame_open {
            return Err(SharedSpaceError::FrameAlreadyOpen);
        }
        self.heads = std::array::from_fn(|course| self.lines[course].read());
        if self.heads.iter().any(|value| !value.is_finite()) {
            return Err(SharedSpaceError::NonFiniteState);
        }
        self.frame_open = true;
        Ok(kinetic(&self.heads, self.dt))
    }

    /// The port exchange from the spec itself (the reference that
    /// `exchange_compiled_port` is checked against, bit for bit).
    #[cfg(test)]
    pub(crate) fn exchange_port(
        &mut self,
        body_momentum: &mut [f64; MODE_COUNT],
        port: &SpacePortSpec,
        controls: SharedSpaceControls,
    ) -> Result<SharedSpaceExchange, SharedSpaceError> {
        if !self.frame_open {
            return Err(SharedSpaceError::FrameNotOpen);
        }
        if body_momentum.iter().any(|value| !value.is_finite()) {
            return Err(SharedSpaceError::NonFiniteState);
        }
        let controls = controls.sanitized();
        let energy_before = kinetic(body_momentum, self.dt) + kinetic(&self.heads, self.dt);
        let mut signed_into_body = 0.0;
        let mut maximum_abs_angle = 0.0_f64;
        for course in 0..SPACE_COURSES {
            let direction = normalized(port.body_directions[course]).map_err(|_| {
                SharedSpaceError::InvalidDirection {
                    name: "body port",
                    index: course,
                }
            })?;
            let before = dot(body_momentum, &direction);
            let angle = TAU * (2.0 + 190.0 * controls.coupling * controls.coupling)
                / self.sample_rate_hz
                * finite(port.coupling_weights[course]);
            let (after, head_after) = givens(before, self.heads[course], angle);
            add_projection(body_momentum, &direction, after - before);
            self.heads[course] = head_after;
            signed_into_body += 0.5 * self.dt * (after * after - before * before);
            maximum_abs_angle = maximum_abs_angle.max(angle.abs());
        }
        let energy_after = kinetic(body_momentum, self.dt) + kinetic(&self.heads, self.dt);
        Ok(SharedSpaceExchange {
            signed_into_body,
            metric_residual: energy_after - energy_before,
            maximum_abs_angle,
        })
    }

    /// `exchange_port` with the port's unit directions normalized once and
    /// each course's rotation kept while its angle is unchanged: the same
    /// arithmetic in the same order, so every result is bit for bit
    /// `exchange_port`'s.
    pub(crate) fn exchange_compiled_port(
        &mut self,
        body_momentum: &mut [f64; MODE_COUNT],
        port: &mut CompiledSpacePort,
        controls: SharedSpaceControls,
    ) -> Result<SharedSpaceExchange, SharedSpaceError> {
        if !self.frame_open {
            return Err(SharedSpaceError::FrameNotOpen);
        }
        if body_momentum.iter().any(|value| !value.is_finite()) {
            return Err(SharedSpaceError::NonFiniteState);
        }
        let controls = controls.sanitized();
        let energy_before = kinetic(body_momentum, self.dt) + kinetic(&self.heads, self.dt);
        let mut signed_into_body = 0.0;
        let mut maximum_abs_angle = 0.0_f64;
        for course in 0..SPACE_COURSES {
            let direction = &port.directions[course];
            let before = dot(body_momentum, direction);
            let angle = TAU * (2.0 + 190.0 * controls.coupling * controls.coupling)
                / self.sample_rate_hz
                * finite(port.weights[course]);
            let rotation = &mut port.rotations[course];
            if rotation.0 != angle.to_bits() {
                let (sin_angle, cos_angle) = angle.sin_cos();
                *rotation = (angle.to_bits(), sin_angle, cos_angle);
            }
            let (_, sin_angle, cos_angle) = *rotation;
            let head = self.heads[course];
            let (after, head_after) = (
                cos_angle * before - sin_angle * head,
                sin_angle * before + cos_angle * head,
            );
            add_projection(body_momentum, direction, after - before);
            self.heads[course] = head_after;
            signed_into_body += 0.5 * self.dt * (after * after - before * before);
            maximum_abs_angle = maximum_abs_angle.max(angle.abs());
        }
        let energy_after = kinetic(body_momentum, self.dt) + kinetic(&self.heads, self.dt);
        Ok(SharedSpaceExchange {
            signed_into_body,
            metric_residual: energy_after - energy_before,
            maximum_abs_angle,
        })
    }

    pub(crate) fn finish_frame(
        &mut self,
        controls: SharedSpaceControls,
        exchange_into_bodies: f64,
        coupling_metric_residual: f64,
    ) -> Result<SharedSpaceFrame, SharedSpaceError> {
        if !self.frame_open {
            return Err(SharedSpaceError::FrameNotOpen);
        }
        self.frame_open = false;
        let controls = controls.sanitized();
        let activity_before = kinetic(&self.heads, self.dt);

        let projection = dot(&self.heads, &self.scattering_direction);
        for course in 0..SPACE_COURSES {
            self.heads[course] -= 2.0 * projection * self.scattering_direction[course];
        }
        let scattered_energy = kinetic(&self.heads, self.dt);

        let decay_seconds = 0.08 + 18.0 * controls.decay * controls.decay;
        let loss_gain = (-self.dt / decay_seconds).exp();
        let before_loss = scattered_energy;
        for head in &mut self.heads {
            *head *= loss_gain;
        }
        let after_loss = kinetic(&self.heads, self.dt);
        let internal_loss = before_loss - after_loss;

        let fraction = (0.000_05 + 0.018 * controls.radiation * controls.radiation).clamp(0.0, 1.0);
        let left_before = dot(&self.heads, &self.radiation_left);
        let right_before = dot(&self.heads, &self.radiation_right);
        let retained = (1.0 - fraction).sqrt();
        let left_after = retained * left_before;
        let right_after = retained * right_before;
        add_projection(
            &mut self.heads,
            &self.radiation_left,
            left_after - left_before,
        );
        add_projection(
            &mut self.heads,
            &self.radiation_right,
            right_after - right_before,
        );
        let emitted_left = fraction.sqrt() * left_before;
        let emitted_right = fraction.sqrt() * right_before;
        let radiation =
            0.5 * self.dt * (emitted_left * emitted_left + emitted_right * emitted_right);
        let after_radiation = kinetic(&self.heads, self.dt);

        for course in 0..SPACE_COURSES {
            self.lines[course].write_advance(self.heads[course]);
        }
        let closure_residual = after_radiation - (after_loss - radiation);
        if self.heads.iter().any(|value| !value.is_finite()) {
            return Err(SharedSpaceError::NonFiniteState);
        }
        Ok(SharedSpaceFrame {
            emitted_left,
            emitted_right,
            exchange_into_bodies,
            internal_loss,
            radiation,
            coupling_metric_residual,
            closure_residual,
            activity_before,
        })
    }
}

fn normalized<const N: usize>(mut values: [f64; N]) -> Result<[f64; N], ()> {
    normalize(&mut values)?;
    Ok(values)
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
fn dot<const N: usize>(left: &[f64; N], right: &[f64; N]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

#[inline]
fn kinetic<const N: usize>(values: &[f64; N], dt: f64) -> f64 {
    0.5 * dt * values.iter().map(|value| value * value).sum::<f64>()
}

#[inline]
fn add_projection<const N: usize>(state: &mut [f64; N], direction: &[f64; N], amount: f64) {
    for (value, basis) in state.iter_mut().zip(direction) {
        *value += amount * basis;
    }
}
