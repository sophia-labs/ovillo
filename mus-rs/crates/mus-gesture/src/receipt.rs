//! Compiler-side receipts, and their replay.
//!
//! A receipt is the immutable statement of what the compiler decided, in a
//! form another process — or another implementation — can check. Consensus
//! §3.1 point 7 keeps predicted and measured fields distinct, and this type
//! honours that: [`GestureReceipt::predicted_minimum_quadratic_cost`] is a
//! closed-form prediction, [`GestureReceipt::endpoint_error_fro`] and
//! [`GestureReceipt::work_residual`] are measurements of the compiled word.
//!
//! Field order in the serialized JSON is the declaration order below and is
//! stable across runs, so two receipts of the same compilation are
//! byte-identical.
//!
//! **Correction 4.** [`GestureReceipt::work_residual`] is `Option`. It is
//! `Some` only when a caller measured the lowered program through the
//! `mus-watsn` ledger in a named metric (see [`crate::work`]); it is `None`
//! otherwise. The reference's `zero_work_residual` — an orthogonality probe on
//! one fixed vector — is not ported, and no predicted zero is ever written
//! into this field.

use serde::{Deserialize, Serialize};

use crate::compile::{compile_on_tree, CompileCertificate, GivensGate};
use crate::digest::{friction_metric_digest, graph_digest, matrix_digest, spanning_tree_digest};
use crate::error::GestureError;
use crate::graph::{ControlGraph, EdgeId, LeafOrder, SpanningTree};
use crate::schedule::ThermodynamicSchedule;
use crate::target::RotationTarget;
use crate::work::WorkMeasurement;

/// Identity of the compiler that produced a receipt.
pub const COMPILER_VERSION: &str = concat!("mus-gesture/", env!("CARGO_PKG_VERSION"));

/// One gate as a receipt records it: the authored edge id, the coordinate
/// orientation, and the angle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReceiptGate {
    pub edge: u32,
    pub left: usize,
    pub right: usize,
    pub theta: f64,
}

impl ReceiptGate {
    fn from_gate(gate: &GivensGate) -> Self {
        Self {
            edge: gate.edge.0,
            left: gate.left,
            right: gate.right,
            theta: gate.theta,
        }
    }
}

/// A measured work statement, always accompanied by the metric it was
/// measured in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkResidual {
    pub metric: String,
    pub probe_states: usize,
    pub frames: u64,
    pub declared_control_work: f64,
    pub max_abs_residual: f64,
    pub global_residual: f64,
}

impl WorkResidual {
    fn from_measurement(measurement: &WorkMeasurement) -> Self {
        Self {
            metric: measurement.metric().to_string(),
            probe_states: measurement.probe_states(),
            frames: measurement.frames(),
            declared_control_work: measurement.declared_control_work(),
            max_abs_residual: measurement.max_abs_residual(),
            global_residual: measurement.global_residual(),
        }
    }
}

/// The immutable record of one compiled gesture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GestureReceipt {
    pub compiler_version: String,
    pub dimension: usize,
    pub graph_digest: String,
    pub target_digest: String,
    pub spanning_tree_digest: String,
    pub friction_metric_digest: String,
    /// The tree edges themselves, so replay needs nothing but this receipt,
    /// the authored graph, and the target.
    pub spanning_tree_edges: Vec<[usize; 2]>,
    pub leaf_order: Vec<usize>,
    pub gates: Vec<ReceiptGate>,
    pub gate_count: usize,
    pub dimension_lower_bound: usize,
    pub duration: f64,
    pub segment_durations: Vec<f64>,
    pub thermodynamic_length: f64,
    pub predicted_minimum_quadratic_cost: f64,
    pub constant_speed_residual: f64,
    pub endpoint_operator: Vec<Vec<f64>>,
    pub endpoint_error_fro: f64,
    pub orthogonality_residual_fro: f64,
    pub determinant_residual: f64,
    pub work_residual: Option<WorkResidual>,
}

impl GestureReceipt {
    /// Build a receipt from a certificate and its timing.
    pub fn new(
        graph: &ControlGraph,
        target: &RotationTarget,
        certificate: &CompileCertificate,
        schedule: &ThermodynamicSchedule,
        work: Option<&WorkMeasurement>,
    ) -> Self {
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
            gates: certificate
                .gates()
                .iter()
                .map(ReceiptGate::from_gate)
                .collect(),
            gate_count: certificate.gate_count(),
            dimension_lower_bound: certificate.dimension_lower_bound(),
            duration: schedule.duration(),
            segment_durations: schedule.segment_durations().to_vec(),
            thermodynamic_length: schedule.thermodynamic_length(),
            predicted_minimum_quadratic_cost: schedule.minimum_quadratic_cost(),
            constant_speed_residual: schedule.constant_speed_residual(),
            endpoint_operator: certificate.endpoint().to_rows(),
            endpoint_error_fro: certificate.reconstruction_error_fro(),
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

    /// Signed integrated angle per authored edge, in canonical orientation.
    /// Correction 3: assert a bound on these, never equality with zero.
    pub fn edge_closure(&self) -> Vec<(EdgeId, f64)> {
        let gates: Vec<GivensGate> = self
            .gates
            .iter()
            .map(|gate| GivensGate {
                edge: EdgeId(gate.edge),
                left: gate.left,
                right: gate.right,
                theta: gate.theta,
            })
            .collect();
        crate::compile::edge_closure(&gates)
    }
}

/// What an independent replay of a receipt found.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayReport {
    /// Recompiled certificate, from the receipt's own tree and leaf order.
    pub certificate: CompileCertificate,
    /// Worst per-gate angle difference between receipt and replay.
    pub max_angle_difference: f64,
    /// Frobenius distance between the recorded and replayed endpoint.
    pub endpoint_difference_fro: f64,
    /// The replay's own reconstruction error against the target.
    pub reconstruction_error_fro: f64,
}

/// Replay a receipt against the authored graph and target it names.
///
/// Every digest must match, the tree and leaf order are taken from the receipt
/// rather than recomputed, and the word is compiled again from scratch. A
/// mismatch in structure is a typed [`GestureError::ReplayMismatch`]; numeric
/// agreement is returned for the caller to bound.
pub fn replay(
    receipt: &GestureReceipt,
    graph: &ControlGraph,
    target: &RotationTarget,
) -> Result<ReplayReport, GestureError> {
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
    if receipt.dimension != graph.dimension() {
        return Err(GestureError::ReplayMismatch {
            field: "dimension",
            detail: format!(
                "receipt {} vs graph {}",
                receipt.dimension,
                graph.dimension()
            ),
        });
    }

    let tree = SpanningTree::new(
        graph,
        receipt
            .spanning_tree_edges
            .iter()
            .map(|pair| (pair[0], pair[1]))
            .collect(),
    )?;
    let expected_tree = spanning_tree_digest(&tree);
    if receipt.spanning_tree_digest != expected_tree {
        return Err(GestureError::ReplayMismatch {
            field: "spanning_tree_digest",
            detail: format!(
                "receipt {} vs rebuilt {expected_tree}",
                receipt.spanning_tree_digest
            ),
        });
    }
    let order = LeafOrder::new(&tree, receipt.leaf_order.clone())?;
    let certificate = compile_on_tree(target, graph, &tree, &order)?;

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

    let recorded_endpoint = crate::matrix::Matrix::from_rows(receipt.endpoint_operator.clone())?;
    if recorded_endpoint.rows() != certificate.dimension()
        || recorded_endpoint.columns() != certificate.dimension()
    {
        return Err(GestureError::ReplayMismatch {
            field: "endpoint_operator",
            detail: format!(
                "receipt {}x{} vs dimension {}",
                recorded_endpoint.rows(),
                recorded_endpoint.columns(),
                certificate.dimension()
            ),
        });
    }

    Ok(ReplayReport {
        endpoint_difference_fro: recorded_endpoint.frobenius_distance(certificate.endpoint()),
        reconstruction_error_fro: certificate.reconstruction_error_fro(),
        max_angle_difference,
        certificate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::compile;
    use crate::graph::{EdgeControl, EdgeId as GraphEdgeId, ModeId};
    use crate::matrix::Matrix;
    use crate::schedule::{thermodynamic_schedule, FrictionMetric};
    use crate::work::{basis_probes, measure_control_work};

    fn weighted_path(n: usize) -> ControlGraph {
        let modes: Vec<ModeId> = (0..n as u32).map(ModeId).collect();
        let edges: Vec<EdgeControl> = (0..n - 1)
            .map(|i| {
                EdgeControl::with_friction(
                    GraphEdgeId(i as u32),
                    ModeId(i as u32),
                    ModeId(i as u32 + 1),
                    1.0 + 0.5 * (i as f64),
                )
            })
            .collect();
        ControlGraph::new(modes, edges).unwrap()
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

    fn fixture() -> (ControlGraph, RotationTarget, GestureReceipt) {
        let graph = weighted_path(6);
        let target = RotationTarget::new(
            dense_givens(6, 0, 4, 1.05).multiply(&dense_givens(6, 2, 5, -0.62)),
            1.0e-9,
        )
        .unwrap();
        let certificate = compile(&target, &graph).unwrap();
        let metric = FrictionMetric::resolve(certificate.gates(), &graph).unwrap();
        let schedule = thermodynamic_schedule(certificate.gates(), &metric, 2.0).unwrap();
        let work = measure_control_work(certificate.program(), &basis_probes(6)).unwrap();
        let receipt = GestureReceipt::new(&graph, &target, &certificate, &schedule, Some(&work));
        (graph, target, receipt)
    }

    #[test]
    fn a_receipt_replays_exactly() {
        let (graph, target, receipt) = fixture();
        let report = replay(&receipt, &graph, &target).unwrap();
        assert_eq!(report.max_angle_difference, 0.0);
        assert_eq!(report.endpoint_difference_fro, 0.0);
        assert!(report.reconstruction_error_fro <= 1.0e-11);
        assert_eq!(receipt.gate_count, receipt.dimension_lower_bound);
    }

    #[test]
    fn serialization_is_stable_and_round_trips() {
        let (_, _, receipt) = fixture();
        let first = receipt.to_json().unwrap();
        let second = receipt.to_json().unwrap();
        assert_eq!(first, second);
        let parsed = GestureReceipt::from_json(&first).unwrap();
        assert_eq!(parsed, receipt);
        assert_eq!(parsed.to_json().unwrap(), first);
        // Declaration order is the wire order.
        let version_at = first.find("compiler_version").unwrap();
        let digest_at = first.find("graph_digest").unwrap();
        let work_at = first.find("work_residual").unwrap();
        assert!(version_at < digest_at);
        assert!(digest_at < work_at);
        assert!(first.starts_with("{\n  \"compiler_version\""));
    }

    /// Regression guard for the `float_roundtrip` feature on `serde_json`.
    /// Without it the deserializer is a fast approximate parser that lands one
    /// ulp away from values like the one below, and a receipt stops being
    /// bit-exact the moment it is written to disk and read back.
    #[test]
    fn awkward_floats_survive_the_json_round_trip_bit_for_bit() {
        for value in [
            -5.311_435_383_648_925_6e-17_f64,
            1.0 / 3.0,
            std::f64::consts::PI,
            1.0e-300,
            -0.0,
            2.220_446_049_250_313e-16,
        ] {
            let text = serde_json::to_string(&value).unwrap();
            let back: f64 = serde_json::from_str(&text).unwrap();
            assert_eq!(
                value.to_bits(),
                back.to_bits(),
                "{value:?} round-tripped through {text} as {back:?}"
            );
        }
    }

    #[test]
    fn a_serialized_receipt_replays_after_a_round_trip() {
        let (graph, target, receipt) = fixture();
        let text = receipt.to_json().unwrap();
        let parsed = GestureReceipt::from_json(&text).unwrap();
        let report = replay(&parsed, &graph, &target).unwrap();
        assert_eq!(report.max_angle_difference, 0.0);
    }

    #[test]
    fn the_work_field_is_measured_and_optional() {
        let (graph, target, with_work) = fixture();
        let work = with_work.work_residual.clone().unwrap();
        assert_eq!(work.metric, crate::work::EUCLIDEAN_NORMALIZED_METRIC);
        assert_eq!(work.declared_control_work, 0.0);
        assert!(work.max_abs_residual <= 1.0e-14);
        assert_eq!(work.frames, (with_work.gate_count * 6) as u64);

        let certificate = compile(&target, &graph).unwrap();
        let metric = FrictionMetric::resolve(certificate.gates(), &graph).unwrap();
        let schedule = thermodynamic_schedule(certificate.gates(), &metric, 2.0).unwrap();
        let without = GestureReceipt::new(&graph, &target, &certificate, &schedule, None);
        assert!(without.work_residual.is_none());
        assert!(without
            .to_json()
            .unwrap()
            .contains("\"work_residual\": null"));
    }

    #[test]
    fn replaying_against_the_wrong_graph_or_target_is_typed() {
        let (graph, target, receipt) = fixture();

        let other_graph = weighted_path(5);
        assert!(matches!(
            replay(&receipt, &other_graph, &target).unwrap_err(),
            GestureError::ReplayMismatch {
                field: "graph_digest",
                ..
            }
        ));

        let other_target = RotationTarget::new(dense_givens(6, 1, 3, 0.2), 1.0e-9).unwrap();
        assert!(matches!(
            replay(&receipt, &graph, &other_target).unwrap_err(),
            GestureError::ReplayMismatch {
                field: "target_digest",
                ..
            }
        ));

        let mut retuned = receipt.clone();
        retuned.friction_metric_digest = "0".repeat(64);
        assert!(matches!(
            replay(&retuned, &graph, &target).unwrap_err(),
            GestureError::ReplayMismatch {
                field: "friction_metric_digest",
                ..
            }
        ));
    }

    #[test]
    fn a_tampered_angle_is_caught_by_the_numeric_report() {
        let (graph, target, receipt) = fixture();
        let mut tampered = receipt.clone();
        tampered.gates[0].theta += 1.0e-6;
        let report = replay(&tampered, &graph, &target).unwrap();
        assert!(report.max_angle_difference >= 9.0e-7);
    }

    #[test]
    fn a_tampered_tree_is_caught_before_recompilation() {
        let (graph, target, receipt) = fixture();
        let mut tampered = receipt.clone();
        tampered.spanning_tree_digest = "f".repeat(64);
        assert!(matches!(
            replay(&tampered, &graph, &target).unwrap_err(),
            GestureError::ReplayMismatch {
                field: "spanning_tree_digest",
                ..
            }
        ));

        let mut off_graph = receipt.clone();
        off_graph.spanning_tree_edges[0] = [0, 5];
        assert!(matches!(
            replay(&off_graph, &graph, &target).unwrap_err(),
            GestureError::TreeEdgeOffGraph { .. }
        ));
    }
}
