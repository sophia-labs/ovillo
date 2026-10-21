//! The tract as a `mus_ariadne::World`: one N-D body, the World's own ledger.
//!
//! ```text
//! lungs = the body's finite reservoir (refilled by the breathing muscles: work_input)
//!   -> flow valve (mus_ariadne::law::valve): fold mass + tension spring + contact,
//!      jet through the slit into oral volume 0, exact work p_bar U dt
//!   -> 24 modes of the branched oral+nasal tube network, recomputed by
//!      mus_ariadne at every configuration point (tongue index, tongue
//!      diameter, lips, velum)
//!   -> wall loss and radiation at lips and nostrils, diagonal in the modes
//!   -> listener: the pressure at the two radiating volumes (non-loading)
//! ```
//!
//! This wrapper only decides *when* to move: the articulators approach their
//! targets at finite speed (the tube tract's one-pole time constants), and the
//! configuration point is handed to the World every `update_interval`
//! samples. Every move is a `World::set_body_configuration_point` with its
//! work receipt; every sample is one `World::process_constitutive_sample`.

use std::f64::consts::TAU;

use mus_ariadne::{
    BodyControls, ConnectionPolicy, ConstitutiveBodyControls, DisposalPolicy, FlowValveControls,
    SpaceBodyControls, World, WorldControls, WorldError,
};

use crate::anatomy::TractNdAnatomy;
use crate::gestures::Gesture;

/// Constriction slots (slot 0 for gestures, 1-4 for touches), as `ovillo-tract`.
pub const MAX_SLOTS: usize = 5;
const MAX_POINT: usize = 4 + 2 * MAX_SLOTS;
/// Points in the stroboscope cycle snapshot, as `ovillo-tract`.
pub const CYCLE_POINTS: usize = 32;
const CYCLE_RING: usize = 2048;
const KNOCK_SAMPLES: usize = 24;
const MAX_KNOCKS: usize = 6;
/// Pressure-RMS readout: one sample of every oral volume every this many samples.
const PRMS_STRIDE: usize = 64;

/// A constriction anywhere along the oral tract: `position` 0 glottis .. 1
/// lips, `degree` 0 none .. 1 closed (as `ovillo-tract`'s `Constriction`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Constriction {
    pub position: f64,
    pub degree: f64,
}

/// Fold-tension modulation (as `ovillo-tract`'s `Vibrato`), booked as valve
/// retuning work.
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

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Knock {
    node: usize,
    amplitude: f64,
    k: usize,
    active: bool,
}

/// What the page reads besides the World's own receipts (non-loading).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Observers {
    pub measured_f0_hz: f64,
    pub fold_cycle_m: [f64; CYCLE_POINTS],
    pub gap_min_m: f64,
    pub gap_max_m: f64,
    pub max_reynolds: f64,
    pub safety_limited: u64,
    pub non_finite: u64,
    pub knocks: u64,
    pub knock_work: f64,
}

/// Where the articulators are (each in 0..1), as `ovillo-tract`'s.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Articulation {
    pub tongue_index: f64,
    pub tongue_diameter: f64,
    pub lips: f64,
    pub velum: f64,
}

impl Articulation {
    #[must_use]
    pub fn point(self) -> [f64; 4] {
        [self.tongue_index, self.tongue_diameter, self.lips, self.velum]
    }
}

/// `ovillo-tract` `presets.rs` fitted articulations (a fit there, against the
/// tube tract; copied, not refitted here).
pub const ART_A: Articulation = Articulation { tongue_index: 0.060, tongue_diameter: 0.730, lips: 0.560, velum: 0.0 };
pub const ART_I: Articulation = Articulation { tongue_index: 0.910, tongue_diameter: 0.740, lips: 0.410, velum: 0.0 };
pub const ART_U: Articulation = Articulation { tongue_index: 0.475, tongue_diameter: 0.975, lips: 0.025, velum: 0.0 };
pub const ART_M: Articulation = Articulation { tongue_index: 0.060, tongue_diameter: 0.730, lips: 0.0, velum: 1.0 };
pub const ART_REST: Articulation = Articulation { tongue_index: 0.5, tongue_diameter: 0.3, lips: 0.6, velum: 0.35 };
/// The public engine's vowels (`TractNdAnatomy::public_v1`, whose velum seals
/// exactly): a fit in this World (`examples/fit_public_presets.rs`, receipt
/// `receipts/public-preset-fit.json`) started from ovillo-tract fe4fb03's
/// sealed-velum re-fit; /u/ needs a 0.01 velopharyngeal opening, as the
/// tube tract's needs 0.015.
pub const PUBLIC_ART_A: Articulation = Articulation { tongue_index: 0.060, tongue_diameter: 0.730, lips: 0.555, velum: 0.0 };
pub const PUBLIC_ART_I: Articulation = Articulation { tongue_index: 0.890, tongue_diameter: 0.685, lips: 0.730, velum: 0.0 };
pub const PUBLIC_ART_U: Articulation = Articulation { tongue_index: 0.435, tongue_diameter: 0.920, lips: 0.036, velum: 0.010 };
pub const PUBLIC_ART_M: Articulation = Articulation { lips: 0.0, velum: 1.0, ..PUBLIC_ART_A };
/// GARGANTA rung-1 vowel-body F1-F3 (`mus-rs/crates/mus-dsp/src/body_profiles.rs`,
/// "design profiles, not measurements").
pub const TARGET_A_HZ: [f64; 3] = [710.0, 1150.0, 2500.0];
pub const TARGET_I_HZ: [f64; 3] = [290.0, 2250.0, 2950.0];
pub const TARGET_U_HZ: [f64; 3] = [310.0, 700.0, 2350.0];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TractControls {
    pub articulation: Articulation,
    pub pitch_hz: f64,
    pub tenseness: f64,
    pub breath: f64,
}

impl Default for TractControls {
    fn default() -> Self {
        Self {
            articulation: Articulation { tongue_index: 0.5, tongue_diameter: 0.3, lips: 0.66, velum: 0.0 },
            pitch_hz: 120.0,
            tenseness: 0.6,
            breath: 0.0,
        }
    }
}

/// Cumulative booking of the moves this wrapper made.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MoveStats {
    pub moves: u64,
    pub control_work: f64,
    pub disposal: f64,
    pub valve_retunes: u64,
    pub valve_work: f64,
}

pub struct TractWorldNd {
    world: World,
    anatomy: TractNdAnatomy,
    sample_rate_hz: f64,
    target: TractControls,
    current: TractControls,
    k_art: f64,
    k_pitch: f64,
    k_tense: f64,
    k_breath: f64,
    /// Samples between configuration moves while the articulators travel.
    pub update_interval: usize,
    /// Skip a scheduled move while the articulators are still travelling if
    /// no coordinate has moved more than this since the last move (0: move at
    /// every interval). The landing move is never skipped. A scheduling
    /// choice of this wrapper, not physics; default 1e-3 (measured in
    /// `examples/zipper.rs`: same spectrum error as moving every 32 samples).
    pub min_move: f64,
    last_moved: [f64; 4],
    phase: usize,
    pending_move: bool,
    pending_tune: bool,
    controls: [ConstitutiveBodyControls; 1],
    world_controls: WorldControls,
    pub output_gain: f64,
    stats: MoveStats,
    // Consonants and expression (inactive: arithmetic identical to before).
    dims: usize,
    point: [f64; MAX_POINT],
    moved_point: [f64; MAX_POINT],
    slots: [Slot; MAX_SLOTS],
    k_close: f64,
    k_release: f64,
    k_slot_position: f64,
    vibrato: Vibrato,
    vibrato_phase: f64,
    clock_s: f64,
    gesture: Option<Gesture>,
    gesture_time: f64,
    aspiration: f64,
    knocks: [Knock; MAX_KNOCKS],
    // Observers.
    ring: [f32; CYCLE_RING],
    ring_pos: usize,
    last_cycle_start: usize,
    prev_gap: f64,
    prev_dg: f64,
    obs: Observers,
    prms: Vec<f64>,
    prms_count: u64,
    prms_phase: usize,
}

impl TractWorldNd {
    pub fn new(sample_rate_hz: f64) -> Result<Self, WorldError> {
        Self::with(
            sample_rate_hz,
            TractNdAnatomy::default(),
            ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate },
            ART_REST,
        )
    }

    pub fn with(
        sample_rate_hz: f64,
        anatomy: TractNdAnatomy,
        connection: ConnectionPolicy,
        initial: Articulation,
    ) -> Result<Self, WorldError> {
        let plan = anatomy.plan(sample_rate_hz, connection, initial.point());
        let world = World::from_constitutive_plan(plan)?;
        let mut current = TractControls::default();
        current.articulation = initial;
        current.pitch_hz = anatomy.valve.initial_tuning_hz;
        current.tenseness = anatomy.valve.initial_adduction;
        let mut world_controls = WorldControls::default();
        world_controls.space.coupling = 0.0;
        let one_pole = |tau: f64| 1.0 - (-1.0 / (tau * sample_rate_hz)).exp();
        Ok(Self {
            world,
            sample_rate_hz,
            target: current,
            current,
            k_art: one_pole(0.025),
            k_pitch: one_pole(0.030),
            k_tense: one_pole(0.050),
            k_breath: one_pole(0.040),
            update_interval: 32,
            min_move: 1.0e-3,
            last_moved: initial.point(),
            phase: 0,
            pending_move: false,
            pending_tune: false,
            controls: [ConstitutiveBodyControls::ModalSpace(SpaceBodyControls {
                body: BodyControls { work_input: anatomy.lung_refill_w, ..Default::default() },
                valve: FlowValveControls { supply: 0.0 },
                radiation_scale: 1.0,
                node_source: None,
            })],
            world_controls,
            output_gain: 1.0 / 2_500.0,
            stats: MoveStats::default(),
            dims: anatomy.dimensions(),
            point: initial_point(initial),
            moved_point: initial_point(initial),
            slots: [Slot::default(); MAX_SLOTS],
            k_close: one_pole(0.008),
            k_release: one_pole(0.005),
            k_slot_position: one_pole(0.015),
            vibrato: Vibrato::default(),
            vibrato_phase: 0.0,
            clock_s: 0.0,
            gesture: None,
            gesture_time: 0.0,
            aspiration: 0.0,
            knocks: [Knock::default(); MAX_KNOCKS],
            ring: [0.0; CYCLE_RING],
            ring_pos: 0,
            last_cycle_start: 0,
            prev_gap: 0.0,
            prev_dg: 0.0,
            obs: Observers {
                measured_f0_hz: 0.0,
                fold_cycle_m: [0.0; CYCLE_POINTS],
                gap_min_m: f64::MAX,
                gap_max_m: f64::MIN,
                max_reynolds: 0.0,
                safety_limited: 0,
                non_finite: 0,
                knocks: 0,
                knock_work: 0.0,
            },
            prms: vec![0.0; anatomy.oral_sections],
            prms_count: 0,
            prms_phase: 0,
            anatomy,
        })
    }

    /// Place (`Some`) or release (`None`) constriction `slot`; it closes and
    /// releases at `ovillo-tract`'s speeds (8 ms, 5 ms). Slot 0 belongs to
    /// gestures while one runs. Without slots in the anatomy this is a no-op.
    pub fn set_constriction(&mut self, slot: usize, constriction: Option<Constriction>) {
        if slot >= MAX_SLOTS.min(self.anatomy.constriction_slots) {
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

    /// Kept for the ABI; the World has no glottal turbulence source (a gap).
    pub fn set_aspiration(&mut self, value: f64) {
        self.aspiration = finite_or(value, 0.0).clamp(0.0, 1.0);
    }

    #[must_use]
    pub fn aspiration(&self) -> f64 {
        self.aspiration
    }

    pub fn set_gesture(&mut self, gesture: Option<Gesture>) {
        if gesture.is_none() && self.gesture.is_some() {
            self.set_constriction(0, None);
        }
        self.gesture = gesture;
        self.gesture_time = 0.0;
    }

    #[must_use]
    pub fn gesture_active(&self) -> bool {
        self.gesture.is_some()
    }

    #[must_use]
    pub fn controls(&self) -> TractControls {
        self.target
    }

    #[must_use]
    pub fn observers(&self) -> Observers {
        self.obs
    }

    /// Mean square pressure (Pa²) per oral volume since the last call; resets.
    pub fn take_pressure_ms(&mut self, out: &mut [f64]) {
        let count = self.prms_count.max(1) as f64;
        for (o, p) in out.iter_mut().zip(&mut self.prms) {
            *o = *p / count;
            *p = 0.0;
        }
        self.prms_count = 0;
    }

    /// Reset the gap and Reynolds windows (after a snapshot).
    pub fn reset_windows(&mut self) {
        self.obs.gap_min_m = f64::MAX;
        self.obs.gap_max_m = f64::MIN;
        self.obs.max_reynolds = 0.0;
    }

    /// An orb knocks the wall at `position` (0 glottis .. 1 lips) with about
    /// `energy_j`: a raised-cosine volume-velocity pulse into that volume
    /// (0.5 ms), its exact work booked as source work by the World. (The
    /// tube tract's wall dent that wobbles back is not ported.)
    pub fn impact(&mut self, position: f64, energy_j: f64) {
        let position = finite_or(position, 0.5).clamp(0.0, 1.0);
        let energy = finite_or(energy_j, 0.0).clamp(0.0, 2.0e-3);
        if energy <= 0.0 {
            return;
        }
        let n = self.anatomy.oral_sections;
        let node = ((position * n as f64) as usize).min(n - 1);
        let Some(shape) = self.world.space_body(0).and_then(|b| b.node_shape(node)) else { return };
        let norm2: f64 = shape.iter().map(|v| v * v).sum();
        let dt = 1.0 / self.sample_rate_hz;
        let hann2: f64 = (0..KNOCK_SAMPLES).map(|k| hann(k, KNOCK_SAMPLES).powi(2)).sum();
        // The pulse's self-work 1/2 dt^2 |phi|^2 U^2 summed: about `energy`.
        let amplitude = (2.0 * energy / (dt * dt * norm2 * hann2)).sqrt();
        let slot = self.knocks.iter().position(|k| !k.active).unwrap_or(0);
        self.knocks[slot] = Knock { node, amplitude, k: 0, active: true };
        self.obs.knocks += 1;
    }

    #[must_use]
    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    #[must_use]
    pub fn anatomy(&self) -> &TractNdAnatomy {
        &self.anatomy
    }

    /// Cumulative energies of the tract body (valve, walls, radiation, air).
    #[must_use]
    pub fn totals(&self) -> mus_ariadne::SpaceTotals {
        self.world.space_body(0).and_then(|b| b.totals()).unwrap_or_default()
    }

    #[must_use]
    pub fn stats(&self) -> MoveStats {
        self.stats
    }

    #[must_use]
    pub fn current(&self) -> TractControls {
        self.current
    }

    pub fn set_controls(&mut self, controls: TractControls) {
        self.target = controls;
    }

    /// Exact-limit knob: 0 removes radiation, 1 is the law.
    pub fn set_radiation_scale(&mut self, scale: f64) {
        if let ConstitutiveBodyControls::ModalSpace(c) = &mut self.controls[0] {
            c.radiation_scale = scale;
        }
    }

    /// Jump straight to the target (one receipted move, no travel).
    pub fn snap(&mut self) -> Result<(), WorldError> {
        self.current = self.target;
        self.move_now()?;
        self.retune_now()?;
        Ok(())
    }

    fn move_now(&mut self) -> Result<(), WorldError> {
        self.last_moved = self.current.articulation.point();
        self.fill_point();
        self.moved_point = self.point;
        let frame = self.world.set_body_configuration_point(0, &self.point[..self.dims])?;
        if frame.modes_recomputed {
            self.stats.moves += 1;
        }
        self.stats.control_work += frame.control_work;
        self.stats.disposal += frame.disposal;
        Ok(())
    }

    fn fill_point(&mut self) {
        let a = self.current.articulation.point();
        self.point[..4].copy_from_slice(&a);
        for k in 0..self.anatomy.constriction_slots.min(MAX_SLOTS) {
            let slot = &self.slots[k];
            self.point[4 + 2 * k] = slot.current.position;
            self.point[5 + 2 * k] = if slot.active { slot.current.degree } else { 0.0 };
        }
    }

    /// Vibrato and wobble as a frequency ratio of the fold tuning (exactly
    /// `ovillo-tract`'s `tension_modulation`; 1 when both are off).
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

    fn retune_now(&mut self) -> Result<(), WorldError> {
        let pitch = self.current.pitch_hz * self.tension_modulation();
        let work = self.world.set_body_valve_tuning(0, pitch, self.current.tenseness)?;
        if work != 0.0 {
            self.stats.valve_retunes += 1;
            self.stats.valve_work += work;
        }
        Ok(())
    }

    /// One audio sample (the World's clock is the audio clock).
    pub fn tick(&mut self) -> Result<f64, WorldError> {
        let dt = 1.0 / self.sample_rate_hz;
        let modulated = self.vibrato.depth_semitones > 0.0 || self.vibrato.wobble > 0.0;
        if modulated {
            self.clock_s += dt;
            self.vibrato_phase = (self.vibrato_phase + self.vibrato.rate_hz * dt).fract();
        }
        if let Some(g) = self.gesture {
            if let Some(c) = g.controls_at(self.gesture_time) {
                self.target = c;
                let k = g.constriction_at(self.gesture_time);
                if k.degree > 0.0 || self.slots[0].active {
                    self.set_constriction(0, Some(k));
                }
            } else {
                self.gesture = None;
                self.set_constriction(0, None);
            }
            self.gesture_time += dt;
        }
        let mut moving_slots = false;
        for slot in self.slots.iter_mut().filter(|s| s.active) {
            let k = if slot.target.degree > slot.current.degree { self.k_close } else { self.k_release };
            moving_slots |= approach(&mut slot.current.degree, slot.target.degree, k);
            moving_slots |= approach(&mut slot.current.position, slot.target.position, self.k_slot_position);
            if slot.target.degree == 0.0 && slot.current.degree < 1.0e-6 {
                slot.current.degree = 0.0;
                slot.active = false;
                moving_slots = true;
            }
        }
        let (t, c) = (self.target, &mut self.current);
        let moving = approach(&mut c.articulation.tongue_index, t.articulation.tongue_index, self.k_art)
            | approach(&mut c.articulation.tongue_diameter, t.articulation.tongue_diameter, self.k_art)
            | approach(&mut c.articulation.lips, t.articulation.lips, self.k_art)
            | approach(&mut c.articulation.velum, t.articulation.velum, self.k_art);
        let tuning = approach(&mut c.pitch_hz, t.pitch_hz, self.k_pitch) | approach(&mut c.tenseness, t.tenseness, self.k_tense);
        approach(&mut c.breath, t.breath, self.k_breath);
        let moving = moving | moving_slots;
        self.pending_move |= moving;
        self.pending_tune |= tuning | modulated;
        if self.phase == 0 {
            if self.pending_move {
                let point = self.current.articulation.point();
                let mut delta = (0..4).map(|i| (point[i] - self.last_moved[i]).abs()).fold(0.0, f64::max);
                if moving_slots {
                    self.fill_point();
                    delta = (4..self.dims).map(|i| (self.point[i] - self.moved_point[i]).abs()).fold(delta, f64::max);
                }
                if !moving || delta > self.min_move {
                    self.move_now()?;
                    self.pending_move = moving;
                }
            }
            if self.pending_tune {
                self.retune_now()?;
                self.pending_tune = tuning | modulated;
            }
        }
        self.phase = (self.phase + 1) % self.update_interval.max(1);
        let mut knock = None;
        for kn in self.knocks.iter_mut().filter(|k| k.active) {
            if knock.is_none() {
                knock = Some((kn.node, kn.amplitude * hann(kn.k, KNOCK_SAMPLES)));
                kn.k += 1;
                if kn.k >= KNOCK_SAMPLES {
                    kn.active = false;
                }
            }
        }
        if let ConstitutiveBodyControls::ModalSpace(controls) = &mut self.controls[0] {
            controls.valve.supply = self.current.breath;
            controls.node_source = knock;
        }
        self.world.process_constitutive_sample(&self.controls, self.world_controls)?;
        let emitted = self.world.body_radiation(0).unwrap_or(&[]);
        let pressure: f64 = emitted.iter().sum();
        self.observe();
        Ok(pressure * self.output_gain)
    }

    fn observe(&mut self) {
        let Some(body) = self.world.space_body(0) else { return };
        let (gap, reynolds) = body.valve_gap_and_reynolds();
        self.obs.gap_min_m = self.obs.gap_min_m.min(gap);
        self.obs.gap_max_m = self.obs.gap_max_m.max(gap);
        self.obs.max_reynolds = self.obs.max_reynolds.max(reynolds);
        self.ring[self.ring_pos % CYCLE_RING] = gap as f32;
        // A cycle starts when the fold turns from closing to opening.
        let dg = gap - self.prev_gap;
        if self.prev_dg <= 0.0 && dg > 0.0 {
            let period = self.ring_pos.wrapping_sub(self.last_cycle_start);
            let hz = self.sample_rate_hz / period as f64;
            // A cycle counts only if the fold really swung (0.05 mm peak to
            // peak); a still fold jiggled by turbulence is not voicing.
            let swing = if period < CYCLE_RING && period > 0 {
                let (mut lo, mut hi) = (f32::MAX, f32::MIN);
                for k in 0..period {
                    let v = self.ring[(self.last_cycle_start + k) % CYCLE_RING];
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
                f64::from(hi - lo)
            } else {
                0.0
            };
            if swing < 5.0e-5 {
                self.obs.measured_f0_hz *= 0.7;
                if self.obs.measured_f0_hz < 20.0 {
                    self.obs.measured_f0_hz = 0.0;
                }
            } else if (20.0..=1_500.0).contains(&hz) && period < CYCLE_RING {
                self.obs.measured_f0_hz = if self.obs.measured_f0_hz > 0.0 { 0.7 * self.obs.measured_f0_hz + 0.3 * hz } else { hz };
                for (k, point) in self.obs.fold_cycle_m.iter_mut().enumerate() {
                    let idx = self.last_cycle_start + k * period / CYCLE_POINTS;
                    *point = f64::from(self.ring[idx % CYCLE_RING]);
                }
            }
            self.last_cycle_start = self.ring_pos;
        }
        if self.ring_pos.wrapping_sub(self.last_cycle_start) > CYCLE_RING {
            self.obs.measured_f0_hz = 0.0;
            self.last_cycle_start = self.ring_pos;
        }
        self.prev_gap = gap;
        self.prev_dg = dg;
        self.ring_pos = self.ring_pos.wrapping_add(1);
        self.prms_phase += 1;
        if self.prms_phase >= PRMS_STRIDE {
            self.prms_phase = 0;
            for (i, acc) in self.prms.iter_mut().enumerate() {
                let p = body.node_pressure(i).unwrap_or(0.0);
                *acc += p * p;
            }
            self.prms_count += 1;
        }
    }

    /// Render into `out` (f32, clamped only on the listener's copy). A World
    /// error is a fault, not a sound: the World is rebuilt silent and counted.
    pub fn process(&mut self, out: &mut [f32]) -> Result<(), WorldError> {
        for sample in out.iter_mut() {
            let v = self.tick()?;
            if v.abs() > 1.0 {
                self.obs.safety_limited += 1;
            }
            *sample = v.clamp(-1.0, 1.0) as f32;
        }
        Ok(())
    }

    #[must_use]
    pub fn sample_rate_hz(&self) -> f64 {
        self.sample_rate_hz
    }
}

/// One-pole approach that snaps within 1e-9 (as `ovillo-tract`); returns
/// whether the value moved.
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

fn initial_point(a: Articulation) -> [f64; MAX_POINT] {
    let mut p = [0.0; MAX_POINT];
    p[..4].copy_from_slice(&a.point());
    for k in 0..MAX_SLOTS {
        p[4 + 2 * k] = 0.9;
    }
    p
}

#[inline]
fn hann(k: usize, len: usize) -> f64 {
    0.5 - 0.5 * (TAU * (k as f64 + 0.5) / len as f64).cos()
}

fn finite_or(v: f64, d: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        d
    }
}
