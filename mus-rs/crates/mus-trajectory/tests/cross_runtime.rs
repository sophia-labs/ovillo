use mus_trajectory::{TrajectoryProgram, TrajectoryReceipt};

#[test]
fn python_fixture_replays_with_the_same_digest_and_values() {
    let program: TrajectoryProgram = serde_json::from_str(include_str!(
        "../../../SPECS/trajectory/fixtures/weave-rise.json"
    ))
    .unwrap();
    let expected: TrajectoryReceipt = serde_json::from_str(include_str!(
        "../../../SPECS/trajectory/fixtures/weave-rise.receipt.json"
    ))
    .unwrap();

    let actual = program.receipt().unwrap();
    assert_eq!(actual, expected);
    let mut values = [0.0; 2];
    program.write_values(0, &mut values).unwrap();
    assert_eq!(values, [0.04, 0.25]);
    program.write_values(23_999, &mut values).unwrap();
    assert_eq!(values, [0.12, 0.72]);
    assert!(program.lanes[0].value_at(12_000) > 0.399_999);
}

#[test]
fn learned_extension_loop_replays_through_the_same_rust_contract() {
    let program: TrajectoryProgram = serde_json::from_str(include_str!(
        "../../../experiments/extension_v1/artifacts/token0.trajectory.json"
    ))
    .unwrap();
    let expected: TrajectoryReceipt = serde_json::from_str(include_str!(
        "../../../experiments/extension_v1/artifacts/token0.trajectory.receipt.json"
    ))
    .unwrap();

    assert_eq!(program.receipt().unwrap(), expected);
    assert_eq!(program.lanes.len(), 4);
    for lane in &program.lanes {
        assert_eq!(lane.knots.first().unwrap().value, 0.0);
        assert_eq!(lane.knots.last().unwrap().value, 0.0);
        assert!(lane.knots[1..lane.knots.len() - 1]
            .iter()
            .any(|knot| knot.value != 0.0));
    }
}
