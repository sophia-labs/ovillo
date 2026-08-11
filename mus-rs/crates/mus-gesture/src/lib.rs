//! Exact graph-constrained inverse rotation compilation.
//!
//! Given an authored [`ControlGraph`] — normalized modes joined by two-mode
//! Givens controls with friction weights — and an [`RotationTarget`] in
//! `SO(n)`, this crate emits an exact word of authored-edge plane rotations
//! that realizes the target, times it at constant thermodynamic speed, and
//! records the decision as a replayable [`GestureReceipt`].
//!
//! ```text
//! authored graph + SO(n) target + friction + duration
//!     -> exact tree-QR word          (n(n-1)/2 authored-edge gates)
//!     -> constant-speed timing       (closed form, no iteration)
//!     -> mus_watsn::OperatorProgram  (the executable artifact)
//!     -> receipt                     (digests, angles, measurements)
//! ```
//!
//! # What this crate is not
//!
//! Consensus rule 1: `mus-gesture` never defines a second factor executor, a
//! second Givens routine, or a second work ledger. The plane-rotation kernel
//! is [`mus_watsn::givens`], the executable word is
//! [`mus_watsn::OperatorProgram`], and measured work goes through
//! [`mus_watsn::WorkLedger`]. The dense matrix helper in [`matrix`] exists
//! only for construction-time bookkeeping at `n <= 32`; nothing here runs in
//! an audio callback.
//!
//! Consensus §5.2 also fixes the scope of this phase. The exact closed
//! `SO(3)` compiler, the MUS-F inverse surface, deformation controls, and the
//! topological lab are later commits and are deliberately absent.
//!
//! # Deliberate deviations from the Python reference
//!
//! The float64 oracle in
//! `paper/ariadne-gesture-compiler/reference/ariadne_gesture` remains the
//! independent implementation this crate replays against
//! (`tests/oracle_replay.rs`). Four differences are intentional, and each is
//! documented at the site that implements it.
//!
//! 1. **Off-graph trees are hard-rejected.** The reference's
//!    `compile_so_on_tree` accepts caller-supplied tree edges that are not in
//!    the graph, compiles anyway, and *reports* `all_gates_on_graph = false`.
//!    Here, [`SpanningTree::new`] and [`compile_on_tree`] return
//!    [`GestureError::TreeEdgeOffGraph`]. A certificate that can carry a false
//!    "on graph" bit is not a certificate, so there is no such bit — every
//!    [`GivensGate`] names the [`EdgeId`] it runs on, structurally.
//!    See [`graph`] and [`compile`].
//!
//! 2. **One composition convention, pinned to `mus-watsn`.** The reference
//!    mixes left-action list order (`schedule_matrix`) with a right-product
//!    (`_loop_product`). This crate has exactly one: an
//!    [`mus_watsn::OperatorProgram`] applies its factors front to back to a
//!    column state, so `[F0, F1, ..., Fm]` realizes `Fm * ... * F1 * F0`.
//!    `compile::tests::three_gate_word_matches_the_hand_computed_product`
//!    checks a non-commuting three-gate word against a product computed by
//!    hand, and checks that the opposite convention gives a different
//!    operator. See [`compile`].
//!
//! 3. **Closure is asserted as a bound.** Per-edge integrated angle closure is
//!    asserted `<= 1e-12`, never `== 0.0`. [`CompileCertificate::edge_closure`]
//!    reports the signed per-edge sums in canonical orientation and callers
//!    bound them. See [`compile`] and `tests/closure.rs`.
//!
//! 4. **No `zero_work_residual`.** The reference function of that name is an
//!    orthogonality probe on one fixed vector, not a work measurement, and is
//!    not ported. A receipt's work field is `Some` only when the lowered
//!    program was actually measured through the `mus-watsn` ledger in the
//!    named metric [`work::EUCLIDEAN_NORMALIZED_METRIC`], and is `None`
//!    otherwise. See [`work`] and [`receipt`].
//!
//! A fifth, smaller difference is recorded in [`schedule`]: a word of total
//! thermodynamic length zero gets `tau_k = 0` everywhere and an empty
//! executable schedule, rather than the reference's uniform spread of the
//! duration over gates that do nothing.
//!
//! # Dependencies
//!
//! `mus-watsn` is the only intra-workspace dependency, as consensus §5.2
//! requires. Beyond it the crate uses `serde`/`serde_json`, which Commit D's
//! receipts are specified in terms of, and `sha2` for the content digests
//! those receipts quote — the same digest dependency `mus-oplog` and
//! `mus-vocab` already carry. There is no parser, RDF, host, audio-device, or
//! linear-algebra dependency.
//!
//! # Example
//!
//! ```
//! use mus_gesture::{
//!     compile, matrix::Matrix, thermodynamic_schedule, ControlGraph, FrictionMetric,
//!     GestureReceipt, RotationTarget,
//! };
//!
//! // A five-mode path: only neighbouring modes have an authored control.
//! let graph = ControlGraph::from_coordinate_edges(5, &[(0, 1), (1, 2), (2, 3), (3, 4)])?;
//!
//! // A target that couples modes 0 and 4, which share no edge.
//! let mut matrix = Matrix::identity(5);
//! let (sin, cos) = 0.9_f64.sin_cos();
//! matrix.set(0, 0, cos);
//! matrix.set(0, 4, -sin);
//! matrix.set(4, 0, sin);
//! matrix.set(4, 4, cos);
//! let target = RotationTarget::new(matrix, 1e-9)?;
//!
//! let certificate = compile(&target, &graph)?;
//! assert_eq!(certificate.gate_count(), 10); // 5 * 4 / 2
//! assert!(certificate.reconstruction_error_fro() < 1e-11);
//!
//! let metric = FrictionMetric::resolve(certificate.gates(), &graph)?;
//! let schedule = thermodynamic_schedule(certificate.gates(), &metric, 2.0)?;
//! let receipt = GestureReceipt::new(&graph, &target, &certificate, &schedule, None);
//! assert_eq!(receipt.gate_count, 10);
//! # Ok::<(), mus_gesture::GestureError>(())
//! ```

pub mod compile;
pub mod digest;
pub mod error;
pub mod graph;
pub mod matrix;
pub mod receipt;
pub mod rng;
pub mod schedule;
pub mod search;
pub mod target;
pub mod work;

pub use compile::{
    compile, compile_block_diagonal, compile_on_tree, edge_closure, endpoint_operator,
    BlockDiagonalCertificate, CompileCertificate, GivensGate,
};
pub use digest::{friction_metric_digest, graph_digest, matrix_digest, spanning_tree_digest};
pub use error::{ErrorClass, GestureError};
pub use graph::{
    canonical_pair, ComponentReport, ControlGraph, EdgeControl, EdgeId, LeafOrder, ModeId,
    SpanningTree,
};
pub use matrix::Matrix;
pub use receipt::{
    replay, GestureReceipt, ReceiptGate, ReplayReport, WorkResidual, COMPILER_VERSION,
};
pub use rng::SplitMix64;
pub use schedule::{thermodynamic_schedule, FrictionMetric, ThermodynamicSchedule};
pub use search::{search_factorization, SearchConfig, SearchOutcome};
pub use target::{RotationTarget, DEFAULT_TARGET_TOLERANCE};
pub use work::{
    basis_probes, measure_control_work, metric_energy, WorkMeasurement, EUCLIDEAN_NORMALIZED_METRIC,
};
