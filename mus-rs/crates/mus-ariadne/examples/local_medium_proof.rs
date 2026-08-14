use mus_ariadne::law::medium::{
    LocalConstitutiveMedium, LocalMediumEdgeSpec, LocalMediumInputFrame,
    LocalMediumObserverSpec, LocalMediumPlan, LocalMediumSiteSpec,
    LocalMediumSourceDrive, LocalMediumSourceSpec, LOCAL_MEDIUM_SCHEMA_VERSION,
};

fn site(id: &str) -> LocalMediumSiteSpec {
    LocalMediumSiteSpec {
        id: id.to_owned(),
        wave_metric: 1.0,
        wave_decay_per_second: 0.02,
        evidence_decay_per_second: 0.01,
        reservoir_capacity: 1.0,
        reservoir_leak_per_second: 0.0,
        active_rate_per_second: 0.0,
        active_threshold: 0.0,
        active_target_wave_energy: 0.0,
        initial_wave: 0.0,
        initial_evidence: 0.0,
        initial_reservoir: 0.0,
    }
}

fn plan() -> LocalMediumPlan {
    LocalMediumPlan {
        schema_version: LOCAL_MEDIUM_SCHEMA_VERSION.to_owned(),
        name: "local-medium-proof".to_owned(),
        sample_rate_hz: 32.0,
        sites: vec![site("north"), site("east"), site("south"), site("west")],
        edges: vec![
            LocalMediumEdgeSpec {
                id: "north-east".to_owned(),
                left: 0,
                right: 1,
                wave_rate_radians_per_second: 7.0,
                evidence_rate_per_second: 3.0,
                enabled: true,
            },
            LocalMediumEdgeSpec {
                id: "east-south".to_owned(),
                left: 1,
                right: 2,
                wave_rate_radians_per_second: -5.0,
                evidence_rate_per_second: 2.0,
                enabled: true,
            },
            LocalMediumEdgeSpec {
                id: "south-west".to_owned(),
                left: 2,
                right: 3,
                wave_rate_radians_per_second: 4.0,
                evidence_rate_per_second: 1.0,
                enabled: true,
            },
        ],
        sources: vec![
            LocalMediumSourceSpec {
                id: "gesture-a".to_owned(),
                site: 0,
                wave_gain: 1.0,
                evidence_gain: 0.0,
                work_gain: 0.0,
            },
            LocalMediumSourceSpec {
                id: "gesture-b".to_owned(),
                site: 1,
                wave_gain: 1.0,
                evidence_gain: 0.0,
                work_gain: 0.0,
            },
            LocalMediumSourceSpec {
                id: "evidence-zero".to_owned(),
                site: 2,
                wave_gain: 0.0,
                evidence_gain: 1.0,
                work_gain: 0.0,
            },
            LocalMediumSourceSpec {
                id: "evidence-one".to_owned(),
                site: 3,
                wave_gain: 0.0,
                evidence_gain: 1.0,
                work_gain: 0.0,
            },
        ],
        observers: vec![LocalMediumObserverSpec {
            id: "weak-observer".to_owned(),
            wave_weights: vec![1.0, -1.0, 0.5, -0.5],
            evidence_weights: vec![0.0, 0.0, -1.0, 1.0],
            reservoir_weights: vec![0.0; 4],
            bias: 0.0,
        }],
    }
}

fn run(order: [usize; 2]) -> (Vec<f64>, f64) {
    let mut medium = LocalConstitutiveMedium::from_plan(plan()).unwrap();
    for source in order {
        medium
            .step(&LocalMediumInputFrame {
                drives: vec![LocalMediumSourceDrive {
                    source,
                    wave: 0.8,
                    evidence: 0.0,
                    work_rate: 0.0,
                }],
            })
            .unwrap();
    }
    medium
        .step(&LocalMediumInputFrame {
            drives: vec![
                LocalMediumSourceDrive {
                    source: 0,
                    wave: 0.8,
                    evidence: 0.0,
                    work_rate: 0.0,
                },
                LocalMediumSourceDrive {
                    source: 3,
                    wave: 0.0,
                    evidence: 0.35,
                    work_rate: 0.0,
                },
            ],
        })
        .unwrap();
    (medium.state().wave.clone(), medium.observe()[0])
}

fn main() {
    let (ab, ab_observation) = run([0, 1]);
    let (ba, ba_observation) = run([1, 0]);
    let order_defect: f64 = ab
        .iter()
        .zip(&ba)
        .map(|(left, right)| (left - right).abs())
        .sum();
    println!(
        "{}",
        serde_json::json!({
            "format": "sophia.instrument.ariadne.local-medium-proof/v1",
            "plan_sha256": plan().digest().unwrap(),
            "ab_observation": ab_observation,
            "ba_observation": ba_observation,
            "order_defect_l1": order_defect,
            "ordered_transport": order_defect > 1.0e-6,
        })
    );
}
