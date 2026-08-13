use mus_dsp::powered_radiator::{PoweredRadiator, PoweredRadiatorControls};
use std::hint::black_box;
use std::time::Instant;

fn run(label: &str, sample_rate: f64, seconds: f64, controls: PoweredRadiatorControls) {
    let frames = (sample_rate * seconds) as usize;
    let mut body = PoweredRadiator::new(sample_rate).expect("supported sample rate");
    let started = Instant::now();
    let mut checksum = 0.0_f64;
    for index in 0..frames {
        let phase = index as f64 * 2.0 * std::f64::consts::PI * 110.0 / sample_rate;
        let source = (phase.sin() * 0.2 + (phase * 2.003).sin() * 0.07) as f32;
        let frame = body.process_sample(black_box(source), black_box(source * 0.93), controls);
        checksum += frame.left as f64 * 0.37 + frame.right as f64 * 0.63;
    }
    let elapsed = started.elapsed().as_secs_f64();
    let realtime = seconds / elapsed;
    let micros_per_frame = elapsed * 1.0e6 / frames as f64;
    let receipt = body.receipt();
    println!(
        "{label}: {sample_rate:.0} Hz, {frames} frames in {elapsed:.4} s, \
         {realtime:.1}x realtime, {micros_per_frame:.4} us/frame, \
         peak_wet={:.6}, faults={}, checksum={checksum:.9}",
        receipt.peak_wet, receipt.non_finite_resets,
    );
}

fn main() {
    let seconds = std::env::args()
        .nth(1)
        .and_then(|argument| argument.parse::<f64>().ok())
        .unwrap_or(30.0)
        .clamp(1.0, 600.0);
    run(
        "default",
        48_000.0,
        seconds,
        PoweredRadiatorControls::default(),
    );
    run(
        "extreme",
        48_000.0,
        seconds,
        PoweredRadiatorControls {
            drive: 1.5,
            headroom: 0.05,
            ..PoweredRadiatorControls::default()
        },
    );
    run(
        "default-high-rate",
        192_000.0,
        seconds,
        PoweredRadiatorControls::default(),
    );
}
