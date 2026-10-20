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
//!   -> oral tube: Kelly-Lochbaum sections in pressure waves, lossless
//!      admittance-weighted scattering, uniform plane-wave loss
//!   -> velum: a lossless three-port junction into the nasal tube
//!   -> lips and nostrils: radiating air plugs (stored end mass + a
//!      radiation port that removes what it emits, once)
//!   -> listener: reads the emitted radiation pressure, never loads it
//! ```
//!
//! Stored energy: every travelling-wave sample `w` in a section with
//! admittance `Y = A / (rho c)` holds `Y w² dt`; each air plug holds `Y_m s² dt`;
//! the folds hold `½ m v² + ½ k (y - y_rest)² + ½ k_c min(y, 0)²`; the lungs
//! hold their reservoir. The configuration (areas, fold tension, adduction) is
//! moved with the state held fixed and the change of stored energy is booked
//! as control work, the connection Ariadne uses for frets and slides
//! (`mus_ariadne::Body::set_configuration_amount`).

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
        for i in 0..self.n {
            let a = self.goal_scratch[i] * 1.0e-4;
            let y_new = a / self.rho_c;
            work += (y_new - self.y[i]) * (self.f[i] * self.f[i] + self.b[i] * self.b[i]) * self.dt;
            self.area[i] = a;
            self.y[i] = y_new;
            self.area_step[i] = 0.0;
        }
        for k in 0..self.m {
            let a = self.ngoal_scratch[k] * 1.0e-4;
            let y_new = a / self.rho_c;
            work += (y_new - self.ny[k]) * (self.nfw[k] * self.nfw[k] + self.nbw[k] * self.nbw[k]) * self.dt;
            self.narea[k] = a;
            self.ny[k] = y_new;
            self.narea_step[k] = 0.0;
        }
        work += self.refresh_junctions();
        work += self.retune_fold();
        self.flows.tract_control_work += work;
    }

    fn compute_goal_areas(&mut self) {
        let n = self.n;
        let m = self.m;
        let art = self.current.articulation;
        self.anatomy.oral_areas_cm2(&art, &mut self.goal_scratch[..n]);
        // Orb dents: log-area pushed in at the hit, wobbling back.
        let mut velum_flap = 0.0;
        for imp in self.impacts.iter().filter(|i| i.active) {
            let w = imp.dent * wobble(imp.age_s);
            velum_flap += w.abs() * crate::anatomy::gaussian(imp.center, self.anatomy.velum_position, 0.08);
            for (i, a) in self.goal_scratch[..n].iter_mut().enumerate() {
                let x = (i as f64 + 0.5) / n as f64;
                let g = crate::anatomy::gaussian(x, imp.center, DENT_WIDTH);
                *a = (*a * (-w * g).exp()).clamp(crate::anatomy::AREA_FLOOR_CM2, crate::anatomy::AREA_CEIL_CM2);
            }
        }
        self.anatomy.nasal_areas_cm2((art.velum + velum_flap).min(1.0), &mut self.ngoal_scratch[..m]);
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
        work
    }

    /// Fold tension (pitch) and adduction (tenseness) at fixed fold state.
    fn retune_fold(&mut self) -> f64 {
        let fa = self.anatomy.fold;
        let before = self.fold.energy(fa.mass_kg);
        let omega = TAU * self.current.pitch_hz * (1.0 + self.larynx_jiggle());
        self.fold.stiffness = fa.mass_kg * omega * omega;
        self.fold.contact_stiffness = fa.contact_stiffness_ratio * self.fold.stiffness;
        self.fold.damping = 2.0 * fa.damping_ratio * fa.mass_kg * omega;
        self.fold.contact_damping = 2.0 * fa.contact_damping_ratio * fa.mass_kg * omega;
        let t = self.current.tenseness;
        self.fold.rest = fa.rest_half_gap_breathy_m * (fa.rest_half_gap_pressed_m / fa.rest_half_gap_breathy_m).powf(t);
        self.fold.energy(fa.mass_kg) - before
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
        if let Some(g) = self.gesture {
            if let Some(c) = g.controls_at(self.gesture_time) {
                self.target = sanitize(c, &self.anatomy);
            } else {
                self.gesture = None;
            }
            self.gesture_time += audio_dt;
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
        if moving_art || impacts {
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
            }
            self.stride_phase = (self.stride_phase + 1) % AREA_STRIDE;
            self.area_moving -= 1;

            // Transport: the area function moves under fixed waves; the change
            // of stored energy is the work the articulators did.
            let mut work = 0.0;
            let inv_rc = 1.0 / self.rho_c;
            for i in 0..self.n {
                let a = self.area[i] + self.area_step[i];
                let y_new = a * inv_rc;
                work += (y_new - self.y[i]) * (self.f[i] * self.f[i] + self.b[i] * self.b[i]);
                self.area[i] = a;
                self.y[i] = y_new;
            }
            for k in 0..self.m {
                let a = self.narea[k] + self.narea_step[k];
                let y_new = a * inv_rc;
                work += (y_new - self.ny[k]) * (self.nfw[k] * self.nfw[k] + self.nbw[k] * self.nbw[k]);
                self.narea[k] = a;
                self.ny[k] = y_new;
            }
            work *= self.dt;
            work += self.refresh_junctions();
            self.flows.tract_control_work += work;
        } else {
            self.stride_phase = 0;
        }
        if moving_fold || impacts || self.fold_moving {
            self.flows.fold_control_work += self.retune_fold();
            self.fold_moving = moving_fold || impacts;
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
            let p = self.wl[j] * self.f[j] + self.wr[j] * self.b[j + 1];
            self.nb[j] = p - self.f[j];
            self.nf[j + 1] = p - self.b[j + 1];
        }
        {
            let j = jv - 1;
            let p = self.w3[0] * self.f[j] + self.w3[1] * self.b[jv] + self.w3[2] * self.nbw[0];
            self.nb[j] = p - self.f[j];
            self.nf[jv] = p - self.b[jv];
            self.nnf[0] = p - self.nbw[0];
        }
        for k in 0..m - 1 {
            let p = self.nwl[k] * self.nfw[k] + self.nwr[k] * self.nbw[k + 1];
            self.nnb[k] = p - self.nfw[k];
            self.nnf[k + 1] = p - self.nbw[k + 1];
        }
        let (refl, p_lip) = self.lips.scatter(self.f[n - 1]);
        self.nb[n - 1] = refl;
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
                + (after.fold_loss - before.fold_loss),
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
        // A cycle starts when the fold turns from closing to opening.
        let v = self.fold.v;
        if self.prev_v <= 0.0 && v > 0.0 {
            let period = self.ring_pos.wrapping_sub(self.last_cycle_start);
            let hz = self.sample_rate_hz / period as f64;
            if (20.0..=1_500.0).contains(&hz) && period < CYCLE_RING {
                self.measured_f0 = if self.measured_f0 > 0.0 { 0.7 * self.measured_f0 + 0.3 * hz } else { hz };
                for (k, point) in self.cycle.iter_mut().enumerate() {
                    let idx = self.last_cycle_start + k * period / CYCLE_POINTS;
                    *point = f64::from(self.ring[idx % CYCLE_RING]);
                }
            }
            self.last_cycle_start = self.ring_pos;
        }
        if self.ring_pos.wrapping_sub(self.last_cycle_start) > CYCLE_RING {
            self.measured_f0 = 0.0;
            self.last_cycle_start = self.ring_pos;
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
