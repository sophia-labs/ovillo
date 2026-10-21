//! Play the trombone World (see `ovillo_dsp::trombone_world`).
//!
//! ```text
//! cargo run --release -p ovillo-dsp --example trombone_world_solo -- <out-dir> [solo|etude|dynamics]
//! ```
//! - `solo`: the slow F-minor solo of `examples/trombone_solo.rs` (same notes,
//!   same timing; its growl has no physical mechanism in this World and is not
//!   played).
//! - `etude`: lip slurs over partials 2..6 in each slide position 1..7.
//! - `dynamics`: Bb3, F3, D4 in first position, each tongued at five dynamics.
//!
//! Writes `trombone-world-<piece>.wav` (stereo float, exactly the World
//! listener's samples), `...-notes.tsv` (what was meant, and where it was
//! played), and `...-receipt.json` (plan and listener digests, the ledger,
//! timing).

use ovillo_dsp::trombone_player::{
    hz_of, perform, place, Art, Note, Placement, Skill, DIM, FALL, RIP, SCOOP, SWELL, VIB,
};
use ovillo_dsp::trombone_world::{TromboneWorld, PLAN_JSON};

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

/// Partials 2..6 slurred up and down in each position; a breath between positions.
fn etude(tw: &TromboneWorld, skill: &Skill) -> (Vec<Note>, Vec<Placement>) {
    let mut notes = Vec::new();
    let mut placed = Vec::new();
    let mut t = 0.4;
    let bb1 = hz_of(34.0);
    for pos in 1..=7 {
        let seq = [2usize, 3, 4, 5, 6, 5, 4, 3, 2];
        for (j, &k) in seq.iter().enumerate() {
            // the tempered note nearest partial k in this position
            let natural = k as f64 * bb1 * 2f64.powf(-((pos - 1) as f64) / 12.0);
            let midi = (69.0 + 12.0 * (natural / 440.0).log2()).round();
            let len = 0.55;
            notes.push(Note { midi, start_s: t, end_s: t + len, vel: 0.5, art: if j == 0 { Art::Tongue } else { Art::Slur }, fx: 0 });
            placed.push(Placement { partial: k, slide: skill.slide_for(tw, k, hz_of(midi)).clamp(0.0, 1.0) });
            t += len;
        }
        t += 0.45;
    }
    (notes, placed)
}

/// Bb3 (4th partial), F3 (3rd), D4 (5th) in first position, each at five dynamics.
fn dynamics(tw: &TromboneWorld, skill: &Skill) -> (Vec<Note>, Vec<Placement>) {
    let mut notes = Vec::new();
    let mut placed = Vec::new();
    let mut t = 0.4;
    for &(midi, k) in &[(58.0, 4usize), (53.0, 3), (62.0, 5)] {
        for step in 0..5 {
            let vel = step as f64 / 4.0;
            notes.push(Note { midi, start_s: t, end_s: t + 1.1, vel, art: Art::Tongue, fx: 0 });
            placed.push(Placement { partial: k, slide: skill.slide_for(tw, k, hz_of(midi)).clamp(0.0, 1.0) });
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
    let skill = Skill::embedded().expect("player skill");
    let mut tw = TromboneWorld::new().expect("trombone world");
    let (notes, placed) = match piece.as_str() {
        "etude" => etude(&tw, &skill),
        "dynamics" => dynamics(&tw, &skill),
        _ => {
            let notes = solo();
            let placed = place(&tw, &skill, &notes);
            (notes, placed)
        }
    };
    let tail_s = 2.5;
    let breaths = perform(&tw, &skill, &notes, &placed, sr, tail_s);

    let mut left = Vec::with_capacity(breaths.len());
    let mut right = Vec::with_capacity(breaths.len());
    let t0 = std::time::Instant::now();
    for b in &breaths {
        let o = tw.tick(*b).expect("the World stepped");
        left.push(o.left);
        right.push(o.right);
    }
    let took = t0.elapsed().as_secs_f64();
    let seconds = breaths.len() as f64 / sr;

    std::fs::create_dir_all(&out_dir).unwrap();
    let stem = format!("trombone-world-{piece}");
    let spec = hound::WavSpec { channels: 2, sample_rate: sr as u32, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let wav = out_dir.join(format!("{stem}.wav"));
    let mut w = hound::WavWriter::create(&wav, spec).unwrap();
    for i in 0..left.len() {
        w.write_sample(left[i] as f32).unwrap();
        w.write_sample(right[i] as f32).unwrap();
    }
    w.finalize().unwrap();

    let mut tsv = String::from("start_s\tend_s\tmidi\tpartial\tslide\tposition\tart\tfx\tvel\n");
    for (x, p) in notes.iter().zip(&placed) {
        tsv += &format!(
            "{:.4}\t{:.4}\t{}\t{}\t{:.4}\t{}\t{:?}\t{}\t{:.2}\n",
            x.start_s, x.end_s, x.midi, p.partial, p.slide, p.position(), x.art, x.fx, x.vel
        );
    }
    std::fs::write(out_dir.join(format!("{stem}-notes.tsv")), tsv).unwrap();

    let ledger = tw.ledger();
    let peak = left.iter().chain(&right).fold(0.0f64, |m, v| m.max(v.abs()));
    let plan_digest = tw.world().plan_digest().to_owned();
    let receipt = serde_json::json!({
        "schema": "ovillo.trombone-world-render.v0",
        "piece": piece,
        "planDigest": plan_digest,
        "planJsonBytes": PLAN_JSON.len(),
        "listenerDigest": tw.listener_digest(),
        "seconds": seconds,
        "sampleRate": sr,
        "renderWallSeconds": took,
        "realTimeFactorWall": seconds / took,
        "peak": peak,
        "notes": notes.len(),
        "ledger": ledger,
    });
    std::fs::write(out_dir.join(format!("{stem}-receipt.json")), serde_json::to_string_pretty(&receipt).unwrap()).unwrap();
    println!("{piece}: {seconds:.1} s rendered in {took:.2} s wall ({:.2}x real time), peak {:.2} dBFS", seconds / took, 20.0 * peak.log10());
    println!(
        "ledger: source {:.6e} control {:.6e} (lip {:.6e}, slide {:.6e}) loss {:.6e} radiation {:.6e}",
        ledger.source_work, ledger.control_work, ledger.lip_control_work, ledger.slide_control_work, ledger.internal_loss, ledger.radiation
    );
    println!(
        "closure: cumulative {:.3e} global {:.3e} max frame {:.3e} max local {:.3e}",
        ledger.cumulative_closure_residual, ledger.global_closure_residual, ledger.maximum_frame_residual, ledger.maximum_local_residual
    );
    println!("partials: {:?}", placed.iter().map(|p| p.partial).collect::<Vec<_>>());
    println!("wrote {}", wav.display());
}
