//! The rotation target and its admissibility checks.

use crate::error::GestureError;
use crate::matrix::Matrix;

/// Default admissibility tolerance, matching the Python oracle's
/// `compile_so_on_tree(tolerance=1e-9)`.
pub const DEFAULT_TARGET_TOLERANCE: f64 = 1.0e-9;

/// A validated `SO(n)` target in energy-normalized coordinates.
///
/// Validation is deliberately front-loaded: by the time the compiler sees a
/// `RotationTarget` it knows the matrix is square, finite, orthogonal within
/// `tolerance`, and has unit determinant. A reflection (`det = -1`) never
/// reaches the elimination loop — it is rejected as
/// [`GestureError::TargetDeterminantNotUnit`], which classes as
/// [`crate::ErrorClass::Unreachable`].
#[derive(Debug, Clone, PartialEq)]
pub struct RotationTarget {
    matrix: Matrix,
    tolerance: f64,
    orthogonality_residual: f64,
    determinant: f64,
}

impl RotationTarget {
    /// Validate a matrix as an `SO(n)` target.
    pub fn new(matrix: Matrix, tolerance: f64) -> Result<Self, GestureError> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(GestureError::InvalidTolerance { tolerance });
        }
        let rows = matrix.rows();
        let columns = matrix.columns();
        if rows == 0 || rows != columns {
            return Err(GestureError::TargetShapeMismatch {
                expected: rows,
                rows,
                columns,
            });
        }
        if !matrix.is_finite() {
            return Err(GestureError::TargetNotFinite);
        }
        let orthogonality_residual = matrix.orthogonality_residual();
        if orthogonality_residual > tolerance {
            return Err(GestureError::TargetNotOrthogonal {
                residual: orthogonality_residual,
                tolerance,
            });
        }
        let determinant = matrix.determinant();
        if (determinant - 1.0).abs() > tolerance {
            return Err(GestureError::TargetDeterminantNotUnit {
                determinant,
                tolerance,
            });
        }
        Ok(Self {
            matrix,
            tolerance,
            orthogonality_residual,
            determinant,
        })
    }

    /// Validate with [`DEFAULT_TARGET_TOLERANCE`].
    pub fn from_rows(rows: Vec<Vec<f64>>) -> Result<Self, GestureError> {
        Self::new(Matrix::from_rows(rows)?, DEFAULT_TARGET_TOLERANCE)
    }

    /// The `n x n` identity as a target.
    pub fn identity(dimension: usize) -> Result<Self, GestureError> {
        Self::new(Matrix::identity(dimension), DEFAULT_TARGET_TOLERANCE)
    }

    pub fn matrix(&self) -> &Matrix {
        &self.matrix
    }

    pub fn dimension(&self) -> usize {
        self.matrix.rows()
    }

    pub fn tolerance(&self) -> f64 {
        self.tolerance
    }

    /// `||R^T R - I||_F` measured at validation time.
    pub fn orthogonality_residual(&self) -> f64 {
        self.orthogonality_residual
    }

    /// `det R` measured at validation time.
    pub fn determinant(&self) -> f64 {
        self.determinant
    }

    /// Check the target against a graph's dimension.
    pub fn require_dimension(&self, expected: usize) -> Result<(), GestureError> {
        if self.dimension() != expected {
            return Err(GestureError::TargetShapeMismatch {
                expected,
                rows: self.matrix.rows(),
                columns: self.matrix.columns(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane_rotation(n: usize, i: usize, j: usize, theta: f64) -> Matrix {
        let mut out = Matrix::identity(n);
        let (sin, cos) = theta.sin_cos();
        out.set(i, i, cos);
        out.set(i, j, -sin);
        out.set(j, i, sin);
        out.set(j, j, cos);
        out
    }

    #[test]
    fn a_plane_rotation_is_admissible() {
        let target = RotationTarget::new(plane_rotation(4, 1, 3, 0.7), 1.0e-9).unwrap();
        assert_eq!(target.dimension(), 4);
        assert!(target.orthogonality_residual() <= 1.0e-14);
        assert!((target.determinant() - 1.0).abs() <= 1.0e-14);
    }

    #[test]
    fn a_reflection_is_rejected_as_unreachable() {
        let mut reflection = Matrix::identity(3);
        reflection.set(0, 0, -1.0);
        let error = RotationTarget::new(reflection, 1.0e-9).unwrap_err();
        match error {
            GestureError::TargetDeterminantNotUnit { determinant, .. } => {
                assert!((determinant - -1.0).abs() <= 1.0e-14);
            }
            other => panic!("expected a determinant rejection, got {other}"),
        }
        assert_eq!(
            crate::ErrorClass::Unreachable,
            GestureError::TargetDeterminantNotUnit {
                determinant: -1.0,
                tolerance: 1.0e-9
            }
            .class()
        );
    }

    #[test]
    fn a_non_orthogonal_matrix_is_rejected() {
        let mut scaled = Matrix::identity(3);
        scaled.set(2, 2, 2.0);
        let error = RotationTarget::new(scaled, 1.0e-9).unwrap_err();
        assert!(matches!(error, GestureError::TargetNotOrthogonal { .. }));
    }

    #[test]
    fn shape_and_finiteness_and_tolerance_are_checked() {
        let rectangular = Matrix::zeros(2, 3);
        assert!(matches!(
            RotationTarget::new(rectangular, 1.0e-9).unwrap_err(),
            GestureError::TargetShapeMismatch { .. }
        ));
        let mut broken = Matrix::identity(2);
        broken.set(0, 1, f64::NAN);
        assert_eq!(
            RotationTarget::new(broken, 1.0e-9).unwrap_err(),
            GestureError::TargetNotFinite
        );
        assert!(matches!(
            RotationTarget::new(Matrix::identity(2), 0.0).unwrap_err(),
            GestureError::InvalidTolerance { .. }
        ));
    }
}
