//! What the move interval costs. The /aiua/ glide rendered with a move every
//! sample (reference R), the same with breath perturbed by 1e-9 (R', the
//! chaos floor of a self-oscillating fold), and with a move every k samples.
//! Metrics: waveform difference; long-term spectrum difference (1/3-octave
//! bands 100 Hz-12 kHz, mean |dB|); F1/F2 tracks (LPC, 50 ms frames every
//! 25 ms, mean |dHz|); energy above 6 kHz relative to R's.
//! `cargo run --release -p ovillo-tract-world --example zipper`
use ovillo_tract_world::bench::Workload;
use ovillo_tract_world::measure;

const SR: f64 = 48_000.0;

fn render(interval: usize, seconds: f64, breath: f64) -> Vec<f64> {
    render_with(interval, seconds, breath, 0.0)
}

fn render_with(interval: usize, seconds: f64, breath: f64, min_move: f64) -> Vec<f64> {
    if std::env::args().any(|a| a == "--public") {
        return render_public(interval, seconds, min_move);
    }
    let mut w = Workload::new(44, 31, interval, true, SR, min_move);
    w.breath = breath;
    let mut out = vec![0.0f32; (seconds * SR) as usize];
    w.render(&mut out);
    out.iter().map(|&v| f64::from(v)).collect()
}

/// The public engine running its own "mama" gesture (lips, velum, tongue).
fn render_public(interval: usize, seconds: f64, min_move: f64) -> Vec<f64> {
    use mus_ariadne::{ConnectionPolicy, DisposalPolicy};
    use ovillo_tract_world::gestures::Gesture;
    use ovillo_tract_world::world::{self as tw, TractWorldNd};
    let mut w = TractWorldNd::with(SR, ovillo_tract_world::TractNdAnatomy::public_v1(), ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate }, tw::ART_REST).unwrap();
    w.update_interval = interval;
    w.min_move = min_move;
    w.set_gesture(Some(Gesture::mama()));
    let mut out = vec![0.0f32; (seconds * SR) as usize];
    w.process(&mut out).unwrap();
    out.iter().map(|&v| f64::from(v)).collect()
}

fn ltas(x: &[f64]) -> Vec<f64> {
    let n = 8192;
    let mut acc = vec![0.0; n / 2];
    let mut start = 0;
    while start + n <= x.len() {
        let frame: Vec<f64> = (0..n).map(|i| x[start + i] * (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos())).collect();
        for (a, d) in acc.iter_mut().zip(measure::magnitude_db(&frame, n)) {
            *a += 10f64.powf(d / 10.0);
        }
        start += n / 2;
    }
    acc
}

fn bands(power: &[f64]) -> Vec<f64> {
    let df = SR / 8192.0;
    let mut out = Vec::new();
    let mut lo = 100.0;
    while lo < 12_000.0 {
        let hi = lo * 2f64.powf(1.0 / 3.0);
        let e: f64 = power[(lo / df) as usize..(hi / df) as usize].iter().sum();
        out.push(10.0 * e.max(1e-300).log10());
        lo = hi;
    }
    out
}

fn above(power: &[f64], hz: f64) -> f64 {
    power[(hz / (SR / 8192.0)) as usize..].iter().sum()
}

fn tracks(x: &[f64]) -> Vec<[f64; 2]> {
    let mut out = Vec::new();
    let mut start = 0;
    while start + 2_400 <= x.len() {
        let f = measure::lpc_formants(&x[start..start + 2_400], SR);
        out.push([f.first().map_or(f64::NAN, |q| q.hz), f.get(1).map_or(f64::NAN, |q| q.hz)]);
        start += 1_200;
    }
    out
}

fn compare(name: &str, x: &[f64], r: &[f64], r_ltas: &[f64], r_tracks: &[[f64; 2]]) {
    let signal = r.iter().map(|v| v * v).sum::<f64>();
    let diff = x.iter().zip(r).map(|(a, b)| (a - b) * (a - b)).sum::<f64>();
    let l = ltas(x);
    let (bx, br) = (bands(&l), bands(r_ltas));
    let band_err = bx.iter().zip(&br).map(|(a, b)| (a - b).abs()).sum::<f64>() / bx.len() as f64;
    let worst_band = bx.iter().zip(&br).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
    let t = tracks(x);
    let (mut d1, mut d2, mut n) = (0.0, 0.0, 0.0);
    for (a, b) in t.iter().zip(r_tracks) {
        if a[0].is_finite() && b[0].is_finite() && a[1].is_finite() && b[1].is_finite() {
            d1 += (a[0] - b[0]).abs();
            d2 += (a[1] - b[1]).abs();
            n += 1.0;
        }
    }
    println!(
        "{name:<26} waveform diff {:>6.1} dB | spectrum mean |dB| {:.2}, worst band {:.2} dB | F1/F2 track mean |d| {:.1}/{:.1} Hz | >6 kHz energy {:+.2} dB vs R",
        10.0 * (diff / signal).log10(),
        band_err,
        worst_band,
        d1 / n,
        d2 / n,
        10.0 * (above(&l, 6_000.0) / above(r_ltas, 6_000.0)).log10()
    );
}

fn main() {
    let seconds = 2.8;
    let r = render(1, seconds, 0.6);
    let (r_ltas, r_tracks) = (ltas(&r), tracks(&r));
    if std::env::args().any(|a| a == "--public") {
        for (interval, min_move) in [(32usize, 1.0e-3), (128, 4.0e-3), (256, 8.0e-3)] {
            let x = render_with(interval, seconds, 0.6, min_move);
            compare(&format!("public every {interval}, min {min_move}"), &x, &r, &r_ltas, &r_tracks);
        }
        return;
    }
    if std::env::args().any(|a| a == "--threshold-only") {
        let x = render_with(32, seconds, 0.6, 0.001);
        compare("every 32, min_move 0.001", &x, &r, &r_ltas, &r_tracks);
        return;
    }
    let perturbed = render(1, seconds, 0.6 * (1.0 + 1e-9));
    compare("R' (breath x(1+1e-9))", &perturbed, &r, &r_ltas, &r_tracks);
    for interval in [8usize, 32, 64, 128] {
        let x = render(interval, seconds, 0.6);
        compare(&format!("move every {interval} samples"), &x, &r, &r_ltas, &r_tracks);
    }
    let x = render_with(32, seconds, 0.6, 0.001);
    compare("every 32, min_move 0.001", &x, &r, &r_ltas, &r_tracks);
}
