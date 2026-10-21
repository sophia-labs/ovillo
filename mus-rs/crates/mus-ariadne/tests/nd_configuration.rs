//! N-D configuration spaces: backward compatibility, ledger closure along
//! random N-D walks, honest billing of closed loops, and a computed (law)
//! configuration against the legacy interpolated (fit) one.
//!
//! Real runtime, no mocks: every test builds `mus_ariadne::World`s from plans.

use mus_ariadne::{
    AffineSpec, AreaTermSpec, BodyConstitutionSpec, BodyControls, ConfigurationCoordinate,
    ConfigurationMapSpec, ConfigurationSpaceSpec, ConnectionPolicy, ConstitutiveBodyControls,
    ConstitutiveWorldPlan, DisposalPolicy, FlowValveControls, FlowValveSpec, RadiatingEndSpec,
    ReservoirSpec, SpaceBodyControls, SpaceBodyLaws, SpaceBodySpec, StiffStringSpec,
    TubeBranchSpec, TubeNetworkSpec, TubeSpec, World, WorldConfiguration, WorldControls, WorldPlan,
    MODE_COUNT,
};

fn repo_file(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(relative)
}

fn steel_string_plan() -> WorldPlan {
    let text =
        std::fs::read_to_string(repo_file("profiles/ariadne-steel-string-v0.plan.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn steel_string_receipt() -> serde_json::Value {
    let text = std::fs::read_to_string(repo_file("profiles/ariadne-steel-string-v0.receipt.json"))
        .unwrap();
    serde_json::from_str(&text).unwrap()
}

/// A 1-D two-anchor space carrying exactly a legacy body's path and laws.
fn one_dimensional_twin(body: &mus_ariadne::BodySpec) -> SpaceBodySpec {
    let path = &body.configuration_path;
    SpaceBodySpec {
        anatomy_name: body.anatomy_name.clone(),
        configuration_space: ConfigurationSpaceSpec {
            coordinates: vec![ConfigurationCoordinate {
                name: path.coordinate.clone(),
                lower: 0.0,
                upper: 1.0,
                initial: path.initial_amount,
            }],
            map: ConfigurationMapSpec::Multilinear {
                axes: vec![vec![0.0, 1.0]],
                anchors: vec![path.left, path.right],
            },
            connection: ConnectionPolicy::ModalCoordinates,
        },
        laws: SpaceBodyLaws::Modal {
            source_direction: body.source_direction,
            active_directions: body.active_directions,
            transport: body.transport,
        },
        reservoir: body.reservoir,
    }
}

fn pluck(sample: usize) -> f64 {
    // A raised-cosine pulse every 9000 samples, 40 samples long.
    let phase = sample % 9_000;
    if phase < 40 {
        0.6 * (0.5 - 0.5 * (std::f64::consts::TAU * phase as f64 / 40.0).cos())
    } else {
        0.0
    }
}

/// G-A1 (runtime level): the steel-string world with string/6 replaced by its
/// 1-D N-D twin renders bitwise the same samples, states and ledger as the
/// legacy body, through plucks and fret moves.
#[test]
fn one_dimensional_space_body_is_bitwise_the_legacy_body() {
    let v2 = steel_string_plan();
    let legacy_plan = ConstitutiveWorldPlan::from_modal_v2(v2.clone()).unwrap();
    let mut twin_plan = legacy_plan.clone();
    let BodyConstitutionSpec::Modal { body } = &twin_plan.bodies[0].constitution else {
        panic!()
    };
    let twin = one_dimensional_twin(body);
    twin_plan.bodies[0].constitution = BodyConstitutionSpec::ModalSpace { body: twin };
    let mut legacy = World::from_constitutive_plan(legacy_plan).unwrap();
    let mut nd = World::from_constitutive_plan(twin_plan).unwrap();
    let bodies = legacy.body_count();
    let frets = [0.0, 2.0, 2.0, 5.0, 7.0, 4.5, 0.0, 12.0, 3.0];
    for sample in 0..60_000usize {
        if sample % 6_000 == 0 {
            let amount = frets[(sample / 6_000) % frets.len()] / 12.0;
            let a = legacy.set_body_configuration_index(0, amount).unwrap();
            let b = nd.set_body_configuration_point(0, &[amount]).unwrap();
            assert_eq!(
                a.total_control_work.to_bits(),
                b.control_work.to_bits(),
                "sample {sample}"
            );
        }
        let mut legacy_controls =
            vec![ConstitutiveBodyControls::Modal(BodyControls::default()); bodies];
        let mut nd_controls = legacy_controls.clone();
        let source = pluck(sample);
        legacy_controls[0] = ConstitutiveBodyControls::Modal(BodyControls {
            source_wave: source,
            radiation_fraction: 2.0e-6,
            ..Default::default()
        });
        nd_controls[0] = ConstitutiveBodyControls::ModalSpace(SpaceBodyControls {
            body: BodyControls {
                source_wave: source,
                radiation_fraction: 2.0e-6,
                ..Default::default()
            },
            ..Default::default()
        });
        let fa = legacy
            .process_constitutive_sample(&legacy_controls, WorldControls::default())
            .unwrap();
        let fb = nd
            .process_constitutive_sample(&nd_controls, WorldControls::default())
            .unwrap();
        assert_eq!(fa, fb, "frame differs at sample {sample}");
        for index in 0..bodies {
            assert_eq!(
                legacy.body_radiation(index).unwrap(),
                nd.body_radiation(index).unwrap()
            );
        }
    }
    for index in 0..bodies {
        assert_eq!(legacy.body_state(index), nd.body_state(index));
    }
    let (ra, rb) = (legacy.receipt(), nd.receipt());
    assert_eq!(
        ra.cumulative_closure_residual.to_bits(),
        rb.cumulative_closure_residual.to_bits()
    );
    assert_eq!(ra.control_work.to_bits(), rb.control_work.to_bits());
    assert_eq!(ra.radiation.to_bits(), rb.radiation.to_bits());
}

/// Coordinator's audit (2026-10-20): the legacy fret path interpolates the
/// partials in log space between open and twelfth fret; a stiff string's
/// partials follow `n f0 sqrt(1 + B n^2)` with `B ~ 1/L^2`. The computed map
/// reproduces the plan's two anchors and shows how far the interpolation is
/// from the law in between.
#[test]
fn stiff_string_law_reproduces_the_plan_anchors_and_measures_the_interpolation_fit() {
    let plan = steel_string_plan();
    let receipt = steel_string_receipt();
    let mut worst = 0.0_f64;
    for (index, body) in plan.bodies.iter().take(6).enumerate() {
        let entry = &receipt["strings"][index];
        assert_eq!(entry["string"].as_u64().unwrap() as usize, 6 - index);
        let b0 = entry["inharmonicity_b"].as_f64().unwrap();
        let palm = entry["palm_mute"].as_f64().unwrap();
        let path = &body.body.configuration_path;
        let f0 = path.left.metric.frequency_hz[0] / (1.0 + b0).sqrt();
        let law = StiffStringSpec {
            open_fundamental_hz: f0,
            open_inharmonicity: b0,
            fret_coordinate: 0,
            tension_coordinate: None,
            // mus_analysis/steel_string_anatomy.py string_partials + palm term.
            decay_constant_per_s: 0.55 + palm * 24.0,
            decay_quadratic_s: 1.9e-7 + palm * 1.2e-5,
            t60_numerator: 6.91,
            aperture: path.left.aperture,
            radiation_direction: path.left.radiation_direction,
        };
        for (frets, anchor) in [(0.0, path.left), (12.0, path.right)] {
            let computed = law.configuration_at(&[frets]);
            for n in 0..MODE_COUNT {
                let rel = computed.metric.frequency_hz[n] / anchor.metric.frequency_hz[n] - 1.0;
                assert!(
                    rel.abs() < 1e-12,
                    "string {} fret {frets} partial {}: {rel:e}",
                    6 - index,
                    n + 1
                );
                let rel_t60 = computed.metric.t60_seconds[n] / anchor.metric.t60_seconds[n] - 1.0;
                assert!(
                    rel_t60.abs() < 1e-9,
                    "t60 string {} partial {}: {rel_t60:e}",
                    6 - index,
                    n + 1
                );
            }
        }
        let mut row = Vec::new();
        for fret in 1..12 {
            let legacy = path.configuration_at(fret as f64 / 12.0);
            let computed = law.configuration_at(&[fret as f64]);
            let cents: Vec<f64> = (0..MODE_COUNT)
                .map(|n| {
                    1200.0
                        * (legacy.metric.frequency_hz[n] / computed.metric.frequency_hz[n]).log2()
                })
                .collect();
            worst = worst.max(cents.iter().fold(0.0_f64, |m, c| m.max(c.abs())));
            row.push((fret, cents[0], cents[11], cents[23]));
        }
        println!(
            "string {}: (fret, cents sharp of the law at partial 1, 12, 24) {:?}",
            6 - index,
            row.iter()
                .map(|(f, a, b, c)| (
                    *f,
                    (a * 100.0).round() / 100.0,
                    (b * 10.0).round() / 10.0,
                    (c * 10.0).round() / 10.0
                ))
                .collect::<Vec<_>>()
        );
        if index == 0 {
            let fret5 = row[4];
            // Closed form for B0 = 2.193e-4 at fret 5: 11.2 and 31.8 cents.
            assert!(
                (fret5.2 - 11.2).abs() < 0.3,
                "low E fret 5 partial 12: {}",
                fret5.2
            );
            assert!(
                (fret5.3 - 31.8).abs() < 0.5,
                "low E fret 5 partial 24: {}",
                fret5.3
            );
            assert!(
                fret5.1.abs() < 0.1,
                "fundamental stays within 0.1 cent: {}",
                fret5.1
            );
        }
    }
    println!("worst interpolation error over strings, frets 1-11, partials 1-24: {worst:.2} cents");
}

fn multilinear_4d_plan(sample_rate: f64) -> ConstitutiveWorldPlan {
    let mut plan =
        ConstitutiveWorldPlan::from_modal_v2(WorldPlan::string_brass_v1(sample_rate)).unwrap();
    let BodyConstitutionSpec::Modal { body } = &plan.bodies[0].constitution else {
        panic!()
    };
    let body = body.clone();
    let (left, right) = (body.configuration_path.left, body.configuration_path.right);
    // 16 anchors: corner c gets a configuration between the two ends, detuned
    // per corner so no two anchors agree.
    let anchors: Vec<WorldConfiguration> = (0..16)
        .map(|corner| {
            let amount = (corner as f64 + 0.5) / 16.0;
            let mut c = mus_ariadne::ConfigurationPath {
                coordinate: "x".into(),
                left_name: "l".into(),
                right_name: "r".into(),
                left,
                right,
                initial_amount: 0.0,
            }
            .configuration_at(amount);
            for f in &mut c.metric.frequency_hz {
                *f *= 1.0 + 0.013 * ((corner * 7 % 5) as f64 - 2.0);
            }
            c
        })
        .collect();
    let coordinates = ["a", "b", "c", "d"]
        .iter()
        .map(|name| ConfigurationCoordinate {
            name: (*name).into(),
            lower: 0.0,
            upper: 1.0,
            initial: 0.5,
        })
        .collect();
    let space = SpaceBodySpec {
        anatomy_name: "four-d-multilinear".into(),
        configuration_space: ConfigurationSpaceSpec {
            coordinates,
            map: ConfigurationMapSpec::Multilinear {
                axes: vec![vec![0.0, 1.0]; 4],
                anchors,
            },
            connection: ConnectionPolicy::ModalCoordinates,
        },
        laws: SpaceBodyLaws::Modal {
            source_direction: body.source_direction,
            active_directions: body.active_directions,
            transport: body.transport,
        },
        reservoir: body.reservoir,
    };
    plan.bodies[0].constitution = BodyConstitutionSpec::ModalSpace { body: space };
    plan
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// G-A2: random walks in a 4-D space keep the ledger closed to roundoff.
#[test]
fn random_walk_in_a_four_dimensional_multilinear_space_closes_the_ledger() {
    let mut world = World::from_constitutive_plan(multilinear_4d_plan(48_000.0)).unwrap();
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    let mut point = [0.5; 4];
    let mut energy_scale = 0.0_f64;
    for sample in 0..96_000usize {
        if sample % 16 == 0 {
            for x in &mut point {
                *x = (*x + 0.08 * (rng.next() - 0.5)).clamp(0.0, 1.0);
            }
            world.set_body_configuration_point(0, &point).unwrap();
        }
        let controls = [ConstitutiveBodyControls::ModalSpace(SpaceBodyControls {
            body: BodyControls {
                source_wave: pluck(sample),
                ..Default::default()
            },
            ..Default::default()
        })];
        world
            .process_constitutive_sample(&controls, WorldControls::default())
            .unwrap();
        energy_scale = energy_scale.max(world.total_stored_energy());
    }
    let receipt = world.receipt();
    println!(
        "4-D multilinear walk: 6000 moves, |control work| {:.3e} J, energy scale {:.3e} J, cumulative residual {:.3e} J, max frame residual {:.3e} J",
        receipt.control_work.abs(),
        energy_scale,
        receipt.cumulative_closure_residual,
        receipt.maximum_frame_residual
    );
    assert!(receipt.control_work.abs() > 0.0);
    assert!(receipt.cumulative_closure_residual.abs() < 1e-12 * energy_scale.max(1e-30) * 96_000.0);
    assert!(receipt.maximum_frame_residual < 1e-13 * energy_scale.max(1e-30));
}

/// A small branched tube network over 4 coordinates (constriction depth and
/// place, end opening, branch port), driven by a flow valve.
fn network_plan(connection: ConnectionPolicy, sample_rate: f64) -> ConstitutiveWorldPlan {
    let mut plan =
        ConstitutiveWorldPlan::from_modal_v2(WorldPlan::string_brass_v1(sample_rate)).unwrap();
    let oral = 28;
    let nasal = 14;
    let network = TubeNetworkSpec {
        speed_of_sound_m_s: 343.0,
        air_density_kg_m3: 1.204,
        area_floor_m2: 2.5e-6,
        area_ceiling_m2: 1.8e-3,
        tubes: vec![
            TubeSpec {
                id: "oral".into(),
                length_m: 0.16,
                base_log_area_m2: vec![(3.0e-4f64).ln(); oral],
                attenuation_np_per_m: 0.45,
            },
            TubeSpec {
                id: "nasal".into(),
                length_m: 0.11,
                base_log_area_m2: vec![(1.0e-4f64).ln(); nasal],
                attenuation_np_per_m: 1.6,
            },
        ],
        branches: vec![TubeBranchSpec {
            trunk: 0,
            trunk_section: 11,
            branch: 1,
        }],
        area_terms: vec![
            AreaTermSpec::Gaussian {
                tube: 0,
                amplitude: AffineSpec {
                    constant: 0.5,
                    terms: vec![(1, -3.5)],
                },
                center: AffineSpec {
                    constant: 0.25,
                    terms: vec![(0, 0.55)],
                },
                width: 0.1,
            },
            AreaTermSpec::LogScaleGaussian {
                tube: 0,
                scale: AffineSpec {
                    constant: 0.0,
                    terms: vec![(2, 1.5)],
                },
                floor: 1e-3,
                center: 0.965,
                width: 0.045,
            },
            AreaTermSpec::SectionArea {
                tube: 1,
                section: 0,
                area_m2: AffineSpec {
                    constant: 0.0,
                    terms: vec![(3, 1.0e-4)],
                },
            },
        ],
        radiating_ends: vec![
            RadiatingEndSpec {
                tube: 0,
                end_correction_ratio: 0.61,
            },
            RadiatingEndSpec {
                tube: 1,
                end_correction_ratio: 0.61,
            },
        ],
        contact: None,
    };
    let coordinates = ["place", "depth", "opening", "port"]
        .iter()
        .map(|name| ConfigurationCoordinate {
            name: (*name).into(),
            lower: 0.0,
            upper: 1.0,
            initial: 0.5,
        })
        .collect();
    let space = SpaceBodySpec {
        anatomy_name: "branched-tube-test".into(),
        configuration_space: ConfigurationSpaceSpec {
            coordinates,
            map: ConfigurationMapSpec::TubeNetwork { network },
            connection,
        },
        laws: SpaceBodyLaws::TubeNetwork {
            valve: test_valve(),
            aero: None,
        },
        reservoir: ReservoirSpec { capacity: 3.0 },
    };
    plan.bodies[0].constitution = BodyConstitutionSpec::ModalSpace { body: space };
    plan
}

/// The fold numbers of `ovillo-tract`'s `TractAnatomy::adult_v0` (design
/// hypotheses there).
fn test_valve() -> FlowValveSpec {
    FlowValveSpec {
        tube: 0,
        section: 0,
        length_m: 0.014,
        thickness_m: 0.003,
        mass_kg: 1.5e-4,
        damping_ratio: 0.10,
        mucosal_delay_s: 6.0e-4,
        contact_stiffness_ratio: 3.0,
        contact_damping_ratio: 1.0,
        rest_half_gap_open_m: 8.0e-4,
        rest_half_gap_closed_m: 5.0e-5,
        min_tuning_hz: 60.0,
        max_tuning_hz: 420.0,
        initial_tuning_hz: 120.0,
        initial_adduction: 0.6,
        max_supply_pressure_pa: 1_000.0,
        sag_fraction: 0.10,
        air_viscosity_pa_s: 1.8e-5,
    }
}

fn quiet_world_controls() -> WorldControls {
    let mut controls = WorldControls::default();
    controls.space.coupling = 0.0;
    controls
}

fn voiced(supply: f64) -> [ConstitutiveBodyControls; 1] {
    [ConstitutiveBodyControls::ModalSpace(SpaceBodyControls {
        body: BodyControls {
            work_input: 0.3,
            ..Default::default()
        },
        valve: FlowValveControls { supply },
        radiation_scale: 1.0,
        node_source: None,
    })]
}

/// G-A2 on a computed (law) map: a random walk in the 4-D tube-network space,
/// voiced by the valve, with modes recomputed at every move and the
/// mass-momentum connection, keeps the books closed.
#[test]
fn random_walk_in_a_four_dimensional_tube_network_closes_the_ledger() {
    for connection in [
        ConnectionPolicy::MassMomentum {
            disposal: DisposalPolicy::Dissipate,
        },
        ConnectionPolicy::ModalCoordinates,
    ] {
        let mut world = World::from_constitutive_plan(network_plan(connection, 48_000.0)).unwrap();
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let mut point = [0.5; 4];
        let mut energy_scale = 0.0_f64;
        for sample in 0..48_000usize {
            if sample % 64 == 0 {
                for x in &mut point {
                    *x = (*x + 0.05 * (rng.next() - 0.5)).clamp(0.0, 1.0);
                }
                world.set_body_configuration_point(0, &point).unwrap();
            }
            world
                .process_constitutive_sample(&voiced(0.6), quiet_world_controls())
                .unwrap();
            energy_scale = energy_scale.max(world.total_stored_energy());
        }
        let receipt = world.receipt();
        println!(
            "4-D tube network walk ({connection:?}): 750 recomputed moves, control work {:.3e} J, disposal {:.3e} J, source {:.3e} J, radiation {:.3e} J, stored scale {:.3e} J, cumulative residual {:.3e} J, max frame residual {:.3e} J",
            receipt.control_work, world.disposal_total(), receipt.source_work, receipt.radiation, energy_scale,
            receipt.cumulative_closure_residual, receipt.maximum_frame_residual
        );
        assert!(receipt.radiation > 0.0 && receipt.control_work != 0.0);
        // The reservoir (3 J) dominates the stored energy; bound against it.
        assert!(
            receipt.cumulative_closure_residual.abs() < 1e-11 * energy_scale,
            "{}",
            receipt.cumulative_closure_residual
        );
        assert!(receipt.maximum_frame_residual < 1e-13 * energy_scale);
    }
}

/// G-A2 holonomy: closed loops in configuration space report their net work.
/// Instantaneous loops (no dynamics between moves): the modal-coordinate
/// connection is flat (net work zero, state returned); the mass-momentum
/// connection is not (net work and disposal billed, state not returned).
/// A loop traversed while the field rings bills its net work under either.
#[test]
fn closed_loops_report_their_net_work() {
    let corners = [
        [0.5, 0.5, 0.5, 0.5],
        [0.9, 0.5, 0.5, 0.5],
        [0.9, 0.9, 0.5, 0.5],
        [0.5, 0.9, 0.9, 0.5],
        [0.5, 0.5, 0.9, 0.9],
        [0.5, 0.5, 0.5, 0.5],
    ];
    for connection in [
        ConnectionPolicy::ModalCoordinates,
        ConnectionPolicy::MassMomentum {
            disposal: DisposalPolicy::Dissipate,
        },
    ] {
        let mut world = World::from_constitutive_plan(network_plan(connection, 48_000.0)).unwrap();
        for _ in 0..4_800 {
            world
                .process_constitutive_sample(&voiced(0.6), quiet_world_controls())
                .unwrap();
        }
        let state_before = world.body_state(0).unwrap();
        let field_before = world.space_body(0).unwrap().field_energy();
        let mut work = 0.0;
        let mut disposal = 0.0;
        for corner in &corners[1..] {
            // Many small steps along each edge, no dynamics in between.
            let from = world.body_configuration_point(0).unwrap().to_vec();
            for step in 1..=20 {
                let t = step as f64 / 20.0;
                let p: Vec<f64> = (0..4)
                    .map(|i| from[i] + t * (corner[i] - from[i]))
                    .collect();
                let frame = world.set_body_configuration_point(0, &p).unwrap();
                work += frame.control_work;
                disposal += frame.disposal;
            }
        }
        let state_after = world.body_state(0).unwrap();
        let field_after = world.space_body(0).unwrap().field_energy();
        let returned = state_before == state_after;
        let distance = (0..MODE_COUNT)
            .map(|k| (state_after.momentum[k] - state_before.momentum[k]).powi(2))
            .sum::<f64>()
            .sqrt();
        println!(
            "instantaneous loop ({connection:?}): net work {work:.6e} J, disposal {disposal:.6e} J, field {field_before:.6e} -> {field_after:.6e} J, state returned {returned}, |dm| {distance:.3e}"
        );
        assert!(
            (field_after - field_before - (work - disposal)).abs()
                < 1e-14 * field_before.max(1e-30) + 1e-24
        );
        match connection {
            ConnectionPolicy::ModalCoordinates => {
                assert!(work.abs() < 1e-13 * field_before);
                assert_eq!(disposal, 0.0);
            }
            ConnectionPolicy::MassMomentum { .. } => {
                assert!(
                    work.abs() > 0.0 || disposal > 0.0,
                    "a curved connection should not return for free"
                );
            }
        }
        // The same loop while the field rings: billed, books closed.
        let before = world.receipt();
        let mut work_dyn = 0.0;
        for corner in &corners[1..] {
            let from = world.body_configuration_point(0).unwrap().to_vec();
            for step in 1..=40 {
                let t = step as f64 / 40.0;
                let p: Vec<f64> = (0..4)
                    .map(|i| from[i] + t * (corner[i] - from[i]))
                    .collect();
                work_dyn += world
                    .set_body_configuration_point(0, &p)
                    .unwrap()
                    .control_work;
                for _ in 0..32 {
                    world
                        .process_constitutive_sample(&voiced(0.6), quiet_world_controls())
                        .unwrap();
                }
            }
        }
        let after = world.receipt();
        println!(
            "voiced loop ({connection:?}): net control work {work_dyn:.6e} J over 6400 samples; cumulative residual {:.3e} -> {:.3e} J",
            before.cumulative_closure_residual, after.cumulative_closure_residual
        );
        assert!((after.control_work - before.control_work - work_dyn).abs() < 1e-15);
        assert!(after.cumulative_closure_residual.abs() < 1e-11 * 3.0);
    }
}

/// The modes the network computes for this 4-D space sit inside the band and
/// move with the coordinates (a sanity check, not a formant claim).
#[test]
fn tube_network_modes_move_with_every_coordinate() {
    let plan = network_plan(
        ConnectionPolicy::MassMomentum {
            disposal: DisposalPolicy::Dissipate,
        },
        48_000.0,
    );
    let mut world = World::from_constitutive_plan(plan).unwrap();
    let base = world.space_body(0).unwrap().frequencies_hz();
    for axis in 0..4 {
        let mut p = [0.5; 4];
        p[axis] = 0.95;
        world.set_body_configuration_point(0, &p).unwrap();
        let moved = world.space_body(0).unwrap().frequencies_hz();
        let change = (0..6)
            .map(|k| (moved[k] / base[k]).ln().abs())
            .fold(0.0, f64::max);
        println!(
            "axis {axis}: lowest 6 modes {:?} -> {:?}",
            base[..6].iter().map(|f| f.round()).collect::<Vec<_>>(),
            moved[..6].iter().map(|f| f.round()).collect::<Vec<_>>()
        );
        assert!(change > 1e-3, "axis {axis} did not move the modes");
        world.set_body_configuration_point(0, &[0.5; 4]).unwrap();
    }
}
