use super::*;

const GUITAR: &str = "# score: one bar\n# tempo: 240    time: 4/4\n# master: rms=-16\n# instruments:\n\
#   gt   = steel-string guitar (treble, -12, anatomy=steel-string-v0, tuning=E2 A2 D3 G3 B3 E4, capo=0)\n\
#   gt.p = thumb, thumbpick (treble, pos=0.10, pick=0.85, pm=0.45)\n\
#   gt.v = fingers (treble, pos=0.20, pick=0.45)\n";

fn score(bar: &str) -> String {
    format!("{GUITAR}{bar}\n")
}

fn codes(report: &CheckReport) -> Vec<&str> {
    report.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

fn utf16_slice(text: &str, span: Span) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    String::from_utf16(&units[span.from..span.to]).unwrap()
}

#[test]
fn a_clean_guitar_checks_clean() {
    let text = score("bar 1: gt.p=A2q@5:0 E3q@4:2 A2q@5:0 E3q@4:2  gt.v=A3e@3:2 B3e@3:4[ho] C4e@2:1 D4e@2:3[ho] C4e@2:1[po] B3e@2:0[po] A3e@3:2 B3e@3:4[sl]");
    let report = check(&text);
    assert!(report.clean, "{:#?}", report.diagnostics);
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    assert_eq!(report.worlds.len(), 1);
    assert_eq!(report.worlds[0].anatomy, "steel-string-v0");
    assert_eq!(report.score["bars"], 1);
}

#[test]
fn a_wrong_fret_is_anchored_to_its_token() {
    let text = score("bar 1: gt.p=A2h@5:0 E3h@4:2  gt.v=C4e@2:3 Re Rh.");
    let report = check(&text);
    assert!(!report.clean);
    assert_eq!(codes(&report), vec!["tab-position-disagrees"]);
    let d = &report.diagnostics[0];
    assert_eq!(d.anchor["bar"], 1);
    assert_eq!(d.anchor["track"], "gt.v");
    let span = d.span.unwrap();
    assert_eq!(utf16_slice(&text, span), "C4e@2:3");
    assert_eq!(span.line, text.lines().count());
    assert!(matches!(Render::begin(&text), Err(Refusal::Check(_))), "a score that does not check clean is not rendered");
}

#[test]
fn spans_survive_dynamics_patterns_and_repeated_tokens() {
    let text = score("bar 1: gt.p=(A2e@5:0 E3e@4:2 ×4)  gt.v={p} E4e@1:0 E4e@1:0 E4e@1:0{f} C4e@2:3 Rh");
    let report = check(&text);
    assert_eq!(codes(&report), vec!["tab-position-disagrees"]);
    let span = report.diagnostics[0].span.unwrap();
    assert_eq!(utf16_slice(&text, span), "C4e@2:3");
    // Non-ASCII before the token (the × of the pattern) is counted in UTF-16 units.
    assert_eq!(&text[..text.find("C4e@2:3").unwrap()].encode_utf16().count(), &span.from);

    let twice = score("bar 1: gt.p=Rw  gt.v=E4q@1:0 C4q@2:3 E4q@1:0 C4q@2:3");
    let report = check(&twice);
    let spans: Vec<Span> = report.diagnostics.iter().map(|d| d.span.unwrap()).collect();
    assert_eq!(spans.len(), 2);
    assert_ne!(spans[0].from, spans[1].from, "the second C4q@2:3 points at the second one");
    assert_eq!(utf16_slice(&twice, spans[1]), "C4q@2:3");
}

#[test]
fn anything_that_is_not_an_ariadne_world_is_refused_and_never_rendered() {
    let cases = [
        ("#   s = synth (perc, synth=sine)\n", "a synth (synth=sine)"),
        ("#   w = weave (treble, synth=weave)\n", "the weave voice (synth=weave)"),
        ("#   v = bird (treble, voice=wren)\n", "a sample-pack voice (voice=wren)"),
        ("#   k = kick (perc, sample=kick.wav)\n", "a sample (sample=kick.wav)"),
        ("#   x = voice (treble)\n", "a track with no anatomy="),
    ];
    for (line, what) in cases {
        let text = format!("{GUITAR}{line}bar 1: gt.p=A2w@5:0 gt.v=Rw\n");
        let report = check(&text);
        assert!(!report.clean, "{line}");
        let refusal = report.diagnostics.iter().find(|d| d.code == "not-ariadne").unwrap_or_else(|| panic!("{line}: {:#?}", report.diagnostics));
        assert_eq!(refusal.severity, "error");
        assert!(refusal.message.contains(what), "{}", refusal.message);
        assert!(refusal.message.contains("Ariadne Worlds only"), "{}", refusal.message);
        let abbrev = line.trim_start_matches(['#', ' ']).split(' ').next().unwrap();
        assert_eq!(utf16_slice(&text, refusal.span.unwrap()), abbrev);
        assert!(matches!(Render::begin(&text), Err(Refusal::Check(_))), "{line} must never render");
    }

    let pack = format!("# pack: birds/pack.json\n{GUITAR}bar 1: gt.p=A2w@5:0 gt.v=Rw\n");
    let report = check(&pack);
    let refusal = report.diagnostics.iter().find(|d| d.code == "not-ariadne").unwrap();
    assert_eq!(refusal.anchor["kind"], "header");
    assert_eq!(utf16_slice(&pack, refusal.span.unwrap()), "# pack: birds/pack.json");

    let hand = score("bar 1: gt.p=A2w@5:0 gt.v=Rw").replace("pos=0.20, pick=0.45", "pos=0.20, synth=saw");
    let report = check(&hand);
    assert!(report.diagnostics.iter().any(|d| d.code == "not-ariadne" && d.message.contains("synth=saw")), "{:#?}", report.diagnostics);

    let file = score("bar 1: gt.p=A2w@5:0 gt.v=Rw").replace("anatomy=steel-string-v0", "anatomy=my-guitar.plan.json");
    let report = check(&file);
    assert_eq!(codes(&report), vec!["anatomy-unavailable"]);
}

#[test]
fn a_staged_render_in_odd_blocks_is_the_engine_render() {
    let text = score("bar 1: gt.p=A2q@5:0 E3q@4:2 A2q@5:0 E3q@4:2  gt.v=A3e@3:2 B3e@3:4[ho] C4e@2:1 D4e@2:3[ho] C4e@2:1[po] B3e@2:0[po] A3e@3:2 B3e@3:4[sl]");
    let mut render = Render::begin(&text).unwrap();
    let total = render.total();
    assert!(total > 0);
    let mut steps = 0;
    while !render.step(12_345).unwrap() {
        steps += 1;
        assert!(render.done() <= total);
    }
    assert!(steps > 3);
    assert_eq!(render.done(), total);
    let staged = render.finish(1.5).unwrap();

    let direct = mus_engine::render(&text, Path::new(".")).unwrap();
    assert_eq!(staged.outcome.audio, direct.audio, "same samples, bit for bit");
    let wav = wav_bytes(&direct.audio).unwrap();
    assert_eq!(staged.wav, wav);
    assert_eq!(staged.receipt.render_digest, format!("sha256:{}", sha256_bytes(&wav)));
    let receipt: Value = serde_json::from_str(&staged.receipt_json()).unwrap();
    assert_eq!(receipt["schema"], "mus.audio.render-receipt.v1");
    assert_eq!(receipt["wallSeconds"], 1.5);
    assert_eq!(receipt["badTokens"], json!([]));
    let world = &receipt["ariadneWorlds"][0];
    assert_eq!(world["hammerOns"], 2);
    assert!(world["cumulativeClosureResidual"].as_f64().unwrap().abs() < 1e-12);
    assert_eq!(*world, serde_json::to_value(&direct.stats.ariadne_worlds[0]).unwrap());
    let energy: Value = serde_json::from_str(&staged.energy_json()).unwrap();
    assert_eq!(energy["schema"], "mus.ariadne.energy-trace.v1");
    assert_eq!(energy["worlds"], serde_json::to_value(&direct.stats.ariadne_energy).unwrap());
}

/// The exports a Worker calls, called the same way (on this test's own thread-local state).
#[test]
fn the_c_abi_checks_renders_and_hands_back_audio() {
    fn put(text: &str) -> u32 {
        let ptr = mus_input_ptr(text.len() as u32);
        unsafe { std::ptr::copy_nonoverlapping(text.as_ptr(), ptr, text.len()) };
        text.len() as u32
    }
    fn text(len: u32) -> Value {
        let bytes = unsafe { std::slice::from_raw_parts(mus_text_ptr(), len as usize) };
        serde_json::from_slice(bytes).unwrap()
    }
    let info = text(mus_info());
    assert_eq!(info["sampleRate"], 48_000);
    assert_eq!(info["ariadneOnly"], true);

    let synth = format!("{GUITAR}#   s = synth (perc, synth=sine)\nbar 1: gt.p=A2w@5:0 gt.v=Rw s=A4q\n");
    let len = put(&synth);
    assert_eq!(text(mus_check(len))["clean"], false);
    let len = put(&synth);
    assert_eq!(mus_render_begin(len), 1, "refused");
    let refused = text(mus_text_len());
    assert!(refused["diagnostics"].as_array().unwrap().iter().any(|d| d["code"] == "not-ariadne"));
    assert_eq!(mus_render_frames(), 0);

    let good = score("bar 1: gt.p=A2w@5:0  gt.v=<A3@3:2 C4@2:1 E4@1:0>w");
    let len = put(&good);
    assert_eq!(mus_render_begin(len), 0);
    let total = mus_render_total();
    assert!(total > 0.0);
    let mut state = 0;
    while state == 0 {
        state = mus_render_step(48_000);
        assert!(mus_render_done() <= total);
    }
    assert_eq!(state, 1);
    assert_eq!(mus_render_finish(0.25), 0);
    let receipt = text(mus_text_len());
    assert_eq!(receipt["schema"], "mus.audio.render-receipt.v1");
    assert_eq!(receipt["wallSeconds"], 0.25);
    assert_eq!(receipt["ariadneWorlds"][0]["plucks"], 4);
    let frames = mus_render_frames() as usize;
    assert_eq!(frames, ((1.0 + 4.0) * 48_000.0) as usize, "a 240 bpm bar and the 4 s tail");
    let samples = unsafe { std::slice::from_raw_parts(mus_render_samples_ptr(), frames * 2) };
    assert!(samples.iter().any(|v| v.abs() > 1e-3), "the World sounds");
    let wav = unsafe { std::slice::from_raw_parts(mus_render_wav_ptr(), mus_render_wav_len() as usize) };
    assert_eq!(&wav[..4], b"RIFF");
    let energy = text(mus_render_energy());
    assert_eq!(energy["worlds"][0]["instrument"], "gt");
    mus_render_reset();
    assert_eq!(mus_render_frames(), 0);
}
