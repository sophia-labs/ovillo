use mus_ariadne::law::medium::{LocalConstitutiveMedium, LocalMediumInputFrame, LocalMediumPlan};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Fixture {
    plan: LocalMediumPlan,
    plan_sha256: String,
    steps: Vec<FixtureStep>,
    expected: Expected,
}

#[derive(Debug, Deserialize)]
struct FixtureStep {
    #[serde(default)]
    drives: Vec<mus_ariadne::law::medium::LocalMediumSourceDrive>,
    #[serde(default)]
    edge_enabled: Option<EdgeToggle>,
}

#[derive(Debug, Deserialize)]
struct EdgeToggle {
    edge: usize,
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct Expected {
    state: ExpectedState,
    stored_energy: f64,
    positive_mass: f64,
    observation: Vec<f64>,
    cumulative_source_work: f64,
    cumulative_internal_loss: f64,
    maximum_transport_residual: f64,
    maximum_positive_mass_residual: f64,
    maximum_closure_residual: f64,
}

#[derive(Debug, Deserialize)]
struct ExpectedState {
    wave: Vec<f64>,
    evidence: Vec<f64>,
    reservoir: Vec<f64>,
}

fn close(left: f64, right: f64, tolerance: f64) {
    assert!(
        (left - right).abs() <= tolerance,
        "{left:.17e} != {right:.17e} within {tolerance:.1e}"
    );
}

#[test]
fn python_and_rust_replay_the_same_compiled_constitution() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../../extension/experiments/generation-3/fixtures/cross-runtime.json"
    ))
    .unwrap();
    assert_eq!(fixture.plan.digest().unwrap(), fixture.plan_sha256);
    let mut medium = LocalConstitutiveMedium::from_plan(fixture.plan).unwrap();
    let mut cumulative_source_work = 0.0;
    let mut cumulative_internal_loss = 0.0;
    let mut maximum_transport_residual = 0.0_f64;
    let mut maximum_positive_mass_residual = 0.0_f64;
    let mut maximum_closure_residual = 0.0_f64;

    for step in fixture.steps {
        if let Some(toggle) = step.edge_enabled {
            medium
                .set_edge_enabled(toggle.edge, toggle.enabled)
                .unwrap();
        }
        let receipt = medium
            .step(&LocalMediumInputFrame {
                drives: step.drives,
            })
            .unwrap();
        cumulative_source_work += receipt.source_work;
        cumulative_internal_loss += receipt.internal_loss;
        maximum_transport_residual = maximum_transport_residual.max(receipt.transport_residual);
        maximum_positive_mass_residual =
            maximum_positive_mass_residual.max(receipt.positive_mass_residual);
        maximum_closure_residual = maximum_closure_residual.max(receipt.closure_residual.abs());
    }

    for (actual, expected) in medium.state().wave.iter().zip(&fixture.expected.state.wave) {
        close(*actual, *expected, 2.0e-12);
    }
    for (actual, expected) in medium
        .state()
        .evidence
        .iter()
        .zip(&fixture.expected.state.evidence)
    {
        close(*actual, *expected, 2.0e-12);
    }
    for (actual, expected) in medium
        .state()
        .reservoir
        .iter()
        .zip(&fixture.expected.state.reservoir)
    {
        close(*actual, *expected, 2.0e-12);
    }
    close(
        medium.stored_energy(),
        fixture.expected.stored_energy,
        2.0e-11,
    );
    close(
        medium.positive_mass(),
        fixture.expected.positive_mass,
        2.0e-12,
    );
    for (actual, expected) in medium.observe().iter().zip(&fixture.expected.observation) {
        close(*actual, *expected, 2.0e-11);
    }
    close(
        cumulative_source_work,
        fixture.expected.cumulative_source_work,
        2.0e-11,
    );
    close(
        cumulative_internal_loss,
        fixture.expected.cumulative_internal_loss,
        2.0e-11,
    );
    close(
        maximum_transport_residual,
        fixture.expected.maximum_transport_residual,
        2.0e-12,
    );
    close(
        maximum_positive_mass_residual,
        fixture.expected.maximum_positive_mass_residual,
        2.0e-12,
    );
    close(
        maximum_closure_residual,
        fixture.expected.maximum_closure_residual,
        2.0e-12,
    );
}
