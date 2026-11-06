//! Gates for the portable path: every World kind rendered offline through
//! `Instance` (the path a worklet runs, a block at a time), its books read
//! back, its receipt written to `receipts/<kind>.receipt.json`; the exact
//! laws the scheduler must keep (block segmentation and other instances
//! change nothing; an event lands on its frame); the trombone score streamed
//! sample for sample as the film driver renders it; the catalog file in step
//! with the code.
//!
//! `cargo test --release -p ovillo-portable` (in `ovillo/`).

use std::path::PathBuf;
use std::time::Instant;

use ovillo_dsp::trombone_nd::TromboneNd;
use ovillo_dsp::trombone_nd_player::{perform, place, ModeTable, Skill};
use ovillo_portable::kinds::trombone::{parse_notes, place_one};
use ovillo_portable::{catalog, kinds, render_offline, Instance, MAX_FRAMES};
use serde_json::{json, Value};

const SR: f64 = 48_000.0;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn event_index(inst: &Instance, name: &str) -> usize {
    inst.kind().events.iter().position(|e| e.name == name).unwrap_or_else(|| panic!("{} has no event {name}", inst.kind().id))
}

fn control_index(inst: &Instance, name: &str) -> usize {
    inst.kind().controls.iter().position(|c| c.name == name).unwrap_or_else(|| panic!("{} has no control {name}", inst.kind().id))
}

/// Schedule a named event at `t` seconds with named arguments (others default).
fn ev(inst: &mut Instance, t: f64, name: &str, args: &[(&str, f64)]) {
    let i = event_index(inst, name);
    let def = &inst.kind().events[i];
    let mut a = vec![f64::NAN; def.args.len()];
    for (k, v) in args {
        let j = def.args.iter().position(|x| x.name == *k).unwrap_or_else(|| panic!("{name} has no argument {k}"));
        a[j] = *v;
    }
    inst.event(i, t * SR, &a).unwrap();
}

fn set(inst: &mut Instance, t: f64, name: &str, value: f64, ramp_s: f64) {
    let i = control_index(inst, name);
    inst.set(i, value, t * SR, ramp_s * SR).unwrap();
}

/// What each kind plays for its receipt: (anatomy, options, seconds, schedule).
fn scenario(kind: &str) -> (Option<&'static str>, Value, Option<f64>, Box<dyn FnMut(&mut Instance) -> Result<(), String>>) {
    match kind {
        "steel-string" => (None, json!({"score": read("examples/tab/steel_string_tour.mus")}), Some(6.0), Box::new(|_| Ok(()))),
        "piano" => (
            None,
            json!({"keys": ["C4", "E4", "G4", "C5"]}),
            Some(4.0),
            Box::new(|i| {
                ev(i, 0.0, "note", &[("pitch", 60.0), ("velocity", 0.6), ("duration", 0.8)]);
                ev(i, 0.5, "note", &[("pitch", 64.0), ("velocity", 0.5), ("duration", 0.8)]);
                set(i, 0.9, "pedal", 1.0, 0.0);
                ev(i, 1.0, "key", &[("pitch", 67.0), ("velocity", 0.7)]);
                ev(i, 1.5, "key", &[("pitch", 72.0), ("velocity", 0.8)]);
                ev(i, 2.0, "release", &[("pitch", 67.0)]);
                ev(i, 2.0, "release", &[("pitch", 72.0)]);
                set(i, 2.8, "pedal", 0.0, 0.0);
                Ok(())
            }),
        ),
        "trombone" => (None, json!({"score": read("examples/ensemble/before-the-law/doorkeeper/doorkeeper-audition.tsv")}), Some(8.0), Box::new(|_| Ok(()))),
        "tract" => (
            None,
            json!({}),
            Some(4.0),
            Box::new(|i| {
                set(i, 0.0, "breath", 0.6, 0.05);
                ev(i, 0.0, "preset", &[("vowel", 0.0)]);
                ev(i, 1.0, "preset", &[("vowel", 1.0)]);
                ev(i, 1.6, "gesture", &[("name", 2.0)]);
                ev(i, 2.6, "constrict", &[("slot", 1.0), ("position", 0.86), ("degree", 0.93)]);
                ev(i, 3.0, "unconstrict", &[("slot", 1.0)]);
                ev(i, 3.2, "impact", &[("position", 0.5), ("energy", 1e-4)]);
                set(i, 3.5, "breath", 0.0, 0.1);
                Ok(())
            }),
        ),
        "fart" => (
            None,
            json!({}),
            Some(5.0),
            Box::new(|i| {
                ev(i, 0.0, "eat", &[("food", 0.0)]);
                ev(i, 0.0, "eat", &[("food", 5.0)]);
                ev(i, 2.0, "style", &[("name", 0.0)]);
                Ok(())
            }),
        ),
        "bench" => (
            None,
            json!({"seed": 7}),
            Some(4.0),
            Box::new(|i| {
                ev(i, 0.0, "pattern", &[("name", 0.0)]);
                ev(i, 0.0, "transport", &[("on", 1.0)]);
                Ok(())
            }),
        ),
        "kart_engine" => (
            None,
            json!({"seed": 3}),
            Some(6.0),
            Box::new(|i| {
                ev(i, 0.0, "start", &[]);
                set(i, 0.0, "throttle", 0.6, 0.0);
                set(i, 0.0, "rpm", 3000.0, 0.0);
                set(i, 0.8, "rpm", 9000.0, 1.5);
                set(i, 2.5, "boost", 0.8, 0.2);
                set(i, 3.0, "skid", 0.5, 0.1);
                set(i, 3.0, "surface", 1.0, 0.0);
                ev(i, 3.2, "bump", &[("impulse", 0.7)]);
                ev(i, 3.5, "land", &[("impulse", 0.5)]);
                set(i, 3.8, "load", 0.0, 0.2);
                ev(i, 4.0, "hit", &[]);
                ev(i, 5.0, "restart", &[]);
                Ok(())
            }),
        ),
        "race_props" => (
            None,
            json!({}),
            Some(9.0),
            Box::new(|i| {
                for (k, n) in [3.0, 2.0, 1.0].iter().enumerate() {
                    ev(i, k as f64, "beep", &[("n", *n)]);
                }
                ev(i, 3.0, "go", &[]);
                ev(i, 4.5, "bell", &[]);
                ev(i, 5.0, "finish", &[]);
                Ok(())
            }),
        ),
        "f16" => (
            None,
            json!({"seed": 5}),
            Some(4.0),
            Box::new(|i| {
                set(i, 0.0, "thrust", 0.5, 0.0);
                set(i, 0.0, "airspeed", 250.0, 0.0);
                set(i, 0.0, "aspect", -1.0, 0.0);
                set(i, 1.0, "afterburner", 1.0, 1.0);
                set(i, 1.0, "aspect", 1.0, 3.0);
                Ok(())
            }),
        ),
        // the tab anatomies: touch every element, glide, bow where it bows
        other => {
            let def = kinds::find(other).expect("a tab kind");
            let n = def.events[0].args[0].max as usize;
            let fretted = def.events.iter().any(|e| e.name == "slide" || e.name == "pedal-glide");
            let bowable = def.events.iter().any(|e| e.name == "bow-land");
            let touch = def.events[0].name.clone();
            let glide = def.events.iter().find(|e| e.name == "slide" || e.name == "pedal-glide").map(|e| e.name.clone());
            (
                None,
                json!({}),
                Some(4.0),
                Box::new(move |i| {
                    for s in 1..=n {
                        let t = 0.05 + 0.25 * (s - 1) as f64;
                        if fretted {
                            ev(i, t, &touch, &[(i.kind().events[0].args[0].name.as_str(), s as f64), ("fret", 2.0), ("velocity", 0.7)]);
                        } else {
                            ev(i, t, &touch, &[(i.kind().events[0].args[0].name.as_str(), s as f64), ("velocity", 0.7)]);
                        }
                    }
                    if let Some(g) = &glide {
                        let first = i.kind().events[0].args[0].name.clone();
                        ev(i, 2.4, g, &[(first.as_str(), 2.0), ("fret", 4.0)]);
                    }
                    if bowable {
                        ev(i, 2.6, "bow-land", &[("string", 1.0), ("speed", 0.5), ("force", 0.5), ("position", 0.12)]);
                        set(i, 3.0, "bowSpeed", 0.8, 0.3);
                        ev(i, 3.6, "bow-lift", &[]);
                    }
                    Ok(())
                }),
            )
        }
    }
}

fn gate(kind: &str) {
    let (anatomy, options, seconds, schedule) = scenario(kind);
    let frames = seconds.map(|s| (s * SR) as usize);
    let t0 = Instant::now();
    let (left, right, receipt) = render_offline(kind, SR, anatomy, &options, frames, schedule).unwrap_or_else(|e| panic!("{kind}: {e}"));
    let wall = t0.elapsed().as_secs_f64();
    let secs = left.len() as f64 / SR;
    assert!(left.iter().chain(&right).all(|v| v.is_finite()), "{kind}: non-finite output");
    let peak = left.iter().chain(&right).fold(0.0f32, |m, v| m.max(v.abs()));
    let l = &receipt["ledger"];
    let (cum, frame) = (l["cumulativeClosureResidual"].as_f64().unwrap(), l["maximumFrameResidual"].as_f64().unwrap());
    println!(
        "{kind}: {secs:.1} s in {wall:.2} s ({:.2}x real time, native, loaded machine); peak {:.1} dBFS; source {:.3e} J, control {:.3e} J, loss {:.3e} J, radiation {:.3e} J; cumulative residual {cum:.2e} J, largest frame residual {frame:.2e} J",
        secs / wall,
        20.0 * f64::from(peak).log10(),
        l["sourceWork"].as_f64().unwrap(),
        l["controlWork"].as_f64().unwrap(),
        l["internalLoss"].as_f64().unwrap(),
        l["radiation"].as_f64().unwrap(),
    );
    assert_eq!(receipt["events"]["refused"], 0, "{kind}: {}", receipt["events"]);
    assert_eq!(receipt["events"]["errors"], 0, "{kind}: {}", receipt["events"]);
    assert!(peak > 0.0, "{kind}: silent");
    assert!(l["sourceWork"].as_f64().unwrap() > 0.0, "{kind}: no source work");
    assert!(l["radiation"].as_f64().unwrap() > 0.0, "{kind}: nothing radiated");
    // Absolute for the Ariadne Worlds (1e-12 J a frame, 1e-9 J in all), relative to the energy moved for the big ones.
    let (frame_tol, tol) = (l["frameTolerance"].as_f64().unwrap(), l["closureTolerance"].as_f64().unwrap());
    assert!(frame.abs() <= frame_tol, "{kind}: frame residual {frame:e} > {frame_tol:e}");
    assert!(cum.abs() <= tol, "{kind}: cumulative residual {cum:e} > {tol:e}");
    assert_eq!(receipt["ledger"]["closes"], true);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("receipts");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{kind}.receipt.json")), serde_json::to_string_pretty(&receipt).unwrap()).unwrap();
}

macro_rules! gates {
    ($($name:ident => $kind:literal),* $(,)?) => {
        $( #[test] fn $name() { gate($kind); } )*
        /// Every kind in the catalog has a gate here.
        #[test]
        fn every_kind_has_a_gate() {
            let gated = [$($kind),*];
            for k in kinds::all() {
                assert!(gated.contains(&k.id.as_str()), "no gate for {}", k.id);
            }
        }
    };
}

gates! {
    steel_string => "steel-string",
    gate_kind => "gate",
    waiting => "waiting",
    glass_bells => "glass-bells",
    fleas => "fleas",
    slam_drums => "slam-drums",
    sunwire => "sunwire",
    tumbler => "tumbler",
    prism => "prism",
    pocket => "pocket",
    piano => "piano",
    trombone => "trombone",
    tract => "tract",
    fart => "fart",
    bench => "bench",
    kart_engine => "kart_engine",
    race_props => "race_props",
    f16 => "f16",
}

/// The tract's singers: a choir voice sings from its own pitch table.
#[test]
fn a_choir_singer_sings_its_table() {
    let (left, _, receipt) = render_offline("tract", SR, Some("bass1"), &json!({}), Some(96_000), |i| {
        ev(i, 0.0, "sing", &[("pitch", 45.0), ("vowel", 3.0), ("breath", 0.55)]);
        ev(i, 1.0, "sing", &[("pitch", 50.0), ("vowel", 0.0), ("breath", 0.55)]);
        Ok(())
    })
    .unwrap();
    assert_eq!(receipt["events"]["refused"], 0, "{}", receipt["events"]);
    assert!(left.iter().any(|v| v.abs() > 1e-4));
    assert!(receipt["ledger"]["maximumFrameResidual"].as_f64().unwrap().abs() < 1e-12);
    // Out of range is refused, not clamped.
    let mut i = Instance::create("tract", SR, Some("child1"), &json!({})).unwrap();
    ev(&mut i, 0.0, "sing", &[("pitch", 30.0)]);
    i.render(128);
    assert!(i.last_error().contains("outside what the calibration heard"), "{}", i.last_error());
}

fn digest_of(kind: &str, anatomy: Option<&str>, frames: usize, block: usize, schedule: impl Fn(&mut Instance)) -> String {
    let mut i = Instance::create(kind, SR, anatomy, &json!({})).unwrap();
    schedule(&mut i);
    let mut done = 0;
    while done < frames {
        done += i.render(block.min(frames - done));
    }
    i.receipt()["outputDigest"].as_str().unwrap().to_owned()
}

/// Block segmentation cannot change state or audio (constitution 10): the
/// same schedule rendered in 128-, 1000- and 4096-frame calls gives the same samples.
#[test]
fn block_segmentation_changes_nothing() {
    for (kind, anatomy) in [("steel-string", None), ("tract", None), ("trombone", None)] {
        let schedule = |i: &mut Instance| match kind {
            "steel-string" => {
                ev(i, 0.01, "pluck", &[("string", 6.0), ("fret", 0.0), ("velocity", 0.6)]);
                ev(i, 0.2, "slide", &[("string", 6.0), ("fret", 3.0)]);
                ev(i, 0.3, "bow-land", &[("string", 3.0)]);
                set(i, 0.35, "bowSpeed", -0.6, 0.1003);
            }
            "tract" => {
                set(i, 0.0, "breath", 0.6, 0.0507);
                ev(i, 0.2, "gesture", &[("name", 7.0)]);
            }
            _ => {
                ev(i, 0.01, "note", &[("pitch", 58.0), ("velocity", 0.5), ("duration", 0.3)]);
                ev(i, 0.35, "note", &[("pitch", 62.0), ("velocity", 0.6), ("articulation", 1.0)]);
                ev(i, 0.55, "release", &[]);
            }
        };
        let frames = 36_000;
        let a = digest_of(kind, anatomy, frames, 128, schedule);
        let b = digest_of(kind, anatomy, frames, 1000, schedule);
        let c = digest_of(kind, anatomy, frames, MAX_FRAMES, schedule);
        assert_eq!(a, b, "{kind}: 128 vs 1000");
        assert_eq!(a, c, "{kind}: 128 vs 4096");
    }
}

/// Many Worlds in one module: rendering another instance between blocks changes nothing.
#[test]
fn instances_are_independent() {
    let schedule = |i: &mut Instance| ev(i, 0.0, "pluck", &[("string", 1.0), ("fret", 5.0), ("velocity", 0.8)]);
    let alone = digest_of("steel-string", None, 24_000, 128, schedule);
    let mut a = Instance::create("steel-string", SR, None, &json!({})).unwrap();
    let mut b = Instance::create("steel-string", SR, None, &json!({})).unwrap();
    let mut t = Instance::create("tract", SR, None, &json!({})).unwrap();
    schedule(&mut a);
    ev(&mut b, 0.0, "pluck", &[("string", 6.0), ("velocity", 1.0)]);
    set(&mut t, 0.0, "breath", 0.7, 0.0);
    let mut done = 0;
    while done < 24_000 {
        let n = 128.min(24_000 - done);
        done += a.render(n);
        b.render(n);
        t.render(128);
    }
    assert_eq!(alone, a.receipt()["outputDigest"].as_str().unwrap());
}

/// An event lands on its frame: an open-string pluck at frame 4321 leaves
/// the World silent before it and sounds from it.
#[test]
fn an_event_lands_on_its_frame() {
    let mut i = Instance::create("steel-string", SR, None, &json!({})).unwrap();
    let k = event_index(&i, "pluck");
    i.event(k, 4321.0, &[6.0, 0.0, 0.7]).unwrap();
    let mut out = Vec::new();
    while out.len() < 8192 {
        let n = i.render(128);
        out.extend_from_slice(&i.left[..n]);
    }
    assert!(out[..4321].iter().all(|v| *v == 0.0), "sound before the pluck");
    assert!(out[4321..4400].iter().any(|v| *v != 0.0), "no sound at the pluck");
}

/// The trombone score streams `perform` (and the film driver's hairpins) sample for sample.
#[test]
fn the_trombone_score_is_the_film_drivers() {
    let text = read("examples/ensemble/before-the-law/doorkeeper/doorkeeper-audition.tsv");
    let (notes, vel_end) = parse_notes(&text).unwrap();
    let table = ModeTable::embedded().unwrap();
    let skill = Skill::embedded().unwrap();
    let placed = place(&table, &skill, &notes);
    // place_one, chained, is place.
    let mut prev = None;
    for (n, p) in notes.iter().zip(&placed) {
        let q = place_one(&table, &skill, n, prev);
        assert_eq!((q.partial, q.slide_m.to_bits()), (p.partial, p.slide_m.to_bits()));
        prev = Some(q);
    }
    // trombone_nd_score.rs, verbatim in effect: perform, then the hairpins.
    let mut breaths = perform(&table, &skill, &notes, &placed, SR, 2.5);
    let smooth = |x: f64| {
        let x = x.clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    };
    for (i, note) in notes.iter().enumerate() {
        let to = vel_end[i];
        if (to - note.vel).abs() < 1e-9 {
            continue;
        }
        let pl = placed[i];
        let emb = |v: f64| skill.embouchure_near(&table, pl.partial, pl.slide_m, v).unwrap().mouth;
        let (m_written, m0, m1) = (emb(note.vel), emb(note.vel), emb(to));
        let s0 = (note.start_s * SR) as usize;
        let owned_until = if i + 1 < notes.len() { (notes[i + 1].start_s * SR) as usize } else { breaths.len() };
        let sounding = (note.end_s - note.start_s) * 0.90;
        for t in s0..owned_until.min(breaths.len()) {
            let tau = t as f64 / SR - note.start_s;
            let k = smooth(tau / sounding);
            let factor = (m0.ln() + k * (m1.ln() - m0.ln())).exp() / m_written;
            breaths[t].mouth = (breaths[t].mouth * factor).clamp(0.0, 1.0);
        }
    }
    let n = breaths.len().min((8.0 * SR) as usize);
    let mut tw = TromboneNd::new().unwrap();
    tw.set_slide_now(breaths[0].slide_m).unwrap();
    let reference: Vec<f32> = breaths[..n].iter().map(|b| tw.tick(*b).unwrap() as f32).collect();
    let (left, right, _) = render_offline("trombone", SR, None, &json!({"score": text}), Some(n), |_| Ok(())).unwrap();
    for k in 0..n {
        assert_eq!(left[k].to_bits(), reference[k].to_bits(), "sample {k}");
        assert_eq!(right[k].to_bits(), reference[k].to_bits(), "sample {k}");
    }
}

/// `web/ovillo/ovillo-catalog.json` is what the code says (regenerate with
/// `web/ovillo/build.sh`, or `OVILLO_WRITE_CATALOG=1` here).
#[test]
fn the_catalog_file_is_the_codes() {
    let path = root().join("web/ovillo/ovillo-catalog.json");
    let now = serde_json::to_string_pretty(&catalog::catalog_json()).unwrap() + "\n";
    if std::env::var_os("OVILLO_WRITE_CATALOG").is_some() {
        std::fs::write(&path, &now).unwrap();
    }
    let file = std::fs::read_to_string(&path).unwrap_or_default();
    // build.sh writes it from the wasm through JSON.stringify (0 for 0.0): compare numbers by value.
    fn by_value(v: Value) -> Value {
        match v {
            Value::Number(n) => json!(n.as_f64()),
            Value::Array(a) => Value::Array(a.into_iter().map(by_value).collect()),
            Value::Object(o) => Value::Object(o.into_iter().map(|(k, v)| (k, by_value(v))).collect()),
            other => other,
        }
    }
    let (a, b): (Value, Value) = (serde_json::from_str(&now).unwrap(), serde_json::from_str(&file).unwrap_or(Value::Null));
    assert!(by_value(a) == by_value(b), "{} is stale: run web/ovillo/build.sh", path.display());
}

/// Every event and control the catalog lists can be reached, and a wrong
/// index or anatomy is refused with a reason.
#[test]
fn the_catalog_is_the_surface() {
    for k in kinds::all() {
        assert!(!k.does_not_model.is_empty(), "{} says what it does not model", k.id);
        let mut i = Instance::create(&k.id, k.sample_rate.unwrap_or(SR), None, &json!({})).unwrap_or_else(|e| panic!("{}: {e}", k.id));
        assert!(i.event(k.events.len(), 0.0, &[]).is_err());
        assert!(i.set(k.controls.len(), 0.0, 0.0, 0.0).is_err());
        for (c, def) in k.controls.iter().enumerate() {
            i.set(c, def.default, 0.0, 0.0).unwrap();
        }
        i.render(256);
        assert_eq!(i.telemetry.len(), ovillo_portable::slot::KIND + k.telemetry.len());
    }
    assert!(Instance::create("steel-string", 44_100.0, None, &json!({})).err().unwrap().contains("48000"));
    assert!(Instance::create("tract", SR, Some("nobody"), &json!({})).is_err());
    assert!(Instance::create("kazoo", SR, None, &json!({})).is_err());
}
