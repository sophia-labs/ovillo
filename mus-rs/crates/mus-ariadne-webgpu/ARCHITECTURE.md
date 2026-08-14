# Ariadne WebGPU architecture

## 1. Causal domains

Every primitive declares one domain and one law class.

```text
Domain
  world          authoritative state that can affect physical evolution
  observer       non-loading state derived from physical observations
  sensor         camera/display memory
  presentation   final viewport composition

Law class
  neutral transport
  observer excitation
  constitutive evolution
  dissipation
  radiation
  non-loading observation
  sensor memory
```

The production graph in this crate uses only observer, sensor, and presentation
state. `mus-ariadne` remains the sole authoritative physical runtime.

## 2. Construction and execution

Construction may compile shaders and allocate textures, buffers, pipelines, and
bind groups. Normal replay performs no resource creation, name lookup, graph
search, or synchronous readback.

```text
OpticalConstitutionPlan
        |
        v
OpticalPacketCompiler
  channel names -> fixed indices
  body/field/edge IDs -> fixed packet slots
        |
        v
WorldOpticalSurface
  integer-rational fixed clock
  preallocated packet storage
  persistent wave surface
        |
        v
WorldPresenter
  current ping-pong view -> presentation target
```

The semantic bridge is a separate crate. The GPU primitive layer never assigns
instrument-, score-, page-, or game-specific meanings to its channels.

## 3. Fixed optical time

Presentation frames request progress toward an absolute material tick. The
clock records a target step but completes a step only after its observation has
been validated and its GPU commands encoded.

A distant seek is replayed in bounded batches. Presentation may pause while the
observer catches up; it may not skip or stretch constitutive steps silently.

The first implementation submits once per fixed optical step because every step
updates the same event and uniform buffers. A later staging-ring implementation
may batch command buffers only if it preserves the identical step sequence.

## 4. Persistent field graph

`ObserverImpulseRaster2D` rasterizes compact local packets over their declared
support. `WaveSheet2D` then performs one tiled dense update from the accumulated
field. This avoids a cell-by-event inner loop.

The four persistent channels are deliberately generic:

```text
R  height / scalar optical displacement
G  motion
B  structure
A  emission
```

They are not physical units. The content-addressed bridge declares how physical
observations map into these observer coordinates.

## 5. Reset and seek

Observer reset is a separate zeroing pass over persistent field state. The next
ordinary constitutive step consumes the requested observation. Seeking backward
therefore means:

```text
reset observer state
replay fixed optical steps from tick zero
present current replay state
```

The immutable physical run is never rerun or modified by a viewer seek.

## 6. Presentation

`WorldPresenter` is presentation-only. It derives a normal from the observer
field and maps motion, structure, and emission to display color. Both ping-pong
views are bound at construction, so selecting the readable state is an index,
not a per-frame resource rebuild.

Presentation failure, surface loss, adaptive quality, or browser suspension
cannot change the physical run or its receipt.

## 7. Promotion rule

A GPU primitive may enter the authoritative world lane only after all of the
following are true:

1. persistent state and storage functional are declared;
2. source work, control work, internal loss, exchange, and radiation are
   separately accounted;
3. reciprocal ports participate in the global ledger;
4. cadence is independent of display refresh;
5. exact-zero, segmentation, refinement, and CPU/GPU conformance laws pass;
6. quality adaptation cannot change authoritative evolution;
7. failure and device-loss semantics preserve or explicitly refuse the physical
   claim.
