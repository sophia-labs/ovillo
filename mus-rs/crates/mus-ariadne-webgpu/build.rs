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
    compiler.build_artifact(
        &"package::world_present"
            .parse()
            .expect("valid World-present WESL module path"),
        "ariadne_world_present",
    );
}
