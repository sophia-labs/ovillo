use ovillo_game_sound::f16::F16;
use ovillo_game_sound::GameWorld;
fn main() {
    let a: Vec<f64> = std::env::args().skip(1).take(4).map(|s| s.parse().unwrap()).collect();
    let (thr, ab, air, asp) = (a[0], a[1], a[2], a[3]);
    let mut e = F16::new(48000.0, 5);
    e.set("thrust", thr); e.set("afterburner", ab); e.set("airspeed", air); e.set("aspect", asp);
    let mut buf = vec![0f32; 48000];
    let t0 = std::time::Instant::now();
    for s in 0..8 { e.render(&mut buf);
        if s >= 6 { let rms = (buf.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / buf.len() as f64).sqrt() * 400.0;
        let pk = buf.iter().fold(0f32, |a, b| a.max(b.abs())) as f64 * 400.0;
        println!("sec {s}: rms {:.2} Pa ({:.0} dB) peak {:.1}  n1 {:.3} bpf {:.0} Uj {:.0}", rms, 20.0*(rms/2e-5).log10(), pk, e.n1(), e.fan_bpf(), e.jet_velocity()); } }
    if let Some(path) = std::env::args().nth(5) {
        let spec = hound::WavSpec{channels:1,sample_rate:48000,bits_per_sample:32,sample_format:hound::SampleFormat::Float};
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for s in &buf { w.write_sample(*s).unwrap(); }
        w.finalize().unwrap();
    }
    println!("wall {:.2}", t0.elapsed().as_secs_f64());
    let r = e.receipt(); let l = &r["ledger"];
    for (k, v) in l["lines"].as_object().unwrap() { println!("  {k}: {v}"); }
    println!("frame {} global {} rel {} flamepk {}", l["maximumFrameResidual"], l["globalResidual"], l["relativeGlobalResidual"], r["peakFlamePlanePressurePa"]);
}
