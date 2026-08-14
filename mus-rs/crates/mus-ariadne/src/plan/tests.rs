use super::*;

#[test]
fn canonical_plan_is_valid_and_content_addressed() {
    let plan = WorldPlan::string_brass_v1(96_000.0);
    plan.validate().unwrap();
    assert_eq!(plan.bodies.len(), 1);
    assert_eq!(plan.digest().unwrap(), plan.digest().unwrap());
}

#[test]
fn duplicate_body_and_unknown_coupling_endpoint_refuse() {
    let mut plan = WorldPlan::string_brass_v1(96_000.0);
    plan.bodies.push(plan.bodies[0].clone());
    assert_eq!(plan.validate().unwrap_err().code, "duplicate-body-id");

    let mut plan = WorldPlan::string_brass_v1(96_000.0);
    let direction = std::array::from_fn(|index| if index == 0 { 1.0 } else { 0.0 });
    plan.direct_couplings.push(DirectCouplingSpec {
        id: "edge/missing".to_owned(),
        left_body: "body/0".to_owned(),
        right_body: "body/missing".to_owned(),
        left_direction: direction,
        right_direction: direction,
        rate_hz: 20.0,
        weight: 1.0,
    });
    assert_eq!(
        plan.validate().unwrap_err().code,
        "invalid-coupling-endpoint"
    );
}

#[test]
fn observation_is_not_part_of_physical_plan_serialization() {
    let serialized = serde_json::to_string(&WorldPlan::string_brass_v1(48_000.0)).unwrap();
    assert!(!serialized.contains("observer"));
    assert!(!serialized.contains("palette"));
    assert!(!serialized.contains("camera"));
}
