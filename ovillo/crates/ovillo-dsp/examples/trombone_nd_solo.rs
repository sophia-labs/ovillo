//! Play the v1 trombone World (one N-D body; see `ovillo_dsp::trombone_nd`).
//!
//! ```text
//! cargo run --release -p ovillo-dsp --example trombone_nd_solo -- <out-dir> [solo|etude|dynamics]
//! ```
//! - `solo`: `examples/trombone_solo.rs`'s notes and timing (its growl has no
//!   physical mechanism in this World and is not played).
//! - `etude`: partials 2..6 up and down in each slide position 1..7.
//! - `dynamics`: F3, Bb3, D4, F4 in first position, each tongued at five dynamics.
//!
//! Writes `trombone-nd-<piece>.wav` (mono float, exactly the World's emission
//! times `OUTPUT_SCALE`, sample for sample), `...-notes.tsv`, `...-receipt.json`.
//! Needs `patches/mus-ariadne-tube-length.patch` applied to mus-ariadne.

use ovillo_dsp::trombone_nd::{TromboneNd, OUTPUT_SCALE, PLAN_JSON};
use ovillo_dsp::trombone_nd_player::{perform, place, ModeTable, Placement, Skill};
use ovillo_dsp::trombone_player::{hz_of, Art, Note, DIM, FALL, RIP, SCOOP, SWELL, VIB};

const BPM: f64 = 66.0;
const GROWL: u8 = 8;

struct Written {
    midi: f64,
    beat: f64,
    len: f64,
    vel: f64,
    art: Art,
    fx: u8,
}

const fn n(midi: f64, beat: f64, len: f64, vel: f64, art: Art, fx: u8) -> Written {
    Written { midi, beat, len, vel, art, fx }
}

use Art::{Slur, Smear, Tongue};

// The note list of examples/trombone_solo.rs, unchanged.
const SCORE: &[Written] = &[
    n(60.0, 0.5, 0.5, 0.40, Tongue, SCOOP),
    n(63.0, 1.0, 0.5, 0.45, Slur, 0),
    n(65.0, 1.5, 2.0, 0.55, Slur, VIB | SWELL),
    n(68.0, 3.5, 0.5, 0.50, Tongue, 0),
    n(67.0, 4.0, 0.5, 0.48, Smear, 0),
    n(65.0, 4.5, 0.5, 0.45, Slur, 0),
    n(63.0, 5.0, 0.5, 0.42, Tongue, 0),
    n(60.0, 5.5, 2.0, 0.46, Smear, VIB | DIM),
    n(56.0, 8.5, 0.5, 0.40, Tongue, SCOOP),
    n(60.0, 9.0, 0.5, 0.45, Slur, 0),
    n(63.0, 9.5, 0.75, 0.50, Slur, 0),
    n(61.0, 10.25, 0.25, 0.45, Tongue, 0),
    n(60.0, 10.5, 1.0, 0.50, Slur, VIB),
    n(58.0, 11.5, 0.5, 0.45, Tongue, 0),
    n(56.0, 12.0, 0.5, 0.42, Slur, 0),
    n(53.0, 12.5, 2.25, 0.50, Smear, VIB | FALL),
    n(65.0, 16.5, 0.5, 0.50, Tongue, SCOOP),
    n(68.0, 17.0, 0.5, 0.58, Slur, 0),
    n(70.0, 17.5, 0.5, 0.65, Slur, 0),
    n(72.0, 18.0, 2.0, 0.85, Tongue, RIP | GROWL | VIB | SWELL),
    n(70.0, 20.0, 1.0 / 3.0, 0.70, Tongue, 0),
    n(68.0, 20.0 + 1.0 / 3.0, 1.0 / 3.0, 0.65, Tongue, 0),
    n(65.0, 20.0 + 2.0 / 3.0, 1.0 / 3.0, 0.60, Tongue, 0),
    n(68.0, 21.0, 0.5, 0.60, Tongue, 0),
    n(67.0, 21.5, 0.5, 0.55, Smear, 0),
    n(65.0, 22.0, 0.5, 0.52, Slur, 0),
    n(63.0, 22.5, 1.5, 0.50, Smear, VIB | DIM),
    n(60.0, 24.5, 0.5, 0.40, Tongue, 0),
    n(63.0, 25.0, 0.5, 0.45, Slur, 0),
    n(65.0, 25.5, 0.5, 0.50, Slur, 0),
    n(61.0, 26.0, 1.0, 0.50, Smear, VIB),
    n(60.0, 27.0, 1.0, 0.45, Smear, VIB),
    n(56.0, 28.0, 0.5, 0.40, Tongue, 0),
    n(58.0, 28.5, 0.5, 0.42, Slur, 0),
    n(53.0, 29.0, 1.5, 0.45, Slur, VIB | DIM),
    n(41.0, 31.0, 3.5, 0.42, Tongue, SCOOP | VIB | DIM | FALL),
];

/// Beats to seconds, with swung off-beat eighths and a little rubato (as trombone_solo).
fn time_of(beat: f64, seed: u64) -> f64 {
    let whole = beat.floor();
    let frac = beat - whole;
    let swung = if (frac - 0.5).abs() < 1e-9 { 0.6 } else { frac };
    let jitter = ((seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40) as f64 / (1u64 << 24) as f64 - 0.5) * 0.024;
    (whole + swung) * 60.0 / BPM + jitter
}

fn solo() -> Vec<Note> {
    SCORE
        .iter()
        .enumerate()
        .map(|(i, x)| Note {
            midi: x.midi,
            start_s: time_of(x.beat, i as u64 + 1),
            end_s: time_of(x.beat + x.len, i as u64 + 1),
            vel: x.vel,
            art: x.art,
            fx: x.fx & !GROWL,
        })
        .collect()
}

fn etude(table: &ModeTable, skill: &Skill) -> (Vec<Note>, Vec<Placement>) {
    let mut notes = Vec::new();
    let mut placed = Vec::new();
    let mut t = 0.4;
    let bb1 = hz_of(34.0);
    for pos in 1..=7 {
        for (j, &k) in [2usize, 3, 4, 5, 6, 5, 4, 3, 2].iter().enumerate() {
            let natural = k as f64 * bb1 * 2f64.powf(-((pos - 1) as f64) / 12.0);
            let midi = (69.0 + 12.0 * (natural / 440.0).log2()).round();
            let len = 0.55;
            notes.push(Note { midi, start_s: t, end_s: t + len, vel: 0.5, art: if j == 0 { Art::Tongue } else { Art::Slur }, fx: 0 });
            let x = skill.slide_for(table, k, hz_of(midi), 0.5).clamp(0.0, table.max_extension());
            placed.push(Placement { partial: k, slide_m: x });
            t += len;
        }
        t += 0.45;
    }
    (notes, placed)
}

fn dynamics(table: &ModeTable, skill: &Skill) -> (Vec<Note>, Vec<Placement>) {
    let mut notes = Vec::new();
    let mut placed = Vec::new();
    let mut t = 0.4;
    for &(midi, k) in &[(53.0, 3usize), (58.0, 4), (62.0, 5), (65.0, 6)] {
        for step in 0..5 {
            let vel = step as f64 / 4.0;
            notes.push(Note { midi, start_s: t, end_s: t + 1.1, vel, art: Art::Tongue, fx: 0 });
            placed.push(Placement { partial: k, slide_m: skill.slide_for(table, k, hz_of(midi), vel).clamp(0.0, table.max_extension()) });
            t += 1.4;
        }
        t += 0.4;
    }
    (notes, placed)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_dir = std::path::PathBuf::from(args.get(1).cloned().unwrap_or_else(|| ".".into()));
    let piece = args.get(2).cloned().unwrap_or_else(|| "solo".into());
    let sr = 48_000.0;
    let table = ModeTable::embedded().expect("mode table");
    let skill = Skill::embedded().expect("player skill");
    let mut tw = TromboneNd::new().expect("trombone World (needs the tube-length patch)");
    let (notes, placed) = match piece.as_str() {
        "etude" => etude(&table, &skill),
        "dynamics" => dynamics(&table, &skill),
        _ => {
            let notes = solo();
            let placed = place(&table, &skill, &notes);
            (notes, placed)
        }
    };
    let breaths = perform(&table, &skill, &notes, &placed, sr, 2.5);
    tw.set_slide_now(breaths[0].slide_m).expect("slide");
    let mut audio = Vec::with_capacity(breaths.len());
    let t0 = std::time::Instant::now();
    for b in &breaths {
        audio.push(tw.tick(*b).expect("the World stepped"));
    }
    let took = t0.elapsed().as_secs_f64();
    let seconds = breaths.len() as f64 / sr;
    std::fs::create_dir_all(&out_dir).unwrap();
    let stem = format!("trombone-nd-{piece}");
    let spec = hound::WavSpec { channels: 1, sample_rate: sr as u32, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let wav = out_dir.join(format!("{stem}.wav"));
    let mut w = hound::WavWriter::create(&wav, spec).unwrap();
    for x in &audio {
        w.write_sample(*x as f32).unwrap();
    }
    w.finalize().unwrap();
    let mut tsv = String::from("start_s\tend_s\tmidi\tpartial\tslide\tposition\tart\tfx\tvel\tbore_hz\n");
    for (x, p) in notes.iter().zip(&placed) {
        tsv += &format!(
            "{:.4}\t{:.4}\t{}\t{}\t{:.5}\t{}\t{:?}\t{}\t{:.2}\t{:.3}\n",
            x.start_s, x.end_s, x.midi, p.partial, p.slide_m, p.position(&table), x.art, x.fx, x.vel, table.mode_hz(p.partial, p.slide_m)
        );
    }
    std::fs::write(out_dir.join(format!("{stem}-notes.tsv")), tsv).unwrap();
    let r = tw.world().receipt();
    let m = tw.moves();
    let peak = audio.iter().fold(0.0f64, |a, v| a.max(v.abs()));
    let receipt = serde_json::json!({
        "schema": "ovillo.trombone-nd-render.v1",
        "piece": piece,
        "planDigest": r.plan_digest,
        "planJsonBytes": PLAN_JSON.len(),
        "outputScale": OUTPUT_SCALE,
        "seconds": seconds,
        "renderWallSeconds": took,
        "realTimeFactorWall": seconds / took,
        "peak": peak,
        "notes": notes.len(),
        "ledger": {
            "sampleFrames": r.sample_frames, "initialEnergy": r.initial_energy, "finalEnergy": r.current_energy,
            "sourceWork": r.source_work, "controlWork": r.control_work, "internalLoss": r.internal_loss, "radiation": r.radiation,
            "cumulativeClosureResidual": r.cumulative_closure_residual, "globalClosureResidual": r.global_closure_residual,
            "maximumFrameResidual": r.maximum_frame_residual, "maximumLocalResidual": r.maximum_local_residual,
        },
        "moves": m,
    });
    std::fs::write(out_dir.join(format!("{stem}-receipt.json")), serde_json::to_string_pretty(&receipt).unwrap()).unwrap();
    println!("{piece}: {seconds:.1} s in {took:.2} s wall ({:.2}x real time), peak {:.2} dBFS", seconds / took, 20.0 * peak.log10());
    println!("ledger: source {:.6e} control {:.6e} (slide {:.6e} disposal {:.3e}, lips {:.6e}) loss {:.6e} radiation {:.6e}",
        r.source_work, r.control_work, m.slide_work, m.slide_disposal, m.lip_work, r.internal_loss, r.radiation);
    println!("closure: cumulative {:.3e} global {:.3e} max frame {:.3e} over {} frames", r.cumulative_closure_residual, r.global_closure_residual, r.maximum_frame_residual, r.sample_frames);
    println!("partials: {:?}", placed.iter().map(|p| p.partial).collect::<Vec<_>>());
    println!("wrote {}", wav.display());
}
