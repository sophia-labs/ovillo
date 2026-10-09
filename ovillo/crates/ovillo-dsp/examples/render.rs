//! Render the rig offline and time it against real time.
//!
//! ```text
//! cargo run -p ovillo-dsp --example render -- <seconds> <pattern|score.json> <out-dir> [pickup] [patch.json]
//! ```
//! `<pattern>` is any composition (`wander travis blues ambient melody strum
//! arpeggio riff notes`); a path ending in `.json` is played as a score.
//! Writes `<name>-dry.wav` (mono DI) and `<name>-wet.wav` (stereo Ariadne),
//! and prints levels, the World's closure residual, and the real-time factor
//! of the guitar alone and of the whole rig.

use ovillo_dsp::ariadne::AriadneControls;
use ovillo_dsp::guitar::{Guitar, GuitarSettings, Pickup};
use ovillo_dsp::player::{Pattern, Player};
use ovillo_dsp::rig::{Command, Rig};
use ovillo_dsp::score;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seconds: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(8.0);
    let which = args.get(2).map(String::as_str).unwrap_or("wander");
    let score_text = which.ends_with(".json").then(|| std::fs::read_to_string(which).expect("score file"));
    let pattern = if score_text.is_some() {
        Pattern::Score
    } else {
        Pattern::from_id(which).unwrap_or_else(|| panic!("unknown pattern {which}"))
    };
    let name = if score_text.is_some() {
        std::path::Path::new(which).file_stem().map_or("score".into(), |s| s.to_string_lossy().into_owned())
    } else {
        pattern.id().to_string()
    };
    let out_dir = std::path::PathBuf::from(args.get(3).cloned().unwrap_or_else(|| ".".into()));
    let pickup = match args.get(4).map(String::as_str).unwrap_or("bridge") {
        "neck" => Pickup::Neck,
        "middle" => Pickup::Middle,
        "humbucker" => Pickup::Humbucker,
        _ => Pickup::Bridge,
    };
    let patch: Option<AriadneControls> =
        args.get(5).map(|p| serde_json::from_str(&std::fs::read_to_string(p).expect("patch")).expect("patch json"));
    // OVILLO_RATE=96000 renders at another rate.
    let sr: f64 = std::env::var("OVILLO_RATE").ok().and_then(|r| r.parse().ok()).unwrap_or(48_000.0);
    let n = (seconds * sr) as usize;
    let settings = GuitarSettings { pickup, ..GuitarSettings::default() };

    // The guitar alone.
    let mut g = Guitar::new(sr);
    g.set(settings);
    let mut p = Player::new(sr);
    p.set_pattern(pattern);
    if let Some(t) = &score_text {
        let _ = p.load_score(Box::new(score::parse_str(t, 92.0).expect("score")));
    }
    let t0 = Instant::now();
    let mut sink = 0.0f32;
    for _ in 0..n {
        p.tick(&mut g);
        sink += g.tick();
    }
    let guitar_s = t0.elapsed().as_secs_f64();

    // The whole rig.
    let (mut rig, mut ends) = Rig::new(sr).expect("rig");
    ends.commands.push(Command::Guitar(settings)).unwrap();
    ends.commands.push(Command::Pattern(pattern)).unwrap();
    if let Some(t) = &score_text {
        let sc = score::parse_str(t, 92.0).expect("score");
        println!("score: {}", sc.summary());
        ends.commands.push(Command::Score(Box::new(sc))).unwrap();
    }
    if let Some(c) = patch {
        ends.commands.push(Command::Ariadne(c)).unwrap();
    }
    rig.poll();
    let mut dry = Vec::with_capacity(n);
    let mut wet = Vec::with_capacity(2 * n);
    let t1 = Instant::now();
    let mut last = None;
    for i in 0..n {
        let (d, l, r) = rig.frame();
        dry.push(d);
        wet.push(l);
        wet.push(r);
        if i % 4096 == 0 {
            while ends.scope.pop().is_ok() {}
            while let Ok(t) = ends.telemetry.pop() {
                last = Some(t);
            }
        }
    }
    let rig_s = t1.elapsed().as_secs_f64();
    while let Ok(t) = ends.telemetry.pop() {
        last = Some(t);
    }
    let t = last.expect("telemetry");

    let peak = |x: &[f32]| x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let rms = |x: &[f32]| (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt();
    let db = |v: f64| 20.0 * v.max(1e-12).log10();
    let (l, r): (Vec<f32>, Vec<f32>) = wet.chunks(2).map(|c| (c[0], c[1])).unzip();
    let corr = {
        let (sl, sr2, slr) = l.iter().zip(&r).fold((0.0, 0.0, 0.0), |(a, b, c), (x, y)| {
            (a + (*x as f64).powi(2), b + (*y as f64).powi(2), c + *x as f64 * *y as f64)
        });
        slr / (sl * sr2).sqrt().max(1e-30)
    };
    println!("pattern {:?}  pickup {:?}  {seconds} s at {sr} Hz  (sink {sink:.1})", pattern, pickup);
    println!("guitar alone: {:.3} s  -> {:.1}x real time", guitar_s, seconds / guitar_s);
    println!("whole rig:    {:.3} s  -> {:.2}x real time", rig_s, seconds / rig_s);
    println!("dry  peak {:6.1} dBFS  rms {:6.1} dBFS", db(peak(&dry) as f64), db(rms(&dry)));
    println!("wet  peak {:6.1} dBFS  rms L {:6.1} R {:6.1} dBFS  L/R correlation {:+.3}", db(peak(&wet) as f64), db(rms(&l)), db(rms(&r)), corr);
    println!(
        "world errors {}  safety-limited frames {}  closure residual {:.2e}  material {:.3e}  reservoir {:.3e}  space {:.3e}",
        t.world_errors, t.safety_limited_frames, t.world.global_closure_residual, t.world.material_energy, t.world.reservoir_energy, t.world.space_energy
    );

    std::fs::create_dir_all(&out_dir).unwrap();
    let spec = |ch| hound::WavSpec { channels: ch, sample_rate: sr as u32, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut w = hound::WavWriter::create(out_dir.join(format!("{name}-dry.wav")), spec(1)).unwrap();
    for x in &dry {
        w.write_sample(*x).unwrap();
    }
    w.finalize().unwrap();
    let mut w = hound::WavWriter::create(out_dir.join(format!("{name}-wet.wav")), spec(2)).unwrap();
    for x in &wet {
        w.write_sample(*x).unwrap();
    }
    w.finalize().unwrap();
    println!("wrote {}/{name}-{{dry,wet}}.wav", out_dir.display());
}
