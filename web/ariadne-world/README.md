# Ariadne World WebGPU observer

This is the canonical browser face for `ariadne-world-audiovisual-bundle-v1`.
It has no Terminal-specific trace, JavaScript physics kernel, page-specific
channel table, audio analyser, or compatibility decoder.

```text
one immutable physical World run
        |
        +-- audio.wav
        +-- typed ObservationTrace
        +-- content-addressed OpticalConstitutionPlan
                       |
                       v
             Rust/WASM wgpu runtime
                       |
             fixed-rate observer field
                       |
                 presentation only
```

The physical World is never run on the GPU in this application. WebGPU receives
non-loading observations through `mus-observation` and
`mus-ariadne-webgpu-bridge`. The page verifies every file hash; Rust validates
the typed run identity, trace identity, World/schema binding, and optical-plan
binding before allocating GPU state. Seeking resets and deterministically
replays only the optical observer; it does not alter the physical run.

## Build

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
./build.sh
```

## Generate a bundle

```bash
cargo run --manifest-path ../../mus-rs/Cargo.toml --release \
  -p ariadne-world-export -- INPUT.wav demo
```

## Serve

```bash
./serve.sh
```

Open `http://127.0.0.1:8080`. The page verifies every file digest before
constructing the viewer. Local bundles can be opened by selecting
`manifest.json` and every file named in its `files` object.

## Runtime laws

- the absolute Ariadne material tick is authoritative;
- optical evolution uses a fixed integer-rational clock;
- presentation grouping cannot change the number or identity of optical steps;
- long seeks replay in bounded batches;
- GPU failure cannot block or rewrite the recorded physical World;
- no observer action feeds work back into Ariadne;
- the first observation after reset is preserved rather than consumed by reset.
