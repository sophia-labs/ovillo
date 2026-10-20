//! Named configurations and control paths.
//!
//! The vowel presets are a **fit**: tongue index, tongue diameter and lips were
//! searched (`examples/fit_presets.rs`) so that the rigid-glottis tube's own
//! resonances land near GARGANTA's rung-1 vowel bodies
//! (`mus-rs/crates/mus-dsp/src/body_profiles.rs` `VOWEL_A_MODES`,
//! `VOWEL_I_MODES`, `VOWEL_U_MODES`), which that file labels "design profiles,
//! not measurements". The fit's receipt is
//! `ovillo/crates/ovillo-tract/receipts/preset-fit.json`.

use crate::anatomy::Articulation;
use crate::world::TractControls;

/// GARGANTA rung-1 vowel-body formants F1-F3 (Hz), copied from
/// `mus-dsp/src/body_profiles.rs`. Targets, not data.
pub const TARGET_A_HZ: [f64; 3] = [710.0, 1150.0, 2500.0];
pub const TARGET_I_HZ: [f64; 3] = [290.0, 2250.0, 2950.0];
pub const TARGET_U_HZ: [f64; 3] = [310.0, 700.0, 2350.0];

/// Fitted articulations (see module docs).
pub const ART_A: Articulation = Articulation { tongue_index: 0.060, tongue_diameter: 0.730, lips: 0.560, velum: 0.0 };
pub const ART_I: Articulation = Articulation { tongue_index: 0.910, tongue_diameter: 0.740, lips: 0.410, velum: 0.0 };
pub const ART_U: Articulation = Articulation { tongue_index: 0.475, tongue_diameter: 0.975, lips: 0.025, velum: 0.0 };
/// /m/: lips at the area floor, velum open, tongue where /a/ will need it.
pub const ART_M: Articulation = Articulation { tongue_index: ART_A.tongue_index, tongue_diameter: ART_A.tongue_diameter, lips: 0.0, velum: 1.0 };
/// Rest: a neutral, half-open mouth.
pub const ART_REST: Articulation = Articulation { tongue_index: 0.5, tongue_diameter: 0.3, lips: 0.6, velum: 0.35 };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    A,
    I,
    U,
    Rest,
}

impl Preset {
    #[must_use]
    pub fn articulation(self) -> Articulation {
        match self {
            Self::A => ART_A,
            Self::I => ART_I,
            Self::U => ART_U,
            Self::Rest => ART_REST,
        }
    }

    #[must_use]
    pub fn from_index(i: u32) -> Option<Self> {
        match i {
            0 => Some(Self::A),
            1 => Some(Self::I),
            2 => Some(Self::U),
            3 => Some(Self::Rest),
            _ => None,
        }
    }
}

/// One step of a control path: from `at_s` until the next key, aim here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    pub at_s: f64,
    pub articulation: Articulation,
    pub pitch_hz: f64,
    pub breath: f64,
    pub tenseness: f64,
}

/// A sample-accurate, piecewise-constant target path (the articulators'
/// finite speed makes the glides). Loops if `loop_s` is set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gesture {
    pub keys: &'static [Key],
    pub loop_s: Option<f64>,
    pub aspiration: f64,
}

impl Gesture {
    /// Controls at time `t`, or `None` when a one-shot path has ended.
    #[must_use]
    pub fn controls_at(&self, t: f64) -> Option<TractControls> {
        let t = match self.loop_s {
            Some(period) if period > 0.0 => t % period,
            _ => t,
        };
        let last = self.keys.last()?;
        if self.loop_s.is_none() && t > last.at_s + 0.25 {
            return None;
        }
        let key = self.keys.iter().rev().find(|k| k.at_s <= t).unwrap_or(&self.keys[0]);
        Some(TractControls {
            articulation: key.articulation,
            pitch_hz: key.pitch_hz,
            tenseness: key.tenseness,
            breath: key.breath,
            aspiration: self.aspiration,
        })
    }

    /// "mama", slowly, falling on the second syllable, then a breath.
    #[must_use]
    pub fn mama() -> Self {
        Self { keys: &MAMA, loop_s: Some(2.4), aspiration: 0.12 }
    }

    /// A slow slide /a/ -> /i/ -> /u/ -> /a/ on one breath.
    #[must_use]
    pub fn aiua() -> Self {
        Self { keys: &AIUA, loop_s: None, aspiration: 0.12 }
    }

    #[must_use]
    pub fn from_index(i: u32) -> Option<Self> {
        match i {
            0 => Some(Self::mama()),
            1 => Some(Self::aiua()),
            _ => None,
        }
    }
}

const fn key(at_s: f64, articulation: Articulation, pitch_hz: f64, breath: f64) -> Key {
    Key { at_s, articulation, pitch_hz, breath, tenseness: 0.6 }
}

static MAMA: [Key; 7] = [
    key(0.00, ART_M, 118.0, 0.55),
    key(0.20, ART_A, 122.0, 0.62),
    key(0.62, ART_M, 112.0, 0.55),
    key(0.80, ART_A, 104.0, 0.60),
    key(1.40, ART_A, 98.0, 0.0),
    key(1.55, ART_REST, 98.0, 0.0),
    key(2.30, ART_M, 112.0, 0.0),
];

static AIUA: [Key; 5] = [
    key(0.00, ART_A, 120.0, 0.6),
    key(0.70, ART_I, 128.0, 0.6),
    key(1.40, ART_U, 116.0, 0.6),
    key(2.10, ART_A, 110.0, 0.6),
    key(2.80, ART_A, 110.0, 0.0),
];
