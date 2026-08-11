//! Shared scaffolding for the acceptance-gate tests: graph families, a
//! deterministic Haar `SO(n)` sampler, and dense reference helpers.
//!
//! The sampler is deliberately independent of anything in `mus-gesture`: it
//! builds targets from `np-rand` gaussians and its own Gram-Schmidt
//! orthonormalization, so a compiler bug cannot hide behind a target that was
//! constructed by the same code.

#![allow(dead_code)]

use mus_gesture::{ControlGraph, EdgeControl, EdgeId, Matrix, ModeId, RotationTarget, SplitMix64};
use np_rand::Pcg64;

/// The five graph families the Phase 1 gate names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Path,
    Star,
    BalancedTree,
    Complete,
    RandomConnected,
}

impl Family {
    pub const ALL: [Family; 5] = [
        Family::Path,
        Family::Star,
        Family::BalancedTree,
        Family::Complete,
        Family::RandomConnected,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Family::Path => "path",
            Family::Star => "star",
            Family::BalancedTree => "balanced_tree",
            Family::Complete => "complete",
            Family::RandomConnected => "random_connected",
        }
    }

    /// Canonical coordinate pairs for this family at dimension `n`.
    pub fn pairs(self, n: usize, rng: &mut SplitMix64) -> Vec<(usize, usize)> {
        match self {
            Family::Path => (0..n.saturating_sub(1)).map(|i| (i, i + 1)).collect(),
            Family::Star => (1..n).map(|i| (0, i)).collect(),
            Family::BalancedTree => (1..n).map(|i| ((i - 1) / 2, i)).collect(),
            Family::Complete => (0..n)
                .flat_map(|i| ((i + 1)..n).map(move |j| (i, j)))
                .collect(),
            Family::RandomConnected => {
                // Random recursive tree, then sparse extra chords: the same
                // construction the reference experiments use.
                let mut pairs: Vec<(usize, usize)> = Vec::new();
                for node in 1..n {
                    pairs.push((rng.below(node), node));
                }
                for i in 0..n {
                    for j in (i + 1)..n {
                        if pairs.contains(&(i, j)) || pairs.contains(&(j, i)) {
                            continue;
                        }
                        if rng.below(100) < 18 {
                            pairs.push((i, j));
                        }
                    }
                }
                pairs
            }
        }
    }
}

/// A graph of the given family with unit friction.
pub fn family_graph(family: Family, n: usize, rng: &mut SplitMix64) -> ControlGraph {
    ControlGraph::from_coordinate_edges(n, &family.pairs(n, rng)).unwrap()
}

/// A graph of the given family with heterogeneous friction drawn from `rng`.
pub fn weighted_family_graph(family: Family, n: usize, rng: &mut SplitMix64) -> ControlGraph {
    let pairs = family.pairs(n, rng);
    let modes: Vec<ModeId> = (0..n as u32).map(ModeId).collect();
    let edges: Vec<EdgeControl> = pairs
        .iter()
        .enumerate()
        .map(|(index, &(left, right))| {
            let draw = uniform01(rng);
            EdgeControl::with_friction(
                EdgeId(index as u32),
                ModeId(left as u32),
                ModeId(right as u32),
                (-1.5 + 3.0 * draw).exp(),
            )
        })
        .collect();
    ControlGraph::new(modes, edges).unwrap()
}

pub fn uniform01(rng: &mut SplitMix64) -> f64 {
    (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64
}

/// A dense plane rotation, written out independently of the compiler.
pub fn dense_givens(n: usize, left: usize, right: usize, theta: f64) -> Matrix {
    let mut out = Matrix::identity(n);
    let (sin, cos) = theta.sin_cos();
    out.set(left, left, cos);
    out.set(left, right, -sin);
    out.set(right, left, sin);
    out.set(right, right, cos);
    out
}

/// Haar-distributed `SO(n)`: orthonormalize a gaussian matrix by modified
/// Gram-Schmidt with one reorthogonalization pass, then flip the last column
/// if the determinant came out negative.
pub fn haar_special_orthogonal(n: usize, rng: &mut Pcg64) -> Matrix {
    let mut columns: Vec<Vec<f64>> = (0..n).map(|_| rng.standard_normal_vec(n)).collect();
    let mut basis: Vec<Vec<f64>> = Vec::with_capacity(n);
    for column in &mut columns {
        let mut vector = column.clone();
        for _ in 0..2 {
            for unit in &basis {
                let projection: f64 = unit.iter().zip(vector.iter()).map(|(u, v)| u * v).sum();
                for (target, source) in vector.iter_mut().zip(unit.iter()) {
                    *target -= projection * source;
                }
            }
        }
        let norm = vector.iter().map(|value| value * value).sum::<f64>().sqrt();
        assert!(norm > 1.0e-8, "degenerate gaussian matrix");
        for value in &mut vector {
            *value /= norm;
        }
        basis.push(vector);
    }
    let mut matrix = Matrix::zeros(n, n);
    for (column, unit) in basis.iter().enumerate() {
        for (row, value) in unit.iter().enumerate() {
            matrix.set(row, column, *value);
        }
    }
    if matrix.determinant() < 0.0 {
        for row in 0..n {
            matrix.set(row, n - 1, -matrix.get(row, n - 1));
        }
    }
    matrix
}

/// A validated Haar `SO(n)` target.
pub fn haar_target(n: usize, rng: &mut Pcg64) -> RotationTarget {
    RotationTarget::new(haar_special_orthogonal(n, rng), 1.0e-9).unwrap()
}

/// A Haar `O(n)` matrix with determinant `-1` — a reflection, which no
/// rotation word can reach.
pub fn haar_reflection(n: usize, rng: &mut Pcg64) -> Matrix {
    let mut matrix = haar_special_orthogonal(n, rng);
    for row in 0..n {
        matrix.set(row, 0, -matrix.get(row, 0));
    }
    matrix
}
