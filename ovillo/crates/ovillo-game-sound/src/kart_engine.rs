//! kart_engine: a ~100 cc single-cylinder two-stroke, outdoors.
//!
//! What is in the World (all one energy ledger):
//! - a cylinder and a crankcase, each a gas compliance whose volume the piston
//!   changes (slider-crank geometry); the piston's work on the gas is exact;
//! - a flywheel: kinetic energy `J w^2 / 2`; `rpm` is a prescribed speed held
//!   by a servo whose work is booked as control work (negative when `load`
//!   is engine braking). After `hit` the servo lets go and the engine spins down
//!   under bearing friction and its own pumping;
//! - a carburettor throat (variable orifice, `throttle`), a reed (check valve),
//!   an optional blower pressure in series (`boost`), an intake duct that
//!   radiates at its mouth;
//! - piston-controlled transfer and exhaust ports (areas from piston position),
//!   Bernoulli orifice losses (closed-form implicit solution);
//! - combustion: a declared heat-release source (Wiebe burn over a crank
//!   interval) proportional to the fresh charge trapped at ignition; nothing
//!   burns if the charge is too weak;
//! - an exhaust: header, diffuser cone, belly, baffle cone, stinger, silencer
//!   can and tail pipe, as a power-normalised wave network with area steps,
//!   wall loss, and a radiating mouth (R || inertance);
//! - four head/barrel shell modes, each a wall of the cylinder gas (reciprocal);
//! - a chassis (8 struck modes) for `bump`, `land`, `hit` through a Hertz
//!   contact; five tread-block stick-slip oscillators for `skid`.
//!
//! What it does not model: real thermodynamics (gas is a linear compliance
//! charged by a fitted burn law; no temperature, no mean flow in ducts, no
//! convective Doppler), crank/rod inertia beyond a flywheel, valve or gear
//! noise, a clutch or drivetrain, the tyre/ground in rolling, the ground,
//! the listener or the air path (the engine adds those). Heat release, port
//! timing, areas, volumes and modal numbers are design choices, not measured
//! from any engine.

use crate::net::*;
use crate::GameWorld;
use std::f64::consts::{PI, TAU};

pub const OUTPUT_SCALE_PA: f64 = 60.0; // 1 m pressure that maps to full scale

const BORE: f64 = 0.050;
const STROKE: f64 = 0.050;
const ROD: f64 = 0.100;
const CR: f64 = 14.0;
const V_K0: f64 = 250e-6;
const X_EX: f64 = 0.0295; // exhaust port top edge, m below TDC
const X_TR: f64 = 0.0345;
const W_EX: f64 = 0.032;
const W_TR: f64 = 0.026;
const CD_PORT: f64 = 0.65;
const C_GAS_HOT: f64 = 520.0;
const RHO_HOT: f64 = 0.50;
const J_FLY: f64 = 0.004;
const C_FRIC: f64 = 4.0e-4; // N m s: friction torque C w, power C w^2
const C_FRIC_DEAD: f64 = 3.0e-3; // a blown engine drags (bent parts, no oil)
const THETA_IGN: f64 = -12.0 * PI / 180.0;
const BURN_SPAN: f64 = 45.0 * PI / 180.0;
const DP_BURN: f64 = 1.5e6; // gauge pressure rise at TDC volume for a full charge
const V_FRESH_REF: f64 = 60e-6; // fresh charge delivered per cycle that counts as full
const XI_MIN: f64 = 0.22;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Run {
    Running,
    Cranking,
    Dead,
}

struct Tread {
    m: f64,
    k: f64,
    c: f64,
    c_rad: f64,
    n: f64,
    x: f64,
    v: f64,
    kappa: f64,
}

pub struct KartEngine {
    fs: f64,
    dt: f64,
    // controls
    rpm: f64,
    throttle: f64,
    load: f64,
    boost: f64,
    skid: f64,
    lakebed: bool,
    // crank
    theta: f64,
    omega: f64,
    run: Run,
    run_t: f64,
    area_p: f64,
    // gas
    cyl: Cap,
    crank: Cap,
    // burn
    burn_x: f64,
    burn_dq: f64,
    burning: bool,
    prev_theta: f64,
    // ducts
    ex: Duct,
    ex_mouth: Mouth,
    inl: Duct,
    in_mouth: Mouth,
    // shell
    shell: Vec<Mode>,
    chassis: Body,
    tread: Vec<Tread>,
    rng: Rng,
    l_rough: usize,
    // books
    b: Books,
    l_comb: usize,
    l_servo: usize,
    l_rope: usize,
    l_blower: usize,
    l_wall: usize,
    l_foam: usize,
    l_orifice: usize,
    l_numeric: usize,
    l_fric: usize,
    l_shell: usize,
    l_chassis: usize,
    l_contact: usize,
    l_tread_fric: usize,
    l_tread_src: usize,
    l_tread_loss: usize,
    l_striker: usize,
    r_exh: usize,
    r_int: usize,
    r_shell: usize,
    r_chassis: usize,
    r_tread: usize,
    // counters
    fires: u64,
    pending_impact: f64,
    pending_lost: f64,
    l_impact: usize,
    fresh: f64,
    pub fresh_last: f64,
    pub xi_last: f64,
    misfires: u64,
    revs: u64,
    last_out: f64,
    pub peak_cyl: f64,
    pub max_numeric_frac: f64,
}

fn piston_x(theta: f64) -> f64 {
    let r = STROKE / 2.0;
    r * (1.0 - theta.cos()) + ROD - (ROD * ROD - r * r * theta.sin().powi(2)).sqrt()
}

impl KartEngine {
    pub fn new(fs: f64) -> Self {
        Self::with_seed(fs, 1)
    }
    pub fn with_seed(fs: f64, seed: u64) -> Self {
        let dt = 1.0 / fs;
        let area_p = PI * BORE * BORE / 4.0;
        let v_d = area_p * STROKE;
        let v_c = v_d / (CR - 1.0);
        // exhaust: lengths in ticks of 520 m/s
        let tick_h = C_GAS_HOT * dt;
        let rc_hot = RHO_HOT * C_GAS_HOT;
        let a_of = |d: f64| PI * d * d / 4.0;
        let tk = |len: f64| ((len / tick_h).round() as usize).max(1);
        let mut s: Vec<Section> = Vec::new();
        s.push(Section { area: a_of(0.028), ticks: tk(0.22) });
        let n_diff = 14;
        for i in 0..n_diff {
            let t = (i as f64 + 0.5) / n_diff as f64;
            s.push(Section { area: a_of(lerp(0.028, 0.080, t)), ticks: 1 });
        }
        s.push(Section { area: a_of(0.080), ticks: tk(0.12) });
        let n_base = 8;
        for i in 0..n_base {
            let t = (i as f64 + 0.5) / n_base as f64;
            s.push(Section { area: a_of(lerp(0.080, 0.022, t)), ticks: 1 });
        }
        s.push(Section { area: a_of(0.022), ticks: tk(0.22) });
        s.push(Section { area: a_of(0.067), ticks: tk(0.15) }); // silencer can
        let tail_d = 0.020;
        s.push(Section { area: a_of(tail_d), ticks: tk(0.05) });
        let ex = Duct::new(&s, rc_hot, 0.9994);
        let ex_mouth = Mouth::new(ex.z_right(), a_of(tail_d), RHO_HOT, dt);

        let tick_c = C0 * dt;
        let in_d = 0.026;
        let inl = Duct::new(&[Section { area: a_of(in_d), ticks: ((0.18 / tick_c).round() as usize).max(1) }], RHO0 * C0, 0.9996);
        let in_mouth = Mouth::with_absorber(inl.z_right(), a_of(in_d), RHO0, dt, 25.0);

        let omega_sphere = 2.0 * PI; // ground-level, hard flat floor
        let shell_f = [1650.0, 2900.0, 4200.0, 6100.0];
        let shell_t = [0.06, 0.05, 0.04, 0.03];
        let shell: Vec<Mode> = shell_f.iter().zip(shell_t.iter()).map(|(&f, &t)| Mode::new(0.15, f, t, 0.05, dt, omega_sphere)).collect();

        let ch_f = [142.0, 231.0, 387.0, 560.0, 842.0, 1260.0, 1890.0, 2730.0];
        let ch_t = [0.5, 0.4, 0.35, 0.3, 0.25, 0.2, 0.15, 0.1];
        let ch_modes: Vec<Mode> = ch_f.iter().zip(ch_t.iter()).map(|(&f, &t)| Mode::new(2.0, f, t, 0.05, dt, omega_sphere)).collect();

        let mut b = Books::new();
        let l_comb = b.line("combustion_heat_release", Kind::Source);
        let l_servo = b.line("crank_servo_or_load", Kind::Control);
        let l_impact = b.line("impact_thrown_striker", Kind::Control);
        let l_rope = b.line("pull_rope", Kind::Control);
        let l_blower = b.line("boost_blower", Kind::Source);
        let l_wall = b.line("duct_wall_loss", Kind::Loss);
        let l_foam = b.line("intake_filter_foam", Kind::Loss);
        let l_orifice = b.line("port_and_throat_jets", Kind::Loss);
        let l_numeric = b.line("backward_euler_dissipation", Kind::Loss);
        let l_fric = b.line("bearing_friction", Kind::Loss);
        let l_shell = b.line("shell_damping", Kind::Loss);
        let l_chassis = b.line("chassis_damping", Kind::Loss);
        let l_contact = b.line("contact_damping", Kind::Loss);
        let l_tread_fric = b.line("tread_friction", Kind::Loss);
        let l_tread_src = b.line("tyre_slip_work", Kind::Source);
        let l_rough = b.line("ground_roughness_stochastic", Kind::Source);
        let l_tread_loss = b.line("tread_damping", Kind::Loss);
        let l_striker = b.line("striker_carries_away", Kind::Loss);
        let r_exh = b.line("exhaust_mouth", Kind::Radiation);
        let r_int = b.line("intake_mouth", Kind::Radiation);
        let r_shell = b.line("engine_shell", Kind::Radiation);
        let r_chassis = b.line("chassis", Kind::Radiation);
        let r_tread = b.line("tyre", Kind::Radiation);

        let tf = [620.0, 880.0, 1150.0, 1420.0, 1750.0];
        let tread: Vec<Tread> = tf
            .iter()
            .map(|&f| {
                let m = 0.02;
                let wa = 2.0 / dt * (PI * f * dt).tan();
                let c = 2.0 * m * 6.9078 / 0.4;
                Tread { m, k: m * wa * wa, c, c_rad: 0.001 * c, n: 60.0, x: 0.0, v: 0.0, kappa: (RHO0 * C0 * 0.001 * c / omega_sphere).sqrt() }
            })
            .collect();

        let mut e = KartEngine {
            fs,
            dt,
            rpm: 1800.0,
            throttle: 0.0,
            load: 1.0,
            boost: 0.0,
            skid: 0.0,
            lakebed: false,
            theta: 0.0,
            omega: 1800.0 * TAU / 60.0,
            run: Run::Running,
            run_t: 0.0,
            area_p,
            cyl: Cap::new(v_c + area_p * piston_x(0.0)),
            crank: Cap::new(V_K0 + area_p * (STROKE - piston_x(0.0))),
            burn_x: 0.0,
            burn_dq: 0.0,
            burning: false,
            prev_theta: 0.0,
            ex,
            ex_mouth,
            inl,
            in_mouth,
            shell,
            chassis: Body::new(ch_modes),
            tread,
            rng: Rng::new(seed),
            l_rough,
            b,
            l_comb,
            l_servo,
            l_rope,
            l_blower,
            l_wall,
            l_foam,
            l_orifice,
            l_numeric,
            l_fric,
            l_shell,
            l_chassis,
            l_contact,
            l_tread_fric,
            l_tread_src,
            l_tread_loss,
            l_striker,
            r_exh,
            r_int,
            r_shell,
            r_chassis,
            r_tread,
            fires: 0,
            pending_impact: 0.0,
            pending_lost: 0.0,
            l_impact,
            fresh: 0.0,
            fresh_last: 0.0,
            xi_last: 0.0,
            misfires: 0,
            revs: 0,
            last_out: 0.0,
            peak_cyl: 0.0,
            max_numeric_frac: 0.0,
        };
        e.b.numerical_loss_index = Some(l_numeric);
        let s0 = e.stored();
        e.b.init(s0);
        e
    }

    fn v_cyl(&self, th: f64) -> f64 {
        let v_c = self.area_p * STROKE / (CR - 1.0);
        v_c + self.area_p * piston_x(th)
    }
    fn v_crank(&self, th: f64) -> f64 {
        V_K0 + self.area_p * (STROKE - piston_x(th))
    }

    fn stored(&self) -> f64 {
        let dt = self.dt;
        let mut e = self.cyl.energy() + self.crank.energy();
        e += 0.5 * J_FLY * self.omega * self.omega;
        e += (self.ex.energy_sum() + self.inl.energy_sum() + self.ex_mouth.stored() + self.in_mouth.stored()) * dt;
        e += self.shell.iter().map(|m| m.energy()).sum::<f64>();
        e += self.chassis.energy();
        e += self.tread.iter().map(|t| 0.5 * t.m * t.v * t.v + 0.5 * t.k * t.x * t.x).sum::<f64>();
        e
    }

    fn kick(&mut self, point: u64, mass: f64, v: f64, kc: f64, cc: f64) {
        let old = self.chassis.strike(point, mass, v, kc, cc, 0.15);
        self.pending_impact += 0.5 * mass * v * v;
        self.pending_lost += old;
    }

    #[inline]
    fn tick(&mut self) -> f64 {
        let dt = self.dt;
        if self.pending_impact != 0.0 {
            self.b.add(self.l_impact, self.pending_impact);
            self.pending_impact = 0.0;
        }
        if self.pending_lost > 0.0 {
            self.b.add(self.l_striker, self.pending_lost);
            self.pending_lost = 0.0;
        }
        let th0 = self.theta;
        let omega0 = self.omega;
        let ke0 = 0.5 * J_FLY * omega0 * omega0;

        // ---------------- crank angle, volumes, the piston's work on the gas
        let th1 = th0 + omega0 * dt;
        self.theta = if th1 >= PI { th1 - TAU } else { th1 };
        let thc = self.theta; // in [-pi, pi): 0 = TDC
        let w_cyl = self.cyl.set_volume(self.v_cyl(thc));
        let w_crk = self.crank.set_volume(self.v_crank(thc));
        let piston_to_gas = w_cyl + w_crk; // >0 when the piston compresses
        // crossing TDC counts a revolution
        if th0 < 0.0 && thc >= 0.0 && th0 > -1.0 {
            self.revs += 1;
        }

        // ---------------- ignition and burn (declared source)
        let th_ign = THETA_IGN;
        let crossed_ign = th0 < th_ign && thc >= th_ign && (thc - th0).abs() < 1.0;
        if crossed_ign {
            let cranking = self.run == Run::Cranking;
            let p_gauge = self.cyl.p().max(0.0);
            let _ = p_gauge;
            self.fresh_last = self.fresh;
            let xi = (self.fresh / V_FRESH_REF).clamp(0.0, 1.3);
            self.fresh = 0.0;
            self.xi_last = xi;
            let thresh = if cranking { 0.06 } else { XI_MIN };
            if self.run != Run::Dead && xi > thresh {
                let rich = if cranking { 1.6 } else { 1.0 };
                let c_tdc = self.v_cyl(0.0) / (GAMMA * P0);
                self.burn_dq = xi * rich * c_tdc * DP_BURN;
                self.burn_x = 0.0;
                self.burning = true;
                self.fires += 1;
            } else if self.run != Run::Dead {
                self.misfires += 1;
            }
        }
        let mut src_comb = 0.0;
        if self.burning {
            let a = 5.0f64;
            let m = 2.0f64;
            let s0 = ((th0 - th_ign) / BURN_SPAN).max(0.0);
            let s1 = ((thc - th_ign) / BURN_SPAN).max(0.0);
            let x = |s: f64| if s >= 1.0 { 1.0 } else { 1.0 - (-a * s.powf(m + 1.0)).exp() };
            let (x0, x1) = (x(s0), x(s1));
            let dq = self.burn_dq * (x1 - x0).max(0.0);
            let e0 = self.cyl.energy();
            self.cyl.q += dq;
            src_comb = self.cyl.energy() - e0;
            if s1 >= 1.0 {
                self.burning = false;
            }
        }
        self.b.add(self.l_comb, src_comb);

        // ---------------- ducts: what arrives
        let (ex_a, ex_ra) = self.ex.arrive();
        let (in_a, in_ra) = self.inl.arrive();
        let rx = self.theta;
        let _ = rx;
        let x_p = piston_x(thc);

        // ---------------- transfer port: crankcase -> cylinder (both capacitors)
        {
            let h = x_p - X_TR;
            let a = if h > 0.0 { (W_TR * h).min(4.0e-4) } else { 0.0 };
            if a > 1e-9 {
                let k = orifice_k(1.2, a, CD_PORT);
                let beta = dt * (1.0 / self.crank.c + 1.0 / self.cyl.c);
                let d = self.crank.p() - self.cyl.p();
                let u = solve_orifice(k, beta, d);
                let e_before = self.crank.energy() + self.cyl.energy();
                self.crank.q -= u * dt;
                self.cyl.q += u * dt;
                self.fresh += u * dt;
                let _ = e_before;
                let jet = k * u.abs().powi(3) * dt;
                let num = 0.5 * beta * u * u * dt;
                self.b.add(self.l_orifice, jet);
                self.b.add(self.l_numeric, num);
            }
        }

        // ---------------- exhaust port: cylinder -> pipe (Thevenin from the duct)
        let z0 = self.ex.z_left();
        let a_p = ex_a * z0.sqrt(); // pressure wave arriving
        let mut ex_b = ex_a; // normalised wave sent
        {
            let h = x_p - X_EX;
            let a = if h > 0.0 { (W_EX * h).min(6.6e-4) } else { 0.0 };
            if a > 1e-9 {
                let k = orifice_k(1.0, a, CD_PORT);
                let beta = z0 + dt / self.cyl.c;
                let d = self.cyl.p() - 2.0 * a_p;
                let u = solve_orifice(k, beta, d);
                let e_before = self.cyl.energy();
                self.cyl.q -= u * dt;
                let b_p = a_p + z0 * u;
                ex_b = b_p / z0.sqrt();
                let p_t = 2.0 * a_p + z0 * u;
                let to_pipe = p_t * u * dt; // exact port power = (b^2-a^2)/Z dt
                let jet = k * u.abs().powi(3) * dt;
                let _ = e_before;
                let num = 0.5 * u * u * dt * dt / self.cyl.c;
                let _ = (to_pipe, jet);
                self.b.add(self.l_orifice, jet);
                self.b.add(self.l_numeric, num);
            }
        }

        // ---------------- intake: pipe -> throat + reed (+ blower) -> crankcase
        let zi = self.inl.z_left();
        let ai_p = in_a * zi.sqrt();
        let mut in_b = in_a;
        {
            let thr_eff = clamp(self.throttle * (0.30 + 0.70 * self.load), 0.0, 1.0);
            let a_thr = 1.0e-5 + 1.5e-4 * thr_eff;
            let k = orifice_k(1.2, a_thr, 0.8) + orifice_k(1.2, 3.0e-4, 0.6);
            let p_b = self.boost * 0.35e5;
            let beta = zi + dt / self.crank.c;
            let d = 2.0 * ai_p + p_b - self.crank.p();
            if d > 0.0 {
                let u = solve_orifice(k, beta, d);
                let e_before = self.crank.energy();
                self.crank.q += u * dt;
                let _ = e_before;
                let b_p = ai_p - zi * u;
                in_b = b_p / zi.sqrt();
                let p_t = 2.0 * ai_p - zi * u;
                let _from_pipe = p_t * u * dt; // energy the pipe gives (u leaves it)
                let jet = k * u.abs().powi(3) * dt;
                let blow = p_b * u * dt;
                let num = 0.5 * u * u * dt * dt / self.crank.c;
                self.b.add(self.l_orifice, jet);
                self.b.add(self.l_blower, blow);
                self.b.add(self.l_numeric, num);
            }
        }

        // ---------------- engine shell: walls of the cylinder gas
        {
            let mut loss = 0.0;
            let mut rad = 0.0;
            let mut press = 0.0;
            for m in self.shell.iter_mut() {
                let (vb, l, r, _taken) = m.step_wall(2.0e-4, &mut self.cyl, dt);
                loss += l;
                rad += r;
                press += m.kappa * vb;
            }
            self.b.add(self.l_shell, loss);
            self.b.add(self.r_shell, rad);
            self.last_out += press;
        }

        // ---------------- wave networks advance
        let b_ex_m = self.ex_mouth.step(ex_ra);
        let b_in_m = self.in_mouth.step(in_ra);
        self.ex.depart(ex_b, b_ex_m);
        self.inl.depart(in_b, b_in_m);
        self.b.add(self.l_wall, (self.ex.loss + self.inl.loss) * dt);
        self.b.add(self.r_exh, self.ex_mouth.radiated * dt);
        self.b.add(self.r_int, self.in_mouth.radiated * dt);
        self.b.add(self.l_foam, self.in_mouth.absorbed * dt);

        // ---------------- crank: bearing friction, rope, servo / spin-down
        let cf = if self.run == Run::Dead { C_FRIC_DEAD } else { C_FRIC };
        let fric = cf * omega0 * omega0 * dt;
        self.b.add(self.l_fric, fric);
        let ke_after_gas = ke0 + piston_to_gas * -1.0; // gas gave energy to the crank when it expanded
        // piston_to_gas > 0: piston compressed the gas, so the crank gave that energy
        let ke_free = ke_after_gas - fric;
        self.run_t += dt;
        match self.run {
            Run::Running => {
                let w_cmd = clamp(self.rpm, 1800.0, 14000.0) * TAU / 60.0;
                let max_d = 3200.0 * dt; // rad/s per tick: about 30 000 rpm/s
                let w1 = omega0 + clamp(w_cmd - omega0, -max_d, max_d);
                let ke1 = 0.5 * J_FLY * w1 * w1;
                self.b.add(self.l_servo, ke1 - ke_free);
                self.omega = w1;
            }
            Run::Cranking => {
                // the rope: a pulled speed profile; the hand's work is control work
                let w_cmd = 55.0 * (self.run_t / 0.12).min(1.0) * if self.run_t < 0.45 { 1.0 } else { 0.0 };
                let w_cmd = if self.run_t < 0.45 { w_cmd } else { clamp(self.rpm, 1800.0, 14000.0) * TAU / 60.0 };
                let max_d = if self.run_t < 0.45 { 1200.0 * dt } else { 3200.0 * dt };
                let w1 = (omega0 + clamp(w_cmd - omega0, -max_d, max_d)).max(0.0);
                let ke1 = 0.5 * J_FLY * w1 * w1;
                self.b.add(self.l_rope, ke1 - ke_free);
                self.omega = w1;
                if self.run_t > 0.6 {
                    self.run = Run::Running;
                }
            }
            Run::Dead => {
                let ke1 = ke_free.max(0.0);
                if ke_free < 0.0 {
                    // the engine stalled between strokes: the shortfall was never there
                    self.b.add(self.l_servo, -ke_free);
                }
                self.omega = (2.0 * ke1 / J_FLY).sqrt().max(0.0);
            }
        }

        // ---------------- chassis (struck)
        let mut p_chassis = 0.0;
        if self.chassis.striker.is_some() || self.chassis.energy() > 1e-14 {
            let st = self.chassis.step(dt, None);
            self.b.add(self.l_chassis, st.loss);
            self.b.add(self.l_contact, st.contact_loss);
            self.b.add(self.r_chassis, st.radiated);
            p_chassis = st.pressure;
            let gone = self.chassis.release_striker_if_gone();
            self.b.add(self.l_striker, gone);
        }

        // ---------------- tyre scrub
        let mut p_tread = 0.0;
        if self.skid > 0.005 || self.tread.iter().any(|t| t.v.abs() > 1e-6) {
            let vs = 14.0 * self.skid;
            let (mu_s, mu_d, vst) = if self.lakebed { (0.55, 0.46, 16.0) } else { (1.25, 0.80, 16.0) };
            let (mut fr, mut src, mut dmp, mut rad, mut rough) = (0.0, 0.0, 0.0, 0.0, 0.0);
            let sig_r = if self.lakebed { 15.0 } else { 0.1 } * self.skid;
            for t in self.tread.iter_mut() {
                // unknown vbar: m*2(vbar-v0)/dt = F(vs-vbar) - k(x0 + dt/2 vbar) - c vbar
                let f_r = t.n * mu_d * sig_r * self.rng.gauss();
                let fric_f = |vb: f64| {
                    let vr = vs - vb;
                    let rr = vr / vst;
                    let mu = mu_d + (mu_s - mu_d) / (1.0 + rr * rr);
                    t.n * mu * (vr / 0.02).tanh()
                };
                let res = |vb: f64| t.m * 2.0 * (vb - t.v) / dt - fric_f(vb) - f_r + t.k * (t.x + 0.5 * dt * vb) + t.c * vb;
                let mut vb = t.v;
                for _ in 0..12 {
                    let r = res(vb);
                    let h = 1e-6;
                    let dr = (res(vb + h) - r) / h;
                    if dr.abs() < 1e-12 {
                        break;
                    }
                    let step = r / dr;
                    vb -= step.clamp(-2.0, 2.0);
                    if step.abs() < 1e-12 {
                        break;
                    }
                }
                let f = fric_f(vb);
                fr += f * (vs - vb) * dt;
                src += f * vs * dt;
                rough += f_r * vb * dt;
                dmp += (t.c - t.c_rad) * vb * vb * dt;
                rad += t.c_rad * vb * vb * dt;
                t.v = 2.0 * vb - t.v;
                t.x += dt * vb;
                p_tread += t.kappa * vb;
            }
            self.b.add(self.l_tread_src, src);
            self.b.add(self.l_rough, rough);
            self.b.add(self.l_tread_fric, fr);
            self.b.add(self.l_tread_loss, dmp);
            self.b.add(self.r_tread, rad);
        }

        // ---------------- books and output
        let stored = self.stored();
        self.b.close(stored);
        if self.cyl.p() > self.peak_cyl {
            self.peak_cyl = self.cyl.p();
        }
        let p_exh = far_pressure(self.ex_mouth.w_r_out, 2.0 * PI, 1.0);
        let p_in = far_pressure(self.in_mouth.w_r_out, 2.0 * PI, 1.0);
        let out = p_exh + p_in + self.last_out + p_chassis + p_tread;
        self.last_out = 0.0;
        self.prev_theta = th0;
        out
    }

    pub fn numeric_fraction(&self) -> f64 {
        let n = self.b.report()["lines"]["loss:backward_euler_dissipation"].as_f64().unwrap_or(0.0);
        let s = self.b.ledger.source_work.max(1e-30);
        n / s
    }
    pub fn fires(&self) -> u64 {
        self.fires
    }
    pub fn revs(&self) -> u64 {
        self.revs
    }
    pub fn misfires(&self) -> u64 {
        self.misfires
    }
    pub fn omega(&self) -> f64 {
        self.omega
    }
    pub fn books(&self) -> &Books {
        &self.b
    }
}

impl GameWorld for KartEngine {
    fn kind(&self) -> &'static str {
        "kart_engine"
    }
    fn set(&mut self, name: &str, v: f64) -> bool {
        match name {
            "rpm" => self.rpm = clamp(v, 1800.0, 14000.0),
            "throttle" => self.throttle = clamp(v, 0.0, 1.0),
            "load" => self.load = clamp(v, 0.0, 1.0),
            "boost" => self.boost = clamp(v, 0.0, 1.0),
            "skid" => self.skid = clamp(v, 0.0, 1.0),
            "surface" => self.lakebed = v >= 0.5,
            _ => return false,
        }
        true
    }
    fn set_text(&mut self, name: &str, v: &str) -> bool {
        match (name, v) {
            ("surface", "track") => self.lakebed = false,
            ("surface", "lakebed") => self.lakebed = true,
            _ => return false,
        }
        true
    }
    fn event(&mut self, name: &str, arg: f64) -> bool {
        match name {
            "start" | "restart" => {
                // a hand stops the flywheel, then the rope is pulled
                let ke = 0.5 * J_FLY * self.omega * self.omega;
                self.pending_impact -= ke;
                self.omega = 0.0;
                self.burning = false;
                self.run = Run::Cranking;
                self.run_t = 0.0;
            }
            "bump" => {
                let i = clamp(arg, 0.0, 1.0);
                let n = self.chassis.contacts;
                self.kick(1000 + n % 5, 1.2, 0.3 + 1.7 * i, 3.0e7, 120.0);
            }
            "land" => {
                let i = clamp(arg, 0.0, 1.0);
                let n = self.chassis.contacts;
                self.kick(2000 + n % 3, 6.0, 0.3 + 1.5 * i, 6.0e6, 400.0);
            }
            "hit" => {
                self.kick(3000, 3.0, 4.0, 2.0e7, 300.0);
                self.run = Run::Dead;
                self.burning = false;
            }
            _ => return false,
        }
        true
    }
    fn render(&mut self, out: &mut [f32]) {
        for o in out.iter_mut() {
            let p = self.tick();
            *o = (p / OUTPUT_SCALE_PA) as f32;
        }
    }
    fn receipt(&self) -> serde_json::Value {
        serde_json::json!({
            "world": "kart_engine",
            "sampleRate": self.fs,
            "ledger": self.b.report(),
            "ignitions": self.fires,
            "misfires": self.misfires,
            "revolutions": self.revs,
            "peakCylinderGaugePa": self.peak_cyl,
            "outputScalePa": OUTPUT_SCALE_PA,
            "declaredSources": ["combustion heat release (fitted Wiebe burn, charge-proportional)", "crank servo / load (control)", "pull rope", "boost blower pressure", "tyre slip work"],
            "stochastic": "ground roughness force on tread blocks (seeded), only while skid > 0",
        })
    }
    fn output_scale(&self) -> f64 {
        OUTPUT_SCALE_PA
    }
    fn sample_rate(&self) -> f64 {
        self.fs
    }
}
