//! Warm-started lowest eigenpairs of a symmetric matrix whose graph is a tree.
//!
//! A tube network's `C^-1/2 K C^-1/2` is tridiagonal except where a branch
//! joins its trunk: its off-diagonal graph is a tree. Eliminating leaves first,
//! `S - lambda I = L D L^T` has no fill, so a solve or a Sylvester inertia
//! count (the number of negative pivots is the number of eigenvalues below
//! `lambda`) costs `O(n)`.
//!
//! Between two nearby configuration points the eigenvectors barely move, so
//! each mode is refined from the previous point's vector by Rayleigh-quotient
//! iteration (cubic convergence; 2-4 tree solves per mode). The result is
//! accepted only when it is certified: every residual `|S x - lambda x|` is at
//! roundoff of `|S|`, the vectors are orthonormal after a final Gram-Schmidt
//! pass, the eigenvalues are distinct, and the inertia count proves that no
//! lower eigenvalue was skipped. Otherwise the caller falls back to the dense
//! solve (`eigen::SymmetricEigen`). Either way the returned pairs are the
//! lowest `rank` eigenpairs to roundoff.

const MAX_ITERATIONS: usize = 6;

#[derive(Debug, Clone)]
pub(crate) struct TreeEigen {
    n: usize,
    rank: usize,
    parent: Vec<usize>,
    /// Children before parents.
    order: Vec<usize>,
    pub(crate) diagonal: Vec<f64>,
    /// `off[c] = S[c][parent(c)]` (unused for the root).
    pub(crate) off: Vec<f64>,
    pivot: Vec<f64>,
    y: Vec<f64>,
    x: Vec<f64>,
    sx: Vec<f64>,
    scratch_values: Vec<f64>,
    scratch_vectors: Vec<f64>,
    index: Vec<usize>,
}

const ROOT: usize = usize::MAX;

impl TreeEigen {
    /// `edges` are the matrix's off-diagonal pairs. Returns `None` unless they
    /// form a spanning tree of `0..n`.
    pub(crate) fn new(n: usize, rank: usize, edges: &[(usize, usize)]) -> Option<Self> {
        if edges.len() + 1 != n {
            return None;
        }
        let mut adjacency = vec![Vec::new(); n];
        for &(a, b) in edges {
            if a >= n || b >= n || a == b {
                return None;
            }
            adjacency[a].push(b);
            adjacency[b].push(a);
        }
        let mut parent = vec![ROOT; n];
        let mut seen = vec![false; n];
        let mut bfs = Vec::with_capacity(n);
        bfs.push(0);
        seen[0] = true;
        let mut head = 0;
        while head < bfs.len() {
            let node = bfs[head];
            head += 1;
            for &next in &adjacency[node] {
                if !seen[next] {
                    seen[next] = true;
                    parent[next] = node;
                    bfs.push(next);
                }
            }
        }
        if bfs.len() != n {
            return None;
        }
        bfs.reverse();
        Some(Self {
            n,
            rank,
            parent,
            order: bfs,
            diagonal: vec![0.0; n],
            off: vec![0.0; n],
            pivot: vec![0.0; n],
            y: vec![0.0; n],
            x: vec![0.0; n],
            sx: vec![0.0; n],
            scratch_values: vec![0.0; rank],
            scratch_vectors: vec![0.0; rank * n],
            index: (0..rank).collect(),
        })
    }

    pub(crate) fn parent(&self, node: usize) -> Option<usize> {
        let p = self.parent[node];
        (p != ROOT).then_some(p)
    }

    /// Eigenvalues strictly below `lambda` (Sylvester inertia of `S - lambda I`).
    pub(crate) fn count_below(&mut self, lambda: f64) -> usize {
        let norm = (0..self.n)
            .map(|i| self.diagonal[i].abs() + 2.0 * self.off[i].abs())
            .fold(0.0_f64, f64::max)
            .max(f64::MIN_POSITIVE);
        self.factor(lambda, f64::EPSILON * norm)
    }

    /// `L D L^T` of `S - lambda I`; returns the number of negative pivots.
    fn factor(&mut self, lambda: f64, tiny: f64) -> usize {
        for i in 0..self.n {
            self.pivot[i] = self.diagonal[i] - lambda;
        }
        let mut negative = 0;
        for &c in &self.order {
            let mut d = self.pivot[c];
            if d == 0.0 || d.abs() < tiny {
                d = if d < 0.0 { -tiny } else { tiny };
                self.pivot[c] = d;
            }
            if d < 0.0 {
                negative += 1;
            }
            let p = self.parent[c];
            if p != ROOT {
                self.pivot[p] -= self.off[c] * self.off[c] / d;
            }
        }
        negative
    }

    /// Solve `(S - lambda I) x = y` in place in `y` with the last factor.
    fn solve(&mut self) {
        for &c in &self.order {
            let p = self.parent[c];
            if p != ROOT {
                self.y[p] -= self.off[c] * self.y[c] / self.pivot[c];
            }
        }
        for &c in self.order.iter().rev() {
            let p = self.parent[c];
            let upstream = if p == ROOT { 0.0 } else { self.off[c] * self.y[p] };
            self.y[c] = (self.y[c] - upstream) / self.pivot[c];
        }
    }

    fn apply(&mut self) {
        for i in 0..self.n {
            self.sx[i] = self.diagonal[i] * self.x[i];
        }
        for c in 0..self.n {
            let p = self.parent[c];
            if p != ROOT {
                self.sx[c] += self.off[c] * self.x[p];
                self.sx[p] += self.off[c] * self.x[c];
            }
        }
    }

    /// Refine `warm` (previous unit eigenvectors, `rank x n`) into the lowest
    /// `rank` eigenpairs of the current matrix. Returns false when the result
    /// cannot be certified (the caller must then use the dense solve).
    pub(crate) fn refine(&mut self, warm: &[f64], values: &mut [f64], vectors: &mut [f64]) -> bool {
        let n = self.n;
        let rank = self.rank;
        let norm_s = (0..n)
            .map(|i| self.diagonal[i].abs() + 2.0 * self.off[i].abs())
            .fold(0.0_f64, f64::max)
            .max(f64::MIN_POSITIVE);
        let tiny = f64::EPSILON * norm_s;
        let tolerance = 64.0 * f64::EPSILON * norm_s;
        for k in 0..rank {
            self.x.copy_from_slice(&warm[k * n..(k + 1) * n]);
            let mut lambda = f64::NAN;
            let mut converged = false;
            for iteration in 0..=MAX_ITERATIONS {
                // Keep away from modes already found near this one (the warm
                // vectors are orthonormal already; only near neighbours can
                // collapse onto each other).
                if iteration > 0 {
                    for j in 0..k {
                        if (self.scratch_values[j] - lambda).abs() < 0.05 * lambda.abs() {
                            let column = &self.scratch_vectors[j * n..(j + 1) * n];
                            let dot: f64 = column.iter().zip(&self.x).map(|(a, b)| a * b).sum();
                            for (xi, ci) in self.x.iter_mut().zip(column) {
                                *xi -= dot * ci;
                            }
                        }
                    }
                }
                let norm = self.x.iter().map(|v| v * v).sum::<f64>().sqrt();
                if !norm.is_finite() || norm == 0.0 {
                    return false;
                }
                let inverse = norm.recip();
                for v in &mut self.x {
                    *v *= inverse;
                }
                self.apply();
                let mut quotient = 0.0;
                for i in 0..n {
                    quotient += self.x[i] * self.sx[i];
                }
                lambda = quotient;
                // Directly: |Sx|^2 - lambda^2 cancels catastrophically.
                let mut residual2 = 0.0;
                for i in 0..n {
                    let r = self.sx[i] - lambda * self.x[i];
                    residual2 += r * r;
                }
                let residual = residual2.sqrt();
                if residual <= tolerance {
                    converged = true;
                    break;
                }
                if iteration == MAX_ITERATIONS {
                    break;
                }
                self.factor(lambda, tiny);
                std::mem::swap(&mut self.x, &mut self.y);
                self.solve();
                std::mem::swap(&mut self.x, &mut self.y);
            }
            if !converged || !lambda.is_finite() {
                return false;
            }
            self.scratch_values[k] = lambda;
            self.scratch_vectors[k * n..(k + 1) * n].copy_from_slice(&self.x);
        }
        // Sort ascending.
        for (i, slot) in self.index.iter_mut().enumerate() {
            *slot = i;
        }
        let values_ref = &self.scratch_values;
        self.index.sort_by(|&a, &b| values_ref[a].total_cmp(&values_ref[b]));
        for (to, &from) in self.index.iter().enumerate() {
            values[to] = self.scratch_values[from];
            vectors[to * n..(to + 1) * n].copy_from_slice(&self.scratch_vectors[from * n..(from + 1) * n]);
        }
        // Distinct, and orthonormal after one modified Gram-Schmidt pass that
        // must not move any vector by more than roundoff-level amounts.
        for k in 1..rank {
            if values[k] - values[k - 1] <= 1.0e-9 * values[k].abs() {
                return false;
            }
        }
        for k in 0..rank {
            for j in 0..k {
                let (done, rest) = vectors.split_at_mut(k * n);
                let column = &done[j * n..(j + 1) * n];
                let v = &mut rest[..n];
                let dot: f64 = column.iter().zip(v.iter()).map(|(a, b)| a * b).sum();
                if dot.abs() > 1.0e-6 {
                    return false;
                }
                for (vi, ci) in v.iter_mut().zip(column) {
                    *vi -= dot * ci;
                }
            }
            let v = &mut vectors[k * n..(k + 1) * n];
            let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
            for x in v.iter_mut() {
                *x /= norm;
            }
        }
        // Inertia: exactly `rank` eigenvalues at or below the largest found,
        // none below the smallest.
        let top = values[rank - 1];
        let above = top + 1.0e-7 * (values[rank - 1] - values[rank - 2]).abs().max(tiny);
        if self.factor(above, tiny) != rank {
            return false;
        }
        let bottom = values[0] - 1.0e-7 * (values[1] - values[0]).abs().max(tiny);
        if self.factor(bottom, tiny) != 0 {
            return false;
        }
        true
    }
}
