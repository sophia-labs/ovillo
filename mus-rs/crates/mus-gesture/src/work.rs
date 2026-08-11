//! Measured control work for a compiled rotation word.
//!
//! # Correction 4: what this module is *not*
//!
//! The Python reference exposes `zero_work_residual`, which the prose treats
//! as a work measurement. It is not: it applies the word to one fixed vector
//! and reports how far the result is from orthogonal. That is an
//! orthogonality probe, and this crate does not port it — the compiled
//! endpoint's orthogonality residual already says the same thing, better.
//!
//! What a work field in a receipt may mean here is narrower and stricter:
//! run the compiled program through the **`mus-watsn` work ledger** in a
//! **named metric**, declaring the model's prediction (`control_work = 0` for
//! a pure rotation in energy-normalized coordinates) and letting
//! [`mus_watsn::WorkFrame::residual`] report the measured discrepancy. If no
//! probe ensemble is supplied, the receipt's work field is simply **absent**.
//! Consensus rule 3 forbids presenting compiler-predicted zero work as
//! measured work, and an absent field is the honest encoding of "not
//! measured".
//!
//! This is still a compiler-side measurement on an operator word, not a
//! whole-render energy audit. It says: executing exactly these factors on
//! these probe states moved the declared metric energy by this much.

use mus_watsn::{OperatorProgram, PlaneRotation, WorkFrame, WorkLedger};

use crate::error::GestureError;

/// The one metric this phase declares: the identity quadratic form on
/// energy-normalized coordinates, which is what `mus-watsn` state transports
/// are normalized into.
pub const EUCLIDEAN_NORMALIZED_METRIC: &str = "euclidean-normalized";

/// A measured work audit of a compiled word.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkMeasurement {
    metric: &'static str,
    probe_states: usize,
    factors: usize,
    frames: u64,
    declared_control_work: f64,
    max_abs_residual: f64,
    global_residual: f64,
}

impl WorkMeasurement {
    /// Name of the metric the energies were computed in.
    pub fn metric(&self) -> &'static str {
        self.metric
    }

    pub fn probe_states(&self) -> usize {
        self.probe_states
    }

    pub fn factors(&self) -> usize {
        self.factors
    }

    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// The model's declared control work per frame: exactly `0.0`.
    pub fn declared_control_work(&self) -> f64 {
        self.declared_control_work
    }

    /// Worst single-factor deviation from the declared zero work.
    pub fn max_abs_residual(&self) -> f64 {
        self.max_abs_residual
    }

    /// End-to-end deviation across the whole word.
    pub fn global_residual(&self) -> f64 {
        self.global_residual
    }
}

/// Metric energy of a state: `sum_i x_i^2` in energy-normalized coordinates.
pub fn metric_energy(state: &[f64]) -> f64 {
    state.iter().map(|value| value * value).sum()
}

/// The `n` unit basis vectors — the default probe ensemble. Running every
/// basis vector through the word measures the whole operator, not one
/// direction of it.
pub fn basis_probes(dimension: usize) -> Vec<Vec<f64>> {
    (0..dimension)
        .map(|index| {
            let mut state = vec![0.0; dimension];
            state[index] = 1.0;
            state
        })
        .collect()
}

/// Measure the control work of `program` on a probe ensemble, one
/// `mus-watsn` work frame per factor per probe.
pub fn measure_control_work(
    program: &OperatorProgram,
    probes: &[Vec<f64>],
) -> Result<WorkMeasurement, GestureError> {
    let dimension = program.dimension();
    let mut ledger = WorkLedger::default();
    let mut single: Vec<PlaneRotation> = Vec::with_capacity(1);
    for probe in probes {
        if probe.len() != dimension {
            return Err(GestureError::Program(
                mus_watsn::ProgramError::DimensionMismatch {
                    expected: dimension,
                    actual: probe.len(),
                },
            ));
        }
        let mut state = probe.clone();
        for factor in program.factors() {
            let energy_start = metric_energy(&state);
            single.clear();
            single.push(*factor);
            let step = OperatorProgram::new(dimension, single.clone())?;
            step.apply(&mut state)?;
            let energy_end = metric_energy(&state);
            ledger.push(WorkFrame {
                energy_start,
                // A rotation has no source, loss, or radiation port, and the
                // model predicts zero control work in this metric. Everything
                // that actually happened lands in the frame residual.
                source_work: 0.0,
                control_work: 0.0,
                internal_loss: 0.0,
                radiation_loss: 0.0,
                output_work: 0.0,
                energy_end,
            });
        }
    }
    Ok(WorkMeasurement {
        metric: EUCLIDEAN_NORMALIZED_METRIC,
        probe_states: probes.len(),
        factors: program.factors().len(),
        frames: ledger.frames,
        declared_control_work: 0.0,
        max_abs_residual: ledger.max_abs_residual,
        global_residual: ledger.cumulative_residual,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::compile;
    use crate::graph::ControlGraph;
    use crate::matrix::Matrix;
    use crate::target::RotationTarget;

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
    fn a_compiled_rotation_word_measures_zero_work_to_roundoff() {
        let pairs: Vec<(usize, usize)> = (0..6).map(|i| (i, i + 1)).collect();
        let graph = ControlGraph::from_coordinate_edges(7, &pairs).unwrap();
        let target = RotationTarget::new(
            dense_givens(7, 0, 4, 1.2).multiply(&dense_givens(7, 3, 6, -0.8)),
            1.0e-9,
        )
        .unwrap();
        let certificate = compile(&target, &graph).unwrap();
        let measurement = measure_control_work(certificate.program(), &basis_probes(7)).unwrap();
        assert_eq!(measurement.metric(), EUCLIDEAN_NORMALIZED_METRIC);
        assert_eq!(measurement.probe_states(), 7);
        assert_eq!(measurement.frames(), (certificate.gate_count() * 7) as u64);
        assert_eq!(measurement.declared_control_work(), 0.0);
        assert!(measurement.max_abs_residual() <= 1.0e-14);
        assert!(measurement.global_residual().abs() <= 1.0e-13);
    }

    #[test]
    fn a_probe_of_the_wrong_length_is_typed() {
        let graph = ControlGraph::from_coordinate_edges(3, &[(0, 1), (1, 2)]).unwrap();
        let target = RotationTarget::identity(3).unwrap();
        let certificate = compile(&target, &graph).unwrap();
        let error = measure_control_work(certificate.program(), &[vec![1.0, 0.0]]).unwrap_err();
        assert!(matches!(error, GestureError::Program(_)));
    }
}
