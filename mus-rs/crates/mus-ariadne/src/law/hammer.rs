//! A felt hammer: a mass thrown against one or more wires, a power-law felt
//! between them (unilateral contact), and the felt's relaxation as its one
//! declared loss.
//!
//! ```text
//! felt compression on wire i   delta_i = y - w_i              (contact while delta_i > 0)
//! elastic force                K delta_i^p,  potential  V(delta) = K delta^(p+1) / (p+1)
//! relaxation                   K alpha d(delta_i^p)/dt        (dissipative)
//! ```
//!
//! This is the hammer of Chabassier, Chaigne and Joly (ESAIM: M2AN 48, 2014,
//! eqs. 4.5-4.8): each wire of a unison has its own compression, and the
//! force is `k_H Phi(e) + r_H d Phi(e)/dt` with `Phi(e) = e^p`. The felt never
//! pulls: the force is clamped at zero.
//!
//! One material step holds each wire's force `F_i` constant over the sample.
//! A modal oscillator under a constant force is solved exactly (a rotation
//! about the shifted equilibrium), so wire `i`'s field changes by exactly
//! `F_i (w_i' - w_i)`, and the hammer, a free mass under `-sum F_i`, by
//! exactly `-sum F_i (y' - y)`. With `F_i` the discrete gradient of `V`
//! between `delta_i` and `delta_i'` plus the relaxation term, the felt's
//! potential changes by the rest, so wires + hammer + felt are conserved to
//! roundoff except for `(F_i - DG_i)(delta_i' - delta_i) >= 0`, booked as
//! internal loss (the felt's). The forces solve a nested monotone scalar
//! problem (each `F_i` given the total, then the total) by safeguarded Newton.
//!
//! The World runs every body's local laws before its couplings, so when the
//! hammer acts the wire has already rotated this sample. The step starts
//! from the inverse rotation of the current state and adds the forced
//! response, `q += f (1 - cos w dt) / w^2`, `m += f sin(w dt) / w` with
//! `f = psi F / sqrt(dt)` (`psi`: the wire's mass-normalized shapes at the
//! strike point, so `w = sqrt(dt) psi . q`).
//!
//! The felt's compression is the hammer's own state, advanced by exactly the
//! displacements above. A law that moves the wire elsewhere in the sample
//! (its loss, the bridge) does not move the felt: while the felt is
//! compressed, the compression and the geometry `y - w_i` drift apart by
//! that splitting (declared). Whenever the felt is free it is re-read from
//! the geometry, which changes no energy.
//!
//! The finger is not modelled. A strike places the hammer `letoff` below the
//! nearest wire with the velocity the action gave it, and books its kinetic
//! energy as source work; gravity over the few millimetres of free flight is
//! ignored. After the rebound the backcheck catches the hammer once it is
//! `catch` below every wire: its remaining kinetic energy is booked as
//! internal loss and it rests (parked, no state) until the next strike.

use serde::{Deserialize, Serialize};

use crate::field::ModalMetricWaveField;

/// Most wires one hammer may strike.
pub const MAX_HAMMER_WIRES: usize = 4;
const NEWTON_STEPS: usize = 60;

/// Where a hammer meets one wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HammerContactSpec {
    /// A `modal_space` body with a stiff-wire map.
    pub body: String,
    /// Strike point, as a fraction of the speaking length from the near end.
    pub strike_fraction: f64,
}

/// Hammer anatomy (a body constitution, `kind: "hammer"`). Every number is
/// the anatomy's to declare and label.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HammerSpec {
    /// Head mass (the shank's rotation is not modelled).
    pub mass_kg: f64,
    /// `K` in `K delta^p` (N / m^p).
    pub felt_stiffness: f64,
    pub felt_exponent: f64,
    /// `alpha` in `K alpha d(delta^p)/dt` (s).
    pub felt_relaxation_s: f64,
    /// Length of wire the felt touches; the strike shape is averaged over it.
    pub contact_width_m: f64,
    /// Free flight between escapement and the wire.
    pub letoff_m: f64,
    /// Rebound distance at which the backcheck catches the hammer.
    pub catch_m: f64,
    pub wires: Vec<HammerContactSpec>,
}

impl HammerSpec {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        let positive = [
            self.mass_kg,
            self.felt_stiffness,
            self.letoff_m,
            self.catch_m,
        ];
        if positive.iter().any(|v| !v.is_finite() || *v <= 0.0)
            || !self.felt_exponent.is_finite()
            || !(1.0..=5.0).contains(&self.felt_exponent)
            || !self.felt_relaxation_s.is_finite()
            || self.felt_relaxation_s < 0.0
            || !self.contact_width_m.is_finite()
            || self.contact_width_m < 0.0
            || self.wires.is_empty()
            || self.wires.len() > MAX_HAMMER_WIRES
            || self.wires.iter().any(|w| {
                !w.strike_fraction.is_finite()
                    || w.strike_fraction <= 0.0
                    || w.strike_fraction >= 0.5
            })
        {
            return Err("hammer needs positive mass, stiffness, letoff and catch, 1 <= p <= 5, \
                        non-negative relaxation and width, and 1..=4 wires struck inside their near half");
        }
        Ok(())
    }
}

/// What one wire offers the hammer this sample.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct WireState {
    /// Strike-point displacement at the start of the step (inverse rotation of the current state).
    pub start: f64,
    /// Strike-point displacement after the free step (the current state).
    pub free: f64,
    /// Displacement per newton of a force held over the step (m/N).
    pub compliance: f64,
}

/// One sample of a hammer.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HammerFrame {
    pub active: bool,
    /// Hammer position (m, along the wires' displacement; the wires rest at 0).
    pub position_m: f64,
    pub velocity_m_s: f64,
    /// Total force on the wires this sample (N).
    pub force_n: f64,
    pub compression_m: [f64; MAX_HAMMER_WIRES],
    pub felt_loss: f64,
}

/// Cumulative energies and counts (J, samples).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HammerTotals {
    pub strikes: usize,
    pub refused_strikes: usize,
    pub source_work: f64,
    pub felt_loss: f64,
    pub catch_loss: f64,
    pub contact_samples: u64,
    /// The last strike: velocity, contact samples from first to last touch,
    /// and peak force.
    pub last_velocity_m_s: f64,
    pub last_contact_samples: u64,
    pub last_peak_force_n: f64,
}

/// One hammer's runtime state.
#[derive(Debug, Clone)]
pub struct FeltHammer {
    spec: HammerSpec,
    dt: f64,
    active: bool,
    y: f64,
    v: f64,
    compression: [f64; MAX_HAMMER_WIRES],
    wires: usize,
    first_contact: Option<u64>,
    last_contact: u64,
    clock: u64,
    totals: HammerTotals,
    last: HammerFrame,
}

impl FeltHammer {
    pub fn new(spec: HammerSpec, sample_rate_hz: f64) -> Self {
        let wires = spec.wires.len();
        Self {
            spec,
            dt: sample_rate_hz.recip(),
            active: false,
            y: 0.0,
            v: 0.0,
            compression: [0.0; MAX_HAMMER_WIRES],
            wires,
            first_contact: None,
            last_contact: 0,
            clock: 0,
            totals: HammerTotals::default(),
            last: HammerFrame::default(),
        }
    }

    #[must_use]
    pub fn spec(&self) -> &HammerSpec {
        &self.spec
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }

    #[must_use]
    pub fn totals(&self) -> HammerTotals {
        self.totals
    }

    #[must_use]
    pub fn last_frame(&self) -> HammerFrame {
        self.last
    }

    /// True while the felt presses on any wire.
    #[must_use]
    pub fn in_contact(&self) -> bool {
        self.active && self.compression[..self.wires].iter().any(|d| *d > 0.0)
    }

    fn potential(&self, delta: f64) -> f64 {
        if delta <= 0.0 {
            return 0.0;
        }
        let p = self.spec.felt_exponent;
        self.spec.felt_stiffness * delta.powf(p + 1.0) / (p + 1.0)
    }

    /// `1/2 M v^2 + sum V(delta_i)`; zero while parked.
    #[must_use]
    pub fn stored_energy(&self) -> f64 {
        if !self.active {
            return 0.0;
        }
        let felt: f64 = self.compression[..self.wires]
            .iter()
            .map(|d| self.potential(*d))
            .sum();
        0.5 * self.spec.mass_kg * self.v * self.v + felt
    }

    /// Throw the hammer at `velocity` from `letoff` below the nearest wire
    /// (`displacement[i]`: wire `i`'s strike-point displacement now). Returns
    /// `(source work, catch loss)`, or `None` (refused, nothing changes)
    /// while the felt still presses on a wire or the velocity is not positive.
    pub fn strike(&mut self, velocity: f64, displacement: &[f64]) -> Option<(f64, f64)> {
        if !velocity.is_finite() || velocity <= 0.0 || self.in_contact() {
            self.totals.refused_strikes += 1;
            return None;
        }
        let caught = self.park();
        let nearest = displacement[..self.wires]
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        self.y = nearest - self.spec.letoff_m;
        self.v = velocity;
        for i in 0..self.wires {
            self.compression[i] = self.y - displacement[i];
        }
        self.active = true;
        self.first_contact = None;
        let work = 0.5 * self.spec.mass_kg * velocity * velocity;
        self.totals.strikes += 1;
        self.totals.source_work += work;
        self.totals.last_velocity_m_s = velocity;
        self.totals.last_contact_samples = 0;
        self.totals.last_peak_force_n = 0.0;
        Some((work, caught))
    }

    /// The backcheck: book the kinetic energy as loss and rest.
    fn park(&mut self) -> f64 {
        if !self.active {
            return 0.0;
        }
        let loss = self.stored_energy();
        self.active = false;
        self.v = 0.0;
        self.compression = [0.0; MAX_HAMMER_WIRES];
        self.totals.catch_loss += loss;
        loss
    }

    /// The elastic discrete gradient plus the clamped relaxation force for a
    /// compression moving `d0 -> d1` over one sample.
    fn force_law(&self, d0: f64, d1: f64) -> f64 {
        let k = self.spec.felt_stiffness;
        let p = self.spec.felt_exponent;
        let gap = d1 - d0;
        let scale = d0.abs().max(d1.abs()).max(1.0e-12);
        let gradient = if gap.abs() > 1.0e-9 * scale {
            (self.potential(d1) - self.potential(d0)) / gap
        } else {
            let mid = 0.5 * (d0 + d1);
            if mid > 0.0 {
                k * mid.powf(p)
            } else {
                0.0
            }
        };
        let phi = |d: f64| if d > 0.0 { d.powf(p) } else { 0.0 };
        let relaxation = k * self.spec.felt_relaxation_s * (phi(d1) - phi(d0)) / self.dt;
        (gradient + relaxation).max(0.0)
    }

    /// One sample. `wires[i]` describes wire `i` now; returns the force on
    /// each wire (N, held over the sample) and the felt's loss (J). The
    /// caller applies each force to its wire with [`apply_force`].
    pub(crate) fn step(
        &mut self,
        wires: &[WireState],
        forces: &mut [f64; MAX_HAMMER_WIRES],
    ) -> f64 {
        self.clock += 1;
        *forces = [0.0; MAX_HAMMER_WIRES];
        if !self.active {
            self.last = HammerFrame::default();
            return 0.0;
        }
        let n = self.wires;
        let dt = self.dt;
        let h = dt * dt / (2.0 * self.spec.mass_kg);
        // Free felt is re-read from the geometry (no energy changes hands).
        for i in 0..n {
            let geometric = self.y - wires[i].start;
            if self.compression[i] <= 0.0 && geometric <= 0.0 {
                self.compression[i] = geometric;
            }
        }
        // delta_i' = a_i - S h - F_i g_i.
        let mut a = [0.0; MAX_HAMMER_WIRES];
        let mut possible = false;
        for i in 0..n {
            a[i] = self.compression[i] + self.v * dt - (wires[i].free - wires[i].start);
            possible |= a[i] > 0.0 || self.compression[i] > 0.0;
        }
        let mut total = 0.0;
        if possible {
            let solve_wire = |i: usize, total: f64| -> f64 {
                let g = wires[i].compliance;
                let d0 = self.compression[i];
                let base = a[i] - total * h;
                let upper = self.force_law(d0, base);
                if upper <= 0.0 {
                    return 0.0;
                }
                // phi(F) = law(base - F g) - F: decreasing, phi(0) > 0, phi(upper) <= 0.
                let (mut lo, mut hi) = (0.0, upper);
                let mut f = upper * 0.5;
                for _ in 0..NEWTON_STEPS {
                    let value = self.force_law(d0, base - f * g) - f;
                    if value > 0.0 {
                        lo = f
                    } else {
                        hi = f
                    }
                    if hi - lo <= 1.0e-14 * upper || value.abs() <= 4.0 * f64::EPSILON * upper {
                        break;
                    }
                    let step = 1.0e-7 * upper.max(f);
                    let slope =
                        (self.force_law(d0, base - (f + step) * g) - (f + step) - value) / step;
                    let next = if slope < 0.0 {
                        f - value / slope
                    } else {
                        f64::NAN
                    };
                    f = if next.is_finite() && next > lo && next < hi {
                        next
                    } else {
                        0.5 * (lo + hi)
                    };
                }
                f.clamp(0.0, upper)
            };
            let sum_at = |total: f64| -> f64 { (0..n).map(|i| solve_wire(i, total)).sum() };
            // G(S) = S - sum F_i(S): increasing, G(0) <= 0, G(sum F_i(0)) >= 0.
            let upper = sum_at(0.0);
            if upper > 0.0 {
                let (mut lo, mut hi) = (0.0, upper);
                let mut s = upper * 0.5;
                for _ in 0..NEWTON_STEPS {
                    let value = s - sum_at(s);
                    if value < 0.0 {
                        lo = s
                    } else {
                        hi = s
                    }
                    if hi - lo <= 1.0e-14 * upper || value.abs() <= 16.0 * f64::EPSILON * upper {
                        break;
                    }
                    let step = 1.0e-7 * upper.max(s);
                    let slope = ((s + step) - sum_at(s + step) - value) / step;
                    let next = if slope > 0.0 {
                        s - value / slope
                    } else {
                        f64::NAN
                    };
                    s = if next.is_finite() && next > lo && next < hi {
                        next
                    } else {
                        0.5 * (lo + hi)
                    };
                }
                for i in 0..n {
                    forces[i] = solve_wire(i, s);
                }
                total = forces[..n].iter().sum();
            }
        }
        // The books use the forces actually applied.
        let dy = self.v * dt - total * h;
        let mut felt_loss = 0.0;
        let mut compression = [0.0; MAX_HAMMER_WIRES];
        for i in 0..n {
            let dw = (wires[i].free - wires[i].start) + forces[i] * wires[i].compliance;
            let d1 = self.compression[i] + dy - dw;
            felt_loss += forces[i] * (d1 - self.compression[i])
                - (self.potential(d1) - self.potential(self.compression[i]));
            compression[i] = d1;
        }
        self.y += dy;
        self.v -= total * dt / self.spec.mass_kg;
        self.compression = compression;
        if total > 0.0 {
            self.first_contact.get_or_insert(self.clock);
            self.last_contact = self.clock;
            self.totals.contact_samples += 1;
            self.totals.last_peak_force_n = self.totals.last_peak_force_n.max(total);
            self.totals.last_contact_samples =
                self.last_contact + 1 - self.first_contact.unwrap_or(self.clock);
        }
        self.totals.felt_loss += felt_loss;
        let mut frame = HammerFrame {
            active: true,
            position_m: self.y,
            velocity_m_s: self.v,
            force_n: total,
            felt_loss,
            ..Default::default()
        };
        frame.compression_m[..n].copy_from_slice(&self.compression[..n]);
        self.last = frame;
        felt_loss
    }

    /// The backcheck, after the step: returns the kinetic energy it took.
    pub(crate) fn catch_if_rebounded(&mut self) -> f64 {
        if self.active
            && self.v < 0.0
            && self.compression[..self.wires]
                .iter()
                .all(|d| *d < -self.spec.catch_m)
        {
            return self.park();
        }
        0.0
    }
}

/// What wire `shape` (mass-normalized shapes at the strike point) offers a
/// hammer this sample: its displacement at the start of the step, after the
/// free step, and its compliance to a force held over the step.
pub(crate) fn wire_state<const N: usize>(
    field: &ModalMetricWaveField<N>,
    shape: &[f64; N],
    sqrt_dt: f64,
) -> WireState {
    let (omega, cosine, sine) = field.rotation();
    let q = field.displacement();
    let m = field.momentum();
    let mut state = WireState::default();
    for k in 0..N {
        let c = shape[k];
        if c == 0.0 {
            continue;
        }
        let start = cosine[k] * q[k] - sine[k] * m[k] / omega[k];
        state.start += c * start;
        state.free += c * q[k];
        state.compliance += c * c * (1.0 - cosine[k]) / (omega[k] * omega[k]);
    }
    state.start *= sqrt_dt;
    state.free *= sqrt_dt;
    state
}

/// Add the forced response of a force `force` (N) held over the step at the
/// strike point to the wire's field.
pub(crate) fn apply_force<const N: usize>(
    field: &mut ModalMetricWaveField<N>,
    shape: &[f64; N],
    force: f64,
    sqrt_dt: f64,
) {
    if force == 0.0 {
        return;
    }
    let (omega, cosine, sine) = field.rotation();
    let mut dq = [0.0; N];
    let mut dm = [0.0; N];
    for k in 0..N {
        let f = shape[k] * force / sqrt_dt;
        if f == 0.0 {
            continue;
        }
        dq[k] = f * (1.0 - cosine[k]) / (omega[k] * omega[k]);
        dm[k] = f * sine[k] / omega[k];
    }
    let q = field.displacement_mut();
    for k in 0..N {
        q[k] += dq[k];
    }
    let m = field.momentum_mut();
    for k in 0..N {
        m[k] += dm[k];
    }
}
