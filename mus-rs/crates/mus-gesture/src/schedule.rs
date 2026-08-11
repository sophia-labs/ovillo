//! Constant-thermodynamic-speed timing for a fixed gate word.
//!
//! For a fixed word and constant per-edge weights the optimal timing is a
//! closed form, not an optimization. Minimizing the quadratic effort
//!
//! ```text
//! D = sum_k w_k theta_k^2 / tau_k     subject to     sum_k tau_k = duration
//! ```
//!
//! is Cauchy-Schwarz: with `length_k = sqrt(w_k) |theta_k|` and
//! `L = sum_k length_k`,
//!
//! ```text
//! D >= L^2 / duration,   with equality iff  tau_k = duration * length_k / L.
//! ```
//!
//! There is no iteration here, and none is needed. This is an exact scheduler
//! *for the declared cost model*; consensus §3.1 point 5 is explicit that it
//! is not a perceptual law.
//!
//! **Zero-angle gates.** A gate with `theta = 0` has `length_k = 0` and gets
//! `tau_k = 0`. It contributes nothing to `L` or to `D`, and it is dropped
//! from the executable schedule: `G(i, j, 0)` is the identity, so removing it
//! leaves the realized endpoint bit-identical while avoiding a segment of zero
//! duration in a renderer. The dropped indices remain visible in
//! [`ThermodynamicSchedule::segment_durations`], which stays aligned with the
//! full word, so a receipt still names every compiled gate.

use mus_watsn::OperatorProgram;

use crate::compile::GivensGate;
use crate::error::GestureError;
use crate::graph::ControlGraph;

/// Resolved metric weights for one word: `weights[k]` is the friction of the
/// authored edge that gate `k` runs on.
#[derive(Debug, Clone, PartialEq)]
pub struct FrictionMetric {
    weights: Vec<f64>,
}

impl FrictionMetric {
    /// Resolve every gate's edge friction from the authored graph.
    pub fn resolve(gates: &[GivensGate], graph: &ControlGraph) -> Result<Self, GestureError> {
        let mut weights = Vec::with_capacity(gates.len());
        for gate in gates {
            let edge = graph.edge_between(gate.left, gate.right).ok_or(
                GestureError::TreeEdgeOffGraph {
                    left: gate.canonical_pair().0,
                    right: gate.canonical_pair().1,
                },
            )?;
            if edge.id != gate.edge {
                return Err(GestureError::TreeEdgeOffGraph {
                    left: gate.canonical_pair().0,
                    right: gate.canonical_pair().1,
                });
            }
            weights.push(edge.friction);
        }
        Ok(Self { weights })
    }

    pub fn weights(&self) -> &[f64] {
        &self.weights
    }
}

/// The closed-form timing of a fixed word.
#[derive(Debug, Clone, PartialEq)]
pub struct ThermodynamicSchedule {
    duration: f64,
    lengths: Vec<f64>,
    segment_durations: Vec<f64>,
    executable: Vec<usize>,
    thermodynamic_length: f64,
    minimum_quadratic_cost: f64,
    constant_speed_residual: f64,
}

impl ThermodynamicSchedule {
    /// Total duration the word is spread over.
    pub fn duration(&self) -> f64 {
        self.duration
    }

    /// `length_k = sqrt(w_k) |theta_k|`, aligned with the full word.
    pub fn lengths(&self) -> &[f64] {
        &self.lengths
    }

    /// `tau_k`, aligned with the full word. Zero-angle gates carry `0.0`.
    pub fn segment_durations(&self) -> &[f64] {
        &self.segment_durations
    }

    /// Indices of the gates that survive into the executable schedule, in
    /// order: exactly those with `tau_k > 0`.
    pub fn executable_indices(&self) -> &[usize] {
        &self.executable
    }

    /// `L = sum_k length_k`.
    pub fn thermodynamic_length(&self) -> f64 {
        self.thermodynamic_length
    }

    /// `D_min = L^2 / duration`, the predicted minimum quadratic cost.
    pub fn minimum_quadratic_cost(&self) -> f64 {
        self.minimum_quadratic_cost
    }

    /// `max_k(length_k / tau_k) - min_k(length_k / tau_k)` over executable
    /// segments. Analytically zero; numerically it is float64 rounding of
    /// `L / duration`, so callers assert a bound rather than equality.
    pub fn constant_speed_residual(&self) -> f64 {
        self.constant_speed_residual
    }

    /// Per-segment thermodynamic speed for the executable segments.
    pub fn executable_speeds(&self) -> Vec<f64> {
        self.executable
            .iter()
            .map(|&index| self.lengths[index] / self.segment_durations[index])
            .collect()
    }

    /// The quadratic cost actually realized by these segment durations —
    /// recomputed from `sum_k length_k^2 / tau_k` rather than from the closed
    /// form, so a test can confirm the two agree.
    pub fn realized_quadratic_cost(&self) -> f64 {
        self.executable
            .iter()
            .map(|&index| {
                let length = self.lengths[index];
                length * length / self.segment_durations[index]
            })
            .sum()
    }

    /// Lower the surviving gates to a `mus-watsn` program. Dropping the
    /// zero-angle gates is exact: each is the identity factor.
    pub fn executable_program(
        &self,
        gates: &[GivensGate],
        dimension: usize,
    ) -> Result<OperatorProgram, GestureError> {
        assert_eq!(
            gates.len(),
            self.segment_durations.len(),
            "schedule and word length disagree"
        );
        let factors = self
            .executable
            .iter()
            .map(|&index| gates[index].to_plane_rotation())
            .collect();
        Ok(OperatorProgram::new(dimension, factors)?)
    }
}

/// Time a word at constant thermodynamic speed.
pub fn thermodynamic_schedule(
    gates: &[GivensGate],
    metric: &FrictionMetric,
    duration: f64,
) -> Result<ThermodynamicSchedule, GestureError> {
    if !duration.is_finite() || duration <= 0.0 {
        return Err(GestureError::InvalidDuration { duration });
    }
    assert_eq!(
        gates.len(),
        metric.weights().len(),
        "friction metric does not match the word"
    );

    let lengths: Vec<f64> = gates
        .iter()
        .zip(metric.weights())
        .map(|(gate, weight)| weight.sqrt() * gate.theta.abs())
        .collect();
    let thermodynamic_length: f64 = lengths.iter().sum();

    if thermodynamic_length == 0.0 {
        // The identity word. Every segment is zero-length, nothing is
        // executable, and both the length and the predicted cost are exactly
        // zero. The reference instead spreads `duration` uniformly over gates
        // that do nothing; that produces nonzero segment durations for a
        // gesture with no extent, which a renderer would have to special-case
        // anyway.
        return Ok(ThermodynamicSchedule {
            duration,
            segment_durations: vec![0.0; lengths.len()],
            lengths,
            executable: Vec::new(),
            thermodynamic_length: 0.0,
            minimum_quadratic_cost: 0.0,
            constant_speed_residual: 0.0,
        });
    }

    let segment_durations: Vec<f64> = lengths
        .iter()
        .map(|length| duration * length / thermodynamic_length)
        .collect();
    let executable: Vec<usize> = segment_durations
        .iter()
        .enumerate()
        .filter(|(_, tau)| **tau > 0.0)
        .map(|(index, _)| index)
        .collect();

    let mut fastest = f64::NEG_INFINITY;
    let mut slowest = f64::INFINITY;
    for &index in &executable {
        let speed = lengths[index] / segment_durations[index];
        fastest = fastest.max(speed);
        slowest = slowest.min(speed);
    }
    let constant_speed_residual = if executable.is_empty() {
        0.0
    } else {
        fastest - slowest
    };

    Ok(ThermodynamicSchedule {
        duration,
        lengths,
        segment_durations,
        executable,
        thermodynamic_length,
        minimum_quadratic_cost: thermodynamic_length * thermodynamic_length / duration,
        constant_speed_residual,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::{compile, endpoint_operator};
    use crate::graph::{ControlGraph, EdgeControl, EdgeId, ModeId};
    use crate::matrix::Matrix;
    use crate::target::RotationTarget;

    fn weighted_path(n: usize) -> ControlGraph {
        let modes: Vec<ModeId> = (0..n as u32).map(ModeId).collect();
        let edges: Vec<EdgeControl> = (0..n - 1)
            .map(|i| {
                EdgeControl::with_friction(
                    EdgeId(i as u32),
                    ModeId(i as u32),
                    ModeId(i as u32 + 1),
                    0.5 + 0.75 * (i as f64),
                )
            })
            .collect();
        ControlGraph::new(modes, edges).unwrap()
    }

    fn dense_givens(n: usize, left: usize, right: usize, theta: f64) -> Matrix {
        let mut out = Matrix::identity(n);
        let (sin, cos) = theta.sin_cos();
        out.set(left, left, cos);
        out.set(left, right, -sin);
        out.set(right, left, sin);
        out.set(right, right, cos);
        out
    }

    #[test]
    fn closed_form_matches_the_realized_cost_and_is_constant_speed() {
        let graph = weighted_path(6);
        let target = RotationTarget::new(
            dense_givens(6, 0, 3, 0.9).multiply(&dense_givens(6, 2, 5, -1.4)),
            1.0e-9,
        )
        .unwrap();
        let certificate = compile(&target, &graph).unwrap();
        let metric = FrictionMetric::resolve(certificate.gates(), &graph).unwrap();
        let schedule = thermodynamic_schedule(certificate.gates(), &metric, 2.0).unwrap();

        assert!((schedule.segment_durations().iter().sum::<f64>() - 2.0).abs() <= 1.0e-12);
        let closed = schedule.minimum_quadratic_cost();
        let realized = schedule.realized_quadratic_cost();
        assert!(
            (closed - realized).abs() <= 1.0e-9 * closed.max(1.0),
            "closed {closed} vs realized {realized}"
        );
        assert!(
            schedule.constant_speed_residual()
                <= 1.0e-12 * (schedule.thermodynamic_length() / 2.0).max(1.0)
        );
    }

    /// Any other admissible split of the same duration costs at least as much:
    /// the Cauchy-Schwarz bound, checked numerically.
    #[test]
    fn no_perturbed_split_beats_the_closed_form() {
        let graph = weighted_path(5);
        let target = RotationTarget::new(
            dense_givens(5, 1, 4, 1.1).multiply(&dense_givens(5, 0, 2, 0.6)),
            1.0e-9,
        )
        .unwrap();
        let certificate = compile(&target, &graph).unwrap();
        let metric = FrictionMetric::resolve(certificate.gates(), &graph).unwrap();
        let schedule = thermodynamic_schedule(certificate.gates(), &metric, 3.0).unwrap();
        let optimum = schedule.minimum_quadratic_cost();

        let lengths = schedule.lengths().to_vec();
        for shift in [0.05_f64, 0.2, 0.5, 0.9] {
            // Move mass from the first executable segment to the last.
            let mut durations = schedule.segment_durations().to_vec();
            let first = schedule.executable_indices()[0];
            let last = *schedule.executable_indices().last().unwrap();
            if first == last {
                continue;
            }
            let moved = durations[first] * shift;
            durations[first] -= moved;
            durations[last] += moved;
            let perturbed: f64 = schedule
                .executable_indices()
                .iter()
                .map(|&index| lengths[index] * lengths[index] / durations[index])
                .sum();
            assert!(
                perturbed >= optimum - 1.0e-12,
                "perturbed {perturbed} beat the closed form {optimum}"
            );
        }
    }

    #[test]
    fn identity_word_closes_with_zero_length_and_no_executable_segments() {
        let graph = weighted_path(6);
        let target = RotationTarget::identity(6).unwrap();
        let certificate = compile(&target, &graph).unwrap();
        let metric = FrictionMetric::resolve(certificate.gates(), &graph).unwrap();
        let schedule = thermodynamic_schedule(certificate.gates(), &metric, 2.0).unwrap();
        assert_eq!(schedule.thermodynamic_length(), 0.0);
        assert_eq!(schedule.minimum_quadratic_cost(), 0.0);
        assert_eq!(schedule.constant_speed_residual(), 0.0);
        assert!(schedule.executable_indices().is_empty());
        assert!(schedule.segment_durations().iter().all(|tau| *tau == 0.0));
        let program = schedule
            .executable_program(certificate.gates(), certificate.dimension())
            .unwrap();
        assert!(program.factors().is_empty());
    }

    #[test]
    fn dropping_zero_angle_gates_leaves_the_endpoint_bit_identical() {
        // A target that only moves one plane leaves most peel angles at zero.
        let graph = weighted_path(6);
        let target = RotationTarget::new(dense_givens(6, 4, 5, 0.77), 1.0e-9).unwrap();
        let certificate = compile(&target, &graph).unwrap();
        let metric = FrictionMetric::resolve(certificate.gates(), &graph).unwrap();
        let schedule = thermodynamic_schedule(certificate.gates(), &metric, 1.5).unwrap();
        assert!(schedule.executable_indices().len() < certificate.gate_count());

        let trimmed = schedule
            .executable_program(certificate.gates(), certificate.dimension())
            .unwrap();
        let full = endpoint_operator(certificate.program()).unwrap();
        let reduced = endpoint_operator(&trimmed).unwrap();
        assert_eq!(full, reduced);
    }

    #[test]
    fn non_positive_duration_is_typed() {
        let graph = weighted_path(3);
        let target = RotationTarget::identity(3).unwrap();
        let certificate = compile(&target, &graph).unwrap();
        let metric = FrictionMetric::resolve(certificate.gates(), &graph).unwrap();
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                thermodynamic_schedule(certificate.gates(), &metric, bad).unwrap_err(),
                GestureError::InvalidDuration { .. }
            ));
        }
    }

    #[test]
    fn friction_resolution_follows_the_authored_weights() {
        let graph = weighted_path(4);
        let target = RotationTarget::new(dense_givens(4, 0, 3, 0.5), 1.0e-9).unwrap();
        let certificate = compile(&target, &graph).unwrap();
        let metric = FrictionMetric::resolve(certificate.gates(), &graph).unwrap();
        for (gate, weight) in certificate.gates().iter().zip(metric.weights()) {
            let edge = graph.edge_between(gate.left, gate.right).unwrap();
            assert_eq!(edge.friction, *weight);
        }
    }
}
