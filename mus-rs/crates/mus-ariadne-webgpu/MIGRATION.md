# Ariadne Sea migration sequence

The current `web/ariadne-sea` implementation remains the visual and gameplay
reference. This crate is introduced without changing that renderer.

## M0 — library skeleton and first graph

This change:

- creates typed constitutive metadata and capability receipts;
- creates `ImpulseRaster2D`, `WaveSheet2D`, and `WaveSurface2D`;
- introduces WESL build artifacts;
- provides CPU reference laws and exact-zero tests;
- registers a performance comparison.

## M1 — headless equivalence harness

Create a small native harness that runs the prototype and library kernels from
identical event packets and compares:

- impulse field;
- wave height/velocity/foam;
- selected optical output;
- GPU timings and upload bytes.

## M2 — browser shadow mode

Run the library graph beside the current JavaScript graph without presenting it.
Capture hashes or small asynchronous reductions, not full synchronous readback.

## M3 — visual cutover

Replace only the sea field evolution after equivalence and performance gates
pass. Preserve the Rust/WASM boat dynamics, audio clock, voyage receipt, and
scientific boundary.

## M4 — derivative and phosphor primitives

Move normal/gradient calculation to field resolution and separate world-space
optical persistence from sensor-space phosphor memory.

Every migration step is behaviorally isolated. Performance work must not alter
the authoritative voyage dynamics or silently change Ariadne claim boundaries.
