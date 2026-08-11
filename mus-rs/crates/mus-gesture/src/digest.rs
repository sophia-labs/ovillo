//! Content digests for the objects a receipt has to pin down.
//!
//! A receipt quotes digests rather than whole inputs so that replay can prove
//! it was handed the same graph, the same target, the same tree, and the same
//! friction metric — cheaply, and without the receipt growing to the size of
//! its inputs.
//!
//! The encoding is deliberately explicit rather than derived from a
//! serialization format: a domain tag, then length-prefixed fields in a fixed
//! order, with every `f64` as its IEEE-754 little-endian bit pattern. Negative
//! zero is normalized to positive zero so that `theta = -0.0` and
//! `theta = 0.0` — which denote the identical rotation, and which `atan2` will
//! produce interchangeably — do not digest differently.

use sha2::{Digest, Sha256};

use crate::graph::{ControlGraph, SpanningTree};
use crate::matrix::Matrix;

/// Incremental digest over a tagged, length-prefixed field sequence.
#[derive(Debug, Clone)]
pub struct Digester {
    hasher: Sha256,
}

impl Digester {
    /// Start a digest in the domain named by `tag`.
    pub fn new(tag: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update((tag.len() as u64).to_le_bytes());
        hasher.update(tag.as_bytes());
        Self { hasher }
    }

    pub fn push_usize(&mut self, value: usize) -> &mut Self {
        self.hasher.update((value as u64).to_le_bytes());
        self
    }

    pub fn push_u32(&mut self, value: u32) -> &mut Self {
        self.hasher.update(value.to_le_bytes());
        self
    }

    /// Push an `f64` by bit pattern, with `-0.0` normalized to `0.0`.
    pub fn push_f64(&mut self, value: f64) -> &mut Self {
        let normalized = if value == 0.0 { 0.0 } else { value };
        self.hasher.update(normalized.to_bits().to_le_bytes());
        self
    }

    pub fn push_bytes(&mut self, value: &[u8]) -> &mut Self {
        self.hasher.update((value.len() as u64).to_le_bytes());
        self.hasher.update(value);
        self
    }

    /// Lowercase hex of the SHA-256 digest.
    pub fn finish(self) -> String {
        let bytes = self.hasher.finalize();
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            out.push_str(&format!("{byte:02x}"));
        }
        out
    }
}

/// Digest of the authored topology: dimension, mode ids in coordinate order,
/// and every edge as `(id, min coordinate, max coordinate)` sorted by
/// coordinate pair. Friction is deliberately excluded — it is the *metric*,
/// digested separately, so a receipt can show that only the metric changed.
pub fn graph_digest(graph: &ControlGraph) -> String {
    let mut digester = Digester::new("mus-gesture/graph/1");
    digester.push_usize(graph.dimension());
    for mode in graph.modes() {
        digester.push_u32(mode.0);
    }
    let mut edges: Vec<(usize, usize, u32)> = graph
        .edges()
        .iter()
        .zip(graph.edge_coordinates())
        .map(|(edge, &(left, right))| (left, right, edge.id.0))
        .collect();
    edges.sort_unstable();
    digester.push_usize(edges.len());
    for (left, right, id) in edges {
        digester.push_usize(left);
        digester.push_usize(right);
        digester.push_u32(id);
    }
    digester.finish()
}

/// Digest of the friction metric: the same edge order as
/// [`graph_digest`], each with its weight and declared angle-rate limit.
pub fn friction_metric_digest(graph: &ControlGraph) -> String {
    let mut digester = Digester::new("mus-gesture/friction-metric/1");
    let mut edges: Vec<(usize, usize, u32, f64, Option<f64>)> = graph
        .edges()
        .iter()
        .zip(graph.edge_coordinates())
        .map(|(edge, &(left, right))| (left, right, edge.id.0, edge.friction, edge.max_angle_rate))
        .collect();
    edges.sort_unstable_by_key(|entry| (entry.0, entry.1, entry.2));
    digester.push_usize(edges.len());
    for (left, right, id, friction, max_angle_rate) in edges {
        digester.push_usize(left);
        digester.push_usize(right);
        digester.push_u32(id);
        digester.push_f64(friction);
        match max_angle_rate {
            Some(rate) => {
                digester.push_usize(1);
                digester.push_f64(rate);
            }
            None => {
                digester.push_usize(0);
            }
        }
    }
    digester.finish()
}

/// Digest of a dense matrix in row-major order.
pub fn matrix_digest(matrix: &Matrix) -> String {
    let mut digester = Digester::new("mus-gesture/matrix/1");
    digester.push_usize(matrix.rows());
    digester.push_usize(matrix.columns());
    for value in matrix.as_slice() {
        digester.push_f64(*value);
    }
    digester.finish()
}

/// Digest of a spanning tree: its canonical sorted coordinate pairs together
/// with the authored edge ids they resolve to.
pub fn spanning_tree_digest(tree: &SpanningTree) -> String {
    let mut digester = Digester::new("mus-gesture/spanning-tree/1");
    digester.push_usize(tree.dimension());
    digester.push_usize(tree.edges().len());
    for (&(left, right), id) in tree.edges().iter().zip(tree.edge_ids()) {
        digester.push_usize(left);
        digester.push_usize(right);
        digester.push_u32(id.0);
    }
    digester.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{EdgeControl, EdgeId, ModeId};

    fn path(n: usize) -> ControlGraph {
        let pairs: Vec<(usize, usize)> = (0..n - 1).map(|i| (i, i + 1)).collect();
        ControlGraph::from_coordinate_edges(n, &pairs).unwrap()
    }

    #[test]
    fn digests_are_stable_and_hex() {
        let graph = path(5);
        let first = graph_digest(&graph);
        assert_eq!(first, graph_digest(&graph));
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn edge_order_does_not_change_the_graph_digest_but_topology_does() {
        let modes = vec![ModeId(0), ModeId(1), ModeId(2)];
        let forward = ControlGraph::new(
            modes.clone(),
            vec![
                EdgeControl::new(EdgeId(0), ModeId(0), ModeId(1)),
                EdgeControl::new(EdgeId(1), ModeId(1), ModeId(2)),
            ],
        )
        .unwrap();
        let reordered = ControlGraph::new(
            modes.clone(),
            vec![
                EdgeControl::new(EdgeId(1), ModeId(2), ModeId(1)),
                EdgeControl::new(EdgeId(0), ModeId(1), ModeId(0)),
            ],
        )
        .unwrap();
        assert_eq!(graph_digest(&forward), graph_digest(&reordered));

        let different = ControlGraph::new(
            modes,
            vec![
                EdgeControl::new(EdgeId(0), ModeId(0), ModeId(1)),
                EdgeControl::new(EdgeId(1), ModeId(0), ModeId(2)),
            ],
        )
        .unwrap();
        assert_ne!(graph_digest(&forward), graph_digest(&different));
    }

    #[test]
    fn friction_is_outside_the_graph_digest_and_inside_the_metric_digest() {
        let light = ControlGraph::new(
            vec![ModeId(0), ModeId(1)],
            vec![EdgeControl::with_friction(
                EdgeId(0),
                ModeId(0),
                ModeId(1),
                1.0,
            )],
        )
        .unwrap();
        let heavy = ControlGraph::new(
            vec![ModeId(0), ModeId(1)],
            vec![EdgeControl::with_friction(
                EdgeId(0),
                ModeId(0),
                ModeId(1),
                4.0,
            )],
        )
        .unwrap();
        assert_eq!(graph_digest(&light), graph_digest(&heavy));
        assert_ne!(
            friction_metric_digest(&light),
            friction_metric_digest(&heavy)
        );
    }

    #[test]
    fn negative_zero_digests_as_zero() {
        let mut positive = Matrix::identity(2);
        positive.set(0, 1, 0.0);
        let mut negative = Matrix::identity(2);
        negative.set(0, 1, -0.0);
        assert_eq!(matrix_digest(&positive), matrix_digest(&negative));
    }

    #[test]
    fn different_trees_of_the_same_graph_digest_differently() {
        let pairs: Vec<(usize, usize)> = (0..4)
            .flat_map(|i| ((i + 1)..4).map(move |j| (i, j)))
            .collect();
        let graph = ControlGraph::from_coordinate_edges(4, &pairs).unwrap();
        let star = graph.bfs_spanning_tree(0).unwrap();
        let chain = SpanningTree::new(&graph, vec![(0, 1), (1, 2), (2, 3)]).unwrap();
        assert_ne!(spanning_tree_digest(&star), spanning_tree_digest(&chain));
    }
}
