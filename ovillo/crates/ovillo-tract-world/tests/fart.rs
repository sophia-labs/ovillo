//! `fart_v1` through `fart::Gut`: the reservoir is spent and paid back as
//! the World books it, a full reservoir stops digestion, and a tune played
//! from the measured tuning curve lands on its notes (the World's own F0).

use ovillo_tract_world::fart::{self, Gut, TuningCurve};

const SR: f64 = 48_000.0;

#[test]
fn notes_and_tunes_parse() {
    assert!((fart::note_hz("A4").unwrap() - 440.0).abs() < 1e-9);
    assert!((fart::note_hz("G2").unwrap() - 97.998_858_995_437_3).abs() < 1e-6);
    assert_eq!(fart::note_hz("Eb2"), fart::note_hz("D#2"));
    assert_eq!(fart::note_hz("98"), Some(98.0));
    assert!(fart::note_hz("H2").is_none());
    let notes = fart::parse_tune("G2:1 r:0.5 Bb2", 120.0).unwrap();
    assert_eq!(notes.len(), 3);
    assert_eq!(notes[1].hz, 0.0);
    assert!((notes[0].seconds - 0.5).abs() < 1e-12 && (notes[2].seconds - 0.5).abs() < 1e-12);
    assert!(fart::parse_tune("G2:x", 120.0).is_err());
    let triplets = fart::parse_tune("G2:1/3 G2:1/3 G2:1/3 G2:1+1/2", 60.0).unwrap();
    assert!((triplets.iter().map(|n| n.seconds).sum::<f64>() - 2.5).abs() < 1e-12);
    assert_eq!(fart::food("Beans").map(|f| f.name), Some("beans"));
    assert_eq!(fart::food("egg").map(|f| f.name), Some("egg"));
}

#[test]
fn a_brap_spends_the_reservoir_and_digestion_pays_it_back() {
    let mut gut = Gut::new(SR).unwrap();
    let audio = gut.release(&fart::style("brap").unwrap()).unwrap();
    assert!(audio.iter().all(|v| v.is_finite()));
    assert!(audio.iter().fold(0.0f32, |m, v| m.max(v.abs())) > 0.05, "audible");
    let spent = gut.gas_fraction();
    assert!(spent < 0.2, "the sag law ends it with the reservoir nearly spent: {spent}");
    for _ in 0..4 {
        gut.eat(fart::food("beans").unwrap());
    }
    let d = gut.digest(60.0).unwrap();
    assert!(d.stopped_full, "four beans overfill the reservoir");
    assert!(!gut.stomach().is_empty(), "the rest waits in the stomach");
    assert!(gut.gas_fraction() > 0.99);
    let r = gut.world().world().receipt();
    assert!((d.paid_j - r.source_work).abs() < 1e-12, "everything paid is the World's source work");
    assert!(r.maximum_frame_residual < 4.0e-15, "{}", r.maximum_frame_residual);
    assert!(r.cumulative_closure_residual.abs() < 1.0e-12, "{}", r.cumulative_closure_residual);
}

#[test]
fn a_tune_lands_on_its_long_notes() {
    let curve = TuningCurve::measure(SR).unwrap();
    let (lo, hi) = curve.range();
    assert!(lo < 60.0 && hi > 250.0, "range {lo}-{hi}");
    let mut gut = Gut::new(SR).unwrap();
    for _ in 0..4 {
        gut.eat(fart::food("beans").unwrap());
    }
    let notes = fart::parse_tune("Bb2:1 Db3:1 C3:1 A2:1 Bb2:2", 66.0).unwrap();
    let (audio, reports) = gut.play_tune(&notes, &curve, false, true).unwrap();
    assert!(audio.iter().all(|v| v.is_finite()));
    for r in &reports {
        let cents = r.cents.expect("every note sounds");
        assert!(cents.abs() < 30.0, "{} Hz landed {cents:+.0} cents off", r.target_hz);
        assert!(r.voiced_fraction > 0.95);
    }
    assert_eq!(gut.world().observers().safety_limited, 0, "the tune voice does not clip");
    let r = gut.world().world().receipt();
    assert!(r.maximum_frame_residual < 4.0e-15, "{}", r.maximum_frame_residual);
}

#[test]
fn a_cannon_fires_on_its_shots_and_the_house_feeds_a_long_part() {
    let mut gut = Gut::new(SR).unwrap();
    gut.house_w = 0.6;
    let shots = fart::parse_tune("G2:1 r:1 G2:1 r:1", 120.0).unwrap();
    let (audio, fired) = gut.play_cannon(&shots, true).unwrap();
    assert_eq!(fired, 2);
    let peak = audio.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    println!("cannon peak {peak:.3}, clipped {}", gut.world().observers().safety_limited);
    assert!(peak > 0.3, "a shot is loud: {peak}");
    let curve = TuningCurve::measure(SR).unwrap();
    let mut gut = Gut::new(SR).unwrap();
    gut.house_w = 0.6;
    let long = fart::parse_tune(&"G2:1 Bb2:1 D3:1 G2:1 ".repeat(8), 100.0).unwrap();
    let (_, reports) = gut.play_tune(&long, &curve, true, true).unwrap();
    let voiced = reports.iter().filter(|r| r.voiced_fraction > 0.9).count();
    assert_eq!(voiced, reports.len(), "fed by the house, a 19 s part never runs dry");
    assert!(gut.gas_fraction() > 0.5, "{}", gut.gas_fraction());
    let r = gut.world().world().receipt();
    assert!(r.maximum_frame_residual < 4.0e-15);
}
