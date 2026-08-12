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
