# GuitarJam listening fixtures

This directory defines a small, reproducible listening subset of
[Julian-br/GuitarJam](https://huggingface.co/datasets/Julian-br/GuitarJam), a
CC0-1.0 collection of clean monophonic electric-guitar DI recordings.

The WAV files are intentionally not vendored here. Fetch the five pinned test
clips into the repository's local `artifacts/` directory with:

```sh
./mus-rs/fixtures/guitarjam/fetch.sh
```

The fetcher resolves the dataset at the immutable revision recorded in
`manifest.json` and verifies every SHA-256 digest before publishing a file.
These are source recordings, not golden numerical fixtures: deterministic DSP
tests should continue to use synthetic signals, while these clips exercise
real pickup dynamics, picking transients, fret noise, and instrument bandwidth.

Render the shortest clip through the powered radiator with:

```sh
cargo run --manifest-path mus-rs/Cargo.toml -p mus-dsp --release \
  --example powered_radiator_audition -- \
  artifacts/guitarjam/source/050.wav \
  artifacts/guitarjam/guitarjam-050-ariadne-v0.wav
```

The montage order is dry, Costa Clara + Plate, Edge + Spring, Grind +
Sympathique, then Wall + Shimmer Lite, with one second of silence between
sections. Separate stems and a JSON receipt are written beside it. One shared
monitor gain is applied only when writing; stems are never independently
normalized, and the processor receives the original DI at its native 44.1 kHz.
Ariadne is a powered material/body rather than an electronic amp schematic,
but its three receipted contact stages intentionally span clean response
through recognizable electric-guitar distortion.
