//! Play the v1 trombone World (`ovillo_dsp::trombone_nd`, `TROMBONE-WORLD.md`)
//! from a note list in a TSV file, instead of the note lists compiled into
//! `trombone_nd_solo.rs`. Nothing about the World, the player or the
//! calibration changes: the notes are placed by `trombone_nd_player::place`
//! and blown by `trombone_nd_player::perform`, exactly as the solo is.
//!
//! ```text
//! cargo run --release -p ovillo-dsp --example trombone_nd_score -- <notes.tsv> <out-dir> <stem>
//! ```
//!
//! The note list (tab separated, `#` starts a comment line):
//!
//! ```text
//! start_s  end_s  midi  vel  vel_end  art     fx
//! 0.50     3.20   50    0.20 0.85     tongue  vib,swell
//! ```
//!
//! - `vel`, 0..1: the written dynamic, a target level the player blows the
//!   calibrated pressure for (as in the solo).
//! - `vel_end`: when it differs from `vel`, the player blows a hairpin: the
//!   mouth pressure the performance computes is scaled, sample by sample, from
//!   the calibrated pressure for `vel` to the calibrated pressure for
//!   `vel_end` (log-interpolated over the sounding part of the note, smoothed).
//!   It is still the player's mouth pressure, the World's one source; nothing
//!   is applied to the sound.
//! - `art`: `tongue`, `slur` or `smear` (`trombone_player::Art`).
//! - `fx`: `-` or a comma list of `scoop`, `fall`, `vib`, `rip`, `swell`, `dim`.
//!
//! Writes `<stem>.wav` (mono f32, exactly the World's emission times
//! `OUTPUT_SCALE`, sample for sample), `<stem>-notes.tsv` (with each note's
//! partial and slide), `<stem>-receipt.json` (the World's books) and
//! `<stem>.energy.json` (`mus.ariadne.energy-trace.v1` shape, as the solo).

use ovillo_dsp::trombone_nd::{EnergyTrace, TromboneNd, OUTPUT_SCALE, PLAN_JSON};
use ovillo_dsp::trombone_nd_player::{perform, place, ModeTable, Skill};
use ovillo_dsp::trombone_player::{Art, Note, DIM, FALL, RIP, SCOOP, SWELL, VIB};

const SR: f64 = 48_000.0;
const TAIL_S: f64 = 2.5;

fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn parse(text: &str) -> Result<(Vec<Note>, Vec<f64>), String> {
    let mut notes = Vec::new();
    let mut ends = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("start_s") {
            continue;
        }
        let f: Vec<&str> = line.split('\t').map(str::trim).collect();
        if f.len() < 7 {
            return Err(format!(
                "line {}: expected 7 tab-separated fields, found {}",
                i + 1,
                f.len()
            ));
        }
        let num = |k: usize| {
            f[k].parse::<f64>().map_err(|_| {
                format!(
                    "line {}: field {} is not a number: {:?}",
                    i + 1,
                    k + 1,
                    f[k]
                )
            })
        };
        let art = match f[5] {
            "tongue" => Art::Tongue,
            "slur" => Art::Slur,
            "smear" => Art::Smear,
            other => {
                return Err(format!(
                    "line {}: art {other:?} is not tongue|slur|smear",
                    i + 1
                ))
            }
        };
        let mut fx = 0u8;
        if f[6] != "-" {
            for name in f[6].split(',') {
                fx |= match name.trim() {
                    "scoop" => SCOOP,
                    "fall" => FALL,
                    "vib" => VIB,
                    "rip" => RIP,
                    "swell" => SWELL,
                    "dim" => DIM,
                    other => {
                        return Err(format!(
                            "line {}: fx {other:?} is not scoop|fall|vib|rip|swell|dim",
                            i + 1
                        ))
                    }
                };
            }
        }
        let (start_s, end_s, midi, vel, vel_end) = (num(0)?, num(1)?, num(2)?, num(3)?, num(4)?);
        if !(end_s > start_s) || !(0.0..=1.0).contains(&vel) || !(0.0..=1.0).contains(&vel_end) {
            return Err(format!(
                "line {}: need end_s > start_s and velocities in 0..1",
                i + 1
            ));
        }
        if let Some(prev) = notes.last() {
            let prev: &Note = prev;
            if start_s < prev.start_s {
                return Err(format!(
                    "line {}: notes must be in time order (one player, one bore)",
                    i + 1
                ));
            }
        }
        notes.push(Note {
            midi,
            start_s,
            end_s,
            vel,
            art,
            fx,
        });
        ends.push(vel_end);
    }
    if notes.is_empty() {
        return Err("no notes".into());
    }
    Ok((notes, ends))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: trombone_nd_score <notes.tsv> <out-dir> <stem>");
        std::process::exit(2);
    }
    let text = std::fs::read_to_string(&args[1]).expect("read the note list");
    let out_dir = std::path::PathBuf::from(&args[2]);
    let stem = args[3].clone();
    let (notes, vel_end) = parse(&text).unwrap_or_else(|e| {
        eprintln!("{}: {e}", args[1]);
        std::process::exit(2);
    });
    let table = ModeTable::embedded().expect("mode table");
    let skill = Skill::embedded().expect("player skill");
    let mut tw = TromboneNd::new().expect("trombone World");
    let placed = place(&table, &skill, &notes);
    let mut breaths = perform(&table, &skill, &notes, &placed, SR, TAIL_S);

    // Hairpins: the player's mouth pressure, moved between two calibrated
    // pressures of the same cell. Only `Breath::mouth` changes.
    let mut hairpins = 0usize;
    for (i, note) in notes.iter().enumerate() {
        let to = vel_end[i];
        if (to - note.vel).abs() < 1e-9 {
            continue;
        }
        hairpins += 1;
        let pl = placed[i];
        let emb = |v: f64| {
            skill
                .embouchure_near(&table, pl.partial, pl.slide_m, v)
                .expect("a calibrated cell")
                .mouth
        };
        let (m_written, m0, m1) = (emb(note.vel), emb(note.vel), emb(to));
        let s0 = (note.start_s * SR) as usize;
        let owned_until = if i + 1 < notes.len() {
            (notes[i + 1].start_s * SR) as usize
        } else {
            breaths.len()
        };
        let sounding = (note.end_s - note.start_s) * 0.90;
        for t in s0..owned_until.min(breaths.len()) {
            let tau = t as f64 / SR - note.start_s;
            let k = smoothstep(tau / sounding);
            let factor = (m0.ln() + k * (m1.ln() - m0.ln())).exp() / m_written;
            breaths[t].mouth = (breaths[t].mouth * factor).clamp(0.0, 1.0);
        }
    }

    tw.set_slide_now(breaths[0].slide_m).expect("slide");
    let mut audio = Vec::with_capacity(breaths.len());
    let mut energy = EnergyTrace::new(SR);
    let t0 = std::time::Instant::now();
    for b in &breaths {
        audio.push(tw.tick(*b).expect("the World stepped"));
        energy.observe(&tw);
    }
    let took = t0.elapsed().as_secs_f64();
    let seconds = breaths.len() as f64 / SR;
    std::fs::create_dir_all(&out_dir).unwrap();
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SR as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let wav = out_dir.join(format!("{stem}.wav"));
    let mut w = hound::WavWriter::create(&wav, spec).unwrap();
    for x in &audio {
        w.write_sample(*x as f32).unwrap();
    }
    w.finalize().unwrap();
    let mut tsv = String::from(
        "start_s\tend_s\tmidi\tpartial\tslide\tposition\tart\tfx\tvel\tvel_end\tbore_hz\n",
    );
    for ((x, p), e) in notes.iter().zip(&placed).zip(&vel_end) {
        tsv += &format!(
            "{:.4}\t{:.4}\t{}\t{}\t{:.5}\t{}\t{:?}\t{}\t{:.2}\t{:.2}\t{:.3}\n",
            x.start_s,
            x.end_s,
            x.midi,
            p.partial,
            p.slide_m,
            p.position(&table),
            x.art,
            x.fx,
            x.vel,
            e,
            table.mode_hz(p.partial, p.slide_m)
        );
    }
    std::fs::write(out_dir.join(format!("{stem}-notes.tsv")), tsv).unwrap();
    let touches = notes
        .iter()
        .zip(&placed)
        .map(|(x, p)| {
            serde_json::json!({ "seconds": x.start_s, "kind": format!("{:?}", x.art).to_lowercase(), "midi": x.midi,
                "partial": p.partial, "position": p.position(&table) })
        })
        .collect();
    let body = tw.world().body_id(0).unwrap_or("trombone").to_owned();
    std::fs::write(
        out_dir.join(format!("{stem}.energy.json")),
        serde_json::to_string(&energy.to_json(&body, touches)).unwrap(),
    )
    .unwrap();
    let r = tw.world().receipt();
    let m = tw.moves();
    let peak = audio.iter().fold(0.0f64, |a, v| a.max(v.abs()));
    let receipt = serde_json::json!({
        "schema": "ovillo.trombone-nd-render.v1",
        "piece": stem,
        "noteList": args[1],
        "planDigest": r.plan_digest,
        "planJsonBytes": PLAN_JSON.len(),
        "outputScale": OUTPUT_SCALE,
        "sampleRateHz": SR,
        "seconds": seconds,
        "renderWallSeconds": took,
        "realTimeFactorWall": seconds / took,
        "peak": peak,
        "notes": notes.len(),
        "hairpins": hairpins,
        "partials": placed.iter().map(|p| p.partial).collect::<Vec<_>>(),
        "positions": placed.iter().map(|p| p.position(&table)).collect::<Vec<_>>(),
        "ledger": {
            "sampleFrames": r.sample_frames, "initialEnergy": r.initial_energy, "finalEnergy": r.current_energy,
            "sourceWork": r.source_work, "controlWork": r.control_work, "internalLoss": r.internal_loss, "radiation": r.radiation,
            "cumulativeClosureResidual": r.cumulative_closure_residual, "globalClosureResidual": r.global_closure_residual,
            "maximumFrameResidual": r.maximum_frame_residual, "maximumLocalResidual": r.maximum_local_residual,
        },
        "moves": m,
    });
    std::fs::write(
        out_dir.join(format!("{stem}-receipt.json")),
        serde_json::to_string_pretty(&receipt).unwrap(),
    )
    .unwrap();
    println!("{stem}: {seconds:.1} s in {took:.2} s wall ({:.2}x real time), peak {:.2} dBFS, {} notes, {hairpins} hairpins", seconds / took, 20.0 * peak.log10(), notes.len());
    println!(
        "ledger: source {:.6e} control {:.6e} loss {:.6e} radiation {:.6e}",
        r.source_work, r.control_work, r.internal_loss, r.radiation
    );
    println!(
        "closure: cumulative {:.3e} max frame {:.3e} over {} frames",
        r.cumulative_closure_residual, r.maximum_frame_residual, r.sample_frames
    );
    println!(
        "partials: {:?}",
        placed.iter().map(|p| p.partial).collect::<Vec<_>>()
    );
    println!("wrote {}", wav.display());
}
