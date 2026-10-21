//! Lowest eigenpairs of a real symmetric matrix, allocation-free after
//! construction.
//!
//! Householder reduction to tridiagonal form (the reflectors are kept, not
//! accumulated), implicit-shift QL for every eigenvalue of the tridiagonal
//! matrix, inverse iteration with partial pivoting for the lowest `r`
//! eigenvectors only, Gram-Schmidt inside eigenvalue clusters, and
//! back-transformation through the stored reflectors. This is the textbook
//! route (Golub and Van Loan, *Matrix Computations*, 4th ed., 8.3 and 8.2.2);
//! it costs about `4/3 n^3 + 30 n^2 + 2 r n^2` flops instead of the `~7 n^3`
//! of a full eigenvector accumulation.

/// Reusable workspace for one matrix size and one requested rank.
#[derive(Debug, Clone)]
pub(crate) struct SymmetricEigen {
    n: usize,
    rank: usize,
    /// Row-major `n x n` working copy; the strict lower triangle holds the
    /// Householder vectors after reduction.
    work: Vec<f64>,
    beta: Vec<f64>,
    diagonal: Vec<f64>,
    off_diagonal: Vec<f64>,
    eigen_scratch_d: Vec<f64>,
    eigen_scratch_e: Vec<f64>,
    sorted: Vec<f64>,
    p: Vec<f64>,
    w: Vec<f64>,
    // Tridiagonal LU with partial pivoting.
    u0: Vec<f64>,
    u1: Vec<f64>,
    u2: Vec<f64>,
    lower: Vec<f64>,
    swapped: Vec<bool>,
    y: Vec<f64>,
    /// Tridiagonal-basis eigenvectors, column `k` at `[k * n ..]`.
    tri_vectors: Vec<f64>,
}

/// Why an eigen solve failed. Every failure refuses the configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EigenError {
    NonFinite,
    NoConvergence,
}

impl SymmetricEigen {
    pub(crate) fn new(n: usize, rank: usize) -> Self {
        assert!(rank <= n && n >= 2, "eigen workspace needs 1 <= rank <= n, n >= 2");
        Self {
            n,
            rank,
            work: vec![0.0; n * n],
            beta: vec![0.0; n],
            diagonal: vec![0.0; n],
            off_diagonal: vec![0.0; n],
            eigen_scratch_d: vec![0.0; n],
            eigen_scratch_e: vec![0.0; n],
            sorted: vec![0.0; n],
            p: vec![0.0; n],
            w: vec![0.0; n],
            u0: vec![0.0; n],
            u1: vec![0.0; n],
            u2: vec![0.0; n],
            lower: vec![0.0; n],
            swapped: vec![false; n],
            y: vec![0.0; n],
            tri_vectors: vec![0.0; n * rank],
        }
    }

    /// Solve for the `rank` lowest eigenpairs of the symmetric `matrix`
    /// (row-major `n x n`; only the lower triangle is read).
    ///
    /// `values[k]` ascending; `vectors[k * n + i]` is component `i` of the
    /// `k`-th unit eigenvector.
    pub(crate) fn lowest(
        &mut self,
        matrix: &[f64],
        values: &mut [f64],
        vectors: &mut [f64],
    ) -> Result<(), EigenError> {
        let n = self.n;
        let rank = self.rank;
        debug_assert_eq!(matrix.len(), n * n);
        debug_assert!(values.len() >= rank && vectors.len() >= rank * n);
        if matrix.iter().any(|value| !value.is_finite()) {
            return Err(EigenError::NonFinite);
        }
        self.work.copy_from_slice(matrix);
        self.householder();
        self.eigen_scratch_d.copy_from_slice(&self.diagonal);
        self.eigen_scratch_e.copy_from_slice(&self.off_diagonal);
        tridiagonal_ql_values(&mut self.eigen_scratch_d, &mut self.eigen_scratch_e)?;
        self.sorted.copy_from_slice(&self.eigen_scratch_d);
        self.sorted.sort_by(f64::total_cmp);
        let scale = self
            .sorted
            .iter()
            .fold(0.0_f64, |acc, value| acc.max(value.abs()))
            .max(f64::MIN_POSITIVE);
        for k in 0..rank {
            values[k] = self.sorted[k];
            self.inverse_iteration(k, scale)?;
        }
        // Back-transform each tridiagonal eigenvector through the reflectors.
        for k in 0..rank {
            let column = &mut vectors[k * n..(k + 1) * n];
            column.copy_from_slice(&self.tri_vectors[k * n..(k + 1) * n]);
            for step in (0..n.saturating_sub(2)).rev() {
                let beta = self.beta[step];
                if beta == 0.0 {
                    continue;
                }
                // v lives in work[(i, step)] for i in step+1..n, with v[step+1] = 1.
                let mut dot = column[step + 1];
                for i in step + 2..n {
                    dot += self.work[i * n + step] * column[i];
                }
                let scaled = beta * dot;
                column[step + 1] -= scaled;
                for i in step + 2..n {
                    column[i] -= scaled * self.work[i * n + step];
                }
            }
        }
        if values[..rank].iter().any(|v| !v.is_finite())
            || vectors[..rank * n].iter().any(|v| !v.is_finite())
        {
            return Err(EigenError::NonFinite);
        }
        Ok(())
    }

    /// Householder tridiagonalization of `work` (lower triangle), keeping the
    /// reflectors in the strict lower triangle below the subdiagonal.
    fn householder(&mut self) {
        let n = self.n;
        // Symmetrize from the lower triangle so the rank-2 updates see one matrix.
        for i in 0..n {
            for j in 0..i {
                self.work[j * n + i] = self.work[i * n + j];
            }
        }
        for k in 0..n.saturating_sub(2) {
            // x = work[k+1.., k]
            let mut sigma = 0.0;
            for i in k + 2..n {
                let v = self.work[i * n + k];
                sigma += v * v;
            }
            let x0 = self.work[(k + 1) * n + k];
            if sigma == 0.0 {
                self.beta[k] = 0.0;
                self.diagonal[k] = self.work[k * n + k];
                self.off_diagonal[k] = x0;
                continue;
            }
            let mu = (x0 * x0 + sigma).sqrt();
            let v0 = if x0 <= 0.0 { x0 - mu } else { -sigma / (x0 + mu) };
            let beta = 2.0 * v0 * v0 / (sigma + v0 * v0);
            // Normalize v so v[k+1] = 1.
            for i in k + 2..n {
                self.work[i * n + k] /= v0;
            }
            self.beta[k] = beta;
            self.diagonal[k] = self.work[k * n + k];
            // (I - beta v v^T) x = mu e1 for either branch of v0 (GVL 5.1.1).
            self.off_diagonal[k] = mu;
            // p = beta * A22 v, with A22 = work[k+1.., k+1..]
            for i in k + 1..n {
                let mut acc = self.work[i * n + (k + 1)];
                for j in k + 2..n {
                    acc += self.work[i * n + j] * self.work[j * n + k];
                }
                self.p[i] = beta * acc;
            }
            // w = p - (beta/2)(p.v) v
            let mut pv = self.p[k + 1];
            for i in k + 2..n {
                pv += self.p[i] * self.work[i * n + k];
            }
            let half = 0.5 * beta * pv;
            self.w[k + 1] = self.p[k + 1] - half;
            for i in k + 2..n {
                self.w[i] = self.p[i] - half * self.work[i * n + k];
            }
            // A22 -= v w^T + w v^T (full square, keeps symmetry).
            for i in k + 1..n {
                let vi = if i == k + 1 { 1.0 } else { self.work[i * n + k] };
                let wi = self.w[i];
                for j in k + 1..n {
                    let vj = if j == k + 1 { 1.0 } else { self.work[j * n + k] };
                    self.work[i * n + j] -= vi * self.w[j] + wi * vj;
                }
            }
        }
        if n >= 2 {
            self.diagonal[n - 2] = self.work[(n - 2) * n + (n - 2)];
            self.off_diagonal[n - 2] = self.work[(n - 1) * n + (n - 2)];
            self.beta[n - 2] = 0.0;
        }
        self.diagonal[n - 1] = self.work[(n - 1) * n + (n - 1)];
        self.off_diagonal[n - 1] = 0.0;
        self.beta[n - 1] = 0.0;
    }

    /// Eigenvector `k` of the tridiagonal matrix for eigenvalue `sorted[k]`.
    fn inverse_iteration(&mut self, k: usize, scale: f64) -> Result<(), EigenError> {
        let n = self.n;
        let lambda = self.sorted[k];
        // A tiny shift keeps the factorization nonsingular; inverse iteration
        // converges to the eigenvector regardless (and faster).
        let shift = lambda + 4.0 * f64::EPSILON * scale;
        self.factor(shift, scale);
        // Deterministic start vector without special structure.
        for i in 0..n {
            self.y[i] = 1.0 + 0.5 * ((i as f64 + 1.0) * 0.618_033_988_749_895 * (k as f64 + 1.3)).sin();
        }
        // Cluster: earlier eigenvalues within this gap get orthogonalized out.
        let cluster_tol = 1.0e-9 * scale;
        for _ in 0..3 {
            self.solve_in_place();
            for j in 0..k {
                if (self.sorted[j] - lambda).abs() <= cluster_tol {
                    let column = &self.tri_vectors[j * n..(j + 1) * n];
                    let dot: f64 = column.iter().zip(&self.y).map(|(a, b)| a * b).sum();
                    for (yi, ci) in self.y.iter_mut().zip(column) {
                        *yi -= dot * ci;
                    }
                }
            }
            let norm = self.y.iter().map(|v| v * v).sum::<f64>().sqrt();
            if !norm.is_finite() || norm == 0.0 {
                return Err(EigenError::NoConvergence);
            }
            for yi in &mut self.y {
                *yi /= norm;
            }
        }
        self.tri_vectors[k * n..(k + 1) * n].copy_from_slice(&self.y);
        Ok(())
    }

    /// LU with partial pivoting of `T - shift I` into `u0/u1/u2/lower/swapped`.
    fn factor(&mut self, shift: f64, scale: f64) {
        let n = self.n;
        let tiny = f64::EPSILON * scale;
        let mut cur_d = self.diagonal[0] - shift;
        let mut cur_s1 = if n > 1 { self.off_diagonal[0] } else { 0.0 };
        for i in 0..n - 1 {
            let below = self.off_diagonal[i];
            let next_d = self.diagonal[i + 1] - shift;
            let next_s1 = if i + 1 < n - 1 { self.off_diagonal[i + 1] } else { 0.0 };
            if cur_d.abs() >= below.abs() {
                let pivot = if cur_d == 0.0 { tiny } else { cur_d };
                let l = below / pivot;
                self.u0[i] = pivot;
                self.u1[i] = cur_s1;
                self.u2[i] = 0.0;
                self.lower[i] = l;
                self.swapped[i] = false;
                cur_d = next_d - l * cur_s1;
                cur_s1 = next_s1;
            } else {
                let l = cur_d / below;
                self.u0[i] = below;
                self.u1[i] = next_d;
                self.u2[i] = next_s1;
                self.lower[i] = l;
                self.swapped[i] = true;
                cur_d = cur_s1 - l * next_d;
                cur_s1 = -l * next_s1;
            }
        }
        self.u0[n - 1] = if cur_d.abs() < tiny { tiny.copysign(if cur_d == 0.0 { 1.0 } else { cur_d }) } else { cur_d };
        self.u1[n - 1] = 0.0;
        self.u2[n - 1] = 0.0;
    }

    fn solve_in_place(&mut self) {
        let n = self.n;
        for i in 0..n - 1 {
            if self.swapped[i] {
                self.y.swap(i, i + 1);
            }
            self.y[i + 1] -= self.lower[i] * self.y[i];
        }
        for i in (0..n).rev() {
            let mut value = self.y[i];
            if i + 1 < n {
                value -= self.u1[i] * self.y[i + 1];
            }
            if i + 2 < n {
                value -= self.u2[i] * self.y[i + 2];
            }
            self.y[i] = value / self.u0[i];
        }
    }
}

/// Every eigenvalue of the symmetric tridiagonal matrix with diagonal `d` and
/// off-diagonal `e` (`e[i]` couples `i` and `i + 1`; `e[n-1]` ignored), by
/// implicit-shift QL. Overwrites `d` with the (unsorted) eigenvalues.
fn tridiagonal_ql_values(d: &mut [f64], e: &mut [f64]) -> Result<(), EigenError> {
    let n = d.len();
    if n == 0 {
        return Ok(());
    }
    e[n - 1] = 0.0;
    for l in 0..n {
        let mut iterations = 0;
        loop {
            let mut m = l;
            while m < n - 1 {
                let dd = d[m].abs() + d[m + 1].abs();
                if e[m].abs() <= f64::EPSILON * dd {
                    break;
                }
                m += 1;
            }
            if m == l {
                break;
            }
            iterations += 1;
            if iterations > 64 {
                return Err(EigenError::NoConvergence);
            }
            let mut g = (d[l + 1] - d[l]) / (2.0 * e[l]);
            let mut r = g.hypot(1.0);
            g = d[m] - d[l] + e[l] / (g + r.copysign(g));
            let mut s = 1.0;
            let mut c = 1.0;
            let mut p = 0.0;
            let mut i = m;
            let mut deflated = false;
            while i > l {
                i -= 1;
                let f = s * e[i];
                let b = c * e[i];
                r = f.hypot(g);
                e[i + 1] = r;
                if r == 0.0 {
                    d[i + 1] -= p;
                    e[m] = 0.0;
                    deflated = true;
                    break;
                }
                s = f / r;
                c = g / r;
                g = d[i + 1] - p;
                r = (d[i] - g) * s + 2.0 * c * b;
                p = s * r;
                d[i + 1] = g + p;
                g = c * r - b;
            }
            if deflated {
                continue;
            }
            d[l] -= p;
            e[l] = g;
            e[m] = 0.0;
        }
    }
    if d.iter().any(|value| !value.is_finite()) {
        return Err(EigenError::NonFinite);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_symmetric(n: usize, seed: u64) -> Vec<f64> {
        let mut state = seed;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1u64 << 53) as f64 - 0.5
        };
        let mut a = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..=i {
                let v = next();
                a[i * n + j] = v;
                a[j * n + i] = v;
            }
        }
        a
    }

    fn check(a: &[f64], n: usize, rank: usize) -> (f64, f64) {
        let mut solver = SymmetricEigen::new(n, rank);
        let mut values = vec![0.0; rank];
        let mut vectors = vec![0.0; rank * n];
        solver.lowest(a, &mut values, &mut vectors).unwrap();
        let mut worst_residual = 0.0_f64;
        let mut worst_orth = 0.0_f64;
        for k in 0..rank {
            let v = &vectors[k * n..(k + 1) * n];
            for i in 0..n {
                let av: f64 = (0..n).map(|j| a[i * n + j] * v[j]).sum();
                worst_residual = worst_residual.max((av - values[k] * v[i]).abs());
            }
            for l in 0..rank {
                let w = &vectors[l * n..(l + 1) * n];
                let dot: f64 = v.iter().zip(w).map(|(a, b)| a * b).sum();
                let target = if k == l { 1.0 } else { 0.0 };
                worst_orth = worst_orth.max((dot - target).abs());
            }
        }
        for k in 1..rank {
            assert!(values[k] >= values[k - 1]);
        }
        (worst_residual, worst_orth)
    }

    #[test]
    fn random_dense_symmetric_matrices_resolve_to_roundoff() {
        for (n, seed) in [(5, 1), (24, 2), (48, 3), (75, 4)] {
            let a = random_symmetric(n, seed);
            let (residual, orth) = check(&a, n, n.min(24));
            assert!(residual < 1.0e-12, "n={n} residual {residual}");
            assert!(orth < 1.0e-12, "n={n} orthogonality {orth}");
        }
    }

    #[test]
    fn path_laplacian_matches_closed_form() {
        // -1 2 -1 path with Dirichlet ends: eigenvalues 2 - 2 cos(k pi/(n+1)).
        let n = 40;
        let mut a = vec![0.0; n * n];
        for i in 0..n {
            a[i * n + i] = 2.0;
            if i + 1 < n {
                a[i * n + i + 1] = -1.0;
                a[(i + 1) * n + i] = -1.0;
            }
        }
        let mut solver = SymmetricEigen::new(n, 10);
        let mut values = vec![0.0; 10];
        let mut vectors = vec![0.0; 10 * n];
        solver.lowest(&a, &mut values, &mut vectors).unwrap();
        for (k, value) in values.iter().enumerate() {
            let exact = 2.0 - 2.0 * ((k as f64 + 1.0) * std::f64::consts::PI / (n as f64 + 1.0)).cos();
            assert!((value - exact).abs() < 1.0e-13, "k={k}: {value} vs {exact}");
        }
    }

    #[test]
    fn degenerate_eigenvalues_still_give_an_orthonormal_basis() {
        // Two identical decoupled blocks: every eigenvalue is doubled.
        let block = 12;
        let n = 2 * block;
        let mut a = vec![0.0; n * n];
        for b in 0..2 {
            for i in 0..block {
                let g = b * block + i;
                a[g * n + g] = 2.0;
                if i + 1 < block {
                    a[g * n + g + 1] = -1.0;
                    a[(g + 1) * n + g] = -1.0;
                }
            }
        }
        let (residual, orth) = check(&a, n, 16);
        assert!(residual < 1.0e-12, "residual {residual}");
        assert!(orth < 1.0e-10, "orthogonality {orth}");
    }
}
