# Ariadne Shared World v1 landing

**Branch:** `agent/ariadne-shared-world-v1`  
**Base:** implementation-team validation head `32ec027913182b63a42ce1ba2b99e21a2e171014`  
**Status:** draft pending compiler-backed repair and one-body golden replay

## What changed

Ariadne's canonical ownership boundary is now:

```text
Body
  local persistent state
  local source/contact boundary
  local active/passive material
  local ordered transport
  local radiation aperture

World
  Body[]
  one shared reciprocal spatial field
  direct reciprocal body/body edges
  non-loading observers
  one WorkLedger
```

The previous `WorldSpec` remains the one-body compatibility surface. It is
compiled through `WorldPlan::from_single_body` and executes through the same
multi-body `World` implementation used by larger plans.

## New implementation surfaces

- `plan.rs`: content-addressed `WorldPlan`, `BodySpec`, body instances, shared
  field, body/field ports, direct couplings, poses, and observers.
- `body.rs`: local body state and law execution with no private room or ledger.
- `coupling.rs`: sparse storage-free reciprocal body/body edges.
- `space.rs`: one delayed field opened once per frame and shared by any number of
  body ports.
- `world.rs`: fixed-order multi-body execution and one global work ledger.
- `trajectory.rs`: typed `body:<id>/...` shared-world trajectory binding.
- `performance.rs`: plan-aware state and operation estimates.
- `examples/shared_world_bench.rs`: multi-body timing and closure boundary.

## Frozen order

```text
body local source/boundary/active/transport/propagation/loss
-> direct body/body coupling
-> all body/shared-field exchanges
-> one shared-field evolution
-> body and field radiation
-> one ledger frame
-> non-loading observers
```

## Compatibility obligations

Before leaving draft, the implementation team should establish:

1. `cargo fmt`, Clippy, and all workspace tests;
2. one-body frame/state/receipt parity against the validated base;
3. stable CLAP/VST3 product tests and bundles;
4. full-song render hash or a documented intended delta;
5. multi-body global closure over long randomized runs;
6. allocation-free material-thread execution;
7. measured 1/2/4/8/16-body scaling at 48/96/192 kHz;
8. deterministic replay under plan ordering and trajectory call segmentation.

## Not yet claimed

- MUS score-to-WorldPlan compilation;
- heterogeneous body kernels;
- stateful collision/contact edges;
- a two-dimensional Ariadne Sea surface;
- floating rigid bodies;
- a universal observer trace;
- parallel body scheduling;
- production multi-body CPU capacity.

The architecture is intended to receive those features without another world or
ledger implementation.
