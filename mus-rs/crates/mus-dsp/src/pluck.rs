//! Physically informed string networks: a deepened guitar and the
//! impossible `weave` instrument.
//!
//! This module begins at extended Karplus--Strong and then makes the
//! feedback loop explicit enough to support instruments that are not
//! reducible to one string plus an output EQ.
//!
//! ## Guitar
//!
//! The `pluck` voice now combines:
//!
//! - a physical triangular initial displacement, with content-keyed pick
//!   roughness rather than a noise-only delay-line fill;
//! - exact fundamental-phase compensation for the damping and dispersion
//!   filters, preserving tuning while adding stiffness;
//! - frequency-dependent decay with fundamental T60 calibration;
//! - amplitude-dependent onset pitch glide whose relaxation time is tied
//!   to the string's energy-decay time;
//! - a shared, damped modal body coupled *inside* the feedback network by
//!   norm-preserving rotations;
//! - optional sympathetic open strings, detuned courses, palm muting,
//!   bends, strums, and a contractive fret/bridge-contact model.
//!
//! ## Weave
//!
//! `weave` is a strict generalisation. It adds virtual courses whose mode
//! count follows a configurable spectral dimension, then scatters their
//! travelling-wave endpoint values through a time-varying ordered product
//! of Givens rotations. Every individual rotation is exactly orthogonal,
//! so modulation can create sidebands and redistribute energy without
//! introducing hidden gain. Because adjacent rotations do not commute,
//! changing their order creates an audible holonomy: the state after a
//! control cycle depends on the path through coupling space, not just the
//! final knob values.
//!
//! The pointwise nonlinear case remains safe. If the rotation angles are
//! functions of the current state, `Q(x)` is still orthogonal for every
//! `x`, and therefore `||Q(x)x||_2 = ||x||_2`. With the explicitly
//! contractive loop and body losses used here, the unforced network is
//! non-expansive. This is the module's mathematical safety contract and is
//! tested directly below and in `tests/pluck_invariants.rs`.
//!
//! ## Bow
//!
//! `bow` breaks that contract on purpose and says so. A friction port at
//! the excitation point reads the loop value as a string-velocity proxy
//! and injects a force from a falling friction curve — negative
//! resistance, an energy SOURCE. Nothing above bounds it. What bounds it
//! is measurement over a declared corner grid, in the invariants file,
//! pending the round-3 nonlinear source-port work contract. Where the v1
//! curve does not produce a tone, the region is declared rather than
//! hidden: see [`BOW_GAIN`].
//!
//! ## Gestures
//!
//! The ring above derives its angles from params; a *compiled* control
//! word needs explicit plane rotations at declared samples. That is the
//! gesture seam (Ariadne Phase 2 §B): a [`GestureFrame`] names two
//! courses, an angle and a sample, and the executor applies exactly that
//! Givens rotation to the same wave variables [`StringNetworkVoice::
//! scatter_weave`] acts on, immediately after the scattering pass of that
//! sample. Nothing else about the voice changes, so the seam inherits the
//! module's orthogonality contract verbatim: every gesture is exactly
//! norm-preserving on the pair it touches, and a network that was
//! non-expansive without gestures stays non-expansive with them.
//!
//! Like every seam before it, it obeys a hard zero limit: an empty
//! gesture list is byte-identical to today's render, on pluck and weave
//! alike. See [`StringNetworkVoice::render_traced_gestured`].

use std::collections::BTreeMap;
use std::f64::consts::{LN_10, PI};

use np_rand::{Pcg64, SeedSequence};

use crate::body_profiles;
use crate::SR_F64;

const OPEN_GUITAR_STRINGS: [f64; 6] = [82.4069, 110.0, 146.832, 195.998, 246.942, 329.628];

/// Everything read by the guitar and Weave string-network voices.
///
/// The original eight fields remain source-compatible. New fields have
/// conservative guitar defaults: `synth=pluck` becomes better without
/// turning into the exotic instrument unless explicitly requested.
#[derive(Debug, Clone, PartialEq)]
pub struct PluckPatch {
    /// Fundamental T60 in seconds (`sus`).
    pub sus: f64,
    /// Frequency-dependent loop loss, 0..0.95 (`damp`).
    pub damp: f64,
    /// Pluck point as a fraction of string length (`pos`).
    pub pos: f64,
    /// Contact hardness: fingertip 0, plectrum 1 (`pick`).
    pub pick: f64,
    /// Shared body-coupling amount (`body`).
    pub body: f64,
    /// Chord stagger in milliseconds; negative reverses order (`strum`).
    pub strum_ms: f64,
    /// Palm mute (`pm`).
    pub pm: bool,
    /// Detuned companion course in cents (`detune`).
    pub detune: f64,
    /// Stiff-string dispersion amount, 0..1 (`stiff`).
    pub stiff: f64,
    /// Initial onset pitch elevation in cents (`tension`).
    pub tension_cents: f64,
    /// Coupling to unplayed standard-guitar open strings (`symp`).
    pub symp: f64,
    /// Contractive fret/bridge contact amount (`buzz`).
    pub buzz: f64,
    /// Body-size ratio; >1 lowers body modes (`body_size`).
    pub body_size: f64,
    /// Weave nearest-neighbour scattering angle in radians (`couple`).
    pub couple: f64,
    /// Ordered-scattering bias, -1..1 (`chirality`).
    pub chirality: f64,
    /// Rate of the travelling coupling field in Hz (`orbit`).
    pub orbit_hz: f64,
    /// Depth of the travelling coupling field (`orbit_depth`).
    pub orbit_depth: f64,
    /// State-dependent metric curvature, 0..1 (`curvature`).
    pub curvature: f64,
    /// Target total number of Weave courses (`courses`).
    pub courses: usize,
    /// Spectral dimension d, with f_k proportional to k^(1/d) (`dimension`).
    pub spectral_dimension: f64,
    /// External excitation mix (`sing`): 0 = pluck burst only, 1 = the
    /// track's sample alone drives the strings through the shared bridge.
    pub sing: f64,
    /// Seconds into the sample the drive starts reading (`sing_at`).
    pub sing_at: f64,
    /// Body profile selector (`bodyp`): `guitar|violin|cello|bass|
    /// mandolin`, default `guitar`. Always one of the five registered
    /// names after parsing — see [`PluckPatch::from`]'s enum-clamp note.
    pub bodyp: String,
    /// Sustained breath excitation mix, 0..1 (`breath`): content-keyed
    /// lowpassed noise injected at the bridge while the note holds its
    /// slot. 0 = pluck burst only.
    pub breath: f64,
    /// Bow pressure (`bow`): 0 = no bow (hard zero limit), 1 = full weight.
    pub bow: f64,
    /// Bow velocity (`bow_speed`), 0..1 mapped to [`BOW_VELOCITY_MIN`]
    /// ..[`BOW_VELOCITY_MAX`] in wave units.
    pub bow_speed: f64,
    /// Retuning policy for the 16-sample coefficient update
    /// (`retune`, Ariadne Phase 0 §A). See [`RetuneMode`].
    pub retune: RetuneMode,
}

/// What the engine does to allpass filter *state* when a retune rewrites
/// an allpass *coefficient* (Ariadne Phase 0 §A).
///
/// [`DelayString::update_loop`] runs every 16 samples and rewrites three
/// allpass coefficients: two dispersion sections and the fractional-delay
/// section. Historically it rewrote them with the filter memory silently
/// retained, which reinterprets the stored state under a new metric — the
/// hidden control work `.string-research/NEXT-PHASE-REPORT.md` §3
/// measured at up to `9.386` in an adversarial sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RetuneMode {
    /// Rewrite the coefficient and keep `(x1, y1)`. This is what the
    /// engine has always done and what every existing render is; it is
    /// the default, and the Phase 0 byte law protects it.
    #[default]
    Legacy,
    /// Apply `mus_watsn::neutral_allpass_state_transport` at each
    /// coefficient change: preserve the normalized coordinate
    /// `z = s/sqrt(1-a^2)` with `s = x1 - a*y1`, retaining `y1` and
    /// reconstructing `x1`. Coefficient-update work drops to roundoff.
    ///
    /// The *delay line's* reinterpretation when `integer_delay` changes
    /// is NOT repaired by this mode — that is Phase 3's open transport
    /// problem, and `update_loop` says so at the site.
    NeutralFilters,
}

impl RetuneMode {
    /// Parse the `retune` param. Unrecognised values fall back to
    /// [`RetuneMode::Legacy`], the same "invalid input degrades to a safe
    /// default" discipline `bodyp` applies to an out-of-domain string.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "legacy" => Self::Legacy,
            "neutral_filters" => Self::NeutralFilters,
            _ => return None,
        })
    }

    /// The canonical param spelling of this mode.
    pub fn name(self) -> &'static str {
        match self {
            Self::Legacy => "legacy",
            Self::NeutralFilters => "neutral_filters",
        }
    }
}

impl Default for PluckPatch {
    fn default() -> Self {
        Self {
            sus: 2.5,
            damp: 0.35,
            pos: 0.13,
            pick: 0.6,
            // The handoff proposes 0.42, contingent on a listening
            // decision; until that A/B, the public default stays at the
            // established 0.25 and presets carry the stronger value.
            body: 0.25,
            strum_ms: 10.0,
            pm: false,
            detune: 0.0,
            stiff: 0.08,
            tension_cents: 5.0,
            symp: 0.22,
            buzz: 0.0,
            body_size: 1.0,
            couple: 0.11,
            chirality: 0.72,
            orbit_hz: 0.31,
            orbit_depth: 0.62,
            curvature: 0.28,
            courses: 11,
            spectral_dimension: 1.35,
            sing: 0.0,
            sing_at: 0.0,
            bodyp: "guitar".to_string(),
            breath: 0.0,
            bow: 0.0,
            bow_speed: 0.5,
            retune: RetuneMode::Legacy,
        }
    }
}

fn parsed(map: &BTreeMap<String, String>, key: &str, default: f64) -> f64 {
    map.get(key)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default)
}

fn parsed_usize(map: &BTreeMap<String, String>, key: &str, default: usize) -> usize {
    map.get(key)
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value.round().max(0.0) as usize)
        .unwrap_or(default)
}

impl From<&BTreeMap<String, String>> for PluckPatch {
    fn from(map: &BTreeMap<String, String>) -> Self {
        let d = Self::default();
        Self {
            sus: parsed(map, "sus", d.sus).clamp(0.05, 20.0),
            damp: parsed(map, "damp", d.damp).clamp(0.0, 0.95),
            pos: parsed(map, "pos", d.pos).clamp(0.02, 0.5),
            pick: parsed(map, "pick", d.pick).clamp(0.0, 1.0),
            body: parsed(map, "body", d.body).clamp(0.0, 1.0),
            strum_ms: parsed(map, "strum", d.strum_ms).clamp(-80.0, 80.0),
            pm: map
                .get("pm")
                .is_some_and(|value| value != "0" && value != "off" && value != "no"),
            detune: parsed(map, "detune", d.detune).clamp(0.0, 100.0),
            stiff: parsed(map, "stiff", d.stiff).clamp(0.0, 1.0),
            tension_cents: parsed(map, "tension", d.tension_cents).clamp(0.0, 80.0),
            symp: parsed(map, "symp", d.symp).clamp(0.0, 1.0),
            buzz: parsed(map, "buzz", d.buzz).clamp(0.0, 1.0),
            body_size: parsed(map, "body_size", d.body_size).clamp(0.45, 2.4),
            couple: parsed(map, "couple", d.couple).clamp(0.0, 0.45),
            chirality: parsed(map, "chirality", d.chirality).clamp(-1.0, 1.0),
            orbit_hz: parsed(map, "orbit", d.orbit_hz).clamp(0.0, 20.0),
            orbit_depth: parsed(map, "orbit_depth", d.orbit_depth).clamp(0.0, 1.0),
            curvature: parsed(map, "curvature", d.curvature).clamp(0.0, 1.0),
            courses: parsed_usize(map, "courses", d.courses).clamp(3, 24),
            spectral_dimension: parsed(map, "dimension", d.spectral_dimension).clamp(0.55, 3.0),
            sing: parsed(map, "sing", d.sing).clamp(0.0, 1.0),
            sing_at: parsed(map, "sing_at", d.sing_at).clamp(0.0, 600.0),
            // Enum clamp: an unrecognised `bodyp` value falls back to
            // guitar rather than erroring — the same "invalid input
            // degrades to a safe default" discipline the numeric
            // `.clamp()` calls above apply to out-of-range numbers,
            // applied here to an out-of-domain string. `body_profiles::
            // profile_for` does the lookup + fallback; storing its
            // canonical `.name` here means `patch.bodyp` is always one of
            // the five registered names after parsing, never an arbitrary
            // string. There is no `mus_audio.py` oracle entry for
            // `bodyp` (new engine-only surface, PW10 §A) — this fallback
            // *is* the domain law, not a port of one.
            bodyp: body_profiles::profile_for(
                map.get("bodyp").map(String::as_str).unwrap_or(&d.bodyp),
            )
            .name
            .to_string(),
            breath: parsed(map, "breath", d.breath).clamp(0.0, 1.0),
            bow: parsed(map, "bow", d.bow).clamp(0.0, 1.0),
            bow_speed: parsed(map, "bow_speed", d.bow_speed).clamp(0.0, 1.0),
            // Enum clamp, `bodyp`'s discipline: an unrecognised `retune`
            // value degrades to legacy rather than erroring. Absent and
            // unparseable both mean "render exactly as this engine always
            // has", which is the Phase 0 byte law's cheapest possible
            // failure mode.
            retune: map
                .get("retune")
                .and_then(|value| RetuneMode::from_name(value))
                .unwrap_or(d.retune),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringNetworkMode {
    Guitar,
    Weave,
}

/// Static friction coefficient of the bow-hair/string contact (PW10 §B).
pub const BOW_MU_S: f64 = 0.8;
/// Dynamic (fully slipping) friction coefficient.
pub const BOW_MU_D: f64 = 0.3;
/// Characteristic slip velocity of the hyperbolic friction curve, in the
/// same wave units as the loop value.
pub const BOW_V0: f64 = 0.1;
/// Bow velocity at `bow_speed = 0`, in wave units.
pub const BOW_VELOCITY_MIN: f64 = 0.05;
/// Bow velocity at `bow_speed = 1`, in wave units.
pub const BOW_VELOCITY_MAX: f64 = 0.4;
/// Force scale converting the dimensionless friction coefficient into the
/// loop's wave units, at `bow = 1`.
///
/// Not a spec constant — the spec fixes `mu_s`/`mu_d`/`v0` and leaves the
/// gain to be tuned against the invariants, so here is the measurement.
/// What sets it is a two-sided trade, not the boundedness bound: the
/// friction only sustains once its small-signal slope beats the loop loss
/// (`BOW_GAIN * bow * (mu_s - mu_d) * v0 / (v0 + v_bow)^2 > 3 ln 10 /
/// (sus * f0)`), which is a minimum-bow-force curve rising with bow speed
/// — Schelleng's diagram, arrived at rather than imposed. Raising the gain
/// widens the speaking region and coarsens the tone. Measured on a 196 Hz
/// note: "speaks" = late-window RMS at least 100x the same note un-bowed;
/// "tonality" = harmonic / between-harmonic power of a held note at
/// `bow = 0.7, bow_speed = 0.5, sus = 2.5` (a plucked onset measures
/// 4.4e3); "corner peak" = worst |sample| over the bowed corner grid.
///
/// | gain | speaking region                    | tonality | corner peak |
/// |------|------------------------------------|----------|-------------|
/// | 0.06 | bow >= 0.4, only to bow_speed 0.2  | 2.0e6    | 1.70        |
/// | 0.10 | bow >= 0.4 to 0.5; bow >= 0.6 all  | 9.1e5    | 1.73        |
/// | 0.15 | bow >= 0.4, every bow_speed        | 2.9e5    | 1.78        |
/// | 0.25 | bow >= 0.2, only to bow_speed 0.8  | 7.3e4    | 1.80        |
///
/// 0.15 is the smallest gain whose speaking region covers the whole
/// `bow_speed` range at moderate pressure, while a held note still
/// measures ~66x more harmonic than a plucked onset. The boundedness bound
/// is nowhere near binding at any of these: the corner grid's worst peak,
/// 2.13, belongs to the UN-bowed pluck.
///
/// DECLARED DOMAIN — both edges measured at this gain and refused rather
/// than papered over (`tests/pluck_invariants.rs`):
/// - below the minimum force (e.g. `bow = 0.1` at `bow_speed >= 0.2`) the
///   string does not speak; the bow only slows the pluck's decay. Real
///   behaviour, and the reason no sustain law is claimed there.
/// - `bow_speed = 0` (0.05 wave units) with `bow >= 0.6` collapses to
///   noise: tonality 0.69, against 3.2e4 for the same pressure at
///   `bow_speed = 0.5`. Too much pressure for the speed is a crushed tone
///   on a real instrument too, but what v1 produces there is broadband
///   chatter rather than a periodic crunch, so that corner is DECLARED
///   UNUSABLE, not offered as a timbre.
pub const BOW_GAIN: f64 = 0.15;

/// One-pole coefficient of the per-course DC blocker on the injected bow
/// force (~5 Hz corner at 48 kHz).
///
/// DECLARED DEVIATION from PW10 §B's mechanism v1, with its reason. The
/// friction characteristic is a relay: Coulomb friction does not vanish as
/// `dv -> 0`, it jumps between `+-mu_s`. Its static part is therefore a
/// constant force, and a single-delay-line loop has a comb peak at 0 Hz
/// (a real string, with its inverting terminations, does not — that DC
/// mode is an artifact of the discrete loop, not physics). Injecting the
/// raw force pumps that artifact: the loop DC climbs to `v_bow`, which is
/// exactly the friction discontinuity, and the port drops into a sliding
/// mode that pins the course instead of letting it ring. Measured, at
/// `bow = 0.7, bow_speed = 0.5, sus = 2.5`, 196 Hz: raw force sustains
/// 0.0021 RMS at tonality 1.16 (a hiss — a pluck onset is 4.4e3); the same
/// patch with the force DC-blocked sustains 0.031 RMS at tonality 2.0e6
/// (a tone). The blocker removes only what the loop should not have been
/// able to store; it is below every played fundamental and does not touch
/// the friction's audio-band behaviour.
pub const BOW_DC_POLE: f64 = 6.5e-4;

/// Hyperbolic (Coulomb + Stribeck) friction characteristic.
///
/// `mu(dv) = sign(dv) * (mu_d + (mu_s - mu_d) / (1 + |dv| / v0))` — full
/// static grip at vanishing relative velocity, falling to the dynamic
/// coefficient as the string slips past the hair. The falling branch has
/// negative slope, which is exactly the negative resistance that makes a
/// bowed string sustain; it is also why the bow is an energy SOURCE and
/// not covered by the module's rotation/passivity argument.
fn bow_friction(relative_velocity: f64) -> f64 {
    relative_velocity.signum()
        * (BOW_MU_D + (BOW_MU_S - BOW_MU_D) / (1.0 + relative_velocity.abs() / BOW_V0))
}

/// Content-keyed seed (doctrine A7): no global random stream and no event
/// ordering dependence. FNV-1a over every value that changes the contact.
fn content_seed(
    f0: f64,
    slot_s: f64,
    patch: &PluckPatch,
    course_index: u64,
    mode: StringNetworkMode,
) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut eat = |bits: u64| {
        for byte in bits.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    eat(f0.to_bits());
    eat(slot_s.to_bits());
    eat(patch.pos.to_bits());
    eat(patch.pick.to_bits());
    eat(patch.stiff.to_bits());
    eat((patch.pm as u64) | ((mode == StringNetworkMode::Weave) as u64) << 1);
    eat(course_index);
    hash
}

/// Breath's noise-stream seed. Built ON TOP of [`content_seed`] (same
/// content-keyed material: f0, slot, pos/pick/stiff, pm/mode, course
/// index) rather than a second independent hash, then tagged and
/// avalanche-mixed (splitmix64 finalizer) so the breath stream never
/// coincides with the pick-roughness stream that also reads
/// `content_seed` directly. `content_seed` itself is untouched by this —
/// the `breath=0` zero-limit law depends on the pick-roughness seed path
/// changing nothing about its own computation.
fn breath_seed(
    f0: f64,
    slot_s: f64,
    patch: &PluckPatch,
    course_index: u64,
    mode: StringNetworkMode,
) -> u64 {
    let base = content_seed(f0, slot_s, patch, course_index, mode);
    let mut seed = base ^ 0xB2EA_5EED_1234_5678_u64;
    seed = (seed ^ (seed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    seed = (seed ^ (seed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    seed ^= seed >> 31;
    seed
}

fn uniform_bipolar(rng: &mut Pcg64) -> f64 {
    (rng.next_u64() >> 11) as f64 / (1_u64 << 53) as f64 * 2.0 - 1.0
}

fn rms(values: &[f64]) -> f64 {
    (values.iter().map(|value| value * value).sum::<f64>() / values.len().max(1) as f64).sqrt()
}

/// Circular box smoothing. The radius is intentionally fractional at the
/// control layer but rounded here: the excitation is one finite spatial
/// period, not an infinite audio stream.
fn circular_smooth(values: &[f64], radius: usize) -> Vec<f64> {
    if radius == 0 || values.len() < 3 {
        return values.to_vec();
    }
    let n = values.len();
    let width = radius * 2 + 1;
    let mut out = vec![0.0; n];
    for (index, slot) in out.iter_mut().enumerate() {
        let mut sum = 0.0;
        for offset in 0..width {
            let source = (index + n + offset - radius) % n;
            sum += values[source];
        }
        *slot = sum / width as f64;
    }
    out
}

/// One spatial period of a plucked string.
///
/// The triangular component is the ideal displaced-string initial
/// condition. Its Fourier coefficients already encode the pick-position
/// nodal pattern. A smaller content-keyed roughness component is circularly
/// combed at the same position and smoothed according to contact width.
fn physical_excitation(
    f0: f64,
    slot_s: f64,
    patch: &PluckPatch,
    course_index: u64,
    mode: StringNetworkMode,
    level: f64,
) -> Vec<f64> {
    let period = (SR_F64 / f0.max(20.0)).ceil().max(8.0) as usize;
    let pick_position = patch.pos.clamp(0.02, 0.98);
    let mut triangle = Vec::with_capacity(period);
    for index in 0..period {
        let x = index as f64 / period as f64;
        let value = if x < pick_position {
            x / pick_position
        } else {
            (1.0 - x) / (1.0 - pick_position)
        };
        triangle.push(value);
    }
    let triangle_mean = triangle.iter().sum::<f64>() / period as f64;
    for value in &mut triangle {
        *value -= triangle_mean;
    }
    let triangle_rms = rms(&triangle).max(1e-12);
    for value in &mut triangle {
        *value /= triangle_rms;
    }

    let mut seed = SeedSequence::new(content_seed(f0, slot_s, patch, course_index, mode));
    let mut rng = Pcg64::from_seed_sequence(&mut seed);
    let noise: Vec<f64> = (0..period).map(|_| uniform_bipolar(&mut rng)).collect();
    let comb = ((pick_position * period as f64).round() as usize).clamp(1, period - 1);
    let mut rough = vec![0.0; period];
    for index in 0..period {
        rough[index] = noise[index] - noise[(index + period - comb) % period];
    }
    let smoothing_radius = ((1.0 - patch.pick) * 0.12 * period as f64).round() as usize;
    let mut rough = circular_smooth(&rough, smoothing_radius);
    let rough_mean = rough.iter().sum::<f64>() / period as f64;
    for value in &mut rough {
        *value -= rough_mean;
    }
    let rough_rms = rms(&rough).max(1e-12);
    for value in &mut rough {
        *value /= rough_rms;
    }

    let rough_mix = 0.10 + 0.45 * patch.pick;
    let mut excitation: Vec<f64> = triangle
        .iter()
        .zip(rough.iter())
        .map(|(shape, grain)| (1.0 - rough_mix) * shape + rough_mix * grain)
        .collect();
    let excitation_rms = rms(&excitation).max(1e-12);
    let palm_scale = if patch.pm { 0.72 } else { 1.0 };
    for value in &mut excitation {
        *value = *value / excitation_rms * level * palm_scale;
    }
    excitation
}

#[derive(Debug, Clone, Copy)]
struct FirstOrderAllpass {
    coefficient: f64,
    x1: f64,
    y1: f64,
}

impl FirstOrderAllpass {
    fn new(coefficient: f64) -> Self {
        Self {
            coefficient,
            x1: 0.0,
            y1: 0.0,
        }
    }

    /// Retune this section under `mode` (Ariadne Phase 0 §A).
    ///
    /// Returns the control work done on the declared allpass storage
    /// `E_a(s) = s^2 / (2 (1 - a^2))`, but ONLY when `account` is set:
    /// `None` means "not measured", never "measured zero". Work
    /// accounting is observation, and per Phase 0 §B observation is a
    /// render-entry-point decision, not part of the score — so with the
    /// observer off, legacy pays no arithmetic it did not pay before.
    ///
    /// Byte-inert no-op law, in BOTH modes: if the clamped coefficient is
    /// bit-identical to the current one, nothing is written. `to_bits`
    /// rather than `==` so that `-0.0` and `+0.0` are not conflated.
    fn retune_coefficient(
        &mut self,
        coefficient: f64,
        mode: RetuneMode,
        account: bool,
    ) -> Option<f64> {
        let new_coefficient = coefficient.clamp(-0.999_5, 0.999_5);
        if new_coefficient.to_bits() == self.coefficient.to_bits() {
            return account.then_some(0.0);
        }
        match mode {
            // Historical behavior, bit for bit: the coefficient is
            // overwritten and `(x1, y1)` stand. The energy readings are
            // pure observation — they touch no state.
            RetuneMode::Legacy => {
                let work = account.then(|| {
                    let before =
                        mus_watsn::allpass_storage_energy(self.coefficient, self.x1, self.y1);
                    let after =
                        mus_watsn::allpass_storage_energy(new_coefficient, self.x1, self.y1);
                    after - before
                });
                self.coefficient = new_coefficient;
                work
            }
            RetuneMode::NeutralFilters => {
                let (x1, y1, receipt) = mus_watsn::neutral_allpass_state_transport(
                    self.coefficient,
                    new_coefficient,
                    self.x1,
                    self.y1,
                );
                self.coefficient = receipt.new_coefficient;
                self.x1 = x1;
                self.y1 = y1;
                // The transport computes the receipt either way, so unlike
                // legacy there is nothing extra to skip here.
                account.then_some(receipt.control_work)
            }
        }
    }

    fn tick(&mut self, input: f64) -> f64 {
        // H(z) = (a + z^-1) / (1 + a z^-1)
        let output = self.coefficient * input + self.x1 - self.coefficient * self.y1;
        self.x1 = input;
        self.y1 = output;
        output
    }
}

fn one_zero_response(damp: f64, omega: f64) -> (f64, f64) {
    let real = 1.0 - damp + damp * omega.cos();
    let imag = -damp * omega.sin();
    let magnitude = real.hypot(imag).max(1e-12);
    let phase_delay = if omega > 1e-12 {
        -imag.atan2(real) / omega
    } else {
        damp
    };
    (magnitude, phase_delay)
}

fn allpass_phase_delay(coefficient: f64, omega: f64) -> f64 {
    if omega <= 1e-12 {
        return (1.0 - coefficient) / (1.0 + coefficient);
    }
    let numerator_phase = (-omega.sin()).atan2(coefficient + omega.cos());
    let denominator_phase = (-coefficient * omega.sin()).atan2(1.0 + coefficient * omega.cos());
    let mut phase = numerator_phase - denominator_phase;
    while phase > 0.0 {
        phase -= 2.0 * PI;
    }
    while phase <= -2.0 * PI {
        phase += 2.0 * PI;
    }
    -phase / omega
}

/// First-order allpass coefficient whose *phase delay* is `delay_samples`
/// at `omega`. This is exact at the fundamental, unlike a broadband
/// approximation based only on DC group delay.
fn allpass_coefficient_for_delay(delay_samples: f64, omega: f64) -> f64 {
    let delay = delay_samples.clamp(0.02, 1.98);
    if omega.abs() < 1e-9 {
        return ((1.0 - delay) / (1.0 + delay)).clamp(-0.999_5, 0.999_5);
    }
    let denominator = (omega * 0.5).tan();
    if denominator.abs() < 1e-12 {
        return ((1.0 - delay) / (1.0 + delay)).clamp(-0.999_5, 0.999_5);
    }
    let ratio = (omega * delay * 0.5).tan() / denominator;
    ((1.0 - ratio) / (1.0 + ratio)).clamp(-0.999_5, 0.999_5)
}

/// Per-course observer accumulators (Ariadne Phase 0 §B).
///
/// Every field is a running integral from sample 0, written ONLY while
/// the observer is enabled and read by nothing on the audio path. Signs
/// follow the ledger convention: `inject_*` and `work_*` add energy to
/// the declared state, `loss_*` removes it.
#[derive(Debug, Default, Clone, Copy)]
struct StringEnergyAccount {
    /// Pluck-burst injection, `w*e + e^2/2` at the excitation point.
    inject_burst: f64,
    /// `sing` external-drive injection, attributed after the burst.
    inject_drive: f64,
    /// `breath` noise injection, attributed after the drive.
    inject_breath: f64,
    /// `bow` friction-port injection, attributed last.
    inject_bow: f64,
    /// Exact one-zero damping dissipation (`mus_watsn::onezero`).
    loss_damping: f64,
    /// Exact loop-gain attenuation, `(1 - g^2) v^2 / 2`.
    loss_loop_gain: f64,
    /// Exact contact removal — the energy `apply_contact` radiates.
    loss_contact: f64,
    /// Allpass coefficient-update work (the Phase 0 §A quantity).
    work_allpass_retune: f64,
    /// Largest single-update `|work|` seen on any of the three sections.
    work_allpass_retune_peak: f64,
    /// Change in declared active-line energy caused solely by
    /// `integer_delay` changing — Phase 3's unsolved transport, measured.
    work_line_reinterpretation: f64,
    /// Largest single `|integer_delay|`-change reinterpretation.
    work_line_reinterpretation_peak: f64,
}

#[derive(Debug)]
struct DelayString {
    line: Vec<f64>,
    write_index: usize,
    f0: f64,
    f1: f64,
    t60: f64,
    damp: f64,
    dispersion_coefficient: f64,
    tension_cents: f64,
    tension_tau_samples: f64,
    start_sample: usize,
    slot_samples: usize,
    excitation: Vec<f64>,
    coupling_scale: f64,
    damping_x1: f64,
    dispersion: [FirstOrderAllpass; 2],
    fractional: FirstOrderAllpass,
    integer_delay: usize,
    loop_gain: f64,
    energy_smooth: f64,
    contact_memory: f64,
    /// Slow mean of the injected bow force (see [`BOW_DC_POLE`]).
    bow_dc: f64,
    /// Phase 0 §A retuning policy, fixed at construction (`retune` is a
    /// static param — there is no `ControlTarget` for it).
    retune: RetuneMode,
    /// Phase 0 §B observer switch. False on every ordinary render.
    observe: bool,
    /// Observer accumulators; inert while `observe` is false.
    account: StringEnergyAccount,
}

#[derive(Debug, Clone, Copy)]
struct DelayStringSpec {
    f0: f64,
    f1: f64,
    t60: f64,
    damp: f64,
    stiff: f64,
    tension_cents: f64,
    start_sample: usize,
    slot_samples: usize,
    coupling_scale: f64,
    max_downward_ratio: f64,
    retune: RetuneMode,
}

impl DelayString {
    fn new(spec: DelayStringSpec, excitation: Vec<f64>) -> Self {
        let minimum_frequency = (spec.f0.min(spec.f1) / spec.max_downward_ratio).max(15.0);
        let line_len = (SR_F64 / minimum_frequency).ceil() as usize + 32;
        let dispersion_coefficient = if spec.stiff <= 1e-12 {
            0.0
        } else {
            // The pole must approach 1 for a first-order pair to develop
            // audible frequency-dependent delay across the first partials:
            // the handoff's -0.72*stiff^0.75 tops out at +0.16 cents of
            // 5th-partial sharpening (110 Hz, stiff=0.75). This mapping
            // measures ~0.03c at the 0.08 default, ~4.6c at 0.75, ~44c at
            // 1.0 (more at higher fundamentals); update_loop's phase-budget
            // shrink keeps short high-note loops causal.
            -0.93 * spec.stiff.powf(0.35)
        };
        // If amplitude reaches -60 dB at T60, squared displacement (and
        // therefore mean elongation) decays with tau=T60/(2 ln 1000).
        let tension_tau_s = (spec.t60 / (6.0 * LN_10)).clamp(0.025, 1.5);
        Self {
            line: vec![0.0; line_len],
            write_index: 0,
            f0: spec.f0,
            f1: spec.f1,
            t60: spec.t60,
            damp: spec.damp,
            dispersion_coefficient,
            tension_cents: spec.tension_cents,
            tension_tau_samples: tension_tau_s * SR_F64,
            start_sample: spec.start_sample,
            slot_samples: spec.slot_samples.max(1),
            excitation,
            coupling_scale: spec.coupling_scale,
            damping_x1: 0.0,
            dispersion: [
                FirstOrderAllpass::new(dispersion_coefficient),
                FirstOrderAllpass::new(dispersion_coefficient),
            ],
            fractional: FirstOrderAllpass::new(0.0),
            integer_delay: 2,
            loop_gain: 0.999,
            energy_smooth: 0.0,
            contact_memory: 0.0,
            bow_dc: 0.0,
            retune: spec.retune,
            observe: false,
            account: StringEnergyAccount::default(),
        }
    }

    /// Declared Euclidean research metric over the ACTIVE delay segment:
    /// the `integer_delay` most recently written cells, the ones the read
    /// tap will still visit. Cells outside that window hold stale data
    /// from earlier cycles and are deliberately excluded — including them
    /// would report the buffer's history, not the string's state.
    fn line_energy(&self) -> f64 {
        let length = self.line.len();
        let mut total = 0.0;
        for step in 1..=self.integer_delay.min(length) {
            let cell = self.line[(self.write_index + length - step) % length];
            total += cell * cell;
        }
        0.5 * total
    }

    /// Declared storage of the three allpass sections plus the one-zero
    /// damping filter — the string's filter memory, in the same units as
    /// [`Self::line_energy`].
    fn filter_energy(&self) -> f64 {
        let mut total = mus_watsn::onezero_storage_energy(self.damp, self.damping_x1);
        for section in &self.dispersion {
            total += mus_watsn::allpass_storage_energy(section.coefficient, section.x1, section.y1);
        }
        total += mus_watsn::allpass_storage_energy(
            self.fractional.coefficient,
            self.fractional.x1,
            self.fractional.y1,
        );
        total
    }

    /// Total declared stored energy of this course.
    fn stored_energy(&self) -> f64 {
        self.line_energy() + self.filter_energy()
    }

    /// Record one allpass coefficient-update work reading.
    fn record_allpass_work(&mut self, work: Option<f64>) {
        if let Some(work) = work {
            self.account.work_allpass_retune += work;
            self.account.work_allpass_retune_peak =
                self.account.work_allpass_retune_peak.max(work.abs());
        }
    }

    fn nominal_frequency(&self, sample_index: usize) -> f64 {
        let age = sample_index.saturating_sub(self.start_sample);
        let bend_progress = (age as f64 / self.slot_samples as f64).min(1.0);
        self.f0 + (self.f1 - self.f0) * bend_progress
    }

    fn instantaneous_frequency(&self, sample_index: usize) -> f64 {
        let nominal = self.nominal_frequency(sample_index);
        if self.tension_cents <= 1e-12 || sample_index < self.start_sample {
            return nominal;
        }
        let age = (sample_index - self.start_sample) as f64;
        let cents = self.tension_cents * (-age / self.tension_tau_samples).exp();
        nominal * 2_f64.powf(cents / 1200.0)
    }

    fn update_loop(&mut self, sample_index: usize) {
        let frequency = self.instantaneous_frequency(sample_index).max(20.0);
        let omega = 2.0 * PI * frequency / SR_F64;
        let (damping_magnitude, damping_delay) = one_zero_response(self.damp, omega);
        let total_delay = SR_F64 / frequency;
        // High fundamentals have only a few samples of round-trip delay.
        // Reduce (never increase) the requested stiffness when its allpass
        // phase would consume the delay budget needed by the causal line.
        let mut effective_dispersion = self.dispersion_coefficient;
        let available_dispersion_delay = (total_delay - damping_delay - 2.25).max(0.0);
        for _ in 0..12 {
            let candidate_delay = if effective_dispersion.abs() > 1e-12 {
                2.0 * allpass_phase_delay(effective_dispersion, omega)
            } else {
                0.0
            };
            if candidate_delay <= available_dispersion_delay + 1e-9 {
                break;
            }
            effective_dispersion *= 0.72;
        }
        // Phase 0 §A, sites 1 and 2 of 3: the dispersion pair.
        let retune = self.retune;
        let account = self.observe;
        let mut dispersion_work = [None, None];
        for (section, work) in self.dispersion.iter_mut().zip(dispersion_work.iter_mut()) {
            *work = section.retune_coefficient(effective_dispersion, retune, account);
        }
        for work in dispersion_work {
            self.record_allpass_work(work);
        }
        let dispersion_delay = if effective_dispersion.abs() > 1e-12 {
            2.0 * allpass_phase_delay(effective_dispersion, omega)
        } else {
            0.0
        };
        let remaining = (total_delay - damping_delay - dispersion_delay).max(2.25);
        let mut integer_delay = (remaining - 0.20).floor().max(2.0) as usize;
        integer_delay = integer_delay.min(self.line.len().saturating_sub(2));
        let mut fractional_delay = remaining - integer_delay as f64;
        if fractional_delay > 1.95 && integer_delay + 1 < self.line.len() {
            integer_delay += 1;
            fractional_delay -= 1.0;
        }
        if fractional_delay < 0.02 && integer_delay > 2 {
            integer_delay -= 1;
            fractional_delay += 1.0;
        }
        // DELIBERATELY UNSOLVED (Ariadne Phase 3, consensus §6): changing
        // `integer_delay` resizes the active delay segment while every
        // cell keeps its value. That is the same silent reinterpretation
        // the allpass sites above now repair, applied to a state whose
        // RANK changes — and no metric-consistent transport for it exists
        // yet. `NEXT-PHASE-REPORT.md` §4 shows the tempting shortcut
        // (scalar-normalized resampling) closes the energy ledger while
        // destroying waveform fidelity (correlation as low as 0.471), so
        // it is explicitly NOT applied here under either mode. What Phase
        // 0 does instead is MEASURE the reinterpretation, so Phase 3
        // inherits a number rather than an assumption.
        if account && integer_delay != self.integer_delay {
            let before = self.line_energy();
            self.integer_delay = integer_delay;
            let jump = self.line_energy() - before;
            self.account.work_line_reinterpretation += jump;
            self.account.work_line_reinterpretation_peak =
                self.account.work_line_reinterpretation_peak.max(jump.abs());
        } else {
            self.integer_delay = integer_delay;
        }
        // Phase 0 §A, site 3 of 3: the fractional-delay section.
        let fractional_work = self.fractional.retune_coefficient(
            allpass_coefficient_for_delay(fractional_delay, omega),
            retune,
            account,
        );
        self.record_allpass_work(fractional_work);

        // Fundamental T60 calibration. The allpasses have unit magnitude;
        // divide out the damping filter's fundamental magnitude so `sus`
        // still means what it says. Very dark settings may require gain >1
        // to compensate; refusing that keeps the total loop contractive.
        let target_circulation_gain = 10_f64.powf(-3.0 / (self.t60 * frequency));
        self.loop_gain = (target_circulation_gain / damping_magnitude).min(0.999_999_5);
    }

    fn read(&mut self, sample_index: usize) -> f64 {
        if sample_index.is_multiple_of(16) {
            self.update_loop(sample_index);
        }
        let read_index =
            (self.write_index + self.line.len() - self.integer_delay) % self.line.len();
        let delayed = self.line[read_index];
        if self.observe {
            // Exact, not an estimate: `mus_watsn::onezero` proves
            // `u^2/2 + d*x1^2/2 - (y^2/2 + d*u^2/2) = d*(1-d)*(u-x1)^2/2`
            // is an identity for this filter.
            self.account.loss_damping +=
                mus_watsn::onezero_dissipation(self.damp, delayed, self.damping_x1);
        }
        let mut value = (1.0 - self.damp) * delayed + self.damp * self.damping_x1;
        self.damping_x1 = delayed;
        if self.dispersion[0].coefficient.abs() > 1e-12 {
            value = self.dispersion[0].tick(value);
            value = self.dispersion[1].tick(value);
        }
        value = self.fractional.tick(value);
        if self.observe {
            let gain = self.loop_gain;
            self.account.loss_loop_gain += 0.5 * (1.0 - gain * gain) * value * value;
        }
        value *= self.loop_gain;
        self.energy_smooth = 0.999 * self.energy_smooth + 0.001 * value * value;
        value
    }

    fn excitation_at(&self, sample_index: usize) -> f64 {
        let Some(age) = sample_index.checked_sub(self.start_sample) else {
            return 0.0;
        };
        self.excitation.get(age).copied().unwrap_or(0.0)
    }

    /// Contractive contact. The feedback value never exceeds the incoming
    /// magnitude; the removed energy appears only in the radiated contact
    /// signal, never back inside the loop.
    fn apply_contact(&mut self, value: f64, buzz: f64) -> (f64, f64) {
        if buzz <= 1e-12 {
            return (value, 0.0);
        }
        let clearance = 1.22 - 0.88 * buzz;
        let magnitude = value.abs();
        if magnitude <= clearance {
            self.contact_memory *= 0.72;
            return (value, self.contact_memory * 0.08 * buzz);
        }
        let over = magnitude - clearance;
        let hardness = 1.0 + 11.0 * buzz;
        let compressed_magnitude = clearance + (hardness * over).tanh() / hardness;
        let feedback = value.signum() * compressed_magnitude.min(magnitude);
        if self.observe {
            // The contact is contractive by construction, so this is a
            // genuine removal from the loop, not a tap.
            self.account.loss_contact += 0.5 * (value * value - feedback * feedback);
        }
        let removed = value - feedback;
        let radiation = removed - 0.78 * self.contact_memory;
        self.contact_memory = removed;
        (feedback, radiation)
    }

    fn write(&mut self, value: f64) {
        self.line[self.write_index] = value;
        self.write_index = (self.write_index + 1) % self.line.len();
    }
}

#[derive(Debug)]
struct BodyMode {
    q: f64,
    p: f64,
    cos_omega: f64,
    sin_omega: f64,
    radius: f64,
    coupling: f64,
    radiation: f64,
}

impl BodyMode {
    fn new(frequency: f64, t60: f64, coupling: f64, radiation: f64) -> Self {
        let omega = 2.0 * PI * frequency / SR_F64;
        Self {
            q: 0.0,
            p: 0.0,
            cos_omega: omega.cos(),
            sin_omega: omega.sin(),
            radius: 10_f64.powf(-3.0 / (t60 * SR_F64)),
            coupling,
            radiation,
        }
    }

    fn free_tick(&mut self) {
        let q = self.radius * (self.cos_omega * self.q - self.sin_omega * self.p);
        let p = self.radius * (self.sin_omega * self.q + self.cos_omega * self.p);
        self.q = q;
        self.p = p;
    }
}

/// Exact real Givens rotation. `a²+b²` is invariant up to floating-point
/// rounding for every finite angle.
fn givens(a: f64, b: f64, angle: f64) -> (f64, f64) {
    let (sin_angle, cos_angle) = angle.sin_cos();
    (cos_angle * a - sin_angle * b, sin_angle * a + cos_angle * b)
}

/// Closed-form Frobenius norm of the commutator defect between rotations
/// on overlapping planes R01(a) and R12(b), divided by sqrt(2).
///
/// It is zero when either rotation vanishes and is approximately `|a*b|`
/// for small angles. In Weave this is an interpretable scalar measure of
/// how strongly changing the order of two coupling gestures can change the
/// resulting sound.
pub fn weave_holonomy_defect(angle_a: f64, angle_b: f64) -> f64 {
    let (sa, ca) = angle_a.sin_cos();
    let (sb, cb) = angle_b.sin_cos();
    (((ca - 1.0).powi(2) * sb.powi(2)) + ((cb - 1.0).powi(2) * sa.powi(2)) + (sa * sb).powi(2))
        .sqrt()
}

/// Build the modal body for `profile_name` (falls back to guitar for an
/// unrecognised name, per [`body_profiles::profile_for`]), scaled by
/// `size` exactly as the single-table version always scaled
/// `BODY_MODE_SPECS`.
fn make_body(size: f64, profile_name: &str) -> Vec<BodyMode> {
    body_profiles::profile_for(profile_name)
        .modes
        .iter()
        .map(|&(frequency, t60, coupling, radiation)| {
            BodyMode::new(frequency / size, t60, coupling, radiation)
        })
        .collect()
}

fn virtual_course_frequency(base: f64, ordinal: usize, dimension: f64) -> f64 {
    // Weyl-like mode counting N(f) proportional to f^d, inverted to
    // f_k proportional to k^(1/d), then octave-folded into a playable
    // four-octave window. Irrational d gives an inharmonic but structured
    // spectrum rather than arbitrary detuning.
    let mut ratio = (ordinal as f64 + 2.0).powf(1.0 / dimension.max(0.55));
    while ratio > 8.0 {
        ratio *= 0.5;
    }
    while ratio < 1.0 {
        ratio *= 2.0;
    }
    base * ratio
}

/// Per-course breath state: a content-keyed noise stream, one-pole
/// lowpassed. Allocated only when `breath > 1e-12` at construction time
/// (the discipline `sing`'s external drive already follows) — never in
/// [`StringNetworkVoice::next_sample`], which only ticks existing state.
struct BreathChannel {
    rng: Pcg64,
    /// One-pole coefficient `a` in `y += a * (x - y)`, mapped from the
    /// per-course cutoff (`2.5 * f0`, clamped below `Nyquist * 0.45`) via
    /// the standard `a = 1 - exp(-2*pi*fc/SR)` one-pole time constant.
    filter_coefficient: f64,
    filter_state: f64,
}

impl BreathChannel {
    fn new(cutoff_hz: f64, seed: u64) -> Self {
        let mut sequence = SeedSequence::new(seed);
        let rng = Pcg64::from_seed_sequence(&mut sequence);
        let omega = 2.0 * PI * cutoff_hz / SR_F64;
        let filter_coefficient = (1.0 - (-omega).exp()).clamp(0.0, 1.0);
        Self {
            rng,
            filter_coefficient,
            filter_state: 0.0,
        }
    }

    /// One lowpassed noise sample. Real-time-safe: no allocation, just a
    /// PRNG draw and a one-pole update.
    fn tick(&mut self) -> f64 {
        let noise = uniform_bipolar(&mut self.rng);
        self.filter_state += self.filter_coefficient * (noise - self.filter_state);
        self.filter_state
    }
}

/// Stateful real-time-safe kernel shared by offline MUS rendering and a
/// future VST/CLAP wrapper. Construction allocates; [`next_sample`] and
/// [`render_block`] do not.
pub struct StringNetworkVoice {
    mode: StringNetworkMode,
    patch: PluckPatch,
    strings: Vec<DelayString>,
    body: Vec<BodyMode>,
    waves: Vec<f64>,
    incoming: Vec<f64>,
    contact: Vec<f64>,
    /// Per-edge |angle| applied this sample (forward + reverse pass),
    /// kept for [`Self::render_traced`] telemetry; never read by audio.
    edge_activity: Vec<f64>,
    /// External bridge drive (`sing`): the track's sample, offset by
    /// `sing_at`, fed to every course during the slot. `None` when
    /// sing = 0 so the un-sung path stays byte-identical.
    external: Option<Vec<f64>>,
    external_gain: f64,
    drive_samples: usize,
    /// Per-course breath noise channels (`breath`): bridge-common like
    /// `sing`, one entry per string, empty when `breath <= 1e-12` at
    /// construction so the render stays byte-identical to the un-breathed
    /// path.
    breath: Vec<BreathChannel>,
    /// Orbit phase accumulator: advanced by 2*pi*orbit_hz/SR per sample so
    /// retargeting `orbit` mid-note bends the field's speed without
    /// jumping its position (crank semantics, PW9).
    orbit_phase: f64,
    active_count: usize,
    sample_index: usize,
    total_samples: usize,
    previous_input: f64,
    previous_output: f64,
    /// Phase 0 §B observer switch, set by the render entry point and by
    /// nothing else. Observation is not part of the score, so there is no
    /// patch param and no [`ControlTarget`] for it.
    observe: bool,
    /// Network-level observer accumulators; inert while `observe` is false.
    account: NetworkEnergyAccount,
    /// Ariadne Phase 2 §B: the armed gesture word, stably sorted by
    /// sample and fully validated by [`StringNetworkVoice::arm_gestures`].
    /// Empty on every ordinary render.
    gestures: Vec<GestureFrame>,
    /// Cursor into `gestures`: the next frame not yet executed.
    next_gesture: usize,
    /// Observer record of executed gestures; empty while none are armed.
    gesture_trace: GestureTrace,
}

/// Network-level observer accumulators (Ariadne Phase 0 §B). Running
/// integrals from sample 0; written only while the observer is enabled.
#[derive(Debug, Default, Clone, Copy)]
struct NetworkEnergyAccount {
    /// Body-mode decay loss, `(1 - r^2) * (q^2 + p^2) / 2` per tick.
    loss_body: f64,
    /// Measured work of the Weave scattering pass. Every rotation is an
    /// exact Givens, so this is a roundoff-level control, not a term the
    /// ledger expects to be large.
    work_scatter: f64,
    /// Measured work of the body-coupling rotation pass, same argument.
    /// Excludes `free_tick` decay, which is counted as `loss_body`.
    work_body_scatter: f64,
    /// Sum of squared output samples (pre-master, post-DC-blocker). The
    /// audio tap READS `incoming`; it never removes energy from the loop,
    /// so this is recorded as an observation and is deliberately NOT a
    /// term in the state ledger. See `PHASE0-TRANSIENT-REPORT.md`.
    output_energy: f64,
}

impl StringNetworkVoice {
    pub fn guitar(
        params: &BTreeMap<String, String>,
        frequencies_hz: &[f64],
        slot_s: f64,
        gliss_ratio: f64,
    ) -> Self {
        Self::new(
            params,
            frequencies_hz,
            slot_s,
            gliss_ratio,
            StringNetworkMode::Guitar,
        )
    }

    pub fn weave(
        params: &BTreeMap<String, String>,
        frequencies_hz: &[f64],
        slot_s: f64,
        gliss_ratio: f64,
    ) -> Self {
        Self::new(
            params,
            frequencies_hz,
            slot_s,
            gliss_ratio,
            StringNetworkMode::Weave,
        )
    }

    pub fn guitar_sung(
        params: &BTreeMap<String, String>,
        frequencies_hz: &[f64],
        slot_s: f64,
        gliss_ratio: f64,
        external: &[f32],
    ) -> Self {
        let mut voice = Self::new(
            params,
            frequencies_hz,
            slot_s,
            gliss_ratio,
            StringNetworkMode::Guitar,
        );
        voice.attach_external(external);
        voice
    }

    pub fn weave_sung(
        params: &BTreeMap<String, String>,
        frequencies_hz: &[f64],
        slot_s: f64,
        gliss_ratio: f64,
        external: &[f32],
    ) -> Self {
        let mut voice = Self::new(
            params,
            frequencies_hz,
            slot_s,
            gliss_ratio,
            StringNetworkMode::Weave,
        );
        voice.attach_external(external);
        voice
    }

    /// The `sing` seam: bridge-common drive for every course during the
    /// slot. At sing = 0 nothing is attached and the render stays
    /// byte-identical to the un-sung path (tested).
    fn attach_external(&mut self, external: &[f32]) {
        if self.patch.sing <= 1e-12 {
            return;
        }
        let offset = (self.patch.sing_at * SR_F64).round() as usize;
        let window: Vec<f64> = external
            .iter()
            .skip(offset)
            .take(self.drive_samples)
            .map(|&sample| sample as f64)
            .collect();
        // Injected coherently into n courses; keep the summed direct
        // level roughly count-independent.
        self.external_gain = self.patch.sing * 0.9 / (self.strings.len().max(1) as f64).sqrt();
        self.external = Some(window);
    }

    fn new(
        params: &BTreeMap<String, String>,
        frequencies_hz: &[f64],
        slot_s: f64,
        gliss_ratio: f64,
        mode: StringNetworkMode,
    ) -> Self {
        let mut patch = PluckPatch::from(params);
        if patch.pm {
            patch.sus = (patch.sus * 0.14).max(0.05);
            patch.damp = (patch.damp + 0.38).min(0.95);
            patch.pick *= 0.48;
            patch.tension_cents *= 0.22;
            patch.symp *= 0.35;
        }

        let slot_samples = (slot_s.max(0.0) * SR_F64).round().max(1.0) as usize;
        let strum_samples = (patch.strum_ms.abs() * SR_F64 / 1000.0).round() as usize;
        let pitch_count = frequencies_hz.len().max(1);
        // The pluck articulates less as the external drive takes over;
        // at sing = 1 the sample is the only excitation. Breath and bow
        // displace the burst the same way (a bowed string is not plucked
        // first; a fully breathed note is bow-bloom, not a pick attack),
        // and the sustained excitations compose by multiplying what each
        // leaves of the burst — no hidden priority. This scale is read
        // once at construction: automating `bow` mid-note moves the
        // friction drive, never the burst that already happened.
        let base_level = 0.82 / (pitch_count as f64).sqrt()
            * (1.0 - patch.sing)
            * (1.0 - patch.bow)
            * (1.0 - patch.breath);
        let mut starts = vec![0_usize; frequencies_hz.len()];
        if patch.strum_ms >= 0.0 {
            for (order, start) in starts.iter_mut().enumerate() {
                *start = order * strum_samples;
            }
        } else {
            let len = starts.len();
            for (index, start) in starts.iter_mut().enumerate() {
                *start = (len - 1 - index) * strum_samples;
            }
        }

        let mut strings = Vec::new();
        let mut course_index = 0_u64;
        for (pitch_index, &frequency) in frequencies_hz.iter().enumerate() {
            if frequency < 20.0 {
                continue;
            }
            let f1 = frequency * gliss_ratio.max(0.05);
            let spec = DelayStringSpec {
                f0: frequency,
                f1,
                t60: patch.sus,
                damp: patch.damp,
                stiff: patch.stiff,
                tension_cents: patch.tension_cents,
                start_sample: starts[pitch_index],
                slot_samples,
                coupling_scale: 1.0,
                max_downward_ratio: 2.4,
                retune: patch.retune,
            };
            let excitation =
                physical_excitation(frequency, slot_s, &patch, course_index, mode, base_level);
            strings.push(DelayString::new(spec, excitation));
            course_index += 1;

            if patch.detune > 0.0 {
                let ratio = 2_f64.powf(patch.detune / 1200.0);
                let detuned_f0 = frequency * ratio;
                let detuned_spec = DelayStringSpec {
                    f0: detuned_f0,
                    f1: f1 * ratio,
                    t60: patch.sus * 1.03,
                    damp: (patch.damp + 0.025).min(0.95),
                    stiff: patch.stiff * 0.92,
                    tension_cents: patch.tension_cents * 0.82,
                    start_sample: starts[pitch_index],
                    slot_samples,
                    coupling_scale: 0.78,
                    max_downward_ratio: 2.4,
                    retune: patch.retune,
                };
                let detuned_excitation = physical_excitation(
                    detuned_f0,
                    slot_s,
                    &patch,
                    course_index,
                    mode,
                    base_level * 0.48,
                );
                strings.push(DelayString::new(detuned_spec, detuned_excitation));
                course_index += 1;
            }
        }
        let active_count = strings.len();

        match mode {
            StringNetworkMode::Guitar if patch.symp > 1e-12 => {
                for &frequency in &OPEN_GUITAR_STRINGS {
                    let spec = DelayStringSpec {
                        f0: frequency,
                        f1: frequency,
                        t60: (patch.sus * 1.35).min(20.0),
                        damp: (patch.damp + 0.08).min(0.88),
                        stiff: patch.stiff * 0.70,
                        tension_cents: 0.0,
                        start_sample: 0,
                        slot_samples,
                        coupling_scale: patch.symp,
                        max_downward_ratio: 1.25,
                        retune: patch.retune,
                    };
                    strings.push(DelayString::new(spec, Vec::new()));
                }
            }
            StringNetworkMode::Weave => {
                let base = frequencies_hz
                    .iter()
                    .copied()
                    .filter(|frequency| *frequency >= 20.0)
                    .reduce(f64::min)
                    .unwrap_or(110.0);
                let target = patch.courses.max(active_count).min(24);
                let mut ordinal = 0;
                while strings.len() < target {
                    let frequency =
                        virtual_course_frequency(base, ordinal, patch.spectral_dimension)
                            .clamp(25.0, 9_000.0);
                    let spec = DelayStringSpec {
                        f0: frequency,
                        f1: frequency,
                        t60: (patch.sus * (1.0 + 0.045 * ordinal as f64)).min(20.0),
                        damp: patch.damp,
                        stiff: (patch.stiff + 0.18 * ordinal as f64 / target as f64).min(1.0),
                        tension_cents: 0.0,
                        start_sample: 0,
                        slot_samples,
                        coupling_scale: 1.0,
                        max_downward_ratio: 1.3,
                        retune: patch.retune,
                    };
                    strings.push(DelayString::new(spec, Vec::new()));
                    ordinal += 1;
                }
            }
            _ => {}
        }

        let n_strings = strings.len();
        // Preallocated at construction only — never in `next_sample`. A
        // note that never engages breath allocates nothing extra and its
        // render stays byte-identical to the un-breathed path.
        let breath = if patch.breath > 1e-12 {
            let nyquist_limit = SR_F64 * 0.5 * 0.45;
            strings
                .iter()
                .enumerate()
                .map(|(index, string)| {
                    let seed = breath_seed(string.f0, slot_s, &patch, index as u64, mode);
                    // Per-course cutoff: 2.5 * f0, clamped below Nyquist *
                    // 0.45 so high courses can't alias the lowpass itself.
                    let cutoff_hz = (2.5 * string.f0).min(nyquist_limit);
                    BreathChannel::new(cutoff_hz, seed)
                })
                .collect()
        } else {
            Vec::new()
        };
        let tail = (patch.sus * 0.78).min(4.0);
        let total_samples = ((slot_s + tail).max(64.0 / SR_F64) * SR_F64) as usize;
        Self {
            mode,
            body: make_body(patch.body_size, &patch.bodyp),
            patch,
            strings,
            waves: vec![0.0; n_strings],
            incoming: vec![0.0; n_strings],
            contact: vec![0.0; n_strings],
            edge_activity: vec![0.0; n_strings],
            active_count,
            sample_index: 0,
            total_samples,
            external: None,
            external_gain: 0.0,
            drive_samples: slot_samples,
            breath,
            orbit_phase: 0.0,
            previous_input: 0.0,
            previous_output: 0.0,
            observe: false,
            account: NetworkEnergyAccount::default(),
            gestures: Vec::new(),
            next_gesture: 0,
            gesture_trace: GestureTrace::default(),
        }
    }

    /// Arm the Phase 0 §B observer on this voice and every course.
    /// Private: the only way in is a render entry point.
    fn set_observer(&mut self, observe: bool) {
        self.observe = observe;
        for string in &mut self.strings {
            string.observe = observe;
        }
    }

    /// Total declared stored energy of the whole network: every course's
    /// active delay segment and filter memory, plus every body mode.
    fn stored_energy(&self) -> f64 {
        let strings: f64 = self.strings.iter().map(DelayString::stored_energy).sum();
        strings + self.body_energy()
    }

    fn body_energy(&self) -> f64 {
        self.body
            .iter()
            .map(|mode| 0.5 * (mode.q * mode.q + mode.p * mode.p))
            .sum()
    }

    fn wave_energy(&self) -> f64 {
        0.5 * self.waves.iter().map(|wave| wave * wave).sum::<f64>()
    }

    pub fn is_finished(&self) -> bool {
        self.sample_index >= self.total_samples
    }

    pub fn remaining_samples(&self) -> usize {
        self.total_samples.saturating_sub(self.sample_index)
    }

    fn scatter_weave(&mut self) {
        let count = self.waves.len();
        if count < 2 || self.patch.couple <= 1e-12 {
            self.edge_activity.iter_mut().for_each(|slot| *slot = 0.0);
            return;
        }
        let phase = self.orbit_phase;
        let total_energy = self
            .strings
            .iter()
            .map(|string| string.energy_smooth)
            .sum::<f64>()
            .max(1e-14);
        let forward_weight = 0.5 * (1.0 + self.patch.chirality);
        let backward_weight = 0.5 * (1.0 - self.patch.chirality);

        for index in 0..count {
            let next = (index + 1) % count;
            let travelling = 1.0
                + self.patch.orbit_depth * (phase + 2.0 * PI * index as f64 / count as f64).sin();
            let previous_energy = self.strings[(index + count - 1) % count].energy_smooth;
            let next_energy = self.strings[next].energy_smooth;
            let gradient = ((previous_energy - next_energy) * count as f64 / total_energy).tanh();
            let metric = 1.0 + 0.72 * self.patch.curvature * gradient;
            let angle =
                (self.patch.couple * travelling * metric * forward_weight).clamp(-0.48, 0.48);
            self.edge_activity[index] = angle.abs();
            let (a, b) = givens(self.waves[index], self.waves[next], angle);
            self.waves[index] = a;
            self.waves[next] = b;
        }
        for reverse_index in (0..count).rev() {
            let next = (reverse_index + 1) % count;
            let travelling = 1.0
                + self.patch.orbit_depth
                    * (phase + 2.0 * PI * reverse_index as f64 / count as f64).sin();
            let previous_energy = self.strings[(reverse_index + count - 1) % count].energy_smooth;
            let next_energy = self.strings[next].energy_smooth;
            let gradient = ((previous_energy - next_energy) * count as f64 / total_energy).tanh();
            let metric = 1.0 + 0.72 * self.patch.curvature * gradient;
            let angle =
                (self.patch.couple * travelling * metric * backward_weight).clamp(-0.48, 0.48);
            self.edge_activity[reverse_index] += angle.abs();
            let (a, b) = givens(self.waves[reverse_index], self.waves[next], angle);
            self.waves[reverse_index] = a;
            self.waves[next] = b;
        }
    }

    /// Arm a gesture word on this voice (Ariadne Phase 2 §B).
    ///
    /// Validation is total and happens HERE, before a single sample is
    /// rendered: an index outside the network's course set is a typed
    /// refusal, never a clamp onto a course the author did not name.
    /// Frames are sorted by sample with a STABLE sort, so frames sharing
    /// a sample execute in the order the caller wrote them — the word's
    /// order is the word's meaning, and a sort must not permute it.
    ///
    /// Zero-limit law: arming an empty list allocates nothing and leaves
    /// the per-sample path with one integer comparison that no float ever
    /// sees, so the render stays byte-identical to the un-gestured one.
    ///
    /// Real-time discipline: every buffer the executor writes is sized
    /// here, from `gestures.len()`, so [`Self::next_sample`] still
    /// allocates nothing.
    pub fn arm_gestures(&mut self, gestures: &[GestureFrame]) -> Result<(), GestureError> {
        let courses = self.strings.len();
        for frame in gestures {
            for course in [frame.course_a, frame.course_b] {
                if course >= courses {
                    return Err(GestureError::CourseOutOfRange {
                        at_sample: frame.at_sample,
                        course,
                        courses,
                    });
                }
            }
            // NOT in the spec's list, and refused for the same reason it
            // lists out-of-range: `givens(w, w, theta)` is not a plane
            // rotation, it is a scaling by sqrt(2) — accepting it would
            // break the energy-neutrality law two lines below by a factor,
            // not by roundoff. A degenerate pair is an authoring mistake
            // with no defensible interpretation, so it is a refusal.
            if frame.course_a == frame.course_b {
                return Err(GestureError::DegeneratePair {
                    at_sample: frame.at_sample,
                    course: frame.course_a,
                });
            }
            // Same discipline: a non-finite angle would poison the whole
            // network through the ring on the next sample.
            if !frame.theta.is_finite() {
                return Err(GestureError::NonFiniteAngle {
                    at_sample: frame.at_sample,
                    theta: frame.theta,
                });
            }
        }
        self.gestures.clear();
        self.gestures.extend_from_slice(gestures);
        self.gestures.sort_by_key(|frame| frame.at_sample);
        self.next_gesture = 0;
        self.gesture_trace = GestureTrace::with_capacity(courses, gestures.len());
        Ok(())
    }

    /// The observer record of the gestures executed so far. `&self` by
    /// design, exactly like [`EnergyTrace::push_frame`]: a reader that
    /// cannot reach `&mut` cannot perturb the render.
    pub fn gesture_trace(&self) -> &GestureTrace {
        &self.gesture_trace
    }

    /// Execute every armed gesture whose sample has arrived.
    ///
    /// Called once per sample from [`Self::next_sample`], immediately
    /// after the scattering pass, on the same `waves` the ring's own
    /// Givens rotations act on. `<=` rather than `==` so a frame armed at
    /// a sample the voice has already passed still executes (at the first
    /// sample it can), matching the catch-up semantics
    /// `render_traced_core` gives [`ControlFrame`]s.
    fn apply_gestures(&mut self) {
        while self.next_gesture < self.gestures.len()
            && self.gestures[self.next_gesture].at_sample <= self.sample_index
        {
            let frame = self.gestures[self.next_gesture];
            self.next_gesture += 1;
            let (a, b) = (frame.course_a, frame.course_b);
            self.gesture_trace.waves.extend_from_slice(&self.waves);
            let before = (self.waves[a], self.waves[b]);
            // Byte-inert no-op law, the discipline
            // [`FirstOrderAllpass::retune_coefficient`] applies to an
            // unchanged coefficient: a zero angle performs NO state
            // arithmetic at all. `cos 0 = 1` and `sin 0 = 0` are exact,
            // but `0.0 * a + 1.0 * b` is not sign-of-zero-inert (it maps
            // `-0.0` to `+0.0`), so the rotation is SKIPPED rather than
            // trusted to be the identity. `!= 0.0` catches `-0.0` too,
            // which is also an identity rotation.
            if frame.theta != 0.0 {
                let (rotated_a, rotated_b) = givens(before.0, before.1, frame.theta);
                self.waves[a] = rotated_a;
                self.waves[b] = rotated_b;
            }
            let after = (self.waves[a], self.waves[b]);
            let pair_energy_before = before.0 * before.0 + before.1 * before.1;
            let pair_energy_after = after.0 * after.0 + after.1 * after.1;
            // The localized "factor-stage work is zero" gate (Phase 2 §B
            // law 3). It is an assertion here and a measurement in the
            // receipt below: debug builds trip on the spot, release
            // builds carry the number out to the test and to the report.
            debug_assert!(
                (pair_energy_after - pair_energy_before).abs()
                    <= pair_energy_before.max(1.0) * 1e-12,
                "gesture at {} on ({a},{b}) moved pair energy {pair_energy_before} -> \
                 {pair_energy_after}",
                frame.at_sample
            );
            self.gesture_trace.waves.extend_from_slice(&self.waves);
            self.gesture_trace.applied.push(AppliedGesture {
                at_sample: self.sample_index,
                course_a: a,
                course_b: b,
                theta: frame.theta,
                pair_energy_before,
                pair_energy_after,
            });
        }
    }

    fn scatter_body(&mut self) -> f64 {
        let count = self.waves.len();
        let observe = self.observe;
        if count == 0 || self.patch.body <= 1e-12 {
            let mut decayed = 0.0;
            for mode in &mut self.body {
                let before = 0.5 * (mode.q * mode.q + mode.p * mode.p);
                mode.free_tick();
                if observe {
                    decayed += before - 0.5 * (mode.q * mode.q + mode.p * mode.p);
                }
            }
            self.account.loss_body += decayed;
            return 0.0;
        }
        let chirality = if self.mode == StringNetworkMode::Weave {
            self.patch.chirality
        } else {
            0.0
        };
        let forward_weight = 0.5 * (1.0 + chirality);
        let backward_weight = 0.5 * (1.0 - chirality);
        let mut body_output = 0.0;
        let mut decayed = 0.0;
        for mode in &mut self.body {
            let before = 0.5 * (mode.q * mode.q + mode.p * mode.p);
            mode.free_tick();
            if observe {
                decayed += before - 0.5 * (mode.q * mode.q + mode.p * mode.p);
            }
            let base_angle = self.patch.body * mode.coupling;
            for index in 0..count {
                let angle = base_angle * self.strings[index].coupling_scale * forward_weight;
                let (wave, momentum) = givens(self.waves[index], mode.p, angle);
                self.waves[index] = wave;
                mode.p = momentum;
            }
            for index in (0..count).rev() {
                let angle = base_angle * self.strings[index].coupling_scale * backward_weight;
                let (wave, momentum) = givens(self.waves[index], mode.p, angle);
                self.waves[index] = wave;
                mode.p = momentum;
            }
            body_output += mode.radiation * mode.q;
        }
        self.account.loss_body += decayed;
        body_output
    }

    /// Render one mono sample. No allocation occurs on this path.
    pub fn next_sample(&mut self) -> f32 {
        self.next_sample_bridged(None)
    }

    /// One sample, optionally bridged to a persistent protected backbone
    /// (unified U3). `None` is byte-identical to the un-bridged engine:
    /// the bridge term is exactly `0.0` and no backbone code runs. The
    /// observer telemetry ledger does not yet own a bridge column, so
    /// bridged rendering with `observe` on is deferred to the U4/U5
    /// integration (documented in `UNIFIED-U3-persistent-backbone.md`).
    pub fn next_sample_bridged(
        &mut self,
        backbone: Option<&mut crate::backbone::ProtectedBackbone>,
    ) -> f32 {
        if self.is_finished() || self.strings.is_empty() {
            return 0.0;
        }
        for index in 0..self.strings.len() {
            let wave = self.strings[index].read(self.sample_index);
            self.waves[index] = wave;
            self.incoming[index] = wave;
        }
        // Phase 0 §B: every observer reading below is taken from state
        // that the audio step has already finished writing, and no
        // observer value is ever read back by the audio path.
        let observe = self.observe;
        let waves_before_scatter = if observe { self.wave_energy() } else { 0.0 };
        if self.mode == StringNetworkMode::Weave {
            self.scatter_weave();
        }
        // Ariadne Phase 2 §B: compiled gestures execute here — after the
        // ring's own scattering pass, on the same wave variables, before
        // the body sees them. Inert unless a word is armed.
        //
        // Deliberately INSIDE the `work_scatter` observation window: a
        // gesture is an exact Givens rotation on the scattering state, so
        // its work is roundoff by construction (asserted at the site) and
        // booking it with the ring's keeps the Phase 0 §B ledger closed
        // under gestures without inventing a column Phase 0 does not own.
        self.apply_gestures();
        if observe {
            self.account.work_scatter += self.wave_energy() - waves_before_scatter;
        }
        // Unified U3: the persistent backbone bridges here — the same wave
        // variables the gesture seam owns, after the ring's scattering and
        // the gesture work booking, before the body sees them. The step
        // returns the room port's radiated contribution for this sample.
        let backbone_emitted = match backbone {
            Some(backbone) => {
                let take = self.active_count.min(self.waves.len());
                backbone.step(&mut self.waves[..take])
            }
            None => 0.0,
        };
        // The body rotations move energy between the waves and the modal
        // momenta; `free_tick` decay is booked separately as `loss_body`,
        // so it is added back to isolate the rotation's own work.
        let before_body = if observe {
            self.wave_energy() + self.body_energy()
        } else {
            0.0
        };
        let body_loss_before = self.account.loss_body;
        let body_output = self.scatter_body();
        if observe {
            let after_body = self.wave_energy() + self.body_energy();
            self.account.work_body_scatter +=
                after_body - before_body + (self.account.loss_body - body_loss_before);
        }

        let drive = match &self.external {
            Some(window) => {
                self.external_gain * window.get(self.sample_index).copied().unwrap_or(0.0)
            }
            None => 0.0,
        };
        // The bow bites only while the note is held (the same drive window
        // `sing` reads through) and only on ACTIVE courses: sympathetic
        // open strings and Weave's virtual courses are not under the hair,
        // they answer through the bridge.
        let bowing = self.patch.bow > 1e-12 && self.sample_index < self.drive_samples;
        let bow_velocity = BOW_VELOCITY_MIN
            + (BOW_VELOCITY_MAX - BOW_VELOCITY_MIN) * self.patch.bow_speed.clamp(0.0, 1.0);

        let mut contact_output = 0.0;
        for index in 0..self.strings.len() {
            let coupling_scale = self.strings[index].coupling_scale;
            // Sustained noise excitation, bridge-common like `sing`: only
            // while this course is inside its own slot window, and only
            // when the channel was preallocated (breath > 1e-12 at
            // construction).
            //
            // Course-set asymmetry (deliberate, spec-directed): `breath`
            // is bridge-common, so like `sing` it reaches EVERY course —
            // sympathetic and Weave-virtual included (their windows span
            // the whole slot). The bow below touches only ACTIVE courses.
            // Air fills the box; the hair touches the string.
            let start_sample = self.strings[index].start_sample;
            let slot_samples = self.strings[index].slot_samples;
            let in_slot = self.sample_index >= start_sample
                && self.sample_index < start_sample + slot_samples;
            let breath = if in_slot && self.patch.breath > 1e-12 {
                match self.breath.get_mut(index) {
                    Some(channel) => self.patch.breath * 0.25 * coupling_scale * channel.tick(),
                    None => 0.0,
                }
            } else {
                0.0
            };
            let burst = self.strings[index].excitation_at(self.sample_index);
            let drive_here = drive * coupling_scale;
            if observe {
                // Ordered attribution, declared: the sources are booked in
                // the order burst, drive, breath, bow, each against the
                // wave the previous ones already raised. The split of a
                // sum of squares is order-dependent; this is the order.
                let wave = self.waves[index];
                let account = &mut self.strings[index].account;
                account.inject_burst += wave * burst + 0.5 * burst * burst;
                let after_burst = wave + burst;
                account.inject_drive += after_burst * drive_here + 0.5 * drive_here * drive_here;
                let after_drive = after_burst + drive_here;
                account.inject_breath += after_drive * breath + 0.5 * breath * breath;
            }
            let mut excitation = burst + drive_here + breath;
            if bowing && index < self.active_count {
                // v1 coupling: explicit and ONE-STEP. The loop value that
                // has just arrived at the excitation point stands in for
                // the string velocity under the hair, the friction force
                // it implies is injected at the same point the pluck burst
                // enters, and nothing is iterated. The physical model
                // solves force and velocity simultaneously (the friction
                // curve intersected with the string's wave impedance line,
                // McIntyre-Woodhouse); iterating this to convergence — or
                // better, the closed-form intersection — is the upgrade.
                //
                // HONESTY (PW10 §B, house law): the bow is an energy
                // SOURCE. The falling friction branch is negative
                // resistance; it PUMPS the network. The module's rotation
                // argument bounds the unforced network only and says
                // nothing here, so no passivity is claimed for any patch
                // with bow > 0. What is claimed is empirical: measured
                // boundedness over a declared corner grid, below. The
                // round-3 ask this defers to is the nonlinear source-port
                // work contract — a ledger that accounts the bow's
                // injected work per sample instead of measuring its peak.
                let relative_velocity = bow_velocity - self.waves[index];
                let force =
                    BOW_GAIN * self.patch.bow * bow_friction(relative_velocity) * coupling_scale;
                let wave_before_bow = self.waves[index] + excitation;
                let string = &mut self.strings[index];
                string.bow_dc += BOW_DC_POLE * (force - string.bow_dc);
                let injected = force - string.bow_dc;
                if observe {
                    string.account.inject_bow +=
                        wave_before_bow * injected + 0.5 * injected * injected;
                }
                excitation += injected;
            }
            let value = self.waves[index] + excitation;
            let (feedback, contact) = self.strings[index].apply_contact(value, self.patch.buzz);
            self.contact[index] = contact;
            contact_output += contact;
            self.strings[index].write(feedback);
        }

        let active_normalization = (self.active_count.max(1) as f64).sqrt();
        let string_direct =
            self.incoming.iter().take(self.active_count).sum::<f64>() / active_normalization;
        let bridge_force = self
            .incoming
            .iter()
            .zip(self.waves.iter())
            .map(|(before, after)| before - after)
            .sum::<f64>()
            / (self.strings.len().max(1) as f64).sqrt();
        contact_output /= (self.strings.len().max(1) as f64).sqrt();

        let raw = 0.18 * string_direct
            + (0.92 + 0.52 * self.patch.body) * body_output
            + 0.18 * bridge_force
            + 0.20 * contact_output
            + backbone_emitted;
        // One-pole DC blocker, useful under strongly curved Weave motion.
        let output = raw - self.previous_input + 0.995 * self.previous_output;
        self.previous_input = raw;
        self.previous_output = output;
        self.orbit_phase =
            (self.orbit_phase + 2.0 * PI * self.patch.orbit_hz / SR_F64).rem_euclid(2.0 * PI);
        self.sample_index += 1;
        let scaled = 0.72 * output;
        if observe {
            // Recorded, but NOT a ledger term: the pickup reads `incoming`
            // and the body's radiation reads `mode.q` — neither removes
            // anything from the state it observes. Treating the output as
            // a loss would double-count. See PHASE0-TRANSIENT-REPORT.md.
            self.account.output_energy += 0.5 * scaled * scaled;
        }
        scaled as f32
    }

    /// Fill a host-provided block and return the number of live samples.
    /// Any unused tail is explicitly zeroed.
    pub fn render_block(&mut self, output: &mut [f32]) -> usize {
        let mut written = 0;
        for sample in output.iter_mut() {
            if self.is_finished() {
                *sample = 0.0;
            } else {
                *sample = self.next_sample();
                written += 1;
            }
        }
        written
    }

    pub fn render(mut self) -> Vec<f32> {
        let mut output = vec![0.0; self.total_samples];
        self.render_block(&mut output);
        output
    }

    /// Render bridged to a persistent backbone that outlives this note.
    /// The backbone advances one step per rendered sample; its radiated
    /// port lands in this note's output. Between-note gaps are the
    /// caller's to advance (`ProtectedBackbone::advance_gap`), so the
    /// backbone's absolute clock is the caller's responsibility.
    pub fn render_bridged(
        mut self,
        backbone: &mut crate::backbone::ProtectedBackbone,
    ) -> Vec<f32> {
        let mut output = vec![0.0; self.total_samples];
        for sample in output.iter_mut() {
            if self.is_finished() {
                *sample = 0.0;
            } else {
                *sample = self.next_sample_bridged(Some(backbone));
            }
        }
        output
    }

    /// Retarget one control now. Patch-only controls take effect on the
    /// next sample (scatter/body/contact re-read the patch every sample);
    /// `Sus`/`Damp` are additionally pushed into every course so the
    /// 16-sample coefficient update picks them up. The loop-gain clamp
    /// keeps every reachable state contractive, so no retarget can make
    /// the network expansive.
    pub fn apply_control(&mut self, target: ControlTarget, value: f64) {
        match target {
            ControlTarget::Couple => self.patch.couple = value.clamp(0.0, 0.45),
            ControlTarget::Chirality => self.patch.chirality = value.clamp(-1.0, 1.0),
            ControlTarget::OrbitHz => self.patch.orbit_hz = value.clamp(0.0, 20.0),
            ControlTarget::OrbitDepth => self.patch.orbit_depth = value.clamp(0.0, 1.0),
            ControlTarget::Curvature => self.patch.curvature = value.clamp(0.0, 1.0),
            ControlTarget::Body => self.patch.body = value.clamp(0.0, 1.0),
            ControlTarget::Buzz => self.patch.buzz = value.clamp(0.0, 1.0),
            ControlTarget::Sing => {
                self.patch.sing = value.clamp(0.0, 1.0);
                if self.external.is_some() {
                    self.external_gain =
                        self.patch.sing * 0.9 / (self.strings.len().max(1) as f64).sqrt();
                }
            }
            // Pressure and speed are re-read every sample, so a crescendo
            // is a trajectory of `Bow` frames. Retargeting cannot restore
            // the pluck burst: `1 - bow` was applied to the excitation
            // table at construction (documented there).
            ControlTarget::Bow => self.patch.bow = value.clamp(0.0, 1.0),
            ControlTarget::BowSpeed => self.patch.bow_speed = value.clamp(0.0, 1.0),
            ControlTarget::Sus => {
                // Decay retargets from here on; buffer length and onset
                // tension were sized at construction and stay put.
                self.patch.sus = value.clamp(0.05, 20.0);
                for string in &mut self.strings {
                    string.t60 = self.patch.sus;
                }
            }
            ControlTarget::Damp => {
                self.patch.damp = value.clamp(0.0, 0.95);
                for string in &mut self.strings {
                    string.damp = self.patch.damp;
                }
            }
            ControlTarget::Breath => {
                // Like `Sing`: the value always updates, but the noise
                // channels themselves were only preallocated in `new()`
                // when breath started above the threshold — a note that
                // starts un-breathed cannot grow the subsystem mid-flight.
                self.patch.breath = value.clamp(0.0, 1.0);
            }
        }
    }

    /// Render under a control trajectory, tracing. Frames apply at their
    /// sample boundary, in order; an empty trajectory is byte-identical
    /// to [`Self::render_traced`] (tested). Determinism law: the output
    /// is a pure function of (patch, notes, external, trajectory).
    pub fn render_traced_with(
        self,
        stride: usize,
        trajectory: &[ControlFrame],
    ) -> (Vec<f32>, WeaveTrace) {
        let (audio, trace, _, _) = self.render_traced_core(stride, trajectory, false);
        (audio, trace)
    }

    /// Render under a control trajectory AND a compiled gesture word
    /// (Ariadne Phase 2 §B), tracing. Gestures execute at their sample,
    /// after that sample's scattering pass, on the network's own wave
    /// variables; frames sharing a sample execute in the order given.
    ///
    /// **Zero-limit law (hard):** an EMPTY gesture list is byte-identical
    /// to [`Self::render_traced_with`] for the same inputs, on pluck and
    /// weave alike. Tested directly.
    ///
    /// Refuses, rather than clamps, a word naming a course this network
    /// does not have — see [`Self::arm_gestures`]. Nothing is rendered
    /// when it refuses.
    ///
    /// Accepts a word on the guitar too: `pluck` has no ring, but it has
    /// the same wave variables, and the executor is a statement about
    /// those. Whether a word is *offered* on a given voice is a layer up
    /// — `renderWeaveProbe` refuses non-weave gestures with
    /// `unsupported-gesture` (Phase 2 §B), because the ring is what makes
    /// a word musically meaningful.
    ///
    /// Determinism law, unchanged in shape: the output is a pure function
    /// of (patch, notes, external, trajectory, gestures).
    pub fn render_traced_gestured(
        mut self,
        stride: usize,
        trajectory: &[ControlFrame],
        gestures: &[GestureFrame],
    ) -> Result<(Vec<f32>, WeaveTrace, GestureTrace), GestureError> {
        self.arm_gestures(gestures)?;
        let (audio, trace, _, gesture_trace) = self.render_traced_core(stride, trajectory, false);
        Ok((audio, trace, gesture_trace))
    }

    /// The Phase 0 §B observer seam: the same render, with the energy
    /// ledger recorded. `observe` is a render-entry-point argument and
    /// deliberately not a patch param — observation is not part of the
    /// score, so it cannot be authored, automated, or digested into a
    /// receipt.
    ///
    /// **Byte law:** the returned audio is bit-for-bit what
    /// [`Self::render_traced_with`] returns for the same inputs. Every
    /// observer reading is taken from state the audio step has already
    /// written, and no accumulator is ever read back. Tested directly.
    pub fn render_traced_observed(
        self,
        stride: usize,
        trajectory: &[ControlFrame],
    ) -> (Vec<f32>, WeaveTrace, EnergyTrace) {
        let (audio, trace, energy, _) = self.render_traced_core(stride, trajectory, true);
        (audio, trace, energy)
    }

    fn render_traced_core(
        mut self,
        stride: usize,
        trajectory: &[ControlFrame],
        observe: bool,
    ) -> (Vec<f32>, WeaveTrace, EnergyTrace, GestureTrace) {
        self.set_observer(observe);
        let mut frames: Vec<&ControlFrame> = trajectory.iter().collect();
        frames.sort_by_key(|frame| frame.at_sample);
        let stride = stride.max(1);
        let courses = self.strings.len();
        let course_freqs: Vec<f64> = self.strings.iter().map(|string| string.f0).collect();
        let played = self.active_count;
        let total = self.total_samples;
        let trace_frames = total.div_ceil(stride);
        let mut data = Vec::with_capacity(trace_frames * (2 * courses + 1));
        let mut energy = EnergyTrace::new(stride, courses, observe.then_some(trace_frames));
        let initial_energy = if observe { self.stored_energy() } else { 0.0 };
        let mut audio = vec![0.0_f32; total];
        let mut next_frame = 0;
        for (index, slot) in audio.iter_mut().enumerate() {
            while next_frame < frames.len() && frames[next_frame].at_sample <= index {
                for &(target, value) in &frames[next_frame].sets {
                    self.apply_control(target, value);
                }
                next_frame += 1;
            }
            let phase_now = self.orbit_phase;
            *slot = self.next_sample();
            if index.is_multiple_of(stride) {
                for string in &self.strings {
                    data.push(string.energy_smooth.sqrt() as f32);
                }
                for edge in &self.edge_activity {
                    data.push(*edge as f32);
                }
                data.push(phase_now as f32);
                if observe {
                    energy.push_frame(&self);
                }
            }
        }
        energy.initial_energy = initial_energy;
        energy.final_energy = if observe { self.stored_energy() } else { 0.0 };
        (
            audio,
            WeaveTrace {
                stride,
                courses,
                played,
                course_freqs,
                frames: trace_frames,
                data,
            },
            energy,
            std::mem::take(&mut self.gesture_trace),
        )
    }

    /// Render while sampling telemetry every `stride` samples. The audio
    /// is byte-identical to [`Self::render`] — the trace is a tap, never
    /// a participant, and an empty trajectory is exactly no trajectory.
    pub fn render_traced(self, stride: usize) -> (Vec<f32>, WeaveTrace) {
        self.render_traced_with(stride, &[])
    }
}

/// Column names of [`EnergyTrace::data`], in order. Energies are declared
/// stored energy at the frame's sample; every `inject_*`, `loss_*` and
/// `work_*` column is a RUNNING INTEGRAL from sample 0, so the ledger
/// between any two frames is a subtraction.
pub const ENERGY_TRACE_COLUMNS: &[&str] = &[
    "state_total",
    "line_energy",
    "filter_energy",
    "body_energy",
    "contact_state",
    "inject_burst",
    "inject_drive",
    "inject_breath",
    "inject_bow",
    "loss_damping",
    "loss_loop_gain",
    "loss_contact",
    "loss_body",
    "work_scatter",
    "work_body_scatter",
    "work_allpass_retune",
    "work_line_reinterpretation",
    "output_energy",
];

/// Per-course columns of [`EnergyTrace::course_data`], in order.
pub const ENERGY_TRACE_COURSE_COLUMNS: &[&str] = &["line_energy", "filter_energy"];

/// Ariadne Phase 0 §B telemetry: an observer-only energy ledger over the
/// whole production network.
///
/// The declared state is the union of every course's ACTIVE delay segment
/// (Euclidean `sum(cell^2)/2`), every course's filter memory (three
/// first-order allpass storages `E_a` plus the one-zero damping storage
/// `E_z`), and every body mode's `(q^2 + p^2)/2`. The ledger law it
/// satisfies is
///
/// ```text
/// final - initial = sum(inject) + sum(work) - sum(loss)
/// ```
///
/// with `output_energy` recorded but NOT a term: the pickup and the body
/// radiation are taps that read state without removing it.
///
/// When the observer is disabled `frames` is 0 and both vectors are empty.
#[derive(Debug, Clone, Default)]
pub struct EnergyTrace {
    pub stride: usize,
    pub courses: usize,
    pub frames: usize,
    /// Frame-major, [`ENERGY_TRACE_COLUMNS`] values per frame.
    pub data: Vec<f64>,
    /// Frame-major, `courses * ENERGY_TRACE_COURSE_COLUMNS.len()` per frame.
    pub course_data: Vec<f64>,
    /// Declared stored energy before the first sample.
    pub initial_energy: f64,
    /// Declared stored energy after the last sample.
    pub final_energy: f64,
    /// Largest single-update allpass coefficient work over the render,
    /// maximized across courses (the Phase 0 §A law-4 quantity).
    pub allpass_update_peak: f64,
    /// Largest single `integer_delay`-change line reinterpretation over
    /// the render, maximized across courses (Phase 3's open problem).
    pub line_reinterpretation_peak: f64,
}

impl EnergyTrace {
    fn new(stride: usize, courses: usize, frames: Option<usize>) -> Self {
        let frames = frames.unwrap_or(0);
        Self {
            stride,
            courses,
            frames: 0,
            data: Vec::with_capacity(frames * ENERGY_TRACE_COLUMNS.len()),
            course_data: Vec::with_capacity(frames * courses * ENERGY_TRACE_COURSE_COLUMNS.len()),
            initial_energy: 0.0,
            final_energy: 0.0,
            allpass_update_peak: 0.0,
            line_reinterpretation_peak: 0.0,
        }
    }

    /// Read one frame off the voice. `&StringNetworkVoice` by design:
    /// an observer that cannot reach `&mut` cannot perturb the render.
    fn push_frame(&mut self, voice: &StringNetworkVoice) {
        let mut line = 0.0;
        let mut filter = 0.0;
        let mut contact_state = 0.0;
        let mut inject = [0.0_f64; 4];
        let mut loss = [0.0_f64; 3];
        let mut work = [0.0_f64; 2];
        for string in &voice.strings {
            let course_line = string.line_energy();
            let course_filter = string.filter_energy();
            line += course_line;
            filter += course_filter;
            contact_state += 0.5 * string.contact_memory * string.contact_memory;
            let account = &string.account;
            inject[0] += account.inject_burst;
            inject[1] += account.inject_drive;
            inject[2] += account.inject_breath;
            inject[3] += account.inject_bow;
            loss[0] += account.loss_damping;
            loss[1] += account.loss_loop_gain;
            loss[2] += account.loss_contact;
            work[0] += account.work_allpass_retune;
            work[1] += account.work_line_reinterpretation;
            self.allpass_update_peak = self
                .allpass_update_peak
                .max(account.work_allpass_retune_peak);
            self.line_reinterpretation_peak = self
                .line_reinterpretation_peak
                .max(account.work_line_reinterpretation_peak);
            self.course_data.push(course_line);
            self.course_data.push(course_filter);
        }
        let body = voice.body_energy();
        let network = &voice.account;
        self.data.extend_from_slice(&[
            line + filter + body,
            line,
            filter,
            body,
            contact_state,
            inject[0],
            inject[1],
            inject[2],
            inject[3],
            loss[0],
            loss[1],
            loss[2],
            network.loss_body,
            network.work_scatter,
            network.work_body_scatter,
            work[0],
            work[1],
            network.output_energy,
        ]);
        self.frames += 1;
    }

    /// One column of one frame.
    pub fn get(&self, frame: usize, column: &str) -> Option<f64> {
        let index = ENERGY_TRACE_COLUMNS.iter().position(|c| *c == column)?;
        self.data
            .get(frame * ENERGY_TRACE_COLUMNS.len() + index)
            .copied()
    }

    /// The last recorded value of one column.
    pub fn last(&self, column: &str) -> Option<f64> {
        self.get(self.frames.checked_sub(1)?, column)
    }

    /// Ledger residual over the whole render: what the accounted flows
    /// predict for the final state, minus the state actually measured.
    /// Zero to roundoff is the Phase 0 §B gate.
    pub fn ledger_residual(&self) -> f64 {
        let sum = |names: &[&str]| -> f64 {
            names
                .iter()
                .map(|name| self.last(name).unwrap_or(0.0))
                .sum()
        };
        let injected = sum(&[
            "inject_burst",
            "inject_drive",
            "inject_breath",
            "inject_bow",
        ]);
        let lost = sum(&[
            "loss_damping",
            "loss_loop_gain",
            "loss_contact",
            "loss_body",
        ]);
        let worked = sum(&[
            "work_scatter",
            "work_body_scatter",
            "work_allpass_retune",
            "work_line_reinterpretation",
        ]);
        // The final frame is sampled at the last stride boundary, so
        // compare against the state at that same frame, not `final_energy`.
        let measured = self.last("state_total").unwrap_or(0.0);
        (self.initial_energy + injected + worked - lost) - measured
    }
}

/// Telemetry from [`StringNetworkVoice::render_traced`]. `data` is
/// frame-major: each frame is `[energy[courses], edge_activity[courses],
/// orbit_phase]` — `2*courses + 1` values, `frames` frames. Energy is the
/// square root of the loop's smoothed mean-square (an amplitude, ready to
/// draw); edge activity is the applied |Givens angle| per ring edge
/// (forward + reverse pass); phase is the orbit field's angle in radians.
pub struct WeaveTrace {
    pub stride: usize,
    pub courses: usize,
    pub played: usize,
    pub course_freqs: Vec<f64>,
    pub frames: usize,
    pub data: Vec<f32>,
}

/// A parameter the network can retarget mid-note at a sample boundary
/// (PW9 §4). Only controls that are re-read from the patch every sample
/// (or safely re-pushed into course state) are listed; topology and
/// excitation-shaping parameters remain construction-static.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlTarget {
    Couple,
    Chirality,
    OrbitHz,
    OrbitDepth,
    Curvature,
    Body,
    Buzz,
    Sing,
    Bow,
    BowSpeed,
    Sus,
    Damp,
    Breath,
}

impl ControlTarget {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "couple" => Self::Couple,
            "chirality" => Self::Chirality,
            "orbit" => Self::OrbitHz,
            "orbit_depth" => Self::OrbitDepth,
            "curvature" => Self::Curvature,
            "body" => Self::Body,
            "buzz" => Self::Buzz,
            "sing" => Self::Sing,
            "bow" => Self::Bow,
            "bow_speed" => Self::BowSpeed,
            "sus" => Self::Sus,
            "damp" => Self::Damp,
            "breath" => Self::Breath,
            _ => return None,
        })
    }
}

/// One point on a control trajectory: at `at_sample`, retarget each
/// listed control (engine clamps apply, same as parse-time). Frames are
/// content: a trajectory-driven render is a pure function of
/// (patch, notes, trajectory) — trajectories digest into receipts.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlFrame {
    pub at_sample: usize,
    pub sets: Vec<(ControlTarget, f64)>,
}

/// One executable gate of a compiled control word (Ariadne Phase 2 §B).
///
/// At `at_sample`, immediately after that sample's scattering pass, the
/// network rotates the wave variables of `course_a` and `course_b` by
/// `theta` — the exact Givens rotation the ring itself uses, on exactly
/// the state the ring acts on. A word is a list of these; frames sharing
/// a sample execute in list order, because the order of a word is its
/// meaning.
///
/// Courses are indexed over the WHOLE network — played courses first (in
/// score order, each optionally followed by its `detune` companion), then
/// weave's virtual courses — the same indexing [`WeaveTrace::course_freqs`]
/// reports. An index outside that set is a typed refusal, not a clamp.
///
/// A gesture is a probe/word argument, not a score param: there is no
/// vocab entry and no [`ControlTarget`] for it, exactly as a trajectory
/// is authored as frames rather than as a knob.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GestureFrame {
    pub at_sample: usize,
    pub course_a: usize,
    pub course_b: usize,
    pub theta: f64,
}

/// Why a gesture word was refused. Refusal is a result: the word is
/// rejected whole, before any audio exists, and nothing is clamped into
/// range on the author's behalf.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GestureError {
    /// A gate named a course this network does not have.
    CourseOutOfRange {
        at_sample: usize,
        course: usize,
        courses: usize,
    },
    /// A gate named the same course twice: not a plane rotation.
    DegeneratePair { at_sample: usize, course: usize },
    /// A gate carried a non-finite angle.
    NonFiniteAngle { at_sample: usize, theta: f64 },
}

impl GestureError {
    /// Stable machine-readable code, the discipline
    /// `mus_dsp::scene::SceneError` follows for the service layer.
    pub fn code(&self) -> &'static str {
        match self {
            Self::CourseOutOfRange { .. } => "gesture-course-out-of-range",
            Self::DegeneratePair { .. } => "gesture-degenerate-pair",
            Self::NonFiniteAngle { .. } => "gesture-nonfinite-angle",
        }
    }
}

impl std::fmt::Display for GestureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CourseOutOfRange {
                at_sample,
                course,
                courses,
            } => write!(
                formatter,
                "gesture at sample {at_sample}: course {course} is outside this network's \
                 {courses} courses"
            ),
            Self::DegeneratePair { at_sample, course } => write!(
                formatter,
                "gesture at sample {at_sample}: course {course} rotated against itself is not \
                 a plane rotation"
            ),
            Self::NonFiniteAngle { at_sample, theta } => write!(
                formatter,
                "gesture at sample {at_sample}: angle {theta} is not finite"
            ),
        }
    }
}

impl std::error::Error for GestureError {}

/// Observer record of one executed gate. Written after the audio step
/// that produced it and read back by nothing on the audio path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppliedGesture {
    /// The sample it actually executed on (equal to the frame's
    /// `at_sample` unless the word was armed after that sample passed).
    pub at_sample: usize,
    pub course_a: usize,
    pub course_b: usize,
    pub theta: f64,
    /// `w_a^2 + w_b^2` before the rotation, and after it. The Phase 2 §B
    /// law-3 quantity: these differ by roundoff, never by a factor.
    pub pair_energy_before: f64,
    pub pair_energy_after: f64,
}

/// The gesture receipt of one render: every gate that executed, with the
/// whole wave vector before and after it.
///
/// Unconditional (unlike the Phase 0 §B energy observer, which is a
/// render-entry-point decision) because it is O(gates), not O(samples):
/// a word of a dozen gates costs a dozen rows, and a render with no word
/// armed carries an empty trace with no capacity. It is what the
/// same-endpoint experiment reads to report trace-endpoint state
/// distances.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GestureTrace {
    /// Courses in the network — the stride of [`Self::waves`] snapshots.
    pub courses: usize,
    /// One row per executed gate, in execution order.
    pub applied: Vec<AppliedGesture>,
    /// Snapshot-major: `2 * courses` values per executed gate, the whole
    /// wave vector immediately before that gate and immediately after it.
    pub waves: Vec<f64>,
}

impl GestureTrace {
    /// Size every buffer up front so the executor never allocates on the
    /// audio path. `gestures` is an upper bound: a gate scheduled past
    /// the end of the note simply never executes.
    fn with_capacity(courses: usize, gestures: usize) -> Self {
        Self {
            courses,
            applied: Vec::with_capacity(gestures),
            waves: Vec::with_capacity(gestures * 2 * courses),
        }
    }

    /// The whole wave vector immediately before executed gate `index`.
    pub fn waves_before(&self, index: usize) -> Option<&[f64]> {
        let start = index * 2 * self.courses;
        self.waves.get(start..start + self.courses)
    }

    /// The whole wave vector immediately after executed gate `index`.
    pub fn waves_after(&self, index: usize) -> Option<&[f64]> {
        let start = index * 2 * self.courses + self.courses;
        self.waves.get(start..start + self.courses)
    }

    /// Worst RELATIVE pair-energy drift over every executed gate — the
    /// law-3 measurement in one number, ready for a receipt. Zero when no
    /// gate executed.
    pub fn worst_pair_energy_drift(&self) -> f64 {
        self.applied
            .iter()
            .map(|gate| {
                (gate.pair_energy_after - gate.pair_energy_before).abs()
                    / gate.pair_energy_before.max(f64::MIN_POSITIVE)
            })
            .fold(0.0, f64::max)
    }
}

/// Render the deepened physical guitar. Existing callers keep the same
/// signature and `synth=pluck` dispatch.
pub fn pluck_note(
    params: &BTreeMap<String, String>,
    frequencies_hz: &[f64],
    slot_s: f64,
    gliss_ratio: f64,
) -> Vec<f32> {
    StringNetworkVoice::guitar(params, frequencies_hz, slot_s, gliss_ratio).render()
}

/// Render the impossible string network selected by `synth=weave`.
pub fn weave_note(
    params: &BTreeMap<String, String>,
    frequencies_hz: &[f64],
    slot_s: f64,
    gliss_ratio: f64,
) -> Vec<f32> {
    StringNetworkVoice::weave(params, frequencies_hz, slot_s, gliss_ratio).render()
}

/// `sing > 0`: the guitar driven by an external sample through the
/// shared bridge (cuerdas simpaticas).
pub fn pluck_note_sung(
    params: &BTreeMap<String, String>,
    frequencies_hz: &[f64],
    slot_s: f64,
    gliss_ratio: f64,
    external: &[f32],
) -> Vec<f32> {
    StringNetworkVoice::guitar_sung(params, frequencies_hz, slot_s, gliss_ratio, external).render()
}

/// `sing > 0`: the weave driven by an external sample.
pub fn weave_note_sung(
    params: &BTreeMap<String, String>,
    frequencies_hz: &[f64],
    slot_s: f64,
    gliss_ratio: f64,
    external: &[f32],
) -> Vec<f32> {
    StringNetworkVoice::weave_sung(params, frequencies_hz, slot_s, gliss_ratio, external).render()
}

#[cfg(test)]
mod mathematical_tests {
    use super::*;

    fn apply_01(vector: [f64; 3], angle: f64) -> [f64; 3] {
        let (a, b) = givens(vector[0], vector[1], angle);
        [a, b, vector[2]]
    }

    fn apply_12(vector: [f64; 3], angle: f64) -> [f64; 3] {
        let (a, b) = givens(vector[1], vector[2], angle);
        [vector[0], a, b]
    }

    fn norm_squared(vector: &[f64]) -> f64 {
        vector.iter().map(|value| value * value).sum()
    }

    #[test]
    fn every_givens_step_is_energy_exact() {
        for (a, b, angle) in [
            (0.7, -0.2, -2.0),
            (1e-9, 3.0, 0.0),
            (-1.2, 0.8, 0.37),
            (40.0, -20.0, PI),
        ] {
            let before = a * a + b * b;
            let (x, y) = givens(a, b, angle);
            let after = x * x + y * y;
            assert!((after - before).abs() <= before.max(1.0) * 2e-13);
        }
    }

    #[test]
    fn state_dependent_angles_remain_pointwise_norm_preserving() {
        let mut state: [f64; 4] = [0.8, -0.3, 0.2, 0.7];
        let before = norm_squared(&state);
        for step in 0..2_000 {
            let gradient: f64 = state[0] * state[0] - state[2] * state[2];
            let angle: f64 = 0.17 * (0.01 * step as f64).sin() * (1.0 + 0.8 * gradient.tanh());
            let left = step % 4;
            let right = (step + 1) % 4;
            let (a, b) = givens(state[left], state[right], angle);
            state[left] = a;
            state[right] = b;
        }
        let after = norm_squared(&state);
        assert!((after - before).abs() < 2e-11, "{before} -> {after}");
    }

    #[test]
    fn overlapping_rotations_have_audible_holonomy() {
        let angle_a = 0.31;
        let angle_b = -0.47;
        let basis = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let mut frobenius_squared = 0.0;
        for vector in basis {
            let ab = apply_12(apply_01(vector, angle_a), angle_b);
            let ba = apply_01(apply_12(vector, angle_b), angle_a);
            frobenius_squared += ab
                .iter()
                .zip(ba.iter())
                .map(|(left, right)| (left - right).powi(2))
                .sum::<f64>();
        }
        let measured = (0.5 * frobenius_squared).sqrt();
        let closed_form = weave_holonomy_defect(angle_a, angle_b);
        assert!((measured - closed_form).abs() < 1e-13);
        assert!(closed_form > 0.1, "defect={closed_form}");
    }

    /// Ariadne Phase 0 §A, the structural half of the legacy byte
    /// argument: `retune=legacy` reaches no state arithmetic at all.
    /// The transport is unreachable from this arm, so no future change to
    /// `mus_watsn::neutral_allpass_state_transport` can move a legacy
    /// render — which is a stronger statement than any A/B of outputs.
    #[test]
    fn legacy_mode_never_calls_the_transport() {
        let mut section = FirstOrderAllpass {
            coefficient: 0.05,
            x1: 0.37,
            y1: -0.29,
        };
        let before = section;
        let work = section.retune_coefficient(0.96, RetuneMode::Legacy, true);
        assert_eq!(section.coefficient, 0.96);
        assert_eq!(section.x1.to_bits(), before.x1.to_bits(), "x1 is retained");
        assert_eq!(section.y1.to_bits(), before.y1.to_bits(), "y1 is retained");
        // The honest positive control: legacy retuning silently
        // reinterprets stored energy, and here is the number.
        assert!(
            work.expect("accounted").abs() > 0.01,
            "legacy coefficient change does hidden work: {work:?}"
        );
    }

    /// The neutral arm applies exactly the mus-watsn law — the engine
    /// does not re-derive it, so this test compares against the module.
    #[test]
    fn neutral_mode_applies_the_mus_watsn_transport() {
        for (old, new) in [(0.05, 0.96), (-0.7, 0.4), (0.92, -0.98), (0.0, 0.61)] {
            let mut section = FirstOrderAllpass {
                coefficient: old,
                x1: 0.37,
                y1: -0.29,
            };
            let work = section
                .retune_coefficient(new, RetuneMode::NeutralFilters, true)
                .expect("accounted");
            let (x1, y1, receipt) =
                mus_watsn::neutral_allpass_state_transport(old, new, 0.37, -0.29);
            assert_eq!(section.x1.to_bits(), x1.to_bits());
            assert_eq!(section.y1.to_bits(), y1.to_bits());
            assert_eq!(work.to_bits(), receipt.control_work.to_bits());
            assert!(work.abs() <= 1e-12, "neutral work {work} is at roundoff");
        }
    }

    /// Byte-inert no-op, in BOTH modes: an unchanged clamped coefficient
    /// performs no state arithmetic whatsoever.
    #[test]
    fn a_noop_coefficient_update_is_byte_inert_in_both_modes() {
        for mode in [RetuneMode::Legacy, RetuneMode::NeutralFilters] {
            let mut section = FirstOrderAllpass {
                coefficient: 0.71,
                x1: -0.33,
                y1: 0.27,
            };
            let before = section;
            // Same value, and a value the clamp maps onto the same one.
            for request in [0.71_f64, 4.0, -4.0, 0.71] {
                let clamped = request.clamp(-0.999_5, 0.999_5);
                if clamped.to_bits() != section.coefficient.to_bits() {
                    section = before;
                    continue;
                }
                let work = section.retune_coefficient(request, mode, true);
                assert_eq!(work, Some(0.0), "{mode:?} no-op reports zero work");
            }
            assert_eq!(section.coefficient.to_bits(), before.coefficient.to_bits());
            assert_eq!(section.x1.to_bits(), before.x1.to_bits());
            assert_eq!(section.y1.to_bits(), before.y1.to_bits());
        }
    }

    /// Work accounting is observation: with the observer off, nothing is
    /// measured (`None`), which is not the same claim as "measured zero".
    #[test]
    fn unobserved_updates_report_nothing_rather_than_zero() {
        let mut section = FirstOrderAllpass {
            coefficient: 0.05,
            x1: 0.37,
            y1: -0.29,
        };
        assert_eq!(
            section.retune_coefficient(0.96, RetuneMode::Legacy, false),
            None
        );
        assert_eq!(
            section.retune_coefficient(0.05, RetuneMode::Legacy, false),
            None,
            "even a no-op reports nothing when unobserved"
        );
    }

    #[test]
    fn retune_parses_as_an_enum_with_a_legacy_fallback() {
        let patch = |value: &str| {
            PluckPatch::from(&BTreeMap::from([("retune".to_string(), value.to_string())])).retune
        };
        assert_eq!(patch("legacy"), RetuneMode::Legacy);
        assert_eq!(patch("neutral_filters"), RetuneMode::NeutralFilters);
        assert_eq!(
            patch("neutral"),
            RetuneMode::Legacy,
            "Phase 4's name is not live yet"
        );
        assert_eq!(patch(""), RetuneMode::Legacy);
        assert_eq!(
            PluckPatch::from(&BTreeMap::new()).retune,
            RetuneMode::Legacy,
            "absent means legacy"
        );
    }

    #[test]
    fn physical_excitation_midpoint_suppresses_even_modes() {
        let patch = PluckPatch {
            pos: 0.5,
            pick: 0.0,
            ..PluckPatch::default()
        };
        let excitation = physical_excitation(187.5, 1.0, &patch, 0, StringNetworkMode::Guitar, 1.0);
        let n = excitation.len();
        let coefficient = |harmonic: usize| -> f64 {
            let omega = 2.0 * PI * harmonic as f64 / n as f64;
            let real = excitation
                .iter()
                .enumerate()
                .map(|(index, value)| value * (omega * index as f64).cos())
                .sum::<f64>();
            let imag = excitation
                .iter()
                .enumerate()
                .map(|(index, value)| value * (omega * index as f64).sin())
                .sum::<f64>();
            real.hypot(imag)
        };
        assert!(coefficient(2) < coefficient(1) * 0.08);
    }
}
