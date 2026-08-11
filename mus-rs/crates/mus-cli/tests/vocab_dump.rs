//! `mus vocab` dump contract: every registered core name is typed, the
//! digest matches the crate's own, and the layering follows ruling R2.

use std::process::Command;

#[test]
fn vocab_dump_covers_the_registry() {
    let out = Command::new(env!("CARGO_BIN_EXE_mus"))
        .arg("vocab")
        .output()
        .expect("run mus vocab");
    assert!(out.status.success());
    let dump: serde_json::Value = serde_json::from_slice(&out.stdout).expect("dump parses");

    assert_eq!(dump["schema"], serde_json::json!("mus.vocab.v1"));
    assert_eq!(
        dump["digest"].as_str().expect("digest"),
        mus_vocab::vocab_digest()
    );

    let params = dump["params"].as_array().expect("params array");
    let names: Vec<&str> = params
        .iter()
        .map(|p| p["name"].as_str().expect("name"))
        .collect();

    // Every core param and quote param is typed, exactly once.
    for core in mus_vocab::CORE_PARAMS
        .iter()
        .chain(mus_vocab::CORE_QUOTE_PARAMS)
    {
        assert_eq!(
            names.iter().filter(|n| n == &core).count(),
            1,
            "core param {core:?} typed exactly once"
        );
    }
    // Every flag is documented.
    let flags = dump["flags"].as_array().expect("flags");
    for flag in mus_vocab::CORE_FLAGS {
        assert!(
            flags.iter().any(|f| f["name"] == serde_json::json!(flag)),
            "flag {flag:?} documented"
        );
    }

    // Layering per R2: core params say core, renderer extensions say
    // extension, and the dump's own layer field agrees with
    // classify_param for every entry it types.
    for p in params {
        let name = p["name"].as_str().unwrap();
        let expected = match mus_vocab::classify_param(name) {
            mus_vocab::ParamLayer::Core => "core",
            mus_vocab::ParamLayer::Extension => "extension",
        };
        assert_eq!(
            p["layer"].as_str().unwrap(),
            expected,
            "layer agrees with classify_param for {name:?}"
        );
    }

    // Spot-check control typing the Stack depends on.
    let by_name = |n: &str| params.iter().find(|p| p["name"] == serde_json::json!(n));
    assert_eq!(
        by_name("gharm").unwrap()["control"]["kind"],
        serde_json::json!("interval-stack")
    );
    assert_eq!(
        by_name("lpf").unwrap()["control"]["sweep"],
        serde_json::json!(true)
    );
    assert_eq!(
        by_name("glayer").unwrap()["control"]["values"],
        serde_json::json!(["resolved", "octave"])
    );
    assert_eq!(
        by_name("str").unwrap()["control"]["literals"],
        serde_json::json!(["fit"])
    );

    // Ariadne registry: the weave voice is in the synth enum, and the four
    // control shapes the handoff calls out dump with honest types — a
    // frequency, a signed ratio, a count, and the spectral-dimension
    // exponent's full range.
    assert!(
        by_name("synth").unwrap()["control"]["values"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("weave")),
        "weave is a registered voice"
    );
    assert_eq!(
        by_name("orbit").unwrap()["control"]["kind"],
        serde_json::json!("hz")
    );
    assert_eq!(
        by_name("chirality").unwrap()["control"]["min"],
        serde_json::json!(-1.0),
        "chirality is a signed ratio"
    );
    assert_eq!(
        by_name("courses").unwrap()["control"]["kind"],
        serde_json::json!("count")
    );
    assert_eq!(
        by_name("dimension").unwrap()["control"]["max"],
        serde_json::json!(3.0)
    );

    // PW10 §A: bodyp is a typed enum with all five profiles, guitar default.
    assert_eq!(
        by_name("bodyp").unwrap()["control"]["values"],
        serde_json::json!(["guitar", "violin", "cello", "bass", "mandolin"])
    );
    assert_eq!(
        by_name("bodyp").unwrap()["control"]["default"],
        serde_json::json!("guitar")
    );
    assert_eq!(
        by_name("breath").unwrap()["control"],
        serde_json::json!({"kind": "ratio", "min": 0.0, "max": 1.0, "default": 0.0}),
        "breath is a 0..1 ratio, defaulting to no breath"
    );

    // PW10 §B: the bow is two ratios, both block-automatable (a crescendo
    // is a `bow` trajectory), and `bow` defaults to its hard zero limit.
    assert_eq!(
        by_name("bow").unwrap()["control"],
        serde_json::json!({"kind": "ratio", "min": 0.0, "max": 1.0, "default": 0.0}),
        "bow is a 0..1 ratio defaulting to no bow"
    );
    assert_eq!(
        by_name("bow_speed").unwrap()["control"],
        serde_json::json!({"kind": "ratio", "min": 0.0, "max": 1.0, "default": 0.5}),
        "bow_speed is a 0..1 ratio defaulting to mid travel"
    );
    for bowing in ["bow", "bow_speed"] {
        assert_eq!(
            by_name(bowing).unwrap()["automation"],
            serde_json::json!("block"),
            "{bowing} is retargetable mid-note"
        );
    }

    // Ariadne Phase 0 §A: `retune` is a two-value enum defaulting to the
    // historical render, and it is STATIC — the retuning policy is fixed
    // when the course is built, so there is no ControlTarget for it and a
    // trajectory cannot change it mid-note.
    assert_eq!(
        by_name("retune").unwrap()["control"],
        serde_json::json!({
            "kind": "enum",
            "values": ["legacy", "neutral_filters"],
            "default": "legacy"
        }),
        "retune is a legacy-defaulting enum"
    );
    assert_eq!(
        by_name("retune").unwrap()["automation"],
        serde_json::json!("static"),
        "retune is construction-time, not block-automatable"
    );
    assert_eq!(
        by_name("retune").unwrap()["layer"],
        serde_json::json!("extension")
    );
}
