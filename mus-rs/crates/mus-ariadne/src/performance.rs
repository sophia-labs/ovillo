//! Deterministic runtime layout and structural operation counts.
//!
//! These are construction-time predictions, not wall-clock measurements.

use crate::ir::{ACTIVE_RANKS, MODE_COUNT, SPACE_COURSES, TRANSPORT_PAIRS};
use crate::plan::{BodyConstitutionSpec, ConstitutiveWorldPlan, WorldPlan};

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
        let delay_cells = delay_cells(plan.shared_space.delays_seconds, sample_rate);
        let body_count = plan.bodies.len();
        let material_scalars = body_count * 2 * MODE_COUNT;
        let aperture_scalars = body_count * 2;
        let constitutive_scalars = body_count * 2;
        Self::assemble(
            body_count,
            material_scalars,
            aperture_scalars,
            constitutive_scalars,
            delay_cells,
            plan.direct_couplings.len(),
        )
    }

    #[must_use]
    pub fn from_constitutive_plan(plan: &ConstitutiveWorldPlan) -> Self {
        let sample_rate = plan.internal_sample_rate_hz.max(1.0);
        let delay_cells = delay_cells(plan.shared_space.delays_seconds, sample_rate);
        let mut material_scalars = 0;
        let mut aperture_scalars = 0;
        let mut constitutive_scalars = 0;
        for body in &plan.bodies {
            match &body.constitution {
                BodyConstitutionSpec::Modal { .. } => {
                    material_scalars += 2 * MODE_COUNT;
                    aperture_scalars += 2;
                    constitutive_scalars += 2;
                }
                BodyConstitutionSpec::LocalMedium { boundary } => {
                    let sites = boundary.medium.sites.len();
                    material_scalars += 2 * sites;
                    aperture_scalars += MODE_COUNT;
                    constitutive_scalars += sites + 1;
                }
                BodyConstitutionSpec::ModalSpace { body } => {
                    material_scalars += 2 * MODE_COUNT;
                    aperture_scalars += 2;
                    constitutive_scalars += 2 + body.configuration_space.dimensions();
                }
                BodyConstitutionSpec::Hammer { hammer } => {
                    // Position, velocity, one felt compression per wire.
                    constitutive_scalars += 2 + hammer.wires.len();
                }
            }
        }
        Self::assemble(
            plan.bodies.len(),
            material_scalars,
            aperture_scalars,
            constitutive_scalars,
            delay_cells,
            plan.direct_couplings.len(),
        )
    }

    fn assemble(
        body_count: usize,
        material_scalars: usize,
        aperture_scalars: usize,
        constitutive_scalars: usize,
        delay_cells: usize,
        direct_coupling_edges: usize,
    ) -> Self {
        let persistent_scalars =
            material_scalars + aperture_scalars + constitutive_scalars + delay_cells;
        Self {
            body_count,
            material_scalars,
            aperture_scalars,
            constitutive_scalars,
            delay_cells,
            delay_heap_bytes: delay_cells * std::mem::size_of::<f64>(),
            direct_coupling_edges,
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
        Self::assemble(
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
            body_count * 8,
        )
    }

    #[must_use]
    pub fn for_constitutive_plan(plan: &ConstitutiveWorldPlan) -> Self {
        let body_count = plan.bodies.len();
        let mut modal_rotations = 0;
        let mut transport_rotations = 0;
        let mut aperture_rotations = 0;
        let mut active_projections = 0;
        let mut dot_product_terms = 0;
        let mut nonlinear_iterations = 0;
        for body in &plan.bodies {
            match &body.constitution {
                BodyConstitutionSpec::Modal { .. } => {
                    modal_rotations += MODE_COUNT;
                    transport_rotations += TRANSPORT_PAIRS;
                    aperture_rotations += 3;
                    active_projections += ACTIVE_RANKS;
                    dot_product_terms += 3 * MODE_COUNT
                        + 2 * TRANSPORT_PAIRS
                        + ACTIVE_RANKS * MODE_COUNT
                        + SPACE_COURSES * MODE_COUNT
                        + MODE_COUNT;
                    nonlinear_iterations += 8;
                }
                BodyConstitutionSpec::LocalMedium { boundary } => {
                    let sites = boundary.medium.sites.len();
                    transport_rotations += boundary.medium.edges.len();
                    aperture_rotations += boundary.inputs.len() + boundary.outputs.len();
                    active_projections += boundary
                        .medium
                        .sites
                        .iter()
                        .filter(|site| site.active_rate_per_second > 0.0)
                        .count();
                    dot_product_terms += SPACE_COURSES * MODE_COUNT
                        + boundary.inputs.len() * MODE_COUNT
                        + boundary.outputs.len() * (3 * sites + MODE_COUNT)
                        + MODE_COUNT;
                    nonlinear_iterations += sites;
                }
                BodyConstitutionSpec::Hammer { hammer } => {
                    // While thrown: each wire's start/free/compliance sums and
                    // its forced response; the nested force solve.
                    dot_product_terms += 5 * MODE_COUNT * hammer.wires.len();
                    nonlinear_iterations += 16 * hammer.wires.len();
                }
                BodyConstitutionSpec::ModalSpace { .. } => {
                    // Per sample, as a modal body; reconfiguration (an eigen
                    // solve for a tube network) is a control-rate cost, not
                    // counted here.
                    modal_rotations += MODE_COUNT;
                    transport_rotations += TRANSPORT_PAIRS;
                    aperture_rotations += 3;
                    active_projections += ACTIVE_RANKS;
                    dot_product_terms += 3 * MODE_COUNT + SPACE_COURSES * MODE_COUNT + MODE_COUNT;
                    nonlinear_iterations += 8;
                }
            }
        }
        let shared_space_rotations = body_count * SPACE_COURSES;
        let direct_coupling_rotations = plan.direct_couplings.len();
        dot_product_terms += 3 * SPACE_COURSES + 2 * direct_coupling_rotations * MODE_COUNT;
        Self::assemble(
            body_count,
            modal_rotations,
            transport_rotations,
            aperture_rotations,
            shared_space_rotations,
            direct_coupling_rotations,
            active_projections,
            SPACE_COURSES,
            SPACE_COURSES,
            dot_product_terms,
            nonlinear_iterations,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn assemble(
        body_count: usize,
        modal_rotations: usize,
        transport_rotations: usize,
        aperture_rotations: usize,
        shared_space_rotations: usize,
        direct_coupling_rotations: usize,
        active_projections: usize,
        delay_reads: usize,
        delay_writes: usize,
        dot_product_terms: usize,
        nonlinear_iterations: usize,
    ) -> Self {
        let trigonometric_evaluations = 2
            * (transport_rotations
                + aperture_rotations
                + shared_space_rotations
                + direct_coupling_rotations);
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

fn delay_cells(delays: [f64; SPACE_COURSES], sample_rate: f64) -> usize {
    delays
        .iter()
        .map(|delay| (delay * sample_rate).round().max(1.0) as usize)
        .sum()
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
