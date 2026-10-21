//! Stiff wires, felt hammers and felt dampers (`law::hammer`, `law::damper`,
//! `ConfigurationMapSpec::StiffWire`): the partials follow from material and
//! geometry, a hammer blow closes the World's ledger to roundoff, a harder
//! blow touches for less time and leaves a brighter wire, and a resting
//! damper stops a ringing wire.
//!
//! Real runtime, no mocks: every test builds `mus_ariadne::World`s from plans.

use mus_ariadne::{
    BodyConstitutionSpec, BodyControls, BodyPose, ConfigurationCoordinate, ConfigurationMapSpec,
    ConfigurationSpaceSpec, ConnectionPolicy, ConstitutiveBodyControls, ConstitutiveBodyInstanceSpec,
    ConstitutiveWorldPlan, DamperSpec, HammerContactSpec, HammerSpec, ReservoirSpec, SpaceBodyControls,
    SpaceBodyLaws, SpaceBodySpec, SpacePortSpec, StiffWireSpec, World, WorldControls, WorldPlan,
    MODE_COUNT, SPACE_COURSES, WORLD_PLAN_V3_SCHEMA_VERSION,
};

const SAMPLE_RATE: f64 = 48_000.0;
const STEEL_E: f64 = 2.0e11;

/// C4 of the Estonia "Parlour" grand (Engelbrecht, Magi & Stulov 1999,
/// Table 1, n = 40): L = 638 mm, plain wire d = 0.975 mm, mu = 5.87 g/m,
/// struck 78.4 mm from the agraffe; tension from f0 = 261.63 Hz.
fn c4_wire(tension_scale: f64) -> StiffWireSpec {
    let (length, mu, f0) = (0.638, 5.87e-3, 261.625_565);
    StiffWireSpec {
        speaking_length_m: length,
        core_diameter_m: 0.975e-3,
        linear_density_kg_m: mu,
        youngs_modulus_pa: STEEL_E,
        tension_n: mu * (2.0 * f0 * length).powi(2) * tension_scale,
        tension_coordinate: 0,
        band_limit_hz: 0.45 * SAMPLE_RATE,
        decay_constant_per_s: 0.25,
        decay_quadratic_s: 6.0e-9,
    }
}

fn inert_port() -> SpacePortSpec {
    SpacePortSpec {
        body_directions: [[1.0 / (MODE_COUNT as f64).sqrt(); MODE_COUNT]; SPACE_COURSES],
        coupling_weights: [0.0; SPACE_COURSES],
    }
}

fn wire_body(id: &str, wire: StiffWireSpec, damper: Option<DamperSpec>) -> ConstitutiveBodyInstanceSpec {
    ConstitutiveBodyInstanceSpec {
        id: id.into(),
        constitution: BodyConstitutionSpec::ModalSpace {
            body: SpaceBodySpec {
                anatomy_name: "test-wire".into(),
                configuration_space: ConfigurationSpaceSpec {
                    coordinates: vec![ConfigurationCoordinate { name: "tension_ratio".into(), lower: 0.98, upper: 1.02, initial: 1.0 }],
                    map: ConfigurationMapSpec::StiffWire { wire },
                    connection: ConnectionPolicy::ModalCoordinates,
                },
                laws: SpaceBodyLaws::Passive { damper },
                reservoir: ReservoirSpec { capacity: 1.0e-6 },
            },
        },
        space_port: inert_port(),
        pose: BodyPose::default(),
    }
}

fn hammer_body(wires: &[&str], strike_fraction: f64) -> ConstitutiveBodyInstanceSpec {
    ConstitutiveBodyInstanceSpec {
        id: "hammer".into(),
        constitution: BodyConstitutionSpec::Hammer {
            hammer: HammerSpec {
                // Stulov 1995 Table 1 (C4 head 8.9 g); felt K and p for C4 as
                // that paper quotes them from Chaigne & Askenfelt.
                mass_kg: 8.9e-3,
                felt_stiffness: 4.5e9,
                felt_exponent: 2.5,
                felt_relaxation_s: 1.0e-5,
                contact_width_m: 6.0e-3,
                letoff_m: 2.0e-3,
                catch_m: 8.0e-3,
                wires: wires
                    .iter()
                    .map(|body| HammerContactSpec { body: (*body).into(), strike_fraction })
                    .collect(),
            },
        },
        space_port: inert_port(),
        pose: BodyPose::default(),
    }
}

fn damper() -> DamperSpec {
    DamperSpec { center_fraction: 0.16, length_m: 0.015, resistance_kg_s: 0.3, points: 4 }
}

/// A C4 unison of `strings` wires (tuned 0, +0.6, -0.6 cents) and its hammer.
fn unison_plan(strings: usize, with_damper: bool) -> ConstitutiveWorldPlan {
    let cents = [0.0, 0.6, -0.6];
    let mut bodies: Vec<ConstitutiveBodyInstanceSpec> = (0..strings)
        .map(|i| {
            let scale = 2f64.powf(2.0 * cents[i] / 1200.0);
            wire_body(&format!("wire/{i}"), c4_wire(scale), with_damper.then(damper))
        })
        .collect();
    let ids: Vec<String> = (0..strings).map(|i| format!("wire/{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    bodies.push(hammer_body(&refs, 78.4 / 638.0));
    ConstitutiveWorldPlan {
        schema_version: WORLD_PLAN_V3_SCHEMA_VERSION.into(),
        world_name: "struck-wire-test".into(),
        internal_sample_rate_hz: SAMPLE_RATE,
        bodies,
        shared_space: WorldPlan::string_brass_v1(SAMPLE_RATE).shared_space,
        direct_couplings: Vec::new(),
    }
}

fn controls(world: &World) -> Vec<ConstitutiveBodyControls> {
    (0..world.body_count())
        .map(|_| {
            ConstitutiveBodyControls::ModalSpace(SpaceBodyControls {
                body: BodyControls { radiation_fraction: 0.0, ..Default::default() },
                ..Default::default()
            })
        })
        .collect()
}

/// Energy-weighted mean frequency of a wire's modes (Hz).
fn modal_centroid(world: &World, body: usize) -> f64 {
    let state = world.body_state(body).unwrap();
    let f = world.space_body(body).unwrap().frequencies_hz();
    let (mut num, mut den) = (0.0, 0.0);
    for k in 0..MODE_COUNT {
        let w = std::f64::consts::TAU * f[k];
        let e = state.momentum[k].powi(2) + (w * state.displacement[k]).powi(2);
        num += f[k] * e;
        den += e;
    }
    num / den
}

#[test]
fn stiff_wire_partials_follow_from_material_and_geometry() {
    let wire = c4_wire(1.0);
    // Independently: f0 = sqrt(T/mu)/(2L), B = pi^3 E d^4 / (64 T L^2).
    let f0 = (wire.tension_n / wire.linear_density_kg_m).sqrt() / (2.0 * wire.speaking_length_m);
    let b = std::f64::consts::PI.powi(3) * STEEL_E * wire.core_diameter_m.powi(4)
        / (64.0 * wire.tension_n * wire.speaking_length_m.powi(2));
    assert!((f0 - 261.625_565).abs() < 1e-9);
    assert!((b - 3.29e-4).abs() < 0.02e-4, "C4 B = {b:e}");
    let world = World::from_constitutive_plan(unison_plan(1, false)).unwrap();
    let body = world.space_body(0).unwrap();
    let (spec, retained) = body.wire().unwrap();
    assert_eq!(retained, MODE_COUNT, "all 24 C4 partials lie below 21.6 kHz");
    for (k, f) in body.frequencies_hz().iter().enumerate() {
        let n = k as f64 + 1.0;
        let law = n * f0 * (1.0 + b * n * n).sqrt();
        assert!((f / law - 1.0).abs() < 1e-13, "partial {n}: {f} vs {law}");
    }
    // Mass-normalized pinned shapes: sum_x mu psi_j psi_k dx = delta_jk.
    let shapes: Vec<[f64; MODE_COUNT]> =
        (0..4_000).map(|i| spec.mode_shape_at((i as f64 + 0.5) / 4_000.0, 0.0, retained)).collect();
    let dx = spec.speaking_length_m / 4_000.0;
    for j in [0, 5, 23] {
        for k in [0, 5, 23] {
            let gram: f64 = shapes.iter().map(|s| s[j] * s[k]).sum::<f64>() * spec.linear_density_kg_m * dx;
            let want = if j == k { 1.0 } else { 0.0 };
            assert!((gram - want).abs() < 1e-9, "<{j},{k}> = {gram}");
        }
    }
    // A wire hardly radiates: its own (line-dipole) rate is far below its loss.
    let dipole = spec.dipole_radiation_sigma(1.0, retained, spec.core_diameter_m, 1.204, 343.0);
    assert!(dipole[0] < 1e-5 && dipole[23] < 0.05 && dipole[23] > dipole[22], "{:e} {:e}", dipole[0], dipole[23]);
    // C7 (Engelbrecht et al. n = 76: L = 90 mm, d = 0.875 mm): only 7
    // partials lie below 21.6 kHz; the other slots carry nothing.
    let c7 = StiffWireSpec {
        speaking_length_m: 0.090,
        core_diameter_m: 0.875e-3,
        linear_density_kg_m: 4.73e-3,
        tension_n: 4.73e-3 * (2.0 * 2093.004_522 * 0.090_f64).powi(2),
        ..c4_wire(1.0)
    };
    assert_eq!(c7.retained_partials(1.02), 7);
    let shape = c7.mode_shape_at(0.0567, 0.004, 7);
    assert!(shape[..7].iter().all(|v| *v != 0.0) && shape[7..].iter().all(|v| *v == 0.0));
    assert!(c7.moving_end_weights(1.0, 7)[7..].iter().all(|v| *v == 0.0));
}

#[test]
fn a_hammer_blow_closes_the_ledger_and_the_backcheck_catches_it() {
    let plan = unison_plan(3, false);
    let mut world = World::from_constitutive_plan(plan).unwrap();
    let controls = controls(&world);
    let hammer = world.body_index("hammer").unwrap();
    let mut scale = 0.0_f64;
    let mut contact_force_peak = 0.0_f64;
    for sample in 0..9_600usize {
        if sample == 10 {
            assert!(world.strike_hammer(hammer, 3.0).unwrap());
        }
        world.process_constitutive_sample(&controls, WorldControls::default()).unwrap();
        scale = scale.max(world.total_stored_energy());
        contact_force_peak = contact_force_peak.max(world.hammer_frame(hammer).unwrap().force_n);
    }
    let totals = world.hammer_totals(hammer).unwrap();
    let receipt = world.receipt();
    let kinetic = 0.5 * 8.9e-3 * 9.0;
    let wires: f64 = (0..3).map(|i| world.body_material_energy(i).unwrap()).sum();
    println!(
        "C4 trichord at 3 m/s: contact {:.3} ms, peak force {:.1} N, felt loss {:.3e} J, caught {:.3e} J, wires {:.3e} J of {:.3e}; \
         cumulative residual {:.3e} J, max frame {:.3e} J",
        totals.last_contact_samples as f64 / SAMPLE_RATE * 1e3,
        totals.last_peak_force_n,
        totals.felt_loss,
        totals.catch_loss,
        wires,
        kinetic,
        receipt.cumulative_closure_residual,
        receipt.maximum_frame_residual
    );
    assert_eq!(receipt.source_work, kinetic, "the throw is the only source work");
    assert!(receipt.maximum_frame_residual < 1e-15 * scale.max(1.0) + 1e-16, "every frame closes to roundoff");
    assert!(receipt.cumulative_closure_residual.abs() < 1e-13);
    assert!(totals.felt_loss > 0.0 && totals.felt_loss < 0.5 * kinetic, "the felt's relaxation is a modest loss");
    assert!(!world.hammer_active(hammer).unwrap(), "after the rebound the backcheck holds the hammer");
    assert!(totals.catch_loss > 0.0);
    let contact_ms = totals.last_contact_samples as f64 / SAMPLE_RATE * 1e3;
    assert!((0.5..5.0).contains(&contact_ms), "contact {contact_ms} ms");
    assert!(wires > 0.2 * kinetic, "a good part of the throw is left in the wires");
}

#[test]
fn a_harder_blow_touches_for_less_time_and_leaves_a_brighter_wire() {
    let mut rows = Vec::new();
    for velocity in [0.5, 1.5, 4.0] {
        let mut world = World::from_constitutive_plan(unison_plan(1, false)).unwrap();
        let controls = controls(&world);
        let hammer = world.body_index("hammer").unwrap();
        world.strike_hammer(hammer, velocity).unwrap();
        for _ in 0..1_200 {
            world.process_constitutive_sample(&controls, WorldControls::default()).unwrap();
        }
        let totals = world.hammer_totals(hammer).unwrap();
        rows.push((velocity, totals.last_contact_samples, modal_centroid(&world, 0), totals.last_peak_force_n));
        assert!(world.receipt().maximum_frame_residual < 1e-15);
    }
    for (v, samples, centroid, force) in &rows {
        println!("C4 single wire, {v} m/s: contact {:.3} ms, wire centroid {centroid:.0} Hz, peak {force:.1} N", *samples as f64 / SAMPLE_RATE * 1e3);
    }
    assert!(rows.windows(2).all(|w| w[1].1 < w[0].1), "contact shortens with velocity");
    assert!(rows.windows(2).all(|w| w[1].2 > w[0].2), "the wire's energy moves up the partials");
}

#[test]
fn a_resting_damper_stops_a_ringing_wire_and_books_its_loss() {
    let mut energies = Vec::new();
    for rest in [false, true] {
        let mut world = World::from_constitutive_plan(unison_plan(1, true)).unwrap();
        let controls = controls(&world);
        let hammer = world.body_index("hammer").unwrap();
        world.strike_hammer(hammer, 2.0).unwrap();
        for sample in 0..24_000usize {
            if rest && sample == 4_800 {
                world.set_body_damper(0, 1.0).unwrap();
            }
            world.process_constitutive_sample(&controls, WorldControls::default()).unwrap();
            if sample == 4_799 {
                energies.push(world.body_material_energy(0).unwrap());
            }
        }
        energies.push(world.body_material_energy(0).unwrap());
        assert!(world.receipt().maximum_frame_residual < 1e-15);
        assert_eq!(world.body_damper(0), Some(if rest { 1.0 } else { 0.0 }));
    }
    let (free_db, damped_db) = (
        10.0 * (energies[1] / energies[0]).log10(),
        10.0 * (energies[3] / energies[2]).log10(),
    );
    println!("C4 wire, 0.4 s after 0.1 s: free {free_db:.1} dB, damper resting {damped_db:.1} dB");
    assert!(damped_db < free_db - 30.0, "the damper takes the note away");
}

#[test]
fn a_hammer_refuses_a_second_throw_while_its_felt_presses() {
    let mut world = World::from_constitutive_plan(unison_plan(2, false)).unwrap();
    let controls = controls(&world);
    let hammer = world.body_index("hammer").unwrap();
    assert!(world.strike_hammer(hammer, 2.0).unwrap());
    let mut refused = false;
    for _ in 0..2_000 {
        world.process_constitutive_sample(&controls, WorldControls::default()).unwrap();
        if world.hammer_in_contact(hammer).unwrap() && !refused {
            assert!(!world.strike_hammer(hammer, 2.0).unwrap());
            refused = true;
        }
    }
    assert!(refused);
    // Flying back (not yet caught) it may be thrown again: the backcheck
    // takes its kinetic energy first, and the books still close.
    assert!(world.strike_hammer(hammer, 1.0).unwrap());
    for _ in 0..2_000 {
        world.process_constitutive_sample(&controls, WorldControls::default()).unwrap();
    }
    let totals = world.hammer_totals(hammer).unwrap();
    assert_eq!((totals.strikes, totals.refused_strikes), (2, 1));
    assert!(world.receipt().maximum_frame_residual < 1e-15);
}
