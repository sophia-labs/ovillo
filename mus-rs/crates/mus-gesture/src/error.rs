//! Typed compiler outcomes.
//!
//! Consensus §5.2 requires typed `Unreachable` / `Unsupported` /
//! `SolverFailed` / `Certified` outcomes. `Certified` is the `Ok` arm of every
//! compiler entry point (a [`crate::CompileCertificate`]); the failure arms are
//! the variants below, each grouped by [`GestureError::class`].
//!
//! Correction 1 to the Python reference lives here: a caller-supplied tree
//! edge that is not an authored graph edge is [`GestureError::TreeEdgeOffGraph`],
//! a hard rejection. The reference instead compiles the word and *reports*
//! `all_gates_on_graph = false`; a certificate that can carry a false "on
//! graph" bit is not a certificate.

use core::fmt;

use crate::graph::{EdgeId, ModeId};

/// Coarse outcome class, for callers that route on kind rather than variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// The request is malformed: bad shapes, ids, or weights.
    Malformed,
    /// The request is well formed but the target is not reachable under the
    /// authored controls (disconnected graph, cross-component target,
    /// determinant outside `SO(n)`).
    Unreachable,
    /// Well formed and reachable in principle, but this phase does not
    /// implement it.
    Unsupported,
    /// An internal invariant of the exact compiler failed. Never expected;
    /// reported rather than panicked so a fixture can retain it.
    SolverFailed,
}

/// Every way this crate declines to certify a gesture.
#[derive(Debug, Clone, PartialEq)]
pub enum GestureError {
    /// A graph with no modes cannot carry a rotation target.
    EmptyGraph,
    /// Two modes were declared with the same [`ModeId`].
    DuplicateModeId { id: ModeId },
    /// Two edges were declared with the same [`EdgeId`].
    DuplicateEdgeId { id: EdgeId },
    /// An edge referenced a mode that is not in the graph.
    UnknownMode { id: ModeId },
    /// An edge joined a mode to itself.
    SelfEdge { edge: EdgeId, mode: ModeId },
    /// Two edges connect the same unordered pair of modes.
    DuplicateEdge {
        edge: EdgeId,
        left: ModeId,
        right: ModeId,
    },
    /// Friction must be finite and strictly positive: it is a metric weight.
    InvalidFriction { edge: EdgeId, friction: f64 },
    /// A declared angle-rate limit must be finite and strictly positive.
    InvalidMaxAngleRate { edge: EdgeId, max_angle_rate: f64 },
    /// Rows of an authored matrix had differing lengths.
    RaggedMatrix { expected: usize, actual: usize },
    /// The target is not square, or its size is not the graph dimension.
    TargetShapeMismatch {
        expected: usize,
        rows: usize,
        columns: usize,
    },
    /// The target contains a non-finite entry.
    TargetNotFinite,
    /// A tolerance must be finite and strictly positive.
    InvalidTolerance { tolerance: f64 },
    /// `||R^T R - I||_F` exceeded the declared tolerance.
    TargetNotOrthogonal { residual: f64, tolerance: f64 },
    /// `|det R - 1|` exceeded the declared tolerance. Reflections (`det = -1`)
    /// land here: they are not in `SO(n)` and no rotation word reaches them.
    TargetDeterminantNotUnit { determinant: f64, tolerance: f64 },
    /// A full `SO(n)` target needs a connected control graph.
    DisconnectedGraph { components: usize },
    /// A disconnected graph was given a target that couples two components:
    /// no product of edge rotations can create that coupling.
    CrossComponentTarget {
        row: usize,
        column: usize,
        magnitude: f64,
    },
    /// Correction 1: a supplied tree edge is not an authored graph edge.
    TreeEdgeOffGraph { left: usize, right: usize },
    /// A spanning tree of an `n`-mode graph has exactly `n - 1` edges.
    TreeSizeMismatch { expected: usize, actual: usize },
    /// The supplied tree edges do not span the graph.
    TreeDisconnected,
    /// A tree edge joined a mode to itself.
    TreeSelfEdge { mode: usize },
    /// The same unordered pair appeared twice in the tree.
    TreeDuplicateEdge { left: usize, right: usize },
    /// A mode index in a supplied tree or leaf order is out of range.
    ModeIndexOutOfRange { index: usize, dimension: usize },
    /// A supplied leaf order is not a permutation of the mode indices.
    LeafOrderNotAPermutation { length: usize, dimension: usize },
    /// A supplied leaf order peels a mode that is not currently a leaf.
    LeafOrderNotAPeel { position: usize, mode: usize },
    /// Total duration must be finite and strictly positive.
    InvalidDuration { duration: f64 },
    /// A principal `SO(3)` angle handed to
    /// [`crate::balanced_commutator_angle`] was outside `[0, pi]`, or was not
    /// finite. The balanced inverse law has no other branch.
    PrincipalAngleOutOfDomain { angle: f64 },
    /// The exact closed-loop compiler is defined at
    /// [`crate::CLOSED_LOOP_DIMENSION`] only. Gotô's theorem supplies
    /// existence of a one-commutator representation in every compact connected
    /// semisimple group, but a constructive choice of macro factors for
    /// `SO(n)`, `n > 3`, is out of scope for this phase — so this is
    /// [`ErrorClass::Unsupported`], not a claim of unreachability.
    ClosedLoopDimension { dimension: usize },
    /// The exact word must contain `n(n-1)/2` gates for a full target.
    GateCountInvariant { expected: usize, actual: usize },
    /// The leaf-peel invariant failed mid-elimination.
    PeelInvariant { root: usize },
    /// `mus_watsn` refused the lowered program.
    Program(mus_watsn::ProgramError),
    /// A receipt did not reproduce under replay.
    ReplayMismatch { field: &'static str, detail: String },
}

impl GestureError {
    /// Coarse class of this outcome.
    pub fn class(&self) -> ErrorClass {
        match self {
            Self::EmptyGraph
            | Self::DuplicateModeId { .. }
            | Self::DuplicateEdgeId { .. }
            | Self::UnknownMode { .. }
            | Self::SelfEdge { .. }
            | Self::DuplicateEdge { .. }
            | Self::InvalidFriction { .. }
            | Self::InvalidMaxAngleRate { .. }
            | Self::RaggedMatrix { .. }
            | Self::TargetShapeMismatch { .. }
            | Self::TargetNotFinite
            | Self::InvalidTolerance { .. }
            | Self::TargetNotOrthogonal { .. }
            | Self::TreeEdgeOffGraph { .. }
            | Self::TreeSizeMismatch { .. }
            | Self::TreeDisconnected
            | Self::TreeSelfEdge { .. }
            | Self::TreeDuplicateEdge { .. }
            | Self::ModeIndexOutOfRange { .. }
            | Self::LeafOrderNotAPermutation { .. }
            | Self::LeafOrderNotAPeel { .. }
            | Self::InvalidDuration { .. }
            | Self::PrincipalAngleOutOfDomain { .. }
            | Self::Program(_) => ErrorClass::Malformed,
            Self::TargetDeterminantNotUnit { .. }
            | Self::DisconnectedGraph { .. }
            | Self::CrossComponentTarget { .. } => ErrorClass::Unreachable,
            Self::ClosedLoopDimension { .. } => ErrorClass::Unsupported,
            Self::GateCountInvariant { .. }
            | Self::PeelInvariant { .. }
            | Self::ReplayMismatch { .. } => ErrorClass::SolverFailed,
        }
    }
}

impl fmt::Display for GestureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGraph => write!(formatter, "control graph has no modes"),
            Self::DuplicateModeId { id } => {
                write!(formatter, "duplicate mode id {}", id.0)
            }
            Self::DuplicateEdgeId { id } => {
                write!(formatter, "duplicate edge id {}", id.0)
            }
            Self::UnknownMode { id } => {
                write!(formatter, "edge references unknown mode {}", id.0)
            }
            Self::SelfEdge { edge, mode } => write!(
                formatter,
                "edge {} joins mode {} to itself; self edges are not Givens controls",
                edge.0, mode.0
            ),
            Self::DuplicateEdge { edge, left, right } => write!(
                formatter,
                "edge {} repeats the mode pair ({}, {})",
                edge.0, left.0, right.0
            ),
            Self::InvalidFriction { edge, friction } => write!(
                formatter,
                "edge {} has friction {friction}; friction must be finite and positive",
                edge.0
            ),
            Self::InvalidMaxAngleRate {
                edge,
                max_angle_rate,
            } => write!(
                formatter,
                "edge {} has max_angle_rate {max_angle_rate}; it must be finite and positive",
                edge.0
            ),
            Self::RaggedMatrix { expected, actual } => write!(
                formatter,
                "matrix row has {actual} entries, expected {expected}"
            ),
            Self::TargetShapeMismatch {
                expected,
                rows,
                columns,
            } => write!(
                formatter,
                "target is {rows}x{columns}, expected {expected}x{expected}"
            ),
            Self::TargetNotFinite => write!(formatter, "target has a non-finite entry"),
            Self::InvalidTolerance { tolerance } => write!(
                formatter,
                "tolerance {tolerance} must be finite and positive"
            ),
            Self::TargetNotOrthogonal {
                residual,
                tolerance,
            } => write!(
                formatter,
                "target orthogonality residual {residual} exceeds tolerance {tolerance}"
            ),
            Self::TargetDeterminantNotUnit {
                determinant,
                tolerance,
            } => write!(
                formatter,
                "target determinant {determinant} is not 1 within tolerance {tolerance}; \
                 only SO(n) targets are reachable by rotations"
            ),
            Self::DisconnectedGraph { components } => write!(
                formatter,
                "graph has {components} components; a full SO(n) target needs a connected graph"
            ),
            Self::CrossComponentTarget {
                row,
                column,
                magnitude,
            } => write!(
                formatter,
                "target entry ({row}, {column}) = {magnitude} couples two graph components"
            ),
            Self::TreeEdgeOffGraph { left, right } => write!(
                formatter,
                "tree edge ({left}, {right}) is not an authored graph edge"
            ),
            Self::TreeSizeMismatch { expected, actual } => write!(
                formatter,
                "spanning tree has {actual} edges, expected {expected}"
            ),
            Self::TreeDisconnected => write!(formatter, "tree edges do not span the graph"),
            Self::TreeSelfEdge { mode } => {
                write!(formatter, "tree edge joins mode {mode} to itself")
            }
            Self::TreeDuplicateEdge { left, right } => {
                write!(formatter, "tree repeats the edge ({left}, {right})")
            }
            Self::ModeIndexOutOfRange { index, dimension } => write!(
                formatter,
                "mode index {index} is outside a graph of dimension {dimension}"
            ),
            Self::LeafOrderNotAPermutation { length, dimension } => write!(
                formatter,
                "leaf order has {length} entries and is not a permutation of {dimension} modes"
            ),
            Self::LeafOrderNotAPeel { position, mode } => write!(
                formatter,
                "leaf order position {position} peels mode {mode}, which is not a leaf there"
            ),
            Self::InvalidDuration { duration } => {
                write!(formatter, "duration {duration} must be finite and positive")
            }
            Self::PrincipalAngleOutOfDomain { angle } => write!(
                formatter,
                "principal SO(3) angle {angle} must lie in [0, pi]"
            ),
            Self::ClosedLoopDimension { dimension } => write!(
                formatter,
                "the exact closed-loop compiler is defined at dimension 3; got {dimension}"
            ),
            Self::GateCountInvariant { expected, actual } => write!(
                formatter,
                "tree-QR emitted {actual} gates, expected {expected}"
            ),
            Self::PeelInvariant { root } => write!(
                formatter,
                "elimination toward root {root} found no removable non-root leaf"
            ),
            Self::Program(error) => write!(formatter, "mus-watsn rejected the program: {error:?}"),
            Self::ReplayMismatch { field, detail } => {
                write!(formatter, "receipt replay mismatch on {field}: {detail}")
            }
        }
    }
}

impl std::error::Error for GestureError {}

impl From<mus_watsn::ProgramError> for GestureError {
    fn from(error: mus_watsn::ProgramError) -> Self {
        Self::Program(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflections_and_disconnection_are_unreachable_not_malformed() {
        assert_eq!(
            GestureError::TargetDeterminantNotUnit {
                determinant: -1.0,
                tolerance: 1.0e-9
            }
            .class(),
            ErrorClass::Unreachable
        );
        assert_eq!(
            GestureError::DisconnectedGraph { components: 2 }.class(),
            ErrorClass::Unreachable
        );
        assert_eq!(
            GestureError::CrossComponentTarget {
                row: 0,
                column: 3,
                magnitude: 0.5
            }
            .class(),
            ErrorClass::Unreachable
        );
    }

    #[test]
    fn off_graph_tree_edges_are_malformed_and_describe_themselves() {
        let error = GestureError::TreeEdgeOffGraph { left: 1, right: 4 };
        assert_eq!(error.class(), ErrorClass::Malformed);
        assert_eq!(
            error.to_string(),
            "tree edge (1, 4) is not an authored graph edge"
        );
    }
}
