//! A felt damper resting on a wire: a viscous pad over a declared segment of
//! the speaking length.
//!
//! ```text
//! pad force per length   f(x) = -r v(x)  on [x_c - l/2, x_c + l/2],  R = r l
//! quadrature             p equal sub-segments, centres x_j, each R / p
//! point dashpot j        rate gamma_j = e (R / p) |psi(x_j)|^2  along  u_j = psi(x_j) / |psi(x_j)|
//! ```
//!
//! `psi(x)` are the wire's mass-normalized mode shapes, so the modal velocity
//! at `x_j` is `psi(x_j) . eta'` and Ariadne momentum is `eta' / sqrt(dt)`.
//! Each dashpot removes `1 - exp(-gamma_j dt)` of the momentum projected on
//! its own unit shape, one after the other, and the energy it removes,
//! `dt / 2 (P^2 - P'^2)`, is booked as internal loss. A dashpot only
//! dissipates, so changing `e` (how fully the felt rests on the wire, in
//! `[0, 1]`) needs no control work. A pad cannot damp a partial with a node
//! under it: that partial rings through, as on a real piano.

use serde::{Deserialize, Serialize};

/// Most quadrature points a pad may declare.
pub const MAX_DAMPER_POINTS: usize = 8;

/// Damper anatomy. Every number is the anatomy's to declare and label.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamperSpec {
    /// Pad centre, as a fraction of the speaking length.
    pub center_fraction: f64,
    /// Pad length along the wire.
    pub length_m: f64,
    /// Total viscous resistance of the pad when it rests fully (kg/s).
    pub resistance_kg_s: f64,
    /// Equal sub-segments the pad is integrated over (1..=8).
    pub points: usize,
}

impl DamperSpec {
    pub(crate) fn validate(&self, speaking_length_m: f64) -> Result<(), &'static str> {
        let half = 0.5 * self.length_m / speaking_length_m;
        if !self.center_fraction.is_finite()
            || !self.length_m.is_finite()
            || !self.resistance_kg_s.is_finite()
            || self.length_m <= 0.0
            || self.resistance_kg_s < 0.0
            || self.center_fraction - half <= 0.0
            || self.center_fraction + half >= 1.0
            || !(1..=MAX_DAMPER_POINTS).contains(&self.points)
        {
            return Err("damper pad must lie inside the wire with a finite non-negative resistance and 1..=8 points");
        }
        Ok(())
    }

    /// Centres of the pad's sub-segments, as fractions of the speaking length.
    #[must_use]
    pub fn point_fractions(&self, speaking_length_m: f64) -> Vec<f64> {
        let start = self.center_fraction - 0.5 * self.length_m / speaking_length_m;
        let step = self.length_m / speaking_length_m / self.points as f64;
        (0..self.points)
            .map(|j| start + (j as f64 + 0.5) * step)
            .collect()
    }
}

/// One damper on one modal field.
#[derive(Debug, Clone)]
pub struct FeltDamper<const N: usize> {
    directions: Vec<[f64; N]>,
    /// `gamma_j` at full engagement (1/s).
    rates: Vec<f64>,
    retain: Vec<f64>,
    dt: f64,
    engagement: f64,
}

impl<const N: usize> FeltDamper<N> {
    /// `shapes[j]` is the wire's mode shape at point `j` (1/sqrt(kg)).
    pub fn new(spec: &DamperSpec, shapes: &[[f64; N]], sample_rate_hz: f64) -> Self {
        let share = spec.resistance_kg_s / shapes.len().max(1) as f64;
        let mut directions = Vec::with_capacity(shapes.len());
        let mut rates = Vec::with_capacity(shapes.len());
        for shape in shapes {
            let norm2: f64 = shape.iter().map(|v| v * v).sum();
            if norm2 <= 0.0 || !norm2.is_finite() {
                continue;
            }
            let norm = norm2.sqrt();
            directions.push(shape.map(|v| v / norm));
            rates.push(share * norm2);
        }
        let retain = vec![1.0; rates.len()];
        Self {
            directions,
            rates,
            retain,
            dt: sample_rate_hz.recip(),
            engagement: 0.0,
        }
    }

    #[must_use]
    pub fn engagement(&self) -> f64 {
        self.engagement
    }

    /// Damping rate of each pad point at full engagement (1/s).
    #[must_use]
    pub fn rates(&self) -> &[f64] {
        &self.rates
    }

    /// How fully the felt rests on the wire, clamped into `[0, 1]`.
    pub fn set_engagement(&mut self, engagement: f64) {
        let engagement = if engagement.is_finite() {
            engagement.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if engagement.to_bits() == self.engagement.to_bits() {
            return;
        }
        self.engagement = engagement;
        for (retain, rate) in self.retain.iter_mut().zip(&self.rates) {
            *retain = (-engagement * rate * self.dt).exp();
        }
    }

    /// One sample on the field's momentum; returns the energy removed (J).
    pub fn process(&self, momentum: &mut [f64; N]) -> f64 {
        if self.engagement == 0.0 {
            return 0.0;
        }
        let mut loss = 0.0;
        for (direction, retain) in self.directions.iter().zip(&self.retain) {
            let projection: f64 = direction
                .iter()
                .zip(momentum.iter())
                .map(|(u, m)| u * m)
                .sum();
            let removed = (1.0 - retain) * projection;
            for (m, u) in momentum.iter_mut().zip(direction) {
                *m -= removed * u;
            }
            let after = projection - removed;
            loss += 0.5 * self.dt * (projection * projection - after * after);
        }
        loss
    }
}
