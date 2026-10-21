//! The public engine's vowels (`TractNdAnatomy::public_v1`: air and slots on)
//! beside the checkpoint table, same procedure and measurement.
use mus_ariadne::{ConnectionPolicy, DisposalPolicy};
use ovillo_tract_world::world::{self as tw, TractControls, TractWorldNd};
use ovillo_tract_world::{measure, modal, TractNdAnatomy};

fn main() {
    let sr = 48_000.0;
    use ovillo_tract_world::world::Articulation;
    // ovillo-tract fe4fb03's re-fit for its exactly sealed velum.
    let _ = Articulation { tongue_index: 0.0, tongue_diameter: 0.0, lips: 0.0, velum: 0.0 };
    let refit = [tw::PUBLIC_ART_A, tw::PUBLIC_ART_I, tw::PUBLIC_ART_U];
    for (v, (name, art, target)) in [("a", tw::ART_A, tw::TARGET_A_HZ), ("i", tw::ART_I, tw::TARGET_I_HZ), ("u", tw::ART_U, tw::TARGET_U_HZ)].into_iter().enumerate() {
        for (label, anatomy, art) in [("checkpoint", TractNdAnatomy::default(), art), ("public", TractNdAnatomy::public_v1(), refit[v])] {
            let mut w = TractWorldNd::with(sr, anatomy, ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate }, tw::ART_REST).unwrap();
            w.set_controls(TractControls { articulation: art, pitch_hz: 120.0, tenseness: 0.6, breath: 0.6 });
            let mut audio = vec![0.0f32; (1.4 * sr) as usize];
            w.process(&mut audio).unwrap();
            let x: Vec<f64> = audio.iter().map(|&v| f64::from(v)).collect();
            let tail = &x[x.len() - (0.4 * sr) as usize..];
            let frames: Vec<Vec<measure::Formant>> = (0..3).map(|k| measure::lpc_formants(&tail[k * 6_000..k * 6_000 + 2_400], sr)).collect();
            let median = |i: usize| -> Option<f64> {
                let mut v: Vec<f64> = frames.iter().filter_map(|f| f.get(i).map(|q| q.hz)).collect();
                v.sort_by(f64::total_cmp);
                v.get(v.len() / 2).copied()
            };
            let modes = w.world().space_body(0).unwrap().network_modes().unwrap();
            let peaks = modal::response_peaks(&modes, 90.0, 4_500.0);
            let r = w.world().receipt();
            println!("/{name}/ {label:<10} target {target:?} render LPC {:?} modal {:?} F0 {:.1} max frame {:.1e} J",
                (0..3).map(|i| median(i).map(f64::round)).collect::<Vec<_>>(), peaks.iter().take(3).map(|v| v.round()).collect::<Vec<_>>(),
                measure::f0_autocorrelation(&tail[..9_600], sr), r.maximum_frame_residual);
        }
    }
}
