# `mus-ariadne-webgpu`

`mus-ariadne-webgpu` is Ariadne's reusable GPU substrate for persistent,
non-loading optical constitutions and presentation. It is not a second physical
World and it is not a game engine.

The clean dependency boundary is:

```text
mus-ariadne World
       |
       v
mus-observation typed frames
       |
       v
mus-ariadne-webgpu-bridge
  content-addressed semantic binding
       |
       v
mus-ariadne-webgpu
  persistent observer fields + presentation
```

The crate owns persistent textures, buffers, pipelines, fixed optical clocks,
and portable WESL-authored laws. It owns no `WorldPlan`, physical ledger,
listener, window, page-specific channel table, or game semantics.

## Primitive graph

```text
ObserverImpulseRaster2D
  compact typed observations -> local additive excitation field
             |
             v
WaveSheet2D
  fixed-rate tiled constitutive evolution
             |
             v
WorldPresenter
  non-loading full-screen presentation
```

`WaveSheet2D` remains observer-owned. Its height, motion, structure, and
emission channels are an optical constitution; they are not the authoritative
Ariadne sea or an alternative physical-energy ledger.

## Runtime laws

- Absolute Ariadne material ticks are the only scheduling authority.
- Optical steps use an integer-rational fixed clock independent of display
  refresh.
- Scheduling is transactional: a failed encode does not complete a step.
- Long seeks replay in bounded batches.
- Reset clears observer history separately, so the first post-reset observation
  is not discarded.
- Persistent GPU resources and bind groups are allocated before replay.
- Presentation quality may vary only in non-authoritative lanes.
- Promotion of any GPU state into `Domain::World` requires an explicit storage
  law, source/loss/radiation decomposition, reciprocal ports, refinement
  evidence, and participation in the one physical ledger.

See `ARCHITECTURE.md`, `INTEGRATION.md`, `BENCHMARK-PLAN.md`, and
`ACKNOWLEDGEMENTS.md`.
