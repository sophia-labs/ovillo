//! Reciprocal delay-course room for the shared Ariadne body.
//!
//! Delay heads exchange momentum with the live body through Givens rotations,
//! then undergo orthogonal scattering, passive retention, and finite-aperture
//! radiation. The room is therefore part of the material graph, not post FX.

use mus_watsn::givens;
use std::f64::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoomHaloControls {
    pub coupling: f64,
    pub decay: f64,
    pub radiation: f64,
}

impl Default for RoomHaloControls {
    fn default() -> Self {
        Self { coupling: 0.18, decay: 0.42, radiation: 0.16 }
    }
}

impl RoomHaloControls {
    fn sanitized(self) -> Self {
        Self {
            coupling: finite(self.coupling).clamp(0.0, 1.0),
            decay: finite(self.decay).clamp(0.0, 1.0),
            radiation: finite(self.radiation).clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RoomHaloFrame {
    pub emitted_left: f64,
    pub emitted_right: f64,
    pub exchange: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    pub coupling_metric_residual: f64,
    pub room_closure_residual: f64,
    pub activity_before: f64,
}

#[derive(Debug, Clone)]
struct DelayCourse {
    data: Vec<f64>,
    index: usize,
}

impl DelayCourse {
    fn new(length: usize) -> Self {
        Self { data: vec![0.0; length.max(1)], index: 0 }
    }

    fn read(&self) -> f64 { self.data[self.index] }

    fn write_advance(&mut self, value: f64) {
        self.data[self.index] = value;
        self.index = (self.index + 1) % self.data.len();
    }

    fn reset(&mut self) {
        self.data.fill(0.0);
        self.index = 0;
    }
}

#[derive(Debug, Clone)]
pub struct ReciprocalRoomHalo<const N: usize, const C: usize> {
    sample_rate: f64,
    dt: f64,
    delays: [f64; C],
    directions: [[f64; N]; C],
    weights: [f64; C],
    scatter: [f64; C],
    left: [f64; C],
    right: [f64; C],
    lines: [DelayCourse; C],
    stored_energy: f64,
    cell_count: usize,
}

impl<const N: usize, const C: usize> ReciprocalRoomHalo<N, C> {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        sample_rate: f64,
        delays: [f64; C],
        mut directions: [[f64; N]; C],
        weights: [f64; C],
        mut scatter: [f64; C],
        mut left: [f64; C],
        mut right: [f64; C],
    ) -> Result<Self, &'static str> {
        if N == 0 || C == 0 || !sample_rate.is_finite() || !(8_000.0..=384_000.0).contains(&sample_rate) {
            return Err("invalid room dimensions or sample rate");
        }
        for delay in delays {
            if !delay.is_finite() || !(0.000_25..=2.0).contains(&delay) {
                return Err("invalid room delay");
            }
        }
        for direction in &mut directions { normalize(direction)?; }
        normalize(&mut scatter)?;
        normalize(&mut left)?;
        let projection = dot(&right, &left);
        for i in 0..C { right[i] -= projection * left[i]; }
        normalize(&mut right)?;

        let lines = std::array::from_fn(|i| DelayCourse::new((delays[i] * sample_rate).round() as usize));
        let cell_count = lines.iter().map(|line| line.data.len()).sum();
        Ok(Self {
            sample_rate,
            dt: sample_rate.recip(),
            delays,
            directions,
            weights: weights.map(finite),
            scatter,
            left,
            right,
            lines,
            stored_energy: 0.0,
            cell_count,
        })
    }

    pub fn reset(&mut self) {
        for line in &mut self.lines { line.reset(); }
        self.stored_energy = 0.0;
    }

    #[must_use]
    pub fn stored_energy(&self) -> f64 { self.stored_energy }

    #[must_use]
    pub fn activity(&self) -> f64 {
        (2.0 * self.stored_energy / (self.dt * self.cell_count as f64)).max(0.0).sqrt()
    }

    pub fn process(&mut self, body: &mut [f64; N], controls: RoomHaloControls) -> RoomHaloFrame {
        let controls = controls.sanitized();
        let room_before = self.stored_energy;
        let activity_before = self.activity();
        let mut heads = std::array::from_fn::<_, C, _>(|i| self.lines[i].read());
        let heads_before = heads;
        let body_before = energy(body, self.dt);
        let head_before = energy(&heads, self.dt);

        let angle = TAU * (2.0 + 118.0 * controls.coupling * controls.coupling) / self.sample_rate;
        for i in 0..C {
            let projection = dot(body, &self.directions[i]);
            let (next_projection, next_head) = givens(projection, heads[i], angle * self.weights[i]);
            add_projection(body, &self.directions[i], next_projection - projection);
            heads[i] = next_head;
        }
        let exchange = energy(&heads, self.dt) - head_before;
        let coupling_metric_residual = energy(body, self.dt) - body_before + exchange;

        let projection = dot(&heads, &self.scatter);
        for i in 0..C { heads[i] -= 2.0 * self.scatter[i] * projection; }

        let before_loss = energy(&heads, self.dt);
        let decay_seconds = 0.12 + 7.88 * controls.decay * controls.decay;
        for i in 0..C {
            heads[i] *= (-3.0 * 10.0_f64.ln() * self.delays[i] / decay_seconds).exp();
        }
        let internal_loss = (before_loss - energy(&heads, self.dt)).max(0.0);

        let eta = (0.000_5 + 0.024 * controls.radiation * controls.radiation).min(0.08);
        let lp = dot(&heads, &self.left);
        let rp = dot(&heads, &self.right);
        let emitted_left = eta.sqrt() * lp;
        let emitted_right = eta.sqrt() * rp;
        let removal = 1.0 - (1.0 - eta).sqrt();
        for i in 0..C { heads[i] -= removal * (self.left[i] * lp + self.right[i] * rp); }
        let radiation = 0.5 * self.dt * (emitted_left * emitted_left + emitted_right * emitted_right);

        self.stored_energy += energy(&heads, self.dt) - energy(&heads_before, self.dt);
        if self.stored_energy.abs() < 1.0e-20 { self.stored_energy = 0.0; }
        for i in 0..C { self.lines[i].write_advance(heads[i]); }
        let room_closure_residual = self.stored_energy - room_before - exchange + internal_loss + radiation;
        RoomHaloFrame {
            emitted_left,
            emitted_right,
            exchange,
            internal_loss,
            radiation,
            coupling_metric_residual,
            room_closure_residual,
            activity_before,
        }
    }
}

fn normalize<const N: usize>(values: &mut [f64; N]) -> Result<(), &'static str> {
    let norm = values.iter().map(|x| x * x).sum::<f64>().sqrt();
    if !norm.is_finite() || norm <= 1.0e-12 { return Err("degenerate room direction"); }
    for value in values { *value /= norm; }
    Ok(())
}

fn dot<const N: usize>(a: &[f64; N], b: &[f64; N]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn add_projection<const N: usize>(state: &mut [f64; N], direction: &[f64; N], amount: f64) {
    for (value, basis) in state.iter_mut().zip(direction) { *value += amount * basis; }
}

fn energy<const N: usize>(state: &[f64; N], dt: f64) -> f64 {
    0.5 * dt * state.iter().map(|x| x * x).sum::<f64>()
}

fn finite(value: f64) -> f64 { if value.is_finite() { value } else { 0.0 } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exchange_and_room_balance_close() {
        let directions = [[1.0, 0.0, 0.2, 0.0], [0.0, 1.0, 0.0, 0.2]];
        let mut room = ReciprocalRoomHalo::try_new(
            48_000.0,
            [0.0073, 0.0111],
            directions,
            [1.0, 0.8],
            [1.0, -0.7],
            [1.0, 0.3],
            [0.2, -0.9],
        ).unwrap();
        let mut body = [0.8, -0.3, 0.2, 0.1];
        let frame = room.process(&mut body, RoomHaloControls { coupling: 0.9, decay: 0.8, radiation: 0.7 });
        assert!(frame.exchange.abs() > 0.0);
        assert!(frame.coupling_metric_residual.abs() < 3.0e-18);
        assert!(frame.room_closure_residual.abs() < 3.0e-18);
    }
}