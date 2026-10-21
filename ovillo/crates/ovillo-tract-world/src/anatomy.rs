//! The tract's anatomy, lowered into one `mus_ariadne` plan.
//!
//! Every number below is copied from `ovillo/crates/ovillo-tract/src/anatomy.rs`
//! (`TractAnatomy::adult_v0`, `neutral_log_area`, `oral_areas_cm2`,
//! `nasal_areas_cm2`), which labels each one (`spec`, `law`,
//! `design-hypothesis`) and cites GARGANTA where it comes from there. Nothing
//! is re-fitted here. What this crate changes is the *physics that consumes
//! them*: instead of a Kelly-Lochbaum waveguide stepped at twice the audio
//! rate, the area function defines a branched lumped tube network whose 24
//! lowest modes `mus_ariadne` computes at every configuration point
//! (`configuration::network`), and the World is a `mus_ariadne::World`.
//!
//! The four configuration coordinates are the tube tract's articulation:
//! `tongue_index`, `tongue_diameter`, `lips`, `velum`, each in `[0, 1]`.

use mus_ariadne::{
    AffineSpec, AreaTermSpec, BodyConstitutionSpec, ConfigurationCoordinate, ConfigurationMapSpec,
    ConfigurationSpaceSpec, ConnectionPolicy, ConstitutiveBodyInstanceSpec, ConstitutiveWorldPlan,
    FlowValveSpec, RadiatingEndSpec, ReservoirSpec, SpaceBodyLaws, SpaceBodySpec, TubeBranchSpec,
    TubeNetworkSpec, TubeSpec, WorldPlan, WORLD_PLAN_V3_SCHEMA_VERSION,
};

/// `ovillo-tract` anatomy constants (`anatomy.rs`), copied.
pub const SPEED_OF_SOUND_M_S: f64 = 343.0;
pub const AIR_DENSITY_KG_M3: f64 = 1.204;
pub const AIR_VISCOSITY_PA_S: f64 = 1.8e-5;
pub const AREA_FLOOR_CM2: f64 = 0.025;
pub const AREA_CEIL_CM2: f64 = 18.0;

/// One labelled number, as `ovillo-tract`'s derivation receipt has them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Provenance {
    pub name: &'static str,
    pub value: f64,
    pub unit: &'static str,
    pub label: &'static str,
    pub source: &'static str,
}

/// The anatomy and its discretization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TractNdAnatomy {
    pub oral_length_m: f64,
    pub nasal_length_m: f64,
    pub velum_position: f64,
    pub oral_attenuation_np_per_m: f64,
    pub nasal_attenuation_np_per_m: f64,
    pub tongue_back_center: f64,
    pub tongue_front_center: f64,
    pub tongue_width: f64,
    pub tongue_depth_low: f64,
    pub tongue_depth_high: f64,
    pub tongue_compensation: f64,
    pub tongue_compensation_width: f64,
    pub lip_center: f64,
    pub lip_width: f64,
    pub lip_open_scale: f64,
    pub velum_port_max_cm2: f64,
    pub nasal_peak_cm2: f64,
    pub nostril_cm2: f64,
    pub end_correction_ratio: f64,
    /// Network volumes per tube (this crate's choice; the modes are computed
    /// on this grid, so its dispersion is the grid's).
    pub oral_sections: usize,
    pub nasal_sections: usize,
    pub valve: FlowValveSpec,
    pub lung_capacity_j: f64,
    pub lung_refill_w: f64,
}

impl Default for TractNdAnatomy {
    fn default() -> Self {
        Self::adult_v0(44, 31)
    }
}

impl TractNdAnatomy {
    /// `ovillo-tract`'s `adult_v0` on a network of `oral` + `nasal` volumes.
    /// `(44, 31)` is the tube tract's own section count at 48 kHz.
    #[must_use]
    pub fn adult_v0(oral: usize, nasal: usize) -> Self {
        Self {
            oral_length_m: 0.157,
            nasal_length_m: 0.11,
            velum_position: 0.40,
            oral_attenuation_np_per_m: 0.45,
            nasal_attenuation_np_per_m: 1.6,
            tongue_back_center: 0.24,
            tongue_front_center: 0.80,
            tongue_width: 0.10,
            tongue_depth_low: -0.9,
            tongue_depth_high: 3.2,
            tongue_compensation: 0.36,
            tongue_compensation_width: 0.16,
            lip_center: 0.965,
            lip_width: 0.045,
            lip_open_scale: 1.5,
            velum_port_max_cm2: 1.0,
            nasal_peak_cm2: 1.8,
            nostril_cm2: 0.7,
            end_correction_ratio: 0.61,
            oral_sections: oral,
            nasal_sections: nasal,
            valve: FlowValveSpec {
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
                air_viscosity_pa_s: AIR_VISCOSITY_PA_S,
            },
            lung_capacity_j: 3.0,
            lung_refill_w: 0.30,
        }
    }

    /// GARGANTA's neutral log-area (cm²) as `ovillo-tract` writes it.
    #[must_use]
    pub fn neutral_log_area_cm2(x: f64) -> f64 {
        3.0f64.ln() - 0.62 * gaussian(x, 0.06, 0.055) + 0.28 * gaussian(x, 0.27, 0.16)
            + 0.16 * gaussian(x, 0.68, 0.19)
            - 0.34 * gaussian(x, 0.97, 0.045)
    }

    /// `ovillo-tract`'s nasal passage (cm²) at section `k` of `m` (`k >= 1`).
    #[must_use]
    pub fn nasal_area_cm2(&self, k: usize, m: usize) -> f64 {
        let x = k as f64 / (m - 1) as f64;
        let body = (std::f64::consts::PI * x).sin().max(0.0).powf(0.7);
        let tail = self.nostril_cm2 * x;
        (0.6 * (1.0 - x) + tail + (self.nasal_peak_cm2 - 0.6) * body).clamp(AREA_FLOOR_CM2, AREA_CEIL_CM2)
    }

    /// The oral volume the nasal branch joins (`ovillo-tract`'s junction
    /// `jv - 1 | jv`; the network attaches at volume `jv`).
    #[must_use]
    pub fn velum_section(&self) -> usize {
        ((self.velum_position * self.oral_sections as f64).round() as usize).clamp(2, self.oral_sections - 2)
    }

    /// The tube network: oral tube (glottis closed at volume 0, lips
    /// radiating), nasal branch (port area = `velum * port_max`, nostrils
    /// radiating), and the articulation as area terms on the oral tube.
    #[must_use]
    pub fn network(&self) -> TubeNetworkSpec {
        let n = self.oral_sections;
        let m = self.nasal_sections;
        let oral_base: Vec<f64> = (0..n)
            .map(|i| Self::neutral_log_area_cm2((i as f64 + 0.5) / n as f64) + (1.0e-4f64).ln())
            .collect();
        let nasal_base: Vec<f64> = (0..m)
            .map(|k| {
                let cm2 = if k == 0 { self.velum_port_max_cm2 } else { self.nasal_area_cm2(k, m) };
                (cm2 * 1.0e-4).ln()
            })
            .collect();
        let span = self.tongue_front_center - self.tongue_back_center;
        let depth_span = self.tongue_depth_high - self.tongue_depth_low;
        // Coordinates: 0 tongue_index, 1 tongue_diameter, 2 lips, 3 velum.
        let area_terms = vec![
            // ln A -= depth * g(s; center, width); depth = lerp(low, high, td),
            // center = lerp(back, front, ti).
            AreaTermSpec::Gaussian {
                tube: 0,
                amplitude: AffineSpec { constant: -self.tongue_depth_low, terms: vec![(1, -depth_span)] },
                center: AffineSpec { constant: self.tongue_back_center, terms: vec![(0, span)] },
                width: self.tongue_width,
            },
            // ln A += comp * depth * g(s; 1.12 - center, comp_width).
            AreaTermSpec::Gaussian {
                tube: 0,
                amplitude: AffineSpec {
                    constant: self.tongue_compensation * self.tongue_depth_low,
                    terms: vec![(1, self.tongue_compensation * depth_span)],
                },
                center: AffineSpec { constant: 1.12 - self.tongue_back_center, terms: vec![(0, -span)] },
                width: self.tongue_compensation_width,
            },
            // ln A += ln(max(lips * scale, 1e-3)) * g(s; lip_center, lip_width).
            AreaTermSpec::LogScaleGaussian {
                tube: 0,
                scale: AffineSpec { constant: 0.0, terms: vec![(2, self.lip_open_scale)] },
                floor: 1.0e-3,
                center: self.lip_center,
                width: self.lip_width,
            },
            // Velopharyngeal port: max(velum * port_max, floor).
            AreaTermSpec::SectionArea {
                tube: 1,
                section: 0,
                area_m2: AffineSpec { constant: 0.0, terms: vec![(3, self.velum_port_max_cm2 * 1.0e-4)] },
            },
        ];
        TubeNetworkSpec {
            speed_of_sound_m_s: SPEED_OF_SOUND_M_S,
            air_density_kg_m3: AIR_DENSITY_KG_M3,
            area_floor_m2: AREA_FLOOR_CM2 * 1.0e-4,
            area_ceiling_m2: AREA_CEIL_CM2 * 1.0e-4,
            tubes: vec![
                TubeSpec {
                    id: "oral".into(),
                    length_m: self.oral_length_m,
                    base_log_area_m2: oral_base,
                    attenuation_np_per_m: self.oral_attenuation_np_per_m,
                },
                TubeSpec {
                    id: "nasal".into(),
                    length_m: self.nasal_length_m,
                    base_log_area_m2: nasal_base,
                    attenuation_np_per_m: self.nasal_attenuation_np_per_m,
                },
            ],
            branches: vec![TubeBranchSpec { trunk: 0, trunk_section: self.velum_section(), branch: 1 }],
            area_terms,
            radiating_ends: vec![
                RadiatingEndSpec { tube: 0, end_correction_ratio: self.end_correction_ratio },
                RadiatingEndSpec { tube: 1, end_correction_ratio: self.end_correction_ratio },
            ],
        }
    }

    /// One-body `ariadne-world-plan-v3`. The shared space is the canonical
    /// one with the tract's port weights at zero (disconnected exactly).
    #[must_use]
    pub fn plan(&self, sample_rate_hz: f64, connection: ConnectionPolicy, initial: [f64; 4]) -> ConstitutiveWorldPlan {
        let template = WorldPlan::string_brass_v1(sample_rate_hz);
        let mut space_port = template.bodies[0].space_port;
        space_port.coupling_weights = [0.0; 8];
        let names = ["tongue_index", "tongue_diameter", "lips", "velum"];
        let coordinates = names
            .iter()
            .zip(initial)
            .map(|(name, value)| ConfigurationCoordinate { name: (*name).into(), lower: 0.0, upper: 1.0, initial: value.clamp(0.0, 1.0) })
            .collect();
        ConstitutiveWorldPlan {
            schema_version: WORLD_PLAN_V3_SCHEMA_VERSION.to_owned(),
            world_name: "ovillo-tract-world-v0".into(),
            internal_sample_rate_hz: sample_rate_hz,
            bodies: vec![ConstitutiveBodyInstanceSpec {
                id: "tract".into(),
                constitution: BodyConstitutionSpec::ModalSpace {
                    body: SpaceBodySpec {
                        anatomy_name: "tract-nd-v0".into(),
                        configuration_space: ConfigurationSpaceSpec {
                            coordinates,
                            map: ConfigurationMapSpec::TubeNetwork { network: self.network() },
                            connection,
                        },
                        laws: SpaceBodyLaws::TubeNetwork { valve: self.valve.clone() },
                        reservoir: ReservoirSpec { capacity: self.lung_capacity_j },
                    },
                },
                space_port,
                pose: Default::default(),
            }],
            shared_space: template.shared_space,
            direct_couplings: Vec::new(),
        }
    }

    /// Where every number came from. Labels as `ovillo-tract`'s receipt, plus
    /// this crate's own choices.
    #[must_use]
    pub fn provenance(&self) -> Vec<Provenance> {
        const COPIED: &str = "ovillo/crates/ovillo-tract/src/anatomy.rs TractAnatomy::adult_v0 (labelled there)";
        vec![
            Provenance { name: "area_function_family", value: 0.0, unit: "-", label: "spec", source: "ovillo-tract anatomy.rs oral_areas_cm2/nasal_areas_cm2 (GARGANTA neutral_log_area + tongue/lip terms), lowered to AreaTermSpec; identical formulas" },
            Provenance { name: "every_anatomy_number", value: 0.0, unit: "-", label: "copied", source: COPIED },
            Provenance { name: "oral_sections", value: self.oral_sections as f64, unit: "-", label: "design-hypothesis", source: "this crate: network grid; 44 = the tube tract's section count at 48 kHz" },
            Provenance { name: "nasal_sections", value: self.nasal_sections as f64, unit: "-", label: "design-hypothesis", source: "this crate: network grid; 31 = the tube tract's" },
            Provenance { name: "modes", value: 24.0, unit: "-", label: "law", source: "mus_ariadne MODE_COUNT: lowest 24 solutions of K phi = w^2 C phi on the network" },
            Provenance { name: "radiation_conductance", value: 0.0, unit: "-", label: "law", source: "G = A r^2 / (4 rho c l^2), l = dx/2 + 0.61 r: the parallel (end column, G) load's low-ka resistance equals GARGANTA _radiation_impedance's (rho c/A)(ka)^2/4" },
            Provenance { name: "lung_refill", value: self.lung_refill_w, unit: "W", label: "design-hypothesis", source: COPIED },
            Provenance { name: "lung_capacity", value: self.lung_capacity_j, unit: "J", label: "design-hypothesis", source: COPIED },
        ]
    }
}

#[inline]
pub(crate) fn gaussian(x: f64, center: f64, width: f64) -> f64 {
    let z = (x - center) / width;
    (-0.5 * z * z).exp()
}
