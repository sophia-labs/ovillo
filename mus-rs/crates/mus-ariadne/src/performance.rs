//! Deterministic runtime layout and structural operation counts.
//!
//! These are construction-time predictions, not wall-clock measurements.

use crate::ir::{ACTIVE_RANKS, MODE_COUNT, SPACE_COURSES, TRANSPORT_PAIRS};
use crate::plan::WorldPlan;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WorldLayout {
    pub body_count: usize,
    pub material_scalars: usize,
    pub aperture_scalars: usize,
    pub constitutive_scalars: usize,
    pub delay_cells: usize,
    pub delay_heap_bytes: usize,
    pub direct_coupling_edges: usize,
    pub persistent_scalar_bytes: usize,
}

impl WorldLayout {
    #[must_use]
    pub fn from_plan(plan: &WorldPlan) -> Self {
        let sample_rate = plan.internal_sample_rate_hz.max(1.0);
        let delay_cells = plan
            .shared_space
            .delays_seconds
            .iter()
            .map(|delay| (delay * sample_rate).round().max(1.0) as usize)
            .sum::<usize>();
        let body_count = plan.bodies.len();
        let material_scalars = body_count * 2 * MODE_COUNT;
        let aperture_scalars = body_count * 2;
        let constitutive_scalars = body_count * 2;
        let persistent_scalars =
            material_scalars + aperture_scalars + constitutive_scalars + delay_cells;
        Self {
            body_count,
            material_scalars,
            aperture_scalars,
            constitutive_scalars,
            delay_cells,
            delay_heap_bytes: delay_cells * std::mem::size_of::<f64>(),
            direct_coupling_edges: plan.direct_couplings.len(),
            persistent_scalar_bytes: persistent_scalars * std::mem::size_of::<f64>(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WorldCostEstimate {
    pub body_count: usize,
    pub modal_rotations: usize,
    pub transport_rotations: usize,
    pub aperture_rotations: usize,
    pub shared_space_rotations: usize,
    pub direct_coupling_rotations: usize,
    pub active_projections: usize,
    pub delay_reads: usize,
    pub delay_writes: usize,
    pub dot_product_terms: usize,
    pub trigonometric_evaluations: usize,
    pub nonlinear_iterations: usize,
    pub estimated_scalar_operations: usize,
    pub measured: bool,
}

impl WorldCostEstimate {
    #[must_use]
    pub fn for_plan(plan: &WorldPlan) -> Self {
        let body_count = plan.bodies.len();
        let modal_rotations = body_count * MODE_COUNT;
        let transport_rotations = body_count * TRANSPORT_PAIRS;
        let aperture_rotations = body_count * 3;
        let shared_space_rotations = body_count * SPACE_COURSES;
        let direct_coupling_rotations = plan.direct_couplings.len();
        let active_projections = body_count * ACTIVE_RANKS;
        let delay_reads = SPACE_COURSES;
        let delay_writes = SPACE_COURSES;
        let dot_product_terms = body_count
            * (3 * MODE_COUNT
                + 2 * TRANSPORT_PAIRS
                + ACTIVE_RANKS * MODE_COUNT
                + SPACE_COURSES * MODE_COUNT
                + MODE_COUNT)
            + 3 * SPACE_COURSES
            + 2 * direct_coupling_rotations * MODE_COUNT;
        let trigonometric_evaluations = 2
            * (transport_rotations
                + aperture_rotations
                + shared_space_rotations
                + direct_coupling_rotations);
        let nonlinear_iterations = body_count * 8;
        let estimated_scalar_operations = 12 * modal_rotations
            + 18 * transport_rotations
            + 24 * aperture_rotations
            + 18 * shared_space_rotations
            + 18 * direct_coupling_rotations
            + 20 * active_projections
            + 3 * (delay_reads + delay_writes)
            + 2 * dot_product_terms
            + 12 * nonlinear_iterations;
        Self {
            body_count,
            modal_rotations,
            transport_rotations,
            aperture_rotations,
            shared_space_rotations,
            direct_coupling_rotations,
            active_projections,
            delay_reads,
            delay_writes,
            dot_product_terms,
            trigonometric_evaluations,
            nonlinear_iterations,
            estimated_scalar_operations,
            measured: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyInstanceSpec, WorldPlan};

    #[test]
    fn shared_delay_storage_is_not_duplicated_per_body() {
        let mut one = WorldPlan::string_brass_v1(48_000.0);
        let one_layout = WorldLayout::from_plan(&one);
        let mut second: BodyInstanceSpec = one.bodies[0].clone();
        second.id = "body/1".to_owned();
        one.bodies.push(second);
        let two_layout = WorldLayout::from_plan(&one);
        assert_eq!(one_layout.delay_cells, two_layout.delay_cells);
        assert_eq!(two_layout.material_scalars, 2 * one_layout.material_scalars);
    }

    #[test]
    fn structural_cost_is_not_mislabeled_as_measurement() {
        let cost = WorldCostEstimate::for_plan(&WorldPlan::string_brass_v1(48_000.0));
        assert!(!cost.measured);
        assert_eq!(cost.modal_rotations, MODE_COUNT);
        assert!(cost.estimated_scalar_operations > 0);
    }
}
