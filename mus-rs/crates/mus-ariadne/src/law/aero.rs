//! Air that is more than linear acoustics, as a proof of concept (POC-3).
//!
//! The tube network's lumped state already carries the mean flow: a steady
//! volume velocity lives in the inertive columns' momenta (the modal
//! displacement coordinates). What linear acoustics lacks is what a jet does.
//! This law adds three things to a tube-network body whose network declares
//! tissue contact (`ContactSpec`, which also makes closure a configuration):
//!
//! 1. **Separation loss** (quasi-steady free jet, discharge coefficient 1, no
//!    pressure recovery) at the jet column, flow aperture `A > 0`:
//!    `dp = rho |u| u / (2 A^2)`, the jet's dynamic pressure, applied by the
//!    implicit midpoint rule on that column's flow:
//!    `ubar = u + kappa dLambda / 2`, `dLambda = -rho |ubar| ubar dt / (2 A^2)`,
//!    `kappa = sum_k F_jk^2 / (L_j^2 omega_k^2)` (how far the retained modes'
//!    flow at `j` moves per unit air momentum). The field loses exactly
//!    `rho |ubar|^3 dt / (2 A^2) >= 0`, booked as the jet's loss.
//! 2. **Turbulence**: a fraction `eta` of that loss returns as sound from a
//!    fluctuating force on the air `obstacle_offset` junctions downstream (a
//!    series pressure source: a dipole; Curle's surface-dipole scaling
//!    `eta ~ M^3` gives the exponent, the constant is a design hypothesis),
//!    switched on with Reynolds number. Its work
//!    `p_s dt (u_o + kappa_o p_s dt / 2)` is exact and is taken from the jet's
//!    loss (`internal loss = loss - work`). The work has a zero-mean cross term
//!    with the column's steady flow, so a per-step cap would bias it; the
//!    second law is kept cumulatively instead: a draw may use this step's
//!    loss plus the credit of loss not yet turned into sound, and a draw larger
//!    than that is halved up to four times, then dropped.
//! 3. **DC-correct wall loss** (`dc_wall_loss`): the per-mode wall loss acts
//!    on the column momenta only, at twice the rate. Each mode decays exactly
//!    as before (`s^2 + 2 sigma s + omega^2`), a steady flow pays a series loss,
//!    and a static pressure behind a closure no longer leaks: the
//!    frequency-independent loss on both coordinates bleeds static pressure
//!    like a shunt conductance, which air in a closed cavity does not do.
//!
//! With every term off (and no contact) the body is linear acoustics,
//! bit for bit.

use serde::{Deserialize, Serialize};

use crate::ir::MODE_COUNT;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AeroSpec {
    /// Multiplier on the separation loss (1 = the law above, 0 = off).
    pub loss_scale: f64,
    /// `eta = min(dipole_efficiency (M / reference_mach)^3, max_efficiency)`
    /// times a smoothstep in Reynolds number from onset to full.
    pub dipole_efficiency: f64,
    pub reference_mach: f64,
    pub max_efficiency: f64,
    pub reynolds_onset: f64,
    pub reynolds_full: f64,
    pub kinematic_viscosity_m2_s: f64,
    pub obstacle_offset: usize,
    pub noise_seed: u64,
    pub dc_wall_loss: bool,
}

impl AeroSpec {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        let finite = [
            self.loss_scale,
            self.dipole_efficiency,
            self.reference_mach,
            self.max_efficiency,
            self.reynolds_onset,
            self.reynolds_full,
            self.kinematic_viscosity_m2_s,
        ];
        if finite.iter().any(|v| !v.is_finite())
            || self.loss_scale < 0.0
            || self.dipole_efficiency < 0.0
            || self.reference_mach <= 0.0
            || !(0.0..=1.0).contains(&self.max_efficiency)
            || self.reynolds_onset < 0.0
            || self.reynolds_full <= self.reynolds_onset
            || self.kinematic_viscosity_m2_s <= 0.0
        {
            return Err("aero coefficients are outside their finite domain");
        }
        Ok(())
    }

    /// True when every flow term is off.
    #[must_use]
    pub fn is_off(&self) -> bool {
        self.loss_scale == 0.0 && self.dipole_efficiency == 0.0 && !self.dc_wall_loss
    }
}

/// One sample of the air, energies in joules.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AeroFrame {
    pub jet_junction: usize,
    /// Flow aperture of the jet column (m²; 0 = closed).
    pub jet_aperture_m2: f64,
    pub jet_closed: bool,
    pub port_closed: bool,
    /// Midpoint volume velocity through the jet column (m³/s).
    pub jet_flow_m3_s: f64,
    pub jet_speed_m_s: f64,
    pub mach: f64,
    pub reynolds: f64,
    pub efficiency: f64,
    /// Separation loss of this step (before the turbulent share).
    pub jet_loss: f64,
    /// Work the turbulent force did on the field (net: its self term plus a
    /// zero-mean cross term with the flow already in its column).
    pub turbulence_work: f64,
    /// The self term `kappa_o (p_s dt)^2 / 2 >= 0`: the fluctuation energy the
    /// source injects (what a readout calls "turbulence paid").
    pub turbulence_injected: f64,
}

/// Cumulative totals.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AeroTotals {
    pub jet_loss: f64,
    pub turbulence_work: f64,
    pub turbulence_injected: f64,
    pub closed_samples: u64,
}

/// One inertive column: flow `u = sqrt(dt) read . q`; a change `dLambda` of
/// its air momentum moves the retained modes by `q += gain dLambda / sqrt(dt)`
/// (`read_k = F_jk / L_j`, `gain_k = read_k / omega_k^2`, the energy-metric
/// projection) and its flow by `kappa dLambda`, `kappa = read . gain`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Column {
    pub read: [f64; MODE_COUNT],
    pub gain: [f64; MODE_COUNT],
    pub kappa: f64,
}

impl Column {
    pub(crate) fn new(
        element: usize,
        flux: &[f64],
        elements: usize,
        inertance: f64,
        omega2: &[f64; MODE_COUNT],
    ) -> Self {
        let mut read = [0.0; MODE_COUNT];
        let mut gain = [0.0; MODE_COUNT];
        let mut kappa = 0.0;
        for k in 0..MODE_COUNT {
            read[k] = flux[k * elements + element] / inertance;
            gain[k] = read[k] / omega2[k];
            kappa += read[k] * gain[k];
        }
        Self { read, gain, kappa }
    }

    #[inline]
    fn flow(&self, q: &[f64; MODE_COUNT], sqrt_dt: f64) -> f64 {
        sqrt_dt * self.read.iter().zip(q).map(|(r, x)| r * x).sum::<f64>()
    }

    #[inline]
    fn push(&self, q: &mut [f64; MODE_COUNT], d_lambda: f64, sqrt_dt: f64) {
        let scale = d_lambda / sqrt_dt;
        for (x, g) in q.iter_mut().zip(&self.gain) {
            *x += g * scale;
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AeroRuntime {
    pub spec: AeroSpec,
    pub density: f64,
    pub speed_of_sound: f64,
    pub jet: Option<Column>,
    pub obstacle: Option<Column>,
    pub jet_junction: usize,
    pub jet_aperture: f64,
    pub port_closed: bool,
    noise: u64,
    /// Jet loss not yet turned into sound (>= 0).
    credit: f64,
    pub last: AeroFrame,
    pub totals: AeroTotals,
}

impl AeroRuntime {
    pub(crate) fn new(spec: AeroSpec, density: f64, speed_of_sound: f64) -> Self {
        Self {
            noise: spec.noise_seed | 1,
            credit: 0.0,
            spec,
            density,
            speed_of_sound,
            jet: None,
            obstacle: None,
            jet_junction: 0,
            jet_aperture: 0.0,
            port_closed: false,
            last: AeroFrame::default(),
            totals: AeroTotals::default(),
        }
    }

    /// Unit-variance deterministic noise (xorshift64*, uniform on +-sqrt 3).
    fn next_noise(&mut self) -> f64 {
        let mut x = self.noise;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.noise = x;
        let unit = (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64;
        (2.0 * unit - 1.0) * 3.0_f64.sqrt()
    }

    /// One sample on the field's column-momentum (displacement) coordinates.
    pub(crate) fn step(&mut self, q: &mut [f64; MODE_COUNT], dt: f64) -> AeroFrame {
        let spec = self.spec;
        let sqrt_dt = dt.sqrt();
        let mut frame = AeroFrame {
            jet_junction: self.jet_junction,
            jet_aperture_m2: self.jet_aperture,
            jet_closed: self.jet_aperture <= 0.0,
            port_closed: self.port_closed,
            ..Default::default()
        };
        if let (Some(jet), true) = (self.jet, self.jet_aperture > 0.0 && spec.loss_scale > 0.0) {
            let area = self.jet_aperture;
            let u = jet.flow(q, sqrt_dt);
            if jet.kappa > 1.0e-300 {
                // ubar + x |ubar| ubar = u, x = scale rho dt kappa / (4 A^2).
                let x = spec.loss_scale * self.density * dt * jet.kappa / (4.0 * area * area);
                let y = x * u.abs();
                let ubar = if y < 1.0e-12 {
                    u * (1.0 - y)
                } else {
                    u.signum() * ((1.0 + 4.0 * y).sqrt() - 1.0) / (2.0 * x)
                };
                let d_lambda = 2.0 * (ubar - u) / jet.kappa;
                jet.push(q, d_lambda, sqrt_dt);
                frame.jet_flow_m3_s = ubar;
                frame.jet_loss = -d_lambda * ubar;
                let speed = ubar.abs() / area;
                let diameter = 2.0 * (area / std::f64::consts::PI).sqrt();
                frame.jet_speed_m_s = speed;
                frame.mach = speed / self.speed_of_sound;
                frame.reynolds = speed * diameter / spec.kinematic_viscosity_m2_s;
                let ramp = ((frame.reynolds - spec.reynolds_onset)
                    / (spec.reynolds_full - spec.reynolds_onset))
                    .clamp(0.0, 1.0);
                let ramp = ramp * ramp * (3.0 - 2.0 * ramp);
                frame.efficiency = (spec.dipole_efficiency
                    * (frame.mach / spec.reference_mach).powi(3))
                .min(spec.max_efficiency)
                    * ramp;
                let budget = frame.efficiency * frame.jet_loss;
                let available = self.credit + frame.jet_loss;
                if let (Some(obstacle), true) = (self.obstacle, budget > 0.0) {
                    if obstacle.kappa > 1.0e-300 {
                        let amplitude = (2.0 * budget / obstacle.kappa).sqrt();
                        let u_o = obstacle.flow(q, sqrt_dt);
                        let mut xi = self.next_noise();
                        for _ in 0..5 {
                            let d = amplitude * xi;
                            let work = d * (u_o + 0.5 * obstacle.kappa * d);
                            if work <= available {
                                obstacle.push(q, d, sqrt_dt);
                                frame.turbulence_work = work;
                                frame.turbulence_injected = 0.5 * obstacle.kappa * d * d;
                                break;
                            }
                            xi *= 0.5;
                        }
                    }
                }
            }
        }
        self.credit = (self.credit + frame.jet_loss - frame.turbulence_work).max(0.0);
        self.totals.jet_loss += frame.jet_loss;
        self.totals.turbulence_work += frame.turbulence_work;
        self.totals.turbulence_injected += frame.turbulence_injected;
        if frame.jet_closed {
            self.totals.closed_samples += 1;
        }
        self.last = frame;
        frame
    }
}
