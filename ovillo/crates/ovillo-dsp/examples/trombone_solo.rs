//! A slow, sultry trombone solo in F minor, played by an Ariadne World whose
//! configuration path is a slide (see `ovillo_dsp::trombone`).
//!
//! ```text
//! cargo run --release -p ovillo-dsp --example trombone_solo -- <out-dir> [bell-mix] [controls.json] [aperture.json]
//! ```
//! Writes `trombone-solo.wav` (stereo), `trombone-solo-notes.tsv` (what was
//! meant: onset, end, midi) for checking pitch, and prints levels.

use ovillo_dsp::trombone::{choose_partial, Blow, Trombone};

const BPM: f64 = 66.0;

#[derive(Clone, Copy, PartialEq)]
enum Art {
    /// Tongued ("doo").
    Tongue,
    /// Lips move to the next partial; no new attack.
    Slur,
    /// The slide carries the pitch from the previous note.
    Smear,
}

const SCOOP: u8 = 1;
const FALL: u8 = 2;
const VIB: u8 = 4;
const GROWL: u8 = 8;
const RIP: u8 = 16;
const SWELL: u8 = 32;
const DIM: u8 = 64;

struct Note {
    midi: f64,
    beat: f64,
    len: f64,
    vel: f64,
    art: Art,
    fx: u8,
}

const fn n(midi: f64, beat: f64, len: f64, vel: f64, art: Art, fx: u8) -> Note {
    Note { midi, beat, len, vel, art, fx }
}

use Art::{Slur, Smear, Tongue};

// F3 53, G3 55, Ab3 56, Bb3 58, C4 60, Db4 61, Eb4 63, F4 65, G4 67, Ab4 68, Bb4 70, C5 72, F2 41.
const SCORE: &[Note] = &[
    // Phrase A: up to F, a blue slide off the A-flat, down to C.
    n(60.0, 0.5, 0.5, 0.40, Tongue, SCOOP),
    n(63.0, 1.0, 0.5, 0.45, Slur, 0),
    n(65.0, 1.5, 2.0, 0.55, Slur, VIB | SWELL),
    n(68.0, 3.5, 0.5, 0.50, Tongue, 0),
    n(67.0, 4.0, 0.5, 0.48, Smear, 0),
    n(65.0, 4.5, 0.5, 0.45, Slur, 0),
    n(63.0, 5.0, 0.5, 0.42, Tongue, 0),
    n(60.0, 5.5, 2.0, 0.46, Smear, VIB | DIM),
    // Phrase B: the answer, lower, ending in a fall.
    n(56.0, 8.5, 0.5, 0.40, Tongue, SCOOP),
    n(60.0, 9.0, 0.5, 0.45, Slur, 0),
    n(63.0, 9.5, 0.75, 0.50, Slur, 0),
    n(61.0, 10.25, 0.25, 0.45, Tongue, 0),
    n(60.0, 10.5, 1.0, 0.50, Slur, VIB),
    n(58.0, 11.5, 0.5, 0.45, Tongue, 0),
    n(56.0, 12.0, 0.5, 0.42, Slur, 0),
    n(53.0, 12.5, 2.25, 0.50, Smear, VIB | FALL),
    // Phrase C: climb, rip into a growled high C, tumble down.
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
    // Phrase D: settle, smear down, and sink to a low F.
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

/// Beats to seconds, with swung off-beat eighths and a little rubato.
fn time_of(beat: f64, seed: u64) -> f64 {
    let whole = beat.floor();
    let frac = beat - whole;
    let swung = if (frac - 0.5).abs() < 1e-9 { 0.6 } else { frac };
    let jitter = ((seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40) as f64 / (1u64 << 24) as f64 - 0.5) * 0.024;
    (whole + swung) * 60.0 / BPM + jitter
}

fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn hz_of(midi: f64) -> f64 {
    440.0 * 2f64.powf((midi - 69.0) / 12.0)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_dir = std::path::PathBuf::from(args.get(1).cloned().unwrap_or_else(|| ".".into()));
    let bell_mix: f64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.2);
    let sr = 48_000.0;
    let tail_s = 4.0;

    // Onsets, ends, partials.
    let starts: Vec<f64> = SCORE.iter().enumerate().map(|(i, x)| time_of(x.beat, i as u64 + 1)).collect();
    let ends: Vec<f64> = SCORE.iter().enumerate().map(|(i, x)| time_of(x.beat + x.len, i as u64 + 1)).collect();
    let mut partials = Vec::new();
    for (i, x) in SCORE.iter().enumerate() {
        let fall = if x.fx & FALL != 0 { 3.5 } else { 0.0 };
        let prefer = if x.art == Smear { partials.last().copied() } else { None };
        partials.push(choose_partial(hz_of(x.midi), 1.2 + fall, 0.4, prefer));
        let _ = i;
    }
    let total_s = ends.last().unwrap() + tail_s;
    let len = (total_s * sr) as usize;

    // The performance, sample by sample.
    let mut blows = vec![Blow::default(); len];
    let mut last_hz = hz_of(SCORE[0].midi);
    let mut last_slide = last_hz / partials[0] as f64;
    for (i, x) in SCORE.iter().enumerate() {
        let s = (starts[i] * sr) as usize;
        let owned_until = if i + 1 < SCORE.len() { (starts[i + 1] * sr) as usize } else { len };
        let legato_next = i + 1 < SCORE.len() && SCORE[i + 1].art != Tongue && (starts[i + 1] - ends[i]).abs() < 0.02;
        let sounding = if legato_next { owned_until as f64 / sr - starts[i] } else { (ends[i] - starts[i]) * 0.92 };
        let release_s = if x.fx & FALL != 0 { 0.16 } else { 0.06 };
        let k = partials[i] as f64;
        let prev_midi = if i > 0 { SCORE[i - 1].midi } else { x.midi };
        let vib_rate = 5.0 + 0.4 * ((i * 37 % 11) as f64 / 11.0);
        let fall_s = if x.fx & FALL != 0 { (0.4 * sounding).min(0.5) } else { 0.0 };
        for t in s..owned_until.min(len) {
            let tau = t as f64 / sr - starts[i];
            // Pitch, in semitones relative to the note.
            let mut semis = 0.0;
            if x.art == Smear {
                semis += (prev_midi - x.midi) * (1.0 - smoothstep(tau / 0.17));
            }
            if x.fx & SCOOP != 0 {
                semis -= 1.0 * (1.0 - smoothstep(tau / 0.12));
            }
            let rip_s = 0.15;
            let ripping = x.fx & RIP != 0 && tau < rip_s;
            if x.fx & RIP != 0 {
                semis -= 9.0 * (1.0 - (tau / rip_s).clamp(0.0, 1.0)).powi(2);
            }
            if x.fx & VIB != 0 {
                let onset = 0.3 * sounding.min(1.2);
                let depth = 0.24 * smoothstep((tau - onset) / 0.5);
                semis += depth * (std::f64::consts::TAU * vib_rate * tau).sin();
            }
            if x.fx & FALL != 0 {
                let into = (tau - (sounding - fall_s)) / (fall_s + release_s);
                if into > 0.0 {
                    semis -= 3.5 * into.clamp(0.0, 1.0).powi(2);
                }
            }
            let hz = hz_of(x.midi + semis);

            // Breath.
            let attack = match x.art {
                Tongue => 1.0 - (-tau / 0.022).exp(),
                Slur | Smear => 1.0,
            };
            let mut shape = 1.0;
            if x.fx & SWELL != 0 {
                shape *= 0.7 + 0.45 * smoothstep(tau / sounding);
            }
            if x.fx & DIM != 0 {
                shape *= 1.0 - 0.6 * smoothstep(tau / sounding);
            }
            let release = if tau > sounding { (-(tau - sounding) / release_s * 2.3).exp() } else { 1.0 };
            let level = (x.vel * attack * shape * release).min(1.0);
            let tongue_burst = if x.art == Tongue { 0.35 * (-tau / 0.018).exp() } else { 0.0 };
            let growl = if x.fx & GROWL != 0 { 0.7 * (1.0 - smoothstep(tau / 0.9)) } else { 0.0 };

            let slide_hz = if ripping { hz_of(x.midi) / k } else { hz / k };
            last_hz = hz;
            last_slide = slide_hz;
            blows[t] = Blow {
                hz,
                slide_hz,
                level,
                brightness: (0.15 + 0.95 * level.powf(1.3)).min(1.0),
                noise: 0.05 + tongue_burst,
                growl,
            };
        }
    }
    // Before the first note: the slide waits where the first note wants it.
    let first = (starts[0] * sr) as usize;
    for b in blows.iter_mut().take(first) {
        *b = Blow { hz: hz_of(SCORE[0].midi), slide_hz: hz_of(SCORE[0].midi) / partials[0] as f64, ..Blow::default() };
    }
    let _ = (last_hz, last_slide);

    // Play it.
    let aperture: Option<serde_json::Value> =
        args.get(4).map(|p| serde_json::from_str(&std::fs::read_to_string(p).expect("aperture")).expect("json"));
    let mut tb = Trombone::with_aperture(sr, |a| {
        if let Some(v) = &aperture {
            let mut c = serde_json::to_value(*a).unwrap();
            for (k, x) in v.as_object().expect("object") {
                c[k] = x.clone();
            }
            *a = serde_json::from_value(c).expect("aperture");
        }
    })
    .expect("trombone world");
    if let Some(p) = args.get(3) {
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(p).expect("controls")).expect("json");
        let mut c = serde_json::to_value(tb.controls).unwrap();
        for (k, x) in v.as_object().expect("object") {
            c[k] = x.clone();
        }
        tb.controls = serde_json::from_value(c).expect("controls");
    }
    let mut l = Vec::with_capacity(len);
    let mut r = Vec::with_capacity(len);
    let mut bell = Vec::with_capacity(len);
    let t0 = std::time::Instant::now();
    for b in &blows {
        let f = tb.tick(*b);
        l.push(f.left as f64);
        r.push(f.right as f64);
        bell.push(f.bell as f64);
    }
    let took = t0.elapsed().as_secs_f64();

    let rms = |v: &[f64]| (v.iter().map(|x| x * x).sum::<f64>() / v.len() as f64).sqrt();
    let body_rms = rms(&l).max(1e-12);
    let bell_rms = rms(&bell).max(1e-12);
    let bell_gain = bell_mix * body_rms / bell_rms;
    let mut mix: Vec<[f64; 2]> = (0..len).map(|i| [l[i] + bell_gain * bell[i], r[i] + bell_gain * bell[i]]).collect();
    let peak = mix.iter().flat_map(|x| x.iter()).fold(0.0f64, |m, x| m.max(x.abs()));
    let gain = 10f64.powf(-3.0 / 20.0) / peak.max(1e-12);
    for x in &mut mix {
        x[0] *= gain;
        x[1] *= gain;
    }

    std::fs::create_dir_all(&out_dir).unwrap();
    let spec = hound::WavSpec { channels: 2, sample_rate: sr as u32, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let path = out_dir.join("trombone-solo.wav");
    let mut w = hound::WavWriter::create(&path, spec).unwrap();
    for x in &mix {
        w.write_sample(x[0] as f32).unwrap();
        w.write_sample(x[1] as f32).unwrap();
    }
    w.finalize().unwrap();
    let mut notes = String::from("start_s\tend_s\tmidi\tpartial\n");
    for (i, x) in SCORE.iter().enumerate() {
        notes += &format!("{:.4}\t{:.4}\t{}\t{}\n", starts[i], ends[i], x.midi, partials[i]);
    }
    std::fs::write(out_dir.join("trombone-solo-notes.tsv"), notes).unwrap();

    println!("{:.1} s rendered in {:.2} s ({:.1}x real time)", total_s, took, total_s / took);
    println!("body rms {:.1} dBFS raw, bell {:.1} dB under it (mix {bell_mix}), normalised to -3 dBFS peak (gain {:+.1} dB)",
        20.0 * body_rms.log10(), 20.0 * (body_rms / (bell_gain * bell_rms)).log10(), 20.0 * gain.log10());
    println!("world errors {}  safety-limited frames {}", tb.world_errors(), tb.safety_limited_frames());
    println!("partials: {:?}", partials);
    println!("wrote {}", path.display());
}
