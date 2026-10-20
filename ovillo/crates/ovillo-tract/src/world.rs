//! The tract World: one persistent physical state, one material clock, one
//! global ledger.
//!
//! ```text
//! lung reservoir (finite work, refilled by breathing muscles)
//!   -> fold valve: mass + tension spring + collision, pushed by the
//!      intraglottal pressure (one-mass surface-wave form)
//!   -> glottal jet: viscous + Bernoulli slit flow between lung pressure and the tract's
//!      own pressure at the glottis (source-tract interaction is not added,
//!      it is the boundary condition)
//!   -> oral tube: Kelly-Lochbaum sections in pressure waves, admittance-
//!      weighted scattering; at a narrowing the junction is a series flow
//!      element (Borda-Carnot expansion + viscous slit) with an aperture that
//!      can go to exactly zero, and a Reynolds-gated turbulence source paid
//!      by that jet's own loss; uniform plane-wave loss
//!   -> velum: a three-port junction whose nasal branch passes through the
//!      velopharyngeal aperture (exactly closed at velum 0)
//!   -> lips and nostrils: radiating air plugs (stored end mass + a
//!      radiation port that removes what it emits, once)
//!   -> listener: reads the emitted radiation pressure, never loads it
//! ```
//!
//! Stored energy: every travelling-wave sample `w` in a section with
//! admittance `Y = A / (rho c)` holds `Y w² dt`; each air plug holds `Y_m s² dt`;
//! the folds hold `½ m v² + ½ k (y - y_rest)² + ½ k_c min(y, 0)²`; the lungs
//! hold their reservoir. The configuration (areas, fold tension, adduction) is
//! moved with the state held fixed (each section's excess air volume and
//! volume flow; the folds' displacement and momentum) and the change of
//! stored energy is booked as control work, the connection Ariadne uses for
//! frets and slides (`mus_ariadne::Body::set_configuration_amount`).

use crate::anatomy::{Articulation, TractAnatomy, AIR_DENSITY_KG_M3, AIR_VISCOSITY_PA_S, MAX_NASAL_SECTIONS, MAX_ORAL_SECTIONS, SPEED_OF_SOUND_M_S};
use crate::presets::Gesture;
use mus_watsn::{WorkFrame, WorkLedger};
use std::f64::consts::{PI, TAU};

/// Samples between area-function evaluations; areas move linearly between.
pub const AREA_STRIDE: usize = 16;
/// Points in the stroboscope cycle snapshot.
pub const CYCLE_POINTS: usize = 32;
const CYCLE_RING: usize = 2048;
/// Simultaneous orb dents the World remembers.
pub const MAX_IMPACTS: usize = 6;
/// Orb knock: raised-cosine pressure pulse length (audio samples, 0.5 ms at 48 kHz).
const PULSE_SAMPLES: usize = 24;
/// Wall wobble after a hit: frequency, decay, width (design hypotheses).
const WOBBLE_HZ: f64 = 7.0;
const WOBBLE_DECAY_S: f64 = 0.22;
const DENT_WIDTH: f64 = 0.06;
/// Peak log-area dent per sqrt(mJ) of orb energy, capped.
const DENT_PER_SQRT_MJ: f64 = 0.55;
const DENT_MAX: f64 = 1.4;
const NEWTON_STEPS: usize = 8;
/// Constriction slots: slot 0 belongs to gestures, the rest to touches.
pub const MAX_CONSTRICTIONS: usize = 5;
/// Below this aperture (m²) a junction carries no flow at all.
const CLOSED_M2: f64 = 1.0e-14;
/// Junctions narrower than this (m²) carry viscous slit resistance.
const VISCOUS_AREA_M2: f64 = 0.25e-4;
/// Kinematic viscosity of air (mu / rho).
const NU: f64 = crate::anatomy::AIR_VISCOSITY_PA_S / AIR_DENSITY_KG_M3;
/// Corner of the turbulence source spectrum (design hypothesis).
const TURBULENCE_CORNER_HZ: f64 = 4_000.0;
/// Turbulent jets paid out per step, at most.
const MAX_NOISE: usize = 8;

/// A constriction placed anywhere along the oral tract: `position` 0 glottis
/// .. 1 lips, `degree` 0 none .. 1 closed (aperture exactly zero).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Constriction {
    pub position: f64,
    pub degree: f64,
}

/// Fold-tension modulation: vibrato (rate, depth in semitones) and a slow
/// irregular wobble (0..1, at most 0.35 semitone).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vibrato {
    pub rate_hz: f64,
    pub depth_semitones: f64,
    pub wobble: f64,
}

impl Default for Vibrato {
    fn default() -> Self {
        Self { rate_hz: 5.5, depth_semitones: 0.0, wobble: 0.0 }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Slot {
    target: Constriction,
    current: Constriction,
    active: bool,
}

/// What the player asks for. The World moves toward it at articulator speed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TractControls {
    pub articulation: Articulation,
    /// Fold tuning: the tension stiffness makes the unloaded fold ring here.
    pub pitch_hz: f64,
    /// 0 breathy (abducted) .. 1 pressed (adducted).
    pub tenseness: f64,
    /// 0..1 of the lungs' maximum pressure.
    pub breath: f64,
    /// 0..1: fraction of the jet's dissipated power allowed to come back as
    /// turbulent pressure (stochastic supply, paid by the jet).
    pub aspiration: f64,
}

impl Default for TractControls {
    fn default() -> Self {
        Self { articulation: Articulation::default(), pitch_hz: 120.0, tenseness: 0.6, breath: 0.0, aspiration: 0.15 }
    }
}

/// An orb hit: a knock (pressure pulse paid by the orb) and a wall dent that
/// wobbles back (configuration, moved with receipted control work).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Impact {
    /// Position along the oral tube, 0 glottis .. 1 lips.
    center: f64,
    dent: f64,
    age_s: f64,
    section: usize,
    pulse_q: f64,
    pulse_k: usize,
    active: bool,
}

/// How the glottal end behaves. `Rigid` is for measuring the tube alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlottisMode {
    Voiced,
    Rigid,
}

/// Cumulative flows since construction, in joules.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Flows {
    /// Breathing muscles into the lung reservoir (the World's source work).
    pub muscle_work: f64,
    /// Orbs knocking the tube (source work too).
    pub impact_work: f64,
    /// Lungs into the air and folds (internal transfer).
    pub lung_supply: f64,
    /// Moving the area function at fixed waves.
    pub tract_control_work: f64,
    /// Retuning fold tension / adduction at fixed fold state.
    pub fold_control_work: f64,
    /// Plane-wave loss in both tubes.
    pub wall_loss: f64,
    /// Glottal jet dissipation (net of turbulence returned to the tract).
    pub jet_loss: f64,
    /// Fold viscous + contact damping.
    pub fold_loss: f64,
    pub radiation_lips: f64,
    pub radiation_nose: f64,
    /// Aerodynamic work done on the folds (internal, booked to the lungs).
    pub fold_aero_work: f64,
    /// Net loss at supraglottal jets (constrictions, lip exit, velum port),
    /// after the turbulence they paid for.
    pub constriction_loss: f64,
    /// Gross dissipation of those jets (before turbulence is paid out).
    pub constriction_dissipation: f64,
    /// Turbulent pressure work the constriction jets put back into the air
    /// (signed: a noise pair can also take a little energy).
    pub turbulence_work: f64,
    /// The positive part of `turbulence_work`, for the second-law check.
    pub turbulence_positive: f64,
}

/// A snapshot for presentation. Read-only: nothing here feeds back.
#[derive(Debug, Clone)]
pub struct TractReadings {
    pub oral_areas_cm2: Vec<f64>,
    pub nasal_areas_cm2: Vec<f64>,
    /// Pressure RMS per oral section since the last snapshot (Pa).
    pub oral_pressure_rms: Vec<f64>,
    /// Fold half-opening now (m), and its range over the last snapshot.
    pub fold_half_gap_m: f64,
    pub fold_gap_min_m: f64,
    pub fold_gap_max_m: f64,
    /// Latest complete fold cycle, resampled to `CYCLE_POINTS` (m).
    pub fold_cycle: [f64; CYCLE_POINTS],
    pub glottal_flow_m3_s: f64,
    pub lung_pressure_pa: f64,
    pub lung_fraction: f64,
    pub measured_f0_hz: f64,
    pub tract_energy_j: f64,
    pub fold_energy_j: f64,
    pub lung_energy_j: f64,
    pub flows: Flows,
    pub ledger_frames: u64,
    pub max_frame_residual_j: f64,
    pub cumulative_residual_j: f64,
    pub global_residual_j: f64,
    pub non_finite_events: u64,
    pub safety_limited_frames: u64,
    /// Largest constriction Reynolds number since the last snapshot.
    pub max_reynolds: f64,
    /// Smallest oral aperture now (m²) and where (0..1).
    pub min_aperture_m2: f64,
    pub min_aperture_position: f64,
    /// Constriction slots as the articulators have them now.
    pub constrictions: [Constriction; MAX_CONSTRICTIONS],
}

#[derive(Debug, Clone, Copy, Default)]
struct AirPlug {
    /// Admittances of the tube end, the radiation port and the plug's mass port.
    y_tube: f64,
    y_rad: f64,
    y_mass: f64,
    w_tube: f64,
    w_mass: f64,
    /// Last wave absorbed by the plug's mass port.
    s: f64,
}

impl AirPlug {
    fn configure(&mut self, area_m2: f64, y_tube: f64, dt: f64, a: &TractAnatomy) -> f64 {
        let radius = (area_m2 / PI).sqrt();
        let mass = AIR_DENSITY_KG_M3 * a.end_correction_ratio * radius / area_m2;
        let y_mass = dt / (2.0 * mass);
        let y_rad = area_m2 / (a.radiation_impedance_ratio * AIR_DENSITY_KG_M3 * SPEED_OF_SOUND_M_S);
        let work = (y_mass - self.y_mass) * self.s * self.s * dt;
        self.y_tube = y_tube;
        self.y_rad = y_rad;
        self.y_mass = y_mass;
        let total = y_tube + y_rad + y_mass;
        self.w_tube = 2.0 * y_tube / total;
        self.w_mass = 2.0 * y_mass / total;
        work
    }

    /// Scatter the arriving wave `a`; returns (reflected into tube, emitted pressure).
    #[inline]
    fn scatter(&mut self, a: f64) -> (f64, f64) {
        let p = self.w_tube * a - self.w_mass * self.s;
        let reflected = p - a;
        self.s += p;
        (reflected, p)
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Fold {
    y: f64,
    v: f64,
    stiffness: f64,
    rest: f64,
    contact_stiffness: f64,
    damping: f64,
    contact_damping: f64,
}

impl Fold {
    fn energy(&self, m: f64) -> f64 {
        let stretch = self.y - self.rest;
        let pen = self.y.min(0.0);
        0.5 * m * self.v * self.v + 0.5 * self.stiffness * stretch * stretch + 0.5 * self.contact_stiffness * pen * pen
    }
}

pub struct TractWorld {
    anatomy: TractAnatomy,
    sample_rate_hz: f64,
    dt: f64,
    rho_c: f64,
    n: usize,
    m: usize,
    /// Oral section index where the nasal branch joins (junction jv-1 | jv).
    jv: usize,
    // Oral tube.
    area: [f64; MAX_ORAL_SECTIONS],
    area_step: [f64; MAX_ORAL_SECTIONS],
    y: [f64; MAX_ORAL_SECTIONS],
    f: [f64; MAX_ORAL_SECTIONS],
    b: [f64; MAX_ORAL_SECTIONS],
    nf: [f64; MAX_ORAL_SECTIONS],
    nb: [f64; MAX_ORAL_SECTIONS],
    wl: [f64; MAX_ORAL_SECTIONS],
    wr: [f64; MAX_ORAL_SECTIONS],
    w3: [f64; 3],
    p2: [f64; MAX_ORAL_SECTIONS],
    // Nasal tube.
    narea: [f64; MAX_NASAL_SECTIONS],
    narea_step: [f64; MAX_NASAL_SECTIONS],
    ny: [f64; MAX_NASAL_SECTIONS],
    nfw: [f64; MAX_NASAL_SECTIONS],
    nbw: [f64; MAX_NASAL_SECTIONS],
    nnf: [f64; MAX_NASAL_SECTIONS],
    nnb: [f64; MAX_NASAL_SECTIONS],
    nwl: [f64; MAX_NASAL_SECTIONS],
    nwr: [f64; MAX_NASAL_SECTIONS],
    goal_scratch: [f64; MAX_ORAL_SECTIONS],
    ngoal_scratch: [f64; MAX_NASAL_SECTIONS],
    unclipped_scratch: [f64; MAX_ORAL_SECTIONS],
    // Junction apertures (m²): junction j joins sections j and j+1. They are
    // flow geometry with no storage, so moving them does no work.
    aperture: [f64; MAX_ORAL_SECTIONS],
    inv_area: [f64; MAX_ORAL_SECTIONS],
    inv_narea: [f64; MAX_NASAL_SECTIONS],
    aperture_goal: [f64; MAX_ORAL_SECTIONS],
    aperture_step: [f64; MAX_ORAL_SECTIONS],
    jet: [bool; MAX_ORAL_SECTIONS],
    k_fwd: [f64; MAX_ORAL_SECTIONS],
    k_back: [f64; MAX_ORAL_SECTIONS],
    r_visc: [f64; MAX_ORAL_SECTIONS],
    lip_aperture: f64,
    lip_aperture_goal: f64,
    lip_aperture_step: f64,
    lip_k_out: f64,
    lip_k_in: f64,
    lip_r_visc: f64,
    port_aperture: f64,
    port_aperture_goal: f64,
    port_aperture_step: f64,
    port_k: f64,
    port_r_visc: f64,
    slots: [Slot; MAX_CONSTRICTIONS],
    vibrato: Vibrato,
    vibrato_phase: f64,
    clock_s: f64,
    max_re: f64,
    noise_queue: [(usize, f64, f64); MAX_NOISE],
    pending_noise: usize,
    /// Sections where a constriction slot presses (an articulator in contact).
    slot_press: [bool; MAX_ORAL_SECTIONS],
    turb_lp: f64,
    turb_lp2: f64,
    turb_k: f64,
    turb_norm: f64,
    lips: AirPlug,
    nose: AirPlug,
    g_oral: f64,
    g_nasal: f64,
    // Glottis and lungs.
    fold: Fold,
    lung_energy: f64,
    lung_pressure: f64,
    glottal_flow: f64,
    pub glottis_mode: GlottisMode,
    pending_impulse: f64,
    noise_state: u64,
    // Controls, as asked and as the articulators have them.
    target: TractControls,
    current: TractControls,
    gesture: Option<Gesture>,
    gesture_time: f64,
    impacts: [Impact; MAX_IMPACTS],
    impact_count: u64,
    stride_phase: usize,
    area_moving: usize,
    fold_moving: bool,
    smooth_articulation: f64,
    /// Constriction closing, releasing and moving coefficients.
    smooth_slots: (f64, f64, f64),
    smooth_pitch: f64,
    smooth_tense: f64,
    smooth_breath: f64,
    // Output.
    pub output_gain: f64,
    // Ledger.
    flows: Flows,
    ledger: WorkLedger,
    non_finite_events: u64,
    safety_limited_frames: u64,
    // Observation (non-loading).
    ring: [f32; CYCLE_RING],
    ring_pos: usize,
    last_cycle_start: usize,
    last_voiced: usize,
    cycle_min: f64,
    cycle_max: f64,
    prev_v: f64,
    cycle: [f64; CYCLE_POINTS],
    measured_f0: f64,
    sample_counter: u64,
    gap_min: f64,
    gap_max: f64,
    rms_count: u64,
}

impl TractWorld {
    #[must_use]
    pub fn new(sample_rate_hz: f64) -> Self {
        Self::with_anatomy(sample_rate_hz, TractAnatomy::adult_v0())
    }

    #[must_use]
    pub fn with_anatomy(sample_rate_hz: f64, anatomy: TractAnatomy) -> Self {
        let sample_rate_hz = if sample_rate_hz.is_finite() { sample_rate_hz.clamp(16_000.0, 192_000.0) } else { 48_000.0 };
        let dt = 1.0 / (2.0 * sample_rate_hz);
        let n = anatomy.oral_sections(sample_rate_hz);
        let m = anatomy.nasal_sections(sample_rate_hz);
        let jv = ((anatomy.velum_position * n as f64).round() as usize).clamp(2, n - 2);
        let dx = SPEED_OF_SOUND_M_S * dt;
        let mut world = Self {
            anatomy,
            sample_rate_hz,
            dt,
            rho_c: AIR_DENSITY_KG_M3 * SPEED_OF_SOUND_M_S,
            n,
            m,
            jv,
            area: [0.0; MAX_ORAL_SECTIONS],
            area_step: [0.0; MAX_ORAL_SECTIONS],
            y: [0.0; MAX_ORAL_SECTIONS],
            f: [0.0; MAX_ORAL_SECTIONS],
            b: [0.0; MAX_ORAL_SECTIONS],
            nf: [0.0; MAX_ORAL_SECTIONS],
            nb: [0.0; MAX_ORAL_SECTIONS],
            wl: [0.0; MAX_ORAL_SECTIONS],
            wr: [0.0; MAX_ORAL_SECTIONS],
            w3: [0.0; 3],
            p2: [0.0; MAX_ORAL_SECTIONS],
            narea: [0.0; MAX_NASAL_SECTIONS],
            narea_step: [0.0; MAX_NASAL_SECTIONS],
            ny: [0.0; MAX_NASAL_SECTIONS],
            nfw: [0.0; MAX_NASAL_SECTIONS],
            nbw: [0.0; MAX_NASAL_SECTIONS],
            nnf: [0.0; MAX_NASAL_SECTIONS],
            nnb: [0.0; MAX_NASAL_SECTIONS],
            nwl: [0.0; MAX_NASAL_SECTIONS],
            nwr: [0.0; MAX_NASAL_SECTIONS],
            goal_scratch: [0.0; MAX_ORAL_SECTIONS],
            ngoal_scratch: [0.0; MAX_NASAL_SECTIONS],
            unclipped_scratch: [0.0; MAX_ORAL_SECTIONS],
            aperture: [0.0; MAX_ORAL_SECTIONS],
            inv_area: [0.0; MAX_ORAL_SECTIONS],
            inv_narea: [0.0; MAX_NASAL_SECTIONS],
            aperture_goal: [0.0; MAX_ORAL_SECTIONS],
            aperture_step: [0.0; MAX_ORAL_SECTIONS],
            jet: [false; MAX_ORAL_SECTIONS],
            k_fwd: [0.0; MAX_ORAL_SECTIONS],
            k_back: [0.0; MAX_ORAL_SECTIONS],
            r_visc: [0.0; MAX_ORAL_SECTIONS],
            lip_aperture: 0.0,
            lip_aperture_goal: 0.0,
            lip_aperture_step: 0.0,
            lip_k_out: 0.0,
            lip_k_in: 0.0,
            lip_r_visc: 0.0,
            port_aperture: 0.0,
            port_aperture_goal: 0.0,
            port_aperture_step: 0.0,
            port_k: 0.0,
            port_r_visc: 0.0,
            slots: [Slot::default(); MAX_CONSTRICTIONS],
            vibrato: Vibrato::default(),
            vibrato_phase: 0.0,
            clock_s: 0.0,
            max_re: 0.0,
            noise_queue: [(0, 0.0, 0.0); MAX_NOISE],
            pending_noise: 0,
            slot_press: [false; MAX_ORAL_SECTIONS],
            turb_lp: 0.0,
            turb_lp2: 0.0,
            turb_k: 1.0 - (-TAU * TURBULENCE_CORNER_HZ * dt).exp(),
            turb_norm: two_pole_norm(1.0 - (-TAU * TURBULENCE_CORNER_HZ * dt).exp()),
            lips: AirPlug::default(),
            nose: AirPlug::default(),
            g_oral: (-anatomy.oral_attenuation_np_per_m * dx).exp(),
            g_nasal: (-anatomy.nasal_attenuation_np_per_m * dx).exp(),
            fold: Fold::default(),
            lung_energy: anatomy.lung.capacity_j,
            lung_pressure: 0.0,
            glottal_flow: 0.0,
            glottis_mode: GlottisMode::Voiced,
            pending_impulse: 0.0,
            noise_state: 0x9E37_79B9_7F4A_7C15,
            target: TractControls::default(),
            current: TractControls::default(),
            gesture: None,
            gesture_time: 0.0,
            impacts: [Impact::default(); MAX_IMPACTS],
            impact_count: 0,
            stride_phase: 0,
            area_moving: 0,
            fold_moving: false,
            smooth_articulation: one_pole(0.025, sample_rate_hz),
            smooth_slots: (
                one_pole(anatomy.constriction_close_s, sample_rate_hz),
                one_pole(anatomy.constriction_release_s, sample_rate_hz),
                one_pole(0.015, sample_rate_hz),
            ),
            smooth_pitch: one_pole(0.030, sample_rate_hz),
            smooth_tense: one_pole(0.050, sample_rate_hz),
            smooth_breath: one_pole(0.040, sample_rate_hz),
            output_gain: 1.0 / 2_500.0,
            flows: Flows::default(),
            ledger: WorkLedger::default(),
            non_finite_events: 0,
            safety_limited_frames: 0,
            ring: [0.0; CYCLE_RING],
            ring_pos: 0,
            last_cycle_start: 0,
            last_voiced: 0,
            cycle_min: 0.0,
            cycle_max: 0.0,
            prev_v: 0.0,
            cycle: [0.0; CYCLE_POINTS],
            measured_f0: 0.0,
            sample_counter: 0,
            gap_min: f64::MAX,
            gap_max: f64::MIN,
            rms_count: 0,
        };
        world.snap_configuration();
        world.fold.y = world.fold.rest;
        world
    }

    #[must_use]
    pub fn anatomy(&self) -> &TractAnatomy {
        &self.anatomy
    }

    #[must_use]
    pub fn sample_rate_hz(&self) -> f64 {
        self.sample_rate_hz
    }

    #[must_use]
    pub fn oral_sections(&self) -> usize {
        self.n
    }

    #[must_use]
    pub fn nasal_sections(&self) -> usize {
        self.m
    }

    #[must_use]
    pub fn velum_section(&self) -> usize {
        self.jv
    }

    #[must_use]
    pub fn controls(&self) -> TractControls {
        self.target
    }

    #[must_use]
    pub fn current_controls(&self) -> TractControls {
        self.current
    }

    /// Ask for a configuration; the articulators get there at their own speed.
    pub fn set_controls(&mut self, controls: TractControls) {
        self.target = sanitize(controls, &self.anatomy);
    }

    /// Run a sample-accurate gesture (a control path), or stop one.
    pub fn set_gesture(&mut self, gesture: Option<Gesture>) {
        self.gesture = gesture;
        self.gesture_time = 0.0;
    }

    #[must_use]
    pub fn gesture_active(&self) -> bool {
        self.gesture.is_some()
    }

    #[must_use]
    pub fn ledger(&self) -> &WorkLedger {
        &self.ledger
    }

    #[must_use]
    pub fn flows(&self) -> Flows {
        self.flows
    }

    /// Jump the configuration to the current controls with no transport
    /// (construction / measurement only: books the jump as control work).
    pub fn snap_configuration(&mut self) {
        self.current = self.target;
        self.compute_goal_areas();
        let mut work = 0.0;
        let dt = self.dt;
        for i in 0..self.n {
            let a = self.goal_scratch[i] * 1.0e-4;
            let y_new = a / self.rho_c;
            if self.y[i] > 0.0 {
                work += transport_section(&mut self.f[i], &mut self.b[i], self.y[i], y_new, dt);
            }
            self.area[i] = a;
            self.y[i] = y_new;
            self.area_step[i] = 0.0;
        }
        for k in 0..self.m {
            let a = self.ngoal_scratch[k] * 1.0e-4;
            let y_new = a / self.rho_c;
            if self.ny[k] > 0.0 {
                work += transport_section(&mut self.nfw[k], &mut self.nbw[k], self.ny[k], y_new, dt);
            }
            self.narea[k] = a;
            self.ny[k] = y_new;
            self.narea_step[k] = 0.0;
        }
        for slot in self.slots.iter_mut() {
            slot.current = slot.target;
        }
        self.compute_goal_areas();
        self.aperture = self.aperture_goal;
        self.aperture_step = [0.0; MAX_ORAL_SECTIONS];
        self.lip_aperture = self.lip_aperture_goal;
        self.lip_aperture_step = 0.0;
        self.port_aperture = self.port_aperture_goal;
        self.port_aperture_step = 0.0;
        work += self.refresh_junctions();
        work += self.retune_fold();
        self.flows.tract_control_work += work;
    }

    fn compute_goal_areas(&mut self) {
        use crate::anatomy::{gaussian, AREA_CEIL_CM2, AREA_FLOOR_CM2};
        let n = self.n;
        let m = self.m;
        let art = self.current.articulation;
        self.anatomy.oral_areas_unclipped_cm2(&art, &mut self.unclipped_scratch[..n]);
        // Orb dents: log-area pushed in at the hit, wobbling back.
        let mut velum_flap = 0.0;
        for imp in self.impacts.iter().filter(|i| i.active) {
            let w = imp.dent * wobble(imp.age_s);
            velum_flap += w.abs() * gaussian(imp.center, self.anatomy.velum_position, 0.08);
            for (i, a) in self.unclipped_scratch[..n].iter_mut().enumerate() {
                let x = (i as f64 + 0.5) / n as f64;
                *a = (*a * (-w * gaussian(x, imp.center, DENT_WIDTH)).exp()).min(AREA_CEIL_CM2);
            }
        }
        // The tongue body, lips and dents stay inside GARGANTA's cone. Only
        // constrictions (and the lip seal) take an aperture below its floor:
        // the wall pressed in by (1 - degree * bump), with a flat top wide
        // enough to hold a section centre, so degree 1 closes exactly.
        let half = 0.6 / n as f64;
        let width = self.anatomy.constriction_width;
        for i in 0..n {
            let x = (i as f64 + 0.5) / n as f64;
            let mut factor = 1.0;
            for slot in self.slots.iter().filter(|s| s.active && s.current.degree > 0.0) {
                let dist = (x - slot.current.position).abs();
                let bump = if dist <= half { 1.0 } else { (-0.5 * ((dist - half) / width).powi(2)).exp() };
                factor *= (1.0 - slot.current.degree * bump).max(0.0);
            }
            // unclipped_scratch now holds the flow area of each section.
            self.slot_press[i] = factor < 0.5;
            let a = self.unclipped_scratch[i].clamp(AREA_FLOOR_CM2, AREA_CEIL_CM2) * factor;
            self.goal_scratch[i] = a.max(AREA_FLOOR_CM2);
            self.unclipped_scratch[i] = a;
        }
        for j in 0..n - 1 {
            self.aperture_goal[j] = self.unclipped_scratch[j].min(self.unclipped_scratch[j + 1]) * 1.0e-4;
        }
        let seal = (art.lips / self.anatomy.lip_seal).clamp(0.0, 1.0);
        self.lip_aperture_goal = self.unclipped_scratch[n - 1] * seal * 1.0e-4;
        let velum = (art.velum + velum_flap).min(1.0);
        self.port_aperture_goal = velum * self.anatomy.velum_port_max_cm2 * 1.0e-4;
        self.anatomy.nasal_areas_cm2(velum, &mut self.ngoal_scratch[..m]);
    }

    /// An orb hits the tract at `position` (0 glottis .. 1 lips) carrying
    /// `energy_j` joules. It knocks the air there (the pulse's work is booked as
    /// source work) and dents the wall, which wobbles back (control work).
    pub fn impact(&mut self, position: f64, energy_j: f64) {
        let position = finite_or(position, 0.5).clamp(0.0, 1.0);
        let energy = finite_or(energy_j, 0.0).clamp(0.0, 2.0e-3);
        if energy <= 0.0 {
            return;
        }
        let section = ((position * self.n as f64) as usize).min(self.n - 1);
        let hann2: f64 = (0..PULSE_SAMPLES).map(|k| hann(k, PULSE_SAMPLES).powi(2)).sum();
        // Two waves (f and b) per sample, two steps' worth of storage time.
        let q = (energy / (2.0 * self.y[section] * self.dt * hann2)).sqrt();
        let dent = (DENT_PER_SQRT_MJ * (energy * 1.0e3).sqrt()).min(DENT_MAX);
        let slot = self.impacts.iter().position(|i| !i.active).unwrap_or_else(|| {
            // Replace the oldest.
            let mut oldest = 0;
            for k in 1..MAX_IMPACTS {
                if self.impacts[k].age_s > self.impacts[oldest].age_s {
                    oldest = k;
                }
            }
            oldest
        });
        self.impacts[slot] = Impact { center: position, dent, age_s: 0.0, section, pulse_q: q, pulse_k: 0, active: true };
        self.impact_count += 1;
    }

    /// Orb dents the larynx: fold tension jiggles with the wall.
    fn larynx_jiggle(&self) -> f64 {
        let mut j = 0.0;
        for imp in self.impacts.iter().filter(|i| i.active) {
            j += imp.dent * wobble(imp.age_s) * crate::anatomy::gaussian(imp.center, 0.0, 0.08);
        }
        (0.12 * j).clamp(-0.3, 0.3)
    }

    fn advance_impacts(&mut self, audio_dt: f64) {
        for k in 0..MAX_IMPACTS {
            if !self.impacts[k].active {
                continue;
            }
            let imp = self.impacts[k];
            if imp.pulse_k < PULSE_SAMPLES {
                let d = imp.pulse_q * hann(imp.pulse_k, PULSE_SAMPLES);
                let i = imp.section;
                let (f, b) = (self.f[i], self.b[i]);
                let work = self.y[i] * ((f + d) * (f + d) - f * f + (b + d) * (b + d) - b * b) * self.dt;
                self.f[i] = f + d;
                self.b[i] = b + d;
                self.flows.impact_work += work;
                self.impacts[k].pulse_k += 1;
            }
            self.impacts[k].age_s += audio_dt;
            if self.impacts[k].age_s > 6.0 * WOBBLE_DECAY_S {
                self.impacts[k].active = false;
            }
        }
    }

    #[must_use]
    pub fn impact_count(&self) -> u64 {
        self.impact_count
    }

    /// Junction weights from the current admittances; returns air-plug work.
    fn refresh_junctions(&mut self) -> f64 {
        let n = self.n;
        for j in 0..n - 1 {
            let (yl, yr) = (self.y[j], self.y[j + 1]);
            let s = yl + yr;
            self.wl[j] = 2.0 * yl / s;
            self.wr[j] = 2.0 * yr / s;
        }
        let (y1, y2, y3) = (self.y[self.jv - 1], self.y[self.jv], self.ny[0]);
        let s = y1 + y2 + y3;
        self.w3 = [2.0 * y1 / s, 2.0 * y2 / s, 2.0 * y3 / s];
        for k in 0..self.m - 1 {
            let (yl, yr) = (self.ny[k], self.ny[k + 1]);
            let s = yl + yr;
            self.nwl[k] = 2.0 * yl / s;
            self.nwr[k] = 2.0 * yr / s;
        }
        let anatomy = self.anatomy;
        let mut work = self.lips.configure(self.area[n - 1], self.y[n - 1], self.dt, &anatomy);
        work += self.nose.configure(self.narea[self.m - 1], self.ny[self.m - 1], self.dt, &anatomy);
        self.refresh_jets();
        work
    }

    /// Flow resistance at each junction from its aperture: Borda-Carnot
    /// expansion loss `(rho/2)(1/A_c - 1/A_down)²` (ramped in as the aperture
    /// falls from 0.8 to 0.5 of the wider side, so a smooth tube stays
    /// lossless) and a viscous slit term `12 mu dx w² / A_c³` below 0.25 cm².
    fn refresh_jets(&mut self) {
        let half_rho = 0.5 * AIR_DENSITY_KG_M3;
        let dx = SPEED_OF_SOUND_M_S * self.dt;
        let w2 = self.anatomy.constriction_slit_width_m.powi(2);
        let visc = |ac: f64| if ac < VISCOUS_AREA_M2 { 12.0 * crate::anatomy::AIR_VISCOSITY_PA_S * dx * w2 / (ac * ac * ac) } else { 0.0 };
        let bc = |ac: f64, down: f64| if ac < down { half_rho * (1.0 / ac - 1.0 / down).powi(2) } else { 0.0 };
        for j in 0..self.n - 1 {
            if j + 1 == self.jv {
                self.jet[j] = false;
                continue;
            }
            let ac = self.aperture[j];
            if ac <= CLOSED_M2 {
                self.jet[j] = true;
                self.k_fwd[j] = f64::INFINITY;
                self.k_back[j] = f64::INFINITY;
                self.r_visc[j] = f64::INFINITY;
                continue;
            }
            let (al, ar) = (self.area[j], self.area[j + 1]);
            // A smooth, open junction (the common case) is lossless: rule it
            // out with a compare before any division.
            if ac >= 0.8 * al.max(ar) && ac >= VISCOUS_AREA_M2 {
                self.jet[j] = false;
                continue;
            }
            let ratio = ac / al.max(ar);
            let ramp = smoothstep((0.8 - ratio) / 0.3);
            self.k_fwd[j] = ramp * bc(ac, ar);
            self.k_back[j] = ramp * bc(ac, al);
            self.r_visc[j] = visc(ac);
            self.jet[j] = self.k_fwd[j] > 0.0 || self.k_back[j] > 0.0 || self.r_visc[j] > 0.0;
        }
        let la = self.lip_aperture;
        if la <= CLOSED_M2 {
            self.lip_k_out = f64::INFINITY;
            self.lip_k_in = f64::INFINITY;
            self.lip_r_visc = f64::INFINITY;
        } else {
            self.lip_k_out = half_rho / (la * la);
            self.lip_k_in = bc(la, self.area[self.n - 1]);
            self.lip_r_visc = visc(la);
        }
        let pa = self.port_aperture;
        if pa <= CLOSED_M2 {
            self.port_k = f64::INFINITY;
            self.port_r_visc = f64::INFINITY;
        } else {
            self.port_k = bc(pa, self.area[self.jv - 1].max(self.area[self.jv]));
            self.port_r_visc = visc(pa);
        }
    }

    /// Fold tension (pitch) and adduction (tenseness) at fixed fold state.
    fn retune_fold(&mut self) -> f64 {
        let fa = self.anatomy.fold;
        let before = self.fold.energy(fa.mass_kg);
        let omega = TAU * self.current.pitch_hz * (1.0 + self.larynx_jiggle()) * self.tension_modulation();
        self.fold.stiffness = fa.mass_kg * omega * omega;
        self.fold.contact_stiffness = fa.contact_stiffness_ratio * self.fold.stiffness;
        self.fold.damping = 2.0 * fa.damping_ratio * fa.mass_kg * omega;
        self.fold.contact_damping = 2.0 * fa.contact_damping_ratio * fa.mass_kg * omega;
        let t = self.current.tenseness;
        let abduct = 1.0 + fa.abduction_boost * ((0.3 - t) / 0.3).max(0.0);
        self.fold.rest = abduct * fa.rest_half_gap_breathy_m * (fa.rest_half_gap_pressed_m / fa.rest_half_gap_breathy_m).powf(t);
        self.fold.energy(fa.mass_kg) - before
    }

    /// Vibrato and wobble as a frequency ratio of the fold tuning.
    fn tension_modulation(&self) -> f64 {
        let v = self.vibrato;
        if v.depth_semitones == 0.0 && v.wobble == 0.0 {
            return 1.0;
        }
        let t = self.clock_s;
        let wob = 0.55 * (TAU * 0.43 * t).sin() + 0.3 * (TAU * 0.79 * t + 1.3).sin() + 0.15 * (TAU * 1.31 * t + 2.1).sin();
        let semis = v.depth_semitones * (TAU * self.vibrato_phase).sin() + 0.35 * v.wobble * wob;
        (semis * (std::f64::consts::LN_2 / 12.0)).exp()
    }

    #[must_use]
    pub fn stored_energy(&self) -> f64 {
        self.tract_energy() + self.fold.energy(self.anatomy.fold.mass_kg) + self.lung_energy
    }

    /// Acoustic energy in both tubes and the air plugs (J).
    #[must_use]
    pub fn tract_energy(&self) -> f64 {
        let mut e = 0.0;
        for i in 0..self.n {
            e += self.y[i] * (self.f[i] * self.f[i] + self.b[i] * self.b[i]);
        }
        for k in 0..self.m {
            e += self.ny[k] * (self.nfw[k] * self.nfw[k] + self.nbw[k] * self.nbw[k]);
        }
        e += self.lips.y_mass * self.lips.s * self.lips.s + self.nose.y_mass * self.nose.s * self.nose.s;
        e * self.dt
    }

    fn noise(&mut self) -> f64 {
        let mut x = self.noise_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.noise_state = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 52) as f64 - 1.0
    }

    /// Control path: gesture, articulator smoothing, area transport.
    fn control_tick(&mut self) {
        let audio_dt = 2.0 * self.dt;
        self.clock_s += audio_dt;
        self.vibrato_phase = (self.vibrato_phase + self.vibrato.rate_hz * audio_dt).fract();
        if let Some(g) = self.gesture {
            if let Some(c) = g.controls_at(self.gesture_time) {
                self.target = sanitize(c, &self.anatomy);
                let k = g.constriction_at(self.gesture_time);
                if k.degree > 0.0 || self.slots[0].active {
                    self.set_constriction(0, Some(k));
                }
            } else {
                self.gesture = None;
                self.set_constriction(0, None);
            }
            self.gesture_time += audio_dt;
        }
        // Constrictions close and release at their own speeds.
        let mut moving_slots = false;
        let (kc, kr, kp) = self.smooth_slots;
        for slot in self.slots.iter_mut().filter(|s| s.active) {
            let k = if slot.target.degree > slot.current.degree { kc } else { kr };
            moving_slots |= approach(&mut slot.current.degree, slot.target.degree, k);
            moving_slots |= approach(&mut slot.current.position, slot.target.position, kp);
            if slot.target.degree == 0.0 && slot.current.degree < 1.0e-6 {
                slot.current.degree = 0.0;
                slot.active = false;
                moving_slots = true;
            }
        }
        // Articulators and laryngeal settings move at finite speed.
        let (t, c) = (self.target, &mut self.current);
        let ka = self.smooth_articulation;
        let moving_art = approach(&mut c.articulation.tongue_index, t.articulation.tongue_index, ka)
            | approach(&mut c.articulation.tongue_diameter, t.articulation.tongue_diameter, ka)
            | approach(&mut c.articulation.lips, t.articulation.lips, ka)
            | approach(&mut c.articulation.velum, t.articulation.velum, ka);
        let moving_fold = approach(&mut c.pitch_hz, t.pitch_hz, self.smooth_pitch)
            | approach(&mut c.tenseness, t.tenseness, self.smooth_tense);
        approach(&mut c.breath, t.breath, self.smooth_breath);
        c.aspiration = t.aspiration;

        let impacts = self.impacts.iter().any(|i| i.active);
        self.advance_impacts(audio_dt);
        // The area function moves while the articulators do, while a dent
        // wobbles, and for one stride after (to land exactly on the goal).
        if moving_art || impacts || moving_slots {
            self.area_moving = AREA_STRIDE + 1;
        }
        if self.area_moving > 0 {
            if self.stride_phase == 0 {
                self.compute_goal_areas();
                let inv = 1.0 / AREA_STRIDE as f64;
                for i in 0..self.n {
                    self.area_step[i] = (self.goal_scratch[i] * 1.0e-4 - self.area[i]) * inv;
                }
                for k in 0..self.m {
                    self.narea_step[k] = (self.ngoal_scratch[k] * 1.0e-4 - self.narea[k]) * inv;
                }
                for j in 0..self.n - 1 {
                    self.aperture_step[j] = (self.aperture_goal[j] - self.aperture[j]) * inv;
                }
                for i in 0..self.n {
                    self.inv_area[i] = 1.0 / self.area[i];
                }
                for k in 0..self.m {
                    self.inv_narea[k] = 1.0 / self.narea[k];
                }
                self.lip_aperture_step = (self.lip_aperture_goal - self.lip_aperture) * inv;
                self.port_aperture_step = (self.port_aperture_goal - self.port_aperture) * inv;
            }
            // Apertures are flow geometry without storage: no work to move.
            // The last step of a stride lands exactly on the goal, so a
            // closure reaches exactly zero.
            let last = self.stride_phase == AREA_STRIDE - 1;
            for j in 0..self.n - 1 {
                self.aperture[j] = if last { self.aperture_goal[j] } else { (self.aperture[j] + self.aperture_step[j]).max(0.0) };
            }
            self.lip_aperture = if last { self.lip_aperture_goal } else { (self.lip_aperture + self.lip_aperture_step).max(0.0) };
            self.port_aperture = if last { self.port_aperture_goal } else { (self.port_aperture + self.port_aperture_step).max(0.0) };
            self.stride_phase = (self.stride_phase + 1) % AREA_STRIDE;
            self.area_moving -= 1;

            // Transport: the area function moves holding each section's air
            // volume excess and momentum; the change of stored energy is the
            // work the articulators did.
            // r = A_old / A_new, to first order with 1/A taken at the stride's
            // start (no division per sample); the work booked is the exact
            // energy change for whatever r is used.
            let mut work = 0.0;
            let inv_rc = 1.0 / self.rho_c;
            for i in 0..self.n {
                let step = self.area_step[i];
                if step == 0.0 {
                    continue;
                }
                let a = self.area[i] + step;
                let y_new = a * inv_rc;
                let r = 1.0 - step * self.inv_area[i];
                let e = self.f[i] * self.f[i] + self.b[i] * self.b[i];
                work += (y_new * r * r - self.y[i]) * e;
                self.f[i] *= r;
                self.b[i] *= r;
                self.area[i] = a;
                self.y[i] = y_new;
            }
            for k in 0..self.m {
                let step = self.narea_step[k];
                if step == 0.0 {
                    continue;
                }
                let a = self.narea[k] + step;
                let y_new = a * inv_rc;
                let r = 1.0 - step * self.inv_narea[k];
                let e = self.nfw[k] * self.nfw[k] + self.nbw[k] * self.nbw[k];
                work += (y_new * r * r - self.ny[k]) * e;
                self.nfw[k] *= r;
                self.nbw[k] *= r;
                self.narea[k] = a;
                self.ny[k] = y_new;
            }
            work *= self.dt;
            work += self.refresh_junctions();
            self.flows.tract_control_work += work;
        } else {
            self.stride_phase = 0;
        }
        let modulated = self.vibrato.depth_semitones > 0.0 || self.vibrato.wobble > 0.0;
        if moving_fold || impacts || modulated || self.fold_moving {
            self.flows.fold_control_work += self.retune_fold();
            self.fold_moving = moving_fold || impacts || modulated;
        }

        // Lungs: muscles refill the finite reservoir; deliverable pressure
        // sags when it runs low.
        let lung = self.anatomy.lung;
        let refill = (lung.refill_w * audio_dt).min((lung.capacity_j - self.lung_energy).max(0.0));
        self.lung_energy += refill;
        self.flows.muscle_work += refill;
        let sag = (self.lung_energy / (lung.sag_fraction * lung.capacity_j)).clamp(0.0, 1.0);
        self.lung_pressure = self.current.breath * lung.max_pressure_pa * sag;
    }

    /// The glottal boundary for one tract step. Returns the wave sent up the tube.
    #[inline]
    fn glottis(&mut self, b0: f64) -> f64 {
        let y0 = self.y[0];
        let z0 = 1.0 / y0;
        if self.glottis_mode == GlottisMode::Rigid {
            let u = std::mem::take(&mut self.pending_impulse);
            self.glottal_flow = u;
            return b0 + z0 * u;
        }
        let fa = self.anatomy.fold;
        let dt = self.dt;
        let ps = self.lung_pressure;
        let (yf, vf) = (self.fold.y, self.fold.v);
        let xi1 = yf + fa.mucosal_delay_s * vf;
        let xi2 = yf - fa.mucosal_delay_s * vf;
        let a1 = 2.0 * fa.length_m * xi1.max(0.0);
        let a2 = 2.0 * fa.length_m * xi2.max(0.0);
        let amin = a1.min(a2);

        // Jet through the slit: P_s - p1 = R_v U + rho U|U| / (2 a²), with
        // the viscous slit resistance R_v = 12 mu d l² / a³; p1 = 2 b0 + Z0 U.
        let delta = ps - 2.0 * b0;
        let u = if amin <= 1.0e-14 {
            0.0
        } else {
            let kappa = AIR_DENSITY_KG_M3 / (2.0 * amin * amin);
            let viscous = 12.0 * AIR_VISCOSITY_PA_S * fa.thickness_m * fa.length_m * fa.length_m / (amin * amin * amin);
            let z = z0 + viscous;
            let root = (z * z + 4.0 * kappa * delta.abs()).sqrt();
            2.0 * delta / (z + root)
        };
        let p1 = 2.0 * b0 + z0 * u;
        let mut jet = (ps - p1) * u * dt;
        let mut f0 = p1 - b0;

        // Turbulence: a fraction of the jet's dissipated power returns as
        // pressure noise; whatever it carries is taken from the jet.
        if self.current.aspiration > 0.0 && jet > 0.0 {
            let fraction = self.current.aspiration * self.anatomy.turbulence_efficiency;
            let amplitude = (fraction * jet / dt * z0).sqrt();
            let mut nw = amplitude * self.noise();
            let mut w_n = y0 * nw * (2.0 * f0 + nw) * dt;
            if w_n > jet {
                // Shrink until the jet can pay it (second law at the jet).
                for _ in 0..4 {
                    nw *= 0.5;
                    w_n = y0 * nw * (2.0 * f0 + nw) * dt;
                    if w_n <= jet {
                        break;
                    }
                }
                if w_n > jet {
                    nw = 0.0;
                    w_n = 0.0;
                }
            }
            f0 += nw;
            jet -= w_n;
        }

        // Intraglottal pressure (one-mass surface-wave form).
        let pg = if a1 > 0.0 && a2 > 0.0 {
            p1 + (ps - p1) * (1.0 - a2 / a1).max(-1.0)
        } else if a1 > 0.0 {
            ps
        } else {
            p1
        };
        let force = fa.length_m * fa.thickness_m * pg;
        let (d, loss) = self.fold_update(force);
        let aero = force * d;
        let supply = ps * u * dt + aero;
        self.lung_energy -= supply;
        self.flows.lung_supply += supply;
        self.flows.fold_aero_work += aero;
        self.flows.jet_loss += jet;
        self.flows.fold_loss += loss;
        self.glottal_flow = u;
        f0
    }

    /// Midpoint step for the fold with a discrete-gradient collision; returns
    /// (displacement increment, dissipated energy).
    fn fold_update(&mut self, force: f64) -> (f64, f64) {
        let m = self.anatomy.fold.mass_kg;
        let dt = self.dt;
        let fold = &mut self.fold;
        let (y0, v0) = (fold.y, fold.v);
        let r = fold.damping + if y0 < 0.0 { fold.contact_damping } else { 0.0 };
        let a = 2.0 * m / (dt * dt) + r / dt + 0.5 * fold.stiffness;
        let rhs = force + 2.0 * m * v0 / dt - fold.stiffness * (y0 - fold.rest);
        let kc = fold.contact_stiffness;
        let vc = |y: f64| 0.5 * kc * y.min(0.0) * y.min(0.0);
        let v0c = vc(y0);
        let g_c = |d: f64| -> f64 {
            if d.abs() < 1.0e-18 {
                kc * y0.min(0.0)
            } else {
                (vc(y0 + d) - v0c) / d
            }
        };
        let mut d = rhs / a;
        if y0 < 0.0 || y0 + d < 0.0 {
            for _ in 0..NEWTON_STEPS {
                let g = a * d + g_c(d) - rhs;
                let h = 1.0e-9_f64.max(d.abs() * 1.0e-6);
                let slope = a + (g_c(d + h) - g_c(d - h)) / (2.0 * h);
                let next = d - g / slope;
                if (next - d).abs() < 1.0e-16 {
                    d = next;
                    break;
                }
                d = next;
            }
        }
        let v1 = 2.0 * d / dt - v0;
        fold.y = y0 + d;
        fold.v = v1;
        (d, r * d * d / dt)
    }

    /// One tract step (half an audio sample). Returns emitted pressure.
    #[inline]
    fn step(&mut self) -> f64 {
        let n = self.n;
        let m = self.m;
        let jv = self.jv;
        let dt = self.dt;
        self.nf[0] = self.glottis(self.b[0]);
        for j in 0..n - 1 {
            if j + 1 == jv {
                continue;
            }
            if !self.jet[j] {
                let p = self.wl[j] * self.f[j] + self.wr[j] * self.b[j + 1];
                self.nb[j] = p - self.f[j];
                self.nf[j + 1] = p - self.b[j + 1];
            } else {
                self.jet_junction(j);
            }
        }
        // Velum: oral two-port with the nasal branch through the port aperture.
        {
            let j = jv - 1;
            let (y1, y2) = (self.y[j], self.y[jv]);
            let (a1, a2, a3) = (self.f[j], self.b[jv], self.nbw[0]);
            let y12 = y1 + y2;
            let z3 = 1.0 / self.ny[0];
            let mean = 2.0 * (y1 * a1 + y2 * a2);
            let un = if self.port_aperture <= CLOSED_M2 {
                0.0
            } else {
                let rhs = mean / y12 - 2.0 * a3;
                jet_flow(rhs, 1.0 / y12 + z3 + self.port_r_visc, self.port_k)
            };
            let p = (mean - un) / y12;
            self.nb[j] = p - a1;
            self.nf[jv] = p - a2;
            self.nnf[0] = a3 + un * z3;
            let loss = (p - (2.0 * a3 + un * z3)) * un * dt;
            self.flows.constriction_loss += loss;
            self.flows.constriction_dissipation += loss;
        }
        for k in 0..m - 1 {
            let p = self.nwl[k] * self.nfw[k] + self.nwr[k] * self.nbw[k + 1];
            self.nnb[k] = p - self.nfw[k];
            self.nnf[k + 1] = p - self.nbw[k + 1];
        }
        let p_lip = self.lip_exit();
        let p_lip = p_lip + self.apply_turbulence(p_lip);
        let (nrefl, p_nose) = self.nose.scatter(self.nfw[m - 1]);
        self.nnb[m - 1] = nrefl;
        self.flows.radiation_lips += self.lips.y_rad * p_lip * p_lip * dt;
        self.flows.radiation_nose += self.nose.y_rad * p_nose * p_nose * dt;

        // Plane-wave loss: every stored wave keeps g of its amplitude.
        let mut e_oral = 0.0;
        let g = self.g_oral;
        for i in 0..n {
            let (fi, bi) = (self.nf[i], self.nb[i]);
            e_oral += self.y[i] * (fi * fi + bi * bi);
            self.f[i] = fi * g;
            self.b[i] = bi * g;
        }
        let mut e_nasal = 0.0;
        let gn = self.g_nasal;
        for k in 0..m {
            let (fk, bk) = (self.nnf[k], self.nnb[k]);
            e_nasal += self.ny[k] * (fk * fk + bk * bk);
            self.nfw[k] = fk * gn;
            self.nbw[k] = bk * gn;
        }
        self.flows.wall_loss += ((1.0 - g * g) * e_oral + (1.0 - gn * gn) * e_nasal) * dt;
        p_lip + p_nose
    }

    /// Reynolds-gated turbulence for a jet of flow `u` through aperture `ac`
    /// that dissipated `gross` joules this step: queue a noise injection of
    /// up to `frication_efficiency` of that loss where the jet lands.
    /// Turbulence source spectrum: white noise through two one-pole lowpasses at
    /// `TURBULENCE_CORNER_HZ` (a design hypothesis), scaled to unit variance.
    #[inline]
    fn turbulent_noise(&mut self) -> f64 {
        let x = 1.732_050_8 * self.noise();
        self.turb_lp += (x - self.turb_lp) * self.turb_k;
        self.turb_lp2 += (self.turb_lp - self.turb_lp2) * self.turb_k;
        self.turb_lp2 * self.turb_norm
    }

    #[inline]
    fn queue_turbulence(&mut self, u: f64, ac: f64, gross: f64, from: usize, downstream: bool, contact: bool) {
        let re = 2.0 * u.abs() / (NU * (PI * ac).sqrt());
        self.max_re = self.max_re.max(re);
        let rc = self.anatomy.frication_reynolds;
        let onset = ((re - rc) / rc).clamp(0.0, 1.0);
        if onset <= 0.0 || gross <= 0.0 || self.pending_noise >= MAX_NOISE {
            return;
        }
        // Where the jet lands: the first section that has widened back to
        // 2.5x the aperture, within five sections; past the lips, the air.
        let n = self.n;
        let mut target = usize::MAX;
        for k in 1..=5 {
            let i = if downstream { from + k } else { from.wrapping_sub(k - 1) };
            if i >= n {
                break;
            }
            if self.area[i] >= 2.5 * ac || k == 5 {
                target = i;
                break;
            }
        }
        if target == usize::MAX && !downstream {
            return;
        }
        // A jet on an articulator in contact at the very mouth (/f/, /v/)
        // makes its noise in the last section, behind the lips' radiation
        // load, not straight into the air.
        if target == usize::MAX && contact {
            target = n - 1;
        }
        // Noise power is a share of the jet's own loss. A jet that lands on a
        // wall, or on an articulator in contact (teeth, lip), is an obstacle
        // (dipole) source: 0.4% at Mach 0.1, growing as Mach³. A jet that leaves
        // an open mouth into free air is a quadrupole, Lighthill's 1e-4 M⁵
        // (silent at speech speeds). Capped at `frication_efficiency`.
        let mach = u.abs() / ac / SPEED_OF_SOUND_M_S;
        let free = target == usize::MAX;
        let eff = if free { 1.0e-4 * mach.powi(5) } else { 0.004 * (mach / 0.1).powi(3) }
            .min(self.anatomy.frication_efficiency);
        self.noise_queue[self.pending_noise] = (target, onset * eff * gross, gross);
        self.pending_noise += 1;
    }

    /// Junction j as a series flow element: continuity of volume flow U,
    /// pressure drop `R_v U + kappa U|U|`. With zero resistance it is exactly
    /// the lossless junction; at zero aperture it carries no flow (each side
    /// sees a rigid wall).
    #[inline]
    fn jet_junction(&mut self, j: usize) {
        let dt = self.dt;
        let (a, b) = (self.f[j], self.b[j + 1]);
        let ac = self.aperture[j];
        if ac <= CLOSED_M2 {
            self.nb[j] = a;
            self.nf[j + 1] = b;
            return;
        }
        let (zl, zr) = (1.0 / self.y[j], 1.0 / self.y[j + 1]);
        let zw = zl + zr;
        let rhs = 2.0 * (a - b);
        let kappa = if rhs >= 0.0 { self.k_fwd[j] } else { self.k_back[j] };
        let u = jet_flow(rhs, zw + self.r_visc[j], kappa);
        let loss = (rhs - u * zw) * u * dt;
        self.nb[j] = a - u * zl;
        self.nf[j + 1] = b + u * zr;
        self.flows.constriction_loss += loss;
        self.flows.constriction_dissipation += loss;
        let contact = self.slot_press[j] || self.slot_press[j + 1];
        self.queue_turbulence(u, ac, loss, if u >= 0.0 { j } else { j + 1 }, u >= 0.0, contact);
    }

    /// The lip exit: tube end -> aperture (jet into free air) -> radiating air
    /// plug (mass port + radiation port). Returns emitted pressure.
    #[inline]
    fn lip_exit(&mut self) -> f64 {
        let dt = self.dt;
        let n = self.n;
        let a = self.f[n - 1];
        let (ym, yr) = (self.lips.y_mass, self.lips.y_rad);
        let yl = ym + yr;
        let s = self.lips.s;
        let zt = 1.0 / self.y[n - 1];
        let zw = zt + 1.0 / yl;
        let rhs = 2.0 * a + 2.0 * ym * s / yl;
        let la = self.lip_aperture;
        let u = if la > CLOSED_M2 {
            let kappa = if rhs >= 0.0 { self.lip_k_out } else { self.lip_k_in };
            jet_flow(rhs, zw + self.lip_r_visc, kappa)
        } else {
            0.0
        };
        let loss = (rhs - u * zw) * u * dt;
        self.nb[n - 1] = a - u * zt;
        let p = (u - 2.0 * ym * s) / yl;
        self.lips.s = s + p;
        self.flows.constriction_loss += loss;
        self.flows.constriction_dissipation += loss;
        if la > CLOSED_M2 && u > 0.0 {
            let contact = self.slot_press[n - 1];
            self.queue_turbulence(u, la, loss, n, true, contact);
        }
        p
    }

    /// Pay out queued turbulence: a pressure pair into the landing section
    /// (or straight into the lip radiation), its work taken from the jet that
    /// queued it and never more than that jet's loss this step.
    /// Returns the extra emitted pressure at the lips.
    #[inline]
    fn apply_turbulence(&mut self, p_lip: f64) -> f64 {
        let dt = self.dt;
        let mut extra = 0.0;
        for q in 0..self.pending_noise {
            let (target, want, cap) = self.noise_queue[q];
            let xi = self.turbulent_noise();
            let mut w;
            if target >= self.n {
                let yr = self.lips.y_rad;
                let p = p_lip + extra;
                let mut nz = (want / (yr * dt)).sqrt() * xi;
                w = yr * nz * (2.0 * p + nz) * dt;
                for _ in 0..4 {
                    if w <= cap {
                        break;
                    }
                    nz *= 0.5;
                    w = yr * nz * (2.0 * p + nz) * dt;
                }
                if w > cap {
                    continue;
                }
                extra += nz;
            } else {
                let y = self.y[target];
                let (f, b) = (self.nf[target], self.nb[target]);
                let mut nz = (want / (2.0 * y * dt)).sqrt() * xi;
                w = y * (2.0 * nz * (f + b) + 2.0 * nz * nz) * dt;
                for _ in 0..4 {
                    if w <= cap {
                        break;
                    }
                    nz *= 0.5;
                    w = y * (2.0 * nz * (f + b) + 2.0 * nz * nz) * dt;
                }
                if w > cap {
                    continue;
                }
                self.nf[target] = f + nz;
                self.nb[target] = b + nz;
            }
            self.flows.constriction_loss -= w;
            self.flows.turbulence_work += w;
            if w > 0.0 {
                self.flows.turbulence_positive += w;
            }
        }
        self.pending_noise = 0;
        extra
    }

    /// Render `out.len()` samples and close one ledger frame.
    pub fn process(&mut self, out: &mut [f32]) {
        let energy_start = self.stored_energy();
        let before = self.flows;
        for o in out.iter_mut() {
            self.control_tick();
            let a = self.step();
            let b = self.step();
            let mut sample = 0.5 * (a + b) * self.output_gain;
            if !sample.is_finite() {
                self.non_finite_events += 1;
                sample = 0.0;
                self.recover();
            }
            // Observer-side safety (as ovillo-dsp's AriadneEngine): the
            // World is untouched, the listener's copy is clamped and counted.
            if sample.abs() > 1.0 {
                self.safety_limited_frames += 1;
                sample = sample.clamp(-1.0, 1.0);
            }
            *o = sample as f32;
            self.observe_sample();
        }
        let after = self.flows;
        let energy_end = self.stored_energy();
        self.ledger.push(WorkFrame {
            energy_start,
            source_work: (after.muscle_work - before.muscle_work) + (after.impact_work - before.impact_work),
            control_work: (after.tract_control_work - before.tract_control_work)
                + (after.fold_control_work - before.fold_control_work),
            internal_loss: (after.wall_loss - before.wall_loss)
                + (after.jet_loss - before.jet_loss)
                + (after.fold_loss - before.fold_loss)
                + (after.constriction_loss - before.constriction_loss),
            radiation_loss: (after.radiation_lips - before.radiation_lips)
                + (after.radiation_nose - before.radiation_nose),
            output_work: 0.0,
            energy_end,
        });
    }

    /// A non-finite state is a fault, not a sound: silence the air and the
    /// folds, count it, and keep the books continuous (the discarded energy
    /// is booked as loss).
    fn recover(&mut self) {
        let lost = self.tract_energy() + self.fold.energy(self.anatomy.fold.mass_kg);
        self.f = [0.0; MAX_ORAL_SECTIONS];
        self.b = [0.0; MAX_ORAL_SECTIONS];
        self.nfw = [0.0; MAX_NASAL_SECTIONS];
        self.nbw = [0.0; MAX_NASAL_SECTIONS];
        self.lips.s = 0.0;
        self.nose.s = 0.0;
        self.fold.y = self.fold.rest;
        self.fold.v = 0.0;
        if lost.is_finite() {
            self.flows.wall_loss += lost;
        }
    }

    fn observe_sample(&mut self) {
        self.sample_counter += 1;
        let y = self.fold.y;
        self.gap_min = self.gap_min.min(y);
        self.gap_max = self.gap_max.max(y);
        self.ring[self.ring_pos % CYCLE_RING] = y as f32;
        // A cycle starts when the fold turns from closing to opening; it counts
        // only if the fold actually swung (more than 0.02 mm), so an abducted
        // fold jiggling in a turbulent stream is not mistaken for voicing.
        self.cycle_min = self.cycle_min.min(y);
        self.cycle_max = self.cycle_max.max(y);
        let v = self.fold.v;
        if self.prev_v <= 0.0 && v > 0.0 {
            let period = self.ring_pos.wrapping_sub(self.last_cycle_start);
            let hz = self.sample_rate_hz / period as f64;
            let swung = self.cycle_max - self.cycle_min > 2.0e-5;
            if swung && (20.0..=1_000.0).contains(&hz) && period < CYCLE_RING {
                self.measured_f0 = if self.measured_f0 > 0.0 { 0.7 * self.measured_f0 + 0.3 * hz } else { hz };
                for (k, point) in self.cycle.iter_mut().enumerate() {
                    let idx = self.last_cycle_start + k * period / CYCLE_POINTS;
                    *point = f64::from(self.ring[idx % CYCLE_RING]);
                }
                self.last_voiced = self.ring_pos;
            }
            self.last_cycle_start = self.ring_pos;
            self.cycle_min = y;
            self.cycle_max = y;
        }
        if self.ring_pos.wrapping_sub(self.last_voiced) > CYCLE_RING {
            self.measured_f0 = 0.0;
            self.last_voiced = self.ring_pos;
        }
        self.prev_v = v;
        self.ring_pos = self.ring_pos.wrapping_add(1);
        for i in 0..self.n {
            let p = self.f[i] + self.b[i];
            self.p2[i] += p * p;
        }
        self.rms_count += 1;
    }

    /// Snapshot for presentation; resets the RMS and range windows.
    pub fn readings(&mut self) -> TractReadings {
        let mut r = TractReadings {
            oral_areas_cm2: vec![0.0; self.n],
            nasal_areas_cm2: vec![0.0; self.m],
            oral_pressure_rms: vec![0.0; self.n],
            fold_half_gap_m: 0.0,
            fold_gap_min_m: 0.0,
            fold_gap_max_m: 0.0,
            fold_cycle: [0.0; CYCLE_POINTS],
            glottal_flow_m3_s: 0.0,
            lung_pressure_pa: 0.0,
            lung_fraction: 0.0,
            measured_f0_hz: 0.0,
            tract_energy_j: 0.0,
            fold_energy_j: 0.0,
            lung_energy_j: 0.0,
            flows: Flows::default(),
            ledger_frames: 0,
            max_frame_residual_j: 0.0,
            cumulative_residual_j: 0.0,
            global_residual_j: 0.0,
            non_finite_events: 0,
            safety_limited_frames: 0,
            max_reynolds: 0.0,
            min_aperture_m2: 0.0,
            min_aperture_position: 0.0,
            constrictions: [Constriction::default(); MAX_CONSTRICTIONS],
        };
        self.readings_into(&mut r);
        r
    }

    /// As [`TractWorld::readings`], into preallocated storage (no allocation).
    pub fn readings_into(&mut self, r: &mut TractReadings) {
        for i in 0..self.n.min(r.oral_areas_cm2.len()) {
            r.oral_areas_cm2[i] = self.area[i] * 1.0e4;
            r.oral_pressure_rms[i] = if self.rms_count > 0 { (self.p2[i] / self.rms_count as f64).sqrt() } else { 0.0 };
            self.p2[i] = 0.0;
        }
        for k in 0..self.m.min(r.nasal_areas_cm2.len()) {
            r.nasal_areas_cm2[k] = self.narea[k] * 1.0e4;
        }
        self.rms_count = 0;
        r.fold_half_gap_m = self.fold.y;
        r.fold_gap_min_m = if self.gap_min <= self.gap_max { self.gap_min } else { self.fold.y };
        r.fold_gap_max_m = if self.gap_min <= self.gap_max { self.gap_max } else { self.fold.y };
        self.gap_min = f64::MAX;
        self.gap_max = f64::MIN;
        r.fold_cycle = self.cycle;
        r.glottal_flow_m3_s = self.glottal_flow;
        r.lung_pressure_pa = self.lung_pressure;
        r.lung_fraction = self.lung_energy / self.anatomy.lung.capacity_j;
        r.measured_f0_hz = self.measured_f0;
        r.tract_energy_j = self.tract_energy();
        r.fold_energy_j = self.fold.energy(self.anatomy.fold.mass_kg);
        r.lung_energy_j = self.lung_energy;
        r.flows = self.flows;
        r.ledger_frames = self.ledger.frames;
        r.max_frame_residual_j = self.ledger.max_abs_residual;
        r.cumulative_residual_j = self.ledger.cumulative_residual;
        r.global_residual_j = self.ledger.global_residual();
        r.non_finite_events = self.non_finite_events;
        r.safety_limited_frames = self.safety_limited_frames;
        r.max_reynolds = self.max_re;
        self.max_re = 0.0;
        let mut best = (f64::MAX, 0usize);
        for j in 0..self.n - 1 {
            if self.aperture[j] < best.0 {
                best = (self.aperture[j], j);
            }
        }
        if self.lip_aperture < best.0 {
            best = (self.lip_aperture, self.n - 1);
        }
        r.min_aperture_m2 = best.0;
        r.min_aperture_position = (best.1 as f64 + 1.0) / self.n as f64;
        for k in 0..MAX_CONSTRICTIONS {
            r.constrictions[k] = if self.slots[k].active { self.slots[k].current } else { Constriction::default() };
        }
    }

    /// Acoustic pressure in one oral section (Pa): both travelling waves.
    #[must_use]
    pub fn oral_pressure(&self, i: usize) -> f64 {
        if i >= self.n {
            return 0.0;
        }
        self.f[i] + self.b[i]
    }

    /// Acoustic energy stored in one oral section (J).
    #[must_use]
    pub fn oral_section_energy(&self, i: usize) -> f64 {
        if i >= self.n {
            return 0.0;
        }
        self.y[i] * (self.f[i] * self.f[i] + self.b[i] * self.b[i]) * self.dt
    }

    /// Place (`Some`) or release (`None`) constriction `slot`. Releasing lets
    /// the wall return at the release time constant; slot 0 is driven by
    /// gestures while one runs.
    pub fn set_constriction(&mut self, slot: usize, constriction: Option<Constriction>) {
        if slot >= MAX_CONSTRICTIONS {
            return;
        }
        let s = &mut self.slots[slot];
        match constriction {
            Some(c) => {
                let c = Constriction {
                    position: finite_or(c.position, 0.9).clamp(0.0, 1.0),
                    degree: finite_or(c.degree, 0.0).clamp(0.0, 1.0),
                };
                if !s.active {
                    s.current = Constriction { position: c.position, degree: 0.0 };
                    s.active = true;
                }
                s.target = c;
            }
            None => s.target.degree = 0.0,
        }
    }

    #[must_use]
    pub fn constriction(&self, slot: usize) -> Option<Constriction> {
        self.slots.get(slot).filter(|s| s.active).map(|s| s.current)
    }

    /// Fold-tension modulation (booked as fold control work like any retune).
    pub fn set_vibrato(&mut self, v: Vibrato) {
        self.vibrato = Vibrato {
            rate_hz: finite_or(v.rate_hz, 5.5).clamp(0.1, 12.0),
            depth_semitones: finite_or(v.depth_semitones, 0.0).clamp(0.0, 2.0),
            wobble: finite_or(v.wobble, 0.0).clamp(0.0, 1.0),
        };
    }

    #[must_use]
    pub fn vibrato(&self) -> Vibrato {
        self.vibrato
    }

    /// The tube's own response: glottis rigid, one volume-velocity impulse
    /// `u0` (m³/s for one step) at the glottis, the configuration snapped to
    /// `articulation`. Returns emitted pressure per tract step (rate 2·fs).
    pub fn impulse_response(&mut self, articulation: Articulation, steps: usize, u0: f64) -> Vec<f64> {
        let mut controls = self.target;
        controls.articulation = articulation;
        controls.breath = 0.0;
        self.set_controls(controls);
        self.f = [0.0; MAX_ORAL_SECTIONS];
        self.b = [0.0; MAX_ORAL_SECTIONS];
        self.nfw = [0.0; MAX_NASAL_SECTIONS];
        self.nbw = [0.0; MAX_NASAL_SECTIONS];
        self.lips.s = 0.0;
        self.nose.s = 0.0;
        self.snap_configuration();
        let mode = self.glottis_mode;
        self.glottis_mode = GlottisMode::Rigid;
        self.pending_impulse = u0;
        let mut out = Vec::with_capacity(steps);
        for _ in 0..steps {
            out.push(self.step());
        }
        self.glottis_mode = mode;
        out
    }
}

/// Move one section's admittance from `y_old` to `y_new` holding its excess
/// air volume `q = C p` (the air does not appear or vanish when a wall moves)
/// and its volume flow `U` (continuity): pressure and flow both scale by
/// `A_old / A_new`, so both travelling waves do. Returns the change of stored
/// energy, which the articulators paid: squeezing a pressurised or flowing
/// section costs work, opening it gets work back.
#[inline]
fn transport_section(f: &mut f64, b: &mut f64, y_old: f64, y_new: f64, dt: f64) -> f64 {
    if y_new == y_old {
        return 0.0;
    }
    let before = y_old * (*f * *f + *b * *b);
    let r = y_old / y_new;
    *f *= r;
    *b *= r;
    (y_new * (*f * *f + *b * *b) - before) * dt
}

/// Solve `kappa U|U| + z U = rhs` for U (U has the sign of rhs); the
/// cancellation-free root. `kappa = inf` means a closed aperture.
#[inline]
fn jet_flow(rhs: f64, z: f64, kappa: f64) -> f64 {
    if !kappa.is_finite() || !z.is_finite() {
        return 0.0;
    }
    let lin = rhs / z;
    // Negligible quadratic term: the linear element (still passive; the loss
    // booked is always computed from the flow actually used).
    if kappa <= 0.0 || kappa * lin.abs() < 1.0e-4 * z {
        return lin;
    }
    let root = (z * z + 4.0 * kappa * rhs.abs()).sqrt();
    2.0 * rhs / (z + root)
}

/// Unit-variance gain for two cascaded one-pole lowpasses `y += k (x - y)`
/// on unit-variance white noise (sum of the squared impulse response).
fn two_pole_norm(k: f64) -> f64 {
    let a = 1.0 - k;
    // h[n] = k² (n + 1) aⁿ; sum h² = k⁴ (1 + a²) / (1 - a²)³.
    let sum = k.powi(4) * (1.0 + a * a) / (1.0 - a * a).powi(3);
    1.0 / sum.sqrt()
}

#[inline]
fn smoothstep(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// One-pole approach; snaps when within 1e-9 so a settled World stops moving.
/// Returns whether the value moved.
#[inline]
fn approach(value: &mut f64, target: f64, k: f64) -> bool {
    let d = target - *value;
    if d == 0.0 {
        return false;
    }
    if d.abs() < 1.0e-9 * (1.0 + target.abs()) {
        *value = target;
    } else {
        *value += d * k;
    }
    true
}

#[inline]
fn wobble(age_s: f64) -> f64 {
    (-age_s / WOBBLE_DECAY_S).exp() * (TAU * WOBBLE_HZ * age_s).cos()
}

#[inline]
fn hann(k: usize, len: usize) -> f64 {
    0.5 - 0.5 * (TAU * (k as f64 + 0.5) / len as f64).cos()
}

fn one_pole(tau_s: f64, sample_rate_hz: f64) -> f64 {
    1.0 - (-1.0 / (tau_s * sample_rate_hz)).exp()
}

fn finite_or(v: f64, d: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        d
    }
}

fn sanitize(c: TractControls, a: &TractAnatomy) -> TractControls {
    let d = TractControls::default();
    TractControls {
        articulation: Articulation {
            tongue_index: finite_or(c.articulation.tongue_index, d.articulation.tongue_index).clamp(0.0, 1.0),
            tongue_diameter: finite_or(c.articulation.tongue_diameter, d.articulation.tongue_diameter).clamp(0.0, 1.0),
            lips: finite_or(c.articulation.lips, d.articulation.lips).clamp(0.0, 1.0),
            velum: finite_or(c.articulation.velum, d.articulation.velum).clamp(0.0, 1.0),
        },
        pitch_hz: finite_or(c.pitch_hz, d.pitch_hz).clamp(a.fold.min_tuning_hz, a.fold.max_tuning_hz),
        tenseness: finite_or(c.tenseness, d.tenseness).clamp(0.0, 1.0),
        breath: finite_or(c.breath, 0.0).clamp(0.0, 1.0),
        aspiration: finite_or(c.aspiration, d.aspiration).clamp(0.0, 1.0),
    }
}
