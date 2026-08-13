//! Export only MUS dry mono stems without also rendering the stereo master.
//!
//! Usage: `cargo run -p mus-cli --release --example render_stems -- SCORE.mus STEM_DIR [BASE_DIR]`

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(error) = run() {
        eprintln!("mus-stem-render: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let score = PathBuf::from(args.next().ok_or_else(usage)?);
    let output_dir = PathBuf::from(args.next().ok_or_else(usage)?);
    let base = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(usage());
    }
    let source =
        fs::read_to_string(&score).map_err(|error| format!("{}: {error}", score.display()))?;
    let score_dir = base
        .as_deref()
        .or_else(|| score.parent())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("{}: {error}", output_dir.display()))?;
    let (stems, stats) =
        mus_engine::render_stems(&source, score_dir).map_err(|error| error.to_string())?;
    let mut written = Vec::with_capacity(stems.len());
    for stem in stems {
        let path = output_dir.join(format!("{}.wav", stem.abbrev));
        mus_engine::write_wav_mono(&stem.samples, &path).map_err(|error| error.to_string())?;
        written.push(serde_json::json!({
            "abbrev": stem.abbrev,
            "path": path.display().to_string(),
            "frames": stem.samples.len(),
        }));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": "mus.audio.stem-export-receipt.v1",
            "source": score.display().to_string(),
            "sampleRate": mus_engine::SAMPLE_RATE,
            "eventsRendered": stats.notes,
            "eventsSkipped": stats.skipped,
            "warnings": stats.warnings,
            "badTokens": stats.bad,
            "stems": written,
        }))
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn usage() -> String {
    "usage: render_stems SCORE.mus STEM_DIR [BASE_DIR]".to_owned()
}
