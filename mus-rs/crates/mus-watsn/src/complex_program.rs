//! Lawful complex factor programs (unified-instrument stage U1).
//!
//! The protected conservative control group is `U(16)`, realified into the
//! orthogonal-symplectic subgroup of `SO(32)` — NOT arbitrary real plane
//! rotations. This module is the complex sibling of [`crate::program`]:
//! graph-edge `U(2)` two-mode blocks plus local `U(1)` phases, validated at
//! construction (finiteness, per-factor unitarity, authored edges) and
//! executed allocation-free per RUST-API-CONTRACT.md §4. The composition
//! convention is `StateLeftMultiplyInListedExecutionOrder`: gate `k` acts on
//! the state produced by gates `0..k`.
//!
//! An `OperatorProgram` of real `PlaneRotation`s remains valid only for state
//! spaces interpreted as real scattering amplitudes; it must never be passed
//! off as a protected factor word (NEGATIVE-RESULTS.md N1). The executable
//! counterexample lives in [`real_givens_symplectic_residual`].

use crate::complex::{state_energy, Complex64};

/// Full `2 x 2` complex unitary acting on an authored graph edge.
/// `matrix` is row-major: `[m00, m01, m10, m11]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComplexTwoModeGate {
    pub left: usize,
    pub right: usize,
    pub matrix: [Complex64; 4],
}

impl ComplexTwoModeGate {
    /// Frobenius residual `||U* U - I||_F`.
    pub fn unitarity_residual(&self) -> f64 {
        let [a, b, c, d] = self.matrix;
        let g00 = a.conj() * a + c.conj() * c - Complex64::ONE;
        let g01 = a.conj() * b + c.conj() * d;
        let g11 = b.conj() * b + d.conj() * d - Complex64::ONE;
        (g00.abs_squared() + 2.0 * g01.abs_squared() + g11.abs_squared()).sqrt()
    }

    pub fn inverse(self) -> Self {
        let [a, b, c, d] = self.matrix;
        Self {
            matrix: [a.conj(), c.conj(), b.conj(), d.conj()],
            ..self
        }
    }
}

/// Local `U(1)` phase shift on one mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComplexPhaseGate {
    pub mode: usize,
    pub phase: f64,
}

impl ComplexPhaseGate {
    pub fn inverse(self) -> Self {
        Self {
            phase: -self.phase,
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComplexGate {
    TwoMode(ComplexTwoModeGate),
    Phase(ComplexPhaseGate),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComplexProgramError {
    DimensionMismatch { expected: usize, actual: usize },
    RepeatedCoordinate { gate_index: usize, index: usize },
    CoordinateOutOfRange { gate_index: usize, index: usize, dimension: usize },
    NonFiniteCoefficient { gate_index: usize },
    GateNotUnitary { gate_index: usize, residual: f64 },
    EdgeNotAuthored { gate_index: usize, left: usize, right: usize },
}

/// Receipt for one execution (RUST-API-CONTRACT.md §5). Digest fields are
/// deferred to the engine-side content-addressing layer; the numeric law
/// fields are computed here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComplexProgramReceipt {
    pub gate_count: u32,
    pub two_mode_gate_count: u32,
    pub phase_gate_count: u32,
    pub maximum_local_unitarity_error_fro: f64,
    pub final_norm_error: f64,
    /// Fixed-metric acoustic state work: `E_after - E_before`. Zero to
    /// floating residual for lawful unitary words. Never includes actuator
    /// overhead (NEGATIVE-RESULTS.md N8).
    pub measured_acoustic_state_work: f64,
}

/// Validated, ordered complex gate word.
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexOperatorProgram {
    dimension: usize,
    gates: Vec<ComplexGate>,
    maximum_local_unitarity_error_fro: f64,
}

impl ComplexOperatorProgram {
    pub const DEFAULT_UNITARITY_TOLERANCE: f64 = 1.0e-12;

    /// Validate and construct. `authored_edges` of `None` authorizes the
    /// complete graph; `Some(edges)` enforces membership (unordered pairs).
    pub fn new(
        dimension: usize,
        gates: Vec<ComplexGate>,
        authored_edges: Option<&[(usize, usize)]>,
        unitarity_tolerance: f64,
    ) -> Result<Self, ComplexProgramError> {
        let mut worst_unitarity = 0.0_f64;
        for (gate_index, gate) in gates.iter().enumerate() {
            match gate {
                ComplexGate::Phase(phase) => {
                    if !phase.phase.is_finite() {
                        return Err(ComplexProgramError::NonFiniteCoefficient { gate_index });
                    }
                    if phase.mode >= dimension {
                        return Err(ComplexProgramError::CoordinateOutOfRange {
                            gate_index,
                            index: phase.mode,
                            dimension,
                        });
                    }
                }
                ComplexGate::TwoMode(block) => {
                    if block.left == block.right {
                        return Err(ComplexProgramError::RepeatedCoordinate {
                            gate_index,
                            index: block.left,
                        });
                    }
                    for index in [block.left, block.right] {
                        if index >= dimension {
                            return Err(ComplexProgramError::CoordinateOutOfRange {
                                gate_index,
                                index,
                                dimension,
                            });
                        }
                    }
                    if block.matrix.iter().any(|entry| !entry.is_finite()) {
                        return Err(ComplexProgramError::NonFiniteCoefficient { gate_index });
                    }
                    let residual = block.unitarity_residual();
                    if residual > unitarity_tolerance {
                        return Err(ComplexProgramError::GateNotUnitary { gate_index, residual });
                    }
                    worst_unitarity = worst_unitarity.max(residual);
                    if let Some(edges) = authored_edges {
                        let canonical = (
                            block.left.min(block.right),
                            block.left.max(block.right),
                        );
                        let authored = edges
                            .iter()
                            .any(|&(a, b)| (a.min(b), a.max(b)) == canonical);
                        if !authored {
                            return Err(ComplexProgramError::EdgeNotAuthored {
                                gate_index,
                                left: block.left,
                                right: block.right,
                            });
                        }
                    }
                }
            }
        }
        Ok(Self {
            dimension,
            gates,
            maximum_local_unitarity_error_fro: worst_unitarity,
        })
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn gates(&self) -> &[ComplexGate] {
        &self.gates
    }

    /// Apply gates in listed execution order. Allocation-free.
    pub fn apply(&self, state: &mut [Complex64]) -> Result<(), ComplexProgramError> {
        if state.len() != self.dimension {
            return Err(ComplexProgramError::DimensionMismatch {
                expected: self.dimension,
                actual: state.len(),
            });
        }
        for gate in &self.gates {
            match gate {
                ComplexGate::Phase(phase) => {
                    state[phase.mode] = state[phase.mode] * Complex64::unit_phasor(phase.phase);
                }
                ComplexGate::TwoMode(block) => {
                    let [a, b, c, d] = block.matrix;
                    let left = state[block.left];
                    let right = state[block.right];
                    state[block.left] = a * left + b * right;
                    state[block.right] = c * left + d * right;
                }
            }
        }
        Ok(())
    }

    /// Apply and return the numeric law receipt.
    pub fn apply_with_receipt(
        &self,
        state: &mut [Complex64],
    ) -> Result<ComplexProgramReceipt, ComplexProgramError> {
        let energy_before = state_energy(state);
        self.apply(state)?;
        let energy_after = state_energy(state);
        let two_mode = self
            .gates
            .iter()
            .filter(|gate| matches!(gate, ComplexGate::TwoMode(_)))
            .count() as u32;
        Ok(ComplexProgramReceipt {
            gate_count: self.gates.len() as u32,
            two_mode_gate_count: two_mode,
            phase_gate_count: self.gates.len() as u32 - two_mode,
            maximum_local_unitarity_error_fro: self.maximum_local_unitarity_error_fro,
            final_norm_error: (energy_after.sqrt() - energy_before.sqrt()).abs(),
            measured_acoustic_state_work: energy_after - energy_before,
        })
    }

    /// Exact inverse word: reversed order, inverted factors.
    pub fn inverse(&self) -> Self {
        let gates = self
            .gates
            .iter()
            .rev()
            .map(|gate| match gate {
                ComplexGate::Phase(phase) => ComplexGate::Phase(phase.inverse()),
                ComplexGate::TwoMode(block) => ComplexGate::TwoMode(block.inverse()),
            })
            .collect();
        Self {
            dimension: self.dimension,
            gates,
            maximum_local_unitarity_error_fro: self.maximum_local_unitarity_error_fro,
        }
    }
}

/// Executable N1 counterexample: Frobenius residual `||G J - J G||_F` for a
/// real Givens rotation on quadrature coordinates of an `n_complex`-mode
/// oscillator state ordered `[q_0..q_{n-1}, p_0..p_{n-1}]`.
///
/// `paired = true` applies the same rotation to `(q_i, q_j)` AND `(p_i, p_j)`
/// — the realification of a real orthogonal two-mode block, which commutes
/// with the complex structure `J` (residual ~ 0). `paired = false` rotates
/// `(q_i, q_j)` alone — energy-preserving, yet it breaks `J` and is therefore
/// not a lawful positive-frequency mode transformation.
pub fn real_givens_symplectic_residual(
    n_complex: usize,
    i: usize,
    j: usize,
    angle: f64,
    paired: bool,
) -> f64 {
    let n = 2 * n_complex;
    let (sin, cos) = angle.sin_cos();
    let mut generator = vec![0.0_f64; n * n];
    for row in 0..n {
        generator[row * n + row] = 1.0;
    }
    let mut rotate = |a: usize, b: usize| {
        for column in 0..n {
            let top = generator[a * n + column];
            let bottom = generator[b * n + column];
            generator[a * n + column] = cos * top - sin * bottom;
            generator[b * n + column] = sin * top + cos * bottom;
        }
    };
    rotate(i, j);
    if paired {
        rotate(n_complex + i, n_complex + j);
    }
    // J maps (q, p) -> (-p, q); commutator residual entrywise.
    let index = |row: usize, column: usize| generator[row * n + column];
    let jg = |row: usize, column: usize| {
        if row < n_complex {
            -index(row + n_complex, column)
        } else {
            index(row - n_complex, column)
        }
    };
    let gj = |row: usize, column: usize| {
        if column < n_complex {
            index(row, column + n_complex)
        } else {
            -index(row, column - n_complex)
        }
    };
    let mut sum = 0.0;
    for row in 0..n {
        for column in 0..n {
            let difference = gj(row, column) - jg(row, column);
            sum += difference * difference;
        }
    }
    sum.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beam_splitter(left: usize, right: usize, angle: f64) -> ComplexGate {
        let (sin, cos) = angle.sin_cos();
        ComplexGate::TwoMode(ComplexTwoModeGate {
            left,
            right,
            matrix: [
                Complex64::new(cos, 0.0),
                Complex64::new(-sin, 0.0),
                Complex64::new(sin, 0.0),
                Complex64::new(cos, 0.0),
            ],
        })
    }

    #[test]
    fn program_preserves_energy_and_reports_zero_work() {
        let program = ComplexOperatorProgram::new(
            3,
            vec![
                beam_splitter(0, 1, 0.37),
                ComplexGate::Phase(ComplexPhaseGate { mode: 2, phase: 1.1 }),
                beam_splitter(1, 2, -0.9),
            ],
            None,
            ComplexOperatorProgram::DEFAULT_UNITARITY_TOLERANCE,
        )
        .unwrap();
        let mut state = [
            Complex64::new(0.6, -0.1),
            Complex64::new(-0.3, 0.8),
            Complex64::new(0.2, 0.4),
        ];
        let receipt = program.apply_with_receipt(&mut state).unwrap();
        assert!(receipt.measured_acoustic_state_work.abs() < 1.0e-14);
        assert_eq!(receipt.two_mode_gate_count, 2);
        assert_eq!(receipt.phase_gate_count, 1);
    }

    #[test]
    fn inverse_word_recovers_state() {
        let program = ComplexOperatorProgram::new(
            2,
            vec![
                beam_splitter(0, 1, 0.61),
                ComplexGate::Phase(ComplexPhaseGate { mode: 0, phase: -0.4 }),
            ],
            None,
            ComplexOperatorProgram::DEFAULT_UNITARITY_TOLERANCE,
        )
        .unwrap();
        let original = [Complex64::new(0.8, 0.1), Complex64::new(-0.2, 0.55)];
        let mut state = original;
        program.apply(&mut state).unwrap();
        program.inverse().apply(&mut state).unwrap();
        for (actual, expected) in state.iter().zip(original.iter()) {
            assert!((*actual - *expected).abs() < 2.0e-15);
        }
    }

    #[test]
    fn non_unitary_block_is_rejected() {
        let mut gate = match beam_splitter(0, 1, 0.5) {
            ComplexGate::TwoMode(block) => block,
            ComplexGate::Phase(_) => unreachable!(),
        };
        gate.matrix[0] = gate.matrix[0].scale(1.01);
        let result = ComplexOperatorProgram::new(
            2,
            vec![ComplexGate::TwoMode(gate)],
            None,
            ComplexOperatorProgram::DEFAULT_UNITARITY_TOLERANCE,
        );
        assert!(matches!(
            result,
            Err(ComplexProgramError::GateNotUnitary { .. })
        ));
    }

    #[test]
    fn unauthored_edge_is_rejected() {
        let result = ComplexOperatorProgram::new(
            4,
            vec![beam_splitter(0, 3, 0.5)],
            Some(&[(0, 1), (1, 2), (2, 3)]),
            ComplexOperatorProgram::DEFAULT_UNITARITY_TOLERANCE,
        );
        assert!(matches!(
            result,
            Err(ComplexProgramError::EdgeNotAuthored { .. })
        ));
    }

    #[test]
    fn non_finite_coefficient_is_rejected() {
        let result = ComplexOperatorProgram::new(
            2,
            vec![ComplexGate::Phase(ComplexPhaseGate {
                mode: 0,
                phase: f64::NAN,
            })],
            None,
            ComplexOperatorProgram::DEFAULT_UNITARITY_TOLERANCE,
        );
        assert!(matches!(
            result,
            Err(ComplexProgramError::NonFiniteCoefficient { .. })
        ));
    }

    #[test]
    fn lone_quadrature_givens_breaks_complex_structure() {
        let unpaired = real_givens_symplectic_residual(2, 0, 1, 0.7, false);
        let paired = real_givens_symplectic_residual(2, 0, 1, 0.7, true);
        assert!(unpaired > 0.5, "unpaired residual {unpaired}");
        assert!(paired < 1.0e-14, "paired residual {paired}");
    }
}
