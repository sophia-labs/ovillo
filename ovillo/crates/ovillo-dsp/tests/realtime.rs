//! The audio path allocates and frees nothing of its own: every composition,
//! scores swapped in mid-stream, the master stage, key-press events and
//! strums, all through `Rig::process` with a counting allocator watching this
//! thread. Replaced scores come back on the garbage ring and are freed here.
//!
//! The Ariadne World itself (`mus-ariadne`, shared with the plug-in) does
//! allocate: today twice a sample, a `String` clone of the body id in
//! `World::set_body_configuration_index` and the `Vec` that a body's
//! `radiate` returns (`shared_world/build.rs`, `shared_world/process.rs`).
//! So the rig is held to exactly Ariadne's count, measured here on the same
//! number of samples: any allocation the rig adds breaks the equality. When
//! Ariadne stops allocating, `ariadne_alone` reads 0 and so must the rig.

use ovillo_dsp::ariadne::AriadneControls;
use ovillo_dsp::player::{Event, Pattern};
use ovillo_dsp::rig::{master_gain, Command, Rig};
use ovillo_dsp::score;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Counting;

thread_local! {
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _ = FREES.try_with(|c| c.set(c.get() + 1));
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static A: Counting = Counting;

fn counts() -> (usize, usize) {
    (ALLOCS.with(Cell::get), FREES.with(Cell::get))
}

/// Allocations and frees per sample of the bare Ariadne engine.
fn ariadne_alone(samples: usize) -> (usize, usize) {
    use ovillo_dsp::ariadne::AriadneEngine;
    let mut e = AriadneEngine::new(48_000.0).unwrap();
    let c = AriadneControls::default();
    let before = counts();
    for i in 0..samples {
        e.process_sample(if i % 97 == 0 { 0.1 } else { 0.0 }, 0.0, c);
    }
    let after = counts();
    (after.0 - before.0, after.1 - before.1)
}

#[test]
fn the_rig_allocates_nothing_beyond_what_ariadne_does() {
    let sr = 48_000.0;
    let (mut rig, mut ends) = Rig::new(sr).unwrap();
    let scores: Vec<_> = (0..3)
        .map(|k| {
            let text = format!(
                r#"{{"tempo": 140, "loop": {}, "humanize": 0.5, "events": [
                    {{"type": "strum", "beat": 0, "chord": "G", "spread_ms": 25}},
                    {{"type": "pluck", "beat": 1, "string": {k}, "fret": 5}},
                    {{"type": "bend", "beat": 1.5, "string": {k}, "semitones": 1}},
                    {{"type": "vibrato", "beat": 2, "string": {k}, "depth": 0.2}},
                    {{"type": "mute", "beat": 3}}
                ]}}"#,
                k == 1
            );
            Box::new(score::parse_str(&text, 92.0).unwrap())
        })
        .collect();
    const BLOCK: usize = 256;
    let mut buf = vec![0.0f32; 2 * BLOCK];
    let mut seen = (0usize, 0usize);
    let mut samples = 0usize;
    let mut process = |rig: &mut Rig, blocks: usize, seen: &mut (usize, usize), samples: &mut usize| {
        for _ in 0..blocks {
            let before = counts();
            rig.process(&mut buf, 2);
            let after = counts();
            seen.0 += after.0 - before.0;
            seen.1 += after.1 - before.1;
            *samples += BLOCK;
        }
    };
    ends.commands.push(Command::Ariadne(AriadneControls::default())).unwrap();
    for p in Pattern::CYCLE {
        ends.commands.push(Command::Pattern(p)).unwrap();
        // Two seconds of each (the pieces' first bars plan their sections).
        process(&mut rig, 375, &mut seen, &mut samples);
        while ends.scope.pop().is_ok() {}
        while ends.events.pop().is_ok() {}
        while ends.telemetry.pop().is_ok() {}
    }
    ends.commands.push(Command::Event { event: Event::Pluck { string: 2, fret: 3, velocity: 0.8, palm: false }, delay_s: 0.0 }).unwrap();
    ends.commands.push(Command::Strum { frets: [0, 2, 2, 1, 0, 0], down: true, velocity: 0.7, spread_s: 0.03 }).unwrap();
    ends.commands.push(Command::Master { gain: master_gain(-20.0), mute: false }).unwrap();
    process(&mut rig, 100, &mut seen, &mut samples);
    for s in scores {
        ends.commands.push(Command::Score(s)).unwrap();
        process(&mut rig, 200, &mut seen, &mut samples);
        while ends.scope.pop().is_ok() {}
        while ends.events.pop().is_ok() {}
        while ends.telemetry.pop().is_ok() {}
    }
    ends.commands.push(Command::Master { gain: 1.0, mute: true }).unwrap();
    process(&mut rig, 100, &mut seen, &mut samples);

    let ariadne = ariadne_alone(samples);
    eprintln!("{samples} samples: rig {seen:?} allocations/frees, Ariadne alone {ariadne:?}");
    assert_eq!(seen, ariadne, "the rig allocates or frees beyond Ariadne's own");

    // The two replaced scores came back to be freed on this thread.
    let before = counts().1;
    let mut returned = 0;
    while let Ok(old) = ends.garbage.pop() {
        drop(old);
        returned += 1;
    }
    assert_eq!(returned, 2);
    assert!(counts().1 > before, "dropping them freed memory here");
}

/// The guitarist, guitar and score playback with no Ariadne at all: zero.
#[test]
fn the_guitarist_and_guitar_never_allocate() {
    use ovillo_dsp::guitar::Guitar;
    use ovillo_dsp::player::Player;
    let sr = 48_000.0;
    let mut g = Guitar::new(sr);
    let mut p = Player::new(sr);
    let sc = Box::new(score::parse_str(r#"{"loop": true, "humanize": 1, "events": [{"type": "strum", "beat": 0, "chord": "Am"}, {"type": "pluck", "beat": 2, "string": 4, "fret": 8}]}"#, 92.0).unwrap());
    let before = counts();
    for pat in Pattern::CYCLE {
        p.set_pattern(pat);
        for _ in 0..(sr as usize * 4) {
            p.tick(&mut g);
            g.tick();
        }
    }
    let old = p.load_score(sc);
    for _ in 0..(sr as usize * 4) {
        p.tick(&mut g);
        g.tick();
    }
    let after = counts();
    assert!(old.is_none());
    assert_eq!((after.0 - before.0, after.1 - before.1), (0, 0));
}
