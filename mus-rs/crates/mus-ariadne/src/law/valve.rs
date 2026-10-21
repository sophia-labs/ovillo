//! A self-oscillating flow valve: a mass on a tension spring with unilateral
//! contact, opening a slit through which a reservoir-paid jet enters a modal
//! field at one volume.
//!
//! The mechanics, jet law and intraglottal-pressure form are copied from
//! `ovillo/crates/ovillo-tract/src/world.rs` (`TractWorld::glottis`,
//! `TractWorld::fold_update`, `retune_fold`), whose numbers that crate labels
//! design hypotheses; what is new here is the port to a *modal* field:
//!
//! ```text
//! node pressure      p = sqrt(dt) sum_k phi_k m_k          (phi: mode shapes at the node)
//! source              m_k += sqrt(dt) phi_k U              (a volume velocity U into the node)
//! midpoint pressure  pbar = p_old + Z U,   Z = dt |phi|^2 / 2
//! jet                ps - pbar = R_v U + rho U|U| / (2 a^2),  R_v = 12 mu d l^2 / a^3
//! ```
//!
//! The field gains exactly `pbar U dt` (the change of `1/2 dt |m|^2` along the
//! injected direction is `dt dP (P + dP/2)`, which is `pbar U dt`); the jet
//! dissipates `(ps - pbar) U dt >= 0` (its sign is `U`'s); the reservoir pays
//! `ps U dt`. The fold is advanced by the implicit midpoint rule with a
//! discrete-gradient contact potential, so its energy changes by exactly
//! `F dy - r dy^2 / dt`. As in `ovillo-tract`, the aerodynamic work `F dy` on
//! the fold is booked to the reservoir: its path through the glottis is not
//! resolved (a declared approximation, not a law).

use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

const NEWTON_STEPS: usize = 8;

/// Valve anatomy. Every number is the anatomy's to declare and label.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowValveSpec {
    /// Tube and section of the network volume the jet enters.
    pub tube: usize,
    pub section: usize,
    /// Vibrating length and medial thickness of the slit.
    pub length_m: f64,
    pub thickness_m: f64,
    /// Effective vibrating mass.
    pub mass_kg: f64,
    pub damping_ratio: f64,
    /// Lumped mucosal-wave delay between the entry and exit edges.
    pub mucosal_delay_s: f64,
    pub contact_stiffness_ratio: f64,
    pub contact_damping_ratio: f64,
    /// Resting half-gap at adduction 0 and 1.
    pub rest_half_gap_open_m: f64,
    pub rest_half_gap_closed_m: f64,
    pub min_tuning_hz: f64,
    pub max_tuning_hz: f64,
    pub initial_tuning_hz: f64,
    pub initial_adduction: f64,
    /// Supply pressure at `supply = 1` with a full reservoir.
    pub max_supply_pressure_pa: f64,
    /// Below this fraction of the reservoir's capacity the pressure sags.
    pub sag_fraction: f64,
    pub air_viscosity_pa_s: f64,
}

impl FlowValveSpec {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        let positive = [
            self.length_m,
            self.thickness_m,
            self.mass_kg,
            self.rest_half_gap_open_m,
            self.rest_half_gap_closed_m,
            self.min_tuning_hz,
            self.max_tuning_hz,
            self.max_supply_pressure_pa,
            self.sag_fraction,
            self.air_viscosity_pa_s,
        ];
        let non_negative = [
            self.damping_ratio,
            self.mucosal_delay_s,
            self.contact_stiffness_ratio,
            self.contact_damping_ratio,
        ];
        if positive.iter().any(|v| !v.is_finite() || *v <= 0.0)
            || non_negative.iter().any(|v| !v.is_finite() || *v < 0.0)
            || self.min_tuning_hz >= self.max_tuning_hz
            || !(self.min_tuning_hz..=self.max_tuning_hz).contains(&self.initial_tuning_hz)
            || !(0.0..=1.0).contains(&self.initial_adduction)
            || self.sag_fraction > 1.0
        {
            return Err("flow valve coefficients are outside their finite domain");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlowValveControls {
    /// Fraction of the maximum supply pressure, 0..1.
    pub supply: f64,
}

impl Default for FlowValveControls {
    fn default() -> Self {
        Self { supply: 0.0 }
    }
}

/// One sample of the valve, all energies in joules.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FlowValveFrame {
    pub supply_pressure_pa: f64,
    pub flow_m3_s: f64,
    /// Midpoint pressure in the receiving volume.
    pub node_pressure_pa: f64,
    pub intraglottal_pressure_pa: f64,
    pub opening_m2: f64,
    /// `pbar U dt`: acoustic work into the field.
    pub field_work: f64,
    /// `(ps - pbar) U dt`.
    pub jet_loss: f64,
    pub fold_loss: f64,
    /// `F dy` on the fold.
    pub aero_work: f64,
    /// `ps U dt + F dy`, taken from the reservoir.
    pub reservoir_draw: f64,
    /// Mean jet speed through the narrowest opening (m/s), for observers.
    pub jet_speed_m_s: f64,
}

#[derive(Debug, Clone)]
pub struct FlowValve {
    spec: FlowValveSpec,
    dt: f64,
    density: f64,
    y: f64,
    v: f64,
    stiffness: f64,
    rest: f64,
    contact_stiffness: f64,
    damping: f64,
    contact_damping: f64,
    tuning_hz: f64,
    adduction: f64,
}

impl FlowValve {
    pub fn try_new(
        sample_rate_hz: f64,
        density: f64,
        spec: FlowValveSpec,
    ) -> Result<Self, &'static str> {
        spec.validate()?;
        if !density.is_finite()
            || density <= 0.0
            || !sample_rate_hz.is_finite()
            || sample_rate_hz <= 0.0
        {
            return Err("flow valve needs a positive density and clock");
        }
        let mut valve = Self {
            dt: sample_rate_hz.recip(),
            density,
            y: 0.0,
            v: 0.0,
            stiffness: 0.0,
            rest: 0.0,
            contact_stiffness: 0.0,
            damping: 0.0,
            contact_damping: 0.0,
            tuning_hz: spec.initial_tuning_hz,
            adduction: spec.initial_adduction,
            spec,
        };
        valve.apply_tuning();
        valve.y = valve.rest;
        Ok(valve)
    }

    #[must_use]
    pub fn spec(&self) -> &FlowValveSpec {
        &self.spec
    }

    #[must_use]
    pub fn half_gap_m(&self) -> f64 {
        self.y
    }

    #[must_use]
    pub fn velocity_m_s(&self) -> f64 {
        self.v
    }

    #[must_use]
    pub fn tuning(&self) -> (f64, f64) {
        (self.tuning_hz, self.adduction)
    }

    /// `1/2 m v^2 + 1/2 k (y - rest)^2 + 1/2 k_c min(y, 0)^2`.
    #[must_use]
    pub fn stored_energy(&self) -> f64 {
        let stretch = self.y - self.rest;
        let pen = self.y.min(0.0);
        0.5 * self.spec.mass_kg * self.v * self.v
            + 0.5 * self.stiffness * stretch * stretch
            + 0.5 * self.contact_stiffness * pen * pen
    }

    fn apply_tuning(&mut self) {
        let m = self.spec.mass_kg;
        let omega = TAU * self.tuning_hz;
        self.stiffness = m * omega * omega;
        self.contact_stiffness = self.spec.contact_stiffness_ratio * self.stiffness;
        self.damping = 2.0 * self.spec.damping_ratio * m * omega;
        self.contact_damping = 2.0 * self.spec.contact_damping_ratio * m * omega;
        self.rest = self.spec.rest_half_gap_open_m
            * (self.spec.rest_half_gap_closed_m / self.spec.rest_half_gap_open_m)
                .powf(self.adduction);
    }

    /// Retune tension and adduction at fixed fold state; returns the control
    /// work (the change of stored energy).
    pub fn retune(&mut self, tuning_hz: f64, adduction: f64) -> f64 {
        let tuning = if tuning_hz.is_finite() {
            tuning_hz
        } else {
            self.tuning_hz
        };
        let tuning = tuning.clamp(self.spec.min_tuning_hz, self.spec.max_tuning_hz);
        let adduction = if adduction.is_finite() {
            adduction.clamp(0.0, 1.0)
        } else {
            self.adduction
        };
        if tuning.to_bits() == self.tuning_hz.to_bits()
            && adduction.to_bits() == self.adduction.to_bits()
        {
            return 0.0;
        }
        let before = self.stored_energy();
        self.tuning_hz = tuning;
        self.adduction = adduction;
        self.apply_tuning();
        self.stored_energy() - before
    }

    /// Supply pressure the reservoir delivers at `supply` (sags when low).
    #[must_use]
    pub fn supply_pressure(&self, supply: f64, reservoir: f64, capacity: f64) -> f64 {
        let supply = if supply.is_finite() {
            supply.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let sag = (reservoir / (self.spec.sag_fraction * capacity)).clamp(0.0, 1.0);
        supply * self.spec.max_supply_pressure_pa * sag
    }

    /// One sample. `shape[k] = phi_k(node)`; `momentum` is the field's.
    pub fn process<const N: usize>(
        &mut self,
        momentum: &mut [f64; N],
        shape: &[f64; N],
        controls: FlowValveControls,
        reservoir: &mut f64,
        capacity: f64,
    ) -> FlowValveFrame {
        let dt = self.dt;
        let sqrt_dt = dt.sqrt();
        let ps = self.supply_pressure(controls.supply, *reservoir, capacity);
        let mut shape_norm2 = 0.0;
        let mut projection = 0.0;
        for k in 0..N {
            shape_norm2 += shape[k] * shape[k];
            projection += shape[k] * momentum[k];
        }
        let p_old = sqrt_dt * projection;
        let z_field = 0.5 * dt * shape_norm2;
        let s = &self.spec;
        let xi1 = self.y + s.mucosal_delay_s * self.v;
        let xi2 = self.y - s.mucosal_delay_s * self.v;
        let a1 = 2.0 * s.length_m * xi1.max(0.0);
        let a2 = 2.0 * s.length_m * xi2.max(0.0);
        let amin = a1.min(a2);
        let delta = ps - p_old;
        let u = if amin <= 1.0e-14 {
            0.0
        } else {
            let kappa = self.density / (2.0 * amin * amin);
            let viscous = 12.0 * s.air_viscosity_pa_s * s.thickness_m * s.length_m * s.length_m
                / (amin * amin * amin);
            let z = z_field + viscous;
            let root = (z * z + 4.0 * kappa * delta.abs()).sqrt();
            2.0 * delta / (z + root)
        };
        let step = sqrt_dt * u;
        for k in 0..N {
            momentum[k] += step * shape[k];
        }
        let p_bar = p_old + z_field * u;
        let field_work = p_bar * u * dt;
        let jet_loss = (ps - p_bar) * u * dt;
        let pg = if a1 > 0.0 && a2 > 0.0 {
            p_bar + (ps - p_bar) * (1.0 - a2 / a1).max(-1.0)
        } else if a1 > 0.0 {
            ps
        } else {
            p_bar
        };
        let force = s.length_m * s.thickness_m * pg;
        let (dy, fold_loss) = self.fold_update(force);
        let aero_work = force * dy;
        let reservoir_draw = ps * u * dt + aero_work;
        *reservoir -= reservoir_draw;
        FlowValveFrame {
            supply_pressure_pa: ps,
            flow_m3_s: u,
            node_pressure_pa: p_bar,
            intraglottal_pressure_pa: pg,
            opening_m2: amin,
            field_work,
            jet_loss,
            fold_loss,
            aero_work,
            reservoir_draw,
            jet_speed_m_s: if amin > 1.0e-14 { u / amin } else { 0.0 },
        }
    }

    /// Implicit midpoint with a discrete-gradient contact (copied from
    /// `ovillo-tract` `fold_update`); returns `(dy, dissipated energy)`.
    fn fold_update(&mut self, force: f64) -> (f64, f64) {
        let m = self.spec.mass_kg;
        let dt = self.dt;
        let (y0, v0) = (self.y, self.v);
        let r = self.damping + if y0 < 0.0 { self.contact_damping } else { 0.0 };
        let a = 2.0 * m / (dt * dt) + r / dt + 0.5 * self.stiffness;
        let rhs = force + 2.0 * m * v0 / dt - self.stiffness * (y0 - self.rest);
        let kc = self.contact_stiffness;
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
        self.v = 2.0 * d / dt - v0;
        self.y = y0 + d;
        (d, r * d * d / dt)
    }
}
