//! Play a short chord progression through the aigua/telar string-network voice
//! offline, mapping guitar string+fret to frequency the way the guitarist does,
//! so the polyphonic [`ovillo_dsp::weave::WeaveEngine`] is exercised end to end.
//!
//! ```text
//! cargo run -p ovillo-dsp --example weave_render -- [out.wav]
//! ```
//! Writes a stereo WAV and prints the peak.

use ovillo_dsp::guitar::OPEN_HZ;
use ovillo_dsp::weave::{WeaveEngine, WeavePatch};

const SR: f64 = 48_000.0;

/// Standard guitar: `OPEN_HZ[string] * 2^(fret/12)`.
fn fret_hz(string: usize, fret: i8) -> f64 {
    OPEN_HZ[string] * 2f64.powf(fret as f64 / 12.0)
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "weave_render.wav".into());

    let mut e = WeaveEngine::new(SR);
    e.set_patch(WeavePatch::violin());

    // Em, C, G, D as the guitarist frets them (string, fret); unplayed strings
    // are simply absent. Strummed every 1.2 s, then a tail to let it ring.
    let chords: &[&[(usize, i8)]] = &[
        &[(0, 0), (1, 2), (2, 2), (3, 0), (4, 0), (5, 0)],
        &[(1, 3), (2, 2), (3, 0), (4, 1), (5, 0)],
        &[(0, 3), (1, 2), (2, 0), (3, 0), (4, 0), (5, 3)],
        &[(2, 0), (3, 2), (4, 3), (5, 2)],
    ];
    let per_chord = 1.2_f64;
    let tail = 3.0_f64;
    let n = ((chords.len() as f64 * per_chord + tail) * SR) as usize;

    let mut mono = Vec::with_capacity(n);
    for i in 0..n {
        for (ci, chord) in chords.iter().enumerate() {
            if i == (ci as f64 * per_chord * SR) as usize {
                for &(string, fret) in *chord {
                    e.note_on(&[fret_hz(string, fret)], 0.7, 1.0);
                }
            }
        }
        mono.push(e.frame());
    }

    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SR as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(&out, spec).expect("create WAV");
    for &s in &mono {
        w.write_sample(s).expect("write L");
        w.write_sample(s).expect("write R");
    }
    w.finalize().expect("finalize WAV");

    let peak = mono.iter().map(|v| v.abs()).fold(0.0_f32, f32::max);
    println!("wrote {out}: {n} samples ({:.1} s), peak {peak:.3}", n as f64 / SR);
}
