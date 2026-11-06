//! `ovillo-body`: the command line.
//!
//! ```text
//! ovillo-body describe
//! ovillo-body presets [--write <dir>]
//! ovillo-body render --body <name|path.json> --in <dry.wav> --out <wet.wav> [--receipt <r.json>] [--set id=value ...]
//! ovillo-body inspect --body <name|path.json> [--set id=value ...] [--seconds S]
//! ovillo-body plan --body <name|path.json> [--set id=value ...]      (the ariadne-world-plan-v3 JSON)
//! ```

use std::path::{Path, PathBuf};
use std::time::Instant;

use ovillo_body::{presets, Body, BodySpec};

const RATE: f64 = 48_000.0;

fn usage() -> ! {
    eprintln!(
        "usage: ovillo-body describe\n       ovillo-body presets [--write <dir>]\n       ovillo-body render --body <name|path.json> --in <dry.wav> --out <wet.wav> [--receipt <r.json>] [--set id=value ...]\n       ovillo-body inspect --body <name|path.json> [--set id=value ...] [--seconds S] [--json]\n       ovillo-body plan --body <name|path.json> [--set id=value ...]"
    );
    std::process::exit(2);
}

fn fail(e: impl std::fmt::Display) -> ! {
    eprintln!("ovillo-body: {e}");
    std::process::exit(1);
}

fn load_body(name: &str) -> Result<BodySpec, String> {
    if let Some(p) = presets::get(name) {
        return Ok(p);
    }
    let path = Path::new(name);
    if path.exists() {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let spec: BodySpec = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        return Ok(spec);
    }
    Err(format!("no preset or file named {name:?} (presets: {})", presets::names().join(", ")))
}

fn apply_sets(spec: &mut BodySpec, args: &[String]) -> Result<Vec<String>, String> {
    let mut notes = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--set" {
            let kv = args.get(i + 1).ok_or("--set needs id=value")?;
            let (id, value) = kv.split_once('=').ok_or_else(|| format!("--set {kv}: expected id=value"))?;
            let value: f64 = value.trim().parse().map_err(|_| format!("--set {kv}: not a number"))?;
            if let Some(note) = spec.set(id.trim(), value)? {
                notes.push(note);
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    Ok(notes)
}

fn value(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned()
}

fn read_wav(path: &Path) -> Result<(Vec<f32>, Vec<String>), String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let spec = reader.spec();
    let mut notes = Vec::new();
    if spec.sample_rate != RATE as u32 {
        return Err(format!("{}: {} Hz; this Body runs at 48000 Hz (resample first)", path.display(), spec.sample_rate));
    }
    let ch = spec.channels as usize;
    if ch != 1 {
        notes.push(format!("input has {ch} channels: averaged to mono"));
    }
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u64 << (spec.bits_per_sample - 1)) as f32;
            reader.samples::<i32>().map(|s| s.unwrap_or(0) as f32 * scale).collect()
        }
    };
    let mono: Vec<f32> = samples.chunks(ch).map(|c| c.iter().sum::<f32>() / ch as f32).collect();
    Ok((mono, notes))
}

fn write_wav(path: &Path, channels: usize, data: &[f32]) -> Result<(), String> {
    let spec = hound::WavSpec { channels: channels as u16, sample_rate: RATE as u32, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| format!("{}: {e}", path.display()))?;
    for &s in data {
        w.write_sample(s).map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        usage();
    }
    match args[0].as_str() {
        "describe" => println!("{}", serde_json::to_string_pretty(&ovillo_body::describe()).unwrap()),
        "presets" => {
            if let Some(dir) = value(&args, "--write") {
                let dir = PathBuf::from(dir);
                std::fs::create_dir_all(&dir).unwrap_or_else(|e| fail(e));
                for name in presets::names() {
                    let spec = presets::get(name).unwrap();
                    let path = dir.join(format!("{name}.json"));
                    std::fs::write(&path, serde_json::to_string_pretty(&spec).unwrap()).unwrap_or_else(|e| fail(e));
                    println!("{}", path.display());
                }
            } else {
                for name in presets::names() {
                    let spec = presets::get(name).unwrap();
                    println!("{name}\t{}", spec.about);
                }
            }
        }
        "render" => {
            let body_name = value(&args, "--body").unwrap_or_else(|| usage());
            let input = value(&args, "--in").unwrap_or_else(|| usage());
            let output = value(&args, "--out").unwrap_or_else(|| usage());
            let mut spec = load_body(&body_name).unwrap_or_else(|e| fail(e));
            let mut notes = apply_sets(&mut spec, &args).unwrap_or_else(|e| fail(e));
            let (dry, mut wav_notes) = read_wav(Path::new(&input)).unwrap_or_else(|e| fail(e));
            notes.append(&mut wav_notes);
            let t_build = Instant::now();
            let mut body = Body::new(&spec, RATE).unwrap_or_else(|e| fail(e));
            let build_seconds = t_build.elapsed().as_secs_f64();
            let ch = body.channels();
            let mut wet = vec![0.0f32; dry.len() * ch];
            let t0 = Instant::now();
            body.process(&dry, &mut wet);
            let render_seconds = t0.elapsed().as_secs_f64();
            write_wav(Path::new(&output), ch, &wet).unwrap_or_else(|e| fail(e));
            let mut receipt = serde_json::to_value(body.receipt()).unwrap();
            let audio_seconds = dry.len() as f64 / RATE;
            let factor = if render_seconds > 0.0 { audio_seconds / render_seconds } else { f64::INFINITY };
            receipt["speed"] = serde_json::json!({
                "audio_seconds": audio_seconds,
                "render_seconds": render_seconds,
                "build_seconds": build_seconds,
                "realtime_factor": factor,
            });
            receipt["input"] = serde_json::json!(input);
            receipt["output"] = serde_json::json!(output);
            receipt["knobs"] = serde_json::to_value(spec.resolved()).unwrap();
            if let Some(arr) = receipt["notes"].as_array_mut() {
                arr.extend(notes.iter().map(|n| serde_json::Value::String(n.clone())));
            }
            if let Some(path) = value(&args, "--receipt") {
                std::fs::write(&path, serde_json::to_string_pretty(&receipt).unwrap()).unwrap_or_else(|e| fail(e));
            }
            let r = &receipt;
            eprintln!(
                "{}: {:.2} s in {:.2} s ({:.1}x real time, build {:.2} s); out peak {:.1} dBFS rms {:.1} dBFS, {} clipped; voice work {:.3e} J, radiated {:.3e} J, lost {:.3e} J; max frame residual {:.1e} J; bandwidth {:.0} Hz",
                spec.name,
                audio_seconds,
                render_seconds,
                factor,
                build_seconds,
                r["output_peak_dbfs"].as_f64().unwrap_or(0.0),
                r["output_rms_dbfs"].as_f64().unwrap_or(0.0),
                r["clipped_samples"],
                r["voice_work_j"].as_f64().unwrap_or(0.0),
                r["ledger"]["radiation_j"].as_f64().unwrap_or(0.0),
                r["ledger"]["internal_loss_j"].as_f64().unwrap_or(0.0),
                r["ledger"]["maximum_frame_residual_j"].as_f64().unwrap_or(0.0),
                r["bandwidth_hz"].as_f64().unwrap_or(0.0),
            );
            for n in notes {
                eprintln!("  note: {n}");
            }
        }
        "inspect" => {
            let body_name = value(&args, "--body").unwrap_or_else(|| usage());
            let mut spec = load_body(&body_name).unwrap_or_else(|e| fail(e));
            let notes = apply_sets(&mut spec, &args).unwrap_or_else(|e| fail(e));
            let seconds: f64 = value(&args, "--seconds").and_then(|s| s.parse().ok()).unwrap_or(3.0);
            let level: f64 = value(&args, "--level").and_then(|s| s.parse().ok()).unwrap_or(0.1);
            let body = Body::new(&spec, RATE).unwrap_or_else(|e| fail(e));
            let transfer = Body::transfer(&spec, RATE, level, seconds).unwrap_or_else(|e| fail(e));
            let receipt = body.receipt();
            if args.iter().any(|a| a == "--json") {
                let v = serde_json::json!({
                    "body": spec.name,
                    "knobs": spec.resolved(),
                    "layout": receipt.layout,
                    "modes": receipt.modes,
                    "bandwidth_hz": receipt.bandwidth_hz,
                    "transfer": transfer,
                    "notes": notes,
                });
                println!("{}", serde_json::to_string_pretty(&v).unwrap());
                return;
            }
            println!("{}: {}", spec.name, spec.about);
            let l = &receipt.layout;
            println!(
                "air: {} volumes (bore {} x {:.1} mm), drive at volume {}, mouth volume {}, {} radiating end(s), bandwidth {:.0} Hz (mode 24)",
                l.nodes, l.bore_sections, l.bore_dx_m * 1e3, l.drive_node, l.mouth_node, l.ends.len(), receipt.bandwidth_hz
            );
            for end in &l.ends {
                println!("  end {:<8} tube {} volume {:3}  x {:+.2} m  column {:.1} mm", end.label, end.tube, end.node, end.x_m, end.column_m * 1e3);
            }
            for rad in &l.radiators {
                println!(
                    "  {:<8} {} touching volume {} at {:.0} Hz; partials {}",
                    rad.label,
                    rad.kind,
                    rad.touches_node,
                    rad.coupling_hz,
                    rad.frequency_hz.iter().take(8).map(|f| format!("{f:.0}")).collect::<Vec<_>>().join(" ")
                );
            }
            println!("modes of the air (Hz, wall T60 ms, radiation sigma 1/s per end, |shape| at drive/mouth):");
            let air = body.world().space_body(0).unwrap();
            let drive_shape = air.node_shape(l.drive_node).unwrap_or([0.0; 24]);
            for (k, m) in receipt.modes.iter().enumerate() {
                let sig: Vec<String> = m.radiation_sigma_per_s.iter().take(l.ends.len()).map(|s| format!("{s:7.1}")).collect();
                println!(
                    "  {:2} {:8.1}  {:8.2}  {}  {:6.3} {:6.3}",
                    k + 1,
                    m.hz,
                    if m.wall_t60_s.is_finite() { m.wall_t60_s * 1e3 } else { f64::INFINITY },
                    sig.join(" "),
                    drive_shape[k].abs(),
                    m.mouth_shape.abs()
                );
            }
            println!(
                "transfer (white noise {level} rms full scale, {seconds} s): mean {:.1} dB over 150-8000 Hz, ripple +/-{:.1} dB",
                transfer.mean_db_150_8000, transfer.ripple_db_150_8000
            );
            println!("  peaks:   {}", transfer.peaks_hz.iter().map(|(f, g)| format!("{f:.0} Hz ({g:+.1})")).collect::<Vec<_>>().join(", "));
            println!("  notches: {}", transfer.notches_hz.iter().map(|(f, g)| format!("{f:.0} Hz ({g:+.1})")).collect::<Vec<_>>().join(", "));
            println!("  third-octave gain (dB, mic full scale to output full scale, power-averaged per band):");
            let mut line = String::new();
            for (f, g) in transfer.band_hz.iter().zip(&transfer.band_db) {
                line.push_str(&format!("{:.0}:{:+.1} ", f, g));
            }
            println!("  {line}");
            for n in notes.iter().chain(&receipt.notes) {
                println!("  note: {n}");
            }
        }
        "analyze" => {
            // Long-term third-octave spectrum of WAV files (a measurement aid, no physics).
            let files: Vec<&String> = args.iter().filter(|a| a.ends_with(".wav")).collect();
            if files.is_empty() {
                usage();
            }
            let mut rows: Vec<(String, Vec<f64>, f64, f64, f64)> = Vec::new();
            let mut centres = Vec::new();
            for f in &files {
                let (x, _) = read_wav(Path::new(f)).unwrap_or_else(|e| fail(e));
                let (c, bands, centroid, rms) = ovillo_body::body::ltas(&x, RATE);
                centres = c;
                let peak = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
                rows.push((f.to_string(), bands, centroid, rms, f64::from(peak)));
            }
            print!("{:>7}", "Hz");
            for r in &rows {
                print!(" {:>12}", Path::new(&r.0).file_stem().and_then(|s| s.to_str()).unwrap_or("?"));
            }
            println!();
            for (i, c) in centres.iter().enumerate() {
                print!("{:>7.0}", c);
                for r in &rows {
                    print!(" {:>12.1}", r.1[i]);
                }
                println!();
            }
            print!("{:>7}", "centroid");
            for r in &rows {
                print!(" {:>12.0}", r.2);
            }
            println!("   (Hz, of the 200-5000 Hz power)");
            print!("{:>7}", "rms/pk");
            for r in &rows {
                print!(" {:>5.1}/{:<6.1}", 20.0 * r.3.max(1e-9).log10(), 20.0 * r.4.max(1e-9).log10());
            }
            println!("   (dBFS)");
        }
        "plan" => {
            let body_name = value(&args, "--body").unwrap_or_else(|| usage());
            let mut spec = load_body(&body_name).unwrap_or_else(|e| fail(e));
            apply_sets(&mut spec, &args).unwrap_or_else(|e| fail(e));
            let (plan, _) = ovillo_body::build::plan(&spec, RATE, spec.get("mouth.open")).unwrap_or_else(|e| fail(e));
            println!("{}", serde_json::to_string_pretty(&plan).unwrap());
        }
        _ => usage(),
    }
}
