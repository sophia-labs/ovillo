//! Native real-time factor of the same render path the worklet drives
//! (128-frame quanta at 48 kHz), for comparison with `site/bench/rtf.mjs`.
//!
//! ```text
//! cargo run --release -p ovillo-web --example rtf -- [seconds] [pattern-index]
//! ```

use ovillo_web::Web;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let secs: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(20.0);
    let pattern: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut w = Web::new(48_000.0, 12_345).expect("rig");
    w.set_pattern(ovillo_dsp::player::Pattern::ALL[pattern]);
    let n = (secs * 48_000.0 / 128.0) as usize;
    let t0 = Instant::now();
    let mut worst = 0.0f64;
    for i in 0..n {
        let q = Instant::now();
        w.render(128);
        if i > 50 {
            worst = worst.max(q.elapsed().as_secs_f64() * 1e3);
        }
    }
    let wall = t0.elapsed().as_secs_f64();
    println!("native: audio {secs}s in {wall:.2}s -> {:.2}x real time; worst quantum {worst:.2} ms of 2.67 ms", secs / wall);
}
