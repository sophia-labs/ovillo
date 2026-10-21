//! The real-time-factor workload, shared by the native example and the wasm
//! export so both time exactly the same thing.

use crate::world::{self as tw, Articulation, TractControls, TractWorldNd};
use crate::TractNdAnatomy;
use mus_ariadne::{ConnectionPolicy, DisposalPolicy};

/// `/a i u a/` on one breath, keys every 0.7 s, looped: the articulators are
/// moving most of the time (the worst case: modes recomputed every
/// `interval` samples while they travel).
pub const AIUA: [(f64, Articulation, f64); 4] = [
    (0.0, tw::ART_A, 120.0),
    (0.7, tw::ART_I, 128.0),
    (1.4, tw::ART_U, 116.0),
    (2.1, tw::ART_A, 110.0),
];

pub struct Workload {
    pub world: TractWorldNd,
    pub moving: bool,
    pub t: usize,
    /// Breath during the glide (0.6); a hook for perturbation experiments.
    pub breath: f64,
}

impl Workload {
    #[must_use]
    pub fn new(oral: usize, nasal: usize, interval: usize, moving: bool, sample_rate: f64, min_move: f64) -> Self {
        let mut world = TractWorldNd::with(
            sample_rate,
            TractNdAnatomy::adult_v0(oral, nasal),
            ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate },
            tw::ART_A,
        )
        .expect("tract plan");
        world.update_interval = interval;
        world.min_move = min_move;
        world.set_controls(TractControls { articulation: tw::ART_A, pitch_hz: 120.0, tenseness: 0.6, breath: 0.6 });
        Self { world, moving, t: 0, breath: 0.6 }
    }

    /// Render `out.len()` samples; returns the sum of squares (keeps the work live).
    pub fn render(&mut self, out: &mut [f32]) -> f64 {
        let sr = self.world.sample_rate_hz();
        let mut acc = 0.0;
        for sample in out.iter_mut() {
            if self.moving {
                let t = (self.t as f64 / sr) % 2.8;
                let key = AIUA.iter().rev().find(|k| k.0 <= t).unwrap_or(&AIUA[0]);
                self.world.set_controls(TractControls { articulation: key.1, pitch_hz: key.2, tenseness: 0.6, breath: self.breath });
            }
            let v = self.world.tick().unwrap_or(0.0);
            acc += v * v;
            *sample = v as f32;
            self.t += 1;
        }
        acc
    }
}
