//! Probe the v1 trombone World: hold mouth pressure and slide, sweep lip
//! tension (log) and write the World's emission.
//!
//! cargo run --release -p ovillo-dsp --example trombone_nd_probe -- <out.wav> <out.tsv> \
//!     [plan.json] [mouth] [slide_m] [lip_from] [lip_to] [seconds] [embouchure]

use ovillo_dsp::trombone_nd::{Breath, TromboneNd, PLAN_JSON};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let plan = a.get(3).filter(|p| *p != "-").map(|p| std::fs::read_to_string(p).unwrap()).unwrap_or_else(|| PLAN_JSON.to_owned());
    let num = |i: usize, d: f64| a.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (mouth, slide, f0, f1, secs, emb) = (num(4, 0.3), num(5, 0.0), num(6, 60.0), num(7, 500.0), num(8, 12.0), num(9, 0.6));
    let sr = 48_000.0;
    let mut tw = TromboneNd::from_json(&plan).unwrap();
    tw.set_slide_now(slide).unwrap();
    eprintln!("modes: {:?}", tw.frequencies_hz().iter().take(10).map(|f| (f * 10.0).round() / 10.0).collect::<Vec<_>>());
    let n = (secs * sr) as usize;
    let mut out = Vec::with_capacity(n);
    let mut tsv = String::from("t\tlip_hz\n");
    let t0 = std::time::Instant::now();
    for i in 0..n {
        let t = i as f64 / sr;
        let lip = f0 * (f1 / f0).powf(t / secs);
        let ramp = (t / 0.02).min(1.0);
        out.push(tw.tick(Breath { mouth: mouth * ramp, lip_hz: lip, embouchure: emb, slide_m: slide }).unwrap());
        if i % 480 == 0 {
            tsv += &format!("{t:.3}\t{lip:.3}\n");
        }
    }
    let took = t0.elapsed().as_secs_f64();
    let r = tw.world().receipt();
    eprintln!(
        "rtf {:.2} src {:.4e} ctl {:.4e} loss {:.4e} rad {:.4e} cum {:.3e} maxf {:.3e} peak {:.3}",
        secs / took, r.source_work, r.control_work, r.internal_loss, r.radiation, r.cumulative_closure_residual,
        r.maximum_frame_residual, out.iter().fold(0.0f64, |m, v| m.max(v.abs()))
    );
    let spec = hound::WavSpec { channels: 1, sample_rate: sr as u32, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut w = hound::WavWriter::create(&a[1], spec).unwrap();
    for x in &out {
        w.write_sample(*x as f32).unwrap();
    }
    w.finalize().unwrap();
    std::fs::write(&a[2], tsv).unwrap();
}
