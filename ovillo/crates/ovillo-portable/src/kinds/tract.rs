//! The vocal tract (`TRACT-WORLD.md`): `ovillo_tract_world::TractWorldNd`, a
//! `mus_ariadne::World` with one N-D body (tongue index, tongue diameter,
//! lips, velum and five constriction slots) whose 24 modes are computed from
//! an oral and a nasal tube network, voiced by the runtime's flow valve and
//! paid from the lungs.
//!
//! Anatomies: the public engine, the checkpoint's two, the choir's six voice
//! types (`tract_choir.rs` `VOICES`) and its twelve singers (the cast in
//! `examples/ensemble/before-the-law/choir/singers.tsv`, each with the pitch
//! table it learned by singing its own World).

use std::collections::BTreeMap;

use mus_ariadne::{ConnectionPolicy, DisposalPolicy};
use ovillo_tract_world::abi::{FULL_SCHEDULE, LIGHT_SCHEDULE};
use ovillo_tract_world::gestures::Gesture;
use ovillo_tract_world::world::{self as tw, Articulation, Constriction, TractControls, TractWorldNd, Vibrato};
use ovillo_tract_world::TractNdAnatomy;
use serde_json::{json, Value};

use super::opt;
use crate::catalog::*;
use crate::{CreateCtx, Source, MAX_ARGS};

const GESTURES: [&str; 12] = ["mama", "aiua", "sss", "shh", "fff", "zzz", "vvv", "pa", "ta", "ka", "papapa", "stop"];
const PRESETS: [&str; 4] = ["a", "i", "u", "rest"];
const VOWELS: [&str; 5] = ["a", "i", "u", "m", "o"];
/// A singer moves from one vowel to the next over this long (`tract_choir.rs` ART_GLIDE_S).
const ART_GLIDE_S: f64 = 0.20;

/// `tract_choir.rs` `VOICES`, copied (design hypotheses there): name, oral and
/// nasal length (m), fold length, thickness (m), mass (kg), tuning range
/// (Hz), lungs (J, W).
const VOICES: [(&str, f64, f64, f64, f64, f64, f64, f64, f64, f64); 6] = [
    ("bass", 0.170, 0.118, 0.016, 0.0033, 1.8e-4, 45.0, 340.0, 3.4, 0.32),
    ("baritone", 0.163, 0.114, 0.015, 0.0031, 1.6e-4, 55.0, 400.0, 3.2, 0.30),
    ("tenor", 0.155, 0.108, 0.013, 0.0029, 1.3e-4, 70.0, 520.0, 3.0, 0.30),
    ("alto", 0.142, 0.099, 0.011, 0.0026, 0.95e-4, 110.0, 760.0, 2.4, 0.25),
    ("soprano", 0.136, 0.095, 0.0095, 0.0024, 0.75e-4, 150.0, 1100.0, 2.2, 0.24),
    ("child", 0.118, 0.082, 0.0075, 0.0021, 0.5e-4, 170.0, 1000.0, 1.4, 0.16),
];

const SINGERS_TSV: &str = include_str!("../../../../../examples/ensemble/before-the-law/choir/singers.tsv");
const SKILLS: [(&str, &str); 12] = [
    ("bass1", include_str!("../../../../../examples/ensemble/before-the-law/choir/skill/bass1.tsv")),
    ("bass2", include_str!("../../../../../examples/ensemble/before-the-law/choir/skill/bass2.tsv")),
    ("bari1", include_str!("../../../../../examples/ensemble/before-the-law/choir/skill/bari1.tsv")),
    ("bari2", include_str!("../../../../../examples/ensemble/before-the-law/choir/skill/bari2.tsv")),
    ("ten1", include_str!("../../../../../examples/ensemble/before-the-law/choir/skill/ten1.tsv")),
    ("ten2", include_str!("../../../../../examples/ensemble/before-the-law/choir/skill/ten2.tsv")),
    ("alto1", include_str!("../../../../../examples/ensemble/before-the-law/choir/skill/alto1.tsv")),
    ("alto2", include_str!("../../../../../examples/ensemble/before-the-law/choir/skill/alto2.tsv")),
    ("sop1", include_str!("../../../../../examples/ensemble/before-the-law/choir/skill/sop1.tsv")),
    ("sop2", include_str!("../../../../../examples/ensemble/before-the-law/choir/skill/sop2.tsv")),
    ("child1", include_str!("../../../../../examples/ensemble/before-the-law/choir/skill/child1.tsv")),
    ("child2", include_str!("../../../../../examples/ensemble/before-the-law/choir/skill/child2.tsv")),
];

/// A singer of the cast (`singers.tsv`).
#[derive(Debug, Clone)]
struct Singer {
    name: String,
    voice: usize,
    tract_scale: f64,
    fold_scale: f64,
    vib_rate_hz: f64,
    wobble: f64,
    intonation_cents: f64,
    breath_scale: f64,
    tongue_offset: f64,
    lips_offset: f64,
}

fn singers() -> Vec<Singer> {
    let mut out = Vec::new();
    let mut header: Vec<&str> = Vec::new();
    for line in SINGERS_TSV.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let f: Vec<&str> = line.split('\t').collect();
        if f[0] == "singer" {
            header = f;
            continue;
        }
        let get = |k: &str| header.iter().position(|h| *h == k).and_then(|i| f.get(i)).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
        let Some(voice) = VOICES.iter().position(|v| f.get(1) == Some(&v.0)) else { continue };
        out.push(Singer {
            name: f[0].to_owned(),
            voice,
            tract_scale: get("tract_scale"),
            fold_scale: get("fold_scale"),
            vib_rate_hz: get("vib_rate_hz"),
            wobble: get("wobble"),
            intonation_cents: get("intonation_cents"),
            breath_scale: get("breath_scale"),
            tongue_offset: get("tongue_offset"),
            lips_offset: get("lips_offset"),
        });
    }
    out
}

/// `tract_choir.rs` `Singer::anatomy`: a voice type on `public_v1`, scaled.
fn voice_anatomy(voice: usize, tract_scale: f64, fold_scale: f64) -> TractNdAnatomy {
    let v = VOICES[voice];
    let mut a = TractNdAnatomy::public_v1();
    a.oral_length_m = v.1 * tract_scale;
    a.nasal_length_m = v.2 * tract_scale;
    a.valve.length_m = v.3 * fold_scale;
    a.valve.thickness_m = v.4 * fold_scale;
    a.valve.mass_kg = v.5 * fold_scale.powi(3);
    a.valve.min_tuning_hz = v.6;
    a.valve.max_tuning_hz = v.7;
    a.valve.initial_tuning_hz = (v.6 * v.7).sqrt();
    a.lung_capacity_j = v.8;
    a.lung_refill_w = v.9;
    a
}

/// `tract_choir.rs` `vowel`: the public vowels, `m` a hum, `o` the unfitted midpoint of a and u.
fn vowel(c: usize) -> Articulation {
    let a = tw::PUBLIC_ART_A;
    let u = tw::PUBLIC_ART_U;
    match c {
        1 => tw::PUBLIC_ART_I,
        2 => u,
        3 => tw::PUBLIC_ART_M,
        4 => Articulation {
            tongue_index: 0.5 * (a.tongue_index + u.tongue_index),
            tongue_diameter: 0.5 * (a.tongue_diameter + u.tongue_diameter),
            lips: 0.5 * (a.lips + u.lips),
            velum: 0.5 * (a.velum + u.velum),
        },
        _ => a,
    }
}

/// A singer's pitch table: per vowel, (fold tuning, sounding Hz) rows (`tract_choir.rs` `SkillTable`).
struct SkillTable(BTreeMap<char, Vec<(f64, f64)>>);

impl SkillTable {
    fn parse(text: &str) -> Self {
        let mut t: BTreeMap<char, Vec<(f64, f64)>> = BTreeMap::new();
        for line in text.lines().filter(|l| !l.starts_with('#') && !l.starts_with("vowel") && !l.trim().is_empty()) {
            let f: Vec<&str> = line.split('\t').collect();
            if let (Some(v), Some(Ok(tuning)), Some(Ok(sounding))) = (f[0].chars().next(), f.get(1).map(|x| x.parse::<f64>()), f.get(2).map(|x| x.parse::<f64>())) {
                if sounding > 0.0 {
                    t.entry(v).or_default().push((tuning, sounding));
                }
            }
        }
        Self(t)
    }

    fn tuning_for(&self, vowel: char, hz: f64) -> Result<f64, String> {
        let rows = self.0.get(&vowel).ok_or_else(|| format!("no calibration for vowel {vowel}"))?;
        for w in rows.windows(2) {
            let ((t0, s0), (t1, s1)) = (w[0], w[1]);
            if s1 > s0 && s0 <= hz && hz <= s1 {
                let k = (hz.ln() - s0.ln()) / (s1.ln() - s0.ln());
                return Ok((t0.ln() + k * (t1.ln() - t0.ln())).exp());
            }
        }
        let (lo, hi) = self.range(vowel);
        Err(format!("{hz:.1} Hz on /{vowel}/ is outside what the calibration heard this singer sing ({lo:.1}-{hi:.1} Hz)"))
    }

    fn range(&self, vowel: char) -> (f64, f64) {
        let rows = self.0.get(&vowel).map_or(&[][..], Vec::as_slice);
        let (mut lo, mut hi) = (f64::INFINITY, 0.0f64);
        for w in rows.windows(2) {
            if w[1].1 > w[0].1 {
                lo = lo.min(w[0].1);
                hi = hi.max(w[1].1);
            }
        }
        (lo, hi)
    }
}

pub fn kind() -> KindDef {
    let mut anatomies = vec![
        anatomy("public-v1", "the public tract", "TractNdAnatomy::public_v1: adult_v0 on the tube tract's grid, POC-3's air (jet, turbulence, contact), five constriction slots, the lip seal. The site's tract.", true),
        anatomy("adult-v0", "the checkpoint tract", "TractNdAnatomy::adult_v0(44, 31): linear acoustics, no constriction slots (consonant gestures do not hiss), CHECKPOINT-2A", false),
        anatomy("adult-v0-air", "the checkpoint tract with air", "adult_v0 with POC-3's air, no constriction slots", false),
    ];
    for v in VOICES {
        anatomies.push(anatomy(v.0, &format!("{} voice type", v.0), &format!("tract_choir.rs VOICES: public_v1 with oral/nasal {:.3}/{:.3} m, fold {:.1} x {:.1} mm, {:.3} g, tuning {}-{} Hz, lungs {} J / {} W (design hypotheses)", v.1, v.2, v.3 * 1e3, v.4 * 1e3, v.5 * 1e3, v.6, v.7, v.8, v.9), false));
    }
    for s in singers() {
        let sk = SKILLS.iter().find(|x| x.0 == s.name).map(|x| SkillTable::parse(x.1));
        let range = sk.map(|t| t.range('a')).unwrap_or((0.0, 0.0));
        anatomies.push(anatomy(&s.name, &format!("singer {} ({})", s.name, VOICES[s.voice].0), &format!("the Before the Law choir's {}: tract x{}, fold x{}, intonation {:+} cents; sings /a/ {:.0}-{:.0} Hz from its own pitch table (the `sing` event)", s.name, s.tract_scale, s.fold_scale, s.intonation_cents, range.0, range.1), false));
    }
    KindDef {
        id: "tract".into(),
        title: "Vocal tract".into(),
        family: "tract".into(),
        summary: "After Neil Thapen's Pink Trombone (2017), implemented independently as a mus_ariadne::World: an oral and a nasal tube network whose 24 modes are recomputed at every articulation, tongue, lips and velum as coordinates, a flow valve at the glottis paid from the lungs.".into(),
        spec: "TRACT-WORLD.md".into(),
        sample_rate: None,
        channels: 1,
        anatomies,
        open_anatomies: false,
        options: vec![
            option("schedule", "string", json!("full"), "\"full\": a configuration move every 32 samples (skipped below 0.001); \"light\": every 128 (below 0.004), for slow devices. The World runs at the full rate either way; the light schedule's spectral cost is in TRACT-WORLD.md"),
        ],
        events: vec![
            event("preset", "Aim the articulators at a vowel (they travel at their own speed); stops a gesture.", vec![arg_enum("vowel", &PRESETS, 0, "the fitted public vowels /a/ /i/ /u/, or rest")]),
            event("gesture", "A named control path (the tube tract's key tables): mama and papapa loop; stop ends one.", vec![arg_enum("name", &GESTURES, 0, "")]),
            event("constrict", "Press the wall in anywhere along the oral tract (a touch): closes over 8 ms; degree 0.98 seals. Every move is a World move with its receipt.", vec![
                arg("slot", "index", 1.0, 4.0, 1.0, "touch slots 1-4 (0 belongs to gestures)"),
                arg("position", "0..1", 0.0, 1.0, 0.86, "glottis 0 .. lips 1"),
                arg("degree", "0..1", 0.0, 1.0, 0.9, "0 open .. 1 closed"),
            ]),
            event("unconstrict", "Release a touch slot (5 ms).", vec![arg("slot", "index", 1.0, 4.0, 1.0, "")]),
            event("impact", "Knock the wall: a raised-cosine volume-velocity pulse (0.5 ms) into that volume; its exact work booked as source work.", vec![
                arg("position", "0..1", 0.0, 1.0, 0.5, "glottis 0 .. lips 1"),
                arg("energy", "J", 0.0, 2.0e-3, 1.0e-4, "about this much work"),
            ]),
            event("sing", "A singer anatomy sings a pitch on a vowel: the fold tuning its own calibration heard sound that pitch (with its intonation offset), its vibrato, its vowel shape, gliding 0.2 s from the last vowel. Other anatomies refuse it (set pitch, the fold tuning, directly).", vec![
                arg("pitch", "MIDI", 20.0, 90.0, 57.0, "the note (outside the singer's calibrated range it is refused, not clamped)"),
                arg_enum("vowel", &VOWELS, 0, "a, i, u, m (hum), o (unfitted)"),
                arg("breath", "0..1", 0.0, 1.0, 0.55, "the valve's supply (scaled by the singer's breath scale); calibrated at 0.55"),
                arg("vibrato", "semitones", 0.0, 2.0, 0.0, "vibrato depth at the singer's own rate"),
            ]),
            event("hush", "Stop the breath (the folds stop; the lungs refill).", vec![]),
        ],
        controls: vec![
            control("tongueIndex", "0..1", 0.0, 1.0, 0.5, 60.0, "the articulators approach targets with 25 ms time constants; the World moves every 32 samples", "where the tongue body is (back .. front)"),
            control("tongueDiameter", "0..1", 0.0, 1.0, 0.3, 60.0, "as tongueIndex", "how far the tongue is raised"),
            control("lips", "0..1", 0.0, 1.0, 0.6, 60.0, "as tongueIndex; below 0.02 the lips seal", "lip opening"),
            control("velum", "0..1", 0.0, 1.0, 0.35, 60.0, "as tongueIndex", "velopharyngeal port (0 sealed, 1 nasal)"),
            control("pitch", "Hz", 20.0, 1200.0, 120.0, 60.0, "30 ms; each retune booked as valve work", "the folds' tuning (not the sounding pitch: a singer's table maps one to the other)"),
            control("tenseness", "0..1", 0.0, 1.0, 0.6, 30.0, "50 ms", "fold adduction"),
            control("breath", "0..1", 0.0, 1.0, 0.0, 60.0, "40 ms", "the valve's supply: lung pressure as a fraction of its maximum"),
            control("vibratoDepth", "semitones", 0.0, 2.0, 0.0, 10.0, "booked fold retunes", "fold-tension modulation"),
            control("vibratoRate", "Hz", 0.1, 12.0, 5.5, 10.0, "", "vibrato rate"),
            control("wobble", "0..1", 0.0, 1.0, 0.0, 10.0, "", "slow irregular pitch drift"),
            control("refill", "W", 0.0, 1.0, 0.3, 10.0, "", "the lungs' refill (the World's work_input, booked as source work); the anatomy's own until set"),
        ],
        telemetry: vec![
            slot("f0Hz", "Hz", "the folds' measured cycle rate (0 when not voicing); a non-loading observer"),
            slot("lungFraction", "0..1", "the reservoir's fill"),
            slot("glottalFlow", "cm3/s", "flow through the valve"),
            slot("narrowestAperture", "cm2", "the jet's aperture"),
            slot("tongueIndex", "0..1", "where the tongue is now"),
            slot("lips", "0..1", "where the lips are now"),
            slot("gestureActive", "0/1", ""),
            slot("impacts", "count", ""),
            slot("turbulenceJ", "J", "fluctuation energy the turbulent source injected"),
        ],
        score: None,
        output: OutputDef {
            observes: "the pressure at the two radiating volumes (lips and nostrils), summed (Pa)".into(),
            gain: 1.0 / 2_500.0,
            channels: "one point (left = right)".into(),
            about: "TractWorldNd::output_gain; the tract's own listener clamps to [-1, 1] and counts it (telemetry clamped)".into(),
        },
        lookahead_seconds: 0.0,
        cost: "one body of 24 modes recomputed per move: 18x real time through a vowel, a fricative and a touch, measured in node (wasm, 128-frame quanta, tools/rtf.mjs) on a 2019 8-core i9 laptop (TRACT-WORLD.md: 7.5-23.7x); building one takes 3-28 ms".into(),
        does_not_model: lines(&[
            "No glottal aspiration noise: a breathy voice and the aspiration after a stop's release are missing.",
            "One jet (the narrowest column), not one per narrow junction; an impact does not dent the wall; the stop gestures hold pa's closure only about 13 ms.",
            "The turbulent source's net work includes a zero-mean cross term with the flow already in its column.",
            "The approximations of CHECKPOINT-2A and POC-3 apply (mode-diagonal losses, stepped moves, numerical seal).",
            "Choir voices keep the adult's cross-sectional area function (only lengths, folds and lungs change); the voice types are design hypotheses, not measured singers; a voice's top is where its World stops voicing.",
            "Not intelligible speech, not a biological reconstruction.",
        ]),
        does_not_model_ref: "TRACT-WORLD.md#gaps".into(),
        build,
    }
}

struct TractSource {
    world: TractWorldNd,
    anatomy: String,
    schedule: (usize, f64),
    singer: Option<(Singer, SkillTable)>,
    /// A vowel glide in progress: (from, to, frames done, frames).
    glide: Option<(Articulation, Articulation, usize, usize)>,
    last_vowel: Option<usize>,
    sample_rate: f64,
}

fn anatomy_for(name: &str) -> Result<(TractNdAnatomy, Option<Singer>), String> {
    match name {
        "public-v1" => Ok((TractNdAnatomy::public_v1(), None)),
        "adult-v0" => Ok((TractNdAnatomy::adult_v0(44, 31), None)),
        "adult-v0-air" => Ok((TractNdAnatomy::adult_v0(44, 31).with_air(), None)),
        n => {
            if let Some(v) = VOICES.iter().position(|v| v.0 == n) {
                return Ok((voice_anatomy(v, 1.0, 1.0), None));
            }
            let s = singers().into_iter().find(|s| s.name == n).ok_or_else(|| format!("no tract anatomy {n:?}"))?;
            Ok((voice_anatomy(s.voice, s.tract_scale, s.fold_scale), Some(s)))
        }
    }
}

fn make_world(sr: f64, anatomy: TractNdAnatomy, schedule: (usize, f64), art: Articulation) -> Result<TractWorldNd, String> {
    let mut w = TractWorldNd::with(sr, anatomy, ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate }, art).map_err(|e| e.to_string())?;
    w.update_interval = schedule.0;
    w.min_move = schedule.1;
    let a = *w.anatomy();
    w.set_controls(TractControls { articulation: art, pitch_hz: a.valve.initial_tuning_hz, tenseness: a.valve.initial_adduction, breath: 0.0 });
    Ok(w)
}

fn build(_def: &KindDef, ctx: &CreateCtx) -> Result<Box<dyn Source>, String> {
    let name = ctx.anatomy.unwrap_or("public-v1").to_owned();
    let schedule = match opt::str_opt(ctx.options, "schedule")?.unwrap_or("full") {
        "full" => FULL_SCHEDULE,
        "light" => LIGHT_SCHEDULE,
        other => return Err(format!("option schedule: {other:?} (full or light)")),
    };
    let (anatomy, singer) = anatomy_for(&name)?;
    let art = if name.starts_with("adult") { tw::ART_REST } else { tw::ART_REST };
    let world = make_world(ctx.sample_rate, anatomy, schedule, art)?;
    let singer = singer.map(|s| {
        let table = SKILLS.iter().find(|x| x.0 == s.name).map(|x| SkillTable::parse(x.1)).unwrap_or(SkillTable(BTreeMap::new()));
        (s, table)
    });
    Ok(Box::new(TractSource { world, anatomy: name, schedule, singer, glide: None, last_vowel: None, sample_rate: ctx.sample_rate }))
}

impl TractSource {
    fn controls(&self) -> TractControls {
        self.world.controls()
    }

    fn checkpoint(&self) -> bool {
        self.anatomy.starts_with("adult")
    }
}

impl Source for TractSource {
    fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> Result<(), String> {
        if let Some((from, to, done, frames)) = self.glide {
            // The vowel glide moves the target sample by sample (`tract_choir.rs` `sing`).
            for i in 0..left.len() {
                let k = (done + i) as f64 / frames as f64;
                let mut c = self.controls();
                c.articulation = mix_art(from, to, k);
                self.world.set_controls(c);
                let mut one = [0.0f32];
                self.world.process(&mut one).map_err(|e| e.to_string())?;
                left[i] = one[0];
            }
            let done = done + left.len();
            self.glide = (done < frames).then_some((from, to, done, frames));
        } else {
            self.world.process(left).map_err(|e| e.to_string())?;
        }
        right.copy_from_slice(left);
        Ok(())
    }

    fn event(&mut self, index: usize, a: &[f64; MAX_ARGS], _at: u64, _now: u64) -> Result<(), String> {
        match index {
            0 => {
                let art = match (a[0].round() as usize, self.checkpoint()) {
                    (0, false) => tw::PUBLIC_ART_A,
                    (1, false) => tw::PUBLIC_ART_I,
                    (2, false) => tw::PUBLIC_ART_U,
                    (0, true) => tw::ART_A,
                    (1, true) => tw::ART_I,
                    (2, true) => tw::ART_U,
                    _ => tw::ART_REST,
                };
                self.world.set_gesture(None);
                self.glide = None;
                let mut c = self.controls();
                c.articulation = art;
                self.world.set_controls(c);
                Ok(())
            }
            1 => {
                let g = Gesture::from_index(a[0].round().max(0.0) as u32);
                self.glide = None;
                self.world.set_gesture(g);
                Ok(())
            }
            2 | 3 => {
                let slot = a[0].round() as usize;
                if !(1..=4).contains(&slot) {
                    return Err(format!("constriction slot {slot}: touches use 1-4"));
                }
                if self.world.anatomy().constriction_slots == 0 {
                    return Err(format!("anatomy {} has no constriction slots", self.anatomy));
                }
                let c = (index == 2).then_some(Constriction { position: a[1], degree: a[2] });
                self.world.set_constriction(slot, c);
                Ok(())
            }
            4 => {
                self.world.impact(a[0], a[1]);
                Ok(())
            }
            5 => {
                let Some((s, table)) = &self.singer else {
                    return Err(format!("anatomy {} has no pitch table: set pitch (the fold tuning) directly", self.anatomy));
                };
                let v = (a[1].round() as usize).min(4);
                let c = ['a', 'i', 'u', 'm', 'o'][v];
                let target = 440.0 * 2f64.powf((a[0] + s.intonation_cents / 100.0 - 69.0) / 12.0);
                let tuning = table.tuning_for(c, target)?;
                let mut art = vowel(v);
                art.tongue_index = (art.tongue_index + s.tongue_offset).clamp(0.0, 1.0);
                if v != 3 {
                    art.lips = (art.lips + s.lips_offset).clamp(0.0, 1.0);
                }
                self.world.set_gesture(None);
                self.world.set_vibrato(Vibrato { rate_hz: s.vib_rate_hz, depth_semitones: a[3], wobble: s.wobble });
                let mut ctl = self.controls();
                let from = ctl.articulation;
                ctl.pitch_hz = tuning;
                ctl.tenseness = 0.6;
                ctl.breath = (a[2] * s.breath_scale).clamp(0.0, 1.0);
                if self.last_vowel.is_some_and(|p| p != v) {
                    self.glide = Some((from, art, 0, ((ART_GLIDE_S * self.sample_rate) as usize).max(1)));
                } else {
                    self.glide = None;
                    ctl.articulation = art;
                }
                self.last_vowel = Some(v);
                self.world.set_controls(ctl);
                Ok(())
            }
            6 => {
                let mut c = self.controls();
                c.breath = 0.0;
                self.world.set_controls(c);
                Ok(())
            }
            _ => Err(format!("tract: no event {index}")),
        }
    }

    fn set(&mut self, index: usize, value: f64) -> Result<(), String> {
        let mut c = self.controls();
        match index {
            0..=3 => {
                match index {
                    0 => c.articulation.tongue_index = value,
                    1 => c.articulation.tongue_diameter = value,
                    2 => c.articulation.lips = value,
                    _ => c.articulation.velum = value,
                }
                // Manual articulation stops a gesture (the ABI's rule).
                if self.world.gesture_active() {
                    self.world.set_gesture(None);
                }
                self.glide = None;
            }
            4 => c.pitch_hz = value,
            5 => c.tenseness = value,
            6 => c.breath = value,
            7..=9 => {
                let mut v = self.world.vibrato();
                match index {
                    7 => v.depth_semitones = value,
                    8 => v.rate_hz = value,
                    _ => v.wobble = value,
                }
                self.world.set_vibrato(v);
                return Ok(());
            }
            10 => {
                self.world.set_refill(value);
                return Ok(());
            }
            _ => return Err(format!("tract: no control {index}")),
        }
        self.world.set_controls(c);
        Ok(())
    }

    fn ledger(&self) -> Ledger {
        Ledger::of(self.world.world())
    }

    fn receipt(&self) -> Value {
        let st = self.world.stats();
        let o = self.world.observers();
        let t = self.world.totals();
        json!({
            "anatomy": self.anatomy,
            "anatomyName": self.world.anatomy().name,
            "planDigest": self.world.world().plan_digest(),
            "outputGain": self.world.output_gain,
            "schedule": {"interval": self.schedule.0, "minMove": self.schedule.1},
            "moves": st.moves, "moveControlWork": st.control_work, "disposal": st.disposal,
            "valveRetunes": st.valve_retunes, "valveWork": st.valve_work,
            "impacts": o.knocks, "safetyLimited": o.safety_limited,
            "wallLoss": t.wall_loss, "valveJetLoss": t.valve_jet_loss, "foldLoss": t.fold_loss,
            "radiationByEnd": t.radiation_by_end, "turbulenceInjected": t.aero.turbulence_injected,
            "singer": self.singer.as_ref().map(|(s, _)| json!({"name": s.name, "voice": VOICES[s.voice].0, "tractScale": s.tract_scale, "foldScale": s.fold_scale, "intonationCents": s.intonation_cents})),
        })
    }

    fn telemetry(&self, out: &mut [f64]) {
        let o = self.world.observers();
        let body = self.world.world().space_body(0);
        out[0] = o.measured_f0_hz;
        out[1] = body.map_or(0.0, |b| b.reservoir_energy()) / self.world.anatomy().lung_capacity_j;
        if let Some(b) = body {
            let ob = b.observation();
            out[2] = ob.valve.flow_m3_s * 1e6;
            out[3] = ob.aero.jet_aperture_m2 * 1e4;
        }
        let cur = self.world.current();
        out[4] = cur.articulation.tongue_index;
        out[5] = cur.articulation.lips;
        out[6] = f64::from(u8::from(self.world.gesture_active()));
        out[7] = o.knocks as f64;
        out[8] = self.world.totals().aero.turbulence_injected;
    }

    fn reset(&mut self) -> Result<(), String> {
        let (anatomy, _) = anatomy_for(&self.anatomy)?;
        self.world = make_world(self.sample_rate, anatomy, self.schedule, tw::ART_REST)?;
        self.glide = None;
        self.last_vowel = None;
        Ok(())
    }

    fn clamped(&self) -> u64 {
        self.world.observers().safety_limited
    }

    fn plan_digest(&self) -> String {
        self.world.world().plan_digest().to_owned()
    }
}

fn mix_art(a: Articulation, b: Articulation, k: f64) -> Articulation {
    let k = k.clamp(0.0, 1.0);
    let l = |x: f64, y: f64| x + (y - x) * k;
    Articulation {
        tongue_index: l(a.tongue_index, b.tongue_index),
        tongue_diameter: l(a.tongue_diameter, b.tongue_diameter),
        lips: l(a.lips, b.lips),
        velum: l(a.velum, b.velum),
    }
}

#[cfg(test)]
mod tests {
    /// The voice types are tract_choir.rs's, number for number.
    #[test]
    fn voices_are_the_choir_drivers() {
        let src = include_str!("../../../ovillo-tract-world/examples/tract_choir.rs");
        for v in super::VOICES {
            let block = src.split(&format!("name: \"{}\",", v.0)).nth(1).expect("the voice");
            let block = &block[..block.find('}').unwrap()];
            for (field, value) in [("oral_m", v.1), ("nasal_m", v.2), ("fold_length_m", v.3), ("fold_thickness_m", v.4), ("fold_mass_kg", v.5), ("min_tuning_hz", v.6), ("max_tuning_hz", v.7), ("lung_capacity_j", v.8), ("lung_refill_w", v.9)] {
                let line = block.lines().find(|l| l.trim().starts_with(&format!("{field}:"))).expect(field);
                let x: f64 = line.split(':').nth(1).unwrap().trim().trim_end_matches(',').parse().unwrap();
                assert_eq!(x, value, "{} {field}", v.0);
            }
        }
    }
}
