//! A fingerprint of the tract World's output: FNV-1a 64 over the f32 bits of
//! a held /a/ (1 s) and the /aiua/ glide (2.8 s) at the wrapper defaults,
//! plus the plan digest. Recorded at the checkpoint so later phases can prove
//! that their linear limit is the checkpoint, bit for bit.
//! `cargo run --release -p ovillo-tract-world --example render_hash`
use ovillo_tract_world::bench::Workload;
use ovillo_tract_world::world as tw;
use ovillo_tract_world::TractNdAnatomy;
use mus_ariadne::{ConnectionPolicy, DisposalPolicy};

fn fnv(samples: &[f32]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for s in samples {
        for b in s.to_bits().to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

fn main() {
    let plan = TractNdAnatomy::default().plan(48_000.0, ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate }, tw::ART_REST.point());
    println!("plan digest (44+31, mass-momentum, rest): {}", plan.digest().unwrap());
    for (name, moving, seconds) in [("held /a/", false, 1.0), ("/aiua/ glide", true, 2.8)] {
        let mut w = Workload::new(44, 31, 32, moving, 48_000.0, 1.0e-3);
        let mut out = vec![0.0f32; (seconds * 48_000.0) as usize];
        w.render(&mut out);
        let r = w.world.world().receipt();
        println!("{name}: fnv1a64 {:016x}, cumulative residual {:e} J, control work {:e} J", fnv(&out), r.cumulative_closure_residual, r.control_work);
    }
}
