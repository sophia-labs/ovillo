//! The bow on the Ariadne steel-string (`SPEC-ARIADNE-STEEL-STRING.md`,
//! "The bow"): gates, with their numbers printed (`--nocapture`).
//!
//! The world is the steel-string plan as `mus render` plays it (the engine's
//! radiation fractions), bowed on string/3 (G3, 196 Hz) at the engine's
//! default bow point. The velocity and force units mirror
//! `mus-engine/src/ariadne_tab.rs` (`BOW_VELOCITY_UNIT`, `BOW_FORCE_UNIT`,
//! `BOW_SLIP_VELOCITY`): a mezzo-forte bow is `speed = force = 0.5`.

use std::f64::consts::PI;

use mus_ariadne::{
    BodyControls, BowControls, BowSpec, StereoListener, StereoListenerSpec, World, WorldControls,
    WorldPlan, MODE_COUNT,
};

const RATE: f64 = 48_000.0;
const STRING: &str = "string/3";
const BOW_FRACTION: f64 = 0.12;
/// Bow velocity at `speed = 1`, in the string body's velocity units.
const VELOCITY_UNIT: f64 = 1.6e-5;
/// Normal force at `force = 1`, in the body's force units.
const FORCE_UNIT: f64 = 0.16;
const SLIP_VELOCITY: f64 = 1.0e-6;

fn spec() -> BowSpec {
    BowSpec {
        slip_velocity: SLIP_VELOCITY,
        ..BowSpec::default()
    }
}

struct Rig {
    world: World,
    listener: StereoListener,
    controls: Vec<BodyControls>,
    string: usize,
    bow: Option<usize>,
    shape: [f64; MODE_COUNT],
    f0: f64,
}

fn build_rig(with_bow: bool) -> Rig {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../profiles/ariadne-steel-string-v0.plan.json");
    let plan: WorldPlan = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(plan.internal_sample_rate_hz, RATE);
    let listener =
        StereoListener::bind(&StereoListenerSpec::all_bodies(&plan, 1.0).unwrap(), &plan).unwrap();
    let mut world = World::from_plan(plan.clone()).unwrap();
    let controls: Vec<BodyControls> = (0..world.body_count())
        .map(|i| {
            let id = world.body_id(i).unwrap();
            let f = &plan.bodies[i]
                .body
                .configuration_path
                .left
                .metric
                .frequency_hz;
            let mean = f.iter().sum::<f64>() / f.len() as f64;
            BodyControls {
                radiation_fraction: if id.starts_with("string/") {
                    2.0e-6
                } else {
                    (0.02 * (mean / 1000.0).max(1.0)).min(0.5)
                },
                ..BodyControls::default()
            }
        })
        .collect();
    let string = world.body_index(STRING).unwrap();
    let f0 = plan.bodies[string]
        .body
        .configuration_path
        .left
        .metric
        .frequency_hz[0];
    let bow = with_bow.then(|| world.attach_bow(string, BOW_FRACTION, spec()).unwrap());
    let shape = bow.map_or([0.0; MODE_COUNT], |b| *world.bow_shape(b).unwrap());
    Rig {
        world,
        listener,
        controls,
        string,
        bow,
        shape,
        f0,
    }
}

impl Rig {
    /// One sample; returns what the listener hears (mono) and the bow point's velocity.
    fn tick(&mut self) -> (f64, f64) {
        let frame = self
            .world
            .process_sample(&self.controls, WorldControls::default())
            .unwrap();
        let heard = self.listener.observe(&self.world, &frame).unwrap();
        let state = self.world.body_state(self.string).unwrap();
        let velocity = RATE.recip().sqrt()
            * self
                .shape
                .iter()
                .zip(&state.momentum)
                .map(|(c, p)| c * p)
                .sum::<f64>();
        (0.5 * (heard.left + heard.right), velocity)
    }

    fn string_energy(&self) -> f64 {
        self.world.body_material_energy(self.string).unwrap()
    }

    fn set_bow(&mut self, speed: f64, force: f64) {
        self.world
            .set_bow(
                self.bow.unwrap(),
                BowControls {
                    velocity: VELOCITY_UNIT * speed,
                    force: FORCE_UNIT * force,
                },
            )
            .unwrap();
    }

    /// Bow for `seconds` at constant `speed` and `force`; the heard signal.
    fn bow_for(&mut self, speed: f64, force: f64, seconds: f64) -> Vec<f64> {
        self.set_bow(speed, force);
        (0..(seconds * RATE) as usize)
            .map(|_| self.tick().0)
            .collect()
    }

    /// A mezzo-forte pluck through the engine's contact port (its pulse, its gain).
    fn pluck_for(&mut self, seconds: f64) -> Vec<f64> {
        let width = 0.00006 + 0.0003 * 0.55;
        (0..(seconds * RATE) as usize)
            .map(|k| {
                let t = k as f64 / RATE;
                self.controls[self.string].source_wave = if t < width {
                    25.0 * 0.5 * (1.0 - (2.0 * PI * t / width).cos())
                } else {
                    0.0
                };
                self.tick().0
            })
            .collect()
    }

    fn totals(&self) -> mus_ariadne::BowTotals {
        self.world.bow_totals(self.bow.unwrap()).unwrap()
    }
}

/// The strongest spectral peak within a minor third of `near` (Goertzel scan, 0.1 Hz).
fn peak_frequency(x: &[f64], near: f64) -> f64 {
    let mut best = (0.0, near);
    let mut f = near * 0.84;
    while f < near * 1.19 {
        let w = 2.0 * PI * f / RATE;
        let (mut s1, mut s2) = (0.0, 0.0);
        for (i, v) in x.iter().enumerate() {
            let hann = 0.5 - 0.5 * (2.0 * PI * i as f64 / x.len() as f64).cos();
            let s = v * hann + 2.0 * w.cos() * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        let power = s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2;
        if power > best.0 {
            best = (power, f);
        }
        f += 0.1;
    }
    best.1
}

fn rms(x: &[f64]) -> f64 {
    (x.iter().map(|v| v * v).sum::<f64>() / x.len().max(1) as f64).sqrt()
}

fn window(x: &[f64], from: f64, to: f64) -> &[f64] {
    &x[(from * RATE) as usize..(to * RATE) as usize]
}

/// Stick fraction and releases per second between two readings of the totals.
fn regime(before: mus_ariadne::BowTotals, after: mus_ariadne::BowTotals) -> (f64, f64) {
    let bowed = (after.bowed_samples - before.bowed_samples) as f64;
    (
        (after.stuck_samples - before.stuck_samples) as f64 / bowed,
        (after.slip_onsets - before.slip_onsets) as f64 / (bowed / RATE),
    )
}

#[test]
fn a_lifted_bow_changes_nothing() {
    let mut bare = build_rig(false);
    let mut bowed = build_rig(true);
    bowed.set_bow(0.5, 0.0);
    let (a, b) = (bare.pluck_for(0.5), bowed.pluck_for(0.5));
    assert_eq!(a, b, "a resting bow with no force is not there");
    let (ra, rb) = (bare.world.receipt(), bowed.world.receipt());
    assert_eq!(ra.source_work, rb.source_work);
    assert_eq!(ra.internal_loss, rb.internal_loss);
    assert_eq!(bowed.totals(), mus_ariadne::BowTotals::default());
}

#[test]
fn a_bowed_note_sustains_and_the_books_close() {
    // The same note plucked, for the scale.
    let mut plucked = build_rig(false);
    let pluck = plucked.pluck_for(2.0);
    let pluck_early = rms(window(&pluck, 0.05, 0.55));
    let pluck_late = rms(window(&pluck, 1.5, 2.0));

    let mut rig = build_rig(true);
    let heard = rig.bow_for(0.5, 0.5, 2.0);
    let t1 = rig.totals();
    let held = rig.bow_for(0.5, 0.5, 0.5);
    let t2 = rig.totals();
    let (stuck, releases) = regime(t1, t2);
    let early = rms(window(&heard, 0.5, 1.0));
    let late = rms(window(&heard, 1.5, 2.0));
    let pitch = peak_frequency(&held, rig.f0);
    let cents = 1200.0 * (pitch / rig.f0).log2();
    let r = rig.world.receipt();
    println!(
        "mf bow 2.5 s on G3: heard rms 0.5-1.0 s {early:.3e}, 1.5-2.0 s {late:.3e} (plucked mf: 0.05-0.55 s {pluck_early:.3e}, 1.5-2.0 s {pluck_late:.3e}); \
         string energy held {:.3e} J; pitch {pitch:.1} Hz ({cents:+.1} cents of {:.1}); stuck {stuck:.3}, releases/s {releases:.1}; \
         bow source {:.4e} J, friction loss {:.4e} J ({:.1} %), peak force {:.3e}; \
         world source {:.4e} control {:.4e} loss {:.4e} radiation {:.4e}; residual cumulative {:.2e}, max frame {:.2e}, max local {:.2e}",
        rig.string_energy(),
        rig.f0,
        t2.source_work,
        t2.friction_loss,
        100.0 * t2.friction_loss / t2.source_work,
        t2.peak_force,
        r.source_work,
        r.control_work,
        r.internal_loss,
        r.radiation,
        r.cumulative_closure_residual,
        r.maximum_frame_residual,
        r.maximum_local_residual
    );
    assert!(
        late > 0.5 * early && late < 2.0 * early,
        "a held note holds"
    );
    assert!(
        late > 50.0 * pluck_late,
        "the plucked note has decayed; the bowed one has not"
    );
    assert!(
        late > 0.3 * pluck_early && late < 3.0 * pluck_early,
        "mezzo-forte is mezzo-forte"
    );
    assert!(cents.abs() < 6.0, "{cents}");
    assert!(
        (0.6..0.95).contains(&stuck),
        "Helmholtz-like: stuck most of the period ({stuck})"
    );
    assert!(
        (releases / rig.f0 - 1.0).abs() < 0.03,
        "one release per period ({releases}/s)"
    );
    assert!(t2.source_work > 0.0 && t2.friction_loss > 0.0 && t2.friction_loss < t2.source_work);
    // The bow is the only source; the silent contact port returns a little
    // of the string's motion to the hand (negative boundary work, ~1e-12 J).
    assert!(
        (r.source_work - t2.source_work).abs() < 1.0e-4 * t2.source_work,
        "{} vs {}",
        r.source_work,
        t2.source_work
    );
    assert!(r.internal_loss > t2.friction_loss);
    assert!(
        r.cumulative_closure_residual.abs() < 1.0e-15,
        "{}",
        r.cumulative_closure_residual
    );
    assert!(
        r.maximum_frame_residual < 1.0e-17,
        "{}",
        r.maximum_frame_residual
    );
    assert!(
        r.maximum_local_residual < 1.0e-13,
        "{}",
        r.maximum_local_residual
    );
}

#[test]
fn the_bow_lifts_and_the_note_decays() {
    let mut rig = build_rig(true);
    let held = rig.bow_for(0.5, 0.5, 1.5);
    let totals = rig.totals();
    let after = rig.bow_for(0.5, 0.0, 1.5);
    assert_eq!(rig.totals(), totals, "a lifted bow books nothing more");
    let (held_level, soon, later) = (
        rms(window(&held, 1.0, 1.5)),
        rms(window(&after, 0.0, 0.5)),
        rms(window(&after, 1.0, 1.5)),
    );
    println!("lift: held {held_level:.3e}, 0-0.5 s after {soon:.3e}, 1.0-1.5 s after {later:.3e}");
    assert!(soon < held_level && later < 0.25 * held_level);
    let r = rig.world.receipt();
    assert!(r.cumulative_closure_residual.abs() < 1.0e-15);
}

#[test]
fn an_up_bow_mirrors_a_down_bow() {
    let mut down = build_rig(true);
    let mut up = build_rig(true);
    let (d, u) = (down.bow_for(0.5, 0.5, 1.0), up.bow_for(-0.5, 0.5, 1.0));
    let peak = d.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
    let worst = d
        .iter()
        .zip(&u)
        .map(|(a, b)| (a + b).abs())
        .fold(0.0_f64, f64::max);
    println!("up/down mirror: peak {peak:.3e}, worst |down + up| {worst:.3e}");
    assert!(worst < 1.0e-9 * peak);
    let (td, tu) = (down.totals(), up.totals());
    assert_eq!(td.stuck_samples, tu.stuck_samples);
    assert!((td.source_work - tu.source_work).abs() < 1.0e-9 * td.source_work);
}

#[test]
fn a_swell_follows_the_bow_speed() {
    let mut rig = build_rig(true);
    let n = (4.0 * RATE) as usize;
    let mut heard = Vec::with_capacity(n);
    let mut marks = Vec::new();
    let mut last = rig.totals();
    for k in 0..n {
        let x = k as f64 / n as f64;
        // 0.2 -> 1.0 -> 0.2, force following speed (the Helmholtz centre).
        let speed = 0.2 + 0.8 * (1.0 - (2.0 * x - 1.0).abs());
        rig.set_bow(speed, speed);
        heard.push(rig.tick().0);
        if (k + 1) % (RATE as usize / 2) == 0 {
            let now = rig.totals();
            let (stuck, releases) = regime(last, now);
            let w = &heard[k + 1 - RATE as usize / 2..=k];
            let pitch = peak_frequency(w, rig.f0);
            marks.push((
                speed,
                rms(w),
                stuck,
                releases,
                1200.0 * (pitch / rig.f0).log2(),
            ));
            last = now;
        }
    }
    println!(
        "{:>6} {:>10} {:>7} {:>10} {:>7}",
        "speed", "heard", "stuck", "releases/s", "cents"
    );
    for (speed, level, stuck, releases, cents) in &marks {
        println!("{speed:>6.2} {level:>10.3e} {stuck:>7.3} {releases:>10.1} {cents:>7.1}");
    }
    let levels: Vec<f64> = marks.iter().map(|m| m.1).collect();
    let peak = levels.iter().cloned().fold(0.0, f64::max);
    let peak_at = levels.iter().position(|&l| l == peak).unwrap();
    assert!(
        (2..=5).contains(&peak_at),
        "loudest near the middle (window {peak_at})"
    );
    assert!(
        levels[0] < 0.5 * peak && levels[7] < 0.5 * peak,
        "soft at both ends"
    );
    for w in 1..=peak_at {
        assert!(levels[w] > 0.9 * levels[w - 1], "rising: {levels:?}");
    }
    for w in peak_at + 1..8 {
        assert!(levels[w] < 1.1 * levels[w - 1], "falling: {levels:?}");
    }
    // The first window is the attack (multiple slipping until the Helmholtz
    // regime locks, 0.25-0.7 s); below speed 0.4 the period carries
    // secondary slips (declared in the spec's limits).
    for (w, (speed, _, stuck, releases, cents)) in marks.iter().enumerate() {
        assert!(
            (0.55..0.95).contains(stuck),
            "stick-slip through the swell at speed {speed}: {stuck}"
        );
        if w > 0 {
            let tolerance = if *speed >= 0.4 { 0.03 } else { 0.2 };
            assert!(
                (releases / rig.f0 - 1.0).abs() < tolerance,
                "at speed {speed}: {releases}/s"
            );
        }
        assert!(cents.abs() < 8.0, "at speed {speed}: {cents} cents");
    }
    let r = rig.world.receipt();
    println!(
        "swell books: source {:.4e} loss {:.4e} radiation {:.4e} residual cumulative {:.2e} max frame {:.2e}",
        r.source_work, r.internal_loss, r.radiation, r.cumulative_closure_residual, r.maximum_frame_residual
    );
    assert!(r.cumulative_closure_residual.abs() < 1.0e-15);
}

/// Schelleng's region, measured: where on the (speed, force) plane this bow
/// gives one release per period. Printed, and the declared centre asserted.
#[test]
fn the_speaking_region_is_declared() {
    println!(
        "{:>6} {:>6} | {:>10} {:>7} {:>10} {:>7}",
        "speed", "force", "heard", "stuck", "releases/s", "cents"
    );
    let mut centre_speaks = true;
    for &speed in &[0.2, 0.5, 1.0] {
        for &ratio in &[0.3, 1.0, 3.0] {
            let force = speed * ratio;
            let mut rig = build_rig(true);
            let heard = rig.bow_for(speed, force, 1.5);
            let t1 = rig.totals();
            let held = rig.bow_for(speed, force, 0.5);
            let (stuck, releases) = regime(t1, rig.totals());
            let cents = 1200.0 * (peak_frequency(&held, rig.f0) / rig.f0).log2();
            println!(
                "{speed:>6.2} {force:>6.2} | {:>10.3e} {stuck:>7.3} {releases:>10.1} {cents:>7.1}",
                rms(window(&heard, 1.0, 1.5))
            );
            if ratio == 1.0 {
                // One release per period from speed 0.4 up; secondary slips
                // ride the period below that (the spec's limits).
                let tolerance = if speed >= 0.4 { 0.03 } else { 0.25 };
                centre_speaks &= (releases / rig.f0 - 1.0).abs() < tolerance
                    && cents.abs() < 6.0
                    && (0.8..0.9).contains(&stuck);
            }
        }
    }
    assert!(centre_speaks, "force = speed speaks at every speed");
}

#[test]
#[ignore = "calibration: the low-speed end of the region, finer"]
fn sweep_low_speed() {
    println!(
        "{:>6} {:>6} {:>6} | {:>10} {:>7} {:>10} {:>7}",
        "speed", "force", "ratio", "heard", "stuck", "releases/s", "cents"
    );
    for &speed in &[0.1, 0.2, 0.3, 0.4, 0.7] {
        for &ratio in &[0.15, 0.3, 0.5, 0.7, 1.0, 1.5] {
            let force = speed * ratio;
            let mut rig = build_rig(true);
            let heard = rig.bow_for(speed, force, 1.5);
            let t1 = rig.totals();
            let held = rig.bow_for(speed, force, 0.5);
            let (stuck, releases) = regime(t1, rig.totals());
            let cents = 1200.0 * (peak_frequency(&held, rig.f0) / rig.f0).log2();
            println!(
                "{speed:>6.2} {force:>6.3} {ratio:>6.2} | {:>10.3e} {stuck:>7.3} {releases:>10.1} {cents:>7.1}",
                rms(window(&heard, 1.0, 1.5))
            );
        }
    }
}

#[test]
#[ignore = "calibration: the attack, with the hair landing and the arm starting"]
fn sweep_attack() {
    // force lands over `land` s; speed rises from 0 over `accel` s; mf.
    println!(
        "{:>6} {:>6} | {:>12} {:>12} {:>12} {:>10}",
        "land", "accel", "rel 0-0.25", "rel .25-.5", "rel .5-1", "settle s"
    );
    for &land in &[0.0, 0.01, 0.05] {
        for &accel in &[0.0, 0.02, 0.05, 0.1, 0.2] {
            let mut rig = build_rig(true);
            let mut marks = Vec::new();
            let mut last = rig.totals();
            let mut settled = None;
            for k in 0..(1.0 * RATE) as usize {
                let t = k as f64 / RATE;
                let f = if land > 0.0 { (t / land).min(1.0) } else { 1.0 };
                let v = if accel > 0.0 {
                    (t / accel).min(1.0)
                } else {
                    1.0
                };
                rig.set_bow(0.5 * v, 0.5 * f);
                rig.tick();
                if (k + 1) % (RATE as usize / 20) == 0 {
                    let now = rig.totals();
                    let (_, releases) = regime(last, now);
                    if settled.is_none() && (releases / rig.f0 - 1.0).abs() < 0.03 {
                        settled = Some(t);
                    }
                    marks.push(releases);
                    last = now;
                }
            }
            let mean = |a: usize, b: usize| marks[a..b].iter().sum::<f64>() / (b - a) as f64;
            println!(
                "{land:>6.2} {accel:>6.2} | {:>12.0} {:>12.0} {:>12.0} {:>10}",
                mean(0, 5),
                mean(5, 10),
                mean(10, 20),
                settled.map_or("never".to_owned(), |t| format!("{t:.2}"))
            );
        }
    }
}
