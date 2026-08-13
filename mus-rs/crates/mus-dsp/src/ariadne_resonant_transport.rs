//! Metric-compatible transport between near-internal-resonance material modes.
//!
//! This is a pure Ariadne coupling law. It does not filter or reshape an
//! emitted sample. A pair's live displacement geometry chooses a small Givens
//! rotation of its momentum coordinates. Every local factor is exactly
//! orthogonal in the declared kinetic metric, so the operation performs no
//! work; subsequent unequal modal propagation turns the transferred momentum
//! into saturation, beating, harmonic exchange, and intermodulation.

#![allow(clippy::needless_range_loop)]

use mus_watsn::givens;
use std::f64::consts::TAU;

const MIN_SAMPLE_RATE_HZ: f64 = 8_000.0;
const MAX_SAMPLE_RATE_HZ: f64 = 384_000.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResonantTransportControls {
    pub interlace: f64,
    pub memory: f64,
}

impl Default for ResonantTransportControls {
    fn default() -> Self {
        Self {
            interlace: 0.42,
            memory: 0.28,
        }
    }
}

impl ResonantTransportControls {
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            interlace: finite_or_zero(self.interlace).clamp(0.0, 1.0),
            memory: finite_or_zero(self.memory).clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ResonantTransportFrame {
    pub kinetic_before: f64,
    pub kinetic_after: f64,
    pub metric_residual: f64,
    pub maximum_abs_angle: f64,
    pub maximum_abs_geometry: f64,
    pub active_pairs: usize,
    pub non_finite_state: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResonantTransportError {
    InvalidSampleRate,
    RepeatedCoordinate { pair: usize, index: usize },
    CoordinateOutOfRange { pair: usize, index: usize, dimension: usize },
}

impl std::fmt::Display for ResonantTransportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSampleRate => {
                formatter.write_str("invalid internal-resonance transport sample rate")
            }
            Self::RepeatedCoordinate { pair, index } => {
                write!(formatter, "resonance pair {pair} repeats coordinate {index}")
            }
            Self::CoordinateOutOfRange {
                pair,
                index,
                dimension,
            } => write!(
                formatter,
                "resonance pair {pair} coordinate {index} exceeds dimension {dimension}"
            ),
        }
    }
}

impl std::error::Error for ResonantTransportError {}

/// Sparse, state-dependent, zero-work transport over authored resonance pairs.
#[derive(Debug, Clone)]
pub struct InternalResonanceTransport<const N: usize, const PAIRS: usize> {
    sample_rate_hz: f64,
    dt: f64,
    pairs: [(usize, usize); PAIRS],
    weights: [f64; PAIRS],
}

impl<const N: usize, const PAIRS: usize> InternalResonanceTransport<N, PAIRS> {
    pub fn try_new(
        sample_rate_hz: f64,
        pairs: [(usize, usize); PAIRS],
        weights: [f64; PAIRS],
    ) -> Result<Self, ResonantTransportError> {
        if !sample_rate_hz.is_finite()
            || !(MIN_SAMPLE_RATE_HZ..=MAX_SAMPLE_RATE_HZ).contains(&sample_rate_hz)
        {
            return Err(ResonantTransportError::InvalidSampleRate);
        }
        for (pair_index, (left, right)) in pairs.iter().copied().enumerate() {
            if left == right {
                return Err(ResonantTransportError::RepeatedCoordinate {
                    pair: pair_index,
                    index: left,
                });
            }
            for index in [left, right] {
                if index >= N {
                    return Err(ResonantTransportError::CoordinateOutOfRange {
                        pair: pair_index,
                        index,
                        dimension: N,
                    });
                }
            }
        }
        Ok(Self {
            sample_rate_hz,
            dt: sample_rate_hz.recip(),
            pairs,
            weights: weights.map(finite_or_zero),
        })
    }

    #[must_use]
    pub fn sample_rate_hz(&self) -> f64 {
        self.sample_rate_hz
    }

    #[must_use]
    pub fn pairs(&self) -> &[(usize, usize); PAIRS] {
        &self.pairs
    }

    #[must_use]
    pub fn weights(&self) -> &[f64; PAIRS] {
        &self.weights
    }

    /// Apply one sample of state-dependent transport.
    ///
    /// The geometry scalar is bounded by `tanh`; the rate is expressed in
    /// hertz and converted to a per-sample angle. Consequently a frozen
    /// geometry has approximately sample-rate-invariant accumulated transport
    /// over a fixed duration.
    pub fn process(
        &self,
        displacements: &[f64; N],
        momenta: &mut [f64; N],
        controls: ResonantTransportControls,
    ) -> ResonantTransportFrame {
        let controls = controls.sanitized();
        if displacements.iter().any(|value| !value.is_finite())
            || momenta.iter().any(|value| !value.is_finite())
        {
            return ResonantTransportFrame {
                non_finite_state: true,
                ..ResonantTransportFrame::default()
            };
        }
        let kinetic_before = kinetic_energy(momenta, self.dt);
        if controls.interlace == 0.0 || controls.memory == 0.0 {
            return ResonantTransportFrame {
                kinetic_before,
                kinetic_after: kinetic_before,
                ..ResonantTransportFrame::default()
            };
        }

        let transport_hz = 90.0 + 900.0 * controls.interlace * controls.interlace;
        let base_angle = TAU * transport_hz / self.sample_rate_hz;
        let memory_scale = 0.35 + 0.65 * controls.memory;
        let mut maximum_abs_angle = 0.0_f64;
        let mut maximum_abs_geometry = 0.0_f64;
        let mut active_pairs = 0usize;

        for pair_index in 0..PAIRS {
            let (left, right) = self.pairs[pair_index];
            let left_q = displacements[left];
            let right_q = displacements[right];
            let geometry = (1.6 * left_q * right_q
                + 0.25 * (left_q * left_q - right_q * right_q))
                .tanh();
            let angle = base_angle * memory_scale * self.weights[pair_index] * geometry;
            maximum_abs_geometry = maximum_abs_geometry.max(geometry.abs());
            maximum_abs_angle = maximum_abs_angle.max(angle.abs());
            if angle != 0.0 {
                let (next_left, next_right) = givens(momenta[left], momenta[right], angle);
                momenta[left] = next_left;
                momenta[right] = next_right;
                active_pairs += 1;
            }
        }

        let kinetic_after = kinetic_energy(momenta, self.dt);
        ResonantTransportFrame {
            kinetic_before,
            kinetic_after,
            metric_residual: kinetic_after - kinetic_before,
            maximum_abs_angle,
            maximum_abs_geometry,
            active_pairs,
            non_finite_state: false,
        }
    }
}

#[inline]
fn kinetic_energy<const N: usize>(momenta: &[f64; N], dt: f64) -> f64 {
    0.5 * dt * momenta.iter().map(|value| value * value).sum::<f64>()
}

#[inline]
fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAIRS: [(usize, usize); 3] = [(0, 2), (1, 3), (2, 4)];
    const WEIGHTS: [f64; 3] = [1.0, 0.75, -0.5];

    #[test]
    fn invalid_pairs_are_rejected() {
        assert!(InternalResonanceTransport::<4, 1>::try_new(
            48_000.0,
            [(0, 0)],
            [1.0]
        )
        .is_err());
        assert!(InternalResonanceTransport::<4, 1>::try_new(
            48_000.0,
            [(0, 4)],
            [1.0]
        )
        .is_err());
    }

    #[test]
    fn zero_controls_are_exact_identity() {
        let transport =
            InternalResonanceTransport::<5, 3>::try_new(48_000.0, PAIRS, WEIGHTS).unwrap();
        let q = [0.7, -0.4, 0.9, 0.2, -0.3];
        let original = [0.3, -0.8, 0.5, 0.1, 0.7];
        let mut p = original;
        let frame = transport.process(
            &q,
            &mut p,
            ResonantTransportControls {
                interlace: 0.0,
                memory: 1.0,
            },
        );
        assert_eq!(p, original);
        assert_eq!(frame.metric_residual, 0.0);
        assert_eq!(frame.active_pairs, 0);
    }

    #[test]
    fn nontrivial_transport_preserves_kinetic_metric() {
        let transport =
            InternalResonanceTransport::<5, 3>::try_new(48_000.0, PAIRS, WEIGHTS).unwrap();
        let q = [0.7, -0.4, 0.9, 0.2, -0.3];
        let original = [0.3, -0.8, 0.5, 0.1, 0.7];
        let mut p = original;
        let frame = transport.process(
            &q,
            &mut p,
            ResonantTransportControls {
                interlace: 0.85,
                memory: 0.72,
            },
        );
        assert_ne!(p, original);
        assert!(frame.active_pairs > 0);
        assert!(frame.maximum_abs_angle > 0.0);
        assert!(frame.metric_residual.abs() < 2.0e-18);
    }

    #[test]
    fn geometry_changes_transport_without_changing_work() {
        let transport =
            InternalResonanceTransport::<5, 3>::try_new(48_000.0, PAIRS, WEIGHTS).unwrap();
        let mut a = [0.3, -0.8, 0.5, 0.1, 0.7];
        let mut b = a;
        let controls = ResonantTransportControls {
            interlace: 0.9,
            memory: 0.9,
        };
        let first = transport.process(&[0.7, -0.4, 0.9, 0.2, -0.3], &mut a, controls);
        let second = transport.process(&[-0.7, 0.4, 0.2, -0.9, 0.3], &mut b, controls);
        assert_ne!(a, b);
        assert!(first.metric_residual.abs() < 2.0e-18);
        assert!(second.metric_residual.abs() < 2.0e-18);
    }

    #[test]
    fn frozen_geometry_accumulates_consistently_across_sample_rates() {
        let low = InternalResonanceTransport::<5, 1>::try_new(
            48_000.0,
            [(0, 2)],
            [1.0],
        )
        .unwrap();
        let high = InternalResonanceTransport::<5, 1>::try_new(
            96_000.0,
            [(0, 2)],
            [1.0],
        )
        .unwrap();
        let q = [0.7, 0.0, 0.9, 0.0, 0.0];
        let controls = ResonantTransportControls {
            interlace: 0.6,
            memory: 0.8,
        };
        let mut low_state = [1.0, 0.0, 0.0, 0.0, 0.0];
        let mut high_state = low_state;
        for _ in 0..480 {
            low.process(&q, &mut low_state, controls);
        }
        for _ in 0..960 {
            high.process(&q, &mut high_state, controls);
        }
        for (left, right) in low_state.iter().zip(high_state) {
            assert!((left - right).abs() < 3.0e-13);
        }
    }
}
