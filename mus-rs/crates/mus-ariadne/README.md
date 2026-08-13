# mus-ariadne

`mus-ariadne` is the canonical host-free runtime for Ariadne's persistent metamaterial world.

```text
WorldSpec
  anatomy, capacities, field metric, compliant boundary, active material,
  ordered transport, reciprocal space, and radiation
        |
        v
World::try_new
  validation, normalization, allocation, digest, and structural cost model
        |
        +---- TrajectoryProgram
        |       -> TrajectoryBinding::compile
        |       -> typed runtime updates
        |
        v
World
  one persistent state + one frozen sample order + one WorkLedger
        |
        v
FrameOutput + WorldReceipt
```

New musical capabilities enter as one of:

1. a reusable local law under `law/`;
2. a field or space representation satisfying the same energy and receipt contract;
3. a `WorldSpec` anatomy or configuration path;
4. a `mus-trajectory` program over canonical world coordinates;
5. a `mus-gesture` compiler lowering to the same trajectory and operator IR.

A new instrument-specific renderer is not an accepted extension mechanism.

## Programming paradigm

- Describe anatomy rather than a serial processor. Instruments are regions and paths in `WorldSpec` configuration space.
- Keep state persistent. Gestures act on one world; fresh bodies require an explicit reset.
- Separate conservative transport, external work, control work, loss, and radiation under one global balance.
- Treat space as a body. Delayed fields exchange energy bidirectionally with material state.
- Compile names away. Parsing, graph search, and optimization finish before the sample path.
- Add configurations before laws. A new law is justified only after honest search and ablation show a missing capability.

## Dependency direction

- `mus-watsn`: executable mathematical laws, sparse operators, and the ledger.
- `mus-trajectory`: validated content-addressed authored paths.
- `mus-gesture`: inverse path compilation.
- `mus-ariadne`: physical-world IR, state, laws, execution, radiation, and receipts.
- `mus-dsp`: historical MUS kernels and explicit scientific controls.
- plug-ins: ABI, smoothing, clock conversion, and state migration; no physical law.

No dependency may point from `mus-ariadne` into `mus-dsp`, a plug-in, analysis code, target fitting, or an optimizer.

## Frozen material-clock order

```text
external work
-> compliant boundary
-> reservoir-paid active material
-> ordered metric-compatible transport
-> conservative field propagation
-> passive loss
-> reciprocal space exchange and evolution
-> material and space radiation
-> one ledger frame
```

Changing this order changes the physical model and requires a runtime version change plus new law, listening, and performance evidence.

## Real-time contract

Construction may allocate, parse, hash, normalize, orthogonalize, and build delay storage. The sample path allocates and locks nothing, performs no name lookup, file I/O, FFT, graph search, optimization, or artifact work, and advances one fixed-capacity persistent world.

`RuntimeLayout` exposes state and delay storage. `CostEstimate` reports deterministic operation counts and is explicitly not a benchmark. Hardware timing is recorded separately with compiler, CPU, sample rate, block size, world digest, and build profile.

## Product compatibility

The currently shipped Terminal remains under `compat::terminal` so existing sessions and parameter IDs survive this migration. The plug-in imports it directly from `mus-ariadne`. It may receive safety and migration fixes, but new physical capability belongs in `World`.
