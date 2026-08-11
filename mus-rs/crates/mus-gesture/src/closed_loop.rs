//! The exact closed-control `SO(3)` compiler: one balanced commutator,
//! twelve authored-edge gates, every edge control integrating back to zero.
//!
//! # What is compiled
//!
//! `EXACT-CLOSED-SO3.md` records the constructive instance of Gotô's theorem
//! this module implements. With
//!
//! ```text
//! A0(t) = R01(t)      B0(t) = R12(t)      C0(t) = A0 B0 A0^-1 B0^-1
//! ```
//!
//! the principal angle `phi` of `C0(t)` satisfies `sin(phi/2)^2 = 4x^2(1-x^2)`
//! for `x = sin(t/2)^2`, whose principal-branch inverse is
//! [`balanced_commutator_angle`]. One balanced local commutator therefore
//! realizes *every* principal angle in `SO(3)`. Conjugating by any `S` with
//! `S n0 = n` moves its axis onto the target's, and because
//!
//! ```text
//! (S A0 S^-1)(S B0 S^-1)(S A0 S^-1)^-1(S B0 S^-1)^-1 = S C0 S^-1 = Q,
//! ```
//!
//! each macro appears once with its inverse: the visible controls close.
//! Lowering `A = S A0 S^-1` and `B = S B0 S^-1` through the Phase 1 tree-QR
//! compiler gives `3 + 3` gates per macro pair and
//! [`CLOSED_LOOP_GATE_COUNT`]` = 12` in total.
//!
//! # Word order is derived here, not copied
//!
//! The reference's execution order is stated for *its* schedule convention.
//! This crate has one pinned convention (see [`crate::compile`]): an
//! [`mus_watsn::OperatorProgram`] applies factors front to back, so the word
//! `[W, V]` realizes `M_V * M_W` — the first-listed block is the **rightmost**
//! matrix factor. Reading `A B A^-1 B^-1` right to left as an application
//! order gives
//!
//! ```text
//! word = inverse(W_B) ++ inverse(W_A) ++ W_B ++ W_A
//! ```
//!
//! where `inverse(W)` is `W` reversed with every angle negated — itself an
//! exact word for the inverse macro, on the same authored edges. This is
//! asserted directly against a dense product in
//! `tests::the_word_realizes_a_b_ainv_binv_in_the_pinned_convention`, and the
//! three other orderings are shown to realize something else.
//!
//! # Closure
//!
//! Every gate appears exactly twice, once with each sign, so the integrated
//! angle on each authored edge is zero analytically. Correction 3 to the
//! reference carries over verbatim: callers assert
//! [`ClosedLoopCertificate::max_edge_closure`]` <= 1e-12`, **never** `== 0.0`.
//! The float64 sums are usually exactly zero and are not required to be.
//!
//! # Numerics
//!
//! [`principal_angle_axis`] extracts `(phi, n)` from a rotation via the
//! largest-pivot quaternion branch — the route
//! `scipy.spatial.transform.Rotation.from_matrix(...).as_rotvec()` takes, so
//! the Rust and Python compilers agree on the axis they are conjugating onto.
//! [`ClosedLoopCertificate::conjugator_branch`] records which of the three
//! deterministic branches of the conjugator construction was taken; the
//! antipodal branch (`S n0 = -n0`) picks the minimum-magnitude coordinate axis
//! exactly as the reference does, so two implementations agree on the *choice*
//! and not merely on the endpoint.

use serde::{Deserialize, Serialize};

use mus_watsn::OperatorProgram;

use crate::compile::{compile_on_tree, endpoint_operator, CompileCertificate, GivensGate};
use crate::digest::{friction_metric_digest, graph_digest, matrix_digest, spanning_tree_digest};
use crate::error::GestureError;
use crate::graph::{ControlGraph, EdgeId, LeafOrder, SpanningTree};
use crate::matrix::Matrix;
use crate::receipt::{ReceiptGate, WorkResidual, COMPILER_VERSION};
use crate::schedule::{thermodynamic_schedule, FrictionMetric, ThermodynamicSchedule};
use crate::target::RotationTarget;
use crate::work::WorkMeasurement;

/// The dimension this compiler is defined at. `SO(n)` for `n > 3` needs a
/// constructive choice of macro factors that this phase does not make.
pub const CLOSED_LOOP_DIMENSION: usize = 3;

/// Authored-edge gates in a nonidentity closed word: four macros of
/// `n(n-1)/2 = 3` gates each.
pub const CLOSED_LOOP_GATE_COUNT: usize = 12;

/// Principal angles below this compile to the empty word. Matches the
/// reference's `1e-14` identity cutoff.
pub const IDENTITY_ANGLE_TOLERANCE: f64 = 1.0e-14;

/// `|n0 x n|` below this takes the parallel/antipodal branch of the
/// conjugator. Matches the reference's `1e-12`.
pub const ANTIPODAL_SINE_THRESHOLD: f64 = 1.0e-12;

/// Which deterministic branch built the conjugator `S` with `S n0 = n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConjugatorBranch {
    /// `n0` and `n` already agree: `S = I`.
    Aligned,
    /// `n = -n0`: a half turn about the deterministic perpendicular built from
    /// the minimum-magnitude coordinate of `n0`.
    Antipodal,
    /// The generic Rodrigues rotation about `n0 x n`.
    General,
}

impl ConjugatorBranch {
    /// Stable lowercase name, as it appears in a receipt.
    pub fn name(self) -> &'static str {
        match self {
            Self::Aligned => "aligned",
            Self::Antipodal => "antipodal",
            Self::General => "general",
        }
    }
}

/// The exact balanced local angle whose commutator has principal angle
/// `target_angle`: `t = 2 asin(sqrt(sin(phi/4)))`.
///
/// The domain is `[0, pi]` — the principal angles of `SO(3)`. Anything else,
/// including a non-finite input, is
/// [`GestureError::PrincipalAngleOutOfDomain`]. Unlike the reference this
/// function does not accept `pi + 1e-12` and clamp: the compiler never needs
/// it to, because [`principal_angle_axis`] derives the angle from a
/// canonicalized quaternion and therefore returns `2 atan2(|v|, w) <= pi` by
/// construction.
pub fn balanced_commutator_angle(target_angle: f64) -> Result<f64, GestureError> {
    if !target_angle.is_finite() || !(0.0..=std::f64::consts::PI).contains(&target_angle) {
        return Err(GestureError::PrincipalAngleOutOfDomain {
            angle: target_angle,
        });
    }
    Ok(2.0 * (target_angle / 4.0).sin().sqrt().asin())
}

/// Principal rotation angle in `[0, pi]` and unit rotation axis of a `3x3`
/// rotation matrix.
///
/// Matrix to quaternion by the largest-pivot (Shepperd) branch, then
/// `angle = 2 atan2(|v|, w)` on the canonical `w >= 0` representative. This is
/// the same route the reference takes through `scipy`, and it is chosen over
/// `acos((tr - 1)/2)` because the latter loses half its digits near `0` and
/// `pi`, exactly where the near-identity and near-pi corpora live.
///
/// The identity returns `(0.0, [0, 0, 0])`: an identity rotation has no axis,
/// and inventing one would hide the degeneracy from the caller.
pub fn principal_angle_axis(rotation: &Matrix) -> Result<(f64, [f64; 3]), GestureError> {
    if rotation.rows() != CLOSED_LOOP_DIMENSION || rotation.columns() != CLOSED_LOOP_DIMENSION {
        return Err(GestureError::ClosedLoopDimension {
            dimension: rotation.rows().max(rotation.columns()),
        });
    }
    let entry = |row: usize, column: usize| rotation.get(row, column);
    let trace = entry(0, 0) + entry(1, 1) + entry(2, 2);
    let decision = [entry(0, 0), entry(1, 1), entry(2, 2), trace];
    let mut choice = 0usize;
    for candidate in 1..4 {
        if decision[candidate] > decision[choice] {
            choice = candidate;
        }
    }

    // Quaternion as (x, y, z, w), unnormalized.
    let mut quaternion = [0.0_f64; 4];
    if choice != 3 {
        let i = choice;
        let j = (i + 1) % 3;
        let k = (j + 1) % 3;
        quaternion[i] = 1.0 - trace + 2.0 * entry(i, i);
        quaternion[j] = entry(j, i) + entry(i, j);
        quaternion[k] = entry(k, i) + entry(i, k);
        quaternion[3] = entry(k, j) - entry(j, k);
    } else {
        quaternion[0] = entry(2, 1) - entry(1, 2);
        quaternion[1] = entry(0, 2) - entry(2, 0);
        quaternion[2] = entry(1, 0) - entry(0, 1);
        quaternion[3] = 1.0 + trace;
    }

    let norm = quaternion
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm <= 0.0 {
        // Unreachable for a real rotation: the largest-pivot branch always
        // yields a quaternion of norm 2. A zero norm means the caller handed
        // in something that is not a rotation at all, and saying so is the
        // honest outcome.
        return Err(GestureError::TargetNotOrthogonal {
            residual: rotation.orthogonality_residual(),
            tolerance: 0.0,
        });
    }
    for value in &mut quaternion {
        *value /= norm;
    }
    if quaternion[3] < 0.0 {
        for value in &mut quaternion {
            *value = -*value;
        }
    }

    let vector_norm = (quaternion[0] * quaternion[0]
        + quaternion[1] * quaternion[1]
        + quaternion[2] * quaternion[2])
        .sqrt();
    let angle = 2.0 * vector_norm.atan2(quaternion[3]);
    let axis = if vector_norm > 0.0 {
        [
            quaternion[0] / vector_norm,
            quaternion[1] / vector_norm,
            quaternion[2] / vector_norm,
        ]
    } else {
        [0.0, 0.0, 0.0]
    };
    Ok((angle, axis))
}

/// A certified exact closed-control factorization of an `SO(3)` target.
#[derive(Debug, Clone, PartialEq)]
pub struct ClosedLoopCertificate {
    graph_edges: Vec<(usize, usize)>,
    tree: SpanningTree,
    leaf_order: LeafOrder,
    target_angle_rad: f64,
    target_axis: [f64; 3],
    balanced_generator_angle_rad: f64,
    local_commutator_axis: [f64; 3],
    conjugator: Matrix,
    conjugator_branch: ConjugatorBranch,
    macro_a: Matrix,
    macro_b: Matrix,
    dense_commutator: Matrix,
    macro_certificates: Option<Box<(CompileCertificate, CompileCertificate)>>,
    gates: Vec<GivensGate>,
    program: OperatorProgram,
    endpoint: Matrix,
    schedule: ThermodynamicSchedule,
    reconstruction_error_fro: f64,
    reconstruction_error_rad: f64,
    orthogonality_error_fro: f64,
    determinant_error: f64,
}

impl ClosedLoopCertificate {
    /// Always [`CLOSED_LOOP_DIMENSION`].
    pub fn dimension(&self) -> usize {
        CLOSED_LOOP_DIMENSION
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

    /// Principal angle of the target, in `[0, pi]`.
    pub fn target_angle_rad(&self) -> f64 {
        self.target_angle_rad
    }

    /// Unit rotation axis of the target; `[0, 0, 0]` for the identity.
    pub fn target_axis(&self) -> [f64; 3] {
        self.target_axis
    }

    /// `t` in `A0 = R01(t)`, `B0 = R12(t)`; zero for the identity.
    pub fn balanced_generator_angle_rad(&self) -> f64 {
        self.balanced_generator_angle_rad
    }

    /// Unit axis of the *local* commutator `C0(t)` before conjugation.
    pub fn local_commutator_axis(&self) -> [f64; 3] {
        self.local_commutator_axis
    }

    /// The conjugator `S` with `S n0 = n`.
    pub fn conjugator(&self) -> &Matrix {
        &self.conjugator
    }

    /// Which deterministic branch built [`Self::conjugator`].
    pub fn conjugator_branch(&self) -> ConjugatorBranch {
        self.conjugator_branch
    }

    /// The dense macro `A = S A0 S^-1`.
    pub fn macro_a(&self) -> &Matrix {
        &self.macro_a
    }

    /// The dense macro `B = S B0 S^-1`.
    pub fn macro_b(&self) -> &Matrix {
        &self.macro_b
    }

    /// `A B A^-1 B^-1`, formed by dense multiplication of the macros and
    /// entirely independent of the lowered word. Comparing it against
    /// [`Self::endpoint`] is the "direct commutator and lowered word agree"
    /// gate.
    pub fn dense_commutator(&self) -> &Matrix {
        &self.dense_commutator
    }

    /// The Phase 1 tree-QR certificates of the two macros, in `(A, B)` order.
    /// `None` for the identity target, which compiles no macros at all.
    pub fn macro_certificates(&self) -> Option<(&CompileCertificate, &CompileCertificate)> {
        self.macro_certificates
            .as_ref()
            .map(|pair| (&pair.0, &pair.1))
    }

    /// The closed word, in execution order:
    /// `inverse(W_B) ++ inverse(W_A) ++ W_B ++ W_A`.
    pub fn gates(&self) -> &[GivensGate] {
        &self.gates
    }

    /// `0` for the identity target, [`CLOSED_LOOP_GATE_COUNT`] otherwise.
    pub fn gate_count(&self) -> usize {
        self.gates.len()
    }

    /// The lowered `mus-watsn` program: the executable artifact.
    pub fn program(&self) -> &OperatorProgram {
        &self.program
    }

    /// The endpoint operator realized by [`Self::program`].
    pub fn endpoint(&self) -> &Matrix {
        &self.endpoint
    }

    /// Constant-thermodynamic-speed timing of the closed word.
    pub fn schedule(&self) -> &ThermodynamicSchedule {
        &self.schedule
    }

    pub fn duration(&self) -> f64 {
        self.schedule.duration()
    }

    pub fn reconstruction_error_fro(&self) -> f64 {
        self.reconstruction_error_fro
    }

    /// Principal angle of `target^T * endpoint` — the endpoint error measured
    /// on the group rather than in the ambient matrix space.
    pub fn reconstruction_error_rad(&self) -> f64 {
        self.reconstruction_error_rad
    }

    pub fn orthogonality_error_fro(&self) -> f64 {
        self.orthogonality_error_fro
    }

    pub fn determinant_error(&self) -> f64 {
        self.determinant_error
    }

    /// Signed integrated angle per authored edge, canonically oriented.
    pub fn edge_closure(&self) -> Vec<(EdgeId, f64)> {
        crate::compile::edge_closure(&self.gates)
    }

    /// Worst `|closure|` over authored edges. Correction 3: assert this
    /// `<= 1e-12`, never `== 0.0`.
    pub fn max_edge_closure(&self) -> f64 {
        self.edge_closure()
            .into_iter()
            .map(|(_, closure)| closure.abs())
            .fold(0.0_f64, f64::max)
    }
}

/// Compile every `SO(3)` target as one exact closed macro commutator on a
/// connected three-mode authored graph, timed over `duration`.
///
/// * The identity target (principal angle below [`IDENTITY_ANGLE_TOLERANCE`])
///   compiles to the **empty word** — no gates, no executable segments.
/// * Every other target compiles to exactly [`CLOSED_LOOP_GATE_COUNT`] gates,
///   all on authored edges, with every edge control integrating back to zero.
///
/// Errors are the Phase 1 typed outcomes plus
/// [`GestureError::ClosedLoopDimension`] for a graph or target that is not
/// three-dimensional.
pub fn compile_exact_closed_loop_so3(
    target: &RotationTarget,
    graph: &ControlGraph,
    duration: f64,
) -> Result<ClosedLoopCertificate, GestureError> {
    if graph.dimension() != CLOSED_LOOP_DIMENSION {
        return Err(GestureError::ClosedLoopDimension {
            dimension: graph.dimension(),
        });
    }
    if target.dimension() != CLOSED_LOOP_DIMENSION {
        return Err(GestureError::ClosedLoopDimension {
            dimension: target.dimension(),
        });
    }
    if !duration.is_finite() || duration <= 0.0 {
        return Err(GestureError::InvalidDuration { duration });
    }
    let report = graph.component_report();
    if !report.is_connected() {
        return Err(GestureError::DisconnectedGraph {
            components: report.components.len(),
        });
    }

    let tree = graph.bfs_spanning_tree(0)?;
    let leaf_order = tree.deterministic_leaf_order()?;
    let (target_angle, target_axis) = principal_angle_axis(target.matrix())?;

    if target_angle < IDENTITY_ANGLE_TOLERANCE {
        return identity_certificate(target, graph, tree, leaf_order, duration);
    }

    let local_angle = balanced_commutator_angle(target_angle)?;
    let a0 = plane_rotation(0, 1, local_angle);
    let b0 = plane_rotation(1, 2, local_angle);
    let local_commutator = commutator(&a0, &b0);
    let (_, local_axis) = principal_angle_axis(&local_commutator)?;
    let (conjugator, conjugator_branch) = rotation_from_to(local_axis, target_axis);
    let conjugator_transpose = conjugator.transpose();
    let macro_a = conjugator.multiply(&a0).multiply(&conjugator_transpose);
    let macro_b = conjugator.multiply(&b0).multiply(&conjugator_transpose);
    let dense_commutator = commutator(&macro_a, &macro_b);

    let tolerance = target.tolerance();
    let certificate_a = compile_on_tree(
        &RotationTarget::new(macro_a.clone(), tolerance)?,
        graph,
        &tree,
        &leaf_order,
    )?;
    let certificate_b = compile_on_tree(
        &RotationTarget::new(macro_b.clone(), tolerance)?,
        graph,
        &tree,
        &leaf_order,
    )?;

    // The word, derived against this crate's pinned convention: factors apply
    // front to back, so the FIRST block listed is the RIGHTMOST matrix factor.
    // `A B A^-1 B^-1` applied to a state runs B^-1, then A^-1, then B, then A.
    let mut gates: Vec<GivensGate> = inverse_word(certificate_b.gates());
    gates.extend(inverse_word(certificate_a.gates()));
    gates.extend_from_slice(certificate_b.gates());
    gates.extend_from_slice(certificate_a.gates());
    if gates.len() != CLOSED_LOOP_GATE_COUNT {
        return Err(GestureError::GateCountInvariant {
            expected: CLOSED_LOOP_GATE_COUNT,
            actual: gates.len(),
        });
    }

    let program = OperatorProgram::new(
        CLOSED_LOOP_DIMENSION,
        gates.iter().map(|gate| gate.to_plane_rotation()).collect(),
    )?;
    let endpoint = endpoint_operator(&program)?;
    let metric = FrictionMetric::resolve(&gates, graph)?;
    let schedule = thermodynamic_schedule(&gates, &metric, duration)?;
    let (reconstruction_error_rad, _) =
        principal_angle_axis(&target.matrix().transpose().multiply(&endpoint))?;

    Ok(ClosedLoopCertificate {
        graph_edges: graph.sorted_edge_coordinates(),
        tree,
        leaf_order,
        target_angle_rad: target_angle,
        target_axis,
        balanced_generator_angle_rad: local_angle,
        local_commutator_axis: local_axis,
        conjugator,
        conjugator_branch,
        macro_a,
        macro_b,
        dense_commutator,
        macro_certificates: Some(Box::new((certificate_a, certificate_b))),
        gates,
        reconstruction_error_fro: endpoint.frobenius_distance(target.matrix()),
        reconstruction_error_rad,
        orthogonality_error_fro: endpoint.orthogonality_residual(),
        determinant_error: (endpoint.determinant() - 1.0).abs(),
        program,
        endpoint,
        schedule,
    })
}

fn identity_certificate(
    target: &RotationTarget,
    graph: &ControlGraph,
    tree: SpanningTree,
    leaf_order: LeafOrder,
    duration: f64,
) -> Result<ClosedLoopCertificate, GestureError> {
    let gates: Vec<GivensGate> = Vec::new();
    let program = OperatorProgram::new(CLOSED_LOOP_DIMENSION, Vec::new())?;
    let endpoint = endpoint_operator(&program)?;
    let metric = FrictionMetric::resolve(&gates, graph)?;
    let schedule = thermodynamic_schedule(&gates, &metric, duration)?;
    let identity = Matrix::identity(CLOSED_LOOP_DIMENSION);
    let (reconstruction_error_rad, _) =
        principal_angle_axis(&target.matrix().transpose().multiply(&endpoint))?;
    Ok(ClosedLoopCertificate {
        graph_edges: graph.sorted_edge_coordinates(),
        tree,
        leaf_order,
        target_angle_rad: 0.0,
        target_axis: [0.0; 3],
        balanced_generator_angle_rad: 0.0,
        local_commutator_axis: [0.0; 3],
        conjugator: identity.clone(),
        conjugator_branch: ConjugatorBranch::Aligned,
        macro_a: identity.clone(),
        macro_b: identity.clone(),
        dense_commutator: identity,
        macro_certificates: None,
        gates,
        reconstruction_error_fro: endpoint.frobenius_distance(target.matrix()),
        reconstruction_error_rad,
        orthogonality_error_fro: endpoint.orthogonality_residual(),
        determinant_error: (endpoint.determinant() - 1.0).abs(),
        program,
        endpoint,
        schedule,
    })
}

/// `G(left, right, theta)` as a dense `3x3`, built by rotating the identity
/// through [`mus_watsn::givens`] — consensus rule 1 forbids a second Givens
/// routine in this crate, so there is not one here either.
fn plane_rotation(left: usize, right: usize, theta: f64) -> Matrix {
    let mut out = Matrix::identity(CLOSED_LOOP_DIMENSION);
    out.rotate_rows(left, right, theta);
    out
}

/// `X Y X^-1 Y^-1` for orthogonal `X`, `Y` (inverse is transpose).
fn commutator(x: &Matrix, y: &Matrix) -> Matrix {
    x.multiply(y)
        .multiply(&x.transpose())
        .multiply(&y.transpose())
}

/// The exact inverse word: reversed order, negated angles, same edges.
fn inverse_word(gates: &[GivensGate]) -> Vec<GivensGate> {
    gates.iter().rev().map(|gate| gate.inverse()).collect()
}

fn normalized(vector: [f64; 3]) -> [f64; 3] {
    let norm = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if norm > 0.0 {
        [vector[0] / norm, vector[1] / norm, vector[2] / norm]
    } else {
        vector
    }
}

fn skew(vector: [f64; 3]) -> Matrix {
    Matrix::from_rows(vec![
        vec![0.0, -vector[2], vector[1]],
        vec![vector[2], 0.0, -vector[0]],
        vec![-vector[1], vector[0], 0.0],
    ])
    .expect("a hand-written 3x3 is rectangular")
}

/// A proper rotation carrying the unit vector `source` onto the unit vector
/// `target`, with the reference's deterministic branch choice.
///
/// The antipodal branch is the one worth naming: with no `source x target` to
/// rotate about, the reference picks the coordinate axis where `|source|` is
/// smallest (first index on a tie, as `numpy.argmin` does), projects `source`
/// out of it, and turns a half circle about the result. `I + 2K^2` is exactly
/// Rodrigues at `theta = pi`, so the branch is a rotation, not a reflection.
fn rotation_from_to(source: [f64; 3], target: [f64; 3]) -> (Matrix, ConjugatorBranch) {
    let source = normalized(source);
    let target = normalized(target);
    let cross = [
        source[1] * target[2] - source[2] * target[1],
        source[2] * target[0] - source[0] * target[2],
        source[0] * target[1] - source[1] * target[0],
    ];
    let cosine = source[0] * target[0] + source[1] * target[1] + source[2] * target[2];
    let sine = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();

    if sine < ANTIPODAL_SINE_THRESHOLD {
        if cosine > 0.0 {
            return (
                Matrix::identity(CLOSED_LOOP_DIMENSION),
                ConjugatorBranch::Aligned,
            );
        }
        let mut coordinate = 0usize;
        for candidate in 1..CLOSED_LOOP_DIMENSION {
            if source[candidate].abs() < source[coordinate].abs() {
                coordinate = candidate;
            }
        }
        let projection = source[coordinate];
        let mut axis = [0.0_f64; 3];
        axis[coordinate] = 1.0;
        for (component, value) in axis.iter_mut().zip(source.iter()) {
            *component -= value * projection;
        }
        let generator = skew(normalized(axis));
        let square = generator.multiply(&generator);
        let mut out = Matrix::identity(CLOSED_LOOP_DIMENSION);
        for row in 0..CLOSED_LOOP_DIMENSION {
            for column in 0..CLOSED_LOOP_DIMENSION {
                out.set(
                    row,
                    column,
                    out.get(row, column) + 2.0 * square.get(row, column),
                );
            }
        }
        return (out, ConjugatorBranch::Antipodal);
    }

    let generator = skew(cross);
    let square = generator.multiply(&generator);
    let scale = (1.0 - cosine) / (sine * sine);
    let mut out = Matrix::identity(CLOSED_LOOP_DIMENSION);
    for row in 0..CLOSED_LOOP_DIMENSION {
        for column in 0..CLOSED_LOOP_DIMENSION {
            out.set(
                row,
                column,
                out.get(row, column) + generator.get(row, column) + scale * square.get(row, column),
            );
        }
    }
    (out, ConjugatorBranch::General)
}

/// One authored edge's integrated control, as a receipt records it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EdgeClosureEntry {
    pub edge: u32,
    pub closure: f64,
}

/// The immutable record of one compiled closed-control gesture.
///
/// This is the Phase 1 [`crate::GestureReceipt`] extended, in the same
/// declaration-order-is-wire-order discipline, with what a closed loop adds:
/// the target's angle and axis, the balanced generator angle, the conjugator
/// and the branch that built it, the per-edge closure of the word, and the
/// thermodynamic timing of the twelve gates.
///
/// Correction 4 carries over: [`Self::work_residual`] is `Some` only when a
/// caller measured the lowered program through the `mus-watsn` ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClosedLoopReceipt {
    pub compiler_version: String,
    pub dimension: usize,
    pub graph_digest: String,
    pub target_digest: String,
    pub spanning_tree_digest: String,
    pub friction_metric_digest: String,
    pub spanning_tree_edges: Vec<[usize; 2]>,
    pub leaf_order: Vec<usize>,
    pub target_angle_rad: f64,
    pub target_axis: [f64; 3],
    pub balanced_generator_angle_rad: f64,
    pub local_commutator_axis: [f64; 3],
    pub conjugator_branch: ConjugatorBranch,
    pub conjugator: Vec<Vec<f64>>,
    pub gates: Vec<ReceiptGate>,
    pub gate_count: usize,
    pub expected_gate_count: usize,
    pub edge_closure: Vec<EdgeClosureEntry>,
    pub max_edge_closure: f64,
    pub duration: f64,
    pub segment_durations: Vec<f64>,
    pub thermodynamic_length: f64,
    pub predicted_minimum_quadratic_cost: f64,
    pub constant_speed_residual: f64,
    pub endpoint_operator: Vec<Vec<f64>>,
    pub endpoint_error_fro: f64,
    pub endpoint_error_rad: f64,
    pub orthogonality_residual_fro: f64,
    pub determinant_residual: f64,
    pub work_residual: Option<WorkResidual>,
}

impl ClosedLoopReceipt {
    /// Build a receipt from a closed-loop certificate.
    pub fn new(
        graph: &ControlGraph,
        target: &RotationTarget,
        certificate: &ClosedLoopCertificate,
        work: Option<&WorkMeasurement>,
    ) -> Self {
        let schedule = certificate.schedule();
        Self {
            compiler_version: COMPILER_VERSION.to_string(),
            dimension: certificate.dimension(),
            graph_digest: graph_digest(graph),
            target_digest: matrix_digest(target.matrix()),
            spanning_tree_digest: spanning_tree_digest(certificate.tree()),
            friction_metric_digest: friction_metric_digest(graph),
            spanning_tree_edges: certificate
                .tree()
                .edges()
                .iter()
                .map(|&(left, right)| [left, right])
                .collect(),
            leaf_order: certificate.leaf_order().as_slice().to_vec(),
            target_angle_rad: certificate.target_angle_rad(),
            target_axis: certificate.target_axis(),
            balanced_generator_angle_rad: certificate.balanced_generator_angle_rad(),
            local_commutator_axis: certificate.local_commutator_axis(),
            conjugator_branch: certificate.conjugator_branch(),
            conjugator: certificate.conjugator().to_rows(),
            gates: certificate
                .gates()
                .iter()
                .map(ReceiptGate::from_gate)
                .collect(),
            gate_count: certificate.gate_count(),
            expected_gate_count: if certificate.gate_count() == 0 {
                0
            } else {
                CLOSED_LOOP_GATE_COUNT
            },
            edge_closure: certificate
                .edge_closure()
                .into_iter()
                .map(|(edge, closure)| EdgeClosureEntry {
                    edge: edge.0,
                    closure,
                })
                .collect(),
            max_edge_closure: certificate.max_edge_closure(),
            duration: schedule.duration(),
            segment_durations: schedule.segment_durations().to_vec(),
            thermodynamic_length: schedule.thermodynamic_length(),
            predicted_minimum_quadratic_cost: schedule.minimum_quadratic_cost(),
            constant_speed_residual: schedule.constant_speed_residual(),
            endpoint_operator: certificate.endpoint().to_rows(),
            endpoint_error_fro: certificate.reconstruction_error_fro(),
            endpoint_error_rad: certificate.reconstruction_error_rad(),
            orthogonality_residual_fro: certificate.orthogonality_error_fro(),
            determinant_residual: certificate.determinant_error(),
            work_residual: work.map(WorkResidual::from_measurement),
        }
    }

    /// Serialize with a stable field order.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Parse a serialized receipt.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

/// What an independent replay of a closed-loop receipt found.
#[derive(Debug, Clone, PartialEq)]
pub struct ClosedLoopReplayReport {
    pub certificate: ClosedLoopCertificate,
    /// Worst per-gate angle difference between receipt and replay.
    pub max_angle_difference: f64,
    /// Frobenius distance between the recorded and replayed endpoint.
    pub endpoint_difference_fro: f64,
    /// The replay's own endpoint error against the target.
    pub reconstruction_error_fro: f64,
}

/// Replay a closed-loop receipt against the authored graph and target it
/// names.
///
/// A closed-loop receipt is fully determined by `(graph, target, duration)`,
/// so replay recompiles from scratch and compares. Structural disagreement is
/// [`GestureError::ReplayMismatch`]; numeric agreement is returned for the
/// caller to bound.
pub fn replay_closed_loop(
    receipt: &ClosedLoopReceipt,
    graph: &ControlGraph,
    target: &RotationTarget,
) -> Result<ClosedLoopReplayReport, GestureError> {
    let expected_graph = graph_digest(graph);
    if receipt.graph_digest != expected_graph {
        return Err(GestureError::ReplayMismatch {
            field: "graph_digest",
            detail: format!("receipt {} vs graph {expected_graph}", receipt.graph_digest),
        });
    }
    let expected_target = matrix_digest(target.matrix());
    if receipt.target_digest != expected_target {
        return Err(GestureError::ReplayMismatch {
            field: "target_digest",
            detail: format!(
                "receipt {} vs target {expected_target}",
                receipt.target_digest
            ),
        });
    }
    let expected_metric = friction_metric_digest(graph);
    if receipt.friction_metric_digest != expected_metric {
        return Err(GestureError::ReplayMismatch {
            field: "friction_metric_digest",
            detail: format!(
                "receipt {} vs graph {expected_metric}",
                receipt.friction_metric_digest
            ),
        });
    }

    let certificate = compile_exact_closed_loop_so3(target, graph, receipt.duration)?;
    if certificate.gate_count() != receipt.gates.len() {
        return Err(GestureError::ReplayMismatch {
            field: "gate_count",
            detail: format!(
                "receipt {} vs replay {}",
                receipt.gates.len(),
                certificate.gate_count()
            ),
        });
    }
    if certificate.conjugator_branch() != receipt.conjugator_branch {
        return Err(GestureError::ReplayMismatch {
            field: "conjugator_branch",
            detail: format!(
                "receipt {} vs replay {}",
                receipt.conjugator_branch.name(),
                certificate.conjugator_branch().name()
            ),
        });
    }

    let mut max_angle_difference = 0.0_f64;
    for (recorded, replayed) in receipt.gates.iter().zip(certificate.gates()) {
        if recorded.edge != replayed.edge.0
            || recorded.left != replayed.left
            || recorded.right != replayed.right
        {
            return Err(GestureError::ReplayMismatch {
                field: "gate_plane",
                detail: format!(
                    "receipt edge {} ({}, {}) vs replay edge {} ({}, {})",
                    recorded.edge,
                    recorded.left,
                    recorded.right,
                    replayed.edge.0,
                    replayed.left,
                    replayed.right
                ),
            });
        }
        max_angle_difference = max_angle_difference.max((recorded.theta - replayed.theta).abs());
    }

    let recorded_endpoint = Matrix::from_rows(receipt.endpoint_operator.clone())?;
    if recorded_endpoint.rows() != CLOSED_LOOP_DIMENSION
        || recorded_endpoint.columns() != CLOSED_LOOP_DIMENSION
    {
        return Err(GestureError::ReplayMismatch {
            field: "endpoint_operator",
            detail: format!(
                "receipt {}x{} vs dimension {CLOSED_LOOP_DIMENSION}",
                recorded_endpoint.rows(),
                recorded_endpoint.columns()
            ),
        });
    }

    Ok(ClosedLoopReplayReport {
        endpoint_difference_fro: recorded_endpoint.frobenius_distance(certificate.endpoint()),
        reconstruction_error_fro: certificate.reconstruction_error_fro(),
        max_angle_difference,
        certificate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorClass;
    use crate::work::{basis_probes, measure_control_work};
    use std::f64::consts::PI;

    fn path_graph() -> ControlGraph {
        ControlGraph::from_coordinate_edges(3, &[(0, 1), (1, 2)]).unwrap()
    }

    /// An axis-angle rotation written out independently of the compiler, by
    /// Rodrigues' formula on the raw entries.
    fn axis_angle(axis: [f64; 3], angle: f64) -> Matrix {
        let axis = normalized(axis);
        let (sin, cos) = angle.sin_cos();
        let mut out = Matrix::zeros(3, 3);
        for row in 0..3 {
            for column in 0..3 {
                let kronecker = if row == column { 1.0 } else { 0.0 };
                out.set(
                    row,
                    column,
                    kronecker * cos + (1.0 - cos) * axis[row] * axis[column],
                );
            }
        }
        out.set(0, 1, out.get(0, 1) - sin * axis[2]);
        out.set(0, 2, out.get(0, 2) + sin * axis[1]);
        out.set(1, 0, out.get(1, 0) + sin * axis[2]);
        out.set(1, 2, out.get(1, 2) - sin * axis[0]);
        out.set(2, 0, out.get(2, 0) - sin * axis[1]);
        out.set(2, 1, out.get(2, 1) + sin * axis[0]);
        out
    }

    fn axis_angle_target(axis: [f64; 3], angle: f64) -> RotationTarget {
        RotationTarget::new(axis_angle(axis, angle), 1.0e-9).unwrap()
    }

    #[test]
    fn the_balanced_angle_inverts_the_commutator_law() {
        let mut worst_law = 0.0_f64;
        let mut worst_angle = 0.0_f64;
        for step in 0..=64 {
            let phi = PI * step as f64 / 64.0;
            let t = balanced_commutator_angle(phi).unwrap();
            // The forward law, written out independently: sin(phi/2)^2 =
            // 4 x^2 (1 - x^2) with x = sin(t/2)^2.
            let x = (t / 2.0).sin().powi(2);
            let predicted = 4.0 * x * x * (1.0 - x * x);
            worst_law = worst_law.max((predicted - (phi / 2.0).sin().powi(2)).abs());
            assert!(
                (predicted - (phi / 2.0).sin().powi(2)).abs() <= 1.0e-15,
                "phi={phi}: law residual too large"
            );

            // And the dense local commutator really has that principal angle.
            let commutator = commutator(&plane_rotation(0, 1, t), &plane_rotation(1, 2, t));
            let (angle, _) = principal_angle_axis(&commutator).unwrap();
            worst_angle = worst_angle.max((angle - phi).abs());
            assert!(
                (angle - phi).abs() <= 1.0e-14,
                "phi={phi}: local commutator turned {angle}"
            );
        }
        println!("balanced law: worst residual {worst_law:e}, worst angle {worst_angle:e}");
    }

    #[test]
    fn the_balanced_angle_domain_is_typed() {
        for bad in [-1.0e-9, -1.0, PI + 1.0e-9, 4.0, f64::NAN, f64::INFINITY] {
            let error = balanced_commutator_angle(bad).unwrap_err();
            assert!(
                matches!(error, GestureError::PrincipalAngleOutOfDomain { .. }),
                "{bad} was not rejected"
            );
            assert_eq!(error.class(), ErrorClass::Malformed);
        }
        assert_eq!(balanced_commutator_angle(0.0).unwrap(), 0.0);
        assert!(balanced_commutator_angle(PI).is_ok());
    }

    /// The load-bearing convention test. The emitted word must realize
    /// `A B A^-1 B^-1` under *this crate's* front-to-back factor convention,
    /// and the reference's word order is only correct because its
    /// `schedule_matrix` happens to compose the same way. Every other block
    /// ordering realizes a materially different operator.
    #[test]
    fn the_word_realizes_a_b_ainv_binv_in_the_pinned_convention() {
        let graph = path_graph();
        let target = axis_angle_target([0.3, -0.7, 0.5], 1.9);
        let certificate = compile_exact_closed_loop_so3(&target, &graph, 1.0).unwrap();

        // Dense A B A^-1 B^-1, by explicit multiplication of the macros.
        let a = certificate.macro_a();
        let b = certificate.macro_b();
        let by_hand = a
            .multiply(b)
            .multiply(&a.transpose())
            .multiply(&b.transpose());
        assert!(
            certificate.endpoint().frobenius_distance(&by_hand) <= 1.0e-14,
            "lowered word disagrees with the dense commutator"
        );
        assert!(certificate.endpoint().frobenius_distance(target.matrix()) <= 1.0e-13);

        // The three other block orders are different operators, so the order
        // above is pinned rather than accidentally symmetric.
        let (certificate_a, certificate_b) = certificate.macro_certificates().unwrap();
        let word_a = certificate_a.gates().to_vec();
        let word_b = certificate_b.gates().to_vec();
        let orders: [Vec<GivensGate>; 3] = [
            // B A B^-1 A^-1 (the swapped commutator).
            [
                inverse_word(&word_a),
                inverse_word(&word_b),
                word_a.clone(),
                word_b.clone(),
            ]
            .concat(),
            // A B A^-1 B^-1 read as a left-action list (the naive port).
            [
                word_a.clone(),
                word_b.clone(),
                inverse_word(&word_a),
                inverse_word(&word_b),
            ]
            .concat(),
            // The whole word reversed.
            {
                let mut reversed = certificate.gates().to_vec();
                reversed.reverse();
                reversed
            },
        ];
        for (index, word) in orders.iter().enumerate() {
            let program = OperatorProgram::new(
                3,
                word.iter().map(|gate| gate.to_plane_rotation()).collect(),
            )
            .unwrap();
            let realized = endpoint_operator(&program).unwrap();
            assert!(
                realized.frobenius_distance(target.matrix()) > 1.0e-6,
                "alternative order {index} also reached the target"
            );
        }
    }

    #[test]
    fn the_identity_target_compiles_to_the_empty_word() {
        let graph = path_graph();
        let target = RotationTarget::identity(3).unwrap();
        let certificate = compile_exact_closed_loop_so3(&target, &graph, 2.0).unwrap();
        assert_eq!(certificate.gate_count(), 0);
        assert!(certificate.program().factors().is_empty());
        assert_eq!(certificate.target_angle_rad(), 0.0);
        assert_eq!(certificate.balanced_generator_angle_rad(), 0.0);
        assert_eq!(certificate.schedule().thermodynamic_length(), 0.0);
        assert!(certificate.schedule().executable_indices().is_empty());
        assert!(certificate.reconstruction_error_fro() <= 0.0);
        assert!(certificate.max_edge_closure() <= 1.0e-12);
    }

    #[test]
    fn the_antipodal_conjugator_branch_is_taken_and_exact() {
        let graph = path_graph();
        // Find the local commutator axis for one phi, then aim at its
        // negation: that is exactly the branch with no cross product.
        let phi = 1.234_f64;
        let t = balanced_commutator_angle(phi).unwrap();
        let local = commutator(&plane_rotation(0, 1, t), &plane_rotation(1, 2, t));
        let (_, local_axis) = principal_angle_axis(&local).unwrap();
        let opposite = [-local_axis[0], -local_axis[1], -local_axis[2]];

        let target = axis_angle_target(opposite, phi);
        let certificate = compile_exact_closed_loop_so3(&target, &graph, 1.0).unwrap();
        assert_eq!(
            certificate.conjugator_branch(),
            ConjugatorBranch::Antipodal,
            "expected the antipodal branch"
        );
        assert_eq!(certificate.gate_count(), CLOSED_LOOP_GATE_COUNT);
        assert!(
            certificate.reconstruction_error_fro() <= 1.0e-11,
            "antipodal endpoint error {:e}",
            certificate.reconstruction_error_fro()
        );
        // The conjugator is a rotation, not a reflection.
        let conjugator = certificate.conjugator();
        assert!((conjugator.determinant() - 1.0).abs() <= 1.0e-14);
        assert!(conjugator.orthogonality_residual() <= 1.0e-14);
        // And it is deterministic: the same input gives the same matrix.
        let again = compile_exact_closed_loop_so3(&target, &graph, 1.0).unwrap();
        assert_eq!(certificate.conjugator(), again.conjugator());
        assert_eq!(certificate.gates(), again.gates());
    }

    #[test]
    fn the_aligned_conjugator_branch_is_the_identity() {
        let graph = path_graph();
        let phi = 0.8_f64;
        let t = balanced_commutator_angle(phi).unwrap();
        let local = commutator(&plane_rotation(0, 1, t), &plane_rotation(1, 2, t));
        let (_, local_axis) = principal_angle_axis(&local).unwrap();
        let target = axis_angle_target(local_axis, phi);
        let certificate = compile_exact_closed_loop_so3(&target, &graph, 1.0).unwrap();
        assert_eq!(certificate.conjugator_branch(), ConjugatorBranch::Aligned);
        assert_eq!(certificate.conjugator(), &Matrix::identity(3));
        assert!(certificate.reconstruction_error_fro() <= 1.0e-11);
    }

    #[test]
    fn rotation_from_to_carries_the_source_onto_the_target() {
        let cases: [([f64; 3], [f64; 3], ConjugatorBranch); 4] = [
            ([1.0, 0.0, 0.0], [1.0, 0.0, 0.0], ConjugatorBranch::Aligned),
            (
                [1.0, 0.0, 0.0],
                [-1.0, 0.0, 0.0],
                ConjugatorBranch::Antipodal,
            ),
            (
                [0.0, 0.0, 1.0],
                [0.0, 0.0, -1.0],
                ConjugatorBranch::Antipodal,
            ),
            ([0.3, 0.4, 0.5], [-0.9, 0.2, 0.1], ConjugatorBranch::General),
        ];
        for (source, target, branch) in cases {
            let (rotation, taken) = rotation_from_to(source, target);
            assert_eq!(taken, branch);
            assert!((rotation.determinant() - 1.0).abs() <= 1.0e-14);
            assert!(rotation.orthogonality_residual() <= 1.0e-14);
            let unit_source = normalized(source);
            let unit_target = normalized(target);
            let mapped: Vec<f64> = (0..3)
                .map(|row| (0..3).map(|k| rotation.get(row, k) * unit_source[k]).sum())
                .collect();
            for (component, expected) in mapped.iter().zip(unit_target.iter()) {
                assert!(
                    (component - expected).abs() <= 1.0e-14,
                    "{source:?} -> {target:?} landed at {mapped:?}"
                );
            }
        }
    }

    #[test]
    fn non_three_dimensional_requests_are_typed_unsupported() {
        let four = ControlGraph::from_coordinate_edges(4, &[(0, 1), (1, 2), (2, 3)]).unwrap();
        let target = RotationTarget::identity(4).unwrap();
        let error = compile_exact_closed_loop_so3(&target, &four, 1.0).unwrap_err();
        assert_eq!(
            error,
            GestureError::ClosedLoopDimension { dimension: 4 },
            "graph dimension must be rejected"
        );
        assert_eq!(error.class(), ErrorClass::Unsupported);

        let graph = path_graph();
        let mismatched = RotationTarget::identity(4).unwrap();
        assert_eq!(
            compile_exact_closed_loop_so3(&mismatched, &graph, 1.0).unwrap_err(),
            GestureError::ClosedLoopDimension { dimension: 4 }
        );
    }

    #[test]
    fn disconnected_graphs_and_bad_durations_are_typed() {
        let split = ControlGraph::from_coordinate_edges(3, &[(0, 1)]).unwrap();
        let target = axis_angle_target([0.0, 0.0, 1.0], 0.5);
        assert_eq!(
            compile_exact_closed_loop_so3(&target, &split, 1.0).unwrap_err(),
            GestureError::DisconnectedGraph { components: 2 }
        );

        let graph = path_graph();
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                compile_exact_closed_loop_so3(&target, &graph, bad).unwrap_err(),
                GestureError::InvalidDuration { .. }
            ));
        }
    }

    #[test]
    fn a_receipt_round_trips_and_replays() {
        let graph = path_graph();
        let target = axis_angle_target([0.2, 0.9, -0.4], 2.7);
        let certificate = compile_exact_closed_loop_so3(&target, &graph, 3.0).unwrap();
        let work = measure_control_work(certificate.program(), &basis_probes(3)).unwrap();
        let receipt = ClosedLoopReceipt::new(&graph, &target, &certificate, Some(&work));

        assert_eq!(receipt.gate_count, CLOSED_LOOP_GATE_COUNT);
        assert_eq!(receipt.expected_gate_count, CLOSED_LOOP_GATE_COUNT);
        assert!(receipt.max_edge_closure <= 1.0e-12);
        assert_eq!(receipt.conjugator_branch, ConjugatorBranch::General);

        let text = receipt.to_json().unwrap();
        assert_eq!(text, receipt.to_json().unwrap());
        assert!(text.starts_with("{\n  \"compiler_version\""));
        let version_at = text.find("compiler_version").unwrap();
        let angle_at = text.find("target_angle_rad").unwrap();
        let work_at = text.find("work_residual").unwrap();
        assert!(version_at < angle_at && angle_at < work_at);
        let parsed = ClosedLoopReceipt::from_json(&text).unwrap();
        assert_eq!(parsed, receipt);

        let report = replay_closed_loop(&parsed, &graph, &target).unwrap();
        assert_eq!(report.max_angle_difference, 0.0);
        assert_eq!(report.endpoint_difference_fro, 0.0);
        assert!(report.reconstruction_error_fro <= 1.0e-11);
    }

    #[test]
    fn replaying_against_the_wrong_graph_or_target_is_typed() {
        let graph = path_graph();
        let target = axis_angle_target([1.0, 0.2, 0.3], 1.1);
        let certificate = compile_exact_closed_loop_so3(&target, &graph, 1.0).unwrap();
        let receipt = ClosedLoopReceipt::new(&graph, &target, &certificate, None);
        assert!(receipt.work_residual.is_none());

        let triangle = ControlGraph::from_coordinate_edges(3, &[(0, 1), (1, 2), (0, 2)]).unwrap();
        assert!(matches!(
            replay_closed_loop(&receipt, &triangle, &target).unwrap_err(),
            GestureError::ReplayMismatch {
                field: "graph_digest",
                ..
            }
        ));

        let other = axis_angle_target([0.0, 1.0, 0.0], 2.0);
        assert!(matches!(
            replay_closed_loop(&receipt, &graph, &other).unwrap_err(),
            GestureError::ReplayMismatch {
                field: "target_digest",
                ..
            }
        ));
    }

    #[test]
    fn principal_angle_axis_rejects_a_non_three_matrix() {
        assert!(matches!(
            principal_angle_axis(&Matrix::identity(4)).unwrap_err(),
            GestureError::ClosedLoopDimension { dimension: 4 }
        ));
    }

    #[test]
    fn principal_angle_axis_agrees_with_rodrigues_at_the_hard_angles() {
        let axis = normalized([0.37, -0.82, 0.44]);
        for angle in [
            1.0e-13_f64,
            1.0e-7,
            0.5,
            PI / 2.0,
            2.9,
            PI - 1.0e-7,
            PI - 1.0e-13,
        ] {
            let (recovered, recovered_axis) = principal_angle_axis(&axis_angle(axis, angle))
                .unwrap_or_else(|error| panic!("angle {angle}: {error}"));
            assert!(
                (recovered - angle).abs() <= 1.0e-7 * angle.max(1.0e-7),
                "angle {angle} recovered as {recovered}"
            );
            // Near pi the axis sign is ambiguous; compare up to sign there.
            let aligned: f64 = (0..3).map(|k| recovered_axis[k] * axis[k]).sum();
            assert!(
                (aligned.abs() - 1.0).abs() <= 1.0e-6,
                "angle {angle}: axis alignment {aligned}"
            );
        }
    }
}
