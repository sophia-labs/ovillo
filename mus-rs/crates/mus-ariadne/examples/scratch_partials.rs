// scratch (not committed): per-partial energy of a struck C4 wire after the blow.
use mus_ariadne::*;
fn main() {
    let args: Vec<f64> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let (v, mass) = (args[0], args[1]);
    let fs = 48_000.0;
    let (l, mu, f0) = (0.638, 5.87e-3, 261.625_565);
    let port = SpacePortSpec { body_directions: [[1.0 / 24f64.sqrt(); 24]; 8], coupling_weights: [0.0; 8] };
    let mut bodies = Vec::new();
    for i in 0..3 {
        let wire = StiffWireSpec { speaking_length_m: l, core_diameter_m: 0.975e-3, linear_density_kg_m: mu, youngs_modulus_pa: 2e11,
            tension_n: mu * (2.0 * f0 * l).powi(2), tension_coordinate: 0, band_limit_hz: 21600.0, decay_constant_per_s: 0.25, decay_quadratic_s: 6e-9 };
        bodies.push(ConstitutiveBodyInstanceSpec { id: format!("w{i}"), constitution: BodyConstitutionSpec::ModalSpace { body: SpaceBodySpec {
            anatomy_name: "w".into(), configuration_space: ConfigurationSpaceSpec { coordinates: vec![ConfigurationCoordinate { name: "t".into(), lower: 0.98, upper: 1.02, initial: 1.0 }],
            map: ConfigurationMapSpec::StiffWire { wire }, connection: ConnectionPolicy::ModalCoordinates }, laws: SpaceBodyLaws::Passive { damper: None },
            reservoir: ReservoirSpec { capacity: 1e-6 } } }, space_port: port, pose: BodyPose::default() });
    }
    bodies.push(ConstitutiveBodyInstanceSpec { id: "h".into(), constitution: BodyConstitutionSpec::Hammer { hammer: HammerSpec {
        mass_kg: mass, felt_stiffness: 4.5e9, felt_exponent: 2.5, felt_relaxation_s: 1e-5, contact_width_m: 0.0065, letoff_m: 0.002, catch_m: 0.008,
        wires: (0..3).map(|i| HammerContactSpec { body: format!("w{i}"), strike_fraction: 78.4 / 638.0 }).collect() } }, space_port: port, pose: BodyPose::default() });
    let plan = ConstitutiveWorldPlan { schema_version: WORLD_PLAN_V3_SCHEMA_VERSION.into(), world_name: "x".into(), internal_sample_rate_hz: fs, bodies,
        shared_space: WorldPlan::string_brass_v1(fs).shared_space, direct_couplings: vec![] };
    let mut world = World::from_constitutive_plan(plan).unwrap();
    let controls: Vec<_> = (0..4).map(|_| ConstitutiveBodyControls::ModalSpace(SpaceBodyControls { body: BodyControls { radiation_fraction: 0.0, ..Default::default() }, ..Default::default() })).collect();
    world.strike_hammer(3, v).unwrap();
    for _ in 0..480 { world.process_constitutive_sample(&controls, WorldControls::default()).unwrap(); }
    let st = world.body_state(0).unwrap();
    let f = world.space_body(0).unwrap().frequencies_hz();
    let e: Vec<f64> = (0..24).map(|k| { let w = 2.0*std::f64::consts::PI*f[k]; 0.5/fs*(st.momentum[k].powi(2) + (w*st.displacement[k]).powi(2)) }).collect();
    let top = e.iter().cloned().fold(0.0, f64::max);
    let s: Vec<String> = e.iter().map(|x| format!("{:.1}", 10.0*(x/top).log10())).collect();
    println!("{}", s.join(" "));
}
