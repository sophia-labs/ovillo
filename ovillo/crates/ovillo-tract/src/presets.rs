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
use crate::world::{Constriction, TractControls};

/// GARGANTA rung-1 vowel-body formants F1-F3 (Hz), copied from
/// `mus-dsp/src/body_profiles.rs`. Targets, not data.
pub const TARGET_A_HZ: [f64; 3] = [710.0, 1150.0, 2500.0];
pub const TARGET_I_HZ: [f64; 3] = [290.0, 2250.0, 2950.0];
pub const TARGET_U_HZ: [f64; 3] = [310.0, 700.0, 2350.0];

/// Fitted articulations (see module docs).
pub const ART_A: Articulation = Articulation { tongue_index: 0.065, tongue_diameter: 0.725, lips: 0.610, velum: 0.0 };
pub const ART_I: Articulation = Articulation { tongue_index: 0.885, tongue_diameter: 0.690, lips: 0.760, velum: 0.0 };
/// /u/ needs a small velopharyngeal opening (0.015 of the port) to fit once
/// the velum seals exactly; that opening is part of the fit.
pub const ART_U: Articulation = Articulation { tongue_index: 0.395, tongue_diameter: 0.885, lips: 0.050, velum: 0.015 };
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
    /// The gesture's own constriction (slot 0); degree 0 means none.
    pub constriction: Constriction,
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

    /// The gesture's constriction (slot 0) at time `t`.
    #[must_use]
    pub fn constriction_at(&self, t: f64) -> Constriction {
        let t = match self.loop_s {
            Some(period) if period > 0.0 => t % period,
            _ => t,
        };
        self.keys.iter().rev().find(|k| k.at_s <= t).map_or(Constriction::default(), |k| k.constriction)
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

    /// Gesture ids for hosts: 0 mama, 1 a-i-u-a, 2 sss, 3 shh, 4 fff, 5 zzz,
    /// 6 vvv, 7 pa, 8 ta, 9 ka, 10 papapa.
    #[must_use]
    pub fn from_index(i: u32) -> Option<Self> {
        let one = |keys: &'static [Key]| Some(Self { keys, loop_s: None, aspiration: 0.08 });
        match i {
            0 => Some(Self::mama()),
            1 => Some(Self::aiua()),
            2 => one(&SSS),
            3 => one(&SHH),
            4 => one(&FFF),
            5 => one(&ZZZ),
            6 => one(&VVV),
            7 => one(&PA),
            8 => one(&TA),
            9 => one(&KA),
            10 => Some(Self { keys: &PAPAPA, loop_s: Some(1.6), aspiration: 0.08 }),
            _ => None,
        }
    }

    /// Names for the ids above.
    pub const NAMES: [&'static str; 11] = ["mama", "aiua", "sss", "shh", "fff", "zzz", "vvv", "pa", "ta", "ka", "papapa"];
}

const NONE: Constriction = Constriction { position: 0.9, degree: 0.0 };

const fn key(at_s: f64, articulation: Articulation, pitch_hz: f64, breath: f64) -> Key {
    Key { at_s, articulation, pitch_hz, breath, tenseness: 0.6, constriction: NONE }
}

/// A consonant key: tenseness (0 abducted, voiceless .. 0.6 voiced) and a
/// constriction at `position` with `degree`.
const fn ckey(at_s: f64, articulation: Articulation, breath: f64, tenseness: f64, position: f64, degree: f64) -> Key {
    Key { at_s, articulation, pitch_hz: 118.0, breath, tenseness, constriction: Constriction { position, degree } }
}

// Consonant places (design hypotheses): alveolar 0.86, post-alveolar 0.80,
// velar 0.56, labiodental 0.985. Fricative degrees leave an aperture of a few
// hundredths of a cm²; stops close (degree 1, aperture exactly zero).
const ALVEOLAR: f64 = 0.86;
const POSTALVEOLAR: f64 = 0.80;
const VELAR: f64 = 0.56;
const LABIODENTAL: f64 = 0.985;
/// Tongue body for /s/ /z/ /t/: front and fairly high.
pub const ART_S: Articulation = Articulation { tongue_index: 0.85, tongue_diameter: 0.55, lips: 0.6, velum: 0.0 };
/// /ʃ/: a little further back, lips rounded.
pub const ART_SH: Articulation = Articulation { tongue_index: 0.72, tongue_diameter: 0.6, lips: 0.32, velum: 0.0 };
/// /f/ /v/ /p/ context: a neutral tongue.
pub const ART_F: Articulation = Articulation { tongue_index: 0.5, tongue_diameter: 0.35, lips: 0.5, velum: 0.0 };
/// /p/ held: lips sealed, velum shut.
pub const ART_P: Articulation = Articulation { tongue_index: ART_A.tongue_index, tongue_diameter: ART_A.tongue_diameter, lips: 0.0, velum: 0.0 };
/// /k/: tongue body back and high (the closure itself is the constriction).
pub const ART_K: Articulation = Articulation { tongue_index: 0.45, tongue_diameter: 0.6, lips: 0.56, velum: 0.0 };

static SSS: [Key; 2] = [ckey(0.0, ART_S, 0.75, 0.0, ALVEOLAR, 0.93), ckey(1.3, ART_S, 0.0, 0.0, ALVEOLAR, 0.0)];
static SHH: [Key; 2] = [ckey(0.0, ART_SH, 0.75, 0.0, POSTALVEOLAR, 0.92), ckey(1.3, ART_SH, 0.0, 0.0, POSTALVEOLAR, 0.0)];
static FFF: [Key; 2] = [ckey(0.0, ART_F, 0.75, 0.0, LABIODENTAL, 0.95), ckey(1.3, ART_F, 0.0, 0.0, LABIODENTAL, 0.0)];
static ZZZ: [Key; 2] = [ckey(0.0, ART_S, 0.7, 0.6, ALVEOLAR, 0.93), ckey(1.3, ART_S, 0.0, 0.6, ALVEOLAR, 0.0)];
static VVV: [Key; 2] = [ckey(0.0, ART_F, 0.7, 0.6, LABIODENTAL, 0.95), ckey(1.3, ART_F, 0.0, 0.6, LABIODENTAL, 0.0)];

/// Closure (breath on, glottis abducted), release, aspiration, voicing, rest.
macro_rules! stop {
    ($held:expr, $pos:expr, $deg:expr, $t0:expr) => {
        [
            ckey($t0 + 0.00, $held, 0.8, 0.15, $pos, $deg),
            ckey($t0 + 0.14, ART_A, 0.8, 0.15, $pos, 0.0),
            ckey($t0 + 0.19, ART_A, 0.75, 0.6, $pos, 0.0),
            ckey($t0 + 0.38, ART_A, 0.0, 0.6, $pos, 0.0),
        ]
    };
}
static PA: [Key; 4] = stop!(ART_P, 0.9, 0.0, 0.0);
static TA: [Key; 4] = stop!(ART_S, ALVEOLAR, 1.0, 0.0);
static KA: [Key; 4] = stop!(ART_K, VELAR, 1.0, 0.0);
static PAPAPA: [Key; 10] = {
    let a = stop!(ART_P, 0.9, 0.0, 0.0);
    let b = stop!(ART_P, 0.9, 0.0, 0.40);
    let c = stop!(ART_P, 0.9, 0.0, 0.80);
    [a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], ckey(1.18, ART_A, 0.0, 0.6, 0.9, 0.0)]
};

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
