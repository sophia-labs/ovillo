use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/pluck.rs");
    println!("cargo:rerun-if-changed=src/pluck_rearticulation.rs");

    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let source = fs::read_to_string(manifest.join("src/pluck.rs"))
        .expect("read parity-pinned pluck source");
    let extension = fs::read_to_string(manifest.join("src/pluck_rearticulation.rs"))
        .expect("read rearticulation extension");

    // `pluck.rs` is normally a module file and begins with inner module docs
    // (`//!`). Once included inside the inline module in `lib.rs`, those inner
    // docs are no longer accepted by rustc. Convert comments only; every code
    // token in the parity-pinned implementation remains source-identical.
    let mut generated = String::with_capacity(source.len() + extension.len() + 256);
    for line in source.lines() {
        if let Some(rest) = line.strip_prefix("//!") {
            generated.push_str("//");
            generated.push_str(rest);
        } else {
            generated.push_str(line);
        }
        generated.push('\n');
    }

    // Appending the extension as a CHILD module gives it the privacy needed to
    // compile an attack schedule against the existing voice without widening
    // the whole 117 kB implementation's internal fields to `pub(crate)`.
    generated.push_str("\nmod rearticulation {\n");
    generated.push_str("    use super::*;\n\n");
    generated.push_str(&extension);
    if !extension.ends_with('\n') {
        generated.push('\n');
    }
    generated.push_str("}\n\n");
    generated.push_str(
        "pub use rearticulation::{\n    AttackFrame, AttackSchedule, AttackScheduleReceipt,\n    RearticulatedStringVoice, RearticulationError,\n};\n",
    );

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("build output directory"))
        .join("pluck_generated.rs");
    fs::write(output, generated).expect("write generated pluck module");
}
