//! The tab family: the steel-string guitar and every other tab anatomy on
//! main (the film's gate, waiting courses, glass bells, fleas and drums;
//! Sunskip's paired courses, bass, chord courses and modal percussion). All
//! are `mus_engine::ariadne_live::LiveTab`: `mus render`'s World, plan, hands
//! and touch laws, touched live. A score (MUS, annotated tab) plays on it as
//! `mus render` plays it.

use mus_engine::ariadne_live::{LiveBow, LiveTab, TabHand, TabSpec, TabTouch};
use serde_json::{json, Value};

use super::{midi_of, opt, velocity_db};
use crate::catalog::*;
use crate::{CreateCtx, Source, MAX_ARGS};


/// One tab anatomy as the portable module carries it.
struct Anatomy {
    id: &'static str,
    title: &'static str,
    summary: &'static str,
    /// The plan (`None`: mus-engine's built-in `steel-string-v0`).
    plan: Option<&'static str>,
    /// `anatomy=` values in a score that mean this plan (file names).
    names: &'static [&'static str],
    /// `tuning=`, lowest course first (the design file's `scoreTuning`).
    tuning: &'static str,
    /// (pos, pick, pm, gain dB) per string, string 1 first; one entry: every string.
    hands: &'static [(f64, f64, f64, f64)],
    /// What each string is, string 1 first.
    elements: &'static [&'static str],
    /// Touches move the stop (frets 0..max_fret); else `@string:0` only.
    fretted: bool,
    max_fret: f64,
    /// Its cells are strings (`sin(n pi x)` shapes): the bow applies.
    bowable: bool,
    touch: &'static str,
    spec: &'static str,
    does_not_model: &'static [&'static str],
}

const STEEL_LIMITS: &[&str] = &[
    "No damping at note-off: a note rings until its string is touched again or its own losses and the palm take it ([stac] and lifted fingers cannot be heard).",
    "Each string has one pluck point, its hand's (set at create): a per-touch pluck point is not modelled.",
    "24 partials per string: the low E reaches about 2 kHz, the high E about 8 kHz.",
    "The back, ribs and neck are rigid; bracing is folded into the plate's stiffness; radiation is monopole-weighted with no directivity.",
    "Linear: no tension modulation, no fret buzz.",
    "High frequencies cross the bridge weakly (the band-rising bridge rate is a fit, not a law): the guitar is warm.",
    "Harmonics are played as ordinary plucks.",
    "The bow: a slow attack (0.25-0.7 s before Helmholtz motion locks), secondary slips below speed 0.4, pitch flattening at heavy force; a velocity source with no bow mass, hair compliance or width, no torsion; one bow per string at one point.",
    "It is a guitar-shaped Ariadne body, not a model of any particular guitar.",
];

const FICTIONAL: &str = "Everything in the steel-string's \"Limits of v0\" applies: no note-off damping, one touch point per cell, 24 modes per body, a linear model. A fictional design: every changed number is a design hypothesis in its *.design.json.";

fn anatomies() -> Vec<Anatomy> {
    vec![
        Anatomy {
            id: "steel-string",
            title: "Steel-string guitar",
            summary: "anatomy=steel-string-v0: six strings, a spruce top with its soundhole, the air in the box and a small room on one World, plucked (or bowed) by hands.",
            plan: None,
            names: &["steel-string-v0"],
            tuning: "E2 A2 D3 G3 B3 E4",
            hands: &[(0.18, 0.4, 0.0, 0.0)],
            elements: &["string 1 (E4)", "string 2 (B3)", "string 3 (G3)", "string 4 (D3)", "string 5 (A2)", "string 6 (E2)"],
            fretted: true,
            max_fret: 12.0,
            bowable: true,
            touch: "pluck",
            spec: "SPEC-ARIADNE-STEEL-STRING.md",
            does_not_model: STEEL_LIMITS,
        },
        Anatomy {
            id: "gate",
            title: "The Gate (Before the Law)",
            summary: "Six struck iron parts (panel, bars, bolt, lock plate, chain) driving a door leaf in a stone room. Fixed geometry: strike an element.",
            plan: Some(include_str!("../../../../../examples/ensemble/before-the-law/gate.plan.json")),
            names: &["gate.plan.json"],
            tuning: "D2 Ab2 Eb3 F5 Bb4 A6",
            hands: &[(0.03, 1.00, 0.0, -4.0), (0.05, 1.00, 0.0, -6.0), (0.11, 0.90, 0.0, -2.0), (0.04, 0.95, 0.0, -3.0), (0.04, 0.95, 0.0, -2.0), (0.07, 0.75, 0.0, 0.0)],
            elements: &["chain (A6)", "lock plate (Bb4)", "bolt (F5)", "cross bar (Eb3)", "long bar (Ab2)", "panel (D2)"],
            fretted: false,
            max_fret: 0.0,
            bowable: false,
            touch: "strike",
            spec: "examples/ensemble/before-the-law/README.md#gate",
            does_not_model: &[FICTIONAL, "No nonlinearity: a fff slam is a louder pp clang (no buzz, no rattle against the frame, no pitch change with amplitude).", "No contact between parts (the chain links do not collide; their high loss stands in), no hinge, no scrape.", "The door leaf is sampled at 72 of its thousands of modes; the bars' fundamentals radiate weakly."],
        },
        Anatomy {
            id: "waiting",
            title: "The Waiting (Before the Law)",
            summary: "Five low plucked courses D1 A1 D2 G2 C3 on a large body (the steel-string box scaled), each fretted over an octave.",
            plan: Some(include_str!("../../../../../examples/ensemble/before-the-law/waiting.plan.json")),
            names: &["waiting.plan.json"],
            tuning: "D1 A1 D2 G2 C3",
            hands: &[(0.18, 0.25, 0.45, 0.0), (0.18, 0.25, 0.45, 0.0), (0.18, 0.25, 0.45, 0.0), (0.18, 0.25, 0.45, 0.0), (0.22, 0.10, 0.0, -2.0)],
            elements: &["course C3", "course G2", "course D2", "course A1", "course D1 (the sub)"],
            fretted: true,
            max_fret: 12.0,
            bowable: true,
            touch: "pluck",
            spec: "examples/ensemble/before-the-law/README.md#waiting",
            does_not_model: &[FICTIONAL, "The body is a scaled guitar box, not a contrabass: its 80-87 Hz and 147-152 Hz modes colour the attack of every low note.", "Bowed only through the steel-string's bow (its limits apply)."],
        },
        Anatomy {
            id: "glass-bells",
            title: "The Light, bells (Before the Law)",
            summary: "Eight undercut glass bars D4..A5 over tuned tubes, in a case: a pure, celesta-like tone falling about 50 dB in 1-1.5 s.",
            plan: Some(include_str!("../../../../../examples/ensemble/before-the-law/light.plan.json")),
            names: &["light.plan.json"],
            tuning: "D4 F4 G4 A4 C5 D5 F5 A5",
            hands: &[(0.42, 0.80, 0.0, 0.0)],
            elements: &["bar A5", "bar F5", "bar D5", "bar C5", "bar A4", "bar G4", "bar F4", "bar D4"],
            fretted: false,
            max_fret: 0.0,
            bowable: false,
            touch: "strike",
            spec: "examples/ensemble/before-the-law/README.md#light-bells",
            does_not_model: &[FICTIONAL, "The undercut 1:4:10 tuning is a design choice; everything above the third bending mode is invented.", "A strike cannot choose its point on the bar per note; the decay is shorter than a vibraphone's."],
        },
        Anatomy {
            id: "fleas",
            title: "The Fleas (Before the Law)",
            summary: "Six invented clicks C6..Bb6 in a fur collar with a brass clasp: clicks at 1.6-6 kHz, gone within about 100 ms. Very quiet (0.13 % of the work radiated).",
            plan: Some(include_str!("../../../../../examples/ensemble/before-the-law/fleas.plan.json")),
            names: &["fleas.plan.json"],
            tuning: "C6 D6 E6 F#6 Ab6 Bb6",
            hands: &[(0.03, 1.00, 0.0, 0.0)],
            elements: &["flea Bb6", "flea Ab6", "flea F#6", "flea E6", "flea D6", "flea C6"],
            fretted: false,
            max_fret: 0.0,
            bowable: false,
            touch: "strike",
            spec: "examples/ensemble/before-the-law/README.md#fleas",
            does_not_model: &[FICTIONAL, "No law at all: an invented click. The fur is a lossy modal body, not a material model.", "Nothing hops: a flea landing on the clasp is a fixed coupling weight."],
        },
        Anatomy {
            id: "slam-drums",
            title: "The Slam: kettledrums and drums (Before the Law)",
            summary: "Two pedal kettledrums (D2, A2; the fret is the pedal, 0..7, a fifth), a bass drum (A1) and a field drum (G3) on a wooden riser in a stone hall.",
            plan: Some(include_str!("../../../../../examples/ensemble/before-the-law/slam.plan.json")),
            names: &["slam.plan.json"],
            tuning: "A1 D2 A2 G3",
            hands: &[(0.08, 0.85, 0.0, -4.0), (0.13, 0.30, 0.0, -1.0), (0.13, 0.30, 0.0, 0.0), (0.20, 0.20, 0.0, 0.0)],
            elements: &["field drum (G3)", "kettledrum (A2)", "kettledrum (D2)", "bass drum (A1)"],
            fretted: true,
            max_fret: 7.0,
            bowable: false,
            touch: "strike",
            spec: "examples/ensemble/before-the-law/README.md#slam",
            does_not_model: &[FICTIONAL, "No snare wires, no noise source: the field drum's roll is tom-like. No second head on the bass drum. The kettle's air is a ratio table, not a cavity.", "The touch pulse is at most 0.36 ms, much shorter than a felt mallet's contact: strikes are brighter than real timpani.", "Frets move only the kettledrums (their pedal); the bass and field drums are fixed geometry."],
        },
        Anatomy {
            id: "sunwire",
            title: "Sunwire (Sunskip)",
            summary: "Eight courses in four pairs, three reciprocal resonators, individually detuned paired modes: Sunskip's lead.",
            plan: Some(include_str!("../../../../../examples/ensemble/sunskip/sunwire.plan.json")),
            names: &["sunwire.plan.json"],
            tuning: "D3 D3 G3 G3 B3 B3 E4 E4",
            hands: &[(0.105, 0.68, 0.035, 0.0)],
            elements: &["course 1 (E4)", "course 2 (E4)", "course 3 (B3)", "course 4 (B3)", "course 5 (G3)", "course 6 (G3)", "course 7 (D3)", "course 8 (D3)"],
            fretted: true,
            max_fret: 12.0,
            bowable: true,
            touch: "pluck",
            spec: "examples/ensemble/sunskip/README.md",
            does_not_model: &[FICTIONAL, "Its numbers are parameter choices, not measurements of the coupled sounding pitch."],
        },
        Anatomy {
            id: "tumbler",
            title: "Tumbler (Sunskip)",
            summary: "Four lower courses E1 A1 D2 G2 on three rescaled resonators, the bass hand's palm loss: Sunskip's bass.",
            plan: Some(include_str!("../../../../../examples/ensemble/sunskip/tumbler.plan.json")),
            names: &["tumbler.plan.json"],
            tuning: "E1 A1 D2 G2",
            hands: &[(0.14, 0.60, 0.22, 0.0)],
            elements: &["course 1 (G2)", "course 2 (D2)", "course 3 (A1)", "course 4 (E1)"],
            fretted: true,
            max_fret: 12.0,
            bowable: true,
            touch: "pluck",
            spec: "examples/ensemble/sunskip/README.md",
            does_not_model: &[FICTIONAL],
        },
        Anatomy {
            id: "prism",
            title: "Prism (Sunskip)",
            summary: "Six courses C3 F3 A3 C4 E4 G4 with a different inharmonicity and loss recipe on three rescaled resonators: Sunskip's chords.",
            plan: Some(include_str!("../../../../../examples/ensemble/sunskip/prism.plan.json")),
            names: &["prism.plan.json"],
            tuning: "C3 F3 A3 C4 E4 G4",
            hands: &[(0.065, 0.88, 0.12, 0.0)],
            elements: &["course 1 (G4)", "course 2 (E4)", "course 3 (C4)", "course 4 (A3)", "course 5 (F3)", "course 6 (C3)"],
            fretted: true,
            max_fret: 12.0,
            bowable: true,
            touch: "pluck",
            spec: "examples/ensemble/sunskip/README.md",
            does_not_model: &[FICTIONAL],
        },
        Anatomy {
            id: "pocket",
            title: "Pocket modal percussion (Sunskip)",
            summary: "Six fixed modal percussion cells (low, tom, rim, side, bead, seed) with distinct inharmonic spectra on three resonators. Fixed geometry: strike a cell.",
            plan: Some(include_str!("../../../../../examples/ensemble/sunskip/pocket.plan.json")),
            names: &["pocket.plan.json"],
            tuning: "E2 A2 D3 G3 B3 E4",
            hands: &[(0.04, 0.83, 0.06, -5.0), (0.055, 0.97, 0.07, -5.0), (0.12, 0.70, 0.12, -6.0), (0.08, 0.91, 0.14, -7.0), (0.20, 0.32, 0.10, -3.0), (0.30, 0.16, 0.08, 0.0)],
            elements: &["seed (E4)", "bead (B3)", "side (G3)", "rim (D3)", "tom (A2)", "low (E2)"],
            fretted: false,
            max_fret: 0.0,
            bowable: false,
            touch: "strike",
            spec: "examples/ensemble/sunskip/README.md",
            does_not_model: &[FICTIONAL, "Its named cells are compositional roles: no real membrane, snare-wire or shaker is simulated."],
        },
    ]
}

/// Which built-in plan a score's `anatomy=` value means (by file name).
pub(crate) fn resolve(name: &str) -> Option<String> {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    anatomies().into_iter().find(|a| a.names.contains(&base) && a.plan.is_some()).and_then(|a| a.plan.map(str::to_owned))
}

pub fn kinds() -> Vec<KindDef> {
    anatomies().iter().map(def).collect()
}

fn def(a: &Anatomy) -> KindDef {
    let n = a.elements.len();
    let what = if a.touch == "strike" { "element" } else { "string" };
    let element_list = a.elements.iter().enumerate().map(|(i, e)| format!("{} = {e}", i + 1)).collect::<Vec<_>>().join(", ");
    let mut touch_args = vec![arg(what, "index", 1.0, n as f64, n as f64, &format!("1-based, string 1 the highest course (SPEC-TAB): {element_list}"))];
    if a.fretted {
        touch_args.push(arg("fret", "frets", 0.0, a.max_fret, 0.0, "stop above the capo; fractional is fine intonation (SPEC-TAB 4.1): fret k is configuration amount k/12"));
    }
    touch_args.push(arg("velocity", "0..1", 0.0, 1.0, 0.63, "the written dynamic, ppp (0) to fff (1) in dB (mus_audio.DYN_DB); 0.63 is mf"));
    let mut events = vec![event(
        a.touch,
        if a.touch == "strike" {
            "A raised-cosine contact pulse into the element through its contact port (the tab pluck's law): the World answers."
        } else {
            "A pluck: the fretting finger lands (if the stop moves) and a raised-cosine pulse drives the string's contact port (SPEC-ARIADNE-STEEL-STRING, Layer 3)."
        },
        touch_args.clone(),
    )];
    if a.fretted {
        let fret_args = |with_velocity: bool| {
            let mut v = vec![touch_args[0].clone(), touch_args[1].clone()];
            if with_velocity {
                v.push(touch_args[2].clone());
            }
            v
        };
        events.push(event("hammer-on", "The stop jumps under the ringing string, with a small sharp tap (0.3 of a pluck).", fret_args(true)));
        events.push(event("pull-off", "The stop jumps; the leaving finger plucks at half strength.", fret_args(true)));
        events.push(event(
            if a.id == "slam-drums" { "pedal-glide" } else { "slide" },
            "The stop travels to the new fret over 60 ms; the energy in the string is carried by transport and the work billed.",
            fret_args(false),
        ));
    }
    let mut controls = Vec::new();
    if a.bowable {
        events.push(event(
            "bow-land",
            "Land the bow on a string (SPEC-ARIADNE-STEEL-STRING, \"The bow\"): the hair lands over 10 ms and drags the string into stick-slip; it stays until bow-lift or force 0.",
            vec![
                arg("string", "index", 1.0, n as f64, 3.0_f64.min(n as f64), "the string the hair rests on"),
                arg("speed", "-1..1", -1.0, 1.0, 0.5, "bow speed; its sign is the direction (+ down-bow). 1 is the score's speed=1 (1.6e-5 in the string's units); mf is 0.5"),
                arg("force", "0..1", 0.0, 1.0, 0.5, "normal force (1 is 0.16 in the string's units); force = speed is the centre of the region where the string speaks"),
                arg("position", "fraction", 0.02, 0.49, 0.12, "bow point from the bridge (0.08-0.15 is a bow's place)"),
            ],
        ));
        events.push(event("bow-lift", "Lift the bow (over 10 ms); the string rings down on its own.", vec![]));
        controls.push(switch("bowString", None, n as f64, 0.0, "the string the bow rests on (0: none). Moving it lifts the hair and lands it on the new string"));
        controls.push(control("bowSpeed", "-1..1", -1.0, 1.0, 0.5, 60.0, "ramped by the scheduler between updates", "bow speed and direction (+ down-bow): loudness follows speed"));
        controls.push(control("bowForce", "0..1", 0.0, 1.0, 0.0, 60.0, "the hair lands and lifts over 10 ms (BOW_LANDING_SECONDS)", "normal force; 0 lifts the bow exactly (nothing stepped or booked)"));
        controls.push(control("bowPosition", "fraction", 0.02, 0.49, 0.12, 10.0, "a new point lifts the hair and lands it there (one bow per string, at one point)", "bow point from the bridge"));
    }
    let mut telemetry: Vec<SlotDef> = (1..=n).map(|s| slot(&format!("stop{s}"), "frets", &format!("where {what} {s}'s stop is (capo included)"))).collect();
    telemetry.push(slot("bowsDown", "count", "bows resting on a string with force"));
    for (i, e) in a.elements.iter().enumerate() {
        telemetry.push(slot(&format!("energy{}", i + 1), "J", &format!("{e}: stored energy (World units)")));
    }
    telemetry.push(slot("boxEnergy", "J", "the radiators' stored energy"));
    telemetry.push(slot("roomEnergy", "J", "the room's stored energy"));
    let tuning_default = json!(a.tuning);
    KindDef {
        id: a.id.into(),
        title: a.title.into(),
        family: "tab".into(),
        summary: a.summary.into(),
        spec: a.spec.into(),
        sample_rate: Some(mus_engine::ariadne_live::SAMPLE_RATE),
        channels: 2,
        anatomies: vec![anatomy(a.names[0], a.title, "the plan this kind carries", true)],
        open_anatomies: false,
        options: vec![
            option("tuning", "string", tuning_default, "each course's pitch, lowest first (tuning=). A course whose rounded first-mode pitch differs is retuned (its modes scaled)"),
            option("capo", "number", json!(0), "frets the capo shifts the base stop by"),
            option("hands", "array", json!(a.hands.iter().map(|h| json!({"pos": h.0, "pick": h.1, "pm": h.2, "gain": h.3})).collect::<Vec<_>>()), "per string (string 1 first; one entry for all): pos (0.02-0.5), pick (0 flesh..1 plectrum), pm (palm loss 0..1), gain (dB). Folded into the bodies when the World is built (Layer 2)"),
            option("score", "string", Value::Null, "MUS text (annotated tab): builds the World the score names (its tuning, capo and hands) and plays its touches as mus render does"),
            option("instrument", "string", Value::Null, "which of the score's tab instruments (abbrev; default the first)"),
        ],
        events,
        controls,
        telemetry,
        score: Some(ScoreDef {
            format: "mus".into(),
            about: format!("MUS-T (SPEC-TAB.md): an instrument with anatomy={} and its hands' tab notes. The World is rebuilt from the score (its tuning, capo, hands) and its touches scheduled as mus render times them: the World's observation equals mus render's World observation sample for sample.", a.names[0]),
        }),
        output: OutputDef {
            observes: "the stereo listener's observation of every body (World units)".into(),
            gain: LiveTab::OBSERVATION_GAIN,
            channels: "two listening directions".into(),
            about: "mus render mixes the same observation, times the same gain, into its bus; here there is no bus, master or limiter".into(),
        },
        lookahead_seconds: 0.008,
        cost: "7-11 bodies of 24 modes, 12-24 bridge edges and a room: 1.1-1.7x real time per World, measured in node (wasm, 128-frame quanta, tools/rtf.mjs) on a 2019 8-core i9 laptop; the worst 128-frame quantum can exceed its 2.67 ms budget, so one or two play live at a time on such a machine. Building one takes 6-46 ms".into(),
        does_not_model: lines(a.does_not_model),
        does_not_model_ref: a.spec.into(),
        build,
    }
}

struct TabSource {
    live: LiveTab,
    spec_id: &'static str,
    strings: usize,
    fretted: bool,
    bowable: bool,
    touch_events: usize,
    /// The instance frame at which `live`'s sample 0 sits.
    origin: u64,
    gain: f32,
    build: TabSpec,
    instrument: Option<String>,
}

fn anatomy_of(id: &str) -> Option<Anatomy> {
    anatomies().into_iter().find(|a| a.id == id)
}

fn parse_tuning(text: &str) -> Result<Vec<f64>, String> {
    text.split_whitespace().map(|n| midi_of(n).ok_or_else(|| format!("tuning note {n:?}"))).collect()
}

fn build(def: &KindDef, ctx: &CreateCtx) -> Result<Box<dyn Source>, String> {
    let a = anatomy_of(&def.id).ok_or("unknown tab anatomy")?;
    let o = ctx.options;
    let tuning = parse_tuning(opt::str_opt(o, "tuning")?.unwrap_or(a.tuning))?;
    let hands: Vec<TabHand> = match o.get("hands") {
        Some(Value::Array(list)) => list
            .iter()
            .map(|h| {
                Ok(TabHand {
                    pos: opt::f64_or(h, "pos", 0.18)?,
                    pick: opt::f64_or(h, "pick", 0.4)?,
                    pm: opt::f64_or(h, "pm", 0.0)?,
                    gain_db: opt::f64_or(h, "gain", 0.0)?,
                })
            })
            .collect::<Result<_, String>>()?,
        Some(Value::Null) | None => a.hands.iter().map(|h| TabHand { pos: h.0, pick: h.1, pm: h.2, gain_db: h.3 }).collect(),
        Some(v) => return Err(format!("option hands: expected an array, got {v}")),
    };
    let spec = TabSpec {
        anatomy: a.names[0].into(),
        plan_json: a.plan.map(str::to_owned),
        tuning: Some(tuning),
        capo: opt::f64_or(o, "capo", 0.0)?,
        hands,
    };
    let s = TabSource {
        live: LiveTab::new(&spec)?,
        spec_id: a.id,
        strings: a.elements.len(),
        fretted: a.fretted,
        bowable: a.bowable,
        touch_events: if a.fretted { 4 } else { 1 },
        origin: 0,
        gain: LiveTab::OBSERVATION_GAIN as f32,
        build: spec,
        instrument: opt::str_opt(o, "instrument")?.map(str::to_owned),
    };
    if s.live.string_count() != s.strings {
        return Err(format!("{}: the plan has {} strings, the catalog {}", a.id, s.live.string_count(), s.strings));
    }
    // A `score` option is played by the caller (ov_create, render_offline), so what reading it found lands in the receipt.
    Ok(Box::new(s))
}

impl TabSource {
    fn local(&self, frame: u64) -> usize {
        frame.saturating_sub(self.origin) as usize
    }
}

impl Source for TabSource {
    fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> Result<(), String> {
        self.live.process(left, right)?;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            *l *= self.gain;
            *r *= self.gain;
        }
        Ok(())
    }

    fn event(&mut self, index: usize, a: &[f64; MAX_ARGS], at: u64, _now: u64) -> Result<(), String> {
        let at = self.local(at);
        let string = a[0].round() as usize;
        let (fret, velocity) = if self.fretted { (a[1], a[2]) } else { (0.0, a[1]) };
        let db = velocity_db(velocity);
        match index {
            0 => self.live.touch(at, string, fret, db, TabTouch::Pluck),
            i if i < self.touch_events => {
                let kind = [TabTouch::HammerOn, TabTouch::PullOff, TabTouch::Slide][i - 1];
                self.live.touch(at, string, fret, db, kind)
            }
            i if self.bowable && i == self.touch_events => {
                // bow-land: string, speed, force, position
                self.live.set_bow(LiveBow { string: a[0].round() as usize, speed: a[1], force: a[2], position: a[3] })
            }
            i if self.bowable && i == self.touch_events + 1 => self.live.set_bow(LiveBow { force: 0.0, ..self.live.bow() }),
            _ => Err(format!("{}: no event {index}", self.spec_id)),
        }
    }

    fn set(&mut self, index: usize, value: f64) -> Result<(), String> {
        if !self.bowable {
            return Err(format!("{} has no controls", self.spec_id));
        }
        let mut b = self.live.bow();
        match index {
            0 => b.string = value.round().max(0.0) as usize,
            1 => b.speed = value,
            2 => b.force = value,
            3 => b.position = value,
            _ => return Err(format!("no control {index}")),
        }
        self.live.set_bow(b)
    }

    fn ledger(&self) -> Ledger {
        Ledger::of(self.live.world())
    }

    fn receipt(&self) -> Value {
        let mut r = self.live.receipt();
        r["originFrame"] = json!(self.origin);
        r
    }

    fn telemetry(&self, out: &mut [f64]) {
        let n = self.strings;
        for s in 0..n {
            out[s] = self.live.stop(s + 1);
        }
        out[n] = self.live.bows_down() as f64;
        let w = self.live.world();
        let bodies = self.live.bodies();
        let mut boxes = 0.0;
        for (i, id) in bodies.iter().enumerate() {
            let e = w.body_material_energy(i).unwrap_or(0.0);
            if let Some(k) = id.strip_prefix("string/").and_then(|k| k.parse::<usize>().ok()) {
                if (1..=n).contains(&k) {
                    out[n + k] = e;
                }
            } else {
                boxes += e;
            }
        }
        out[2 * n + 1] = boxes;
        out[2 * n + 2] = w.space_stored_energy();
    }

    fn reset(&mut self) -> Result<(), String> {
        self.live = LiveTab::new(&self.build)?;
        self.origin = 0;
        Ok(())
    }

    fn play_score(&mut self, text: &str, at: u64) -> Result<Value, String> {
        // A score names its World: rebuild it, starting now, with the score's first instant at `at`.
        let now = self.origin + self.live.now() as u64;
        let (live, read) = LiveTab::from_score_at(text, self.instrument.as_deref(), &resolve, at.saturating_sub(now) as usize)?;
        if live.string_count() != self.strings {
            return Err(format!("the score's instrument has {} strings; {} has {}", live.string_count(), self.spec_id, self.strings));
        }
        self.live = live;
        self.origin = now;
        Ok(json!({"format": "mus", "instrument": read.instrument, "samples": read.samples, "notes": read.notes, "badTokens": read.bad_tokens, "startFrame": at}))
    }

    fn lookahead(&self) -> u64 {
        (0.008 * mus_engine::ariadne_live::SAMPLE_RATE) as u64
    }

    fn plan_digest(&self) -> String {
        self.live.world().plan_digest().to_owned()
    }
}
