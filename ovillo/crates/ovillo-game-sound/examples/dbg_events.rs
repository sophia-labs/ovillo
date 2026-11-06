use ovillo_game_sound::kart_engine::KartEngine;
use ovillo_game_sound::GameWorld;
fn run(name: &str, f: impl Fn(&mut KartEngine)) {
    let mut e = KartEngine::new(48000.0);
    e.set("rpm", 9000.0); e.set("throttle", 0.8);
    let mut b = vec![0f32; 4800];
    for _ in 0..10 { e.render(&mut b); }
    f(&mut e);
    let mut pk = 0f32;
    for _ in 0..40 { e.render(&mut b); pk = pk.max(b.iter().fold(0f32, |a, x| a.max(x.abs()))); }
    let l = e.receipt()["ledger"].clone();
    println!("{name}: peak {:.3} maxframe {:.2e} global {:.2e}", pk, l["maximumFrameResidual"].as_f64().unwrap(), l["globalResidual"].as_f64().unwrap());
}
fn main() {
    run("none", |_| {});
    run("bump", |e| { e.event("bump", 1.0); });
    run("land", |e| { e.event("land", 1.0); });
    run("hit", |e| { e.event("hit", 1.0); });
    run("start-run", |e| { e.event("start", 0.0); });
    run("skid", |e| { e.set("skid", 0.8); });
    run("hit-wait-restart", |e| { e.event("hit", 1.0); let mut b = vec![0f32; 4800]; for _ in 0..24 { e.render(&mut b); } e.event("restart", 0.0); });
    run("stop-start", |e| { e.event("hit", 1.0); let mut b = vec![0f32; 4800]; for _ in 0..60 { e.render(&mut b); } println!("omega {}", e.omega()); e.event("restart", 0.0); });
    run("skid-lake", |e| { e.set("surface", 1.0); e.set("skid", 0.8); let mut b = vec![0f32; 4800]; for _ in 0..40 { e.render(&mut b); } });
    run("skid-then-off", |e| { e.set("skid", 0.8); let mut b = vec![0f32; 4800]; for _ in 0..20 { e.render(&mut b); } e.set("skid", 0.0); });
    run("boost", |e| { e.set("boost", 1.0); });
}
#[allow(dead_code)]
fn seq() {}
