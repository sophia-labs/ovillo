# `mus-extension-reference`

This is a candidate Rust lowering of the exact `Cl(0,4)` basis gestures into the existing `mus-watsn::OperatorProgram` and `PlaneRotation` types.

It is intentionally staged inside the experiment and is not added to the `mus-rs` workspace yet. Its manifest declares an empty nested `[workspace]` so Cargo can compile it directly without changing the production workspace membership. The intended command in a Rust-enabled checkout is:

```bash
cargo test --manifest-path \
  mus-rs/experiments/extension_v1/rust/mus-extension-reference/Cargo.toml
```

The tests cover:

- the `1,4,6,4,1` grade layout;
- eight disjoint local rotations per gesture;
- `D_i^2 = -I`;
- `D_iD_j = -D_jD_i` for distinct generators;
- all 24 permutation-orientation cases;
- exact inverse-program replay to binary64 tolerance.

## Validation status

The source was derived against `mus-watsn` at branch `agent/threader-poc-v0`, where `OperatorProgram`, `PlaneRotation`, and `ProgramError` have the APIs used here. The current execution container does not contain `cargo` or `rustc`, so this Rust candidate has **not** been compiled in this delivery. It must remain a candidate until replayed in a Rust-enabled environment. The Python oracle and its equivalent sparse factor schedule are fully tested.
