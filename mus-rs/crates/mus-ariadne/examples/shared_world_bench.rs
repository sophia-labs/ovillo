//! Release-profile timing and closure probe for one genuinely shared Ariadne
//! world. Several bodies exchange with one spatial field and one global ledger.
//!
//! Usage:
//! `cargo run -p mus-ariadne --release --example shared_world_bench -- [BODIES] [SECONDS]`

use std::env;
use std::time::Instant;

use mus_ariadne::{
    BodyControls, BodyInstanceSpec, CouplingControls, DirectCouplingSpec, ObserverBodyTap,
    SharedWorldControls, World, WorldPlan, WorldSpec, MODE_COUNT,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("ariadne-shared-world-bench: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let body_count = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| error.to_string())?
        .unwrap_or(4)
        .clamp(1, 32);
    let seconds = args
        .next()
        .map(|value| value.parse::<f64>())
        .transpose()
        .map_err(|error| error.to_string())?
        .unwrap_or(2.0)
        .clamp(0.1, 30.0);
    if args.next().is_some() {
        return Err("usage: shared_world_bench [BODIES] [SECONDS]".to_owned());
    }

    for sample_rate_hz in [48_000.0, 96_000.0, 192_000.0] {
        let plan = ring_plan(body_count, sample_rate_hz)?;
        let mut world = World::from_plan(plan).map_err(|error| error.to_string())?;
        let frames = (seconds * sample_rate_hz).round() as usize;
        let mut controls = vec![BodyControls::default(); body_count];
        let shared = SharedWorldControls {
            coupling: CouplingControls { scale: 0.75 },
            ..Default::default()
        };
        let started = Instant::now();
        let mut checksum = 0.0_f64;
        for sample in 0..frames {
            for (index, control) in controls.iter_mut().enumerate() {
                control.source_wave = if sample == 64 + index * 23 {
                    0.55 / (1.0 + 0.15 * index as f64)
                } else {
                    0.0
                };
                control.active_coupling = if index % 2 == 0 { 0.35 } else { 0.0 };
                control.active.drive = 0.55;
                control.work_input = 0.002;
            }
            let frame = world
                .process_shared_sample(&controls, shared)
                .map_err(|error| error.to_string())?;
            checksum += frame.primary.left * 0.31 + frame.primary.right * 0.17;
        }
        let elapsed = started.elapsed().as_secs_f64();
        let ns_per_sample = 1.0e9 * elapsed / frames.max(1) as f64;
        let realtime = seconds / elapsed.max(f64::MIN_POSITIVE);
        let receipt = world.shared_receipt();
        println!(
            "rate={sample_rate_hz:.0} bodies={} edges={} observers={} frames={} ns/sample={ns_per_sample:.1} realtime={realtime:.2}x residual={:.3e} local={:.3e} checksum={checksum:.9e} delay_bytes={} structural_ops={}",
            receipt.body_count,
            receipt.direct_coupling_count,
            receipt.observer_count,
            receipt.world.sample_frames,
            receipt.world.global_closure_residual,
            receipt.world.maximum_local_residual,
            receipt.world.runtime_layout.delay_heap_bytes,
            receipt.world.structural_cost.estimated_scalar_operations,
        );
    }
    Ok(())
}

fn ring_plan(body_count: usize, sample_rate_hz: f64) -> Result<WorldPlan, String> {
    let mut spec = WorldSpec::string_brass_v1();
    spec.internal_sample_rate_hz = sample_rate_hz;
    let mut plan = WorldPlan::from_single_body(spec);
    let template = plan.bodies[0].clone();
    plan.bodies.clear();
    plan.observers[0].body_taps.clear();
    for index in 0..body_count {
        let mut body: BodyInstanceSpec = template.clone();
        body.id = format!("body/{index}");
        let phase = std::f64::consts::TAU * index as f64 / body_count as f64;
        body.pose.position = [phase.cos(), phase.sin(), 0.0];
        plan.observers[0].body_taps.push(ObserverBodyTap {
            body_id: body.id.clone(),
            left_projection: 0.5 + 0.5 * phase.cos(),
            right_projection: 0.5 - 0.5 * phase.cos(),
        });
        plan.bodies.push(body);
    }
    if body_count > 1 {
        for index in 0..body_count {
            let right = (index + 1) % body_count;
            let left_direction = unit_direction(index % MODE_COUNT);
            let right_direction = unit_direction((right + 3) % MODE_COUNT);
            plan.direct_couplings.push(DirectCouplingSpec {
                id: format!("ring/{index}-{right}"),
                left_body: format!("body/{index}"),
                right_body: format!("body/{right}"),
                left_direction,
                right_direction,
                rate_hz: 90.0 + 7.0 * index as f64,
                weight: 0.35,
            });
        }
    }
    plan.world_name = format!("shared-ring-{body_count}");
    plan.validate().map_err(|error| error.to_string())?;
    Ok(plan)
}

fn unit_direction(index: usize) -> [f64; MODE_COUNT] {
    std::array::from_fn(|candidate| if candidate == index { 1.0 } else { 0.0 })
}
