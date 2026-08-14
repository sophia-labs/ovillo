use crate::body::BodyControls;
use crate::ir::WorldSpec;
use crate::plan::{
    BodyInstanceSpec, DirectCouplingSpec, ObserverBodyTap, ObserverSpec,
    WorldPlan,
};

use super::*;

fn two_body_plan(coupled: bool) -> WorldPlan {
    let mut plan =
        WorldPlan::from_single_body(WorldSpec::string_brass_v1());
    plan.world_name = "two-body-fixture".to_owned();
    let mut second: BodyInstanceSpec = plan.bodies[0].clone();
    second.id = "body/1".to_owned();
    second.pose.position = [1.0, 0.0, 0.0];
    plan.bodies.push(second);
    plan.observers[0].body_taps.push(ObserverBodyTap {
        body_id: "body/1".to_owned(),
        left_projection: 0.7,
        right_projection: 1.0,
    });
    if coupled {
        let direction = std::array::from_fn(|index| {
            if index == 0 { 1.0 } else { 0.0 }
        });
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
fn zero_shared_world_is_an_exact_fixed_point() {
    let mut world = SharedWorld::from_plan(two_body_plan(false)).unwrap();
    let controls = vec![BodyControls::default(); 2];
    for _ in 0..4_096 {
        let frame = world
            .process_sample(&controls, SharedWorldControls::default())
            .unwrap();
        assert_eq!(frame.left, 0.0);
        assert_eq!(frame.right, 0.0);
    }
    assert!(world.receipt().global_closure_residual.abs() < 1.0e-14);
}

#[test]
fn body_instances_have_independent_mutable_state() {
    let mut plan = two_body_plan(false);
    plan.bodies[1].space_port.coupling_weights =
        [0.0; crate::ir::SPACE_COURSES];
    let mut world = SharedWorld::from_plan(plan).unwrap();
    let initial_second = world.body_state(1).unwrap();
    let mut controls = vec![BodyControls::default(); 2];
    controls[0].source_wave = 0.8;
    world
        .process_sample(&controls, SharedWorldControls::default())
        .unwrap();
    assert_ne!(world.body_state(0).unwrap(), initial_second);
    assert_eq!(world.body_state(1).unwrap(), initial_second);
}

#[test]
fn direct_coupling_moves_state_without_creating_global_energy() {
    let mut plan = two_body_plan(true);
    for body in &mut plan.bodies {
        body.space_port.coupling_weights =
            [0.0; crate::ir::SPACE_COURSES];
    }
    let mut world = SharedWorld::from_plan(plan).unwrap();
    let initial_second = world.body_state(1).unwrap();
    let mut controls = vec![BodyControls::default(); 2];
    controls[0].source_wave = 0.8;
    world
        .process_sample(&controls, SharedWorldControls::default())
        .unwrap();
    controls[0].source_wave = 0.0;
    for _ in 0..128 {
        world
            .process_sample(&controls, SharedWorldControls::default())
            .unwrap();
    }
    assert_ne!(world.body_state(1).unwrap(), initial_second);
    assert!(world.receipt().global_closure_residual.abs() < 2.0e-9);
}

#[test]
fn adding_a_nonloading_observer_does_not_change_physical_state() {
    let plan = two_body_plan(true);
    let mut observed_plan = plan.clone();
    observed_plan.observers.push(ObserverSpec {
        id: "observer/extra".to_owned(),
        body_taps: vec![ObserverBodyTap {
            body_id: "body/0".to_owned(),
            left_projection: -0.4,
            right_projection: 0.2,
        }],
        space_left_projection: 0.3,
        space_right_projection: -0.2,
        output_scale: 5.0,
    });
    let mut a = SharedWorld::from_plan(plan).unwrap();
    let mut b = SharedWorld::from_plan(observed_plan).unwrap();
    let mut controls = vec![BodyControls::default(); 2];
    for sample in 0..2_048 {
        controls[0].source_wave = if sample == 0 { 0.7 } else { 0.0 };
        let fa = a
            .process_sample(&controls, SharedWorldControls::default())
            .unwrap();
        let fb = b
            .process_sample(&controls, SharedWorldControls::default())
            .unwrap();
        assert_eq!(fa.source_work, fb.source_work);
        assert_eq!(fa.radiation, fb.radiation);
    }
    for index in 0..2 {
        assert_eq!(a.body_state(index), b.body_state(index));
    }
    let ar = a.receipt();
    let br = b.receipt();
    assert_eq!(ar.source_work, br.source_work);
    assert_eq!(ar.internal_loss, br.internal_loss);
    assert_eq!(ar.radiation, br.radiation);
    assert_eq!(ar.current_energy, br.current_energy);
}
