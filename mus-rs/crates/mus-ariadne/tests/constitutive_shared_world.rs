use mus_ariadne::law::medium::{
    LocalMediumEdgeSpec, LocalMediumInputFrame, LocalMediumObserverSpec, LocalMediumPlan,
    LocalMediumSiteSpec, LocalMediumSourceDrive, LocalMediumSourceSpec,
    LOCAL_MEDIUM_SCHEMA_VERSION,
};
use mus_ariadne::{
    BodyConstitutionSpec, BodyControls, BodyPose, ConstitutiveBodyControls,
    ConstitutiveBodyInstanceSpec, ConstitutiveWorldPlan, LocalMediumBodyControls,
    LocalMediumBoundarySpec, LocalMediumPortInputBinding, LocalMediumPortOutputBinding,
    SharedSpaceControls, SharedSpaceSpec, SpacePortSpec, World, WorldControls, MODE_COUNT,
    SPACE_COURSES, WORLD_PLAN_V3_SCHEMA_VERSION,
};

fn direction(index: usize) -> [f64; MODE_COUNT] {
    std::array::from_fn(|position| if position == index { 1.0 } else { 0.0 })
}

fn medium(name: &str) -> LocalMediumPlan {
    let site = |id: &str| LocalMediumSiteSpec {
        id: id.to_owned(),
        wave_metric: 1.0,
        wave_decay_per_second: 0.2,
        evidence_decay_per_second: 0.2,
        reservoir_capacity: 2.0,
        reservoir_leak_per_second: 0.0,
        active_rate_per_second: 0.0,
        active_threshold: 0.0,
        active_target_wave_energy: 0.0,
        initial_wave: 0.0,
        initial_evidence: 0.0,
        initial_reservoir: 0.0,
    };
    LocalMediumPlan {
        schema_version: LOCAL_MEDIUM_SCHEMA_VERSION.to_owned(),
        name: name.to_owned(),
        sample_rate_hz: 48_000.0,
        sites: vec![site("left"), site("right")],
        edges: vec![LocalMediumEdgeSpec {
            id: "internal".to_owned(),
            left: 0,
            right: 1,
            wave_rate_radians_per_second: 900.0,
            evidence_rate_per_second: 0.0,
            enabled: true,
        }],
        sources: vec![LocalMediumSourceSpec {
            id: "world-port".to_owned(),
            site: 0,
            wave_gain: 1.0,
            evidence_gain: 0.0,
            work_gain: 0.0,
        }],
        observers: Vec::new(),
    }
}

fn boundary(name: &str) -> LocalMediumBoundarySpec {
    LocalMediumBoundarySpec {
        medium: medium(name),
        inputs: vec![LocalMediumPortInputBinding {
            source: 0,
            boundary_direction: direction(0),
            wave_gain: 0.08,
            evidence_gain: 0.0,
        }],
        outputs: vec![LocalMediumPortOutputBinding {
            wave_weights: vec![1.0, -0.25],
            evidence_weights: vec![0.0, 0.0],
            reservoir_weights: vec![0.0, 0.0],
            boundary_direction: direction(0),
            gain_per_second: 12_000.0,
        }],
        boundary_reservoir_capacity: 32.0,
        initial_boundary_reservoir: 16.0,
        boundary_decay_per_second: 0.2,
        radiation_direction: direction(1),
    }
}

fn port(enabled: bool) -> SpacePortSpec {
    SpacePortSpec {
        body_directions: std::array::from_fn(direction),
        coupling_weights: if enabled {
            [8.0, 0.0, 0.0, 0.0]
        } else {
            [0.0; SPACE_COURSES]
        },
    }
}

fn plan(predictive_port_enabled: bool) -> ConstitutiveWorldPlan {
    ConstitutiveWorldPlan {
        schema_version: WORLD_PLAN_V3_SCHEMA_VERSION.to_owned(),
        world_name: "constitutive-shared-world-fixture".to_owned(),
        internal_sample_rate_hz: 48_000.0,
        bodies: vec![
            ConstitutiveBodyInstanceSpec {
                id: "lexical".to_owned(),
                constitution: BodyConstitutionSpec::LocalMedium {
                    boundary: boundary("lexical-medium"),
                },
                space_port: port(true),
                pose: BodyPose::default(),
            },
            ConstitutiveBodyInstanceSpec {
                id: "predictive".to_owned(),
                constitution: BodyConstitutionSpec::LocalMedium {
                    boundary: boundary("predictive-medium"),
                },
                space_port: port(predictive_port_enabled),
                pose: BodyPose {
                    position: [1.0, 0.0, 0.0],
                    ..BodyPose::default()
                },
            },
        ],
        shared_space: SharedSpaceSpec {
            delays_seconds: [0.000_25, 0.000_5, 0.000_75, 0.001],
            scattering_direction: [0.5; SPACE_COURSES],
            radiation_left: [1.0, 0.0, 0.0, 0.0],
            radiation_right: [0.0, 1.0, 0.0, 0.0],
        },
        direct_couplings: Vec::new(),
    }
}

fn run(predictive_port_enabled: bool) -> (f64, f64) {
    let mut world = World::from_constitutive_plan(plan(predictive_port_enabled)).unwrap();
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
    let world_controls = WorldControls {
        space: SharedSpaceControls {
            coupling: 1.0,
            decay: 0.1,
            radiation: 0.0,
        },
        ..WorldControls::default()
    };
    for frame in 0..1_024 {
        if let ConstitutiveBodyControls::LocalMedium(lexical) = &mut controls[0] {
            lexical.input = if frame < 4 {
                LocalMediumInputFrame {
                    drives: vec![LocalMediumSourceDrive {
                        source: 0,
                        wave: 0.25,
                        evidence: 0.0,
                        work_rate: 0.0,
                    }],
                }
            } else {
                LocalMediumInputFrame::default()
            };
        }
        world
            .process_constitutive_sample(&controls, world_controls)
            .unwrap();
    }
    let predictive_wave = world
        .body_local_medium_state(1)
        .unwrap()
        .wave
        .iter()
        .map(|value| value.abs())
        .sum::<f64>();
    (
        predictive_wave,
        world.receipt().global_closure_residual.abs(),
    )
}

#[test]
fn local_media_share_the_canonical_world_field_and_ledger() {
    let (connected, connected_residual) = run(true);
    let (isolated, isolated_residual) = run(false);
    assert!(connected > 1.0e-7, "connected={connected:e}");
    assert!(
        connected > isolated + 1.0e-7,
        "connected={connected:e} isolated={isolated:e}"
    );
    assert!(
        connected_residual < 1.0e-8,
        "residual={connected_residual:e}"
    );
    assert!(isolated_residual < 1.0e-8, "residual={isolated_residual:e}");
}

#[test]
fn constitutive_world_plan_is_content_addressed() {
    let plan = plan(true);
    plan.validate().unwrap();
    assert_eq!(plan.digest().unwrap(), plan.digest().unwrap());
}

#[test]
fn embedded_local_observers_are_refused_from_physical_world_identity() {
    let mut plan = plan(true);
    let BodyConstitutionSpec::LocalMedium { boundary } = &mut plan.bodies[0].constitution else {
        panic!("fixture body must be a local medium");
    };
    boundary.medium.observers.push(LocalMediumObserverSpec {
        id: "forbidden-observer".to_owned(),
        wave_weights: vec![1.0, 0.0],
        evidence_weights: vec![0.0, 0.0],
        reservoir_weights: vec![0.0, 0.0],
        bias: 0.0,
    });
    let error = plan.validate().unwrap_err();
    assert_eq!(error.code, "embedded-local-medium-observer");
}

#[test]
fn modal_and_local_media_share_one_world_executor() {
    let template = mus_ariadne::WorldPlan::string_brass_v1(48_000.0);
    let plan = ConstitutiveWorldPlan {
        schema_version: WORLD_PLAN_V3_SCHEMA_VERSION.to_owned(),
        world_name: "mixed-constitution-fixture".to_owned(),
        internal_sample_rate_hz: 48_000.0,
        bodies: vec![
            ConstitutiveBodyInstanceSpec {
                id: "modal".to_owned(),
                constitution: BodyConstitutionSpec::Modal {
                    body: template.bodies[0].body.clone(),
                },
                space_port: template.bodies[0].space_port,
                pose: BodyPose::default(),
            },
            ConstitutiveBodyInstanceSpec {
                id: "local".to_owned(),
                constitution: BodyConstitutionSpec::LocalMedium {
                    boundary: boundary("mixed-local-medium"),
                },
                space_port: port(false),
                pose: BodyPose::default(),
            },
        ],
        shared_space: template.shared_space,
        direct_couplings: Vec::new(),
    };
    let mut world = World::from_constitutive_plan(plan).unwrap();
    let controls = vec![
        ConstitutiveBodyControls::Modal(BodyControls::default()),
        ConstitutiveBodyControls::LocalMedium(LocalMediumBodyControls {
            radiation_fraction: 0.0,
            ..LocalMediumBodyControls::default()
        }),
    ];
    for _ in 0..64 {
        world
            .process_constitutive_sample(&controls, WorldControls::default())
            .unwrap();
    }
    assert_eq!(world.body_count(), 2);
    assert!(world.body_state(0).is_some());
    assert!(world.body_local_medium_state(1).is_some());
    assert!(world.receipt().global_closure_residual.abs() < 1.0e-8);
}

#[test]
fn modal_v2_upgrade_preserves_construction_without_reusing_v2_identity() {
    let legacy = mus_ariadne::WorldPlan::string_brass_v1(48_000.0);
    let legacy_digest = legacy.digest().unwrap();
    let upgraded = ConstitutiveWorldPlan::from_modal_v2(legacy).unwrap();
    upgraded.validate().unwrap();
    assert_eq!(upgraded.schema_version, WORLD_PLAN_V3_SCHEMA_VERSION);
    assert_ne!(upgraded.digest().unwrap(), legacy_digest);
    let world = World::from_constitutive_plan(upgraded).unwrap();
    assert_eq!(world.body_count(), 1);
}
