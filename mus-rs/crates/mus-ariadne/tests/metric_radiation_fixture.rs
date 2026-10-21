use mus_ariadne::{MetricRadiationAperture, MetricRadiationPlan};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ExpectedFrame {
    state_after: Vec<f64>,
    emitted: Vec<f64>,
    storage_before: f64,
    storage_after: f64,
    projected_intensity: f64,
    booked_radiation: f64,
    residual: f64,
}

#[derive(Debug, Deserialize)]
struct Fixture {
    plan: MetricRadiationPlan,
    plan_sha256: String,
    state: Vec<f64>,
    metric: Vec<f64>,
    loading: ExpectedFrame,
    nonloading: ExpectedFrame,
}

fn assert_close(left: f64, right: f64) {
    assert!(
        (left - right).abs() <= 1.0e-13,
        "left={left:?}, right={right:?}, difference={:?}",
        left - right
    );
}

fn assert_vector_close(left: &[f64], right: &[f64]) {
    assert_eq!(left.len(), right.len());
    for (left, right) in left.iter().zip(right.iter()) {
        assert_close(*left, *right);
    }
}

#[test]
fn python_and_rust_metric_radiation_replay_identically() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../../extension/experiments/token-radiation-v0/fixtures/radiation-cross-runtime.json"
    ))
    .unwrap();
    let aperture =
        MetricRadiationAperture::from_plan(fixture.plan.clone(), fixture.state.len()).unwrap();
    assert_eq!(aperture.digest(), fixture.plan_sha256);

    let mut loading_state = fixture.state.clone();
    let loading = aperture
        .radiate(&mut loading_state, &fixture.metric)
        .unwrap();
    assert_vector_close(&loading_state, &fixture.loading.state_after);
    assert_vector_close(&loading.emitted, &fixture.loading.emitted);
    assert_close(loading.storage_before, fixture.loading.storage_before);
    assert_close(loading.storage_after, fixture.loading.storage_after);
    assert_close(
        loading.projected_intensity,
        fixture.loading.projected_intensity,
    );
    assert_close(loading.booked_radiation, fixture.loading.booked_radiation);
    assert_close(loading.residual, fixture.loading.residual);

    let nonloading = aperture.observe(&fixture.state, &fixture.metric).unwrap();
    assert_vector_close(&fixture.state, &fixture.nonloading.state_after);
    assert_vector_close(&nonloading.emitted, &fixture.nonloading.emitted);
    assert_close(nonloading.storage_before, fixture.nonloading.storage_before);
    assert_close(nonloading.storage_after, fixture.nonloading.storage_after);
    assert_close(
        nonloading.projected_intensity,
        fixture.nonloading.projected_intensity,
    );
    assert_close(
        nonloading.booked_radiation,
        fixture.nonloading.booked_radiation,
    );
    assert_close(nonloading.residual, fixture.nonloading.residual);
}
