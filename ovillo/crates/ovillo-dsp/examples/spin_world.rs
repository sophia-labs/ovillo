//! Run the Ariadne engine on silence for a while (for a profiler to sample).
use ovillo_dsp::ariadne::{AriadneControls, AriadneEngine};
fn main() {
    let mut e = AriadneEngine::new(48_000.0).unwrap();
    let c = AriadneControls::default();
    let mut s = 0.0f32;
    for _ in 0..(48_000 * 12) {
        s += e.process_sample(0.0, 0.0, c).left;
    }
    println!("{s}");
}
