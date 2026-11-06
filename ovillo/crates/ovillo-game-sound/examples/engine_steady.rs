// Renders steady-state engine snippets for analysis: engine_steady <outdir>
use ovillo_game_sound::kart_engine::KartEngine;
use ovillo_game_sound::GameWorld;
fn main() {
    let dir = std::env::args().nth(1).unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    for &(rpm, thr, load) in &[(1800.0,0.0,1.0),(1800.0,0.0,0.0),(3000.0,1.0,1.0),(6000.0,1.0,1.0),(6000.0,0.0,0.0),(6000.0,0.5,1.0),(9000.0,1.0,1.0),(12000.0,1.0,1.0),(14000.0,1.0,1.0)] {
        let mut e = KartEngine::new(48000.0);
        e.set("rpm", rpm); e.set("throttle", thr); e.set("load", load);
        let mut warm = vec![0f32; 48000]; e.render(&mut warm);
        let f0 = e.fires();
        let mut buf = vec![0f32; 48000]; e.render(&mut buf);
        let name = format!("{dir}/rpm{}_thr{}_load{}.wav", rpm as i32, thr, load);
        let spec = hound::WavSpec{channels:1,sample_rate:48000,bits_per_sample:32,sample_format:hound::SampleFormat::Float};
        let mut w = hound::WavWriter::create(&name, spec).unwrap();
        for s in &buf { w.write_sample(*s).unwrap(); }
        w.finalize().unwrap();
        println!("{name} fires_in_1s={} misfires={}", e.fires()-f0, e.misfires());
    }
}
