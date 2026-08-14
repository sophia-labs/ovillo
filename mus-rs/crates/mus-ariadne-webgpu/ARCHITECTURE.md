# Ariadne GPU architecture v0

## 1. Constitutional unit

A primitive is a law over persistent GPU resources, not merely a draw call.
Every primitive declares:

```text
Domain          world | observer | sensor | presentation
Law             neutral transport | work injection | constitutive evolution |
                dissipation | radiation | non-loading observation | sensor memory
Cadence         every frame | fixed rate | event-driven | dirty
Support         dense | event raster | active tiles | sparse bricks | full screen
Zero law        an exact or tolerance-bounded absence statement
Receipt         timing, quality, resource, and optionally physical residuals
```

This keeps the causal boundary inspectable when many optical and material laws
share one render graph.

## 2. Three execution lanes

### Authoritative

State that affects sound, gameplay, physical receipts, or future world
evolution. Its integration law cannot change because presentation quality is
reduced.

### Optical constitution

Derived emission, stress, density, orientation, refraction, and radiation.
Quality may vary only when the approximation is declared non-loading.

### Sensor and presentation

Phosphor, exposure, bloom, temporal reconstruction, and viewport composition.
This lane may adapt aggressively to a measured frame budget.

## 3. First graph

```text
host event ring
      |
      v
ImpulseRaster2D -- one local quad per event
      |
      v
accumulated RGBA16F impulse field
      |
      v
WaveSheet2D -- tiled 16 x 16 stencil with an 18 x 18 shared-memory halo
      |
      v
persistent RGBA16F state
  R height
  G velocity
  B foam/turbulence
  A optical excitation
```

The previous Ariadne Sea proof evaluates every event in every cell. The first
library graph instead evaluates each compact event only over its covered pixels,
then advances the dense field with one impulse read per output cell.

## 4. Resource ownership

The library owns persistent textures, bind groups, pipelines, and hot-path GPU
buffers. Consumers own:

- adapter/device creation;
- surfaces and presentation;
- the absolute Ariadne clock;
- authoritative world state;
- event semantics;
- final graph composition.

After warm-up, normal operation should allocate no GPU resources and no host
vectors per frame. V0 exposes upload methods; the next migration adds a
preallocated event-ring writer for consumers.

## 5. Shader system

WESL is the authored shader format. `build.rs` compiles named modules into WGSL
artifacts; Rust embeds those artifacts with `include_wesl!`. Generated WGSL is
the code passed to `wgpu`/Naga and browser WebGPU.

The portable baseline requires only:

- `f32` shader arithmetic;
- `rgba16float` render/storage textures;
- ordinary workgroup memory;
- render and compute pipelines.

Timestamp queries and `shader-f16` are optional capability tiers, never hidden
requirements.

## 6. Promotion rule

A primitive may begin as an observer-owned visual law. It becomes an Ariadne
world law only after:

1. its state and energy/storage meaning are declared;
2. source, loss, radiation, and transport semantics are separated;
3. its zero laws pass;
4. refinement behavior is measured;
5. authoritative timing is independent of display refresh;
6. any reciprocal loading is included in the global ledger.
