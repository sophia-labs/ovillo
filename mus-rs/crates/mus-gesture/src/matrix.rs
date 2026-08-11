//! A minimal row-major dense `f64` matrix.
//!
//! Deliberately not a linear-algebra dependency: the compiler works at
//! `n <= 32`, where a dense `Vec<f64>` is trivial, and every use here is
//! construction-time. Nothing in this module runs in an audio callback.
//!
//! The one rotation primitive, [`Matrix::rotate_rows`], delegates to
//! [`mus_watsn::givens`] rather than recomputing the plane-rotation kernel —
//! consensus rule 1 forbids a second Givens routine in this crate.

use crate::error::GestureError;

/// Row-major dense matrix of `f64`.
#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    rows: usize,
    columns: usize,
    data: Vec<f64>,
}

impl Matrix {
    /// All-zero `rows x columns` matrix.
    pub fn zeros(rows: usize, columns: usize) -> Self {
        Self {
            rows,
            columns,
            data: vec![0.0; rows * columns],
        }
    }

    /// `n x n` identity.
    pub fn identity(n: usize) -> Self {
        let mut out = Self::zeros(n, n);
        for i in 0..n {
            out.set(i, i, 1.0);
        }
        out
    }

    /// Build from a rectangular row list. Every row must have equal length.
    pub fn from_rows(rows: Vec<Vec<f64>>) -> Result<Self, GestureError> {
        let row_count = rows.len();
        let columns = rows.first().map_or(0, Vec::len);
        let mut data = Vec::with_capacity(row_count * columns);
        for row in rows {
            if row.len() != columns {
                return Err(GestureError::RaggedMatrix {
                    expected: columns,
                    actual: row.len(),
                });
            }
            data.extend_from_slice(&row);
        }
        Ok(Self {
            rows: row_count,
            columns,
            data,
        })
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    #[inline]
    pub fn get(&self, row: usize, column: usize) -> f64 {
        self.data[row * self.columns + column]
    }

    #[inline]
    pub fn set(&mut self, row: usize, column: usize, value: f64) {
        self.data[row * self.columns + column] = value;
    }

    /// Row-major backing slice, in the order receipts and digests consume it.
    pub fn as_slice(&self) -> &[f64] {
        &self.data
    }

    /// Nested row form, for serde receipts.
    pub fn to_rows(&self) -> Vec<Vec<f64>> {
        (0..self.rows)
            .map(|row| self.data[row * self.columns..(row + 1) * self.columns].to_vec())
            .collect()
    }

    pub fn is_finite(&self) -> bool {
        self.data.iter().all(|value| value.is_finite())
    }

    /// `self^T`.
    pub fn transpose(&self) -> Self {
        let mut out = Self::zeros(self.columns, self.rows);
        for row in 0..self.rows {
            for column in 0..self.columns {
                out.set(column, row, self.get(row, column));
            }
        }
        out
    }

    /// Dense `self * other`.
    pub fn multiply(&self, other: &Self) -> Self {
        assert_eq!(
            self.columns, other.rows,
            "matrix multiply shape mismatch: {}x{} * {}x{}",
            self.rows, self.columns, other.rows, other.columns
        );
        let mut out = Self::zeros(self.rows, other.columns);
        for row in 0..self.rows {
            for inner in 0..self.columns {
                let left = self.get(row, inner);
                if left == 0.0 {
                    continue;
                }
                for column in 0..other.columns {
                    let value = out.get(row, column) + left * other.get(inner, column);
                    out.set(row, column, value);
                }
            }
        }
        out
    }

    /// Frobenius norm of `self - other`. Shapes must agree.
    pub fn frobenius_distance(&self, other: &Self) -> f64 {
        assert_eq!(self.rows, other.rows);
        assert_eq!(self.columns, other.columns);
        self.data
            .iter()
            .zip(other.data.iter())
            .map(|(left, right)| {
                let delta = left - right;
                delta * delta
            })
            .sum::<f64>()
            .sqrt()
    }

    /// `||self^T self - I||_F`, the orthogonality residual.
    pub fn orthogonality_residual(&self) -> f64 {
        let gram = self.transpose().multiply(self);
        gram.frobenius_distance(&Self::identity(self.rows.min(self.columns)))
    }

    /// Determinant by LU with partial pivoting. Square matrices only.
    pub fn determinant(&self) -> f64 {
        assert_eq!(self.rows, self.columns, "determinant needs a square matrix");
        let n = self.rows;
        if n == 0 {
            return 1.0;
        }
        let mut work = self.clone();
        let mut determinant = 1.0_f64;
        for column in 0..n {
            let mut pivot = column;
            let mut best = work.get(column, column).abs();
            for row in (column + 1)..n {
                let candidate = work.get(row, column).abs();
                if candidate > best {
                    best = candidate;
                    pivot = row;
                }
            }
            if best == 0.0 {
                return 0.0;
            }
            if pivot != column {
                for index in 0..n {
                    let left = work.get(column, index);
                    let right = work.get(pivot, index);
                    work.set(column, index, right);
                    work.set(pivot, index, left);
                }
                determinant = -determinant;
            }
            let head = work.get(column, column);
            determinant *= head;
            for row in (column + 1)..n {
                let factor = work.get(row, column) / head;
                if factor == 0.0 {
                    continue;
                }
                for index in column..n {
                    let value = work.get(row, index) - factor * work.get(column, index);
                    work.set(row, index, value);
                }
            }
        }
        determinant
    }

    /// Left-multiply in place by the plane rotation `G(left, right, theta)`,
    /// i.e. `self <- G * self`.
    ///
    /// `G` is the identity with `G[left][left] = cos`, `G[left][right] = -sin`,
    /// `G[right][left] = sin`, `G[right][right] = cos` — the same convention as
    /// [`mus_watsn::givens`], which supplies the kernel.
    pub fn rotate_rows(&mut self, left: usize, right: usize, theta: f64) {
        debug_assert_ne!(left, right);
        for column in 0..self.columns {
            let (updated_left, updated_right) =
                mus_watsn::givens(self.get(left, column), self.get(right, column), theta);
            self.set(left, column, updated_left);
            self.set(right, column, updated_right);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_rows_matches_dense_left_multiplication() {
        let mut dense = Matrix::from_rows(vec![
            vec![1.0, 2.0, 3.0],
            vec![4.0, 5.0, 6.0],
            vec![7.0, 8.0, 9.0],
        ])
        .unwrap();
        let theta = 0.37_f64;
        let (sin, cos) = theta.sin_cos();
        let rotation = Matrix::from_rows(vec![
            vec![cos, 0.0, -sin],
            vec![0.0, 1.0, 0.0],
            vec![sin, 0.0, cos],
        ])
        .unwrap();
        let expected = rotation.multiply(&dense);
        dense.rotate_rows(0, 2, theta);
        assert!(dense.frobenius_distance(&expected) <= 1.0e-15);
    }

    #[test]
    fn determinant_of_a_known_matrix() {
        let matrix = Matrix::from_rows(vec![
            vec![2.0, -1.0, 0.0],
            vec![1.0, 3.0, 4.0],
            vec![0.0, 5.0, -2.0],
        ])
        .unwrap();
        // 2*(3*-2 - 4*5) + 1*(1*-2 - 4*0) = 2*(-26) - 2 = -54
        assert!((matrix.determinant() - -54.0).abs() <= 1.0e-12);
    }

    #[test]
    fn identity_is_orthogonal_with_unit_determinant() {
        let identity = Matrix::identity(7);
        assert!(identity.orthogonality_residual() <= 1.0e-15);
        assert!((identity.determinant() - 1.0).abs() <= 1.0e-15);
    }

    #[test]
    fn ragged_rows_are_rejected() {
        let error = Matrix::from_rows(vec![vec![1.0, 2.0], vec![3.0]]).unwrap_err();
        assert_eq!(
            error,
            GestureError::RaggedMatrix {
                expected: 2,
                actual: 1
            }
        );
    }
}
