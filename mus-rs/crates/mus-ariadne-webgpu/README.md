# `mus-ariadne-webgpu`

`mus-ariadne-webgpu` is the first reusable WebGPU substrate for Ariadne visual
worlds. It is deliberately **not** a general-purpose game engine. Its primitive
unit is a persistent field law with a declared Ariadne meaning:

- where the state lives (`world`, `observer`, `sensor`, or `presentation`);
- whether the pass transports, injects work, evolves constitution, dissipates,
  radiates, or observes without loading;
- when the pass runs;
- what exact-zero law it must satisfy;
- what performance and scientific receipts it emits.

The first slice extracts the two load-bearing operations behind Ariadne Sea:

1. `ImpulseRaster2D`: local event splats into a compact additive impulse field;
2. `WaveSheet2D`: a tiled, persistent wave/foam/optical field advanced from one
   accumulated impulse sample per cell.

This removes the prototype's `cells × events` inner loop and creates the seam
for later derivatives, Schlieren optics, director transport, phosphor banks,
active tiles, and indirect dispatch.

## Deliberate boundaries

- The crate owns no window or swap-chain policy.
- The crate accepts an existing `wgpu::Device`, `Queue`, and command encoder.
- The first primitives are non-loading visual/material support; promotion into
  authoritative Ariadne physics requires an explicit bridge and receipts.
- Shader source is authored as WESL and compiled to ordinary WGSL at build time.
- Native and browser builds use the same Rust API and generated WGSL.
- The current Ariadne Sea renderer remains the behavioral reference until a
  later migration commit proves visual and game equivalence.

See `ARCHITECTURE.md`, `BENCHMARK-PLAN.md`, and `ACKNOWLEDGEMENTS.md`.
