//! Cost per sample of the tract World, natively.
//! `cargo run --release -p ovillo-tract --example bench_tract`
use ovillo_tract::{presets, Gesture, TractControls, TractWorld};
use std::time::Instant;

fn main() {
    let sr = 48_000.0;
    let mut w = TractWorld::new(sr);
    let mut c = TractControls::default();
    c.articulation = presets::ART_A;
    c.breath = 0.6;
    w.set_controls(c);
    w.set_gesture(Some(Gesture::mama()));
    let mut block = [0.0f32; 128];
    // warm
    for _ in 0..375 { w.process(&mut block); }
    let seconds = 20.0;
    let blocks = (seconds * sr / 128.0) as usize;
    let t = Instant::now();
    let mut peak = 0.0f32;
    for _ in 0..blocks {
        w.process(&mut block);
        for s in block { peak = peak.max(s.abs()); }
    }
    let el = t.elapsed().as_secs_f64();
    let ns = el * 1e9 / (blocks as f64 * 128.0);
    println!("sections oral {} nasal {}; {:.1} ns/sample; real-time factor {:.1}x; peak {:.3}", w.oral_sections(), w.nasal_sections(), ns, seconds / el, peak);
    let r = w.readings();
    println!("F0 {:.1} Hz, lung {:.2}, max frame residual {:.3e} J, global {:.3e} J, flows {:?}", r.measured_f0_hz, r.lung_fraction, r.max_frame_residual_j, r.global_residual_j, r.flows);
}
