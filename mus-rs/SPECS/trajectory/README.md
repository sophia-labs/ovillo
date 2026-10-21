# Continuous trajectory program

This directory is the binding v1 seam between MUS authorship, persistent
physical bodies, inverse gesture compilation, and learned gesture models.

- [`TRAJECTORY-v1.md`](./TRAJECTORY-v1.md) — semantic and implementation contract.
- [`fixtures/weave-rise.json`](./fixtures/weave-rise.json) — direct Weave control path.
- [`fixtures/garganta-ba.json`](./fixtures/garganta-ba.json) — current GARGANTA adapter path.
- matching `*.receipt.json` files — Python-produced, Rust-replayed receipts.

The executable Rust object is `mus_trajectory::lanes::TrajectoryProgram`. The Python
research mirror is `mus_analysis.trajectory.TrajectoryProgram`.
