use crate::body::BodyControls;
use crate::plan::{BodyInstanceSpec, DirectCouplingSpec, WorldPlan};

use super::*;

fn two_body_plan(coupled: bool) -> WorldPlan {
    let mut plan = WorldPlan::string_brass_v1(96_000.0);
    plan.world_name = "two-body-fixture".to_owned();
    let mut second: BodyInstanceSpec = plan.bodies[0].clone();
    second.id = "body/1".to_owned();
    second.pose.position = [1.0, 0.0, 0.0];
    plan.bodies.push(second);
    if coupled {
        let direction = std::array::from_fn(|index| if index == 0 { 1.0 } else { 0.0 });
        plan.direct_couplings.push(DirectCouplingSpec {
            id: "bridge/0".to_owned(),
            left_body: "body/0".to_owned(),
            right_body: "body/1".to_owned(),
            left_direction: direction,
            right_direction: direction,
            rate_hz: 240.0,
            weight: 1.0,
        });
    }
    plan
}

#[test]
fn zero_world_is_an_exact_fixed_point() {
    let mut world = World::from_plan(two_body_plan(false)).unwrap();
    let controls = vec![BodyControls::default(); 2];
    for _ in 0..4_096 {
        let frame = world
            .process_sample(&controls, WorldControls::default())
            .unwrap();
        assert_eq!(frame.source_work, 0.0);
        assert_eq!(frame.radiation, 0.0);
        assert_eq!(frame.space_emitted_left, 0.0);
        assert_eq!(frame.space_emitted_right, 0.0);
        assert!(world.body_emissions().iter().all(|value| *value == 0.0));
    }
    assert!(world.receipt().global_closure_residual.abs() < 1.0e-14);
}

#[test]
fn body_instances_have_independent_mutable_state() {
    let mut plan = two_body_plan(false);
    plan.bodies[1].space_port.coupling_weights = [0.0; crate::ir::SPACE_COURSES];
    let mut world = World::from_plan(plan).unwrap();
    let initial_second = world.body_state(1).unwrap();
    let mut controls = vec![BodyControls::default(); 2];
    controls[0].source_wave = 0.8;
    world
        .process_sample(&controls, WorldControls::default())
        .unwrap();
    assert_ne!(world.body_state(0).unwrap(), initial_second);
    assert_eq!(world.body_state(1).unwrap(), initial_second);
}

#[test]
fn direct_coupling_moves_state_without_creating_global_energy() {
    let mut plan = two_body_plan(true);
    for body in &mut plan.bodies {
        body.space_port.coupling_weights = [0.0; crate::ir::SPACE_COURSES];
    }
    let mut world = World::from_plan(plan).unwrap();
    let initial_second = world.body_state(1).unwrap();
    let mut controls = vec![BodyControls::default(); 2];
    controls[0].source_wave = 0.8;
    world
        .process_sample(&controls, WorldControls::default())
        .unwrap();
    controls[0].source_wave = 0.0;
    for _ in 0..128 {
        world
            .process_sample(&controls, WorldControls::default())
            .unwrap();
    }
    assert_ne!(world.body_state(1).unwrap(), initial_second);
    assert!(world.coupling_last_exchange_into_left(0).is_some());
    assert!(world.receipt().global_closure_residual.abs() < 2.0e-9);
}

#[test]
fn world_digest_is_independent_of_any_observation_program() {
    let plan = two_body_plan(true);
    let a = World::from_plan(plan.clone()).unwrap();
    let b = World::from_plan(plan).unwrap();
    assert_eq!(a.plan_digest(), b.plan_digest());
    assert_eq!(a.total_stored_energy(), b.total_stored_energy());
}
