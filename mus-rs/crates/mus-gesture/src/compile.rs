//! The exact tree-QR compiler: an `SO(n)` target to a word of authored-edge
//! Givens gates.
//!
//! # Composition convention (correction 2)
//!
//! The Python package mixes conventions: `schedule_matrix` left-multiplies in
//! list order while `_loop_product` right-multiplies. This crate has exactly
//! one convention, and it is [`mus_watsn::OperatorProgram`]'s:
//!
//! > `OperatorProgram::apply` walks its factors **front to back**, updating a
//! > column state in place. For factors `[F0, F1, ..., Fm]` the realized
//! > matrix is therefore `Fm * ... * F1 * F0` — the **first** listed gate is
//! > the one applied **first**, i.e. the rightmost matrix factor.
//!
//! A single [`GivensGate`] with coordinates `(left, right)` and angle `theta`
//! denotes the matrix that is the identity except for
//!
//! ```text
//! G[left ][left ] =  cos(theta)   G[left ][right] = -sin(theta)
//! G[right][left ] =  sin(theta)   G[right][right] =  cos(theta)
//! ```
//!
//! which is exactly [`mus_watsn::givens`] acting on the pair
//! `(state[left], state[right])`. Orientation is load-bearing: swapping
//! `left` and `right` negates the angle.
//!
//! # Algorithm
//!
//! Faithful to `reference/ariadne_gesture/core.py::compile_so_on_tree`. Peel
//! the spanning tree leaf by leaf. For each peeled mode `root`, rotate the
//! `root`-th column of the working matrix onto `root` using only tree edges,
//! innermost leaf first, each gate chosen as `theta = atan2(-b, a)` so the
//! leaf entry is annihilated exactly. Removing `root` leaves a connected
//! subtree, so the next column can be collapsed the same way. The elimination
//! word is the inverse of the synthesis word: reverse the order and negate
//! every angle. A full target consumes `(n-1) + (n-2) + ... + 1 = n(n-1)/2`
//! gates.

use mus_watsn::{OperatorProgram, PlaneRotation};

use crate::error::GestureError;
use crate::graph::{
    canonical_pair, ComponentReport, ControlGraph, EdgeId, LeafOrder, SpanningTree,
};
use crate::matrix::Matrix;
use crate::target::RotationTarget;

/// One compiled plane rotation, resolved to an authored edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GivensGate {
    /// The authored control this gate runs on. Because the compiler only ever
    /// emits gates on validated spanning-tree edges, and a spanning tree is
    /// hard-rejected unless every edge is authored (correction 1), this field
    /// is always a real graph edge — no "all gates on graph" report bit is
    /// needed or offered.
    pub edge: EdgeId,
    /// Coordinate index playing the `i` role in `G(i, j, theta)`.
    pub left: usize,
    /// Coordinate index playing the `j` role in `G(i, j, theta)`.
    pub right: usize,
    pub theta: f64,
}

impl GivensGate {
    /// `G(left, right, -theta)`, the inverse rotation on the same plane.
    pub fn inverse(self) -> Self {
        Self {
            theta: -self.theta,
            ..self
        }
    }

    /// The canonical `(min, max)` coordinate pair this gate acts on.
    pub fn canonical_pair(self) -> (usize, usize) {
        canonical_pair(self.left, self.right)
    }

    /// The angle re-expressed in the canonical `(min, max)` orientation.
    /// Angles only add across gates once they share an orientation.
    pub fn oriented_theta(self) -> f64 {
        if self.left <= self.right {
            self.theta
        } else {
            -self.theta
        }
    }

    /// Lower to the `mus-watsn` factor representation.
    pub fn to_plane_rotation(self) -> PlaneRotation {
        PlaneRotation {
            left: self.left,
            right: self.right,
            angle: self.theta,
        }
    }
}

/// A certified exact factorization of an `SO(n)` target.
#[derive(Debug, Clone, PartialEq)]
pub struct CompileCertificate {
    dimension: usize,
    graph_edges: Vec<(usize, usize)>,
    tree: SpanningTree,
    leaf_order: LeafOrder,
    gates: Vec<GivensGate>,
    program: OperatorProgram,
    endpoint: Matrix,
    reconstruction_error_fro: f64,
    orthogonality_error_fro: f64,
    determinant_error: f64,
}

impl CompileCertificate {
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Canonical sorted coordinate pairs of the authored graph.
    pub fn graph_edges(&self) -> &[(usize, usize)] {
        &self.graph_edges
    }

    pub fn tree(&self) -> &SpanningTree {
        &self.tree
    }

    pub fn leaf_order(&self) -> &LeafOrder {
        &self.leaf_order
    }

    /// The synthesis word, in execution order.
    pub fn gates(&self) -> &[GivensGate] {
        &self.gates
    }

    pub fn gate_count(&self) -> usize {
        self.gates.len()
    }

    /// `dim SO(n) = n(n-1)/2`: the number of gates a generic target needs.
    /// The compiler always emits exactly this many for a full target; special
    /// targets simply carry zero angles rather than shorter words.
    pub fn dimension_lower_bound(&self) -> usize {
        self.dimension * (self.dimension - 1) / 2
    }

    /// The lowered `mus-watsn` program. This is the executable artifact; this
    /// crate never executes a factor word itself.
    pub fn program(&self) -> &OperatorProgram {
        &self.program
    }

    /// The endpoint operator realized by [`Self::program`], reconstructed by
    /// running the program on each basis vector.
    pub fn endpoint(&self) -> &Matrix {
        &self.endpoint
    }

    pub fn reconstruction_error_fro(&self) -> f64 {
        self.reconstruction_error_fro
    }

    pub fn orthogonality_error_fro(&self) -> f64 {
        self.orthogonality_error_fro
    }

    pub fn determinant_error(&self) -> f64 {
        self.determinant_error
    }

    /// Signed integrated angle per authored edge, in canonical orientation.
    /// A closed control path has every entry at zero; correction 3 to the
    /// reference means callers assert `<= 1e-12` on these, never `== 0.0`.
    pub fn edge_closure(&self) -> Vec<(EdgeId, f64)> {
        edge_closure(&self.gates)
    }
}

/// Signed integrated angle per authored edge for an arbitrary word.
pub fn edge_closure(gates: &[GivensGate]) -> Vec<(EdgeId, f64)> {
    let mut out: Vec<(EdgeId, f64)> = Vec::new();
    for gate in gates {
        match out.iter_mut().find(|(id, _)| *id == gate.edge) {
            Some((_, total)) => *total += gate.oriented_theta(),
            None => out.push((gate.edge, gate.oriented_theta())),
        }
    }
    out.sort_unstable_by_key(|(id, _)| *id);
    out
}

/// Reconstruct the dense matrix a program realizes, by executing it on each
/// basis vector. The executor is `mus-watsn`'s; this is only bookkeeping.
pub fn endpoint_operator(program: &OperatorProgram) -> Result<Matrix, GestureError> {
    let n = program.dimension();
    let mut out = Matrix::zeros(n, n);
    let mut state = vec![0.0; n];
    for column in 0..n {
        state.iter_mut().for_each(|value| *value = 0.0);
        state[column] = 1.0;
        program.apply(&mut state)?;
        for (row, value) in state.iter().enumerate() {
            out.set(row, column, *value);
        }
    }
    Ok(out)
}

/// Compile with the deterministic baseline plan: breadth-first spanning tree
/// from mode 0, minimum-index leaf peel.
pub fn compile(
    target: &RotationTarget,
    graph: &ControlGraph,
) -> Result<CompileCertificate, GestureError> {
    let tree = graph.bfs_spanning_tree(0)?;
    let order = tree.deterministic_leaf_order()?;
    compile_on_tree(target, graph, &tree, &order)
}

/// Compile against a caller-supplied spanning tree and leaf order.
pub fn compile_on_tree(
    target: &RotationTarget,
    graph: &ControlGraph,
    tree: &SpanningTree,
    leaf_order: &LeafOrder,
) -> Result<CompileCertificate, GestureError> {
    let n = graph.dimension();
    target.require_dimension(n)?;
    let report = graph.component_report();
    if !report.is_connected() {
        return Err(GestureError::DisconnectedGraph {
            components: report.components.len(),
        });
    }
    if tree.dimension() != n {
        return Err(GestureError::TreeSizeMismatch {
            expected: n.saturating_sub(1),
            actual: tree.edges().len(),
        });
    }
    // Re-check the tree against *this* graph: a tree validated elsewhere must
    // not smuggle an off-graph edge in (correction 1).
    for &(left, right) in tree.edges() {
        if graph.edge_between(left, right).is_none() {
            return Err(GestureError::TreeEdgeOffGraph { left, right });
        }
    }
    // Re-validate the peel against this tree.
    LeafOrder::new(tree, leaf_order.as_slice().to_vec())?;

    let mut reduced = target.matrix().clone();
    let mut alive = vec![true; n];
    let expected_gates = n * (n - 1) / 2;
    let mut elimination: Vec<GivensGate> = Vec::with_capacity(expected_gates);

    for &root in leaf_order.as_slice().iter().take(n.saturating_sub(1)) {
        let mut active = alive.clone();
        let mut active_count = active.iter().filter(|flag| **flag).count();
        while active_count > 1 {
            let leaf = (0..n)
                .find(|&node| active[node] && node != root && tree.alive_degree(node, &active) == 1)
                .ok_or(GestureError::PeelInvariant { root })?;
            let parent = tree
                .neighbours(leaf)
                .iter()
                .copied()
                .find(|&node| active[node])
                .ok_or(GestureError::PeelInvariant { root })?;
            let a = reduced.get(parent, root);
            let b = reduced.get(leaf, root);
            let theta = (-b).atan2(a);
            reduced.rotate_rows(parent, leaf, theta);
            let edge = graph
                .edge_between(parent, leaf)
                .ok_or(GestureError::TreeEdgeOffGraph {
                    left: parent.min(leaf),
                    right: parent.max(leaf),
                })?;
            elimination.push(GivensGate {
                edge: edge.id,
                left: parent,
                right: leaf,
                theta,
            });
            active[leaf] = false;
            active_count -= 1;
        }
        alive[root] = false;
    }

    // Synthesis is the inverse of elimination: reverse and negate.
    let gates: Vec<GivensGate> = elimination
        .iter()
        .rev()
        .map(|gate| gate.inverse())
        .collect();
    if gates.len() != expected_gates {
        return Err(GestureError::GateCountInvariant {
            expected: expected_gates,
            actual: gates.len(),
        });
    }

    let program = OperatorProgram::new(
        n,
        gates.iter().map(|gate| gate.to_plane_rotation()).collect(),
    )?;
    let endpoint = endpoint_operator(&program)?;

    Ok(CompileCertificate {
        dimension: n,
        graph_edges: graph.sorted_edge_coordinates(),
        tree: tree.clone(),
        leaf_order: leaf_order.clone(),
        gates,
        reconstruction_error_fro: endpoint.frobenius_distance(target.matrix()),
        orthogonality_error_fro: endpoint.orthogonality_residual(),
        determinant_error: (endpoint.determinant() - 1.0).abs(),
        program,
        endpoint,
    })
}

/// A certified factorization of a block-diagonal target on a graph with
/// several components.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockDiagonalCertificate {
    report: ComponentReport,
    blocks: Vec<CompileCertificate>,
    gates: Vec<GivensGate>,
    program: OperatorProgram,
    endpoint: Matrix,
    reconstruction_error_fro: f64,
    orthogonality_error_fro: f64,
    determinant_error: f64,
}

impl BlockDiagonalCertificate {
    pub fn component_report(&self) -> &ComponentReport {
        &self.report
    }

    /// Per-component certificates, in component order. Their coordinates are
    /// local to the induced subgraph.
    pub fn blocks(&self) -> &[CompileCertificate] {
        &self.blocks
    }

    /// The concatenated word, in global coordinates. Gates from different
    /// components act on disjoint coordinates and therefore commute, so the
    /// concatenation order across components is immaterial.
    pub fn gates(&self) -> &[GivensGate] {
        &self.gates
    }

    pub fn program(&self) -> &OperatorProgram {
        &self.program
    }

    pub fn endpoint(&self) -> &Matrix {
        &self.endpoint
    }

    pub fn gate_count(&self) -> usize {
        self.gates.len()
    }

    pub fn reconstruction_error_fro(&self) -> f64 {
        self.reconstruction_error_fro
    }

    pub fn orthogonality_error_fro(&self) -> f64 {
        self.orthogonality_error_fro
    }

    pub fn determinant_error(&self) -> f64 {
        self.determinant_error
    }
}

/// Compile a target on a possibly disconnected graph.
///
/// The handoff asks for cross-component targets to be "rejected or explicitly
/// decomposed". Both happen here, and which one is typed:
///
/// * any entry coupling two distinct components with magnitude above the
///   target tolerance is [`GestureError::CrossComponentTarget`] — no product
///   of edge rotations can create that coupling, so it is
///   [`crate::ErrorClass::Unreachable`];
/// * otherwise the target is block diagonal and each block is compiled on its
///   own component with the exact tree-QR compiler.
///
/// A connected graph is the one-block case, and this agrees gate-for-gate
/// with [`compile`] there.
pub fn compile_block_diagonal(
    target: &RotationTarget,
    graph: &ControlGraph,
) -> Result<BlockDiagonalCertificate, GestureError> {
    let n = graph.dimension();
    target.require_dimension(n)?;
    let report = graph.component_report();

    // Cross-component coupling check.
    for row in 0..n {
        for column in 0..n {
            if report.membership[row] == report.membership[column] {
                continue;
            }
            let magnitude = target.matrix().get(row, column).abs();
            if magnitude > target.tolerance() {
                return Err(GestureError::CrossComponentTarget {
                    row,
                    column,
                    magnitude,
                });
            }
        }
    }

    let mut blocks = Vec::with_capacity(report.components.len());
    let mut gates: Vec<GivensGate> = Vec::new();
    for component in &report.components {
        let subgraph = graph.induced_subgraph(component)?;
        let size = component.len();
        let mut block = Matrix::zeros(size, size);
        for (local_row, &global_row) in component.iter().enumerate() {
            for (local_column, &global_column) in component.iter().enumerate() {
                block.set(
                    local_row,
                    local_column,
                    target.matrix().get(global_row, global_column),
                );
            }
        }
        let block_target = RotationTarget::new(block, target.tolerance())?;
        let certificate = compile(&block_target, &subgraph)?;
        for gate in certificate.gates() {
            gates.push(GivensGate {
                edge: gate.edge,
                left: component[gate.left],
                right: component[gate.right],
                theta: gate.theta,
            });
        }
        blocks.push(certificate);
    }

    let program = OperatorProgram::new(
        n,
        gates.iter().map(|gate| gate.to_plane_rotation()).collect(),
    )?;
    let endpoint = endpoint_operator(&program)?;
    Ok(BlockDiagonalCertificate {
        report,
        blocks,
        gates,
        reconstruction_error_fro: endpoint.frobenius_distance(target.matrix()),
        orthogonality_error_fro: endpoint.orthogonality_residual(),
        determinant_error: (endpoint.determinant() - 1.0).abs(),
        program,
        endpoint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{EdgeControl, ModeId};
    use std::f64::consts::FRAC_PI_2;

    fn path(n: usize) -> ControlGraph {
        let pairs: Vec<(usize, usize)> = (0..n - 1).map(|i| (i, i + 1)).collect();
        ControlGraph::from_coordinate_edges(n, &pairs).unwrap()
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

    /// Correction 2, part one: an exact hand-computed three-gate product.
    ///
    /// With `F0 = G(0,1,pi/2)`, `F1 = G(1,2,pi/2)`, `F2 = G(0,2,pi/2)` the
    /// dense factors are
    ///
    /// ```text
    /// G0 = [[0,-1,0],[1,0,0],[0,0,1]]
    /// G1 = [[1,0,0],[0,0,-1],[0,1,0]]
    /// G2 = [[0,0,-1],[0,1,0],[1,0,0]]
    /// ```
    ///
    /// and, by hand, `G1 G0 = [[0,-1,0],[0,0,-1],[1,0,0]]`, so
    /// `G2 G1 G0 = [[-1,0,0],[0,0,-1],[0,-1,0]]`. The `OperatorProgram`
    /// convention says the *authored* order `[F0, F1, F2]` realizes exactly
    /// that product, and the reverse authored order does not.
    #[test]
    fn three_gate_word_matches_the_hand_computed_product() {
        let gates = [
            GivensGate {
                edge: EdgeId(0),
                left: 0,
                right: 1,
                theta: FRAC_PI_2,
            },
            GivensGate {
                edge: EdgeId(1),
                left: 1,
                right: 2,
                theta: FRAC_PI_2,
            },
            GivensGate {
                edge: EdgeId(2),
                left: 0,
                right: 2,
                theta: FRAC_PI_2,
            },
        ];
        let program = OperatorProgram::new(
            3,
            gates.iter().map(|gate| gate.to_plane_rotation()).collect(),
        )
        .unwrap();
        let realized = endpoint_operator(&program).unwrap();

        let by_hand = Matrix::from_rows(vec![
            vec![-1.0, 0.0, 0.0],
            vec![0.0, 0.0, -1.0],
            vec![0.0, -1.0, 0.0],
        ])
        .unwrap();
        assert!(realized.frobenius_distance(&by_hand) <= 1.0e-15);

        // The reversed authored order realizes G0 G1 G2, a different element:
        // the word is genuinely non-commuting, so the convention is pinned,
        // not merely consistent.
        let reversed = OperatorProgram::new(
            3,
            gates
                .iter()
                .rev()
                .map(|gate| gate.to_plane_rotation())
                .collect(),
        )
        .unwrap();
        let reversed_matrix = endpoint_operator(&reversed).unwrap();
        assert!(reversed_matrix.frobenius_distance(&by_hand) > 1.0);
    }

    /// Correction 2, part two: the same claim at generic angles, checked
    /// against dense left-multiplication rather than against special values.
    #[test]
    fn word_order_is_last_factor_leftmost_at_generic_angles() {
        let angles = [0.31_f64, -0.47, 1.13];
        let planes = [(0usize, 1usize), (1, 2), (0, 2)];
        let gates: Vec<GivensGate> = planes
            .iter()
            .zip(angles.iter())
            .enumerate()
            .map(|(index, (&(left, right), &theta))| GivensGate {
                edge: EdgeId(index as u32),
                left,
                right,
                theta,
            })
            .collect();
        let program = OperatorProgram::new(
            3,
            gates.iter().map(|gate| gate.to_plane_rotation()).collect(),
        )
        .unwrap();
        let realized = endpoint_operator(&program).unwrap();

        // Fm * ... * F0 by explicit dense multiplication.
        let mut expected = Matrix::identity(3);
        for gate in &gates {
            expected = dense_givens(3, gate.left, gate.right, gate.theta).multiply(&expected);
        }
        assert!(realized.frobenius_distance(&expected) <= 1.0e-14);

        // The opposite convention differs materially.
        let mut wrong = Matrix::identity(3);
        for gate in &gates {
            wrong = wrong.multiply(&dense_givens(3, gate.left, gate.right, gate.theta));
        }
        assert!(realized.frobenius_distance(&wrong) > 1.0e-3);
    }

    #[test]
    fn orientation_negates_the_angle() {
        let forward = dense_givens(3, 0, 2, 0.4);
        let reversed = dense_givens(3, 2, 0, -0.4);
        assert!(forward.frobenius_distance(&reversed) <= 1.0e-15);
        let gate = GivensGate {
            edge: EdgeId(0),
            left: 2,
            right: 0,
            theta: 0.4,
        };
        assert!((gate.oriented_theta() - -0.4).abs() <= 1.0e-18);
        assert_eq!(gate.canonical_pair(), (0, 2));
    }

    #[test]
    fn identity_target_compiles_to_a_zero_word() {
        let graph = path(6);
        let target = RotationTarget::identity(6).unwrap();
        let certificate = compile(&target, &graph).unwrap();
        assert_eq!(certificate.gate_count(), 15);
        for gate in certificate.gates() {
            assert!(gate.theta.abs() <= 0.0, "angle {} is not zero", gate.theta);
        }
        assert!(certificate.reconstruction_error_fro() <= 0.0);
        for (_, closure) in certificate.edge_closure() {
            // Correction 3: assert the bound, never equality with zero.
            assert!(closure.abs() <= 1.0e-12);
        }
    }

    #[test]
    fn a_single_plane_rotation_on_a_path_is_exact() {
        let graph = path(4);
        let target = RotationTarget::new(dense_givens(4, 1, 2, 0.83), 1.0e-9).unwrap();
        let certificate = compile(&target, &graph).unwrap();
        assert_eq!(certificate.gate_count(), 6);
        assert_eq!(certificate.dimension_lower_bound(), 6);
        assert!(certificate.reconstruction_error_fro() <= 1.0e-14);
        for gate in certificate.gates() {
            assert!(graph.edge_between(gate.left, gate.right).is_some());
        }
    }

    #[test]
    fn compiling_on_a_disconnected_graph_is_typed() {
        let graph = ControlGraph::from_coordinate_edges(5, &[(0, 1), (1, 2), (3, 4)]).unwrap();
        let target = RotationTarget::identity(5).unwrap();
        assert_eq!(
            compile(&target, &graph).unwrap_err(),
            GestureError::DisconnectedGraph { components: 2 }
        );
    }

    #[test]
    fn cross_component_targets_are_rejected_and_block_targets_decompose() {
        let graph = ControlGraph::from_coordinate_edges(5, &[(0, 1), (1, 2), (3, 4)]).unwrap();

        let coupling = RotationTarget::new(dense_givens(5, 2, 3, 0.6), 1.0e-9).unwrap();
        match compile_block_diagonal(&coupling, &graph).unwrap_err() {
            GestureError::CrossComponentTarget { magnitude, .. } => {
                assert!(magnitude > 0.5);
            }
            other => panic!("expected a cross-component rejection, got {other}"),
        }

        let block = dense_givens(5, 0, 2, 0.4).multiply(&dense_givens(5, 3, 4, -1.1));
        let block_target = RotationTarget::new(block, 1.0e-9).unwrap();
        let certificate = compile_block_diagonal(&block_target, &graph).unwrap();
        assert_eq!(certificate.blocks().len(), 2);
        // 3 gates for the 3-mode component, 1 for the 2-mode component.
        assert_eq!(certificate.gate_count(), 4);
        assert!(certificate.reconstruction_error_fro() <= 1.0e-14);
        for gate in certificate.gates() {
            assert!(graph.edge_between(gate.left, gate.right).is_some());
        }
    }

    #[test]
    fn block_diagonal_agrees_with_compile_on_a_connected_graph() {
        let graph = path(5);
        let target = RotationTarget::new(
            dense_givens(5, 0, 1, 0.3).multiply(&dense_givens(5, 2, 4, -0.9)),
            1.0e-9,
        )
        .unwrap();
        let direct = compile(&target, &graph).unwrap();
        let blocked = compile_block_diagonal(&target, &graph).unwrap();
        assert_eq!(direct.gates(), blocked.gates());
    }

    #[test]
    fn non_authored_modes_and_mismatched_dimensions_are_typed() {
        let graph = path(4);
        let target = RotationTarget::identity(3).unwrap();
        assert!(matches!(
            compile(&target, &graph).unwrap_err(),
            GestureError::TargetShapeMismatch { .. }
        ));
    }

    #[test]
    fn a_tree_from_another_graph_cannot_be_smuggled_in() {
        let complete: Vec<(usize, usize)> = (0..4)
            .flat_map(|i| ((i + 1)..4).map(move |j| (i, j)))
            .collect();
        let dense = ControlGraph::from_coordinate_edges(4, &complete).unwrap();
        let star_tree = dense.bfs_spanning_tree(0).unwrap();
        let order = star_tree.deterministic_leaf_order().unwrap();
        let sparse = path(4);
        let target = RotationTarget::identity(4).unwrap();
        assert_eq!(
            compile_on_tree(&target, &sparse, &star_tree, &order).unwrap_err(),
            GestureError::TreeEdgeOffGraph { left: 0, right: 2 }
        );
    }

    #[test]
    fn friction_and_ids_survive_into_the_gates() {
        let graph = ControlGraph::new(
            vec![ModeId(10), ModeId(20), ModeId(30)],
            vec![
                EdgeControl::with_friction(EdgeId(100), ModeId(10), ModeId(20), 2.0),
                EdgeControl::with_friction(EdgeId(200), ModeId(20), ModeId(30), 0.5),
            ],
        )
        .unwrap();
        let target = RotationTarget::new(dense_givens(3, 0, 2, 0.77), 1.0e-9).unwrap();
        let certificate = compile(&target, &graph).unwrap();
        for gate in certificate.gates() {
            let edge = graph.edge_between(gate.left, gate.right).unwrap();
            assert_eq!(edge.id, gate.edge);
        }
        assert!(certificate.reconstruction_error_fro() <= 1.0e-14);
    }

    #[test]
    fn dimension_one_is_the_empty_word() {
        let graph = ControlGraph::new(vec![ModeId(0)], vec![]).unwrap();
        let target = RotationTarget::identity(1).unwrap();
        let certificate = compile(&target, &graph).unwrap();
        assert_eq!(certificate.gate_count(), 0);
        assert_eq!(certificate.dimension_lower_bound(), 0);
        assert!(certificate.reconstruction_error_fro() <= 0.0);
    }
}
