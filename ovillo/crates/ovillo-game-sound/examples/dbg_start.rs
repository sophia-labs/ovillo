use ovillo_game_sound::kart_engine::KartEngine;
use ovillo_game_sound::GameWorld;
fn main() {
    let mut e = KartEngine::new(48000.0);
    e.set("rpm", 1800.0); e.set("throttle", 0.15);
    let mut b = vec![0f32; 4800];
    e.event("start", 0.0);
    for i in 0..30 { e.render(&mut b); println!("t={:.1} rpm {:.0} fires {} mis {} pk {:.3}", (i+1) as f64*0.1, e.omega()*60.0/6.283, e.fires(), e.misfires(), b.iter().fold(0f32,|a,x|a.max(x.abs()))); }
    println!("{}", e.receipt()["ledger"]["maximumFrameResidual"]);
}
