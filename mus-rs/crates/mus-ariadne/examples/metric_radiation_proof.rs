use mus_ariadne::{
    MetricRadiationAperture, MetricRadiationPlan, METRIC_RADIATION_SCHEMA_VERSION,
};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plan = MetricRadiationPlan {
        schema_version: METRIC_RADIATION_SCHEMA_VERSION.to_owned(),
        id: "metric-radiation-proof".to_owned(),
        fraction: 0.25,
        rows: vec![
            vec![1.0, 0.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0, 0.0],
        ],
    };
    let aperture = MetricRadiationAperture::from_plan(plan, 4)?;
    let metric = [0.75, 1.25, 2.0, 3.0];
    let initial = [0.8, -0.4, 0.7, 1.1];

    let observed = aperture.observe(&initial, &metric)?;
    let mut loaded_state = initial;
    let radiated = aperture.radiate(&mut loaded_state, &metric)?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "format": "sophia.instrument.ariadne.metric-radiation-proof/v1",
            "aperture_digest": aperture.digest(),
            "initial_state": initial,
            "metric": metric,
            "nonloading": observed,
            "passive": radiated,
            "passive_state_after": loaded_state,
        }))?
    );
    Ok(())
}
