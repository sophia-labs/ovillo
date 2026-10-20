//! The tract's anatomy: every number the World is built from, with where it
//! came from.
//!
//! Labels follow `SPEC-ARIADNE-STEEL-STRING.md` ("a derivation receipt labels
//! every number as manufacturer, fit or design-hypothesis"), with two more:
//!
//! - `law`: follows from physics or from another number here, no choice made;
//! - `spec`: copied from a file in this repository, cited by path;
//! - `design-hypothesis`: chosen here, plausible, not measured, not fitted;
//! - `fit`: tuned against a stated target, with the fit's own receipt.
//!
//! The area-function family is GARGANTA's: `mus_analysis/garganta_anatomy.py`
//! `neutral_log_area` (all static coefficients zero, warp 1, lip scale 1) is the
//! neutral tract, and its positive area cone `[0.025, 18] cm²`
//! (`area_function_cm2`) bounds every section. The tongue is a movable version
//! of GARGANTA's two tongue modes (`articulation_basis`: `tongue_back` centred
//! at 0.43, `tongue_front` at 0.72); position and depth along that family are
//! this crate's controls.

/// Speed of sound, `mus_analysis/garganta_anatomy.py` `SPEED_OF_SOUND_M_S`.
pub const SPEED_OF_SOUND_M_S: f64 = 343.0;
/// Air density, `mus_analysis/garganta_anatomy.py` `AIR_DENSITY_KG_M3`.
pub const AIR_DENSITY_KG_M3: f64 = 1.204;
/// Dynamic viscosity of air near 20 °C: a physical constant, not found in this
/// repository (labelled as such in the receipt). Sets the glottal slit's
/// viscous resistance `12 mu d l² / a³` (plane Poiseuille flow in a slit).
pub const AIR_VISCOSITY_PA_S: f64 = 1.8e-5;
/// GARGANTA's positive area cone, lower edge (`area_function_cm2`): near
/// closure without an exactly singular tube. Exact closure is G2's rank change
/// and is not implemented here.
pub const AREA_FLOOR_CM2: f64 = 0.025;
/// GARGANTA's positive area cone, upper edge.
pub const AREA_CEIL_CM2: f64 = 18.0;
/// Fixed capacity of the oral tube (sections). No allocation after `new`.
pub const MAX_ORAL_SECTIONS: usize = 64;
/// Fixed capacity of the nasal tube (sections).
pub const MAX_NASAL_SECTIONS: usize = 48;

/// One labelled number for the derivation receipt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Provenance {
    pub name: &'static str,
    pub value: f64,
    pub unit: &'static str,
    pub label: &'static str,
    pub source: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoldAnatomy {
    /// Vibrating length of each fold (anterior-posterior).
    pub length_m: f64,
    /// Medial-surface thickness: the face the intraglottal pressure pushes.
    pub thickness_m: f64,
    /// Effective vibrating mass of one fold.
    pub mass_kg: f64,
    /// Viscous damping as a fraction of critical.
    pub damping_ratio: f64,
    /// Lumped mucosal-wave delay between the fold's entry and exit edges.
    pub mucosal_delay_s: f64,
    /// Collision stiffness as a multiple of the tension stiffness.
    pub contact_stiffness_ratio: f64,
    /// Extra damping while in contact, fraction of critical.
    pub contact_damping_ratio: f64,
    /// Resting half-gap at tenseness 0 (breathy, abducted).
    pub rest_half_gap_breathy_m: f64,
    /// Resting half-gap at tenseness 1 (pressed, adducted).
    pub rest_half_gap_pressed_m: f64,
    /// Extra abduction below tenseness 0.3, for voiceless consonants: the
    /// rest gap grows by up to this fraction at tenseness 0.
    pub abduction_boost: f64,
    /// Fold tuning range (the pitch control sets the tension stiffness so the
    /// unloaded fold rings here; the World decides the actual F0).
    pub min_tuning_hz: f64,
    pub max_tuning_hz: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LungAnatomy {
    /// Subglottal pressure at breath = 1.
    pub max_pressure_pa: f64,
    /// Finite work reservoir (pneumatic work the lungs hold).
    pub capacity_j: f64,
    /// Work the breathing muscles put back per second, up to capacity.
    pub refill_w: f64,
    /// Below this fraction of capacity the deliverable pressure sags.
    pub sag_fraction: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TractAnatomy {
    /// Glottis-to-lips length of the oral tube.
    pub oral_length_m: f64,
    /// Velopharyngeal port to nostrils.
    pub nasal_length_m: f64,
    /// Where the nasal branch joins the oral tube (0 glottis .. 1 lips).
    pub velum_position: f64,
    /// Plane-wave attenuation in the oral tube (wall + viscothermal, lumped,
    /// frequency independent).
    pub oral_attenuation_np_per_m: f64,
    /// The nasal passage is narrower and convoluted: more loss.
    pub nasal_attenuation_np_per_m: f64,
    /// Tongue constriction centre range for tongue index 0 (back) .. 1 (front).
    pub tongue_back_center: f64,
    pub tongue_front_center: f64,
    /// Gaussian width of the tongue constriction (normalized position).
    pub tongue_width: f64,
    /// Log-area depth at diameter 0 (low, open) and diameter 1 (high, closed).
    pub tongue_depth_low: f64,
    pub tongue_depth_high: f64,
    /// Incompressible-tongue compensation: fraction of the constriction's
    /// log-area depth returned as widening in the opposite cavity.
    pub tongue_compensation: f64,
    /// Width of the compensating lobe.
    pub tongue_compensation_width: f64,
    /// Lip constriction centre and width.
    pub lip_center: f64,
    pub lip_width: f64,
    /// Lip area multiplier at lips = 1 (spread open).
    pub lip_open_scale: f64,
    /// Velopharyngeal port area at velum = 1.
    pub velum_port_max_cm2: f64,
    /// Nasal passage area at its widest and at the nostrils.
    pub nasal_peak_cm2: f64,
    pub nostril_cm2: f64,
    /// Unflanged-end inertance length as a multiple of the opening radius.
    pub end_correction_ratio: f64,
    /// Radiation conductance factor: radiating air plug's dissipative
    /// impedance as a multiple of the characteristic impedance rho c / A.
    pub radiation_impedance_ratio: f64,
    /// Largest fraction of the glottal jet's dissipated power that comes back
    /// as turbulent pressure at aspiration = 1.
    pub turbulence_efficiency: f64,
    /// Supraglottal jets: largest fraction of a constriction jet's dissipated
    /// power returned as turbulent pressure (fully turbulent jet).
    pub frication_efficiency: f64,
    /// Reynolds number (hydraulic diameter of the aperture) where constriction
    /// noise starts; it reaches full strength at twice this.
    pub frication_reynolds: f64,
    /// Lateral width of a constriction slit, for its viscous resistance.
    pub constriction_slit_width_m: f64,
    /// Gaussian shoulder width of a touch constriction (normalized position).
    pub constriction_width: f64,
    /// Constriction closing and releasing time constants.
    pub constriction_close_s: f64,
    pub constriction_release_s: f64,
    /// Below this lips value the lip aperture closes linearly to exactly zero.
    pub lip_seal: f64,
    pub fold: FoldAnatomy,
    pub lung: LungAnatomy,
}

impl Default for TractAnatomy {
    fn default() -> Self {
        Self::adult_v0()
    }
}

impl TractAnatomy {
    /// The one anatomy this crate ships (`tract-v0`).
    #[must_use]
    pub fn adult_v0() -> Self {
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
            radiation_impedance_ratio: 1.4884,
            turbulence_efficiency: 0.02,
            frication_efficiency: 0.06,
            frication_reynolds: 3_000.0,
            constriction_slit_width_m: 0.015,
            constriction_width: 0.022,
            constriction_close_s: 0.008,
            constriction_release_s: 0.005,
            lip_seal: 0.02,
            fold: FoldAnatomy {
                length_m: 0.014,
                thickness_m: 0.003,
                mass_kg: 1.5e-4,
                damping_ratio: 0.10,
                mucosal_delay_s: 6.0e-4,
                contact_stiffness_ratio: 3.0,
                contact_damping_ratio: 1.0,
                rest_half_gap_breathy_m: 8.0e-4,
                abduction_boost: 1.0,
                rest_half_gap_pressed_m: 5.0e-5,
                min_tuning_hz: 60.0,
                max_tuning_hz: 420.0,
            },
            lung: LungAnatomy {
                max_pressure_pa: 1_000.0,
                capacity_j: 3.0,
                refill_w: 0.30,
                sag_fraction: 0.10,
            },
        }
    }

    /// Every number, labelled. The example writes this into its receipt.
    #[must_use]
    pub fn provenance(&self) -> Vec<Provenance> {
        const DH: &str = "design-hypothesis";
        let f = &self.fold;
        let l = &self.lung;
        vec![
            Provenance { name: "speed_of_sound", value: SPEED_OF_SOUND_M_S, unit: "m/s", label: "spec", source: "garganta_anatomy.py SPEED_OF_SOUND_M_S" },
            Provenance { name: "air_density", value: AIR_DENSITY_KG_M3, unit: "kg/m3", label: "spec", source: "garganta_anatomy.py AIR_DENSITY_KG_M3" },
            Provenance { name: "air_viscosity", value: AIR_VISCOSITY_PA_S, unit: "Pa s", label: "law", source: "physical constant near 20 C, not present in this repository; slit resistance 12 mu d l^2/a^3 is plane Poiseuille flow" },
            Provenance { name: "area_floor", value: AREA_FLOOR_CM2, unit: "cm2", label: "spec", source: "garganta_anatomy.py area_function_cm2 positive cone" },
            Provenance { name: "area_ceiling", value: AREA_CEIL_CM2, unit: "cm2", label: "spec", source: "garganta_anatomy.py area_function_cm2 positive cone" },
            Provenance { name: "neutral_area_function", value: 0.0, unit: "-", label: "spec", source: "garganta_anatomy.py neutral_log_area, coefficients 0, warp 1, lip scale 1" },
            Provenance { name: "oral_length", value: self.oral_length_m, unit: "m", label: DH, source: "inside GARGANTA's tract_length_cm cone [13.5, 21] (AnatomyGenome.from_unit_cube); section count = round(L * 2 fs / c)" },
            Provenance { name: "nasal_length", value: self.nasal_length_m, unit: "m", label: DH, source: "no nasal anatomy exists in this repository" },
            Provenance { name: "velum_position", value: self.velum_position, unit: "-", label: DH, source: "just below GARGANTA's palate bend (0.46, _warped_coordinate)" },
            Provenance { name: "oral_attenuation", value: self.oral_attenuation_np_per_m, unit: "Np/m", label: DH, source: "lumped frequency-independent; GARGANTA's oracle uses wall_loss*sqrt(f/1 kHz), not realizable per sample" },
            Provenance { name: "nasal_attenuation", value: self.nasal_attenuation_np_per_m, unit: "Np/m", label: DH, source: "nasal passage lossier than oral" },
            Provenance { name: "tongue_back_center", value: self.tongue_back_center, unit: "-", label: DH, source: "extends GARGANTA tongue_back (0.43) toward the pharynx" },
            Provenance { name: "tongue_front_center", value: self.tongue_front_center, unit: "-", label: DH, source: "extends GARGANTA tongue_front (0.72) toward the alveolar ridge" },
            Provenance { name: "tongue_width", value: self.tongue_width, unit: "-", label: DH, source: "between GARGANTA tongue widths 0.095 and 0.11" },
            Provenance { name: "tongue_depth_low", value: self.tongue_depth_low, unit: "ln(area)", label: DH, source: "low tongue widens the constriction site" },
            Provenance { name: "tongue_depth_high", value: self.tongue_depth_high, unit: "ln(area)", label: DH, source: "reaches the area floor from the neutral 3 cm2" },
            Provenance { name: "tongue_compensation", value: self.tongue_compensation, unit: "-", label: DH, source: "GARGANTA's compensating lobes are 0.38/1.12 and 0.42/1.15 of the constriction" },
            Provenance { name: "tongue_compensation_width", value: self.tongue_compensation_width, unit: "-", label: "spec", source: "garganta_anatomy.py articulation_basis compensating lobes (width 0.16)" },
            Provenance { name: "lip_center", value: self.lip_center, unit: "-", label: DH, source: "near GARGANTA lip_round centre 0.95" },
            Provenance { name: "lip_width", value: self.lip_width, unit: "-", label: DH, source: "near GARGANTA lip_round width 0.055" },
            Provenance { name: "lip_open_scale", value: self.lip_open_scale, unit: "-", label: DH, source: "spread lips" },
            Provenance { name: "velum_port_max", value: self.velum_port_max_cm2, unit: "cm2", label: DH, source: "" },
            Provenance { name: "nasal_peak_area", value: self.nasal_peak_cm2, unit: "cm2", label: DH, source: "" },
            Provenance { name: "nostril_area", value: self.nostril_cm2, unit: "cm2", label: DH, source: "" },
            Provenance { name: "end_correction_ratio", value: self.end_correction_ratio, unit: "-", label: "spec", source: "garganta_anatomy.py _radiation_impedance low-ka reactance 0.61 ka" },
            Provenance { name: "radiation_impedance_ratio", value: self.radiation_impedance_ratio, unit: "-", label: "law", source: "0.61^2/0.25: makes a parallel air-plug mass + radiation load match the oracle's low-ka limits (0.25 (ka)^2 resistance, 0.61 ka reactance); diverges above ka~1" },
            Provenance { name: "turbulence_efficiency", value: self.turbulence_efficiency, unit: "-", label: DH, source: "stochastic supply paid by the jet (ANATOMY-EVOLUTION.md section 4 StochasticSupplySpec)" },
            Provenance { name: "frication_efficiency", value: self.frication_efficiency, unit: "-", label: DH, source: "cap; below it the share is 0.004 (M/0.1)^3 of each constriction jet's own dissipation (dipole Mach-cubed scaling, coefficient a design hypothesis)" },
            Provenance { name: "frication_reynolds", value: self.frication_reynolds, unit: "-", label: DH, source: "onset Re = 2|U| / (nu sqrt(pi A_c)), full at 2x; value not from this repository" },
            Provenance { name: "constriction_slit_width", value: self.constriction_slit_width_m, unit: "m", label: DH, source: "viscous slit resistance 12 mu dx w^2 / A_c^3 at narrow junctions" },
            Provenance { name: "constriction_width", value: self.constriction_width, unit: "-", label: DH, source: "flat top of 1.2 sections plus this Gaussian shoulder" },
            Provenance { name: "constriction_close", value: self.constriction_close_s, unit: "s", label: DH, source: "" },
            Provenance { name: "constriction_release", value: self.constriction_release_s, unit: "s", label: DH, source: "" },
            Provenance { name: "lip_seal", value: self.lip_seal, unit: "-", label: DH, source: "lip aperture goes to exactly zero below this lips value" },
            Provenance { name: "fold_length", value: f.length_m, unit: "m", label: DH, source: "" },
            Provenance { name: "fold_thickness", value: f.thickness_m, unit: "m", label: DH, source: "" },
            Provenance { name: "fold_mass", value: f.mass_kg, unit: "kg", label: DH, source: "" },
            Provenance { name: "fold_damping_ratio", value: f.damping_ratio, unit: "-", label: DH, source: "" },
            Provenance { name: "mucosal_delay", value: f.mucosal_delay_s, unit: "s", label: DH, source: "one-mass surface-wave form after Titze (1988, JASA 83:1536); value not from data" },
            Provenance { name: "contact_stiffness_ratio", value: f.contact_stiffness_ratio, unit: "-", label: DH, source: "" },
            Provenance { name: "contact_damping_ratio", value: f.contact_damping_ratio, unit: "-", label: DH, source: "" },
            Provenance { name: "rest_half_gap_breathy", value: f.rest_half_gap_breathy_m, unit: "m", label: DH, source: "" },
            Provenance { name: "abduction_boost", value: f.abduction_boost, unit: "-", label: DH, source: "voiceless consonants: rest gap x(1 + boost (0.3 - t)/0.3) below tenseness 0.3" },
            Provenance { name: "rest_half_gap_pressed", value: f.rest_half_gap_pressed_m, unit: "m", label: DH, source: "" },
            Provenance { name: "fold_tuning_min", value: f.min_tuning_hz, unit: "Hz", label: DH, source: "" },
            Provenance { name: "fold_tuning_max", value: f.max_tuning_hz, unit: "Hz", label: DH, source: "" },
            Provenance { name: "lung_max_pressure", value: l.max_pressure_pa, unit: "Pa", label: DH, source: "" },
            Provenance { name: "lung_capacity", value: l.capacity_j, unit: "J", label: DH, source: "" },
            Provenance { name: "lung_refill", value: l.refill_w, unit: "W", label: DH, source: "" },
            Provenance { name: "lung_sag_fraction", value: l.sag_fraction, unit: "-", label: DH, source: "" },
        ]
    }

    /// Oral section count at an audio sample rate (the tract steps at twice it).
    #[must_use]
    pub fn oral_sections(&self, sample_rate_hz: f64) -> usize {
        let dx = SPEED_OF_SOUND_M_S / (2.0 * sample_rate_hz);
        ((self.oral_length_m / dx).round() as usize).clamp(16, MAX_ORAL_SECTIONS)
    }

    #[must_use]
    pub fn nasal_sections(&self, sample_rate_hz: f64) -> usize {
        let dx = SPEED_OF_SOUND_M_S / (2.0 * sample_rate_hz);
        ((self.nasal_length_m / dx).round() as usize).clamp(8, MAX_NASAL_SECTIONS)
    }

    /// GARGANTA's neutral log-area (cm²) at normalized position `x`
    /// (`neutral_log_area` with zero static coefficients, warp 1).
    #[must_use]
    pub fn neutral_log_area(x: f64) -> f64 {
        3.0f64.ln() - 0.62 * gaussian(x, 0.06, 0.055) + 0.28 * gaussian(x, 0.27, 0.16)
            + 0.16 * gaussian(x, 0.68, 0.19)
            - 0.34 * gaussian(x, 0.97, 0.045)
    }

    /// Oral area function (cm²) for an articulation, written into `out`
    /// (section centres `(i + 0.5) / n`), inside GARGANTA's area cone.
    pub fn oral_areas_cm2(&self, a: &Articulation, out: &mut [f64]) {
        self.oral_areas_unclipped_cm2(a, out);
        for v in out.iter_mut() {
            *v = v.clamp(AREA_FLOOR_CM2, AREA_CEIL_CM2);
        }
    }

    /// As [`TractAnatomy::oral_areas_cm2`] but without the lower edge of the
    /// cone (the ceiling still applies): what a constriction's aperture sees.
    pub fn oral_areas_unclipped_cm2(&self, a: &Articulation, out: &mut [f64]) {
        let n = out.len();
        let center = lerp(self.tongue_back_center, self.tongue_front_center, a.tongue_index.clamp(0.0, 1.0));
        let depth = lerp(self.tongue_depth_low, self.tongue_depth_high, a.tongue_diameter.clamp(0.0, 1.0));
        let comp_center = 1.12 - center;
        let lips = a.lips.clamp(0.0, 1.0);
        let lip_factor = (lips * self.lip_open_scale).max(1.0e-3);
        let lip_log = lip_factor.ln();
        for (i, value) in out.iter_mut().enumerate() {
            let x = (i as f64 + 0.5) / n as f64;
            let mut log_area = Self::neutral_log_area(x);
            log_area -= depth * gaussian(x, center, self.tongue_width);
            log_area += self.tongue_compensation * depth * gaussian(x, comp_center, self.tongue_compensation_width);
            log_area += lip_log * gaussian(x, self.lip_center, self.lip_width);
            *value = log_area.exp().min(AREA_CEIL_CM2);
        }
    }

    /// Nasal area function (cm²): the first section is the velopharyngeal
    /// port, the rest a passage widening and narrowing to the nostrils.
    pub fn nasal_areas_cm2(&self, velum: f64, out: &mut [f64]) {
        let m = out.len();
        let port = (velum.clamp(0.0, 1.0) * self.velum_port_max_cm2).max(AREA_FLOOR_CM2);
        for (k, value) in out.iter_mut().enumerate() {
            if k == 0 {
                *value = port;
                continue;
            }
            let x = k as f64 / (m - 1) as f64;
            let body = (std::f64::consts::PI * x).sin().max(0.0).powf(0.7);
            let tail = self.nostril_cm2 * x;
            *value = (0.6 * (1.0 - x) + tail + (self.nasal_peak_cm2 - 0.6) * body).clamp(AREA_FLOOR_CM2, AREA_CEIL_CM2);
        }
    }
}

/// Where the articulators are. Each in 0..1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Articulation {
    /// 0 back (pharyngeal) .. 1 front (alveolar).
    pub tongue_index: f64,
    /// 0 low (open) .. 1 high (closed).
    pub tongue_diameter: f64,
    /// 0 closed .. 1 spread open.
    pub lips: f64,
    /// 0 closed .. 1 open.
    pub velum: f64,
}

impl Default for Articulation {
    fn default() -> Self {
        Self { tongue_index: 0.5, tongue_diameter: 0.3, lips: 0.66, velum: 0.0 }
    }
}

#[inline]
pub(crate) fn gaussian(x: f64, center: f64, width: f64) -> f64 {
    let z = (x - center) / width;
    (-0.5 * z * z).exp()
}

#[inline]
pub(crate) fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
