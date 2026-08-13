# Ariadne unification report

Ariadne now has one canonical implementation home: `mus-rs/crates/mus-ariadne`.

New instruments are `WorldSpec` configurations and typed trajectories of one persistent `World`. The world owns one field, one compliant boundary, one finite work reservoir, reusable active material, ordered transport, reciprocal delayed space, finite-aperture radiation, and one `WorkLedger`.

The frozen sample order is external work, boundary exchange, active material, ordered transport, conservative propagation, passive loss, reciprocal space, radiation, and ledger.

Every file-backed `ariadne_*.rs` implementation has been removed from `mus-dsp`. The plug-in now depends directly on `mus-ariadne`. The current product terminal remains under `compat::terminal` only to preserve sessions while its controls migrate onto the canonical world.

Construction may allocate, parse, normalize, and build delay storage. The sample path performs no allocation, locking, file I/O, FFT, graph search, or coordinate-name lookup. `RuntimeLayout` exposes persistent storage, `CostEstimate` exposes structural operation counts, and `examples/world_bench.rs` is the timing harness.

CI now rejects duplicate Ariadne architectures and scans the canonical crate for forbidden implementation drift.

## Validation status

This is an architectural landing, not a release claim. GitHub Actions currently refuses runner allocation because of account billing or spending-limit state, and this execution environment has no Rust toolchain. Compilation, formatting, Clippy, workspace tests, plug-in bundling, lockfile regeneration, and measured benchmark results remain draft-PR gates.
