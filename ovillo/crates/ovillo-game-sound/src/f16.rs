//! f16: a low-bypass afterburning turbofan on a flyover, heard from outside.
//! "F-16C, F100-PW-229 class" is the request's label for the scale of thing;
//! nothing here is measured from or fitted to that engine. Every dimension,
//! blade count and gain below is a design choice.
//!
//! LAWFUL PARTS (power-normalised wave networks, exact books):
//! - inlet, core, turbine and tail/nozzle ducts as chains of ducts of
//!   different gas (cold inlet, hot core) joined at plane junctions;
//! - three blade rows (fan, high-pressure compressor, turbine) as series
//!   pressure sources at their planes: `p = A sum_h c_h cos(h Z phi)`, the
//!   periodic loading of Z blades (a Gutin-style dipole), amplitude
//!   `kappa (1/2) rho0 U_tip^2`. Their work `p u` is booked as source;
//! - the reheat flame as a parallel volume-flow source `Q = Qmax tanh(G p(t-tau)/p_sat)`
//!   at a plane of the tail duct: a Rayleigh (in-phase) thermoacoustic
//!   feedback; above a threshold it limit-cycles at the tail pipe's quarter-wave
//!   mode (rumble). Its work `p Q` is booked as source;
//! - two radiating mouths (inlet, nozzle), each parallel R || inertance.
//!
//! DECLARED STOCHASTIC SOURCE TERMS (seeded, listed in the receipt):
//! - jet mixing noise: an `EddyBank` of 28 radiating oscillators, power
//!   `P = eta rho0 U^8 A / c0^5` (Lighthill's eighth-power law; `eta` fitted),
//!   log-normal about the Strouhal peak `0.25 U / D`;
//! - airframe boundary-layer noise: another bank, `P = eta_w rho0 U^6 S / c0^3`,
//!   peaked at `0.2 U / 0.05 m`, driven by `airspeed`;
//! - reheat crackle: Poisson pops of the flame's volume source at a rate
//!   `~ 110 afterburner` per second.
//! These are not derived from the Navier-Stokes equations. They are source
//! terms with declared power laws; the energy they inject is in the ledger.
//!
//! NOT MODELLED: mean flow in the ducts (no convective Doppler or flow
//! refraction inside), bypass stream (one mixed stream), real blade counts or
//! rotor/stator cut-off, supersonic tip "buzz-saw", shock-cell noise, the
//! spool as a stored energy (it is a prescribed speed with a lag; the
//! thermodynamic cycle and shaft energy are outside the ledger, only the
//! acoustic work the blades and flame do on the air is in it), real nozzle
//! geometry changes, the ground, the listener, air absorption, propagation
//! delay and Doppler (the engine does those). Directivity is a declared
//! non-loading readout: it changes what a listener at `aspect` hears of
//! power the ledger has already booked as radiated in all directions.

use crate::net::*;
use crate::GameWorld;
use std::f64::consts::PI;

pub const R_REF: f64 = 50.0; // m: output is pressure at this distance
pub const OUTPUT_SCALE_PA: f64 = 400.0;

const ETA_JET: f64 = 1.0e-4;
const ETA_WIND: f64 = 1.0e-7;
const D_NOZZLE: f64 = 0.56;
const N_RATIO_MIN: f64 = 0.42;

fn aft_angle(aspect: f64) -> f64 {
    PI * (1.0 - clamp(aspect, -1.0, 1.0)) / 2.0
}

/// <D^2> over the sphere for a directivity function of the aft angle.
fn norm_of(f: impl Fn(f64) -> f64) -> f64 {
    let n = 400;
    let mut s = 0.0;
    for i in 0..n {
        let th = (i as f64 + 0.5) / n as f64 * PI;
        s += f(th).powi(2) * th.sin();
    }
    (s * PI / n as f64 / 2.0).sqrt()
}

fn d_jet(th: f64, f: f64) -> f64 {
    let q = 1.0 + 2.2 * (-f / 600.0).exp();
    0.2 + (th - 0.6).cos().max(0.0).powf(q)
}
fn d_tail(th: f64) -> f64 {
    0.10 + th.cos().max(0.0).powi(2)
}
fn d_inlet(th: f64) -> f64 {
    0.07 + (PI - th).cos().max(0.0).powf(2.5)
}

struct Row {
    z: f64,       // blades
    h: usize,     // harmonics kept
    c: [f64; 16], // harmonic weights
}
impl Row {
    fn new(z: f64, f_bpf_max: f64, rolloff: f64) -> Self {
        let h = ((20000.0 / f_bpf_max).floor() as usize).clamp(1, 12);
        let mut c = [0.0; 16];
        for (i, ci) in c.iter_mut().enumerate().take(h) {
            let k = (i + 1) as f64;
            *ci = (-(k / rolloff).powi(2)).exp() / k.sqrt();
        }
        Row { z, h, c }
    }
    #[inline]
    fn p(&self, phi: f64) -> f64 {
        let a = self.z * phi;
        let mut s = 0.0;
        for k in 0..self.h {
            s += self.c[k] * ((k + 1) as f64 * a).cos();
        }
        s
    }
}

pub struct F16 {
    fs: f64,
    dt: f64,
    // controls
    thrust: f64,
    ab_cmd: f64,
    airspeed: f64,
    aspect: f64,
    // slow state (prescribed kinematics with lags)
    n1: f64,
    ab: f64,
    phi1: f64,
    // ducts
    d0: Duct,
    d1: Duct,
    d2: Duct,
    d3a: Duct,
    d3b: Duct,
    m_in: Mouth,
    m_out: Mouth,
    fan: Row,
    hpc: Row,
    trb: Row,
    // reheat
    p_hist: Vec<f64>,
    p_pos: usize,
    pop_left: Vec<(f64, f64)>, // (samples remaining, amplitude m^3/s)
    jet: EddyBank,
    wind: EddyBank,
    rng: Rng,
    ctl_count: usize,
    dir_norm_jet: Vec<f64>,
    dir_tail: f64,
    dir_in: f64,
    // books
    b: Books,
    l_fan: usize,
    l_flame: usize,
    l_pops: usize,
    l_jet_src: usize,
    l_wind_src: usize,
    l_wall: usize,
    l_jet_loss: usize,
    l_wind_loss: usize,
    r_in: usize,
    r_out: usize,
    r_jet: usize,
    r_wind: usize,
    pub p_node_rms: f64,
    pub rumble_p_peak: f64,
    last_jet_w: f64,
}

impl F16 {
    pub fn new(fs: f64, seed: u64) -> Self {
        let dt = 1.0 / fs;
        let rc_cold = RHO0 * C0;
        let tk = |len: f64, c: f64| ((len / (c * dt)).round() as usize).max(1);
        let d0 = Duct::new(&[Section { area: 0.45, ticks: tk(3.0, C0) }], rc_cold, 0.99985);
        let d1 = Duct::new(&[Section { area: 0.20, ticks: tk(0.5, 480.0) }], 0.8 * 480.0, 0.9997);
        let d2 = Duct::new(&[Section { area: 0.22, ticks: tk(0.9, 640.0) }, Section { area: 0.26, ticks: tk(0.6, 760.0) }], 0.45 * 700.0, 0.9996);
        let d3a = Duct::new(&[Section { area: 0.30, ticks: tk(0.6, 800.0) }], 0.30 * 800.0, 0.9997);
        let d3b = Duct::new(&[Section { area: 0.30, ticks: tk(2.4, 800.0) }, Section { area: 0.25, ticks: tk(0.9, 800.0) }], 0.30 * 800.0, 0.9997);
        let m_in = Mouth::new(d0.z_left(), 0.45, RHO0, dt);
        let m_out = Mouth::new(d3b.z_right(), 0.25, 0.30, dt);
        let omega4 = 4.0 * PI;
        let jet = EddyBank::new(28, 30.0, 12000.0, 3.0, 0.5, 1.0, dt, omega4, seed ^ 0x11);
        let wind = EddyBank::new(24, 120.0, 9000.0, 3.0, 0.5, 1.0, dt, omega4, seed ^ 0x22);
        let dir_norm_jet = jet.freqs.iter().map(|&f| norm_of(|th| d_jet(th, f))).collect();
        let mut b = Books::new();
        let l_fan = b.line("blade_rows_and_planes", Kind::Source);
        let l_flame = b.line("reheat_flame_volume_source", Kind::Source);
        let l_pops = b.line("reheat_crackle_stochastic", Kind::Source);
        let l_jet_src = b.line("jet_turbulence_stochastic", Kind::Source);
        let l_wind_src = b.line("airframe_turbulence_stochastic", Kind::Source);
        let l_wall = b.line("duct_wall_loss", Kind::Loss);
        let l_jet_loss = b.line("jet_eddy_decay", Kind::Loss);
        let l_wind_loss = b.line("airframe_eddy_decay", Kind::Loss);
        let r_in = b.line("inlet_mouth", Kind::Radiation);
        let r_out = b.line("nozzle_mouth", Kind::Radiation);
        let r_jet = b.line("jet_eddies", Kind::Radiation);
        let r_wind = b.line("airframe_eddies", Kind::Radiation);
        let mut s = F16 {
            fs,
            dt,
            thrust: 0.0,
            ab_cmd: 0.0,
            airspeed: 0.0,
            aspect: 0.0,
            n1: N_RATIO_MIN,
            ab: 0.0,
            phi1: 0.0,
            d0,
            d1,
            d2,
            d3a,
            d3b,
            m_in,
            m_out,
            fan: Row::new(24.0, 10400.0 / 60.0 * 24.0, 2.4),
            hpc: Row::new(38.0, 14000.0 / 60.0 * 38.0, 2.0),
            trb: Row::new(62.0, 14000.0 / 60.0 * 62.0, 1.8),
            p_hist: vec![0.0; 24],
            p_pos: 0,
            pop_left: vec![],
            jet,
            wind,
            rng: Rng::new(seed),
            ctl_count: 0,
            dir_norm_jet,
            dir_tail: norm_of(d_tail),
            dir_in: norm_of(d_inlet),
            b,
            l_fan,
            l_flame,
            l_pops,
            l_jet_src,
            l_wind_src,
            l_wall,
            l_jet_loss,
            l_wind_loss,
            r_in,
            r_out,
            r_jet,
            r_wind,
            p_node_rms: 0.0,
            rumble_p_peak: 0.0,
            last_jet_w: 0.0,
        };
        s.control_update();
        let e0 = s.stored();
        s.b.init(e0);
        s
    }

    fn stored(&self) -> f64 {
        let dt = self.dt;
        (self.d0.energy_sum() + self.d1.energy_sum() + self.d2.energy_sum() + self.d3a.energy_sum() + self.d3b.energy_sum() + self.m_in.stored() + self.m_out.stored()) * dt
            + self.jet.energy()
            + self.wind.energy()
    }

    pub fn n1(&self) -> f64 {
        self.n1
    }
    pub fn jet_velocity(&self) -> f64 {
        let g = ((self.n1 - N_RATIO_MIN) / (1.0 - N_RATIO_MIN)).clamp(0.0, 1.0).powf(1.3);
        160.0 + 290.0 * g + 180.0 * self.ab
    }
    pub fn fan_bpf(&self) -> f64 {
        self.n1 * 10400.0 / 60.0 * 24.0
    }

    fn control_update(&mut self) {
        let u = self.jet_velocity();
        let a_e = 0.25;
        let p_jet = ETA_JET * RHO0 * u.powi(8) * a_e / C0.powi(5);
        let f_peak = 0.25 * u / D_NOZZLE;
        self.jet.set_power(p_jet, f_peak, 1.4);
        let ua = self.airspeed;
        let p_w = ETA_WIND * RHO0 * ua.powi(6) * 30.0 / C0.powi(3);
        let fw = (0.2 * ua / 0.05).max(150.0);
        self.wind.set_power(p_w, fw, 1.2);
        // directivity weights for the observer's aspect
        let th = aft_angle(self.aspect);
        for i in 0..self.jet.modes.len() {
            self.jet.dir[i] = d_jet(th, self.jet.freqs[i]) / self.dir_norm_jet[i];
        }
        self.last_jet_w = p_jet;
    }

    #[inline]
    fn tick(&mut self) -> f64 {
        let dt = self.dt;
        // spool: a prescribed speed with a lag
        let t_eff = (self.thrust / 0.5).min(1.0).powf(0.9);
        let n_tgt = N_RATIO_MIN + (1.0 - N_RATIO_MIN) * t_eff;
        let tau = if n_tgt > self.n1 { 1.4 } else { 2.2 };
        self.n1 += (n_tgt - self.n1) * dt / tau;
        let ab_tgt = self.ab_cmd * ((self.n1 - 0.95) / 0.05).clamp(0.0, 1.0);
        self.ab += (ab_tgt - self.ab) * dt / 0.25;
        self.ctl_count += 1;
        if self.ctl_count >= 256 {
            self.ctl_count = 0;
            self.control_update();
        }
        let n_rpm = self.n1 * 10400.0;
        let om1 = n_rpm * 2.0 * PI / 60.0;
        self.phi1 += om1 * dt;
        if self.phi1 > 2.0 * PI * 1.0e4 {
            self.phi1 -= 2.0 * PI * 1.0e4;
        }
        let phi2 = self.phi1 * 1.35; // high-pressure spool turns faster
        let u_tip = self.n1 * 430.0;
        let kap = 0.5 * RHO0 * u_tip * u_tip;
        let p_fan = 0.005 * kap * self.fan.p(self.phi1);
        let p_hpc = 0.0025 * kap * self.hpc.p(phi2);
        let p_trb = 0.003 * kap * self.trb.p(phi2 * 0.9);

        let (a0_l, a_d0r) = self.d0.arrive();
        let (a1_l, a1_r) = self.d1.arrive();
        let (a2_l, a2_r) = self.d2.arrive();
        let (a3a_l, a3a_r) = self.d3a.arrive();
        let (a3b_l, a3b_r) = self.d3b.arrive();

        // pressure waves
        let (z0, z1l, z1r, z2l, z2r, z3al, z3ar, z3bl) = (self.d0.z_right(), self.d1.z_left(), self.d1.z_right(), self.d2.z_left(), self.d2.z_right(), self.d3a.z_left(), self.d3a.z_right(), self.d3b.z_left());
        let _ = z3bl;
        let sq = |z: f64| z.sqrt();
        // series source at the fan plane: d0 right <-> d1 left
        let (b0r, b1l, w_fan) = series(a_d0r * sq(z0), a1_l * sq(z1l), z0, z1l, p_fan);
        // hpc plane: d1 right <-> d2 left
        let (b1r, b2l, w_hpc) = series(a1_r * sq(z1r), a2_l * sq(z2l), z1r, z2l, p_hpc);
        // turbine plane: d2 right <-> d3a left
        let (b2r, b3al, w_trb) = series(a2_r * sq(z2r), a3a_l * sq(z3al), z2r, z3al, p_trb);
        // flame plane: d3a right <-> d3b left, parallel volume source
        let z3bl = self.d3b.z_left();
        let p_old = self.p_hist[self.p_pos];
        let q_flame = 0.35 * self.ab * (2.0 * p_old / 2000.0).tanh();
        let mut q_pop = 0.0;
        for pop in self.pop_left.iter_mut() {
            let x = (pop.0 - 8.0) / 5.0;
            q_pop += pop.1 * (-(x * x)).exp();
            pop.0 -= 1.0;
        }
        self.pop_left.retain(|p| p.0 > 0.0);
        if self.ab > 0.02 && self.rng.uniform() < 110.0 * self.ab * dt {
            let amp = 4.0 * (0.4 * self.rng.gauss()).exp();
            self.pop_left.push((16.0, amp));
        }
        let (b3ar, b3bl, w_q, p_node) = parallel(a3a_r * sq(z3ar), a3b_l * sq(z3bl), z3ar, z3bl, q_flame + q_pop);
        self.p_hist[self.p_pos] = p_node;
        self.p_pos = (self.p_pos + 1) % self.p_hist.len();
        let flame_w = q_flame * p_node * dt;
        let pop_w = q_pop * p_node * dt;
        let _ = w_q;

        // mouths
        let b0l = self.m_in.step(a0_l);
        let b_out = self.m_out.step(a3b_r);

        self.d0.depart(b0l, b0r / sq(z0));
        self.d1.depart(b1l / sq(z1l), b1r / sq(z1r));
        self.d2.depart(b2l / sq(z2l), b2r / sq(z2r));
        self.d3a.depart(b3al / sq(z3al), b3ar / sq(z3ar));
        self.d3b.depart(b3bl / sq(z3bl), b_out);

        self.b.add(self.l_fan, (w_fan + w_hpc + w_trb) * dt);
        self.b.add(self.l_flame, flame_w);
        self.b.add(self.l_pops, pop_w);
        self.b.add(self.l_wall, (self.d0.loss + self.d1.loss + self.d2.loss + self.d3a.loss + self.d3b.loss) * dt);
        self.b.add(self.r_in, self.m_in.radiated * dt);
        self.b.add(self.r_out, self.m_out.radiated * dt);

        // turbulence banks
        let p_jet = self.jet.step(R_REF);
        self.b.add(self.l_jet_src, self.jet.src);
        self.b.add(self.l_jet_loss, self.jet.loss);
        self.b.add(self.r_jet, self.jet.rad);
        let p_wind = self.wind.step(R_REF);
        self.b.add(self.l_wind_src, self.wind.src);
        self.b.add(self.l_wind_loss, self.wind.loss);
        self.b.add(self.r_wind, self.wind.rad);

        let st = self.stored();
        self.b.close(st);
        if p_node.abs() > self.rumble_p_peak {
            self.rumble_p_peak = p_node.abs();
        }

        // observed pressure at R_REF: directivity is a non-loading readout
        let th = aft_angle(self.aspect);
        let o_in = far_pressure(self.m_in.w_r_out, 4.0 * PI, R_REF) * d_inlet(th) / self.dir_in;
        let o_out = far_pressure(self.m_out.w_r_out, 4.0 * PI, R_REF) * d_tail(th) / self.dir_tail;
        o_in + o_out + p_jet + p_wind
    }

}

/// Series pressure source `p_src` between ducts of impedance z1 (left) and z2 (right).
/// Inputs and outputs are pressure waves. Returns (wave sent into the left duct, wave sent
/// into the right duct, source work in watts).
fn series(a1: f64, a2: f64, z1: f64, z2: f64, p_src: f64) -> (f64, f64, f64) {
    let u = (p_src + 2.0 * a1 - 2.0 * a2) / (z1 + z2);
    let b1 = a1 - z1 * u;
    let b2 = a2 + z2 * u;
    // power into the ducts = (b1^2-a1^2)/z1 + (b2^2-a2^2)/z2 = p_src u
    (b1, b2, p_src * u)
}

/// Parallel volume source `q` at the plane between z1 (left) and z2 (right).
/// Returns (wave to left, wave to right, work in watts, node pressure).
fn parallel(a1: f64, a2: f64, z1: f64, z2: f64, q: f64) -> (f64, f64, f64, f64) {
    let p = (q + 2.0 * a1 / z1 + 2.0 * a2 / z2) / (1.0 / z1 + 1.0 / z2);
    (p - a1, p - a2, p * q, p)
}

impl GameWorld for F16 {
    fn kind(&self) -> &'static str {
        "f16"
    }
    fn set(&mut self, name: &str, v: f64) -> bool {
        match name {
            "thrust" => self.thrust = clamp(v, 0.0, 1.0),
            "afterburner" => self.ab_cmd = clamp(v, 0.0, 1.0),
            "airspeed" => self.airspeed = clamp(v, 0.0, 300.0),
            "aspect" => self.aspect = clamp(v, -1.0, 1.0),
            _ => return false,
        }
        true
    }
    fn event(&mut self, _name: &str, _arg: f64) -> bool {
        false
    }
    fn render(&mut self, out: &mut [f32]) {
        for o in out.iter_mut() {
            let p = self.tick();
            *o = (p / OUTPUT_SCALE_PA) as f32;
        }
    }
    fn receipt(&self) -> serde_json::Value {
        serde_json::json!({
            "world": "f16",
            "sampleRate": self.fs,
            "ledger": self.b.report(),
            "referenceDistanceM": R_REF,
            "outputScalePa": OUTPUT_SCALE_PA,
            "spoolN1Fraction": self.n1,
            "fanBladePassageHz": self.fan_bpf(),
            "jetVelocityMs": self.jet_velocity(),
            "jetAcousticPowerW": self.last_jet_w,
            "peakFlamePlanePressurePa": self.rumble_p_peak,
            "declaredSources": [
                "fan, compressor and turbine blade rows: series pressure sources (Gutin-style loading harmonics)",
                "reheat flame: parallel volume source with in-phase delayed feedback (Rayleigh)",
            ],
            "stochastic": ["jet mixing noise (eddy bank, Lighthill U^8 power law, fitted eta)", "airframe boundary-layer noise (eddy bank, U^6)", "reheat crackle (Poisson pops)"],
        })
    }
    fn output_scale(&self) -> f64 {
        OUTPUT_SCALE_PA
    }
    fn sample_rate(&self) -> f64 {
        self.fs
    }
}
