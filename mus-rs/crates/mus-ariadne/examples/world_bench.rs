//! Reproducible benchmark for the canonical Ariadne sample path.
//! Run: `cargo run -p mus-ariadne --release --example world_bench`

use mus_ariadne::{World, WorldControls, WorldSpec};
use std::hint::black_box;
use std::time::Instant;

fn main() {
    for rate in [48_000.0, 96_000.0, 192_000.0] {
        run_case("clean", rate, false);
        run_case("active", rate, true);
    }
}

fn run_case(label: &str, rate: f64, active: bool) {
    let mut spec = WorldSpec::string_brass_v1();
    spec.internal_sample_rate_hz = rate;
    let mut world = World::try_new(spec).expect("valid benchmark world");
    let mut controls = if active {
        WorldControls {
            active_coupling: 1.0,
            active: mus_ariadne::law::active::ActiveInclusionControls {
                drive: 1.05,
                headroom: 0.28,
                sustain: 0.78,
                sag: 0.68,
                feedback: 0.10,
                ..Default::default()
            },
            work_input: 0.010,
            transport: mus_ariadne::law::transport::ResonantTransportControls {
                interlace: 0.58,
                memory: 0.68,
            },
            ..Default::default()
        }
    } else {
        WorldControls::default()
    };

    let warmup = rate as usize / 4;
    let frames = rate as usize * 2;
    let mut checksum = 0.0;
    for sample in 0..warmup {
        controls.source_wave = source(sample, rate as usize);
        let frame = world.process_sample(controls).expect("finite warmup");
        checksum += frame.left + frame.right;
    }

    let started = Instant::now();
    for sample in 0..frames {
        controls.source_wave = source(sample, rate as usize);
        let frame = world.process_sample(controls).expect("finite benchmark");
        checksum += frame.left + frame.right;
    }
    let seconds = started.elapsed().as_secs_f64();
    let receipt = world.receipt();
    println!(
        "case={label} rate={rate:.0} frames={frames} seconds={seconds:.9} ns_per_sample={:.3} real_time_factor={:.3} delay_cells={} delay_bytes={} structural_ops={} max_local_residual={:.6e} global_residual={:.6e} checksum={:.12e}",
        seconds * 1.0e9 / frames as f64,
        (frames as f64 / rate) / seconds,
        receipt.runtime_layout.delay_cells,
        receipt.runtime_layout.delay_heap_bytes,
        receipt.structural_cost.estimated_scalar_operations,
        receipt.maximum_local_residual,
        receipt.global_closure_residual,
        black_box(checksum),
    );
}

#[inline]
fn source(sample: usize, sample_rate: usize) -> f64 {
    let phase = sample % (sample_rate / 5);
    if phase < 96 {
        let x = phase as f64 / 96.0;
        0.36 * (1.0 - (std::f64::consts::TAU * x).cos())
    } else {
        0.0
    }
}
