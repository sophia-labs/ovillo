use mus_ariadne::law::medium::{
    LocalMediumInputFrame, LocalMediumPlan, LocalMediumSiteSpec, LocalMediumSourceDrive,
    LocalMediumSourceSpec, LOCAL_MEDIUM_SCHEMA_VERSION,
};
use mus_ariadne::{
    BodyConstitutionSpec, BodyPose, ConstitutiveBodyControls, ConstitutiveBodyInstanceSpec,
    ConstitutiveWorldPlan, LocalMediumBodyControls, LocalMediumBoundarySpec,
    LocalMediumPortInputBinding, LocalMediumPortOutputBinding, SharedSpaceControls, SpacePortSpec,
    World, WorldControls, WorldPlan, MODE_COUNT, WORLD_PLAN_V3_SCHEMA_VERSION,
};

fn direction(index: usize) -> [f64; MODE_COUNT] {
    std::array::from_fn(|position| if position == index { 1.0 } else { 0.0 })
}

fn body(name: &str) -> LocalMediumBoundarySpec {
    let sites = vec![LocalMediumSiteSpec {
        id: "state".to_owned(),
        wave_metric: 1.0,
        wave_decay_per_second: 0.1,
        evidence_decay_per_second: 0.1,
        reservoir_capacity: 1.0,
        reservoir_leak_per_second: 0.0,
        active_rate_per_second: 0.0,
        active_threshold: 0.0,
        active_target_wave_energy: 0.0,
        initial_wave: 0.0,
        initial_evidence: 0.0,
        initial_reservoir: 0.0,
    }];
    LocalMediumBoundarySpec {
        medium: LocalMediumPlan {
            schema_version: LOCAL_MEDIUM_SCHEMA_VERSION.to_owned(),
            name: name.to_owned(),
            sample_rate_hz: 48_000.0,
            sites,
            edges: Vec::new(),
            sources: vec![LocalMediumSourceSpec {
                id: "port".to_owned(),
                site: 0,
                wave_gain: 1.0,
                evidence_gain: 0.0,
                work_gain: 0.0,
            }],
            observers: Vec::new(),
        },
        inputs: vec![LocalMediumPortInputBinding {
            source: 0,
            boundary_direction: direction(0),
            wave_gain: 0.1,
            evidence_gain: 0.0,
        }],
        outputs: vec![LocalMediumPortOutputBinding {
            wave_weights: vec![1.0],
            evidence_weights: vec![0.0],
            reservoir_weights: vec![0.0],
            boundary_direction: direction(0),
            gain_per_second: 10_000.0,
        }],
        boundary_reservoir_capacity: 16.0,
        initial_boundary_reservoir: 8.0,
        boundary_decay_per_second: 0.1,
        radiation_direction: direction(1),
    }
}

fn main() {
    let template = WorldPlan::string_brass_v1(48_000.0);
    let mut port: SpacePortSpec = template.bodies[0].space_port;
    port.coupling_weights = [8.0, 0.0, 0.0, 0.0];
    port.body_directions = std::array::from_fn(direction);
    let plan = ConstitutiveWorldPlan {
        schema_version: WORLD_PLAN_V3_SCHEMA_VERSION.to_owned(),
        world_name: "constitutive-shared-world-proof".to_owned(),
        internal_sample_rate_hz: 48_000.0,
        bodies: vec![
            ConstitutiveBodyInstanceSpec {
                id: "lexical".to_owned(),
                constitution: BodyConstitutionSpec::LocalMedium {
                    boundary: body("lexical"),
                },
                space_port: port,
                pose: BodyPose::default(),
            },
            ConstitutiveBodyInstanceSpec {
                id: "predictive".to_owned(),
                constitution: BodyConstitutionSpec::LocalMedium {
                    boundary: body("predictive"),
                },
                space_port: port,
                pose: BodyPose::default(),
            },
        ],
        shared_space: template.shared_space,
        direct_couplings: Vec::new(),
    };
    let digest = plan.digest().unwrap();
    let mut world = World::from_constitutive_plan(plan).unwrap();
    let mut controls = vec![
        ConstitutiveBodyControls::LocalMedium(LocalMediumBodyControls {
            radiation_fraction: 0.0,
            ..LocalMediumBodyControls::default()
        }),
        ConstitutiveBodyControls::LocalMedium(LocalMediumBodyControls {
            radiation_fraction: 0.0,
            ..LocalMediumBodyControls::default()
        }),
    ];
    for frame in 0..4_096 {
        if let ConstitutiveBodyControls::LocalMedium(lexical) = &mut controls[0] {
            lexical.input = if frame < 8 {
                LocalMediumInputFrame {
                    drives: vec![LocalMediumSourceDrive {
                        source: 0,
                        wave: 0.2,
                        evidence: 0.0,
                        work_rate: 0.0,
                    }],
                }
            } else {
                LocalMediumInputFrame::default()
            };
        }
        world
            .process_constitutive_sample(
                &controls,
                WorldControls {
                    space: SharedSpaceControls {
                        coupling: 1.0,
                        decay: 0.1,
                        radiation: 0.0,
                    },
                    ..WorldControls::default()
                },
            )
            .unwrap();
    }
    let predictive = world.body_local_medium_state(1).unwrap();
    let receipt = world.receipt();
    println!("plan_digest={digest}");
    println!("predictive_wave={:?}", predictive.wave);
    println!("world_receipt={receipt:?}");
}
