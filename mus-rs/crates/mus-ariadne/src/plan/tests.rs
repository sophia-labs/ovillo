use super::*;
use crate::ir::WorldSpec;

#[test]
fn one_body_embedding_is_valid_and_content_addressed() {
    let plan = WorldPlan::from_single_body(WorldSpec::string_brass_v1());
    plan.validate().unwrap();
    assert_eq!(plan.bodies.len(), 1);
    assert_eq!(plan.digest().unwrap(), plan.digest().unwrap());
}

#[test]
fn duplicate_body_and_unknown_observer_body_refuse() {
    let mut plan = WorldPlan::from_single_body(WorldSpec::string_brass_v1());
    plan.bodies.push(plan.bodies[0].clone());
    assert_eq!(plan.validate().unwrap_err().code, "duplicate-body-id");

    let mut plan = WorldPlan::from_single_body(WorldSpec::string_brass_v1());
    plan.observers[0].body_taps[0].body_id = "missing".to_owned();
    assert_eq!(plan.validate().unwrap_err().code, "unknown-observer-body");
}
