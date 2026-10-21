mod check;
mod service;
mod vocab_dump;

use mus_engine::receipt::{energy_trace_json, head_selection, RenderReceipt as Receipt};
use mus_engine::{render, render_bars, sha256_bytes, write_wav};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn main() {
    if let Err(error) = run() {
        eprintln!("mus: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        return Err(usage());
    };
    if command == "--help" || command == "-h" {
        println!("{}", usage());
        return Ok(());
    }
    if command == "service" {
        return service::run_service();
    }
    if command == "vocab" {
        println!(
            "{}",
            serde_json::to_string_pretty(&vocab_dump::dump()).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if command == "check" {
        let first = args.next().ok_or_else(usage)?;
        if first == "--help" || first == "-h" {
            println!("{}", usage());
            return Ok(());
        }
        let score = PathBuf::from(first);
        let mut base = None;
        let mut json_output = false;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--base" => base = Some(PathBuf::from(args.next().ok_or_else(usage)?)),
                "--json" => json_output = true,
                "--help" | "-h" => {
                    println!("{}", usage());
                    return Ok(());
                }
                _ => return Err(format!("unknown argument {arg}\n{}", usage())),
            }
        }
        let report = check::run(&score, base.as_deref())?;
        if json_output {
            println!(
                "{}",
                serde_json::to_string(&report).map_err(|e| e.to_string())?
            );
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
            );
        }
        return Ok(());
    }
    if command == "scene-render" {
        let first = args.next().ok_or_else(usage)?;
        if first == "--help" || first == "-h" {
            println!("{}", usage());
            return Ok(());
        }
        let scene_path = PathBuf::from(first);
        let mut root: Option<PathBuf> = None;
        let mut output: Option<PathBuf> = None;
        let mut foa = false;
        let mut compact = false;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => root = Some(PathBuf::from(args.next().ok_or_else(usage)?)),
                "-o" => output = Some(PathBuf::from(args.next().ok_or_else(usage)?)),
                "--foa" => foa = true,
                "--json" => compact = true,
                "--help" | "-h" => {
                    println!("{}", usage());
                    return Ok(());
                }
                _ => return Err(format!("unknown argument {arg}\n{}", usage())),
            }
        }
        let root =
            root.ok_or_else(|| format!("scene-render requires --root <dir>\n{}", usage()))?;
        let output = output.unwrap_or_else(|| scene_path.with_extension("wav"));
        return match run_scene_render(&scene_path, &root, &output, foa) {
            Ok(receipt) => {
                let text = if compact {
                    serde_json::to_string(&receipt)
                } else {
                    serde_json::to_string_pretty(&receipt)
                }
                .map_err(|e| e.to_string())?;
                println!("{text}");
                Ok(())
            }
            Err(message) => {
                // Same "refusals as structured errors" contract as `render`
                // (see below): a JSON object on stdout either way.
                let error = serde_json::json!({
                    "schema": "mus.spatial-render-error.v1",
                    "error": message,
                });
                println!("{error}");
                Err(message)
            }
        };
    }
    if command != "render" {
        return Err(usage());
    }
    let first = args.next().ok_or_else(usage)?;
    if first == "--help" || first == "-h" {
        println!("{}", usage());
        return Ok(());
    }
    let score = PathBuf::from(first);
    let mut output = None;
    let mut base = None;
    let mut compact = false;
    let mut stems_dir = None;
    let mut energy_path = None;
    let mut bars = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" => output = Some(PathBuf::from(args.next().ok_or_else(usage)?)),
            "--base" => base = Some(PathBuf::from(args.next().ok_or_else(usage)?)),
            "--stems" => stems_dir = Some(PathBuf::from(args.next().ok_or_else(usage)?)),
            "--energy" => energy_path = Some(PathBuf::from(args.next().ok_or_else(usage)?)),
            "--bars" => bars = Some(parse_bars_range(&args.next().ok_or_else(usage)?)?),
            "--json" => compact = true,
            "--help" | "-h" => {
                println!("{}", usage());
                return Ok(());
            }
            _ => return Err(format!("unknown argument {arg}\n{}", usage())),
        }
    }
    let output = output.unwrap_or_else(|| score.with_extension("wav"));
    match run_render(
        &score,
        &output,
        base.as_deref(),
        stems_dir.as_deref(),
        energy_path.as_deref(),
        bars,
    ) {
        Ok(receipt) => {
            let text = if compact {
                serde_json::to_string(&receipt)
            } else {
                serde_json::to_string_pretty(&receipt)
            }
            .map_err(|e| e.to_string())?;
            println!("{text}");
            Ok(())
        }
        Err(message) => {
            // "refusals as structured errors" — a render failure still
            // prints a JSON object on stdout (mirroring `check`'s always-
            // JSON contract) rather than only a plain-text line; `main()`'s
            // generic handler still prints the human-readable form to
            // stderr and sets the exit code.
            let error = serde_json::json!({
                "schema": "mus.audio.render-error.v1",
                "error": message,
            });
            println!("{error}");
            Err(message)
        }
    }
}

fn run_render(
    score: &std::path::Path,
    output: &std::path::Path,
    base: Option<&std::path::Path>,
    stems_dir: Option<&std::path::Path>,
    energy_path: Option<&std::path::Path>,
    bars: Option<(u32, u32)>,
) -> Result<Receipt, String> {
    let started = Instant::now();
    let source = fs::read_to_string(score).map_err(|e| format!("{}: {e}", score.display()))?;
    let score_dir = base
        .or_else(|| score.parent())
        .unwrap_or_else(|| std::path::Path::new("."));

    // `headSelection` is the graph-shaped view of the score (see
    // `mus_engine::receipt::head_selection`).
    let head_selection = head_selection(&source);

    let outcome = match bars {
        Some((from, to)) => render_bars(&source, score_dir, from, to).map_err(|e| e.to_string())?,
        None => render(&source, score_dir).map_err(|e| e.to_string())?,
    };
    write_wav(&outcome.audio, output).map_err(|e| e.to_string())?;
    if let Some(path) = energy_path {
        let trace = energy_trace_json(&outcome.stats);
        let partial = path.with_extension("json.partial");
        fs::write(
            &partial,
            serde_json::to_vec(&trace).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("{}: {e}", partial.display()))?;
        fs::rename(&partial, path).map_err(|e| format!("{}: {e}", path.display()))?;
    }

    let stems = match stems_dir {
        Some(dir) => {
            fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let (stems, _stem_stats) =
                mus_engine::render_stems(&source, score_dir).map_err(|e| e.to_string())?;
            let mut written = BTreeMap::new();
            for stem in &stems {
                let path = dir.join(format!("{}.wav", stem.abbrev));
                mus_engine::write_wav_mono(&stem.samples, &path).map_err(|e| e.to_string())?;
                written.insert(stem.abbrev.clone(), path.display().to_string());
            }
            Some(written)
        }
        None => None,
    };

    let wav = fs::read(output).map_err(|e| format!("{}: {e}", output.display()))?;
    let source_digest = format!("sha256:{}", sha256_bytes(source.as_bytes()));
    let mut receipt = Receipt::from_stats(
        &outcome.stats,
        format!("sha256:{}", sha256_bytes(&wav)),
        source_digest,
        head_selection,
        outcome.audio.peak_dbfs(),
        outcome.audio.rms_dbfs(),
        started.elapsed().as_secs_f64(),
    );
    receipt.stems = stems;
    Ok(receipt)
}

/// `mus scene-render <scene.json> --root <dir> [-o out.wav] [--foa]`: the
/// spatial scene renderer (PW7). Prints `mus_dsp::scene::RenderReceipt`
/// as JSON (matching `render`'s always-JSON-on-stdout, refusal-or-not,
/// contract above) and writes the rendered PCM as a `FLOAT`-subtype WAV
/// — stereo equal-power-plus-diffuse by default, 4-channel ACN/SN3D FOA
/// under `--foa`.
fn run_scene_render(
    scene_path: &Path,
    root: &Path,
    output: &Path,
    foa: bool,
) -> Result<mus_dsp::scene::RenderReceipt, String> {
    let scene = mus_dsp::scene::SpatialScene::read(scene_path).map_err(|e| e.to_string())?;
    let (interleaved, channels, receipt) = if foa {
        let (pcm, receipt) = mus_dsp::scene::render_foa(&scene, root).map_err(|e| e.to_string())?;
        (pcm, 4u16, receipt)
    } else {
        let (pcm, receipt) =
            mus_dsp::scene::render_stereo(&scene, root).map_err(|e| e.to_string())?;
        (pcm, 2u16, receipt)
    };
    mus_dsp::scene::write_wav_f32(&interleaved, channels, scene.sample_rate, output)
        .map_err(|e| e.to_string())?;
    Ok(receipt)
}

fn parse_bars_range(s: &str) -> Result<(u32, u32), String> {
    let (from, to) = if let Some((a, b)) = s.split_once("..") {
        (a, b)
    } else if let Some((a, b)) = s.split_once('-') {
        (a, b)
    } else {
        (s, s)
    };
    let from: u32 = from
        .trim()
        .parse()
        .map_err(|_| format!("--bars: not a bar number: {from:?}"))?;
    let to: u32 = to
        .trim()
        .parse()
        .map_err(|_| format!("--bars: not a bar number: {to:?}"))?;
    if from == 0 || to < from {
        return Err(format!(
            "--bars: expected FROM..TO (or FROM-TO) with 1 <= FROM <= TO, got {from}..{to}"
        ));
    }
    Ok((from, to))
}

fn usage() -> String {
    "usage:\n  mus check <score.mus> [--base DIR] [--json]\n  mus render <score.mus> [--base DIR] [-o <out.wav>] [--stems DIR] [--energy <trace.json>] [--bars FROM..TO] [--json]\n  mus scene-render <scene.json> --root <dir> [-o <out.wav>] [--foa] [--json]\n\nAtril swap:\n  MUS_CHECK_CMD=\"<repo>/mus-rs/target/release/mus check\"".into()
}
