//! The bench (`ovillo/README.md`): a simulated guitar and guitarist whose DI
//! signal drives the canonical Ariadne World (`WorldPlan::string_brass_v1`),
//! `ovillo_web::Web` as the browser bench runs it. The guitar itself is a
//! signal model (waveguide strings, pickup-position combs, a coil-and-cable
//! resonance, hum), not a World; the World is what it drives. The bench's
//! monitor (master) is set to unity here: mixing is the consumer's. Its
//! runaway guard stays (it fades out, rests the World and fades back in when
//! the World's safety limiter holds most of the time; telemetry `guard`).

use ovillo_dsp::ariadne::PARAMS;
use ovillo_dsp::guitar::Pickup;
use ovillo_dsp::player::Pattern;
use ovillo_web::{slot as ws, Web};
use serde_json::{json, Value};

use super::opt;
use crate::catalog::*;
use crate::{CreateCtx, Source, MAX_ARGS};

fn patterns() -> Vec<Pattern> {
    Pattern::ALL.iter().copied().filter(|p| *p != Pattern::Score).collect()
}

pub fn kind() -> KindDef {
    let pats: Vec<&str> = patterns().iter().map(|p| p.id()).collect();
    let pickups: Vec<&str> = Pickup::ALL.iter().map(|p| p.label()).collect();
    let mut controls: Vec<ControlDef> = PARAMS
        .iter()
        .zip(ovillo_dsp::ariadne::AriadneControls::default().as_array())
        .map(|(p, d)| {
            control(p.id, "", p.min, p.max, d, 30.0, if p.slow { "the rig smooths it over 60 ms" } else { "the rig smooths it over 20 ms" }, &format!("{} (the Ariadne World plug-in's parameter)", p.name))
        })
        .collect();
    let patch: ovillo_dsp::ariadne::AriadneControls = serde_json::from_str(ovillo_web::DEFAULT_PATCH).unwrap_or_default();
    for (c, v) in controls.iter_mut().zip(patch.as_array()) {
        c.default = v;
    }
    controls.push(control("tempo", "BPM", 30.0, 240.0, 92.0, 2.0, "", "the guitarist's tempo"));
    controls.push(control("tone", "0..1", 0.0, 1.0, 1.0, 10.0, "", "the simulated guitar's tone knob (a signal model, outside the World)"));
    controls.push(control("volume", "0..1", 0.0, 1.0, 1.0, 10.0, "", "the simulated guitar's volume knob"));
    controls.push(switch("hum", Some(&["off", "on"]), 1.0, 1.0, "single-coil hum in the simulated DI"));
    KindDef {
        id: "bench".into(),
        title: "The bench".into(),
        family: "bench".into(),
        summary: "A simulated electric guitar and guitarist whose DI signal drives the canonical Ariadne World (a six-course string body and a B-flat brass bore on one configuration path, active material, a shared space). The guitar is a signal model, not a World.".into(),
        spec: "ovillo/README.md".into(),
        sample_rate: None,
        channels: 2,
        anatomies: vec![anatomy("string-brass-v1", "WorldPlan::string_brass_v1", "the canonical World; Material Organization moves it from the string body (0) to the brass bore (1)", true)],
        open_anatomies: false,
        options: vec![option("seed", "number", json!(1), "the guitarist's randomness (patterns never repeat)")],
        events: vec![
            event("pluck", "The simulated guitar's string is plucked (its DI drives the World).", vec![
                arg("course", "index", 0.0, 5.0, 0.0, "0 = low E2 .. 5 = high E4 (the bench's numbering, not SPEC-TAB's)"),
                arg("fret", "frets", 0.0, 22.0, 0.0, ""),
                arg("velocity", "0..1", 0.0, 1.0, 0.8, ""),
            ]),
            event("pattern", "What the simulated guitarist plays (patterns walk fixed tables with seeded randomness).", vec![arg_enum("name", &pats, 0, "")]),
            event("transport", "Start (the last pattern) or stop the guitarist; what rings, rings out.", vec![arg("on", "0/1", 0.0, 1.0, 1.0, "")]),
            event("readout", "Where the simulated guitar's DI is read from (its pickup setting: a signal-model choice of the guitar, outside the World; named a readout here per ARIADNE-PURE-PHYSICS-CONSTITUTION 9.3).", vec![arg_enum("position", &pickups, 0, "")]),
            event("mute-strings", "Mute the simulated strings.", vec![]),
            event("reset-world", "The World at rest (body, reservoir, space).", vec![]),
        ],
        controls,
        telemetry: vec![
            slot("material", "", "the World's material energy"),
            slot("reservoir", "", "its reservoir"),
            slot("space", "", "the shared space's stored energy"),
            slot("configuration", "0..1", "where on the configuration path"),
            slot("guard", "state", "0 ok, 1 fading out, 2 resting, 3 fading in"),
            slot("limiting", "0..1", "fraction of the guard's window the World's safety limiter held"),
            slot("dryRms", "", "the simulated DI's level"),
            slot("sounding", "bits", "bit i: course i sounding"),
            slot("bpm", "BPM", ""),
            slot("pattern", "index", "into the pattern list"),
        ],
        score: Some(ScoreDef { format: "bench-json".into(), about: "a bench score (ovillo/README.md, \"Scores\": events pluck/strum/mute/bend/vibrato/rest on beats; scores/first-light.json)".into() }),
        output: OutputDef {
            observes: "the World's stereo listener times Listener Scale, mixed with the DI by Mix, times Output, safety-limited at +-1 (the plug-in's own stages)".into(),
            gain: 1.0,
            channels: "two listening directions".into(),
            about: "the bench's master (monitor) is at unity; its runaway guard may fade the output (telemetry guard)".into(),
        },
        lookahead_seconds: 0.0,
        cost: "the canonical World plus six waveguide strings: 4.9x real time, measured in node (wasm, 128-frame quanta, tools/rtf.mjs) on a 2019 8-core i9 laptop".into(),
        does_not_model: lines(&[
            "The guitar is a signal model (waveguide strings, pickup-position combs, a coil-and-cable resonance under a tone knob, hum): not an Ariadne World and not under the pure-physics constitution; only what it drives is.",
            "The World's Output stage clamps at +-1 (the plug-in's safety limiter), and the runaway guard fades and rests the World when it is held there.",
            "The canonical World's controls are the plug-in's twenty parameters; their behaviour is the plug-in's (ovillo_dsp::ariadne, lifted line for line).",
        ]),
        does_not_model_ref: "ovillo/README.md".into(),
        build,
    }
}

struct BenchSource {
    web: Web,
    seed: u32,
    sample_rate: f64,
    tone: f64,
    volume: f64,
    hum: bool,
}

fn build(_def: &KindDef, ctx: &CreateCtx) -> Result<Box<dyn Source>, String> {
    let seed = opt::f64_or(ctx.options, "seed", 1.0)? as u32;
    let mut web = Web::new(ctx.sample_rate, seed)?;
    web.set_master_now(1.0, false);
    Ok(Box::new(BenchSource { web, seed, sample_rate: ctx.sample_rate, tone: 1.0, volume: 1.0, hum: true }))
}

impl Source for BenchSource {
    fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> Result<(), String> {
        let mut i = 0;
        while i < left.len() {
            let n = self.web.render(left.len() - i);
            left[i..i + n].copy_from_slice(&self.web.left[..n]);
            right[i..i + n].copy_from_slice(&self.web.right[..n]);
            i += n;
        }
        Ok(())
    }

    fn event(&mut self, index: usize, a: &[f64; MAX_ARGS], _at: u64, _now: u64) -> Result<(), String> {
        match index {
            0 => self.web.pluck(a[0].round().clamp(0.0, 5.0) as u8, a[1].round().clamp(0.0, 22.0) as u8, a[2] as f32),
            1 => {
                let p = *patterns().get(a[0].round() as usize).ok_or("no such pattern")?;
                self.web.set_pattern(p);
            }
            2 => self.web.transport(a[0] >= 0.5),
            3 => self.web.set_pickup(a[0].round() as usize),
            4 => self.web.mute_strings(),
            5 => self.web.reset_world(),
            _ => return Err(format!("bench: no event {index}")),
        }
        Ok(())
    }

    fn set(&mut self, index: usize, value: f64) -> Result<(), String> {
        let n = PARAMS.len();
        match index {
            i if i < n => {
                self.web.set_param(i, value);
            }
            i if i == n => self.web.set_tempo(value),
            i if i <= n + 3 => {
                match i - n {
                    1 => self.tone = value,
                    2 => self.volume = value,
                    _ => self.hum = value >= 0.5,
                }
                self.web.set_guitar(self.tone, self.volume, self.hum);
            }
            _ => return Err(format!("bench: no control {index}")),
        }
        Ok(())
    }

    fn ledger(&self) -> Ledger {
        Ledger::of(self.web.rig().ariadne().world())
    }

    fn receipt(&self) -> Value {
        let s = &self.web.slots;
        json!({
            "anatomy": "string-brass-v1",
            "planDigest": self.web.rig().plan_digest(),
            "seed": self.seed,
            "pattern": self.web.pattern().id(),
            "guardTrips": s[ws::GUARD_TRIPS],
            "safetyLimitedFrames": s[ws::SAFETY_LIMITED_FRAMES],
            "worldErrors": s[ws::WORLD_ERRORS],
            "simulatedGuitar": "a signal model (not a World): its DI drives the World",
        })
    }

    fn telemetry(&self, out: &mut [f64]) {
        let s = &self.web.slots;
        out[0] = s[ws::MATERIAL];
        out[1] = s[ws::RESERVOIR];
        out[2] = s[ws::SPACE];
        out[3] = s[ws::CONFIGURATION];
        out[4] = s[ws::GUARD];
        out[5] = s[ws::LIMITING];
        out[6] = s[ws::DRY_RMS];
        out[7] = s[ws::SOUNDING];
        out[8] = s[ws::BPM];
        out[9] = patterns().iter().position(|p| *p == self.web.pattern()).unwrap_or(0) as f64;
    }

    fn reset(&mut self) -> Result<(), String> {
        self.web = Web::new(self.sample_rate, self.seed)?;
        self.web.set_master_now(1.0, false);
        Ok(())
    }

    fn play_score(&mut self, text: &str, _at: u64) -> Result<Value, String> {
        let summary = self.web.load_score(text)?;
        Ok(json!({"format": "bench-json", "summary": summary}))
    }

    fn clamped(&self) -> u64 {
        self.web.slots[ws::SAFETY_LIMITED_FRAMES] as u64
    }

    fn plan_digest(&self) -> String {
        self.web.rig().plan_digest().to_owned()
    }
}
