//! A bow: a friction port on a modal string, honestly a source.
//!
//! ```text
//! string velocity under the hair   v = dw / dt,  w = sqrt(dt) sum_n psi_n q_n   (psi_n = sin(n pi beta))
//! relative velocity                 dv = v_bow - v
//! friction                          F = F_N mu(dv),  mu(dv) = sign(dv) (mu_d + (mu_s - mu_d) / (1 + |dv| / v0))
//! stick                             dv = 0 while |F| <= mu_s F_N
//! ```
//!
//! This is the hyperbolic (Coulomb plus Stribeck) friction curve of
//! `SPECS/playable/PW10-bodies-bow-breath.md` §B, brought into the Ariadne
//! World as a contact law of the hammer's kind (`law::hammer`): one material
//! step holds the force `F` constant over the sample, and a modal oscillator
//! under a constant force is solved exactly, so the string's field changes by
//! exactly `F (w' - w)`. The force is found the way McIntyre and Woodhouse
//! draw it (J. Acoust. Soc. Am. 66, 1979): the friction curve intersected
//! with the string's load line, `v(F) = (dw_free + g F) / dt` with `g` the
//! bow point's compliance to a force held over the step. The string sticks
//! to the hair when the force that holds `v = v_bow` is within static
//! friction; otherwise it slips on the falling branch. Where the two are
//! both possible (a shallow load line against the falling branch) the
//! hysteresis rule keeps the regime the string was in.
//!
//! The books. The arm moves the bow at `v_bow` against `F`, so it does work
//! `F v_bow dt`: that is the bow's **source work**, the only place energy
//! enters. The string receives `F (w' - w)`. The difference, `F dv dt`, is
//! zero while sticking and positive while slipping (`F` and `dv` share a
//! sign): it is heat in the hair and the rosin, booked as **friction loss**.
//! Source minus loss is the string's gain to roundoff, so the World's ledger
//! closes with the bow in it. No passivity is claimed for a bowed string:
//! the falling friction branch is negative resistance, and the arm pays.
//!
//! What is not here: the bow's own mass and the hair's compliance (the bow
//! is a velocity source), the width of the hair ribbon (a point contact),
//! the string's torsional motion, and any thermal rosin law. The friction
//! constants are the PW10 starting points, labelled design-hypothesis.

use serde::{Deserialize, Serialize};

use crate::law::hammer::WireState;

const SCAN_POINTS: usize = 24;
const BISECTION_STEPS: usize = 48;

/// The friction curve. Every number is the anatomy's to declare and label.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BowSpec {
    /// `mu_s`: the hair's grip at rest.
    pub static_friction: f64,
    /// `mu_d`: the fully slipping coefficient (`< mu_s`).
    pub dynamic_friction: f64,
    /// `v0`: the slip velocity at which the curve has fallen halfway, in the
    /// string body's velocity units (`dw / dt`).
    pub slip_velocity: f64,
}

impl Default for BowSpec {
    /// PW10 §B's starting points (`mu_s = 0.8`, `mu_d = 0.3`); the knee of the
    /// curve is set where the steel-string's bowed notes speak (measured in
    /// `tests/bowed_string.rs`, design-hypothesis).
    fn default() -> Self {
        Self {
            static_friction: 0.8,
            dynamic_friction: 0.3,
            slip_velocity: 0.1,
        }
    }
}

impl BowSpec {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if !self.static_friction.is_finite()
            || !self.dynamic_friction.is_finite()
            || !self.slip_velocity.is_finite()
            || self.dynamic_friction <= 0.0
            || self.static_friction <= self.dynamic_friction
            || self.slip_velocity <= 0.0
        {
            return Err("a bow needs 0 < mu_d < mu_s and a positive slip velocity");
        }
        Ok(())
    }

    /// `mu_d + (mu_s - mu_d) / (1 + x / v0)` for a slip speed `x >= 0`.
    fn slipping_coefficient(&self, slip_speed: f64) -> f64 {
        self.dynamic_friction
            + (self.static_friction - self.dynamic_friction)
                / (1.0 + slip_speed / self.slip_velocity)
    }
}

/// What the arm does this sample.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BowControls {
    /// Bow velocity in the string body's velocity units; its sign is the
    /// bow's direction (a down-bow is positive, an up-bow negative).
    pub velocity: f64,
    /// Normal force (`F_N`) in the body's force units; `0` lifts the bow,
    /// exactly (nothing is stepped or booked).
    pub force: f64,
}

/// One sample of a bow.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BowFrame {
    /// The hair is on the string with a positive normal force.
    pub bowing: bool,
    pub sticking: bool,
    /// Friction force on the string this sample (held over the sample).
    pub force: f64,
    /// Mean string velocity under the hair over the sample.
    pub string_velocity: f64,
    /// `v_bow - string_velocity`.
    pub relative_velocity: f64,
    /// The arm's work, `F v_bow dt`.
    pub source_work: f64,
    /// Heat in the hair, `F dv dt >= 0`.
    pub friction_loss: f64,
}

/// Cumulative energies and counts.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BowTotals {
    pub bowed_samples: u64,
    pub stuck_samples: u64,
    /// Releases: samples at which the string went from sticking to slipping.
    pub slip_onsets: u64,
    pub source_work: f64,
    pub friction_loss: f64,
    pub peak_force: f64,
}

/// One bow's runtime state.
#[derive(Debug, Clone)]
pub struct FrictionBow {
    spec: BowSpec,
    dt: f64,
    controls: BowControls,
    sticking: bool,
    totals: BowTotals,
    last: BowFrame,
}

impl FrictionBow {
    pub fn new(spec: BowSpec, sample_rate_hz: f64) -> Self {
        Self {
            spec,
            dt: sample_rate_hz.recip(),
            controls: BowControls::default(),
            sticking: false,
            totals: BowTotals::default(),
            last: BowFrame::default(),
        }
    }

    #[must_use]
    pub fn spec(&self) -> &BowSpec {
        &self.spec
    }

    #[must_use]
    pub fn controls(&self) -> BowControls {
        self.controls
    }

    #[must_use]
    pub fn totals(&self) -> BowTotals {
        self.totals
    }

    #[must_use]
    pub fn last_frame(&self) -> BowFrame {
        self.last
    }

    /// Set the arm's velocity and normal force from this sample on. A
    /// non-finite or negative force lifts the bow.
    pub fn set_controls(&mut self, controls: BowControls) {
        let velocity = if controls.velocity.is_finite() {
            controls.velocity
        } else {
            0.0
        };
        let force = if controls.force.is_finite() {
            controls.force.max(0.0)
        } else {
            0.0
        };
        self.controls = BowControls { velocity, force };
        if force == 0.0 {
            self.sticking = false;
        }
    }

    /// One sample. `wire` describes the bow point now (`law::hammer::wire_state`
    /// with the bow's shape); returns the force to hold over the sample (the
    /// caller applies it with `apply_force`), the arm's work and the friction
    /// loss. With the bow lifted nothing is stepped: `(0, 0, 0)` exactly.
    pub(crate) fn step(&mut self, wire: WireState) -> (f64, f64, f64) {
        let normal = self.controls.force;
        let g = wire.compliance;
        if normal <= 0.0 || !g.is_finite() || g <= 0.0 {
            self.last = BowFrame::default();
            return (0.0, 0.0, 0.0);
        }
        let dt = self.dt;
        let v_bow = self.controls.velocity;
        let dw_free = wire.free - wire.start;
        // The force that would carry the bow point exactly with the hair.
        let f_stick = (v_bow * dt - dw_free) / g;
        let f_static = self.spec.static_friction * normal;
        let s = if f_stick >= 0.0 { 1.0 } else { -1.0 };
        let slip_speed = |f: f64| s * (v_bow - (dw_free + g * f) / dt);
        // Slipping on branch `s`: `F = s F_N mu(|dv|)` with `|dv| > 0`, which
        // confines `|F|` to `[mu_d F_N, min(mu_s F_N, |f_stick|))`.
        let slip_root = {
            let lower = self.spec.dynamic_friction * normal;
            let upper = f_static.min(f_stick.abs()) * (1.0 - 1.0e-12);
            if upper <= lower {
                None
            } else {
                let h = |f: f64| f - self.spec.slipping_coefficient(slip_speed(s * f)) * normal;
                let mut found = None;
                let mut previous = (lower, h(lower));
                for k in 1..=SCAN_POINTS {
                    let f = lower + (upper - lower) * k as f64 / SCAN_POINTS as f64;
                    let value = h(f);
                    if previous.1 <= 0.0 && value >= 0.0 {
                        let (mut lo, mut hi) = (previous.0, f);
                        for _ in 0..BISECTION_STEPS {
                            let mid = 0.5 * (lo + hi);
                            if h(mid) <= 0.0 {
                                lo = mid;
                            } else {
                                hi = mid;
                            }
                        }
                        found = Some(s * 0.5 * (lo + hi));
                        break;
                    }
                    previous = (f, value);
                }
                found
            }
        };
        let can_stick = f_stick.abs() <= f_static;
        // McIntyre-Woodhouse: stay in the regime the string is in while it is
        // possible; a sticking string holds until static friction gives way,
        // a slipping one until its branch of the curve runs out.
        let (force, sticking) = if self.sticking {
            if can_stick {
                (f_stick, true)
            } else {
                (slip_root.unwrap_or(s * f_static), false)
            }
        } else {
            match slip_root {
                Some(f) => (f, false),
                None if can_stick => (f_stick, true),
                None => (s * f_static, false),
            }
        };
        let dw = dw_free + g * force;
        let string_velocity = dw / dt;
        let source_work = force * v_bow * dt;
        let friction_loss = if sticking {
            0.0
        } else {
            (source_work - force * dw).max(0.0)
        };
        let relative_velocity = v_bow - string_velocity;
        if sticking {
            self.totals.stuck_samples += 1;
        } else if self.sticking {
            self.totals.slip_onsets += 1;
        }
        self.sticking = sticking;
        self.totals.bowed_samples += 1;
        self.totals.source_work += source_work;
        self.totals.friction_loss += friction_loss;
        self.totals.peak_force = self.totals.peak_force.max(force.abs());
        self.last = BowFrame {
            bowing: true,
            sticking,
            force,
            string_velocity,
            relative_velocity,
            source_work,
            friction_loss,
        };
        (force, source_work, friction_loss)
    }
}

/// The bow's shape on a modal string whose coordinates are its first `N`
/// partials: `sin(n pi beta)`, a point contact at fraction `beta` of the
/// speaking length.
#[must_use]
pub fn bow_shape<const N: usize>(bow_fraction: f64) -> [f64; N] {
    std::array::from_fn(|index| ((index as f64 + 1.0) * std::f64::consts::PI * bow_fraction).sin())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bow() -> FrictionBow {
        let mut bow = FrictionBow::new(BowSpec::default(), 48_000.0);
        bow.set_controls(BowControls {
            velocity: 1.0,
            force: 1.0e4,
        });
        bow
    }

    #[test]
    fn a_lifted_bow_steps_nothing() {
        let mut bow = FrictionBow::new(BowSpec::default(), 48_000.0);
        let wire = WireState {
            start: 0.0,
            free: 1.0e-6,
            compliance: 1.0e-9,
        };
        assert_eq!(bow.step(wire), (0.0, 0.0, 0.0));
        assert_eq!(bow.totals(), BowTotals::default());
    }

    #[test]
    fn a_string_at_rest_under_a_heavy_bow_sticks_and_takes_no_heat() {
        let mut bow = bow();
        bow.set_controls(BowControls {
            velocity: 1.0e-3,
            force: 1.0e4,
        });
        let wire = WireState {
            start: 0.0,
            free: 0.0,
            compliance: 1.0e-6,
        };
        let (force, source, loss) = bow.step(wire);
        assert!(bow.last_frame().sticking);
        assert!((force - 1.0e-3 / 48_000.0 / 1.0e-6).abs() < 1.0e-9);
        assert_eq!(loss, 0.0);
        assert!((source - force * 1.0e-3 / 48_000.0).abs() < 1.0e-18);
    }

    #[test]
    fn a_string_at_rest_under_a_light_bow_slips_on_the_falling_branch() {
        let mut bow = bow();
        let wire = WireState {
            start: 0.0,
            free: 0.0,
            compliance: 1.0e-12,
        };
        let (force, source, loss) = bow.step(wire);
        let frame = bow.last_frame();
        assert!(!frame.sticking);
        assert!(force > 0.3 * 1.0e4 && force < 0.8 * 1.0e4, "{force}");
        assert!(loss > 0.0 && loss <= source);
        // The force sits on the curve at the slip speed it produced.
        let mu = bow.spec().slipping_coefficient(frame.relative_velocity);
        assert!((force - mu * 1.0e4).abs() < 1.0e-6 * 1.0e4);
        // Source minus loss is what the string received.
        assert!((source - loss - force * frame.string_velocity / 48_000.0).abs() < 1.0e-18);
    }

    #[test]
    fn an_up_bow_pulls_the_other_way() {
        let mut bow = bow();
        bow.set_controls(BowControls {
            velocity: -1.0,
            force: 1.0e4,
        });
        let wire = WireState {
            start: 0.0,
            free: 0.0,
            compliance: 1.0e-12,
        };
        let (force, source, loss) = bow.step(wire);
        assert!(force < 0.0);
        assert!(source > 0.0 && loss >= 0.0);
    }
}
