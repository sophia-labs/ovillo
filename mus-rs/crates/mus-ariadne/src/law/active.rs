//! Thresholded, reservoir-paid active matter for Ariadne.
//!
//! This module is an implementation candidate extracted from the first
//! electric-region fidelity campaign. It is not an amplifier stage and it does
//! not process an emitted sample. Instead, it acts on three orthogonal
//! directions of a persistent material momentum state.
//!
//! The inclusion has one slow constitutive activation coordinate. Boundary
//! motion, existing material motion, and returning room motion can move that
//! coordinate across a threshold. Activation has no acoustic energy of its own;
//! it only gates conversion from a caller-owned finite work reservoir. For each
//! material direction `v_r`, with projection `d_r = v_r^T p`, the local law is
//!
//! ```text
//! P_r = mu_r a d_r^2 (1 - d_r^2 / A_r^2).
//! ```
//!
//! Below `A_r` the inclusion requests positive work. Above `A_r` it removes
//! work, yielding a saturable limit-cycle tendency rather than unbounded gain.
//! Every positive increment is removed from the reservoir exactly in the
//! declared kinetic metric; every negative increment is reported as internal
//! loss. A zero momentum state remains an exact fixed point.

#![allow(clippy::needless_range_loop)]

#[cfg(test)]
use std::array;

pub const ACTIVE_RANKS: usize = 3;
const MIN_SAMPLE_RATE_HZ: f64 = 8_000.0;
const MAX_SAMPLE_RATE_HZ: f64 = 384_000.0;
const DIRECTION_EPSILON: f64 = 1.0e-12;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActiveInclusionControls {
    pub drive: f64,
    pub headroom: f64,
    pub sustain: f64,
    pub sag: f64,
    pub bloom: f64,
    pub feedback: f64,
    pub space: f64,
}

impl Default for ActiveInclusionControls {
    fn default() -> Self {
        Self {
            drive: 0.42,
            headroom: 0.72,
            sustain: 0.24,
            sag: 0.35,
            bloom: 0.36,
            feedback: 0.04,
            space: 0.16,
        }
    }
}

impl ActiveInclusionControls {
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            drive: finite_or_zero(self.drive).clamp(0.0, 1.5),
            headroom: finite_or_zero(self.headroom).clamp(0.02, 1.0),
            sustain: finite_or_zero(self.sustain).clamp(0.0, 1.0),
            sag: finite_or_zero(self.sag).clamp(0.0, 1.0),
            bloom: finite_or_zero(self.bloom).clamp(0.0, 1.0),
            feedback: finite_or_zero(self.feedback).clamp(0.0, 1.0),
            space: finite_or_zero(self.space).clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ActiveInclusionFrame {
    pub activation_before: f64,
    pub activation_after: f64,
    pub requested_positive_work: f64,
    pub accepted_positive_work: f64,
    pub dissipated_work: f64,
    pub reservoir_before: f64,
    pub reservoir_after: f64,
    pub kinetic_before: f64,
    pub kinetic_after: f64,
    pub closure_residual: f64,
    pub maximum_abs_projection: f64,
    pub non_finite_state: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveInclusionError {
    InvalidSampleRate,
    DegenerateDirection { rank: usize },
    InvalidActivation,
}

impl std::fmt::Display for ActiveInclusionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSampleRate => formatter.write_str("invalid active-inclusion sample rate"),
            Self::DegenerateDirection { rank } => {
                write!(formatter, "active-inclusion direction {rank} is degenerate")
            }
            Self::InvalidActivation => {
                formatter.write_str("active-inclusion activation must be finite in [0, 1]")
            }
        }
    }
}

impl std::error::Error for ActiveInclusionError {}

/// One slow, thresholded active inclusion acting on three orthogonal material
/// directions.
#[derive(Debug, Clone)]
pub struct SynchronizedActiveInclusion<const N: usize> {
    sample_rate_hz: f64,
    dt: f64,
    directions: [[f64; N]; ACTIVE_RANKS],
    activation: f64,
}

impl<const N: usize> SynchronizedActiveInclusion<N> {
    pub fn try_new(
        sample_rate_hz: f64,
        mut directions: [[f64; N]; ACTIVE_RANKS],
    ) -> Result<Self, ActiveInclusionError> {
        if !sample_rate_hz.is_finite()
            || !(MIN_SAMPLE_RATE_HZ..=MAX_SAMPLE_RATE_HZ).contains(&sample_rate_hz)
        {
            return Err(ActiveInclusionError::InvalidSampleRate);
        }
        orthonormalize(&mut directions)?;
        Ok(Self {
            sample_rate_hz,
            dt: sample_rate_hz.recip(),
            directions,
            activation: 0.0,
        })
    }

    #[must_use]
    pub fn sample_rate_hz(&self) -> f64 {
        self.sample_rate_hz
    }

    #[must_use]
    pub fn activation(&self) -> f64 {
        self.activation
    }

    #[must_use]
    pub fn directions(&self) -> &[[f64; N]; ACTIVE_RANKS] {
        &self.directions
    }

    pub fn reset(&mut self) {
        self.activation = 0.0;
    }

    pub fn restore_activation(&mut self, activation: f64) -> Result<(), ActiveInclusionError> {
        if !activation.is_finite() || !(0.0..=1.0).contains(&activation) {
            return Err(ActiveInclusionError::InvalidActivation);
        }
        self.activation = activation;
        Ok(())
    }

    /// Advance the active inclusion by one host sample.
    ///
    /// `boundary_activity` and `room_activity` are nonnegative normalized
    /// motion magnitudes. They carry no energy into this method. The only
    /// positive acoustic work performed here is `accepted_positive_work`, which
    /// is removed from `reservoir` exactly. The caller remains responsible for
    /// booking external reservoir recharge separately.
    pub fn process(
        &mut self,
        momenta: &mut [f64; N],
        boundary_activity: f64,
        room_activity: f64,
        controls: ActiveInclusionControls,
        reservoir: &mut f64,
    ) -> ActiveInclusionFrame {
        let controls = controls.sanitized();
        let reservoir_before = finite_or_zero(*reservoir).max(0.0);
        *reservoir = reservoir_before;
        let kinetic_before = kinetic_energy(momenta, self.dt);
        let activation_before = self.activation;

        if momenta.iter().any(|value| !value.is_finite()) {
            return ActiveInclusionFrame {
                activation_before,
                activation_after: activation_before,
                reservoir_before,
                reservoir_after: reservoir_before,
                kinetic_before,
                kinetic_after: kinetic_before,
                non_finite_state: true,
                ..ActiveInclusionFrame::default()
            };
        }

        let material_rms = if N == 0 {
            0.0
        } else {
            (momenta.iter().map(|value| value * value).sum::<f64>() / N as f64).sqrt()
        };
        let sensed = finite_or_zero(boundary_activity).abs()
            + 0.08 * controls.sustain * material_rms
            + 12.0 * controls.feedback * controls.space * finite_or_zero(room_activity).abs();
        let threshold = 0.025 + 0.25 * controls.headroom;
        let excess = (sensed - threshold).max(0.0);
        let target_activation = excess / (threshold + excess + f64::EPSILON);
        let attack_seconds = 0.012 + 0.060 * controls.bloom;
        let release_seconds = 0.10 + 1.40 * controls.sag * controls.sag;
        let time_constant = if target_activation > self.activation {
            attack_seconds
        } else {
            release_seconds
        };
        self.activation +=
            (target_activation - self.activation) * (1.0 - (-self.dt / time_constant).exp());
        self.activation = self.activation.clamp(0.0, 1.0);

        let rank_weights = [1.0, 0.62, 0.38];
        let target_amplitudes = [
            0.11 + 0.30 * controls.headroom,
            0.09 + 0.23 * controls.headroom,
            0.07 + 0.18 * controls.headroom,
        ];
        let mu_base = (4.0 + 180.0 * controls.drive * controls.drive) * self.activation;

        let mut requested_positive_work = 0.0;
        let mut accepted_positive_work = 0.0;
        let mut dissipated_work = 0.0;
        let mut maximum_abs_projection = 0.0_f64;

        for rank in 0..ACTIVE_RANKS {
            let direction = &self.directions[rank];
            let projection = dot(momenta, direction);
            maximum_abs_projection = maximum_abs_projection.max(projection.abs());
            if projection.abs() <= 1.0e-15 || mu_base == 0.0 {
                continue;
            }
            let target = target_amplitudes[rank];
            let signed_work = self.dt
                * mu_base
                * rank_weights[rank]
                * projection
                * projection
                * (1.0 - projection * projection / (target * target));

            if signed_work > 0.0 {
                requested_positive_work += signed_work;
                let accepted = signed_work.min(*reservoir);
                if accepted > 0.0 {
                    let next_projection = projection.signum()
                        * (projection * projection + 2.0 * accepted / self.dt).sqrt();
                    add_projection(momenta, direction, next_projection - projection);
                    *reservoir -= accepted;
                    accepted_positive_work += accepted;
                }
            } else if signed_work < 0.0 {
                let available = 0.5 * self.dt * projection * projection;
                let dissipated = (-signed_work).min(available);
                let next_squared = (projection * projection - 2.0 * dissipated / self.dt).max(0.0);
                let next_projection = projection.signum() * next_squared.sqrt();
                add_projection(momenta, direction, next_projection - projection);
                dissipated_work += dissipated;
            }
        }

        let kinetic_after = kinetic_energy(momenta, self.dt);
        let closure_residual =
            kinetic_after - kinetic_before - accepted_positive_work + dissipated_work;
        ActiveInclusionFrame {
            activation_before,
            activation_after: self.activation,
            requested_positive_work,
            accepted_positive_work,
            dissipated_work,
            reservoir_before,
            reservoir_after: *reservoir,
            kinetic_before,
            kinetic_after,
            closure_residual,
            maximum_abs_projection,
            non_finite_state: false,
        }
    }
}

fn orthonormalize<const N: usize>(
    directions: &mut [[f64; N]; ACTIVE_RANKS],
) -> Result<(), ActiveInclusionError> {
    for rank in 0..ACTIVE_RANKS {
        for previous in 0..rank {
            let projection = dot(&directions[rank], &directions[previous]);
            let previous_direction = directions[previous];
            for (value, basis) in directions[rank].iter_mut().zip(previous_direction) {
                *value -= projection * basis;
            }
        }
        let norm = directions[rank]
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        if !norm.is_finite() || norm <= DIRECTION_EPSILON {
            return Err(ActiveInclusionError::DegenerateDirection { rank });
        }
        for value in &mut directions[rank] {
            *value /= norm;
        }
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
fn add_projection<const N: usize>(state: &mut [f64; N], direction: &[f64; N], amount: f64) {
    for (value, basis) in state.iter_mut().zip(direction) {
        *value += amount * basis;
    }
}

#[inline]
fn kinetic_energy<const N: usize>(momenta: &[f64; N], dt: f64) -> f64 {
    0.5 * dt * momenta.iter().map(|value| value * value).sum::<f64>()
}

#[inline]
fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directions() -> [[f64; 4]; ACTIVE_RANKS] {
        [
            [1.0, 1.0, 0.0, 0.0],
            [0.0, 1.0, 1.0, 0.0],
            [1.0, -1.0, 1.0, -1.0],
        ]
    }

    #[test]
    fn directions_are_orthonormalized() {
        let inclusion = SynchronizedActiveInclusion::try_new(48_000.0, directions()).unwrap();
        for rank in 0..ACTIVE_RANKS {
            assert!(
                (dot(&inclusion.directions()[rank], &inclusion.directions()[rank]) - 1.0).abs()
                    < 1.0e-14
            );
            for other in rank + 1..ACTIVE_RANKS {
                assert!(
                    dot(
                        &inclusion.directions()[rank],
                        &inclusion.directions()[other]
                    )
                    .abs()
                        < 1.0e-14
                );
            }
        }
    }

    #[test]
    fn exact_zero_state_remains_zero() {
        let mut inclusion = SynchronizedActiveInclusion::try_new(48_000.0, directions()).unwrap();
        let mut momenta = [0.0; 4];
        let mut reservoir = 1.0;
        for _ in 0..48_000 {
            let frame = inclusion.process(
                &mut momenta,
                0.0,
                0.0,
                ActiveInclusionControls::default(),
                &mut reservoir,
            );
            assert_eq!(frame.accepted_positive_work, 0.0);
            assert_eq!(frame.dissipated_work, 0.0);
            assert_eq!(frame.closure_residual, 0.0);
        }
        assert_eq!(momenta, [0.0; 4]);
        assert_eq!(reservoir, 1.0);
        assert_eq!(inclusion.activation(), 0.0);
    }

    #[test]
    fn below_threshold_input_does_not_activate() {
        let mut inclusion = SynchronizedActiveInclusion::try_new(48_000.0, directions()).unwrap();
        let mut momenta = [0.01, 0.0, 0.0, 0.0];
        let mut reservoir = 1.0;
        for _ in 0..4_800 {
            inclusion.process(
                &mut momenta,
                0.01,
                0.0,
                ActiveInclusionControls::default(),
                &mut reservoir,
            );
        }
        assert_eq!(inclusion.activation(), 0.0);
        assert_eq!(reservoir, 1.0);
    }

    #[test]
    fn positive_work_is_paid_exactly() {
        let mut inclusion = SynchronizedActiveInclusion::try_new(48_000.0, directions()).unwrap();
        let mut momenta = [0.08, 0.02, -0.01, 0.0];
        let mut reservoir = 0.25;
        let mut accepted = 0.0;
        let reservoir_before = reservoir;
        for _ in 0..2_400 {
            let frame = inclusion.process(
                &mut momenta,
                0.8,
                0.0,
                ActiveInclusionControls {
                    drive: 0.9,
                    ..ActiveInclusionControls::default()
                },
                &mut reservoir,
            );
            accepted += frame.accepted_positive_work;
            assert!(frame.closure_residual.abs() < 2.0e-14);
        }
        assert!(accepted > 0.0);
        assert!((reservoir_before - reservoir - accepted).abs() < 2.0e-14);
    }

    #[test]
    fn above_target_motion_is_dissipative() {
        let mut inclusion = SynchronizedActiveInclusion::try_new(48_000.0, directions()).unwrap();
        inclusion.restore_activation(1.0).unwrap();
        let first = inclusion.directions()[0];
        let mut momenta = array::from_fn(|index| 2.0 * first[index]);
        let mut reservoir = 0.25;
        let frame = inclusion.process(
            &mut momenta,
            0.0,
            0.0,
            ActiveInclusionControls {
                drive: 1.0,
                ..ActiveInclusionControls::default()
            },
            &mut reservoir,
        );
        assert_eq!(frame.accepted_positive_work, 0.0);
        assert!(frame.dissipated_work > 0.0);
        assert!(frame.kinetic_after < frame.kinetic_before);
        assert!(frame.closure_residual.abs() < 2.0e-14);
    }

    #[test]
    fn room_motion_can_cross_the_activation_threshold() {
        let mut dry = SynchronizedActiveInclusion::try_new(48_000.0, directions()).unwrap();
        let mut coupled = dry.clone();
        let mut dry_momenta = [0.02, 0.0, 0.0, 0.0];
        let mut coupled_momenta = dry_momenta;
        let mut dry_reservoir = 1.0;
        let mut coupled_reservoir = 1.0;
        let controls = ActiveInclusionControls {
            feedback: 1.0,
            space: 1.0,
            ..ActiveInclusionControls::default()
        };
        for _ in 0..4_800 {
            dry.process(&mut dry_momenta, 0.0, 0.0, controls, &mut dry_reservoir);
            coupled.process(
                &mut coupled_momenta,
                0.0,
                0.04,
                controls,
                &mut coupled_reservoir,
            );
        }
        assert_eq!(dry.activation(), 0.0);
        assert!(coupled.activation() > 0.05);
        assert!(coupled_reservoir < dry_reservoir);
    }

    #[test]
    fn activation_has_release_memory() {
        let mut inclusion = SynchronizedActiveInclusion::try_new(48_000.0, directions()).unwrap();
        let mut momenta = [0.06, 0.01, 0.0, 0.0];
        let mut reservoir = 1.0;
        let controls = ActiveInclusionControls {
            sag: 1.0,
            ..ActiveInclusionControls::default()
        };
        for _ in 0..2_400 {
            inclusion.process(&mut momenta, 0.8, 0.0, controls, &mut reservoir);
        }
        let activated = inclusion.activation();
        for _ in 0..2_400 {
            inclusion.process(&mut momenta, 0.0, 0.0, controls, &mut reservoir);
        }
        assert!(activated > 0.0);
        assert!(inclusion.activation() > 0.0);
        assert!(inclusion.activation() < activated);
    }
}
