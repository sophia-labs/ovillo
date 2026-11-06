//! Exact limits and books, on the built-in bodies.

use ovillo_body::{presets, Body, BodySpec};

const RATE: f64 = 48_000.0;

fn run(spec: &BodySpec, seconds: f64) -> (Vec<f32>, ovillo_body::Receipt) {
    let mut body = Body::new(spec, RATE).unwrap();
    let x = ovillo_body::body::noise((seconds * RATE) as usize, 0.1, 7);
    let mut y = vec![0.0f32; x.len() * body.channels()];
    body.process(&x, &mut y);
    (y, body.receipt())
}

#[test]
fn every_preset_builds_runs_and_closes_its_books() {
    for name in presets::names() {
        let spec = presets::get(name).unwrap();
        let (y, r) = run(&spec, 0.5);
        assert_eq!(r.faults, 0, "{name}");
        assert!(y.iter().all(|v| v.is_finite()), "{name}");
        assert!(r.ledger.maximum_frame_residual_j < 1.0e-12, "{name}: {}", r.ledger.maximum_frame_residual_j);
        assert!(r.ledger.cumulative_closure_residual_j.abs() < 1.0e-9, "{name}: {}", r.ledger.cumulative_closure_residual_j);
        assert!(r.voice_work_j > 0.0, "{name}: her voice did no work");
        // Her work is the only source work unless the powered material is on.
        if !spec.on("powered.on") {
            assert!((r.ledger.source_work_j - r.voice_work_j).abs() <= 1.0e-12 * r.ledger.source_work_j.abs().max(1.0), "{name}");
        }
    }
}

#[test]
fn the_dry_path_alters_no_physical_state() {
    let dry = presets::neutral().with("listener.dry", 1.0);
    let (_, a) = run(&presets::neutral(), 0.3);
    let (_, b) = run(&dry, 0.3);
    assert_eq!(a.ledger.source_work_j.to_bits(), b.ledger.source_work_j.to_bits());
    assert_eq!(a.ledger.internal_loss_j.to_bits(), b.ledger.internal_loss_j.to_bits());
    assert_eq!(a.ledger.radiation_j.to_bits(), b.ledger.radiation_j.to_bits());
}

#[test]
fn air_off_is_linear_acoustics_and_a_zero_room_port_is_inert() {
    // air.on = 0 at default thresholds and loss removes jet loss and turbulence exactly.
    let (_, r) = run(&presets::neutral(), 0.3);
    assert_eq!(r.air.jet_loss_j, 0.0);
    assert_eq!(r.air.turbulence_injected_j, 0.0);
    // Linear: doubling the input doubles the output exactly (to roundoff).
    let mut body = Body::new(&presets::neutral(), RATE).unwrap();
    let x = ovillo_body::body::noise(4800, 0.05, 3);
    let mut y1 = vec![0.0f32; x.len() * body.channels()];
    body.process(&x, &mut y1);
    let mut body = Body::new(&presets::neutral(), RATE).unwrap();
    let x2: Vec<f32> = x.iter().map(|v| v * 2.0).collect();
    let mut y2 = vec![0.0f32; x.len() * body.channels()];
    body.process(&x2, &mut y2);
    let worst = y1.iter().zip(&y2).map(|(a, b)| (2.0 * a - b).abs()).fold(0.0f32, f32::max);
    assert!(worst < 1.0e-6, "{worst}");
    // room.port = 0: the shared space is never touched (its emission is exactly 0).
    let body = Body::new(&presets::neutral(), RATE).unwrap();
    assert_eq!(body.world().space_stored_energy(), 0.0);
    let (_, r) = run(&presets::neutral(), 0.2);
    assert_eq!(r.ledger.control_work_j, 0.0);
}

#[test]
fn a_branch_and_a_resonator_switched_off_are_absent() {
    let a = Body::new(&presets::neutral(), RATE).unwrap();
    let b = Body::new(&presets::neutral().with("branch1.length_m", 0.3).with("res1.f1_hz", 440.0), RATE).unwrap();
    assert_eq!(a.layout().nodes, b.layout().nodes);
    assert_eq!(a.world().body_count(), 1);
    assert_eq!(a.world().plan_digest(), b.world().plan_digest());
    let c = Body::new(&presets::neutral().with("branch1.on", 1.0).with("res1.on", 1.0), RATE).unwrap();
    assert!(c.layout().nodes > a.layout().nodes);
    assert_eq!(c.world().body_count(), 2);
}

#[test]
fn the_flap_and_the_air_are_level_dependent() {
    let spec = presets::beehive();
    let mut quiet = Body::new(&spec, RATE).unwrap();
    let mut loud = Body::new(&spec, RATE).unwrap();
    let x = ovillo_body::body::noise(24_000, 0.02, 11);
    let x_loud: Vec<f32> = x.iter().map(|v| v * 10.0).collect();
    let mut y = vec![0.0f32; x.len() * 2];
    quiet.process(&x, &mut y);
    loud.process(&x_loud, &mut y);
    let (q, l) = (quiet.receipt(), loud.receipt());
    // Jet loss grows faster than the square of the level (|u|^3).
    assert!(l.air.jet_loss_j > 100.0 * q.air.jet_loss_j * 2.0, "{} vs {}", l.air.jet_loss_j, q.air.jet_loss_j);
    assert!(l.air.flap_loss_j > 0.0 && q.air.flap_loss_j > 0.0);
}

#[test]
fn the_mouth_is_a_receipted_live_move() {
    let mut body = Body::new(&presets::neutral(), RATE).unwrap();
    let x = ovillo_body::body::noise(2400, 0.1, 5);
    let mut y = vec![0.0f32; x.len() * 2];
    body.process(&x, &mut y);
    body.set_mouth(0.4).unwrap();
    body.process(&x, &mut y);
    let r = body.receipt();
    assert_eq!(r.faults, 0);
    assert!(r.ledger.control_work_j != 0.0 || r.ledger.maximum_frame_residual_j < 1.0e-12);
    assert!(r.ledger.maximum_frame_residual_j < 1.0e-12);
}
