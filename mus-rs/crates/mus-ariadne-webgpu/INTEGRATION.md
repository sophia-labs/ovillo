# Canonical World/WebGPU integration

There is one clean runtime path.

```text
WorldPlan
  bodies + shared field + reciprocal couplings
       |
       v
mus-ariadne::World
  physical state + material tick + global ledger
       |
       +--> StereoListener -> audio
       |
       +--> BoundObservationProgram -> ObservationTrace / live packet ring
                                              |
                                              v
                                OpticalConstitutionPlan
                                              |
                                              v
                                WorldOpticalSurface
                                              |
                                              v
                                     WorldPresenter
```

## Repository ownership

- `mus-ariadne`: physical construction, state, intervention, receipts, and
  non-loading observation sources.
- `mus-observation`: domain-neutral typed schemas and traces.
- `mus-ariadne-webgpu-bridge`: content-addressed binding from one World and
  observation schema into fixed GPU packet slots.
- `mus-ariadne-webgpu`: persistent observer/sensor/presentation resources and
  laws.
- `mus-ariadne-webgpu-web`: canonical Rust/WASM browser boundary.
- `ariadne-world-export`: immutable audiovisual run-bundle exporter.
- `web/ariadne-world`: thin HTML/JavaScript host; it owns no physics or channel
  table.

## Run bundle

`ariadne-world-export` writes:

```text
manifest.json
run.identity.json
intervention.json
world.plan.json
listener.json
observation.program.json
observation.schema.json
optical.plan.json
observation.aotr
audio.wav
```

The manifest records SHA-256 digests for every file. `intervention.json`
content-addresses source adaptation and controls. The typed `run.identity.json`
then binds the physical world, listener, intervention, observation program,
observation schema, optical constitution, clocks, and horizon into one run
digest. The trace carries that digest internally.

The browser verifies file digests, recomputes the semantic run digest in Rust,
validates all plan/schema bindings, and only then creates GPU resources.

## Browser replay

The browser uses the audio element as an offline presentation cursor only. It
converts presentation seconds into an absolute material tick and asks the Rust
viewer to advance toward that tick. The viewer samples the typed trace according
to channel interpolation declarations and executes the fixed optical clock.

A backward seek resets and replays only observer history. A long forward seek is
processed in bounded batches while audio is paused. No WebGPU operation is on an
audio deadline.

## Live extension

The same contracts extend to live operation without changing the physical
model:

```text
AudioWorklet/WASM World
  -> fixed-capacity observation ring
  -> dedicated GPU worker / OffscreenCanvas

controller/game worker
  -> typed absolute-tick interventions
  -> AudioWorklet/WASM World
```

The material thread never waits for WebGPU. Dropped, late, or refused packets
must appear in runtime receipts.
