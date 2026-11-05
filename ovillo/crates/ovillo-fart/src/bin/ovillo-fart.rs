//! `ovillo-fart`: play a score directory of fart parts (the part format is in
//! the library's documentation).
//!
//! ```text
//! ovillo-fart check   <dir> [--section S] [--voice V]            parse, beats, bars, tensions, feeding; renders nothing
//! ovillo-fart render  <dir> [--section S] [--voice V] [--force] [--jobs N] [--no-mix]   (with --section, mixes only it)
//! ovillo-fart analyze <dir> --section S [--voice V]              a part's per-note table, or a section mix's timbre
//! ovillo-fart probe   [--score <dir> [--voice V] | --anatomy soprano|alto|bass|cannon] [--push P] [--cheeks C] [--tone T]
//!                     [--wobble W] [--from 30] [--to 300] [--steps 16]      (with --score, the score's direction is on)
//! ovillo-fart mix     <dir>
//! ```
//!
//! Add `--json` for the whole report. Exit status 1 when `check` finds
//! problems or a part fails to render.

use ovillo_fart::{self as ens, Filter};
use serde_json::Value;
use std::path::PathBuf;

fn usage() -> ! {
    eprintln!("usage: ovillo-fart check|render|analyze|mix <score dir> [--section S] [--voice V] [--force] [--jobs N] [--no-mix] [--json]\n       ovillo-fart probe [--score <dir> [--voice V] | --anatomy NAME] [--push P] [--cheeks C] [--tone T] [--wobble W] [--from Hz] [--to Hz] [--steps N] [--json]");
    std::process::exit(2);
}

fn fail(e: String) -> ! {
    eprintln!("{e}");
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args.iter().any(|a| a == "-h" || a == "--help") {
        usage();
    }
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let num = |flag: &str, d: f64| value(flag).and_then(|v| v.parse().ok()).unwrap_or(d);
    let has = |flag: &str| args.iter().any(|a| a == flag);
    let as_json = has("--json");
    let print = |v: &Value| println!("{}", serde_json::to_string_pretty(v).unwrap());
    if args[1] == "probe" {
        // --score <dir> probes with that score's direction; --anatomy names the gut (or --voice, with --score, picks the voice's)
        let score = value("--score").map(|d| ens::load(&PathBuf::from(d)).unwrap_or_else(|e| fail(e)));
        let anatomy = value("--anatomy")
            .or_else(|| score.as_ref().zip(value("--voice")).and_then(|(s, v)| s.voices.get(&v).map(|spec| spec.anatomy.clone())))
            .unwrap_or_else(|| "alto".into());
        let direction = score.as_ref().map(|s| s.direction.clone()).unwrap_or_default();
        let r = ens::probe_on(&anatomy, &direction, num("--push", 0.7), num("--cheeks", 0.4), num("--tone", 0.75), num("--wobble", 0.0), num("--from", 30.0), num("--to", 300.0), num("--steps", 16.0) as usize).unwrap_or_else(|e| fail(e));
        if as_json {
            print(&r);
        } else {
            println!("anatomy {} direction {} push {} cheeks {} tone {} wobble {} (a held note per tension on a practice gut; measurement only)", r["anatomy"], r["direction"], r["push"], r["cheeks"], r["tone"], r["wobble"]);
            println!("  tension Hz -> measured Hz  note       cents   rms dB  sharpness  centroid Hz");
            for row in r["rows"].as_array().into_iter().flatten() {
                println!(
                    "  {:>9} -> {:>9}  {:<9} {:>6}  {:>6}  {:>9}  {:>8}",
                    row["tensionHz"], row["measuredHz"].as_f64().map_or("no swing".into(), |m| m.to_string()),
                    row["note"].as_str().unwrap_or("-"), row["centsFromTension"].as_f64().map_or("-".into(), |c| format!("{c:+}")),
                    row["rmsDb"], row["sharpness"], row["centroidHz"]
                );
            }
        }
        return;
    }
    if args.len() < 3 {
        usage();
    }
    let dir = PathBuf::from(&args[2]);
    let filter = Filter { section: value("--section"), voice: value("--voice") };
    let score = ens::load(&dir).unwrap_or_else(|e| fail(e));
    match args[1].as_str() {
        "check" => {
            let report = ens::check(&dir, &score, &filter);
            if as_json { print(&report) } else { print_check(&report) }
            if report["ok"] != true {
                std::process::exit(1);
            }
        }
        "render" => {
            let jobs = value("--jobs").and_then(|j| j.parse().ok()).unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
            let t0 = std::time::Instant::now();
            let report = ens::render(&dir, &score, &filter, has("--force"), jobs);
            let mixed = if has("--no-mix") { None } else { Some(ens::mix(&dir, &score, filter.section.as_deref()).unwrap_or_else(|e| fail(e))) };
            if as_json {
                print(&serde_json::json!({"render": report, "mix": mixed}));
            } else {
                print_render(&report);
                if let Some(m) = &mixed {
                    print_mix(m);
                }
                println!("({:.1} s)", t0.elapsed().as_secs_f64());
            }
            if report["ok"] != true {
                std::process::exit(1);
            }
        }
        "analyze" => {
            let section = filter.section.clone().unwrap_or_else(|| fail("analyze needs --section".into()));
            let r = match &filter.voice {
                Some(v) => ens::analyze_part(&dir, &section, v),
                None => ens::analyze_section(&dir, &section),
            }
            .unwrap_or_else(|e| fail(e));
            if as_json {
                print(&r);
            } else if filter.voice.is_some() {
                print_part(&r);
            } else {
                print_section(&r);
            }
        }
        "mix" => {
            let m = ens::mix(&dir, &score, filter.section.as_deref()).unwrap_or_else(|e| fail(e));
            if as_json { print(&m) } else { print_mix(&m) }
        }
        _ => usage(),
    }
}

fn s(v: &Value) -> String {
    match v {
        Value::Null => "-".into(),
        Value::String(t) => t.clone(),
        other => other.to_string(),
    }
}

fn print_check(r: &Value) {
    for sec in r["sections"].as_array().into_iter().flatten() {
        println!("{} @ {} bpm, {} beats", s(&sec["section"]), sec["bpm"], sec["beats"]);
        for p in sec["parts"].as_array().into_iter().flatten() {
            let v = s(&p["voice"]);
            if let Some(e) = p["error"].as_str() {
                println!("  {v}: {e}");
                continue;
            }
            if let Some(errs) = p["errors"].as_array() {
                for e in errs {
                    println!("  {v}: {}", s(e));
                }
                continue;
            }
            println!("  {v}: {} notes, {} beats ({} s), eats {} J, digests {}{}", p["notes"], p["beats"], p["seconds"], p["eatsJ"], p["digests"],
                p["warning"].as_str().map_or(String::new(), |w| format!("  WARNING {w}")));
        }
        for m in sec["misaligned"].as_array().into_iter().flatten() {
            println!("  MISALIGNED {}", s(m));
        }
    }
    println!("{}", if r["ok"] == true { "ok".to_string() } else { format!("{} problem(s)", r["problems"]) });
}

fn note_line(n: &Value) -> String {
    if n["targetHz"].is_null() {
        return format!("line {} {} (shot at {} s, pop {} dB)", n["line"], s(&n["token"]), n["atS"], n["popDb"]);
    }
    format!("line {} {} {}", n["line"], s(&n["token"]), n["cents"].as_f64().map_or("NO SWING".into(), |c| format!("{c:+} c")))
}

fn print_render(r: &Value) {
    for st in r["stems"].as_array().into_iter().flatten() {
        let name = format!("{}/{}", s(&st["section"]), s(&st["voice"]));
        if let Some(e) = st["error"].as_str() {
            println!("{name}: ERROR {e}");
            continue;
        }
        println!(
            "{name}: {}, {} s; in tune {} (50 c), mean {} c; silent {}, started dry {}; gas at end {}, stomach left {} J; rms {} dB, sharpness {}, centroid {} Hz, mean pop {} dB; clipped {}; max frame residual {:.1e} J",
            if st["cached"] == true { "cached" } else { "rendered" }, st["seconds"], s(&st["inTune50Cents"]), st["meanAbsCents"], st["silentNotes"], st["notesStartedDry"],
            st["gasEnd"], st["stomachLeftJ"], st["rmsDb"], st["sharpness"], st["centroidHz"], st["meanPopDb"], st["clippedSamples"],
            st["maximumFrameResidualJ"].as_f64().unwrap_or(f64::NAN)
        );
        let worst: Vec<String> = st["worstTuning"].as_array().into_iter().flatten().filter(|n| n["cents"].as_f64().map_or(true, |c| c.abs() > 50.0)).map(note_line).collect();
        if !worst.is_empty() {
            println!("    worst tuning: {}", worst.join(", "));
        }
        let pops: Vec<String> = st["worstPops"].as_array().into_iter().flatten().map(|n| format!("line {} {} {} dB", n["line"], s(&n["token"]), n["popDb"])).collect();
        if !pops.is_empty() {
            println!("    loudest attack pops: {}", pops.join(", "));
        }
    }
    println!("{} rendered, {} error(s)", r["rendered"], r["errors"]);
}

fn print_part(r: &Value) {
    println!(
        "{}/{}: in tune {} (50 c), mean {} c, silent {}, started dry {}, gas at end {}; rms {} dB, sharpness {}, centroid {} Hz, >4 kHz {}; clipped {}",
        s(&r["section"]), s(&r["voice"]), s(&r["inTune50Cents"]), r["meanAbsCents"], r["silentNotes"], r["notesStartedDry"], r["gasEnd"],
        r["rmsDb"], r["sharpness"], r["centroidHz"], r["above4k"], r["clippedSamples"]
    );
    println!("  line token            at s   gas    target  tension  measured  cents  swung  rms dB  pop dB  sharp  centroid");
    for n in r["notes"].as_array().into_iter().flatten() {
        println!(
            "  {:>4} {:<16} {:>6} {:>5}  {:>7} {:>8} {:>9} {:>6} {:>6} {:>7} {:>7} {:>6} {:>9}",
            n["line"], s(&n["token"]), n["atS"], n["gas"], s(&n["targetHz"]), s(&n["tensionHz"]), s(&n["measuredHz"]), s(&n["cents"]), s(&n["swung"]),
            n["rmsDb"], n["popDb"], n["sharpness"], n["centroidHz"]
        );
    }
}

fn print_section(r: &Value) {
    println!("{}: {} s, rms {} dB, sharpness {}, centroid {} Hz, >4 kHz {}", s(&r["section"]), r["seconds"], r["rmsDb"], r["sharpness"], r["centroidHz"], r["above4k"]);
    println!("  sharpest half-seconds: {}", r["sharpestWindows"].as_array().into_iter().flatten().map(|w| format!("{} s ({}, {} dB)", w["atS"], w["sharpness"], w["rmsDb"])).collect::<Vec<_>>().join(", "));
    println!("  at s   rms dB  sharpness  centroid Hz");
    for w in r["windows"].as_array().into_iter().flatten() {
        println!("  {:>5} {:>7} {:>10} {:>11}", w["atS"], w["rmsDb"], w["sharpness"], w["centroidHz"]);
    }
}

fn print_mix(m: &Value) {
    let missing = m["missingStems"].as_array().map_or(0, Vec::len);
    println!("mix: {} ({} s){}", s(&m["mix"]), m["seconds"], if missing > 0 { format!(", {missing} stem(s) missing") } else { String::new() });
}
