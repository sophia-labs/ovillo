//! Reproducible benchmark for the canonical Ariadne sample path: the one-body
//! world (a one-element `WorldPlan`) heard by the plug-in's stereo listener.
//! Run: `cargo run -p mus-ariadne --release --example world_bench`

use mus_ariadne::{
    BodyControls, StereoListener, StereoListenerSpec, World, WorldControls, WorldCostEstimate,
    WorldLayout, WorldPlan,
};
use std::hint::black_box;
use std::time::Instant;

fn main() {
    for rate in [48_000.0, 96_000.0, 192_000.0] {
        run_case("clean", rate, false);
        run_case("active", rate, true);
    }
}

fn run_case(label: &str, rate: f64, active: bool) {
    let plan = WorldPlan::string_brass_v1(rate);
    let listener_spec = StereoListenerSpec::all_bodies(&plan, 1.0).expect("valid listener");
    let listener = StereoListener::bind(&listener_spec, &plan).expect("bound listener");
    let layout = WorldLayout::from_plan(&plan);
    let cost = WorldCostEstimate::for_plan(&plan);
    let mut world = World::from_plan(plan).expect("valid benchmark world");
    let world_controls = WorldControls::default();
    let mut controls = if active {
        BodyControls {
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
        BodyControls::default()
    };

    let warmup = rate as usize / 4;
    let frames = rate as usize * 2;
    let mut checksum = 0.0;
    for sample in 0..warmup {
        controls.source_wave = source(sample, rate as usize);
        let frame = world
            .process_sample(&[controls], world_controls)
            .expect("finite warmup");
        let heard = listener.observe(&world, &frame).expect("finite listener");
        checksum += heard.left + heard.right;
    }

    let started = Instant::now();
    for sample in 0..frames {
        controls.source_wave = source(sample, rate as usize);
        let frame = world
            .process_sample(&[controls], world_controls)
            .expect("finite benchmark");
        let heard = listener.observe(&world, &frame).expect("finite listener");
        checksum += heard.left + heard.right;
    }
    let seconds = started.elapsed().as_secs_f64();
    let receipt = world.receipt();
    println!(
        "case={label} rate={rate:.0} frames={frames} seconds={seconds:.9} ns_per_sample={:.3} real_time_factor={:.3} delay_cells={} delay_bytes={} structural_ops={} max_local_residual={:.6e} global_residual={:.6e} checksum={:.12e}",
        seconds * 1.0e9 / frames as f64,
        (frames as f64 / rate) / seconds,
        layout.delay_cells,
        layout.delay_heap_bytes,
        cost.estimated_scalar_operations,
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
