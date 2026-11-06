//! Lowering a [`BodySpec`] into one `ariadne-world-plan-v3`.
//!
//! ```text
//! body "air"      a tube network (the bore, its bumps, up to three side branches),
//!                 her voice as a volume velocity at one volume, a sealed or loose
//!                 flap in the entry wall, POC-3's air at the narrowest column,
//!                 radiating ends at the mouth and at every open branch
//! bodies "res1-3" passive modal resonators (string, bar, membrane, plate, bell:
//!                 declared partial series), each touching one air volume through a
//!                 reciprocal direct coupling, each radiating mode by mode
//! body "powered"  the canonical string/brass material with its reservoir-paid
//!                 active inclusion, touching one air volume the same way
//! shared space    the canonical eight-course delayed field, ported at the mouth
//! ```

use std::f64::consts::PI;

use mus_ariadne::{
    AeroSpec, AffineSpec, AreaTermSpec, BodyConstitutionSpec, ConfigurationCoordinate, ConfigurationMapSpec,
    ConfigurationSpaceSpec, ConnectionPolicy, ConstitutiveBodyInstanceSpec, ConstitutiveWorldPlan, ContactSpec,
    DirectCouplingSpec, DisposalPolicy, FlowValveSpec, MetricSpec, RadiatingEndSpec, ReservoirSpec, SpaceBody,
    SpaceBodyLaws, SpaceBodySpec, SpacePortSpec, TubeBranchSpec, TubeNetworkSpec, TubeSpec, WorldConfiguration,
    WorldPlan, MODE_COUNT, WORLD_PLAN_V3_SCHEMA_VERSION,
};

use crate::surface::{BodySpec, BRANCHES, RESONATORS};

pub const SPEED_OF_SOUND_M_S: f64 = 343.0;
pub const AIR_DENSITY_KG_M3: f64 = 1.204;
pub const AIR_VISCOSITY_PA_S: f64 = 1.8e-5;
/// The positive cone every section is clamped into.
pub const AREA_FLOOR_M2: f64 = 2.0e-6;
pub const AREA_CEIL_M2: f64 = 0.25;
pub const MAX_NODES: usize = 128;

/// One radiating end of the air, as the listener needs it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AirEnd {
    pub label: String,
    pub tube: usize,
    pub node: usize,
    /// Sideways position of the opening (m; the mouth is at 0).
    pub x_m: f64,
    /// Half-section plus end correction: the end column's length (m).
    pub column_m: f64,
    pub end_correction_ratio: f64,
    pub dx_m: f64,
}

/// A resonator or the powered material, as the listener and the receipt need it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Radiator {
    pub label: String,
    pub body: usize,
    pub x_m: f64,
    pub level: f64,
    /// The air volume it touches and the coupling's rate (Hz).
    pub touches_node: usize,
    pub coupling_hz: f64,
    #[serde(skip)]
    pub powered: bool,
    pub frequency_hz: Vec<f64>,
    pub t60_seconds: Vec<f64>,
    pub kind: String,
}

/// What the lowering decided, kept beside the World.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Layout {
    pub bore_sections: usize,
    pub bore_dx_m: f64,
    pub nodes: usize,
    pub drive_node: usize,
    pub mouth_node: usize,
    pub ends: Vec<AirEnd>,
    pub radiators: Vec<Radiator>,
    pub branch_tubes: Vec<Option<usize>>,
    pub room_ported: bool,
    pub air_on: bool,
    pub flap_on: bool,
    pub notes: Vec<String>,
}

/// The tube network of the air.
pub fn network(spec: &BodySpec, mouth_initial: f64) -> Result<(TubeNetworkSpec, Layout), String> {
    let n = spec.get_usize("bore.sections").clamp(12, 96);
    let length = spec.get("bore.length_m");
    let dx = length / n as f64;
    let a0 = spec.get("bore.entry_cm2") * 1.0e-4;
    let a1 = spec.get("bore.exit_cm2") * 1.0e-4;
    let flare = spec.get("bore.flare");
    let mut bore_log: Vec<f64> = (0..n)
        .map(|i| {
            let s = (i as f64 + 0.5) / n as f64;
            let mut ln_a = a0.ln() + (a1.ln() - a0.ln()) * s.powf(flare);
            for k in 1..=crate::surface::BUMPS {
                let depth = spec.get(&format!("bump{k}.depth"));
                if depth != 0.0 {
                    let c = spec.get(&format!("bump{k}.position"));
                    let w = spec.get(&format!("bump{k}.width"));
                    let z = (s - c) / w;
                    ln_a += depth * (-0.5 * z * z).exp();
                }
            }
            ln_a
        })
        .collect();
    for v in &mut bore_log {
        *v = v.clamp(AREA_FLOOR_M2.ln(), AREA_CEIL_M2.ln());
    }
    let mut tubes = vec![TubeSpec {
        id: "bore".into(),
        length_m: length,
        base_log_area_m2: bore_log,
        attenuation_np_per_m: spec.get("bore.wall_loss_np_m"),
    }];
    let mut branches = Vec::new();
    let mut ends = vec![AirEnd {
        label: "mouth".into(),
        tube: 0,
        node: n - 1,
        x_m: 0.0,
        column_m: 0.0,
        end_correction_ratio: spec.get("bore.end_correction"),
        dx_m: dx,
    }];
    let mut radiating_ends = vec![RadiatingEndSpec { tube: 0, end_correction_ratio: spec.get("bore.end_correction") }];
    let mut area_terms = Vec::new();
    let mut branch_tubes = vec![None; BRANCHES];
    let mut notes = Vec::new();
    let mut nodes = n;
    for k in 1..=BRANCHES {
        let p = |name: &str| spec.get(&format!("branch{k}.{name}"));
        if p("on") < 0.5 {
            continue;
        }
        let sections = spec.get_usize(&format!("branch{k}.sections")).clamp(2, 32);
        if nodes + sections > MAX_NODES {
            return Err(format!("branch{k}: the network would exceed {MAX_NODES} volumes ({} + {sections}); use fewer sections", nodes));
        }
        let tube = tubes.len();
        let neck = (p("neck_cm2") * 1.0e-4).clamp(AREA_FLOOR_M2, AREA_CEIL_M2);
        let area = (p("area_cm2") * 1.0e-4).clamp(AREA_FLOOR_M2, AREA_CEIL_M2);
        let mut log_area = vec![area.ln(); sections];
        log_area[0] = neck.ln();
        let blen = p("length_m");
        tubes.push(TubeSpec {
            id: format!("branch{k}"),
            length_m: blen,
            base_log_area_m2: log_area,
            attenuation_np_per_m: p("wall_loss_np_m"),
        });
        // Not on the mouth volume itself (the mouth constriction lives there).
        let trunk_section = ((p("position") * (n - 1) as f64).round() as usize).min(n - 2);
        branches.push(TubeBranchSpec { trunk: 0, trunk_section, branch: tube });
        if p("open") >= 0.5 {
            if radiating_ends.len() >= 4 {
                return Err("at most four radiating ends (the mouth and three open branches)".into());
            }
            radiating_ends.push(RadiatingEndSpec { tube, end_correction_ratio: 0.61 });
            ends.push(AirEnd {
                label: format!("branch{k}"),
                tube,
                node: nodes + sections - 1,
                x_m: p("vent_x_m"),
                column_m: 0.0,
                end_correction_ratio: 0.61,
                dx_m: blen / sections as f64,
            });
        }
        branch_tubes[k - 1] = Some(tube);
        nodes += sections;
    }
    // The one live coordinate: the mouth volume's opening, as a constriction
    // pressed into the last section, degree 1 - open.
    area_terms.push(AreaTermSpec::Constriction {
        tube: 0,
        position: AffineSpec::constant((n as f64 - 0.5) / n as f64),
        degree: AffineSpec { constant: 1.0, terms: vec![(0, -1.0)] },
        half_width: 0.6 / n as f64,
        shoulder_width: 1.0e-3,
    });
    let air_on = spec.on("air.on");
    let contact = air_on.then(|| ContactSpec {
        tube: 0,
        first_junction: 1,
        last_junction: n - 2,
        contact_area_m2: 0.0,
        sealed_area_m2: 1.0e-13,
        seal_branch_port: false,
        follow_below_floor: true,
    });
    // Never the mouth volume itself: a volume velocity dumped into the volume
    // the listener reads adds a one-sample, frequency-flat pressure term
    // (the volume's compliance cannot flow out within a sample: its local
    // Helmholtz rate is about 0.7 per sample), measured as a flat -6 dB floor
    // under the monopole law. One volume back it is gone.
    let drive_node = ((spec.get("drive.position") * (n - 1) as f64).round() as usize).min(n - 2);
    if spec.get("drive.position") > (n as f64 - 2.0) / (n as f64 - 1.0) {
        notes.push(format!("drive.position moved back to volume {drive_node}: the mouth volume cannot be driven directly (its readout would carry the injection's own one-sample term)"));
    }
    let _ = mouth_initial;
    if air_on && drive_node == 0 && a0 < 1.0e-5 {
        notes.push("the entry is narrower than 0.1 cm2 with air.on: her whole flow passes a jet there".into());
    }
    let network = TubeNetworkSpec {
        speed_of_sound_m_s: SPEED_OF_SOUND_M_S,
        air_density_kg_m3: AIR_DENSITY_KG_M3,
        area_floor_m2: AREA_FLOOR_M2,
        area_ceiling_m2: AREA_CEIL_M2,
        tubes,
        branches,
        area_terms,
        radiating_ends,
        contact,
    };
    let layout = Layout {
        bore_sections: n,
        bore_dx_m: dx,
        nodes,
        drive_node,
        mouth_node: n - 1,
        ends,
        radiators: Vec::new(),
        branch_tubes,
        room_ported: spec.get("room.port") > 0.0,
        air_on,
        flap_on: spec.on("flap.on"),
        notes,
    };
    Ok((network, layout))
}

fn valve(spec: &BodySpec) -> FlowValveSpec {
    if spec.on("flap.on") {
        let tuning = spec.get("flap.tuning_hz");
        FlowValveSpec {
            tube: 0,
            section: 0,
            length_m: spec.get("flap.length_mm") * 1.0e-3,
            thickness_m: spec.get("flap.thickness_mm") * 1.0e-3,
            mass_kg: spec.get("flap.mass_g") * 1.0e-3,
            damping_ratio: spec.get("flap.damping"),
            mucosal_delay_s: 0.0,
            contact_stiffness_ratio: spec.get("flap.contact_stiffness"),
            contact_damping_ratio: spec.get("flap.contact_damping"),
            rest_half_gap_open_m: spec.get("flap.gap_mm") * 1.0e-3,
            rest_half_gap_closed_m: 1.0e-15,
            min_tuning_hz: tuning * 0.5,
            max_tuning_hz: tuning * 2.0,
            initial_tuning_hz: tuning,
            initial_adduction: 0.0,
            max_supply_pressure_pa: 1.0,
            sag_fraction: 0.1,
            air_viscosity_pa_s: AIR_VISCOSITY_PA_S,
        }
    } else {
        // Sealed: a 1 kg plate on a 2 kHz spring, adducted to a 1e-15 m gap
        // (slit area 2.8e-17 m2 < the valve's 1e-14 cutoff: flow exactly 0).
        FlowValveSpec {
            tube: 0,
            section: 0,
            length_m: 0.014,
            thickness_m: 0.003,
            mass_kg: 1.0,
            damping_ratio: 1.0,
            mucosal_delay_s: 0.0,
            contact_stiffness_ratio: 1.0,
            contact_damping_ratio: 1.0,
            rest_half_gap_open_m: 1.0e-3,
            rest_half_gap_closed_m: 1.0e-15,
            min_tuning_hz: 1000.0,
            max_tuning_hz: 4000.0,
            initial_tuning_hz: 2000.0,
            initial_adduction: 1.0,
            max_supply_pressure_pa: 1.0,
            sag_fraction: 0.1,
            air_viscosity_pa_s: AIR_VISCOSITY_PA_S,
        }
    }
}

fn aero(spec: &BodySpec) -> Option<AeroSpec> {
    spec.on("air.on").then(|| {
        let onset = spec.get("air.reynolds_onset");
        AeroSpec {
            loss_scale: spec.get("air.jet_loss"),
            dipole_efficiency: spec.get("air.turbulence"),
            reference_mach: 0.1,
            max_efficiency: 0.05,
            reynolds_onset: onset,
            reynolds_full: spec.get("air.reynolds_full").max(onset * 1.05 + 1.0),
            kinematic_viscosity_m2_s: AIR_VISCOSITY_PA_S / AIR_DENSITY_KG_M3,
            obstacle_offset: spec.get_usize("air.obstacle_offset").max(1),
            noise_seed: 0x9E37_79B9_7F4A_7C15,
            dc_wall_loss: true,
        }
    })
}

/// The air body's spec.
pub fn air_body(spec: &BodySpec, mouth_initial: f64) -> Result<(SpaceBodySpec, Layout), String> {
    let (network, layout) = network(spec, mouth_initial)?;
    let body = SpaceBodySpec {
        anatomy_name: format!("ovillo-body/{}", spec.name),
        configuration_space: ConfigurationSpaceSpec {
            // The live range of the mouth: down to 0.3 (or to where the
            // preset starts, if lower). The planner checks the band at the
            // corners of this box; a nearly shut mouth pushes the 24th mode
            // up, so short bodies would be refused for a closure they never use.
            coordinates: vec![ConfigurationCoordinate {
                name: "mouth_open".into(),
                lower: mouth_initial.clamp(0.02, 0.3),
                upper: 1.0,
                initial: mouth_initial.clamp(0.02, 1.0),
            }],
            map: ConfigurationMapSpec::TubeNetwork { network },
            connection: ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate },
        },
        laws: SpaceBodyLaws::TubeNetwork { valve: valve(spec), aero: aero(spec) },
        reservoir: ReservoirSpec { capacity: 1.0 },
    };
    Ok((body, layout))
}

/// Partial series of the resonator kinds, as ratios to the first (24 of them).
pub fn series(kind: usize, inharmonicity: f64) -> (&'static str, [f64; MODE_COUNT]) {
    let mut r = [0.0; MODE_COUNT];
    match kind {
        1 => {
            // Free-free Euler-Bernoulli bar: f_n ~ beta_n^2, beta_n L = 4.730, 7.853, 10.996, then (2n+1) pi/2.
            let beta = |n: usize| match n {
                1 => 4.730_04,
                2 => 7.853_20,
                3 => 10.995_6,
                4 => 14.137_2,
                m => (2.0 * m as f64 + 1.0) * PI / 2.0,
            };
            for (i, v) in r.iter_mut().enumerate() {
                *v = (beta(i + 1) / beta(1)).powi(2);
            }
            ("bar", r)
        }
        2 => {
            // Circular membrane: sorted zeros j_mn of the Bessel functions, over 2.4048.
            const ZEROS: [f64; MODE_COUNT] = [
                2.4048, 3.8317, 5.1356, 5.5201, 6.3802, 7.0156, 7.5883, 8.4172, 8.6537, 8.7715, 9.7610, 9.9361,
                10.1735, 11.0647, 11.0864, 11.6198, 11.7915, 12.2251, 12.3386, 13.0152, 13.3237, 13.3543, 13.5893,
                14.3725,
            ];
            for (i, v) in r.iter_mut().enumerate() {
                *v = ZEROS[i] / ZEROS[0];
            }
            ("membrane", r)
        }
        3 => {
            // Simply supported rectangular plate, sides 1 : 1.3: f_mn ~ m^2 + (n/1.3)^2.
            let mut all = Vec::new();
            for m in 1..=8 {
                for n in 1..=8 {
                    all.push((m * m) as f64 + (n as f64 / 1.3).powi(2));
                }
            }
            all.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for (i, v) in r.iter_mut().enumerate() {
                *v = all[i] / all[0];
            }
            ("plate", r)
        }
        4 => {
            // A bell's partials, prime = 1 (hum, prime, tierce, quint, nominal,
            // ...): a declared design profile, not a measurement of any bell.
            const BELL: [f64; MODE_COUNT] = [
                0.5, 1.0, 1.2, 1.5, 2.0, 2.5, 2.67, 3.0, 3.33, 4.0, 4.5, 5.0, 5.33, 6.0, 6.67, 7.0, 8.0, 9.0, 10.0,
                11.0, 12.0, 13.33, 14.67, 16.0,
            ];
            r = BELL;
            ("bell", r)
        }
        _ => {
            for (i, v) in r.iter_mut().enumerate() {
                let n = (i + 1) as f64;
                *v = n * (1.0 + inharmonicity * n * n).sqrt();
            }
            ("string", r)
        }
    }
}

/// A passive modal resonator: one anchor repeated on a one-axis map.
fn resonator_body(spec: &BodySpec, k: usize, template: &WorldConfiguration, rate: f64) -> (SpaceBodySpec, Radiator, [f64; MODE_COUNT]) {
    let p = |name: &str| spec.get(&format!("res{k}.{name}"));
    let kind = spec.get_usize(&format!("res{k}.kind"));
    let (kind_name, ratios) = series(kind, p("inharmonicity"));
    let f1 = p("f1_hz");
    let t60 = p("t60_s");
    let slope = p("t60_slope");
    let rad = p("radiation_per_s");
    let rad_slope = p("radiation_slope");
    let touch = p("touch");
    let band = 0.47 * rate;
    let mut frequency_hz = [0.0; MODE_COUNT];
    let mut t60_seconds = [0.0; MODE_COUNT];
    let mut rates = [0.0; MODE_COUNT];
    let mut direction = [0.0; MODE_COUNT];
    let mut radiation_direction = [0.0; MODE_COUNT];
    for i in 0..MODE_COUNT {
        let f = f1 * ratios[i];
        if f < band {
            frequency_hz[i] = f;
            t60_seconds[i] = (t60 / ratios[i].powf(slope)).max(1.0e-3);
            rates[i] = rad * ratios[i].powf(rad_slope);
            direction[i] = ((i as f64 + 1.0) * PI * touch).sin();
            radiation_direction[i] = 1.0;
        } else {
            // Above the band: inert slots (no touch, no radiation, dead fast).
            frequency_hz[i] = band;
            t60_seconds[i] = 1.0e-3;
        }
    }
    if direction.iter().all(|v| v.abs() < 1.0e-9) {
        direction[0] = 1.0;
    }
    if radiation_direction.iter().all(|v| *v == 0.0) {
        radiation_direction[0] = 1.0;
    }
    let anchor = WorldConfiguration {
        metric: MetricSpec { frequency_hz, t60_seconds },
        aperture: template.aperture,
        radiation_direction,
    };
    let body = SpaceBodySpec {
        anatomy_name: format!("ovillo-body/res{k}/{kind_name}"),
        configuration_space: ConfigurationSpaceSpec {
            coordinates: vec![ConfigurationCoordinate { name: "fixed".into(), lower: 0.0, upper: 1.0, initial: 0.0 }],
            map: ConfigurationMapSpec::Multilinear { axes: vec![vec![0.0, 1.0]], anchors: vec![anchor.clone(), anchor] },
            connection: ConnectionPolicy::ModalCoordinates,
        },
        laws: SpaceBodyLaws::Passive { damper: None, radiation_rates_per_s: Some(rates) },
        reservoir: ReservoirSpec { capacity: 1.0e-3 },
    };
    let radiator = Radiator {
        label: format!("res{k}"),
        body: 0,
        x_m: p("x_m"),
        level: p("level"),
        touches_node: 0,
        coupling_hz: p("coupling_hz"),
        powered: false,
        frequency_hz: frequency_hz.to_vec(),
        t60_seconds: t60_seconds.to_vec(),
        kind: kind_name.into(),
    };
    (body, radiator, direction)
}

/// Which air volume a resonator or the powered material touches.
fn touch_node(spec: &BodySpec, prefix: &str, layout: &Layout, network: &TubeNetworkSpec) -> usize {
    let tube = spec.get_usize(&format!("{prefix}.couple_tube"));
    let pos = spec.get(&format!("{prefix}.couple_position"));
    let tube_index = if tube == 0 { 0 } else { layout.branch_tubes.get(tube - 1).copied().flatten().unwrap_or(0) };
    let sections = network.tubes[tube_index].base_log_area_m2.len();
    network.node(tube_index, ((pos * (sections - 1) as f64).round() as usize).min(sections - 1))
}

fn unit(mut v: [f64; MODE_COUNT]) -> [f64; MODE_COUNT] {
    let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm > 1.0e-12 {
        for x in &mut v {
            *x /= norm;
        }
    } else {
        v[0] = 1.0;
    }
    v
}

/// The whole plan and its layout.
pub fn plan(spec: &BodySpec, rate: f64, mouth_initial: f64) -> Result<(ConstitutiveWorldPlan, Layout), String> {
    let (air, mut layout) = air_body(spec, mouth_initial)?;
    let network = match &air.configuration_space.map {
        ConfigurationMapSpec::TubeNetwork { network } => network.clone(),
        _ => unreachable!(),
    };
    // The air's mode shapes at the volumes other bodies and the room touch.
    let probe = SpaceBody::try_new(rate, air.clone()).map_err(|e| describe_error(&e.to_string()))?;
    let template = WorldPlan::string_brass_v1(rate);
    let mut space_port = template.bodies[0].space_port;
    let mouth_shape = probe.node_shape(layout.mouth_node).ok_or("no mouth volume")?;
    let port = spec.get("room.port");
    for course in 0..mus_ariadne::SPACE_COURSES {
        space_port.body_directions[course] = unit(mouth_shape);
        space_port.coupling_weights[course] *= port;
    }
    let inert_port = SpacePortSpec { body_directions: template.bodies[0].space_port.body_directions, coupling_weights: [0.0; 8] };
    let mut bodies = vec![ConstitutiveBodyInstanceSpec {
        id: "air".into(),
        constitution: BodyConstitutionSpec::ModalSpace { body: air },
        space_port,
        pose: Default::default(),
    }];
    let mut couplings = Vec::new();
    let anchor = &template.bodies[0].body.configuration_path.left;
    for k in 1..=RESONATORS {
        if !spec.on(&format!("res{k}.on")) {
            continue;
        }
        let (body, mut radiator, direction) = resonator_body(spec, k, anchor, rate);
        let node = touch_node(spec, &format!("res{k}"), &layout, &network);
        let shape = probe.node_shape(node).ok_or("resonator touches a volume outside the air")?;
        radiator.body = bodies.len();
        radiator.touches_node = node;
        couplings.push(DirectCouplingSpec {
            id: format!("air-res{k}"),
            left_body: "air".into(),
            right_body: format!("res{k}"),
            left_direction: unit(shape),
            right_direction: unit(direction),
            rate_hz: radiator.coupling_hz.min(0.25 * rate),
            weight: 1.0,
        });
        bodies.push(ConstitutiveBodyInstanceSpec {
            id: format!("res{k}"),
            constitution: BodyConstitutionSpec::ModalSpace { body },
            space_port: inert_port.clone(),
            pose: Default::default(),
        });
        layout.radiators.push(radiator);
    }
    if spec.on("powered.on") {
        let node = touch_node(spec, "powered", &layout, &network);
        let shape = probe.node_shape(node).ok_or("the powered material touches a volume outside the air")?;
        let body = template.bodies[0].body.clone();
        let coupling_hz = spec.get("powered.coupling_hz");
        couplings.push(DirectCouplingSpec {
            id: "air-powered".into(),
            left_body: "air".into(),
            right_body: "powered".into(),
            left_direction: unit(shape),
            right_direction: unit(body.source_direction),
            rate_hz: coupling_hz.min(0.25 * rate),
            weight: 1.0,
        });
        let left = &body.configuration_path.left.metric;
        layout.radiators.push(Radiator {
            label: "powered".into(),
            body: bodies.len(),
            x_m: spec.get("powered.x_m"),
            level: spec.get("powered.level"),
            touches_node: node,
            coupling_hz,
            powered: true,
            frequency_hz: left.frequency_hz.to_vec(),
            t60_seconds: left.t60_seconds.to_vec(),
            kind: "string-brass material (canonical shared-wave-material-v1)".into(),
        });
        bodies.push(ConstitutiveBodyInstanceSpec {
            id: "powered".into(),
            constitution: BodyConstitutionSpec::Modal { body },
            space_port: inert_port.clone(),
            pose: Default::default(),
        });
    }
    let plan = ConstitutiveWorldPlan {
        schema_version: WORLD_PLAN_V3_SCHEMA_VERSION.to_owned(),
        world_name: format!("ovillo-body-{}", spec.name.replace(|c: char| !c.is_ascii_alphanumeric(), "-")),
        internal_sample_rate_hz: rate,
        bodies,
        shared_space: template.shared_space,
        direct_couplings: couplings,
    };
    Ok((plan, layout))
}

/// A plan error, with the advice the surface can give.
pub fn describe_error(message: &str) -> String {
    if message.contains("tube-network-band") {
        format!(
            "{message}\n  The World keeps the 24 lowest modes of the air and needs all of them below 0.48 x 48 kHz: \
             the network is too short or too simple. Lengthen bore.length_m (a closed-open pipe needs about 0.18 m \
             alone), add a side branch, or add sections to a branch."
        )
    } else {
        message.to_owned()
    }
}
