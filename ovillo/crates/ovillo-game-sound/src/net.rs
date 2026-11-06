//! The shared parts every game World is built from. Each is a passive,
//! energy-bookkept element: it stores energy in a declared state, loses it
//! through a named dissipation, radiates it through a named port, or is a
//! declared source. Nothing here is a filter or an oscillator that is not a
//! mass on a spring.
//!
//! Units are SI. Waves in ducts are power-normalised, `w = p / sqrt(Z)`, so
//! `w^2` is watts and the energy in flight is `sum(w^2) dt` joules.

use mus_watsn::{WorkFrame, WorkLedger};

pub const RHO0: f64 = 1.204; // ambient air, kg/m^3 (20 C)
pub const C0: f64 = 343.0; // ambient speed of sound, m/s
pub const GAMMA: f64 = 1.4;
pub const P0: f64 = 101_325.0;

// ---------------------------------------------------------------- random

/// A seeded generator. Used ONLY by declared stochastic source terms
/// (jet turbulence, reheat crackle, debris timing); each World lists where.
#[derive(Clone, Debug)]
pub struct Rng(u64, Option<f64>);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03, None)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in [0,1).
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Standard normal (Box-Muller, one value per call).
    pub fn gauss(&mut self) -> f64 {
        if let Some(g) = self.1.take() {
            return g;
        }
        let u1 = self.uniform().max(1e-300);
        let u2 = self.uniform();
        let r = (-2.0 * u1.ln()).sqrt();
        let (s, c) = (std::f64::consts::TAU * u2).sin_cos();
        self.1 = Some(r * s);
        r * c
    }
}

/// A deterministic number in [-1, 1) from two integers (contact-point shapes).
pub fn hash_unit(a: u64, b: u64) -> f64 {
    let mut x = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x >> 11) as f64 / (1u64 << 52) as f64 - 1.0
}

// ---------------------------------------------------------------- books

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Source,
    Control,
    Loss,
    Radiation,
    Output,
}

/// The World's books: a `mus_watsn::WorkLedger` plus named lines under each
/// heading, and the same residuals the existing Worlds report.
#[derive(Clone, Debug)]
pub struct Books {
    pub ledger: WorkLedger,
    lines: Vec<(&'static str, Kind, f64)>,
    frame: Vec<f64>,
    pub energy_now: f64,
    started: bool,
    pub max_stored: f64,
    pub numerical_loss_index: Option<usize>,
}

impl Books {
    pub fn new() -> Self {
        Books {
            ledger: WorkLedger::default(),
            lines: Vec::new(),
            frame: Vec::new(),
            energy_now: 0.0,
            started: false,
            max_stored: 0.0,
            numerical_loss_index: None,
        }
    }
    /// Declare the stored energy the World starts with (not a source).
    pub fn init(&mut self, stored: f64) {
        self.energy_now = stored;
        self.started = true;
    }
    pub fn line(&mut self, name: &'static str, kind: Kind) -> usize {
        self.lines.push((name, kind, 0.0));
        self.frame.push(0.0);
        self.lines.len() - 1
    }
    #[inline]
    pub fn add(&mut self, idx: usize, joules: f64) {
        self.frame[idx] += joules;
    }
    /// Close one frame against the World's independently computed stored energy.
    pub fn close(&mut self, stored_now: f64) {
        if !self.started {
            self.energy_now = 0.0;
            self.started = true;
        }
        let mut f = WorkFrame { energy_start: self.energy_now, energy_end: stored_now, ..Default::default() };
        for (i, v) in self.frame.iter_mut().enumerate() {
            let kind = self.lines[i].1;
            match kind {
                Kind::Source => f.source_work += *v,
                Kind::Control => f.control_work += *v,
                Kind::Loss => f.internal_loss += *v,
                Kind::Radiation => f.radiation_loss += *v,
                Kind::Output => f.output_work += *v,
            }
            self.lines[i].2 += *v;
            *v = 0.0;
        }
        self.ledger.push(f);
        self.energy_now = stored_now;
        if stored_now > self.max_stored {
            self.max_stored = stored_now;
        }
    }
    pub fn report(&self) -> serde_json::Value {
        let mut by = serde_json::Map::new();
        for (n, k, v) in &self.lines {
            by.insert(format!("{:?}:{}", k, n).to_lowercase(), serde_json::json!(*v));
        }
        let l = &self.ledger;
        let throughput = l.source_work + l.control_work.abs() + l.initial_energy.unwrap_or(0.0);
        serde_json::json!({
            "frames": l.frames,
            "sourceWork": l.source_work,
            "controlWork": l.control_work,
            "internalLoss": l.internal_loss,
            "radiation": l.radiation_loss,
            "outputWork": l.output_work,
            "initialEnergy": l.initial_energy,
            "finalEnergy": l.final_energy,
            "maxStoredEnergy": self.max_stored,
            "maximumFrameResidual": l.max_abs_residual,
            "cumulativeClosureResidual": l.cumulative_residual,
            "globalResidual": l.global_residual(),
            "relativeGlobalResidual": if throughput > 0.0 { l.global_residual().abs() / throughput } else { 0.0 },
            "lines": by,
        })
    }
}

impl Default for Books {
    fn default() -> Self {
        Self::new()
    }
}

// ------------------------------------------------------------ delay lines

/// A delay of `n` ticks that remembers the energy it holds.
#[derive(Clone, Debug)]
pub struct Ring {
    buf: Vec<f64>,
    pos: usize,
    energy: f64, // sum of squares in flight
}
impl Ring {
    pub fn new(n: usize) -> Self {
        Ring { buf: vec![0.0; n.max(1)], pos: 0, energy: 0.0 }
    }
    #[inline]
    pub fn out(&self) -> f64 {
        self.buf[self.pos]
    }
    /// Replace the oldest sample (the one `out` returned) with `v`.
    #[inline]
    pub fn push(&mut self, v: f64) {
        let old = self.buf[self.pos];
        self.energy += v * v - old * old;
        self.buf[self.pos] = v;
        self.pos += 1;
        if self.pos == self.buf.len() {
            self.pos = 0;
        }
    }
    pub fn energy_sum(&self) -> f64 {
        self.energy.max(0.0)
    }
    /// Recompute from the samples (kills incremental drift); returns the drift.
    pub fn resync(&mut self) -> f64 {
        let e: f64 = self.buf.iter().map(|x| x * x).sum();
        let d = self.energy - e;
        self.energy = e;
        d
    }
    pub fn len(&self) -> usize {
        self.buf.len()
    }
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

/// One section of a duct: area (m^2), length in ticks, gas impedance.
#[derive(Clone, Copy, Debug)]
pub struct Section {
    pub area: f64,
    pub ticks: usize,
}

/// A chain of uniform sections joined by area steps. Waves are power
/// normalised, so every junction is an orthogonal 2x2 and energy is conserved
/// by construction; wall loss `g` per tick is booked as loss where it acts.
#[derive(Clone, Debug)]
pub struct Duct {
    pub z: Vec<f64>,
    fw: Vec<Ring>,
    bw: Vec<Ring>,
    g: f64,
    jr: Vec<f64>,
    jt: Vec<f64>,
    arr_f: Vec<f64>,
    arr_b: Vec<f64>,
    pub loss: f64, // wall loss accumulated by the last `arrive` (watts, i.e. w^2)
    since_sync: u32,
}

impl Duct {
    /// `rho_c` is the gas's rho*c; `g` is the per-tick amplitude retention.
    pub fn new(sections: &[Section], rho_c: f64, g: f64) -> Self {
        let n = sections.len();
        let z: Vec<f64> = sections.iter().map(|s| rho_c / s.area).collect();
        let mut jr = Vec::new();
        let mut jt = Vec::new();
        for i in 0..n.saturating_sub(1) {
            let r = (z[i + 1] - z[i]) / (z[i + 1] + z[i]);
            jr.push(r);
            jt.push((1.0 - r * r).max(0.0).sqrt());
        }
        Duct {
            z,
            fw: sections.iter().map(|s| Ring::new(s.ticks)).collect(),
            bw: sections.iter().map(|s| Ring::new(s.ticks)).collect(),
            g,
            jr,
            jt,
            arr_f: vec![0.0; n],
            arr_b: vec![0.0; n],
            loss: 0.0,
            since_sync: 0,
        }
    }
    pub fn sections(&self) -> usize {
        self.z.len()
    }
    /// Read what arrives at both ends and at every junction (applying wall loss).
    /// Returns (incoming at left terminal, incoming at right terminal), normalised.
    pub fn arrive(&mut self) -> (f64, f64) {
        let g = self.g;
        let g2 = 1.0 - g * g;
        let mut loss = 0.0;
        for i in 0..self.fw.len() {
            let f = self.fw[i].out();
            let b = self.bw[i].out();
            loss += g2 * (f * f + b * b);
            self.arr_f[i] = f * g;
            self.arr_b[i] = b * g;
        }
        self.loss = loss;
        (self.arr_b[0], self.arr_f[self.fw.len() - 1])
    }
    /// Scatter at the junctions and send `left_out`/`right_out` from the terminals.
    pub fn depart(&mut self, left_out: f64, right_out: f64) {
        let n = self.fw.len();
        self.fw[0].push(left_out);
        self.bw[n - 1].push(right_out);
        for i in 0..n - 1 {
            let al = self.arr_f[i];
            let ar = self.arr_b[i + 1];
            let (r, t) = (self.jr[i], self.jt[i]);
            self.bw[i].push(r * al + t * ar);
            self.fw[i + 1].push(t * al - r * ar);
        }
        self.since_sync += 1;
        if self.since_sync >= 8192 {
            self.since_sync = 0;
            for r in self.fw.iter_mut().chain(self.bw.iter_mut()) {
                r.resync();
            }
        }
    }
    /// Energy in flight (J per unit dt): sum of w^2 over every sample held.
    pub fn energy_sum(&self) -> f64 {
        self.fw.iter().chain(self.bw.iter()).map(|r| r.energy_sum()).sum()
    }
    /// The pressure waves are w * sqrt(Z); handy for readouts (Pa).
    pub fn z_left(&self) -> f64 {
        self.z[0]
    }
    pub fn z_right(&self) -> f64 {
        self.z[self.z.len() - 1]
    }
}

/// An open end: the tube meets, in parallel, the radiation resistance of free
/// space (`R = rho0 c0 / A_mouth`, power leaving through it is radiation) and
/// the inertance of the air plug at the mouth (`M = rho_gas l_end / A`, which
/// stores energy and makes low frequencies reflect). A parallel wave-digital
/// adaptor: orthogonal, so exact.
#[derive(Clone, Debug)]
pub struct Mouth {
    sg0: f64,
    sgr: f64,
    sgl: f64,
    sga: f64,
    inv_sum: f64,
    w_l_in: f64,
    w_l_out: f64,
    pub radiated: f64, // w^2 of the last step
    pub absorbed: f64, // w^2 into the foam, if any
    pub w_r_out: f64,
}
impl Mouth {
    pub fn new(z_tube: f64, area: f64, rho_gas: f64, dt: f64) -> Self {
        Self::with_absorber(z_tube, area, rho_gas, dt, 0.0)
    }
    /// As `new`, plus an absorbing port (a filter element: matched foam) whose
    /// conductance is `absorb` times the tube's own.
    pub fn with_absorber(z_tube: f64, area: f64, rho_gas: f64, dt: f64, absorb: f64) -> Self {
        let radius = (area / std::f64::consts::PI).sqrt();
        let l_end = 0.6133 * radius; // unflanged end correction
        let m = rho_gas * l_end / area;
        let g0 = 1.0 / z_tube;
        let gr = area / (RHO0 * C0);
        let gl = dt / (2.0 * m);
        let ga = absorb * g0;
        Mouth { sg0: g0.sqrt(), sgr: gr.sqrt(), sgl: gl.sqrt(), sga: ga.sqrt(), inv_sum: 1.0 / (g0 + gr + gl + ga), w_l_in: 0.0, w_l_out: 0.0, radiated: 0.0, absorbed: 0.0, w_r_out: 0.0 }
    }
    /// `a` = normalised wave arriving from the tube. Returns the wave sent back.
    pub fn step(&mut self, a: f64) -> f64 {
        let s = self.sg0 * a + self.sgl * self.w_l_in;
        let k = 2.0 * s * self.inv_sum;
        let b = k * self.sg0 - a;
        let wl = k * self.sgl - self.w_l_in;
        let wr = k * self.sgr;
        self.w_l_out = wl;
        self.w_l_in = -wl;
        self.w_r_out = wr;
        self.radiated = wr * wr;
        let wa = k * self.sga;
        self.absorbed = wa * wa;
        b
    }
    pub fn stored(&self) -> f64 {
        self.w_l_out * self.w_l_out
    }
}

/// Pressure at `r` metres from a radiator that sends `p_watts` (instantaneous,
/// signed through `w`) into a solid angle `omega`: `p = w sqrt(rho0 c0 / omega) / r`.
pub fn far_pressure(w: f64, omega: f64, r: f64) -> f64 {
    w * (RHO0 * C0 / omega).sqrt() / r
}

// --------------------------------------------------------------- orifices

/// Solve `K u|u| + beta u = d` for u (K >= 0, beta > 0), stably.
#[inline]
pub fn solve_orifice(k: f64, beta: f64, d: f64) -> f64 {
    let ad = d.abs();
    let m = 2.0 * ad / (beta + (beta * beta + 4.0 * k * ad).sqrt());
    m.copysign(d)
}

/// Loss coefficient of a sharp-edged orifice: dp = K u|u|.
#[inline]
pub fn orifice_k(rho: f64, area: f64, cd: f64) -> f64 {
    if area <= 1e-12 {
        f64::INFINITY
    } else {
        rho / (2.0 * cd * cd * area * area)
    }
}

/// A gas volume as a linear compliance holding charge `q` (m^3): p = q / C,
/// stored energy q^2 / 2C. C = V / (gamma P0).
#[derive(Clone, Copy, Debug)]
pub struct Cap {
    pub q: f64,
    pub c: f64,
}
impl Cap {
    pub fn new(v: f64) -> Self {
        Cap { q: 0.0, c: v / (GAMMA * P0) }
    }
    #[inline]
    pub fn p(&self) -> f64 {
        self.q / self.c
    }
    #[inline]
    pub fn energy(&self) -> f64 {
        0.5 * self.q * self.q / self.c
    }
    /// Change the volume (the piston moves). Returns the work the piston does
    /// ON the gas (negative when the gas expands and gives work out).
    #[inline]
    pub fn set_volume(&mut self, v: f64) -> f64 {
        let e0 = self.energy();
        let v_old = self.c * GAMMA * P0;
        self.q -= v - v_old; // a shrinking volume squeezes charge into the gas
        self.c = v / (GAMMA * P0);
        self.energy() - e0
    }
}

// ------------------------------------------------------------- oscillators

/// A mass on a spring, integrated with the implicit midpoint rule, which keeps
/// `E = m v^2/2 + k x^2/2` exactly: `dE = F vbar dt - c vbar^2 dt`. The spring
/// is pre-warped so the digital frequency is the one asked for.
#[derive(Clone, Debug)]
pub struct Mode {
    pub m: f64,
    pub k: f64,
    pub c_int: f64,
    pub c_rad: f64,
    pub x: f64,
    pub v: f64,
    pub kappa: f64, // sqrt(rho0 c0 c_rad / omega): pressure at 1 m per m/s
}
impl Mode {
    /// `f` Hz, `t60` seconds (amplitude -60 dB), `rad_share` of the damping that radiates.
    pub fn new(m: f64, f: f64, t60: f64, rad_share: f64, dt: f64, omega: f64) -> Self {
        let wa = 2.0 / dt * (std::f64::consts::PI * f * dt).tan();
        let sigma = 6.907_755 / t60;
        let c = 2.0 * m * sigma;
        let c_rad = c * rad_share;
        Mode {
            m,
            k: m * wa * wa,
            c_int: c - c_rad,
            c_rad,
            x: 0.0,
            v: 0.0,
            kappa: (RHO0 * C0 * c_rad / omega).sqrt(),
        }
    }
    pub fn energy(&self) -> f64 {
        0.5 * self.m * self.v * self.v + 0.5 * self.k * self.x * self.x
    }
    /// Step under force `f`. Returns (vbar, internal loss J, radiated J, work in J).
    #[inline]
    pub fn step(&mut self, f: f64, dt: f64) -> (f64, f64, f64, f64) {
        let c = self.c_int + self.c_rad;
        let den = 2.0 * self.m / dt + self.k * dt * 0.5 + c;
        let vbar = (f - self.k * self.x + 2.0 * self.m * self.v / dt) / den;
        self.v = 2.0 * vbar - self.v;
        self.x += dt * vbar;
        (vbar, self.c_int * vbar * vbar * dt, self.c_rad * vbar * vbar * dt, f * vbar * dt)
    }
    /// A mode that is also a wall of a gas cavity (area `a`, cavity compliance
    /// `cap`): the pressure pushes the wall and the wall's motion changes the
    /// cavity's charge. One implicit step, so the exchange is exact.
    /// Returns (vbar, loss, radiated, work taken from the gas).
    pub fn step_wall(&mut self, a: f64, cap: &mut Cap, dt: f64) -> (f64, f64, f64, f64) {
        let c = self.c_int + self.c_rad;
        let p0 = cap.p();
        let den = 2.0 * self.m / dt + self.k * dt * 0.5 + c + a * a * dt / (2.0 * cap.c);
        let vbar = (a * p0 - self.k * self.x + 2.0 * self.m * self.v / dt) / den;
        let e_gas0 = cap.energy();
        cap.q -= a * vbar * dt;
        let taken = e_gas0 - cap.energy();
        self.v = 2.0 * vbar - self.v;
        self.x += dt * vbar;
        (vbar, self.c_int * vbar * vbar * dt, self.c_rad * vbar * vbar * dt, taken)
    }
}

/// A body made of modes, struck through a Hertzian contact by a striker mass.
#[derive(Clone, Debug)]
pub struct Body {
    pub modes: Vec<Mode>,
    pub phi: Vec<f64>,          // mode shape at the current contact point
    pub striker: Option<Striker>,
    pub e_contact: f64,         // energy stored in the contact spring
    pub contacts: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct Striker {
    pub mass: f64,
    pub x: f64, // striker coordinate toward the body; gap delta = x - sum(phi x_n)
    pub v: f64,
    pub kc: f64,   // F = kc delta^1.5
    pub cc: f64,   // contact damping, N s/m per mm of penetration
    pub ks: f64,   // optional spring holding the striker (N/m)
    pub rest: f64, // striker coordinate at which that spring is relaxed
    pub cs: f64,   // damping on the striker itself (N s/m)
    pub f_ext: f64, // an applied force on the striker, N (held for the step)
    pub pinned: bool, // never retire (an armature, not a thrown mass)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BodyStep {
    pub loss: f64,
    pub radiated: f64,
    pub contact_loss: f64,
    pub striker_loss: f64,
    pub applied_work: f64, // work of the striker's applied force
    pub pressure: f64, // sum kappa vbar, Pa at 1 m
    pub force: f64,
}

impl Body {
    pub fn new(modes: Vec<Mode>) -> Self {
        let n = modes.len();
        Body { modes, phi: vec![0.0; n], striker: None, e_contact: 0.0, contacts: 0 }
    }
    pub fn energy(&self) -> f64 {
        let mut e: f64 = self.modes.iter().map(|m| m.energy()).sum();
        if let Some(s) = &self.striker {
            e += 0.5 * s.mass * s.v * s.v + 0.5 * s.ks * (s.x - s.rest).powi(2) + self.e_contact;
        }
        e
    }
    pub fn is_quiet(&self, floor: f64) -> bool {
        self.striker.is_none() && self.energy() < floor
    }
    /// Launch a striker that starts `gap0` metres from touching, moving at `v`
    /// toward the body. `point` picks where on the body it lands (the mode shape).
    /// Returns the energy of any striker it replaces (the caller books it as lost).
    pub fn strike(&mut self, point: u64, mass: f64, v: f64, kc: f64, cc: f64, spread: f64) -> f64 {
        let old = self.striker.map(|s| 0.5 * s.mass * s.v * s.v + 0.5 * s.ks * (s.x - s.rest).powi(2)).unwrap_or(0.0) + self.e_contact;
        for (n, p) in self.phi.iter_mut().enumerate() {
            let h = hash_unit(point, n as u64 + 1);
            // shapes fall off for high modes so a blow excites the low ones most
            *p = h * (1.0 / (1.0 + spread * n as f64));
        }
        self.striker = Some(Striker { mass, x: -1e-6, v, kc, cc, ks: 0.0, rest: 0.0, cs: 0.0, f_ext: 0.0, pinned: false });
        self.e_contact = 0.0;
        self.contacts += 1;
        old
    }
    /// One step with an external force on each mode (zero if None).
    pub fn step(&mut self, dt: f64, ext: Option<(usize, f64)>) -> BodyStep {
        let mut out = BodyStep::default();
        let n = self.modes.len();
        // free solution parts (force-free midpoint velocities) and sensitivities
        let mut fbar = 0.0;
        if let Some(s) = self.striker {
            let den_s = 2.0 * s.mass / dt + s.ks * dt * 0.5 + s.cs;
            let vs_free = (s.f_ext - s.ks * (s.x - s.rest) + 2.0 * s.mass * s.v / dt) / den_s;
            let mut a = vs_free; // free relative approach velocity of the striker
            let mut b = 1.0 / den_s;
            let mut dens = vec![0.0; n];
            let mut base = vec![0.0; n];
            for i in 0..n {
                let m = &self.modes[i];
                let c = m.c_int + m.c_rad;
                let den = 2.0 * m.m / dt + m.k * dt * 0.5 + c;
                dens[i] = den;
                let mut f_ext = 0.0;
                if let Some((j, f)) = ext {
                    if j == i {
                        f_ext = f;
                    }
                }
                base[i] = (f_ext - m.k * m.x + 2.0 * m.m * m.v / dt) / den;
                a -= self.phi[i] * base[i];
                b += self.phi[i] * self.phi[i] / den;
            }
            let delta0 = s.x - self.modes.iter().zip(self.phi.iter()).map(|(m, p)| p * m.x).sum::<f64>();
            let engaged = delta0 > 0.0 || delta0 + dt * a > 0.0;
            if engaged {
                let pot = |d: f64| if d > 0.0 { 0.4 * s.kc * d.powf(2.5) } else { 0.0 };
                let g = |f: f64| {
                    let rel = a - b * f;
                    let d1 = delta0 + dt * rel;
                    let dg = if (d1 - delta0).abs() > 1e-15 { (pot(d1) - pot(delta0)) / (d1 - delta0) } else if d1 > 0.0 { s.kc * d1.powf(1.5) } else { 0.0 };
                    f - dg - s.cc * rel * (0.5 * (delta0 + d1)).max(0.0) * 1.0e3
                };
                // g is increasing in f; bracket then bisect
                let mut hi = 1.0;
                let mut guard = 0;
                while g(hi) < 0.0 && guard < 80 {
                    hi *= 2.0;
                    guard += 1;
                }
                if g(0.0) >= 0.0 {
                    fbar = 0.0;
                } else {
                    let (mut lo, mut h) = (0.0, hi);
                    for _ in 0..60 {
                        let mid = 0.5 * (lo + h);
                        if g(mid) < 0.0 {
                            lo = mid;
                        } else {
                            h = mid;
                        }
                    }
                    fbar = 0.5 * (lo + h);
                }
            }
            // advance striker and contact potential
            let rel = a - b * fbar;
            let d1 = delta0 + dt * rel;
            let vbar_s = vs_free - fbar / den_s;
            out.striker_loss = s.cs * vbar_s * vbar_s * dt;
            out.applied_work = s.f_ext * vbar_s * dt;
            let mut st = s;
            st.v = 2.0 * vbar_s - s.v;
            st.x = s.x + dt * vbar_s;
            let pot = |d: f64| if d > 0.0 { 0.4 * s.kc * d.powf(2.5) } else { 0.0 };
            let mut released = 0.0;
            if fbar > 0.0 || !engaged {
                self.e_contact += pot(d1) - pot(delta0);
            } else {
                // the contact force would have had to pull: the touch is over; its spring energy is dissipated
                released = self.e_contact;
                self.e_contact = 0.0;
            }
            if self.e_contact < 0.0 && self.e_contact > -1e-18 {
                self.e_contact = 0.0;
            }
            let hc = (0.5 * (delta0 + d1)).max(0.0) * 1.0e3;
            out.contact_loss = if fbar > 0.0 { s.cc * hc * rel * rel * dt } else { 0.0 } + released;
            // contact damping force is part of fbar (cc*rel) so its work is booked as loss above
            out.force = fbar;
            for i in 0..n {
                let m = &mut self.modes[i];
                let vbar = base[i] + self.phi[i] * fbar / dens[i];
                m.v = 2.0 * vbar - m.v;
                m.x += dt * vbar;
                out.loss += m.c_int * vbar * vbar * dt;
                out.radiated += m.c_rad * vbar * vbar * dt;
                out.pressure += m.kappa * vbar;
            }
            self.striker = Some(st);
            // retire the striker once it has left the body and is moving away
            let d_now = st.x - self.modes.iter().zip(self.phi.iter()).map(|(m, p)| p * m.x).sum::<f64>();
            if d_now < -2e-3 && st.v < 0.0 {
                // its remaining kinetic energy leaves with it: booked by the caller as a release
                out.contact_loss += 0.0;
            }
        } else {
            for i in 0..n {
                let f = match ext {
                    Some((j, f)) if j == i => f,
                    _ => 0.0,
                };
                let (vbar, l, r, _) = self.modes[i].step(f, dt);
                out.loss += l;
                out.radiated += r;
                out.pressure += self.modes[i].kappa * vbar;
            }
        }
        out
    }
    /// Remove a spent striker; returns the kinetic energy it carries away.
    pub fn release_striker_if_gone(&mut self) -> f64 {
        if let Some(s) = self.striker {
            let d_now = s.x - self.modes.iter().zip(self.phi.iter()).map(|(m, p)| p * m.x).sum::<f64>();
            if !s.pinned && d_now < -1e-3 && s.v < 0.0 {
                self.striker = None;
                let e = 0.5 * s.mass * s.v * s.v + 0.5 * s.ks * (s.x - s.rest).powi(2) + self.e_contact;
                self.e_contact = 0.0;
                return e;
            }
        }
        0.0
    }
}

/// Smooth window 0..1..0 over [a, b] (raised cosine edges of width `w`).
pub fn window(x: f64, a: f64, b: f64, w: f64) -> f64 {
    if x <= a || x >= b {
        return 0.0;
    }
    let up = ((x - a) / w).min(1.0);
    let dn = ((b - x) / w).min(1.0);
    let s = up.min(dn);
    0.5 - 0.5 * (std::f64::consts::PI * s).cos()
}

pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
pub fn clamp(x: f64, a: f64, b: f64) -> f64 {
    x.max(a).min(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_struck_body_closes_its_books() {
        let dt = 1.0 / 48000.0;
        let freqs = [140.0, 390.0, 1200.0, 2600.0];
        let modes: Vec<Mode> = freqs.iter().map(|&f| Mode::new(2.0, f, 0.4, 0.05, dt, 2.0 * std::f64::consts::PI)).collect();
        let mut b = Body::new(modes);
        let _ = b.strike(7, 1.2, 2.0, 3.0e7, 120.0, 0.15);
        let e0 = b.energy();
        let mut loss = 0.0;
        let mut worst: f64 = 0.0;
        let mut prev = e0;
        let mut carried = 0.0;
        for _ in 0..24000 {
            let st = b.step(dt, None);
            loss += st.loss + st.radiated + st.contact_loss;
            let gone = b.release_striker_if_gone();
            carried += gone;
            loss += gone;
            let now = b.energy();
            let step_loss = st.loss + st.radiated + st.contact_loss + gone;
            worst = worst.max((now - prev + step_loss).abs());
            prev = now;
        }
        assert!(worst < 1e-9, "worst frame residual {worst} (carried {carried}, e0 {e0}, loss {loss})");
    }

    #[test]
    fn a_duct_with_a_mouth_closes_its_books() {
        let dt = 1.0 / 48000.0;
        let secs = [Section { area: 6e-4, ticks: 20 }, Section { area: 2e-3, ticks: 3 }, Section { area: 8e-3, ticks: 9 }, Section { area: 2e-4, ticks: 12 }];
        let rc = 0.5 * 520.0;
        let mut d = Duct::new(&secs, rc, 0.999);
        let mut m = Mouth::with_absorber(d.z_right(), 2e-4, 0.5, dt, 0.3);
        let mut rng = Rng::new(3);
        let (mut src, mut loss, mut rad) = (0.0, 0.0, 0.0);
        let mut worst: f64 = 0.0;
        let mut prev = 0.0;
        for n in 0..20000 {
            let (_a_l, a_r) = d.arrive();
            let inj = if n < 400 { rng.gauss() * 3.0 } else { 0.0 };
            // inject a wave at the left terminal: a source that also absorbs what arrives (matched)
            let b_r = m.step(a_r);
            d.depart(inj, b_r);
            // matched left terminal absorbs the arrival; injection is power inj^2
            let e_src = inj * inj * dt;
            let e_abs_left = _a_l * _a_l * dt;
            let e_loss = (d.loss + m.absorbed) * dt;
            let e_rad = m.radiated * dt;
            src += e_src;
            loss += e_loss + e_abs_left;
            rad += e_rad;
            let stored = (d.energy_sum() + m.stored()) * dt;
            worst = worst.max((stored - prev - (e_src - e_loss - e_abs_left - e_rad)).abs());
            prev = stored;
        }
        assert!(worst < 1e-12, "worst {worst}, src {src} loss {loss} rad {rad}");
    }
}


/// A bank of stochastically driven, radiating oscillators: the declared
/// stand-in for turbulence (jet mixing noise, boundary-layer noise).
///
/// THIS IS A STOCHASTIC SOURCE TERM. It is not derived from Navier-Stokes.
/// What is lawful: every mode is a mass on a spring with a declared damping
/// and a declared radiation share, the force that drives it is a source whose
/// work `F vbar dt` is booked, and the radiated power is booked. What is
/// declared (and fitted, not derived): the spectral envelope (log-normal
/// around `f_peak`), the total acoustic power law the caller chooses, and the
/// random drive's seed. The drive variance of mode i is set so its mean
/// radiated power is `P_i`: for a white force of variance s^2 per tick on a
/// mass m, the mean power injected is s^2 dt / (2 m), whatever its frequency.
#[derive(Clone, Debug)]
pub struct EddyBank {
    pub modes: Vec<Mode>,
    pub freqs: Vec<f64>,
    pub sigma: Vec<f64>,
    pub dir: Vec<f64>,
    share: f64,
    dt: f64,
    rng: Rng,
    pub src: f64,
    pub loss: f64,
    pub rad: f64,
}

impl EddyBank {
    pub fn new(n: usize, f_lo: f64, f_hi: f64, q: f64, share: f64, mass: f64, dt: f64, omega: f64, seed: u64) -> Self {
        let mut modes = Vec::new();
        let mut freqs = Vec::new();
        for i in 0..n {
            let f = f_lo * (f_hi / f_lo).powf(i as f64 / (n as f64 - 1.0));
            let t60 = 6.9078 * q / (std::f64::consts::PI * f);
            modes.push(Mode::new(mass, f, t60, share, dt, omega));
            freqs.push(f);
        }
        EddyBank { modes, freqs, sigma: vec![0.0; n], dir: vec![1.0; n], share, dt, rng: Rng::new(seed), src: 0.0, loss: 0.0, rad: 0.0 }
    }
    /// Set the radiated power (watts) and its log-normal envelope.
    pub fn set_power(&mut self, total_w: f64, f_peak: f64, octaves: f64) {
        let w: Vec<f64> = self.freqs.iter().map(|&f| (-0.5 * ((f / f_peak).ln() / (octaves * std::f64::consts::LN_2)).powi(2)).exp()).collect();
        let sum: f64 = w.iter().sum::<f64>().max(1e-30);
        for i in 0..self.modes.len() {
            let p_i = total_w * w[i] / sum;
            self.sigma[i] = (2.0 * self.modes[i].m * p_i / (self.share * self.dt)).sqrt();
        }
    }
    /// One tick. Returns the pressure (Pa at the mode's reference distance, with directivity weights `dir`).
    #[inline]
    pub fn step(&mut self, r: f64) -> f64 {
        let (mut p, mut src, mut loss, mut rad) = (0.0, 0.0, 0.0, 0.0);
        for i in 0..self.modes.len() {
            let s = self.sigma[i];
            let f = if s > 0.0 { s * self.rng.gauss() } else { 0.0 };
            let m = &mut self.modes[i];
            let (vb, l, rd, w) = m.step(f, self.dt);
            p += m.kappa * vb * self.dir[i];
            src += w;
            loss += l;
            rad += rd;
        }
        self.src = src;
        self.loss = loss;
        self.rad = rad;
        p / r
    }
    pub fn energy(&self) -> f64 {
        self.modes.iter().map(|m| m.energy()).sum()
    }
}
