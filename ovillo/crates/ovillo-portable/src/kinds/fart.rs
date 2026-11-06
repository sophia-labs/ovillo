//! The fart (`ovillo_tract_world::fart`, Vera's October work): the tract
//! World's other end. The valve is a sphincter, the tube a canal flaring into
//! a cleft, `lips` the cheeks; the reservoir is filled by digestion (a food
//! table) and spent through the valve. A `Gut` keeps one World between calls.
//! Every number in the anatomy, the foods and the styles is invented (no
//! measurement, no source), as `fart.rs` says.
//!
//! The listener is `fart.rs`'s: the World's radiated pressure times
//! `LISTENER_GAIN`, through its 20 Hz one-pole DC blocker (steady flow is
//! steady pressure at the mouth), here applied sample by sample.

use ovillo_tract_world::fart::{self, Gut, Style, FOODS, HOP_S, LISTENER_GAIN, RASP_AT, RASP_SLOT, SEAL_AT, SHOT_LOAD_S, STYLES};
use ovillo_tract_world::world::{Constriction, TractControls, Vibrato};
use ovillo_tract_world::TractNdAnatomy;
use serde_json::{json, Value};

use crate::catalog::*;
use crate::{CreateCtx, Source, MAX_ARGS};

const SEAL_SLOT: usize = 1;
/// `fart.rs` `SHOT_OPEN_S`.
const SHOT_OPEN_S: f64 = 0.1;
const DC_BLOCK_HZ: f64 = 20.0;

pub fn kind() -> KindDef {
    let styles: Vec<&str> = STYLES.iter().map(|s| s.name).collect();
    let foods: Vec<&str> = FOODS.iter().map(|f| f.name).collect();
    KindDef {
        id: "fart".into(),
        title: "Fart".into(),
        family: "fart".into(),
        summary: "The tract World's other end: a sphincter valve, a canal flaring into a cleft (the cheeks as its lips), a reservoir filled by digestion and spent through the valve. Every number is invented.".into(),
        spec: "ovillo/crates/ovillo-tract-world/src/fart.rs".into(),
        sample_rate: None,
        channels: 1,
        anatomies: vec![
            anatomy("alto", "fart_v1", "TractNdAnatomy::fart_v1: a 1.3 g valve, a slower slam than v0 (fewer clicks, fewer notes in tune)", true),
            anatomy("soprano", "fart_v1_soprano", "fart_v1 with v0's 1 g valve", false),
            anatomy("bass", "fart_v1_bass", "a 1.6 g valve on a 28 cm canal into an 8 cm2 cleft", false),
            anatomy("cannon", "fart_v1_cannon", "a 2.2 g valve, 40 cm canal, 14 cm2 cleft, 3.5 kPa supply, 2.5 J reservoir", false),
            anatomy("v0", "fart_v0", "the first: a 1 g valve that clicks once per cycle on held notes", false),
        ],
        open_anatomies: false,
        options: vec![option("gas", "number", json!(0.0), "joules already in the reservoir at the start (0..capacity); otherwise eat and digest first")],
        events: vec![
            event("style", "Play a named control path (fart.rs STYLES), from whatever the reservoir holds; set every 10 ms.", vec![arg_enum("name", &styles, 0, &STYLES.iter().map(|s| format!("{}: {}", s.name, s.about)).collect::<Vec<_>>().join("; "))]),
            event("eat", "A food into the stomach: it pays its gas into the reservoir over its digestion time while digest is on (invented: a game's table, not nutrition).", vec![arg_enum("food", &foods, 0, &FOODS.iter().map(|f| format!("{} {} J over {} s", f.name, f.gas_j, f.seconds)).collect::<Vec<_>>().join(", "))]),
            event("shot", "A cannon shot: the cheeks clench at 0.85 push for 120 ms while the pressure builds behind the seal, then open wide (fart.rs play_cannon).", vec![]),
            event("stop", "End a style or shot: no push, cheeks unclenched.", vec![]),
        ],
        controls: vec![
            control("push", "0..1", 0.0, 1.0, 0.0, 60.0, "set at once; the World's articulators and valve follow at their own speed", "the valve's supply (the push)"),
            control("cheeks", "0..1", 0.0, 1.0, 0.4, 60.0, "the articulators' 25 ms approach", "the cleft's opening (the tract's lips; below 0.02 it seals)"),
            control("tone", "0..1", 0.0, 1.0, 0.75, 30.0, "", "the sphincter's adduction (tenseness)"),
            control("tension", "Hz", 25.0, 300.0, 70.0, 60.0, "30 ms; booked valve retunes", "the valve's tuning"),
            switch("clench", Some(&["open", "clenched"]), 1.0, 0.0, "clench the cheeks (a seal inside the cleft): pressure builds behind it; releasing it bursts"),
            control("rasp", "0..1", 0.0, 1.0, 0.0, 30.0, "", "a partial clench near the exit: the flow there becomes a jet and the World's turbulence law sputters"),
            control("wobble", "0..1", 0.0, 1.0, 0.0, 10.0, "", "slow irregular tension drift"),
            control("vibratoDepth", "semitones", 0.0, 2.0, 0.0, 10.0, "", ""),
            control("vibratoRate", "Hz", 0.1, 12.0, 5.5, 10.0, "", ""),
            control("jitter", "semitones", 0.0, 6.0, 0.0, 10.0, "redrawn every 10 ms", "random tension jitter (flutter)"),
            switch("digest", Some(&["off", "on"]), 1.0, 1.0, "the stomach pays the reservoir (on) or holds its food"),
            control("refill", "W", 0.0, 1.0, 0.0, 10.0, "", "the house feed: what digestion pays while the stomach is empty (the World's work_input)"),
        ],
        telemetry: vec![
            slot("gasFraction", "0..1", "the reservoir's fill"),
            slot("stomachJ", "J", "gas the stomach still holds"),
            slot("f0Hz", "Hz", "the valve's measured cycle rate"),
            slot("supplyPa", "Pa", "the valve's supply pressure"),
            slot("styleActive", "0/1", ""),
        ],
        score: None,
        output: OutputDef {
            observes: "the radiated pressure at the cleft and the stub (Pa)".into(),
            gain: LISTENER_GAIN,
            channels: "one point (left = right)".into(),
            about: "fart.rs's listener: pressure x 1/1000, clamped to [-1, 1] by the tract's listener, then its 20 Hz one-pole DC blocker (a listening stage, not part of the World; the ledger is untouched)".into(),
        },
        lookahead_seconds: 0.0,
        cost: "the tract World on a 28+3 volume network: 29x real time, measured in node (wasm, 128-frame quanta, tools/rtf.mjs) on a 2019 8-core i9 laptop".into(),
        does_not_model: lines(&[
            "Every number (anatomy, foods, styles) is invented: no measurement, no source.",
            "The canal is 20 cm or longer because the World's 24 modes must lie inside its band; a real one is shorter.",
            "Digestion is a reservoir refill rate, not chemistry; the gas is energy, not a fluid with composition or temperature.",
            "Everything the tract does not model applies (one jet, mode-diagonal losses, stepped moves).",
        ]),
        does_not_model_ref: "ovillo/crates/ovillo-tract-world/src/fart.rs".into(),
        build,
    }
}

fn anatomy_for(name: &str) -> Result<TractNdAnatomy, String> {
    fart::anatomy(name).ok_or_else(|| format!("no fart anatomy {name:?}"))
}

struct FartSource {
    gut: Gut,
    anatomy: String,
    sample_rate: f64,
    hop: usize,
    phase: usize,
    style: Option<(Style, f64)>,
    shot: Option<f64>,
    digest: bool,
    jitter: f64,
    rng: u64,
    base_tension: f64,
    clench: bool,
    rasp: f64,
    x1: f64,
    y1: f64,
    r: f64,
}

fn build(_def: &KindDef, ctx: &CreateCtx) -> Result<Box<dyn Source>, String> {
    let name = ctx.anatomy.unwrap_or("alto").to_owned();
    let gas = super::opt::f64_or(ctx.options, "gas", 0.0)?;
    Ok(Box::new(FartSource::new(name, ctx.sample_rate, gas)?))
}

impl FartSource {
    fn new(anatomy: String, sample_rate: f64, gas: f64) -> Result<Self, String> {
        let mut gut = Gut::with_anatomy(sample_rate, anatomy_for(&anatomy)?).map_err(|e| e.to_string())?;
        if gas > 0.0 {
            // Prefill: one silent pass of the reservoir's own refill (booked as source work).
            let cap = gut.world().anatomy().lung_capacity_j;
            let watts = 1.0;
            let samples = ((gas.min(cap) / watts) * sample_rate) as usize;
            gut.world_mut().set_refill(watts);
            let mut block = vec![0.0f32; 480];
            let mut left = samples;
            while left > 0 {
                let n = left.min(block.len());
                gut.world_mut().process(&mut block[..n]).map_err(|e| e.to_string())?;
                left -= n;
            }
            gut.world_mut().set_refill(0.0);
        }
        let tension = gut.world().anatomy().valve.initial_tuning_hz;
        Ok(Self {
            gut,
            anatomy,
            sample_rate,
            hop: ((HOP_S * sample_rate) as usize).max(1),
            phase: 0,
            style: None,
            shot: None,
            digest: true,
            jitter: 0.0,
            rng: 0x2545_F491_4F6C_DD1D,
            base_tension: tension,
            clench: false,
            rasp: 0.0,
            x1: 0.0,
            y1: 0.0,
            r: (-std::f64::consts::TAU * DC_BLOCK_HZ / sample_rate).exp(),
        })
    }

    fn rand(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }

    fn seal(&mut self, sealed: bool) {
        self.gut.world_mut().set_constriction(SEAL_SLOT, sealed.then_some(Constriction { position: SEAL_AT, degree: 1.0 }));
    }

    /// What the player does at the start of each 10 ms hop (`fart.rs`'s hop loop).
    fn hop(&mut self) {
        if self.digest {
            self.gut.pay_hop();
        } else {
            let house = self.gut.house_w;
            self.gut.world_mut().set_refill(house);
        }
        if let Some((style, t)) = self.style {
            if t >= style.seconds {
                self.style = None;
                let mut c = self.gut.world().controls();
                c.breath = 0.0;
                self.gut.world_mut().set_controls(c);
                self.seal(self.clench);
            } else {
                let mut c = (style.at)(t);
                if style.jitter_semitones > 0.0 {
                    let u = self.rand();
                    c.pitch_hz *= ((2.0 * u - 1.0) * style.jitter_semitones / 12.0).exp2();
                }
                self.gut.world_mut().set_controls(c);
                self.seal((style.sealed)(t));
                self.style = Some((style, t + HOP_S));
                return;
            }
        }
        if let Some(t) = self.shot {
            let loading = t < SHOT_LOAD_S;
            let open = t < SHOT_LOAD_S + SHOT_OPEN_S;
            let mut c = self.gut.world().controls();
            c.articulation = fart::cheeks(if loading { 0.45 } else { 0.8 });
            c.pitch_hz = 40.0;
            c.tenseness = 0.6;
            c.breath = if open { 0.85 } else { 0.0 };
            self.gut.world_mut().set_controls(c);
            self.seal(loading);
            self.shot = (t + HOP_S < SHOT_LOAD_S + SHOT_OPEN_S + 0.05).then_some(t + HOP_S);
            if self.shot.is_none() {
                self.seal(self.clench);
            }
            return;
        }
        if self.jitter > 0.0 {
            let u = self.rand();
            let mut c = self.gut.world().controls();
            c.pitch_hz = self.base_tension * ((2.0 * u - 1.0) * self.jitter / 12.0).exp2();
            self.gut.world_mut().set_controls(c);
        }
    }
}

impl Source for FartSource {
    fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> Result<(), String> {
        let mut i = 0;
        while i < left.len() {
            if self.phase == 0 {
                self.hop();
            }
            let n = (self.hop - self.phase).min(left.len() - i);
            self.gut.world_mut().process(&mut left[i..i + n]).map_err(|e| e.to_string())?;
            for v in &mut left[i..i + n] {
                let x = f64::from(*v);
                self.y1 = x - self.x1 + self.r * self.y1;
                self.x1 = x;
                *v = self.y1 as f32;
            }
            self.phase = (self.phase + n) % self.hop;
            i += n;
        }
        right.copy_from_slice(left);
        Ok(())
    }

    fn event(&mut self, index: usize, a: &[f64; MAX_ARGS], _at: u64, _now: u64) -> Result<(), String> {
        match index {
            0 => {
                let s = *STYLES.get(a[0].round() as usize).ok_or("no such style")?;
                self.gut.world_mut().set_vibrato(s.vibrato);
                self.style = Some((s, 0.0));
                self.shot = None;
                Ok(())
            }
            1 => {
                let f = *FOODS.get(a[0].round() as usize).ok_or("no such food")?;
                self.gut.eat(f);
                Ok(())
            }
            2 => {
                self.style = None;
                self.shot = Some(0.0);
                Ok(())
            }
            3 => {
                self.style = None;
                self.shot = None;
                let mut c = self.gut.world().controls();
                c.breath = 0.0;
                self.gut.world_mut().set_controls(c);
                self.seal(self.clench);
                Ok(())
            }
            _ => Err(format!("fart: no event {index}")),
        }
    }

    fn set(&mut self, index: usize, value: f64) -> Result<(), String> {
        let w = self.gut.world_mut();
        let mut c: TractControls = w.controls();
        match index {
            0 => c.breath = value,
            1 => c.articulation = fart::cheeks(value),
            2 => c.tenseness = value,
            3 => {
                c.pitch_hz = value;
                self.base_tension = value;
            }
            4 => {
                self.clench = value >= 0.5;
                let clench = self.clench;
                self.seal(clench);
                return Ok(());
            }
            5 => {
                self.rasp = value;
                w.set_constriction(RASP_SLOT, (value > 0.0).then_some(Constriction { position: RASP_AT, degree: value }));
                return Ok(());
            }
            6..=8 => {
                let mut v: Vibrato = w.vibrato();
                match index {
                    6 => v.wobble = value,
                    7 => v.depth_semitones = value,
                    _ => v.rate_hz = value,
                }
                w.set_vibrato(v);
                return Ok(());
            }
            9 => {
                self.jitter = value;
                return Ok(());
            }
            10 => {
                self.digest = value >= 0.5;
                return Ok(());
            }
            11 => {
                self.gut.house_w = value;
                return Ok(());
            }
            _ => return Err(format!("fart: no control {index}")),
        }
        w.set_controls(c);
        Ok(())
    }

    fn ledger(&self) -> Ledger {
        Ledger::of(self.gut.world().world())
    }

    fn receipt(&self) -> Value {
        let w = self.gut.world();
        let st = w.stats();
        json!({
            "anatomy": self.anatomy,
            "anatomyName": w.anatomy().name,
            "planDigest": w.world().plan_digest(),
            "listenerGain": LISTENER_GAIN,
            "dcBlockHz": DC_BLOCK_HZ,
            "gasFraction": self.gut.gas_fraction(),
            "stomach": self.gut.stomach().iter().map(|(f, left)| json!({"food": f.name, "leftJ": left})).collect::<Vec<_>>(),
            "moves": st.moves, "valveRetunes": st.valve_retunes, "valveWork": st.valve_work,
            "safetyLimited": w.observers().safety_limited,
        })
    }

    fn telemetry(&self, out: &mut [f64]) {
        let w = self.gut.world();
        out[0] = self.gut.gas_fraction();
        out[1] = self.gut.stomach().iter().map(|(_, l)| l).sum();
        out[2] = w.observers().measured_f0_hz;
        out[3] = w.world().space_body(0).map_or(0.0, |b| b.observation().valve.supply_pressure_pa);
        out[4] = f64::from(u8::from(self.style.is_some() || self.shot.is_some()));
    }

    fn reset(&mut self) -> Result<(), String> {
        *self = Self::new(self.anatomy.clone(), self.sample_rate, 0.0)?;
        Ok(())
    }

    fn clamped(&self) -> u64 {
        self.gut.world().observers().safety_limited
    }

    fn plan_digest(&self) -> String {
        self.gut.world().world().plan_digest().to_owned()
    }
}
