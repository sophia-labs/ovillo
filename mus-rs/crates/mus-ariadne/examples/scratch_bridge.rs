// scratch (not committed): decay of one partial of the C4 wire through the composed bridge.
use mus_ariadne::*;
use serde_json::Value;
fn unit(v: &[f64; 24]) -> [f64; 24] { let n = v.iter().map(|x| x * x).sum::<f64>().sqrt(); v.map(|x| x / n) }
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let a: Value = serde_json::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let key = 40; let partial: usize = args[2].parse().unwrap();
    let fs = 48_000.0;
    let note = &a["notes"][key - 1];
    let w0 = &note["wires"][0];
    let wire = StiffWireSpec { speaking_length_m: note["speaking_length_m"].as_f64().unwrap(), core_diameter_m: note["core_diameter_m"].as_f64().unwrap(),
        linear_density_kg_m: note["linear_density_kg_m"].as_f64().unwrap(), youngs_modulus_pa: 2e11, tension_n: w0["tension_n"].as_f64().unwrap(),
        tension_coordinate: 0, band_limit_hz: 21600.0, decay_constant_per_s: 1e-9, decay_quadratic_s: 0.0 };
    let port0 = SpacePortSpec { body_directions: [[1.0 / 24f64.sqrt(); 24]; 8], coupling_weights: [0.0; 8] };
    let mut bodies = vec![ConstitutiveBodyInstanceSpec { id: "w".into(), constitution: BodyConstitutionSpec::ModalSpace { body: SpaceBodySpec {
        anatomy_name: "w".into(), configuration_space: ConfigurationSpaceSpec { coordinates: vec![ConfigurationCoordinate { name: "t".into(), lower: 0.98, upper: 1.02, initial: 1.0 }],
        map: ConfigurationMapSpec::StiffWire { wire: wire.clone() }, connection: ConnectionPolicy::ModalCoordinates }, laws: SpaceBodyLaws::Passive { damper: None },
        reservoir: ReservoirSpec { capacity: 1e-6 } } }, space_port: port0, pose: BodyPose::default() }];
    let retained = wire.retained_partials(1.02);
    let weights = wire.moving_end_weights(1.0, retained);
    let wn = weights.iter().map(|v| v * v).sum::<f64>().sqrt();
    let mut edges = vec![]; let mut fractions = vec![0.0];
    let inert = ApertureSpec { natural_frequency_hz: 500.0, quartic_stiffness: 0.0, asymmetry: 0.0, equilibrium_displacement: 0.0, opening_bias: 0.0, damping_per_second: 50.0,
        source_to_aperture_rate_hz: 0.0, source_to_field_rate_hz: 0.0, sweep_rate_hz: 0.0, active_rate_per_second: 0.0, active_target_momentum: 0.25 };
    for (b, board) in a["board"]["bodies"].as_array().unwrap().iter().enumerate() {
        let arr = |k: &str| -> [f64; 24] { let v: Vec<f64> = board[k].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect(); v.try_into().unwrap() };
        let c = WorldConfiguration { metric: MetricSpec { frequency_hz: arr("frequency_hz"), t60_seconds: arr("t60_seconds") }, aperture: inert, radiation_direction: arr("radiation_direction") };
        bodies.push(ConstitutiveBodyInstanceSpec { id: format!("b{b}"), constitution: BodyConstitutionSpec::ModalSpace { body: SpaceBodySpec { anatomy_name: "b".into(),
            configuration_space: ConfigurationSpaceSpec { coordinates: vec![ConfigurationCoordinate { name: "s".into(), lower: 0.0, upper: 1.0, initial: 0.0 }],
            map: ConfigurationMapSpec::Multilinear { axes: vec![vec![0.0, 1.0]], anchors: vec![c, c] }, connection: ConnectionPolicy::ModalCoordinates },
            laws: SpaceBodyLaws::Passive { damper: None }, reservoir: ReservoirSpec { capacity: 1e-6 } } }, space_port: port0, pose: BodyPose::default() });
        fractions.push(if args.len() > 3 { 0.0 } else { board["radiation_fraction"].as_f64().unwrap() });
        let sh: Vec<f64> = w0["board_shapes"][b].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        let sh: [f64; 24] = sh.try_into().unwrap(); let sn = sh.iter().map(|v| v * v).sum::<f64>().sqrt();
        edges.push(DirectCouplingSpec { id: format!("e{b}"), left_body: "w".into(), right_body: format!("b{b}"), left_direction: unit(&weights), right_direction: sh.map(|v| v / sn), rate_hz: wn * sn / std::f64::consts::TAU, weight: 1.0 });
    }
    bodies.push(ConstitutiveBodyInstanceSpec { id: "h".into(), constitution: BodyConstitutionSpec::Hammer { hammer: HammerSpec {
        mass_kg: 0.0089 / 3.0, felt_stiffness: 4.5e9, felt_exponent: 2.5, felt_relaxation_s: 1e-5, contact_width_m: 0.0065, letoff_m: 0.002, catch_m: 0.008,
        wires: vec![HammerContactSpec { body: "w".into(), strike_fraction: note["strike_fraction"].as_f64().unwrap() }] } }, space_port: port0, pose: BodyPose::default() });
    fractions.push(0.0);
    let plan = ConstitutiveWorldPlan { schema_version: WORLD_PLAN_V3_SCHEMA_VERSION.into(), world_name: "x".into(), internal_sample_rate_hz: fs, bodies,
        shared_space: WorldPlan::string_brass_v1(fs).shared_space, direct_couplings: edges };
    let mut world = World::from_constitutive_plan(plan).unwrap();
    let controls: Vec<_> = fractions.iter().map(|f| ConstitutiveBodyControls::ModalSpace(SpaceBodyControls { body: BodyControls { radiation_fraction: *f, ..Default::default() }, ..Default::default() })).collect();
    let h = world.body_count() - 1;
    world.strike_hammer(h, 2.0).unwrap();
    let f = world.space_body(0).unwrap().frequencies_hz();
    let mut series: Vec<Vec<f64>> = vec![vec![]; 24];
    let nb = world.body_count() - 2;
    let (mut radiated, mut lost) = (vec![0.0; nb], vec![0.0; nb]);
    let (mut esum, mut expect) = (vec![0.0; nb], vec![0.0; nb]);
    let sig: Vec<[f64; 24]> = a["board"]["bodies"].as_array().unwrap().iter().map(|b| {
        let phi = b["radiation_fraction"].as_f64().unwrap();
        let r: Vec<f64> = b["radiation_direction"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        std::array::from_fn(|k| phi * r[k] * r[k] * fs / 4.0) }).collect();
    let bf: Vec<[f64; 24]> = a["board"]["bodies"].as_array().unwrap().iter().map(|b| {
        let r: Vec<f64> = b["frequency_hz"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect(); r.try_into().unwrap() }).collect();
    for s in 0..(fs as usize) {
        world.process_constitutive_sample(&controls, WorldControls::default()).unwrap();
        for b in 0..nb { let e = world.body_emission(b + 1).unwrap(); radiated[b] += 0.5 / fs * e * e; lost[b] += world.body_last_internal_loss(b + 1).unwrap();
            if s % 16 == 0 { let st = world.body_state(b + 1).unwrap(); for k in 0..24 { let w = std::f64::consts::TAU * bf[b][k]; let ek = 0.5 / fs * (st.momentum[k].powi(2) + (w * st.displacement[k]).powi(2)); esum[b] += ek; expect[b] += 2.0 * sig[b][k] * ek; } } }
        if s >= 960 && s % 48 == 0 {
            let st = world.body_state(0).unwrap();
            for k in 0..24 { let w = std::f64::consts::TAU * f[k]; series[k].push(0.5 / fs * (st.momentum[k].powi(2) + (w * st.displacement[k]).powi(2))); }
        }
    }
    let _ = partial;
    for b in 0..nb { println!("board/{b}: radiated {:.3e} lost {:.3e} share {:.3}; diagonal law would radiate {:.3e}", radiated[b], lost[b], radiated[b] / (radiated[b] + lost[b]), expect[b] * 16.0 / fs); }
    for k in 0..16 {
        let y: Vec<f64> = series[k].iter().map(|e| e.ln()).collect();
        let n = y.len() as f64; let t: Vec<f64> = (0..y.len()).map(|i| i as f64 * 0.001).collect();
        let (mt, my) = (t.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
        let slope = t.iter().zip(&y).map(|(a, b)| (a - mt) * (b - my)).sum::<f64>() / t.iter().map(|a| (a - mt).powi(2)).sum::<f64>();
        println!("partial {:2} {:8.1} Hz: energy decay {:7.2} /s -> amplitude sigma {:6.3} /s; start {:.2e}", k + 1, f[k], -slope, -slope / 2.0, series[k][0]);
    }
}
