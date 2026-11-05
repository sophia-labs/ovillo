//! A held note across push x tone x cheeks: one WAV per point, for timbre
//! measurements (`fart-timbre-p{push}-t{tone}-c{cheeks}.wav`, 1.2 s, the
//! reservoir refilled at 1 W so it does not sag, tension commanded at 85 Hz).
//!
//! ```text
//! cargo run --release -p ovillo-tract-world --example fart_timbre -- <out_dir>
//! ```

use ovillo_tract_world::fart::{self, Gut};

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("out dir"));
    std::fs::create_dir_all(&dir).unwrap();
    for push in [0.4, 0.7, 1.0] {
        for tone in [0.2, 0.45, 0.75, 0.95] {
            for cheeks in [0.15, 0.4, 0.8] {
                let mut gut = Gut::new(48_000.0).unwrap();
                gut.world_mut().set_refill(1.0);
                let mut c = gut.world().controls();
                c.articulation = fart::cheeks(cheeks);
                c.tenseness = tone;
                c.pitch_hz = 85.0;
                c.breath = push;
                gut.world_mut().set_controls(c);
                let mut audio = vec![0.0f32; 57_600];
                gut.world_mut().process(&mut audio).unwrap();
                fart::dc_block(&mut audio, 48_000.0);
                let spec = hound::WavSpec { channels: 1, sample_rate: 48_000, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
                let mut w = hound::WavWriter::create(dir.join(format!("fart-timbre-p{push}-t{tone}-c{cheeks}.wav")), spec).unwrap();
                for s in &audio[9_600..] {
                    w.write_sample(*s).unwrap();
                }
                w.finalize().unwrap();
            }
        }
    }
}
