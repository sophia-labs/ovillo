//! Finite, seeded search over spanning trees and leaf orders.
//!
//! The exact compiler is the oracle; the search only chooses *which* exact
//! factorization to run. The gate count is `n(n-1)/2` for every candidate, so
//! nothing here can trade accuracy for cost. What varies is which authored
//! edges carry the angles, and a heterogeneous friction metric can make that
//! worth a great deal of thermodynamic length.
//!
//! Two properties are structural rather than empirical.
//!
//! * **Monotone by construction.** The baseline (breadth-first tree from mode
//!   0, minimum-index leaf peel) is evaluated first and held as the incumbent.
//!   A candidate replaces it only on a *strictly* lower predicted cost, so the
//!   returned cost can never exceed the baseline cost, whatever the seed or
//!   candidate budget.
//! * **Every candidate is independently replayed.** Consensus rule 6: a
//!   candidate is rejected outright unless its own recomputed endpoint error
//!   is within [`SearchConfig::endpoint_tolerance`]. Optimization cannot
//!   silently degrade endpoint correctness, because a degraded candidate is
//!   never eligible to win.

use crate::compile::{compile, compile_on_tree, CompileCertificate};
use crate::error::GestureError;
use crate::graph::ControlGraph;
use crate::rng::SplitMix64;
use crate::schedule::{thermodynamic_schedule, FrictionMetric, ThermodynamicSchedule};
use crate::target::RotationTarget;

/// Search budget and acceptance policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchConfig {
    /// Total candidates *including* the baseline. `0` or `1` evaluates the
    /// baseline alone.
    pub candidates: usize,
    /// Seed for the tree/leaf-order generator. The whole search is a pure
    /// function of this seed and the inputs.
    pub seed: u64,
    /// Total duration the word is timed over.
    pub duration: f64,
    /// A candidate whose own reconstruction error exceeds this is rejected
    /// before its cost is even considered. Defaults to the Phase 1 gate.
    pub endpoint_tolerance: f64,
}

impl SearchConfig {
    /// `candidates` candidates at `seed` over `duration`, with the Phase 1
    /// endpoint tolerance of `1e-11` Frobenius.
    pub fn new(candidates: usize, seed: u64, duration: f64) -> Self {
        Self {
            candidates,
            seed,
            duration,
            endpoint_tolerance: 1.0e-11,
        }
    }
}

/// The result of a finite factorization search.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchOutcome {
    seed: u64,
    candidates_requested: usize,
    candidates_evaluated: usize,
    candidates_rejected: usize,
    improvements: usize,
    baseline_cost: f64,
    best_cost: f64,
    certificate: CompileCertificate,
    metric: FrictionMetric,
    schedule: ThermodynamicSchedule,
}

impl SearchOutcome {
    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn candidates_requested(&self) -> usize {
        self.candidates_requested
    }

    /// Candidates whose compilation succeeded and was scored, including the
    /// baseline.
    pub fn candidates_evaluated(&self) -> usize {
        self.candidates_evaluated
    }

    /// Candidates discarded before scoring, either because the random tree
    /// construction failed or because the independent endpoint replay was out
    /// of tolerance.
    pub fn candidates_rejected(&self) -> usize {
        self.candidates_rejected
    }

    /// How many times the incumbent was strictly improved upon.
    pub fn improvements(&self) -> usize {
        self.improvements
    }

    pub fn baseline_cost(&self) -> f64 {
        self.baseline_cost
    }

    pub fn best_cost(&self) -> f64 {
        self.best_cost
    }

    /// `(baseline - best) / baseline`, or `0.0` for a zero-cost baseline.
    pub fn improvement_fraction(&self) -> f64 {
        if self.baseline_cost > 0.0 {
            (self.baseline_cost - self.best_cost) / self.baseline_cost
        } else {
            0.0
        }
    }

    pub fn certificate(&self) -> &CompileCertificate {
        &self.certificate
    }

    pub fn metric(&self) -> &FrictionMetric {
        &self.metric
    }

    pub fn schedule(&self) -> &ThermodynamicSchedule {
        &self.schedule
    }
}

/// Search spanning trees and leaf orders for a cheaper exact factorization.
pub fn search_factorization(
    target: &RotationTarget,
    graph: &ControlGraph,
    config: SearchConfig,
) -> Result<SearchOutcome, GestureError> {
    let baseline_certificate = compile(target, graph)?;
    let baseline_metric = FrictionMetric::resolve(baseline_certificate.gates(), graph)?;
    let baseline_schedule = thermodynamic_schedule(
        baseline_certificate.gates(),
        &baseline_metric,
        config.duration,
    )?;
    let baseline_cost = baseline_schedule.minimum_quadratic_cost();

    let mut outcome = SearchOutcome {
        seed: config.seed,
        candidates_requested: config.candidates.max(1),
        candidates_evaluated: 1,
        candidates_rejected: 0,
        improvements: 0,
        baseline_cost,
        best_cost: baseline_cost,
        certificate: baseline_certificate,
        metric: baseline_metric,
        schedule: baseline_schedule,
    };

    let mut rng = SplitMix64::new(config.seed);
    for _ in 1..config.candidates.max(1) {
        let Ok(tree) = graph.random_spanning_tree(&mut rng) else {
            outcome.candidates_rejected += 1;
            continue;
        };
        let Ok(order) = tree.randomized_leaf_order(&mut rng) else {
            outcome.candidates_rejected += 1;
            continue;
        };
        let Ok(certificate) = compile_on_tree(target, graph, &tree, &order) else {
            outcome.candidates_rejected += 1;
            continue;
        };
        // Independent replay before the candidate is allowed to compete.
        let error = certificate.reconstruction_error_fro();
        if !error.is_finite() || error > config.endpoint_tolerance {
            outcome.candidates_rejected += 1;
            continue;
        }
        let metric = FrictionMetric::resolve(certificate.gates(), graph)?;
        let schedule = thermodynamic_schedule(certificate.gates(), &metric, config.duration)?;
        outcome.candidates_evaluated += 1;
        let cost = schedule.minimum_quadratic_cost();
        if cost < outcome.best_cost {
            outcome.best_cost = cost;
            outcome.improvements += 1;
            outcome.certificate = certificate;
            outcome.metric = metric;
            outcome.schedule = schedule;
        }
    }

    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{EdgeControl, EdgeId, ModeId};
    use crate::matrix::Matrix;
    use crate::rng::SplitMix64;

    /// An 8-mode graph with a path backbone, a few chords, and strongly
    /// heterogeneous friction — the shape the reference's weighted-tree
    /// experiment uses.
    fn weighted_graph() -> ControlGraph {
        let n = 8usize;
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                if j == i + 1 || (i * 7 + j * 11) % 5 == 0 {
                    pairs.push((i, j));
                }
            }
        }
        let modes: Vec<ModeId> = (0..n as u32).map(ModeId).collect();
        let mut rng = SplitMix64::new(0xf1c7_1047);
        let edges: Vec<EdgeControl> = pairs
            .iter()
            .enumerate()
            .map(|(index, &(left, right))| {
                let draw = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
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

    fn dense_givens(n: usize, left: usize, right: usize, theta: f64) -> Matrix {
        let mut out = Matrix::identity(n);
        let (sin, cos) = theta.sin_cos();
        out.set(left, left, cos);
        out.set(left, right, -sin);
        out.set(right, left, sin);
        out.set(right, right, cos);
        out
    }

    fn sample_target() -> RotationTarget {
        let planes = [(0, 3), (1, 6), (2, 5), (4, 7), (0, 1), (3, 6)];
        let angles = [0.9_f64, -1.3, 0.4, 1.7, -0.6, 1.1];
        let mut matrix = Matrix::identity(8);
        for (&(left, right), &theta) in planes.iter().zip(angles.iter()) {
            matrix = dense_givens(8, left, right, theta).multiply(&matrix);
        }
        RotationTarget::new(matrix, 1.0e-9).unwrap()
    }

    #[test]
    fn search_never_worsens_the_baseline() {
        let graph = weighted_graph();
        let target = sample_target();
        for seed in [1u64, 7, 42, 1234, 0x0a71_ad0c] {
            for candidates in [0usize, 1, 2, 8, 64] {
                let outcome =
                    search_factorization(&target, &graph, SearchConfig::new(candidates, seed, 2.0))
                        .unwrap();
                assert!(
                    outcome.best_cost() <= outcome.baseline_cost(),
                    "seed {seed} candidates {candidates}: {} > {}",
                    outcome.best_cost(),
                    outcome.baseline_cost()
                );
                assert!(outcome.improvement_fraction() >= 0.0);
                assert!(outcome.candidates_evaluated() >= 1);
                assert!(
                    outcome.candidates_evaluated() + outcome.candidates_rejected()
                        <= candidates.max(1)
                );
                // Whatever won is still an exact factorization.
                assert_eq!(
                    outcome.certificate().gate_count(),
                    outcome.certificate().dimension_lower_bound()
                );
                assert!(outcome.certificate().reconstruction_error_fro() <= 1.0e-11);
                for gate in outcome.certificate().gates() {
                    assert!(graph.edge_between(gate.left, gate.right).is_some());
                }
            }
        }
    }

    #[test]
    fn a_zero_candidate_budget_returns_the_baseline_itself() {
        let graph = weighted_graph();
        let target = sample_target();
        let outcome = search_factorization(&target, &graph, SearchConfig::new(0, 3, 2.0)).unwrap();
        let baseline = compile(&target, &graph).unwrap();
        assert_eq!(outcome.certificate().gates(), baseline.gates());
        assert_eq!(outcome.best_cost(), outcome.baseline_cost());
        assert_eq!(outcome.improvements(), 0);
    }

    #[test]
    fn the_search_is_a_pure_function_of_its_seed() {
        let graph = weighted_graph();
        let target = sample_target();
        let config = SearchConfig::new(48, 0x5eed, 2.0);
        let first = search_factorization(&target, &graph, config).unwrap();
        let second = search_factorization(&target, &graph, config).unwrap();
        assert_eq!(first, second);
        let other = search_factorization(
            &target,
            &graph,
            SearchConfig {
                seed: 0x5eed ^ 1,
                ..config
            },
        )
        .unwrap();
        assert_eq!(other.baseline_cost(), first.baseline_cost());
    }

    #[test]
    fn a_heterogeneous_metric_gives_the_search_something_to_find() {
        let graph = weighted_graph();
        let target = sample_target();
        let outcome =
            search_factorization(&target, &graph, SearchConfig::new(256, 0xbeef, 2.0)).unwrap();
        assert!(
            outcome.best_cost() < outcome.baseline_cost(),
            "expected a cheaper factorization to exist: {} vs {}",
            outcome.best_cost(),
            outcome.baseline_cost()
        );
        assert!(outcome.improvements() >= 1);
        // The winner's schedule really does cost what the search claims.
        assert!(
            (outcome.schedule().realized_quadratic_cost() - outcome.best_cost()).abs()
                <= 1.0e-9 * outcome.best_cost()
        );
    }

    #[test]
    fn an_identity_target_has_a_zero_baseline_and_no_improvement() {
        let graph = weighted_graph();
        let target = RotationTarget::identity(8).unwrap();
        let outcome =
            search_factorization(&target, &graph, SearchConfig::new(16, 11, 2.0)).unwrap();
        assert_eq!(outcome.baseline_cost(), 0.0);
        assert_eq!(outcome.best_cost(), 0.0);
        assert_eq!(outcome.improvement_fraction(), 0.0);
    }
}
