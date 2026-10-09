//! Time the Ariadne engine alone on silence, a sine, and the strummed guitar.
use ovillo_dsp::ariadne::{AriadneControls, AriadneEngine};
use ovillo_dsp::guitar::Guitar;
use ovillo_dsp::player::Player;
use std::time::Instant;

fn main() {
    let sr = 48_000.0;
    let n = 48_000;
    let c = AriadneControls::default();
    let run = |label: &str, input: &dyn Fn(usize) -> f32| {
        let mut e = AriadneEngine::new(sr).unwrap();
        let t = Instant::now();
        let mut s = 0.0f32;
        for i in 0..n {
            let x = input(i);
            s += e.process_sample(x, x, c).left;
        }
        let dt = t.elapsed().as_secs_f64();
        println!("{label:28} {:7.2} us/sample  {:5.2}x real time  (sink {s:.3})", dt / n as f64 * 1e6, (n as f64 / sr) / dt);
    };
    run("silence", &|_| 0.0);
    run("sine 196 Hz -40 dBFS", &|i| 0.01 * (std::f32::consts::TAU * 196.0 * i as f32 / 48_000.0).sin());
    run("sine 196 Hz -20 dBFS", &|i| 0.1 * (std::f32::consts::TAU * 196.0 * i as f32 / 48_000.0).sin());
    run("sine 196 Hz -6 dBFS", &|i| 0.5 * (std::f32::consts::TAU * 196.0 * i as f32 / 48_000.0).sin());
    let mut g = Guitar::new(sr);
    let mut p = Player::new(sr);
    let strum: Vec<f32> = (0..n).map(|_| { p.tick(&mut g); g.tick() }).collect();
    let peak = strum.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    println!("strum peak {:.1} dBFS", 20.0 * peak.log10());
    run("strum", &|i| strum[i]);
}
