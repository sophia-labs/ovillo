//! Native profile target: the public engine through its ABI, idle at the
//! page's default controls (rest, breath 0.55), light schedule.
use ovillo_tract_world::abi::*;
fn main() {
    let seconds: f64 = std::env::args().nth(1).and_then(|v| v.parse().ok()).unwrap_or(20.0);
    tract_init_decimated(48_000.0, 2);
    tract_set(6, 0.55);
    tract_set(7, 0.15);
    let t = std::time::Instant::now();
    for i in 0..(seconds * 48_000.0 / 128.0) as usize {
        tract_process(128);
        if i % 4 == 0 { tract_snapshot(); }
    }
    println!("{:.2}x real time", seconds / t.elapsed().as_secs_f64());
    tract_snapshot();
    let s = unsafe { std::slice::from_raw_parts(tract_state_ptr(), tract_state_len() as usize) };
    println!("non-finite faults {}, F0 {}, max frame residual {:e}, frames {}", s[35], s[9], s[23], s[22]);
}
