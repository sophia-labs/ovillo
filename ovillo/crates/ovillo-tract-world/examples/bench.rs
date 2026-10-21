//! Real-time factor of the N-D tract World, natively, one configuration per
//! process so `/usr/bin/time` can give its CPU time on a loaded machine:
//! `/usr/bin/time -l target/release/examples/bench <oral> <nasal> <interval> <moving 0|1> [seconds]`
use ovillo_tract_world::bench::Workload;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |i: usize, d: usize| args.get(i).and_then(|v| v.parse().ok()).unwrap_or(d);
    let (oral, nasal, interval, moving) = (arg(1, 44), arg(2, 31), arg(3, 32), arg(4, 1) == 1);
    let seconds = args.get(5).and_then(|v| v.parse().ok()).unwrap_or(10.0);
    let min_move: f64 = args.get(6).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let sr = 48_000.0;
    let mut w = Workload::new(oral, nasal, interval, moving, sr, min_move);
    let mut block = vec![0.0f32; 128];
    let blocks = (seconds * sr / 128.0) as usize;
    let start = Instant::now();
    let mut acc = 0.0;
    for _ in 0..blocks {
        acc += w.render(&mut block);
    }
    let elapsed = start.elapsed().as_secs_f64();
    let r = w.world.world().receipt();
    println!(
        "oral {oral} nasal {nasal} interval {interval} min_move {min_move} {}: audio {seconds} s, wall {elapsed:.2} s (wall RTF {:.2}x), moves {}, max frame residual {:.2e} J, cumulative {:.2e} J (energy {acc:.3e})",
        if moving { "moving /aiua/" } else { "held /a/" },
        seconds / elapsed,
        w.world.stats().moves,
        r.maximum_frame_residual,
        r.cumulative_closure_residual,
    );
}
