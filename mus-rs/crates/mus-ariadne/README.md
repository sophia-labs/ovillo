# mus-ariadne

`mus-ariadne` is the canonical host-free runtime for Ariadne's persistent metamaterial world.

```text
WorldPlan
  shared clock
  BodyInstance[]
  shared spatial field
  reciprocal coupling graph
  Observer[]
        |
        v
World::from_plan
  validation, content address, allocation, port binding, cost model
        |
        +---- TrajectoryProgram
        |       -> SharedTrajectoryBinding::compile
        |       -> typed body/world updates
        |
        v
World
  several persistent bodies
  + one shared field
  + one global WorkLedger
        |
        v
radiation + non-loading observers + SharedWorldReceipt
```

The validated one-body `WorldSpec` remains an exact compatibility embedding:

```text
World::try_new(WorldSpec)
    ==
World::from_plan(WorldPlan::from_single_body(WorldSpec))
```

This keeps the product path stable while moving the canonical ontology from
“one body with a private room” to “many bodies in one shared world.”

## Programming paradigm

- A **track** is an authoring or mix lane.
- A **voice** is an event stream.
- A **BodySpec** is reusable local anatomy.
- A **BodyInstance** is one persistent physical individual.
- A **shared field** is world state through which several bodies may exchange energy.
- A **coupling edge** is an explicit reciprocal relation between body apertures.
- An **observer** is non-loading unless it is promoted to a physical port.

Notes and game actions excite named bodies; they do not create fresh bodies.
A piano track may address one compound body, a drum-kit track may address several
bodies, and doubled parts instantiate separate bodies from one anatomy.

New musical capabilities enter as one of:

1. a reusable local law under `law/`;
2. a `BodySpec` or shared-field representation satisfying the energy contract;
3. a reciprocal coupling edge or declared stateful coupling law;
4. a `WorldPlan` composition;
5. a `mus-trajectory` program over typed body/world coordinates;
6. a `mus-gesture` or Extension compiler lowering to the same program IR.

A new instrument-specific renderer, private room, or private work ledger is not
an accepted extension mechanism.

## Dependency direction

- `mus-watsn`: executable mathematical laws, sparse operators, and the ledger.
- `mus-trajectory`: validated content-addressed authored paths.
- `mus-gesture`: inverse path compilation.
- `mus-ariadne`: bodies, fields, couplings, observers, execution, and receipts.
- `mus-dsp`: historical MUS kernels and explicit scientific controls.
- plug-ins and games: ABI, interaction, presentation, and program compilation; no private physical law.

No dependency may point from `mus-ariadne` into `mus-dsp`, a plug-in, game,
analysis code, target fitting, or an optimizer.

## Frozen material-clock order

```text
for each body:
  external work
  -> compliant boundary
  -> reservoir-paid active material
  -> ordered metric-compatible transport
  -> conservative field propagation
  -> passive loss

all bodies:
  -> stateful contact laws (hammers on wires)
  -> direct reciprocal body/body coupling
  -> body/shared-field exchange
  -> shared-field scattering, loss, and propagation
  -> body and shared-field radiation
  -> one global ledger frame
  -> non-loading observers
```

Changing this order changes the physical model and requires a runtime-version
change plus new law, compatibility, listening, and performance evidence. The
contact stage is empty in every world whose plan declares no hammer, so those
worlds run exactly the order above without it.

## Struck wires

A piano enters as these pieces (`SPEC-ARIADNE-PIANO.md` at the repository root):

- `ConfigurationMapSpec::StiffWire`: a body whose partials are computed from a
  pinned wire's length, core diameter, linear density, Young's modulus and
  tension (the tuning pin is its coordinate), with its mass-normalized mode
  shapes and its kinetic coupling to a moving bridge end;
- `SpaceBodyLaws::Passive`: the field, its loss and projection radiation, and
  an optional felt damper (`law::damper`, a viscous pad whose engagement is
  set by `World::set_body_damper`; it only dissipates);
- `BodyConstitutionSpec::Hammer`: a mass whose power-law felt contact with up
  to four wires is a stateful coupling law (`law::hammer`), thrown by
  `World::strike_hammer` (its kinetic energy is source work) and caught by
  its backcheck (the rebound energy is internal loss). Its stored energy is
  `1/2 M v^2` plus the felt's potential, so `H_world` includes it.

## Typed shared trajectories

A shared trajectory names body coordinates as:

```text
body:<body-id>/material_organization
body:<body-id>/source_wave
body:<body-id>/work_input
body:<body-id>/active.coupling
body:<body-id>/active.drive
body:<body-id>/transport.interlace
body:<body-id>/radiation.fraction
```

World coordinates remain:

```text
space.coupling
space.decay
space.radiation
coupling.scale
observation.scale
```

Names, body IDs, units, bounds, and sample rate are resolved at construction.
The material thread sees typed indices and numeric values only.

## Energy constitution

For bodies `i`, shared fields `a`, and any future stateful coupling edges `e`,

```text
H_world = sum_i H_body_i + sum_a H_field_a + sum_e H_edge_e
```

One global receipt closes:

```text
Delta H_world
  = source work
  + configuration/control work
  - internal loss
  - physical radiation
  + numerical residual
```

Body/body and body/field exchange are internal diagnostics. They never appear as
newly created source work. Physical radiation removes energy once; any number of
ordinary observers may read the emitted coordinates without removing it again.

## Real-time contract

Construction may allocate, parse, hash, normalize, orthogonalize, build delay
storage, compile body IDs, and bind observer taps. The sample path allocates and
locks nothing, performs no name lookup, file I/O, FFT, graph search,
optimization, or artifact work, and advances fixed-capacity state.

`RuntimeLayout::from_plan` exposes state and shared delay storage.
`CostEstimate::for_plan` exposes deterministic body, edge, port, and observer
operation counts and is explicitly not a benchmark.

## Product compatibility

The Ariadne CLAP/VST3 product now runs through canonical `World::try_new`. Its
stable one-body API, parameter IDs, and session identity remain intact. New
multi-body products and MUS score lowering should construct `WorldPlan` and use
`World::from_plan`; they must not create a bank of independent Worlds and call
their studio sum a shared physical system.
