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
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringNetworkMode {
    Guitar,
    Weave,
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

    fn set_coefficient(&mut self, coefficient: f64) {
        self.coefficient = coefficient.clamp(-0.999_5, 0.999_5);
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
        for section in &mut self.dispersion {
            section.set_coefficient(effective_dispersion);
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
        self.integer_delay = integer_delay;
        self.fractional
            .set_coefficient(allpass_coefficient_for_delay(fractional_delay, omega));

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
        let mut value = (1.0 - self.damp) * delayed + self.damp * self.damping_x1;
        self.damping_x1 = delayed;
        if self.dispersion[0].coefficient.abs() > 1e-12 {
            value = self.dispersion[0].tick(value);
            value = self.dispersion[1].tick(value);
        }
        value = self.fractional.tick(value) * self.loop_gain;
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
    /// Orbit phase accumulator: advanced by 2*pi*orbit_hz/SR per sample so
    /// retargeting `orbit` mid-note bends the field's speed without
    /// jumping its position (crank semantics, PW9).
    orbit_phase: f64,
    active_count: usize,
    sample_index: usize,
    total_samples: usize,
    previous_input: f64,
    previous_output: f64,
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
        // at sing = 1 the sample is the only excitation.
        let base_level = 0.82 / (pitch_count as f64).sqrt() * (1.0 - patch.sing);
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
                    };
                    strings.push(DelayString::new(spec, Vec::new()));
                    ordinal += 1;
                }
            }
            _ => {}
        }

        let n_strings = strings.len();
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
            orbit_phase: 0.0,
            previous_input: 0.0,
            previous_output: 0.0,
        }
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

    fn scatter_body(&mut self) -> f64 {
        let count = self.waves.len();
        if count == 0 || self.patch.body <= 1e-12 {
            for mode in &mut self.body {
                mode.free_tick();
            }
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
        for mode in &mut self.body {
            mode.free_tick();
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
        body_output
    }

    /// Render one mono sample. No allocation occurs on this path.
    pub fn next_sample(&mut self) -> f32 {
        if self.is_finished() || self.strings.is_empty() {
            return 0.0;
        }
        for index in 0..self.strings.len() {
            let wave = self.strings[index].read(self.sample_index);
            self.waves[index] = wave;
            self.incoming[index] = wave;
        }
        if self.mode == StringNetworkMode::Weave {
            self.scatter_weave();
        }
        let body_output = self.scatter_body();

        let drive = match &self.external {
            Some(window) => {
                self.external_gain * window.get(self.sample_index).copied().unwrap_or(0.0)
            }
            None => 0.0,
        };
        let mut contact_output = 0.0;
        for index in 0..self.strings.len() {
            let excitation = self.strings[index].excitation_at(self.sample_index)
                + drive * self.strings[index].coupling_scale;
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
            + 0.20 * contact_output;
        // One-pole DC blocker, useful under strongly curved Weave motion.
        let output = raw - self.previous_input + 0.995 * self.previous_output;
        self.previous_input = raw;
        self.previous_output = output;
        self.orbit_phase =
            (self.orbit_phase + 2.0 * PI * self.patch.orbit_hz / SR_F64).rem_euclid(2.0 * PI);
        self.sample_index += 1;
        (0.72 * output) as f32
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
        }
    }

    /// Render under a control trajectory, tracing. Frames apply at their
    /// sample boundary, in order; an empty trajectory is byte-identical
    /// to [`Self::render_traced`] (tested). Determinism law: the output
    /// is a pure function of (patch, notes, external, trajectory).
    pub fn render_traced_with(
        mut self,
        stride: usize,
        trajectory: &[ControlFrame],
    ) -> (Vec<f32>, WeaveTrace) {
        let mut frames: Vec<&ControlFrame> = trajectory.iter().collect();
        frames.sort_by_key(|frame| frame.at_sample);
        let stride = stride.max(1);
        let courses = self.strings.len();
        let course_freqs: Vec<f64> = self.strings.iter().map(|string| string.f0).collect();
        let played = self.active_count;
        let total = self.total_samples;
        let trace_frames = total.div_ceil(stride);
        let mut data = Vec::with_capacity(trace_frames * (2 * courses + 1));
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
            }
        }
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
        )
    }

    /// Render while sampling telemetry every `stride` samples. The audio
    /// is byte-identical to [`Self::render`] — the trace is a tap, never
    /// a participant, and an empty trajectory is exactly no trajectory.
    pub fn render_traced(self, stride: usize) -> (Vec<f32>, WeaveTrace) {
        self.render_traced_with(stride, &[])
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
    Sus,
    Damp,
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
            "sus" => Self::Sus,
            "damp" => Self::Damp,
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
