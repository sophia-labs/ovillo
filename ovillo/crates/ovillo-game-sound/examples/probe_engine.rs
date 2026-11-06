use ovillo_game_sound::kart_engine::KartEngine;
use ovillo_game_sound::GameWorld;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let rpm: f64 = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(6000.0);
    let thr: f64 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(1.0);
    let fs = 48000.0;
    let mut e = KartEngine::new(fs);
    e.set("rpm", rpm); e.set("throttle", thr); e.set("load", 1.0);
    let mut buf = vec![0f32; 48000];
    let t0 = std::time::Instant::now();
    for s in 0..3 { e.render(&mut buf); 
        let pk = buf.iter().fold(0f32, |a, b| a.max(b.abs())) as f64 * 60.0;
        let rms = (buf.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / buf.len() as f64).sqrt() * 60.0;
        println!("xi {:.3} fresh {:.3e}", e.xi_last, e.fresh_last); println!("sec {s}: peak {:.2} Pa rms {:.2} Pa  fires {} mis {} revs {} peakcyl {:.2e}", pk, rms, e.fires(), e.misfires(), e.revs(), e.peak_cyl);
    }
    println!("wall {:.2}s", t0.elapsed().as_secs_f64());
    println!("{}", serde_json::to_string_pretty(&e.receipt()["ledger"]).unwrap());
}
