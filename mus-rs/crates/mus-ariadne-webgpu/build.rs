use wesl::Wesl;

fn main() {
    println!("cargo:rerun-if-changed=shaders");

    let compiler = Wesl::new("shaders");
    compiler.build_artifact(
        &"package::impulse_raster"
            .parse()
            .expect("valid impulse-raster WESL module path"),
        "ariadne_impulse_raster",
    );
    compiler.build_artifact(
        &"package::wave_sheet"
            .parse()
            .expect("valid wave-sheet WESL module path"),
        "ariadne_wave_sheet",
    );
}
