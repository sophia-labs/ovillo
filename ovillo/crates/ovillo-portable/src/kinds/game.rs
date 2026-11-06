//! The game's in-world sources (`ovillo/crates/ovillo-game-sound`, branch
//! ovillo/game-sound, partial): standalone Worlds behind its `GameWorld`
//! trait. They are **not** `mus_ariadne` plans: each runs on that crate's own
//! power-normalised wave networks and modal bodies (`net.rs`) and books a
//! `mus_watsn::WorkLedger` with the same receipt fields; declared stochastic
//! source terms (seeded) are listed in each receipt.
//!
//! Any `GameWorld` becomes a kind here through [`GameSource`]: the catalog
//! entry names its controls and events (the trait's string names), and the
//! adapter forwards them on their frame. Adding one (maverick and explosion
//! are stubs today and left out) is one more entry in [`kinds`] and one arm
//! in `ovillo_game_sound::create`.

use ovillo_game_sound::GameWorld;
use serde_json::{json, Value};

use super::opt;
use crate::catalog::*;
use crate::{CreateCtx, Source, MAX_ARGS};

const SPEC: &str = "ovillo/crates/ovillo-game-sound (module headers); the request ~/dev/verabot/requests/2026-11-05-sound-sources.md";
const NOT_ARIADNE: &str = "Not a mus_ariadne plan: a standalone World on ovillo-game-sound's wave-network runtime, booking a mus_watsn WorkLedger (same receipt fields). Not under the steel-string/piano/tract specs.";

fn base(id: &str, title: &str, summary: &str, gain: f64, observes: &str) -> KindDef {
    KindDef {
        id: id.into(),
        title: title.into(),
        family: "game-sound".into(),
        summary: summary.into(),
        spec: SPEC.into(),
        sample_rate: None,
        channels: 1,
        anatomies: vec![],
        open_anatomies: false,
        options: vec![option("seed", "number", json!(1), "the World's declared stochastic sources are seeded: the same seed renders the same samples")],
        events: vec![],
        controls: vec![],
        telemetry: vec![],
        score: None,
        output: OutputDef { observes: observes.into(), gain, channels: "one point (left = right)".into(), about: "the World's own pressure over its OUTPUT_SCALE_PA; propagation delay, Doppler, distance, directivity toward the listener (except f16's declared aspect readout), ground and air are the engine's".into() },
        lookahead_seconds: 0.0,
        cost: String::new(),
        does_not_model: vec![],
        does_not_model_ref: SPEC.into(),
        build,
    }
}

pub fn kinds() -> Vec<KindDef> {
    let impulse = || arg("impulse", "0..1", 0.0, 1.0, 0.5, "how hard");
    let mut kart = base(
        "kart_engine",
        "Kart engine (~100 cc two-stroke)",
        "A single-cylinder two-stroke outdoors: cylinder and crankcase gas compliances driven by the piston, a flywheel held at rpm by a booked servo, a carburettor throat and reed, ports, a fitted Wiebe heat release, a seven-part expansion-chamber exhaust radiating at its mouth, shell modes, a struck chassis and five tread stick-slip oscillators.",
        1.0 / ovillo_game_sound::kart_engine::OUTPUT_SCALE_PA,
        "pressure at 1 m (Pa)",
    );
    kart.controls = vec![
        control("rpm", "rev/min", 1800.0, 14000.0, 6000.0, 60.0, "a prescribed speed held by a servo whose work is control work", "engine speed"),
        control("throttle", "0..1", 0.0, 1.0, 0.0, 60.0, "the carburettor throat's area", ""),
        control("load", "0..1", 0.0, 1.0, 0.5, 60.0, "", "0 engine braking .. 1 uphill/accelerating (the servo's work turns negative under braking)"),
        control("boost", "0..1", 0.0, 1.0, 0.0, 60.0, "", "blower pressure in series with the intake (drift or start boost)"),
        control("skid", "0..1", 0.0, 1.0, 0.0, 60.0, "", "tyre scrub: drives the tread-block stick-slip oscillators"),
        switch("surface", Some(&["track", "lakebed"]), 1.0, 0.0, "what the treads scrub on"),
    ];
    kart.events = vec![
        event("start", "A hand stops the flywheel, the rope is pulled: cranking, then it catches (or coughs).", vec![]),
        event("restart", "As start.", vec![]),
        event("bump", "Kart-on-kart or tyre wall: the chassis struck through a Hertz contact.", vec![impulse()]),
        event("land", "Landing: the chassis struck, softer and lower.", vec![impulse()]),
        event("hit", "The kart blown up: a clunk, then the servo lets go and the engine spins down (dead until restart).", vec![]),
    ];
    kart.telemetry = vec![slot("ignitions", "count", ""), slot("misfires", "count", "charges too weak to burn"), slot("revolutions", "count", ""), slot("peakCylinderGaugePa", "Pa", "")];
    kart.cost = "17.6x real time for one, measured in node (wasm, 128-frame quanta, tools/rtf.mjs) on a 2019 8-core i9 laptop (the request's ceiling is 8 at once; the game-sound branch measured 8 at once at about 5x)".into();
    kart.does_not_model = lines(&[
        NOT_ARIADNE,
        "No real thermodynamics: the gas is a linear compliance charged by a fitted burn law; no temperature, no mean flow in ducts, no convective Doppler.",
        "No crank/rod inertia beyond a flywheel, no valve or gear noise, no clutch or drivetrain, no tyre/ground in rolling.",
        "Heat release, port timing, areas, volumes and modal numbers are design choices, not measured from any engine.",
        "Declared stochastic source: ground roughness force on the tread blocks (seeded), only while skid > 0.",
    ]);
    let mut props = base(
        "race_props",
        "Race props: start-light buzzer and finish bell",
        "Buzzer: a spring-held armature dragged by a constant pull while a position-gated contact is closed (it self-oscillates), striking a plate and a panel through a Hertz contact. Bell: sixteen modes (eight beating pairs) struck by a clapper.",
        1.0 / ovillo_game_sound::race_props::OUTPUT_SCALE_PA,
        "pressure at the reference distance (Pa)",
    );
    props.events = vec![
        event("beep", "A countdown beep: the buzzer driven for 0.22 s (the same buzzer for 3, 2 and 1).", vec![arg("n", "count", 1.0, 3.0, 3.0, "3, 2, 1 (does not change the sound)")]),
        event("go", "The buzzer driven harder for 0.9 s.", vec![]),
        event("bell", "One stroke of the bell (the last lap).", vec![]),
        event("finish", "A swung clapper: nine strokes over about 3 s, each arriving a little slower.", vec![]),
    ];
    props.telemetry = vec![slot("armatureImpacts", "count", ""), slot("bellStrokes", "count", "")];
    props.cost = "two small modal bodies: 64x real time, measured in node (wasm, 128-frame quanta, tools/rtf.mjs) on a 2019 8-core i9 laptop".into();
    props.does_not_model = lines(&[
        NOT_ARIADNE,
        "No coil or circuit: the pull is a declared force whose work is the buzzer's only source; no contact bounce beyond the gate, no housing cavity.",
        "The bell's ratios are a design choice after the minor-third church bell series; the pairs' splits and the shapes at the strike point are invented; the clapper's swing is not a pendulum (its arrival speed is the control); no stand.",
    ]);
    let mut jet = base(
        "f16",
        "Turbofan flyover (probed only)",
        "A low-bypass afterburning turbofan heard from outside: inlet, core, turbine and nozzle ducts as wave networks, three blade rows as Gutin-style pressure sources, the reheat flame as a delayed volume source (Rayleigh feedback), two radiating mouths; jet and airframe noise as declared eddy banks. Probed, not auditioned (game-sound branch status).",
        1.0 / ovillo_game_sound::f16::OUTPUT_SCALE_PA,
        "pressure at 50 m (Pa)",
    );
    jet.controls = vec![
        control("thrust", "0..1", 0.0, 1.0, 0.5, 60.0, "the spool follows with a lag", "0.5 is MIL"),
        control("afterburner", "0..1", 0.0, 1.0, 0.0, 60.0, "", "reheat: the flame's gain, crackle rate"),
        control("airspeed", "m/s", 0.0, 300.0, 150.0, 60.0, "", "drives the airframe boundary-layer noise"),
        control("aspect", "-1..1", -1.0, 1.0, 0.0, 60.0, "a declared non-loading directivity readout", "-1 the listener in front of the jet, +1 behind it"),
    ];
    jet.telemetry = vec![slot("n1", "fraction", "the spool's speed"), slot("fanBladePassageHz", "Hz", ""), slot("jetVelocity", "m/s", "")];
    jet.cost = "9x real time, measured in node (wasm, 128-frame quanta, tools/rtf.mjs) on a 2019 8-core i9 laptop (the request wants exactly one)".into();
    jet.does_not_model = lines(&[
        NOT_ARIADNE,
        "\"F-16C, F100-PW-229 class\" is the request's label for the scale: nothing is measured from or fitted to that engine; every dimension, blade count and gain is a design choice.",
        "No mean flow in the ducts, one mixed stream (no bypass), no real blade counts or cut-off, no buzz-saw, no shock-cell noise; the spool is a prescribed speed (the cycle and shaft energy are outside the ledger).",
        "Declared stochastic source terms, not derived from the Navier-Stokes equations: jet mixing noise (Lighthill U^8, fitted eta), airframe noise (U^6), reheat crackle (Poisson pops); their energy is in the ledger.",
        "Probed only: no audition yet.",
    ]);
    vec![kart, props, jet]
}

/// Any `GameWorld` as a portable source: catalog names forwarded to the trait.
pub struct GameSource {
    world: Box<dyn GameWorld>,
    kind: &'static KindDef,
    seed: u64,
    sample_rate: f64,
    /// The World's receipt, re-read every `TELEMETRY_EVERY` frames for telemetry.
    cached: Value,
    since: usize,
}

const TELEMETRY_EVERY: usize = 2048;

fn build(def: &KindDef, ctx: &CreateCtx) -> Result<Box<dyn Source>, String> {
    let seed = opt::f64_or(ctx.options, "seed", 1.0)?.max(0.0) as u64;
    let world = ovillo_game_sound::create(&def.id, ctx.sample_rate, seed).ok_or_else(|| format!("ovillo-game-sound has no World {}", def.id))?;
    let kind = super::find(&def.id).ok_or("unregistered game kind")?;
    let cached = world.receipt();
    Ok(Box::new(GameSource { world, kind, seed, sample_rate: ctx.sample_rate, cached, since: 0 }))
}

fn num(v: &Value, key: &str) -> f64 {
    v.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

impl Source for GameSource {
    fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> Result<(), String> {
        self.world.render(left);
        right.copy_from_slice(left);
        self.since += left.len();
        if self.since >= TELEMETRY_EVERY {
            self.since = 0;
            self.cached = self.world.receipt();
        }
        Ok(())
    }

    fn event(&mut self, index: usize, a: &[f64; MAX_ARGS], _at: u64, _now: u64) -> Result<(), String> {
        let def = self.kind.events.get(index).ok_or_else(|| format!("{}: no event {index}", self.kind.id))?;
        let arg = if def.args.is_empty() { 0.0 } else { a[0] };
        if self.world.event(&def.name, arg) {
            Ok(())
        } else {
            Err(format!("{} refused event {}", self.kind.id, def.name))
        }
    }

    fn set(&mut self, index: usize, value: f64) -> Result<(), String> {
        let def = self.kind.controls.get(index).ok_or_else(|| format!("{}: no control {index}", self.kind.id))?;
        let ok = match &def.values {
            // A text control (surface): its value's name.
            Some(values) => self.world.set_text(&def.name, values.get(value.round() as usize).map_or("", String::as_str)),
            None => self.world.set(&def.name, value),
        };
        if ok {
            Ok(())
        } else {
            Err(format!("{} refused control {}", self.kind.id, def.name))
        }
    }

    fn ledger(&self) -> Ledger {
        let r = self.world.receipt();
        let l = &r["ledger"];
        Ledger {
            source_work: num(l, "sourceWork"),
            control_work: num(l, "controlWork"),
            internal_loss: num(l, "internalLoss"),
            radiation: num(l, "radiation"),
            initial_energy: num(l, "initialEnergy"),
            current_energy: num(l, "finalEnergy"),
            cumulative_closure_residual: num(l, "cumulativeClosureResidual"),
            maximum_frame_residual: num(l, "maximumFrameResidual"),
            global_closure_residual: num(l, "globalResidual"),
            sample_frames: l.get("frames").and_then(Value::as_u64).unwrap_or(0),
        }
    }

    fn receipt(&self) -> Value {
        let mut r = self.world.receipt();
        r["seed"] = json!(self.seed);
        r["runtime"] = json!("ovillo-game-sound (not a mus_ariadne plan)");
        r
    }

    fn telemetry(&self, out: &mut [f64]) {
        let r = &self.cached;
        let fields: &[&str] = match self.kind.id.as_str() {
            "kart_engine" => &["ignitions", "misfires", "revolutions", "peakCylinderGaugePa"],
            "race_props" => &["armatureImpacts", "bellStrokes"],
            "f16" => &["spoolN1Fraction", "fanBladePassageHz", "jetVelocityMs"],
            _ => &[],
        };
        for (o, f) in out.iter_mut().zip(fields) {
            *o = num(r, f);
        }
    }

    fn reset(&mut self) -> Result<(), String> {
        self.world = ovillo_game_sound::create(&self.kind.id, self.sample_rate, self.seed).ok_or("rebuild")?;
        self.cached = self.world.receipt();
        self.since = 0;
        Ok(())
    }

    fn plan_digest(&self) -> String {
        format!("none (not a plan): ovillo-game-sound {} seed {}", self.kind.id, self.seed)
    }
}
