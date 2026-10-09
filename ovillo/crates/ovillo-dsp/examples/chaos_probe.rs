//! Is the loud-sine regime chaotic? Perturb the input by 1e-12 and watch the outputs part.
use ovillo_dsp::ariadne::{AriadneControls, AriadneEngine};
fn main() {
    let c = AriadneControls::default();
    for amp in [0.1f32, 0.5] {
        let mut a = AriadneEngine::new(48_000.0).unwrap();
        let mut b = AriadneEngine::new(48_000.0).unwrap();
        let (mut sa, mut sb) = (0.0f64, 0.0f64);
        print!("amp {amp}: ");
        for i in 0..48_000 {
            let x = amp * (std::f32::consts::TAU * 196.0 * i as f32 / 48_000.0).sin();
            let fa = a.process_sample(x, x, c).left as f64;
            let fb = b.process_sample(x * (1.0 + 1e-7), x * (1.0 + 1e-7), c).left as f64;
            sa += fa;
            sb += fb;
            if [100, 1000, 4800, 12000, 24000, 47999].contains(&i) {
                print!("t={:.3}s |diff|={:.1e}  ", i as f64 / 48_000.0, (fa - fb).abs());
            }
        }
        println!("\n   sums {sa:.4} vs {sb:.4}");
    }
}
