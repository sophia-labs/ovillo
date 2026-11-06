//! race_props: the start-light buzzer and the finish bell, outdoors.
//!
//! Buzzer: a spring-held armature that a constant pull drags toward a plate
//! while a spring contact is closed. The contact is a position gate: it opens
//! when the armature has travelled `Y_OPEN`, closes again when it has come
//! back to `Y_CLOSE` (hysteresis), so the armature self-oscillates. It also
//! strikes the plate through a Hertz contact; the plate (and the housing's
//! panel) are struck modal bodies that radiate. There is no coil or circuit:
//! the pull is a declared force whose work is the buzzer's only source.
//! `beep` drives it for 0.22 s; `go` drives it harder (stiffer return
//! spring) for 0.9 s.
//!
//! Bell: sixteen modes (eight partials of a bell-like series, each as a close
//! pair a few tenths of a percent apart, so it beats) struck by a clapper
//! mass through a Hertz contact. `bell` is one stroke (the last-lap bell);
//! `finish` is a swung clapper: nine strokes over about 3 s, each the clapper
//! arriving a little slower.
//!
//! What it does not model: the coil, contact bounce beyond the gate, a
//! housing cavity, the bell's real mode shapes (the ratios are a design choice
//! after the minor-third church bell series; the pairs' splits and the shapes
//! at the strike point are invented), the clapper swing as a pendulum (its
//! arrival speed is the control), the stand it hangs from, the air path.

use crate::net::*;
use crate::GameWorld;
use std::f64::consts::PI;

pub const OUTPUT_SCALE_PA: f64 = 20.0;
const Y_OPEN: f64 = 0.22e-3;
const Y_CLOSE: f64 = 0.17e-3;
const GAP: f64 = 0.40e-3;
const M_ARM: f64 = 0.004;

pub struct RaceProps {
    fs: f64,
    dt: f64,
    buzzer: Body,
    bell: Body,
    gate_on: bool,
    drive_left: f64, // seconds of drive remaining
    f_pull: f64,
    ks_target: f64,
    queue: Vec<(f64, f64)>, // (seconds from now, clapper speed)
    clock: f64,
    b: Books,
    l_pull: usize,
    l_clap: usize,
    l_ctrl_spring: usize,
    l_plate: usize,
    l_arm: usize,
    l_contact: usize,
    l_bell: usize,
    l_bell_contact: usize,
    l_gone: usize,
    r_buzz: usize,
    r_bell: usize,
    pending_clap: f64,
    pending_spring: f64,
    pending_lost: f64,
    pub contacts_armature: u64,
    pub strokes: u64,
}

impl RaceProps {
    pub fn new(fs: f64) -> Self {
        let dt = 1.0 / fs;
        let omega = 2.0 * PI;
        // buzzer housing: plate and panel modes
        let plate = [(1100.0, 0.20, 0.008), (2150.0, 0.12, 0.004), (3950.0, 0.09, 0.003), (6100.0, 0.07, 0.002), (8800.0, 0.05, 0.002)];
        let pm: Vec<Mode> = plate.iter().map(|&(f, t60, m)| Mode::new(m, f, t60, 0.25, dt, omega)).collect();
        let mut buzzer = Body::new(pm);
        // the armature as a pinned striker held by its return spring
        let f_arm = 480.0;
        let ks = M_ARM * (2.0 * PI * f_arm).powi(2);
        for (n, p) in buzzer.phi.iter_mut().enumerate() {
            *p = 0.8 / (1.0 + 0.25 * n as f64) * if n % 2 == 0 { 1.0 } else { -0.7 };
        }
        buzzer.striker = Some(Striker { mass: M_ARM, x: -GAP, v: 0.0, kc: 5.0e8, cc: 20.0, ks, rest: -GAP, cs: 0.8, f_ext: 0.0, pinned: true });
        // the bell
        let prime = 640.0;
        let ratios = [0.5, 1.0, 1.2, 1.5, 2.0, 2.5, 3.0, 4.0];
        let t60s = [5.0, 3.2, 2.6, 2.0, 1.5, 1.0, 0.8, 0.5];
        let mut bm = Vec::new();
        for (i, (&r, &t)) in ratios.iter().zip(t60s.iter()).enumerate() {
            let f = prime * r;
            bm.push(Mode::new(0.4, f, t, 0.06, dt, omega));
            bm.push(Mode::new(0.4, f * (1.0 + 0.0035 + 0.0007 * i as f64), t * 0.9, 0.06, dt, omega));
        }
        let bell = Body::new(bm);
        let mut b = Books::new();
        let l_pull = b.line("armature_pull_force", Kind::Source);
        let l_clap = b.line("clapper_thrown", Kind::Control);
        let l_ctrl_spring = b.line("return_spring_retuned", Kind::Control);
        let l_plate = b.line("plate_damping", Kind::Loss);
        let l_arm = b.line("armature_air_damping", Kind::Loss);
        let l_contact = b.line("armature_contact_damping", Kind::Loss);
        let l_bell = b.line("bell_damping", Kind::Loss);
        let l_bell_contact = b.line("clapper_contact_damping", Kind::Loss);
        let l_gone = b.line("clapper_carries_away", Kind::Loss);
        let r_buzz = b.line("buzzer_housing", Kind::Radiation);
        let r_bell = b.line("bell", Kind::Radiation);
        let mut s = RaceProps {
            fs,
            dt,
            buzzer,
            bell,
            gate_on: false,
            drive_left: 0.0,
            f_pull: 14.0,
            ks_target: ks,
            queue: vec![],
            clock: 0.0,
            b,
            l_pull,
            l_clap,
            l_ctrl_spring,
            l_plate,
            l_arm,
            l_contact,
            l_bell,
            l_bell_contact,
            l_gone,
            r_buzz,
            r_bell,
            pending_clap: 0.0,
            pending_spring: 0.0,
            pending_lost: 0.0,
            contacts_armature: 0,
            strokes: 0,
        };
        let e0 = s.stored();
        s.b.init(e0);
        s
    }

    fn stored(&self) -> f64 {
        self.buzzer.energy() + self.bell.energy()
    }

    fn drive(&mut self, secs: f64, f_pull: f64, f_arm: f64) {
        self.drive_left = secs;
        self.f_pull = f_pull;
        // retune the return spring: control work is the spring energy it changes
        let ks_new = M_ARM * (2.0 * PI * f_arm).powi(2);
        if let Some(s) = self.buzzer.striker.as_mut() {
            let y = s.x - s.rest;
            self.pending_spring += 0.5 * (ks_new - s.ks) * y * y;
            s.ks = ks_new;
        }
        self.ks_target = ks_new;
    }

    fn clap(&mut self, v: f64) {
        let old = self.bell.strike(self.strokes % 7 + 11, 0.03, v, 3.0e8, 40.0, 0.04);
        self.pending_clap += 0.5 * 0.03 * v * v;
        self.pending_lost += old;
        self.strokes += 1;
    }

    fn tick(&mut self) -> f64 {
        let dt = self.dt;
        self.clock += dt;
        // scheduled strokes
        let mut i = 0;
        while i < self.queue.len() {
            self.queue[i].0 -= dt;
            if self.queue[i].0 <= 0.0 {
                let v = self.queue[i].1;
                self.queue.remove(i);
                self.clap(v);
            } else {
                i += 1;
            }
        }
        if self.pending_clap != 0.0 {
            self.b.add(self.l_clap, self.pending_clap);
            self.pending_clap = 0.0;
        }
        if self.pending_spring != 0.0 {
            self.b.add(self.l_ctrl_spring, self.pending_spring);
            self.pending_spring = 0.0;
        }
        if self.pending_lost != 0.0 {
            self.b.add(self.l_gone, self.pending_lost);
            self.pending_lost = 0.0;
        }
        // the buzzer's contact gate
        if self.drive_left > 0.0 {
            self.drive_left -= dt;
            if let Some(s) = self.buzzer.striker.as_mut() {
                let y = s.x - s.rest;
                if self.gate_on && y > Y_OPEN {
                    self.gate_on = false;
                } else if !self.gate_on && y < Y_CLOSE {
                    self.gate_on = true;
                }
                s.f_ext = if self.gate_on { self.f_pull } else { 0.0 };
            }
        } else {
            self.gate_on = false;
            if let Some(s) = self.buzzer.striker.as_mut() {
                s.f_ext = 0.0;
            }
        }
        let mut p = 0.0;
        let before = self.buzzer.contacts;
        let _ = before;
        // buzzer
        let active = self.drive_left > 0.0 || self.buzzer.energy() > 1e-13;
        if active {
            let st = self.buzzer.step(dt, None);
            self.b.add(self.l_pull, st.applied_work);
            self.b.add(self.l_plate, st.loss);
            self.b.add(self.l_arm, st.striker_loss);
            self.b.add(self.l_contact, st.contact_loss);
            self.b.add(self.r_buzz, st.radiated);
            p += st.pressure;
            if st.force > 0.0 {
                self.contacts_armature += 1;
            }
        }
        if self.bell.striker.is_some() || self.bell.energy() > 1e-13 {
            let st = self.bell.step(dt, None);
            self.b.add(self.l_bell, st.loss);
            self.b.add(self.l_bell_contact, st.contact_loss);
            self.b.add(self.r_bell, st.radiated);
            p += st.pressure;
            let gone = self.bell.release_striker_if_gone();
            self.b.add(self.l_gone, gone);
        }
        let e = self.stored();
        self.b.close(e);
        p
    }
}

impl GameWorld for RaceProps {
    fn kind(&self) -> &'static str {
        "race_props"
    }
    fn set(&mut self, _name: &str, _v: f64) -> bool {
        false
    }
    fn event(&mut self, name: &str, arg: f64) -> bool {
        match name {
            "beep" => {
                let _ = arg; // n = 3, 2, 1: the same physical buzzer each time
                self.drive(0.22, 16.0, 600.0);
            }
            "go" => self.drive(0.9, 26.0, 800.0),
            "bell" => self.clap(2.4),
            "finish" => {
                for i in 0..9 {
                    self.queue.push((0.33 * i as f64 + 0.001, 2.5 - 0.15 * i as f64));
                }
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
            "world": "race_props",
            "sampleRate": self.fs,
            "ledger": self.b.report(),
            "armatureImpacts": self.contacts_armature,
            "bellStrokes": self.strokes,
            "outputScalePa": OUTPUT_SCALE_PA,
            "declaredSources": ["armature pull force (constant while the contact gate is closed)", "clapper thrown at a set speed (control)"],
            "stochastic": false,
        })
    }
    fn output_scale(&self) -> f64 {
        OUTPUT_SCALE_PA
    }
    fn sample_rate(&self) -> f64 {
        self.fs
    }
}
