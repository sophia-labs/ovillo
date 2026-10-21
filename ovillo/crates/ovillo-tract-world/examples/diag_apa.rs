//! Diagnostic trace of /apa/ with the PoC air (every 2 ms): closure state,
//! oral pressure, output rms, disposal and moves.
use mus_ariadne::{ConnectionPolicy, DisposalPolicy};
use ovillo_tract_world::world::{self as tw, Articulation, TractControls, TractWorldNd};
use ovillo_tract_world::TractNdAnatomy;
fn main() {
    let sr = 48_000.0;
    let mut w = TractWorldNd::with(sr, TractNdAnatomy::default().with_air(), ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate }, tw::ART_A).unwrap();
    let art_p = Articulation { lips: 0.0, ..tw::ART_A };
    let behind = w.anatomy().oral_sections / 2;
    let mut acc = 0.0;
    for i in 0..(0.7 * sr) as usize {
        let t = i as f64 / sr;
        let art = if (0.30..0.48).contains(&t) { art_p } else { tw::ART_A };
        w.set_controls(TractControls { articulation: art, pitch_hz: 120.0, tenseness: 0.6, breath: 0.6 });
        let y = w.tick().unwrap();
        acc += y * y;
        if i % 96 == 95 && t > 0.38 && t < 0.56 {
            let o = w.world().body_space_observation(0).unwrap();
            let b = w.world().space_body(0).unwrap();
            let f = b.frequencies_hz();
            println!("t {:.3} closed {} jet {} A {:.4} cm2 oral {:7.1} Pa out_rms {:.4} disposal {:.3e} moves {} f0..2 {:.2} {:.1} {:.1} lips {:.4} end_p {:.2e} {:.2e}",
                t, o.aero.jet_closed, o.aero.jet_junction, o.aero.jet_aperture_m2 * 1e4, b.node_pressure(behind).unwrap(), (acc / 96.0).sqrt(), w.world().disposal_total(), w.stats().moves, f[0], f[1], f[2], w.current().articulation.lips, o.end_pressure_pa[0], o.end_pressure_pa[1]);
            acc = 0.0;
        } else if i % 96 == 95 {
            acc = 0.0;
        }
    }
}
