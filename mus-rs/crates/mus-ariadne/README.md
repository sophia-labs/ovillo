# mus-ariadne

`mus-ariadne` is the canonical host-free runtime for Ariadne's persistent
metamaterial world.

```text
WorldSpec
  anatomy, capacities, fields, apertures, active material, space
        |
        v
World::compile
  validation, normalization, allocation, digest, cost model
        |
        +---- TrajectoryProgram -> TrajectoryBinding -> typed control frames
        |
        v
World
  persistent state + fixed stage order + one work ledger
        |
        v
WorldFrame + WorldReceipt
```

New musical capabilities enter as one of:

1. a reusable local law under `law/`;
2. a field or space representation satisfying the same energy/receipt contract;
3. a `WorldSpec` anatomy or configuration path;
4. a `mus-trajectory` program over canonical world coordinates;
5. a `mus-gesture` compiler that lowers to the same trajectory/operator IR.

A new instrument-specific renderer is not an accepted extension mechanism.

## Dependency direction

- `mus-watsn`: small executable mathematical laws and sparse operators.
- `mus-ariadne`: physical-world IR, state, runtime, radiation, ledger, layout.
- `mus-trajectory`: direct content-addressed control paths.
- `mus-gesture`: inverse path compilation.
- `mus-dsp`: historical MUS kernels and explicit regression controls.
- plug-ins: ABI, smoothing, clock conversion, and no new sound-generating law.

## Real-time contract

Construction may allocate, parse, hash, normalize, and build delay storage.
The sample path allocates and locks nothing, performs no schema/name lookup,
file I/O, FFT, graph search, or artifact generation, and advances one fixed
persistent world. `RuntimeLayout` and `CostEstimate` make state and hot-path
cost visible before an anatomy is promoted.

## Product compatibility

The currently shipped Terminal remains a quarantined session-compatibility
surface while its controls are migrated onto `WorldSpec` and typed trajectories.
It may receive safety and migration fixes, not new physical capabilities.
