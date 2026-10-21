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

use mus_ariadne::{
    BodyControls, ConnectionPolicy, ConstitutiveBodyControls, DisposalPolicy, FlowValveControls,
    SpaceBodyControls, World, WorldControls, WorldError,
};

use crate::anatomy::TractNdAnatomy;

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
            })],
            world_controls,
            output_gain: 1.0 / 2_500.0,
            stats: MoveStats::default(),
            anatomy,
        })
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
        let frame = self.world.set_body_configuration_point(0, &self.last_moved)?;
        if frame.modes_recomputed {
            self.stats.moves += 1;
        }
        self.stats.control_work += frame.control_work;
        self.stats.disposal += frame.disposal;
        Ok(())
    }

    fn retune_now(&mut self) -> Result<(), WorldError> {
        let work = self.world.set_body_valve_tuning(0, self.current.pitch_hz, self.current.tenseness)?;
        if work != 0.0 {
            self.stats.valve_retunes += 1;
            self.stats.valve_work += work;
        }
        Ok(())
    }

    /// One audio sample (the World's clock is the audio clock).
    pub fn tick(&mut self) -> Result<f64, WorldError> {
        let (t, c) = (self.target, &mut self.current);
        let moving = approach(&mut c.articulation.tongue_index, t.articulation.tongue_index, self.k_art)
            | approach(&mut c.articulation.tongue_diameter, t.articulation.tongue_diameter, self.k_art)
            | approach(&mut c.articulation.lips, t.articulation.lips, self.k_art)
            | approach(&mut c.articulation.velum, t.articulation.velum, self.k_art);
        let tuning = approach(&mut c.pitch_hz, t.pitch_hz, self.k_pitch) | approach(&mut c.tenseness, t.tenseness, self.k_tense);
        approach(&mut c.breath, t.breath, self.k_breath);
        self.pending_move |= moving;
        self.pending_tune |= tuning;
        if self.phase == 0 {
            if self.pending_move {
                let point = self.current.articulation.point();
                let delta = (0..4).map(|i| (point[i] - self.last_moved[i]).abs()).fold(0.0, f64::max);
                if !moving || delta > self.min_move {
                    self.move_now()?;
                    self.pending_move = moving;
                }
            }
            if self.pending_tune {
                self.retune_now()?;
                self.pending_tune = tuning;
            }
        }
        self.phase = (self.phase + 1) % self.update_interval.max(1);
        if let ConstitutiveBodyControls::ModalSpace(controls) = &mut self.controls[0] {
            controls.valve.supply = self.current.breath;
        }
        self.world.process_constitutive_sample(&self.controls, self.world_controls)?;
        let emitted = self.world.body_radiation(0).unwrap_or(&[]);
        let pressure: f64 = emitted.iter().sum();
        Ok(pressure * self.output_gain)
    }

    /// Render into `out` (f32, clamped only on the listener's copy).
    pub fn process(&mut self, out: &mut [f32]) -> Result<(), WorldError> {
        for sample in out.iter_mut() {
            *sample = self.tick()?.clamp(-1.0, 1.0) as f32;
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
