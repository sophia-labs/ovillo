//! Deterministic runtime layout and operation counts for an Ariadne world.
//!
//! These are structural predictions, not wall-clock benchmark results.

use crate::ir::{WorldSpec, ACTIVE_RANKS, MODE_COUNT, SPACE_COURSES, TRANSPORT_PAIRS};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeLayout {
    pub material_scalars: usize,
    pub aperture_scalars: usize,
    pub constitutive_scalars: usize,
    pub delay_cells: usize,
    pub delay_heap_bytes: usize,
    pub persistent_scalar_bytes: usize,
}

impl RuntimeLayout {
    #[must_use]
    pub fn from_spec(spec: &WorldSpec) -> Self {
        let sample_rate = spec.internal_sample_rate_hz.max(1.0);
        let delay_cells = spec
            .space
            .delays_seconds
            .iter()
            .map(|delay| (delay * sample_rate).round().max(1.0) as usize)
            .sum::<usize>();
        let material_scalars = 2 * MODE_COUNT;
        let aperture_scalars = 2;
        let constitutive_scalars = 2;
        let persistent_scalars =
            material_scalars + aperture_scalars + constitutive_scalars + delay_cells;
        Self {
            material_scalars,
            aperture_scalars,
            constitutive_scalars,
            delay_cells,
            delay_heap_bytes: delay_cells * std::mem::size_of::<f64>(),
            persistent_scalar_bytes: persistent_scalars * std::mem::size_of::<f64>(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CostEstimate {
    pub modal_rotations: usize,
    pub transport_rotations: usize,
    pub aperture_rotations: usize,
    pub space_rotations: usize,
    pub active_projections: usize,
    pub delay_reads: usize,
    pub delay_writes: usize,
    pub dot_product_terms: usize,
    pub trigonometric_evaluations: usize,
    pub nonlinear_iterations: usize,
    pub estimated_scalar_operations: usize,
    pub measured: bool,
}

impl CostEstimate {
    #[must_use]
    pub const fn canonical() -> Self {
        let modal_rotations = MODE_COUNT;
        let transport_rotations = TRANSPORT_PAIRS;
        let aperture_rotations = 3;
        let space_rotations = SPACE_COURSES;
        let active_projections = ACTIVE_RANKS;
        let delay_reads = SPACE_COURSES;
        let delay_writes = SPACE_COURSES;
        let dot_product_terms = 3 * MODE_COUNT
            + 2 * TRANSPORT_PAIRS
            + ACTIVE_RANKS * MODE_COUNT
            + SPACE_COURSES * MODE_COUNT
            + 3 * SPACE_COURSES
            + MODE_COUNT;
        let trigonometric_evaluations =
            2 * (transport_rotations + aperture_rotations + space_rotations);
        let nonlinear_iterations = 8;
        let estimated_scalar_operations = 12 * modal_rotations
            + 18 * transport_rotations
            + 24 * aperture_rotations
            + 18 * space_rotations
            + 20 * active_projections
            + 3 * (delay_reads + delay_writes)
            + 2 * dot_product_terms
            + 12 * nonlinear_iterations;
        Self {
            modal_rotations,
            transport_rotations,
            aperture_rotations,
            space_rotations,
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

    #[must_use]
    pub fn for_spec(_spec: &WorldSpec) -> Self {
        Self::canonical()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_counts_delay_storage() {
        let spec = WorldSpec::string_brass_v1();
        let layout = RuntimeLayout::from_spec(&spec);
        assert!(layout.delay_cells > 0);
        assert_eq!(layout.delay_heap_bytes, layout.delay_cells * 8);
    }

    #[test]
    fn cost_is_not_mislabeled_as_measurement() {
        let cost = CostEstimate::canonical();
        assert!(!cost.measured);
        assert_eq!(cost.modal_rotations, MODE_COUNT);
        assert!(cost.estimated_scalar_operations > 0);
    }
}
