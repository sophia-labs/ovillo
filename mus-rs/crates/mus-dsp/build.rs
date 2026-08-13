use std::env;
use std::fs;
use std::path::PathBuf;

fn module_source(source: &str) -> String {
    // A module file begins with inner docs (`//!`). Once included inside an
    // inline module, rustc expects ordinary comments instead. Convert comments
    // only; every code token in the committed implementation remains intact.
    let mut generated = String::with_capacity(source.len() + 256);
    for line in source.lines() {
        if let Some(rest) = line.strip_prefix("//!") {
            generated.push_str("//");
            generated.push_str(rest);
        } else {
            generated.push_str(line);
        }
        generated.push('\n');
    }
    generated
}

fn replace_once(source: &mut String, old: &str, new: &str, label: &str) {
    let count = source.matches(old).count();
    assert_eq!(
        count, 1,
        "{label}: expected one guarded replacement, found {count}"
    );
    *source = source.replacen(old, new, 1);
}

fn main() {
    println!("cargo:rerun-if-changed=src/pluck.rs");
    println!("cargo:rerun-if-changed=src/pluck_rearticulation.rs");
    println!("cargo:rerun-if-changed=src/pluck_attack_work.rs");
    println!("cargo:rerun-if-changed=src/trajectory.rs");

    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let output_dir = PathBuf::from(env::var_os("OUT_DIR").expect("build output directory"));

    let pluck_source =
        fs::read_to_string(manifest.join("src/pluck.rs")).expect("read parity-pinned pluck source");
    let mut extension = fs::read_to_string(manifest.join("src/pluck_rearticulation.rs"))
        .expect("read rearticulation extension");
    replace_once(
        &mut extension,
        "use serde::Serialize;",
        "use serde::{Deserialize, Serialize};",
        "rearticulation serde imports",
    );
    replace_once(
        &mut extension,
        "#[derive(Debug, Clone, Copy, PartialEq, Serialize)]",
        "#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]",
        "attack frame deserialize",
    );
    replace_once(
        &mut extension,
        "#[derive(Debug, Clone, PartialEq, Serialize)]",
        "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]",
        "attack schedule receipt deserialize",
    );
    let camel_case_count = extension
        .matches("#[serde(rename_all = \"camelCase\")]\n")
        .count();
    assert_eq!(
        camel_case_count, 2,
        "rearticulation receipt casing: expected two generated attributes"
    );
    extension = extension.replace("#[serde(rename_all = \"camelCase\")]\n", "");

    let work_extension = fs::read_to_string(manifest.join("src/pluck_attack_work.rs"))
        .expect("read attack-work extension");
    let mut pluck = module_source(&pluck_source);

    // Appending both extensions inside one CHILD module gives them the privacy
    // needed to compile against the existing voice without widening the 117 kB
    // implementation's internal fields to `pub(crate)`.
    pluck.push_str("\nmod rearticulation {\n");
    pluck.push_str("    use super::*;\n\n");
    pluck.push_str(&extension);
    if !extension.ends_with('\n') {
        pluck.push('\n');
    }
    pluck.push('\n');
    pluck.push_str(&work_extension);
    if !work_extension.ends_with('\n') {
        pluck.push('\n');
    }
    pluck.push_str("}\n\n");
    pluck.push_str(
        "pub use rearticulation::{\n    AttackFrame, AttackSchedule, AttackScheduleReceipt, AttackWorkReceipt,\n    RearticulatedStringVoice, RearticulationError, RearticulationReceipt,\n    WorkReceiptedRender, WorkReceiptedStringVoice,\n};\n",
    );
    fs::write(output_dir.join("pluck_generated.rs"), pluck).expect("write generated pluck module");

    // The body-neutral trajectory module keeps its committed source stable.
    // This guarded stitch adds the optional body-runtime receipt at the one
    // adapter boundary where the two values meet, without making mus-trajectory
    // depend on mus-dsp.
    let trajectory_source = fs::read_to_string(manifest.join("src/trajectory.rs"))
        .expect("read string trajectory adapter");
    let mut trajectory = module_source(&trajectory_source);
    replace_once(
        &mut trajectory,
        "use crate::pluck::{ControlFrame, ControlTarget, StringNetworkVoice, WeaveTrace};",
        "use crate::pluck::{\n    ControlFrame, ControlTarget, RearticulationReceipt, StringNetworkVoice, WeaveTrace,\n};",
        "trajectory rearticulation import",
    );
    replace_once(
        &mut trajectory,
        "    pub strict_transport: bool,\n}",
        "    pub strict_transport: bool,\n    #[serde(default, skip_serializing_if = \"Option::is_none\")]\n    pub rearticulation: Option<RearticulationReceipt>,\n}",
        "trajectory receipt field",
    );
    replace_once(
        &mut trajectory,
        "            strict_transport,\n        },",
        "            strict_transport,\n            rearticulation: None,\n        },",
        "trajectory receipt constructor",
    );
    fs::write(output_dir.join("trajectory_generated.rs"), trajectory)
        .expect("write generated trajectory module");
}
