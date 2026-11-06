// Renders the audition WAVs and receipts: audition <outdir> [name ...]
use ovillo_game_sound::{create, GameWorld};
use std::time::Instant;

const FS: f64 = 48000.0;

struct Rec {
    name: String,
    w: Box<dyn GameWorld>,
    out: Vec<f32>,
    wall: f64,
    ctrl: Vec<(String, f64)>,
}
impl Rec {
    fn new(name: &str, kind: &str, seed: u64) -> Self {
        Rec { name: name.into(), w: create(kind, FS, seed).unwrap(), out: vec![], wall: 0.0, ctrl: vec![] }
    }
    /// advance `secs`, calling `f(t)` at 60 Hz to set controls
    fn run(&mut self, secs: f64, mut f: impl FnMut(&mut dyn GameWorld, f64)) {
        let t0 = self.out.len() as f64 / FS;
        let n60 = (secs * 60.0).round() as usize;
        let blk = (FS / 60.0).round() as usize;
        let t_wall = Instant::now();
        for i in 0..n60 {
            let t = t0 + i as f64 / 60.0;
            f(self.w.as_mut(), t);
            let mut b = vec![0f32; blk];
            self.w.render(&mut b);
            self.out.extend_from_slice(&b);
        }
        self.wall += t_wall.elapsed().as_secs_f64();
        if std::env::var("AUD_DEBUG").is_ok() { println!("  t={:.1} maxframe {:.2e}", self.out.len() as f64 / FS, self.w.receipt()["ledger"]["maximumFrameResidual"].as_f64().unwrap()); }
    }
    fn finish(self, dir: &str, extra: serde_json::Value) -> Vec<f32> {
        let path = format!("{dir}/{}.wav", self.name);
        write_wav(&path, &self.out);
        let secs = self.out.len() as f64 / FS;
        let peak = self.out.iter().fold(0f32, |a, b| a.max(b.abs())) as f64;
        let rms = (self.out.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / self.out.len() as f64).sqrt();
        let mut r = self.w.receipt();
        r["audition"] = serde_json::json!({
            "wav": path, "seconds": secs, "peakDbfs": 20.0 * peak.max(1e-12).log10(), "rmsDbfs": 20.0 * rms.max(1e-12).log10(),
            "wallSeconds": self.wall, "realTimeFactor": secs / self.wall.max(1e-9), "extra": extra,
        });
        std::fs::write(format!("{dir}/{}.receipt.json", self.name), serde_json::to_string_pretty(&r).unwrap()).unwrap();
        let l = &r["ledger"];
        println!("{:<28} {:6.1}s peak {:6.1} dBFS rms {:6.1} dBFS  RTF {:6.1}x  closure frame {:.1e} J global {:.1e} J rel {:.1e}",
            self.name, secs, 20.0 * peak.max(1e-12).log10(), 20.0 * rms.max(1e-12).log10(), secs / self.wall.max(1e-9),
            l["maximumFrameResidual"].as_f64().unwrap_or(f64::NAN), l["globalResidual"].as_f64().unwrap_or(f64::NAN), l["relativeGlobalResidual"].as_f64().unwrap_or(f64::NAN));
        self.out
    }
}

fn write_wav(path: &str, x: &[f32]) {
    let spec = hound::WavSpec { channels: 1, sample_rate: FS as u32, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for s in x {
        w.write_sample((s.clamp(-1.0, 1.0) * 32767.0) as i16).unwrap();
    }
    w.finalize().unwrap();
}

fn smooth(t: f64, a: f64, b: f64, va: f64, vb: f64) -> f64 {
    if t <= a { va } else if t >= b { vb } else { va + (vb - va) * (t - a) / (b - a) }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = args[1].clone();
    let only: Vec<String> = args[2..].to_vec();
    std::fs::create_dir_all(&dir).unwrap();
    let want = |n: &str| only.is_empty() || only.iter().any(|o| n.contains(o.as_str()));

    if want("kart_engine_rev_sweep") {
        // idle -> redline on throttle, hold, lift (engine braking), back to idle, then on-throttle blip
        let mut r = Rec::new("kart_engine_rev_sweep", "kart_engine", 1);
        r.run(1.5, |w, _| { w.set("rpm", 1800.0); w.set("throttle", 0.0); w.set("load", 1.0); });
        r.run(7.0, |w, t| { // t from 1.5
            let u = (t - 1.5) / 7.0;
            w.set("rpm", 1800.0 + (14000.0 - 1800.0) * u.powf(1.3)); w.set("throttle", 1.0); w.set("load", 1.0);
        });
        r.run(1.0, |w, _| { w.set("rpm", 14000.0); w.set("throttle", 1.0); w.set("load", 1.0); });
        r.run(5.0, |w, t| { // lift off: throttle 0, load 0 => engine braking, rpm falls
            let u = (t - 9.5) / 5.0;
            w.set("rpm", 14000.0 - (14000.0 - 1800.0) * (1.0 - (1.0 - u).powi(2))); w.set("throttle", 0.0); w.set("load", 0.0);
        });
        r.run(2.0, |w, _| { w.set("rpm", 1800.0); w.set("throttle", 0.0); w.set("load", 1.0); });
        r.finish(&dir, serde_json::json!({"script": "idle 1.5s, full throttle ramp 1800->14000 rpm over 7s, hold 1s, lift (throttle 0, load 0) with rpm falling over 5s, idle 2s"}));
    }

    if want("kart_engine_events") {
        let mut r = Rec::new("kart_engine_events", "kart_engine", 2);
        // pull-start, catch, rev, then each event in turn
        r.run(0.3, |w, t| { w.set("rpm", 1800.0); w.set("throttle", 0.0); if t < 0.01 { w.event("start", 0.0); } });
        r.run(2.2, |w, _| { w.set("rpm", 1800.0); w.set("throttle", 0.15); });
        r.run(2.0, |w, t| { w.set("rpm", smooth(t, 2.5, 4.0, 1800.0, 9000.0)); w.set("throttle", 1.0); });
        // boost on at 4.5..6.0
        r.run(2.0, |w, t| { w.set("rpm", 9000.0); w.set("throttle", 1.0); w.set("boost", if t > 4.5 && t < 6.0 { 1.0 } else { 0.0 }); });
        // skid on track then lakebed
        r.run(1.5, |w, t| { w.set("boost", 0.0); w.set("rpm", 9000.0); w.set("skid", if t > 6.7 { 0.8 } else { 0.0 }); w.set("surface", 0.0); });
        r.run(1.5, |w, t| { let _ = t; w.set("skid", 0.0); });
        r.run(1.5, |w, t| { w.set("skid", 0.8); w.set("surface", 1.0); let _ = t; });
        r.run(1.0, |w, _| { w.set("skid", 0.0); });
        // bumps (soft, hard), land (soft, hard)
        let mut fired = [false; 4];
        r.run(4.0, |w, t| {
            w.set("rpm", 9000.0); w.set("throttle", 0.8);
            let k = ((t - 12.3) / 0.9).floor() as i32;
            if k >= 0 && k < 4 && !fired[k as usize] { fired[k as usize] = true;
                match k { 0 => { w.event("bump", 0.25); } 1 => { w.event("bump", 1.0); } 2 => { w.event("land", 0.3); } _ => { w.event("land", 1.0); } } }
        });
        // hit, engine dies ~2s, restart
        r.run(0.1, |w, _| { w.event("hit", 1.0); });
        r.run(2.4, |w, _| { w.set("rpm", 9000.0); w.set("throttle", 1.0); });
        r.run(0.1, |w, _| { w.event("restart", 0.0); });
        r.run(2.5, |w, _| { w.set("rpm", 3000.0); w.set("throttle", 0.5); });
        r.finish(&dir, serde_json::json!({"script": "start; idle; rev to 9000; boost 4.5-6.0s; skid on track and lakebed; bump .25/1.0 at 12.3,13.2s; land .3/1.0 at 14.1,15.0s; hit then 2.4 s dead; restart"}));
    }

    if want("kart_engine_8_karts_mix") {
        // eight karts on one straight: a standing start, a bump, a blown-up kart, a drift. Mixed here, outside the Worlds,
        // with a plain 1/r gain per kart (the game engine does the real spatialisation).
        let dist = [6.0, 9.0, 14.0, 18.0, 24.0, 31.0, 40.0, 55.0];
        let mut karts: Vec<Box<dyn GameWorld>> = (0..8).map(|i| create("kart_engine", FS, 100 + i as u64).unwrap()).collect();
        let mut out: Vec<f32> = vec![];
        let blk = (FS / 60.0).round() as usize;
        let secs = 14.0;
        let t_wall = Instant::now();
        for i in 0..(secs * 60.0) as usize {
            let t = i as f64 / 60.0;
            let mut mix = vec![0f32; blk];
            for (k, w) in karts.iter_mut().enumerate() {
                let t0 = 0.12 * k as f64; // staggered launch
                if i == 0 { w.event("start", 0.0); }
                let u = ((t - 0.9 - t0) / (4.5 + 0.35 * k as f64)).clamp(0.0, 1.0);
                let drift = if k == 2 && t > 8.0 && t < 10.0 { 1.0 } else { 0.0 };
                let rpm = 1800.0 + (11800.0 - 600.0 * (k as f64)) * (1.0 - (1.0 - u).powi(2)) - if t > 6.0 { 700.0 * (((t - 6.0) * (1.0 + 0.1 * k as f64) * 2.0).sin().abs()) * (k % 3) as f64 } else { 0.0 };
                w.set("rpm", rpm); w.set("throttle", if t < 0.9 + t0 { 0.1 } else { 1.0 }); w.set("load", 0.6 + 0.4 * u);
                w.set("skid", drift); if drift > 0.0 { w.set("surface", 1.0); }
                if k == 4 && (t - 5.0).abs() < 0.008 { w.event("bump", 0.8); }
                if k == 5 && (t - 9.0).abs() < 0.008 { w.event("hit", 1.0); }
                if k == 5 && (t - 11.2).abs() < 0.008 { w.event("restart", 0.0); }
                let mut b = vec![0f32; blk];
                w.render(&mut b);
                let g = 1.0 / dist[k] as f32;
                for (m, x) in mix.iter_mut().zip(b.iter()) { *m += x * g; }
            }
            out.extend_from_slice(&mix);
        }
        let wall = t_wall.elapsed().as_secs_f64();
        let peak = out.iter().fold(0f32, |a, b| a.max(b.abs()));
        let gain = if peak > 0.9 { 0.9 / peak } else { 1.0 };
        for x in out.iter_mut() { *x *= gain; }
        write_wav(&format!("{dir}/kart_engine_8_karts_mix.wav"), &out);
        let rec: Vec<_> = karts.iter().enumerate().map(|(k, w)| serde_json::json!({"kart": k, "distanceM": dist[k], "receipt": w.receipt()})).collect();
        let worst = rec.iter().map(|r| r["receipt"]["ledger"]["maximumFrameResidual"].as_f64().unwrap()).fold(0.0, f64::max);
        let worst_rel = rec.iter().map(|r| r["receipt"]["ledger"]["relativeGlobalResidual"].as_f64().unwrap()).fold(0.0, f64::max);
        let pk = out.iter().fold(0f32, |a, b| a.max(b.abs())) as f64;
        let doc = serde_json::json!({"world": "kart_engine x8", "seconds": secs, "mixGain": gain, "distancesM": dist, "wallSeconds": wall,
            "realTimeFactorAllEight": secs / wall, "worstMaximumFrameResidualJ": worst, "worstRelativeGlobalResidual": worst_rel,
            "peakDbfs": 20.0 * pk.log10() as f64, "karts": rec});
        std::fs::write(format!("{dir}/kart_engine_8_karts_mix.receipt.json"), serde_json::to_string_pretty(&doc).unwrap()).unwrap();
        println!("kart_engine_8_karts_mix      {:.1}s peak {:.1} dBFS (mix gain {:.3}), 8 Worlds at {:.1}x real time, worst frame residual {:.1e} J, worst relative global {:.1e}", secs, 20.0 * pk.log10() as f64, gain, secs / wall, worst, worst_rel);
    }

    if want("race_props_buzzer") {
        let mut r = Rec::new("race_props_buzzer", "race_props", 3);
        let mut done = std::collections::HashSet::new();
        r.run(5.5, |w, t| {
            for (i, (tt, ev, a)) in [(0.2, "beep", 3.0), (1.2, "beep", 2.0), (2.2, "beep", 1.0), (3.2, "go", 0.0)].iter().enumerate() {
                if t >= *tt && !done.contains(&i) { done.insert(i); w.event(ev, *a); }
            }
        });
        r.finish(&dir, serde_json::json!({"script": "beep 3,2,1 at 0.2,1.2,2.2 s; go at 3.2 s"}));
    }
    if want("race_props_bell") {
        let mut r = Rec::new("race_props_bell", "race_props", 4);
        let mut done = std::collections::HashSet::new();
        r.run(9.0, |w, t| {
            for (i, (tt, ev)) in [(0.2, "bell"), (3.5, "finish")].iter().enumerate() {
                if t >= *tt && !done.contains(&i) { done.insert(i); w.event(ev, 0.0); }
            }
        });
        r.finish(&dir, serde_json::json!({"script": "bell (final lap) at 0.2 s; finish (nine swung strokes) at 3.5 s"}));
    }
}
