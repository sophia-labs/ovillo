# Ariadne guitar metamaterial audit — corrected constitution

**Base audited:** `agent/instrument-convergence-guitar-finish`  
**Revision branch:** `agent/ariadne-guitar-multidomain-core`  
**Scope:** guitar/string renderer, live terminal VST, pedal/amp trajectory  
**Decision:** Ariadne must become more physically accountable without becoming an electrical/circuit model

## 0. Correction to the first audit

The first version of this document correctly diagnosed the three-stage source cascade and cabinet sections as an effects-chain island. It made the wrong architectural repair. It proposed electrical, magnetic, mechanical, and acoustic domains joined by effort/flow ports, then landed a moving-coil loudspeaker cell.

That repair was mathematically respectable and constitutionally wrong for Ariadne.

The binding project documents are unusually consistent on this point:

- the ordinary guitar is a calibrated, low-coupling limit of one wider string-network ontology;
- the powered body is a reservoir-limited nonlinear mode graph, not a virtual analog amplifier;
- the radiator is a finite spatial aperture of the same state, not a simulated commercial loudspeaker;
- string, body, near field, and room are regions of one persistent metric state;
- control acts by work-accounted deformation and ordered metric-compatible transport;
- circuit and amplifier accuracy are explicitly outside the first claim.

Port-Hamiltonian and bond-graph mathematics remain useful prior art for power accounting. They do not imply that Ariadne should import voltage, current, flux, transistors, transformers, voice coils, or a second circuit ontology into its production core.

The corrected rule is:

> **Ariadne may borrow laws from any physics, but every borrowed law must be translated into Ariadne’s one persistent acoustic/metamaterial state rather than installing the donor domain as a privileged simulator inside the graph.**

The electrical files introduced in the first draft are therefore removed. Their useful invariant—no unaccounted energy—survives in a more native form.

## 1. What Ariadne is

Ariadne is a family of finite metric state spaces over a control/configuration space. A configuration `lambda` owns state

```text
x in H_lambda
```

and positive energy metric

```text
E_lambda(x) = 1/2 x^T M_lambda x.
```

A control step transports live state between configurations. In energy-normalized coordinates, the transport has weighted polar decomposition

```text
A = U H.
```

- `U` is metric-compatible transport. It changes the orientation of state while preserving energy. Ordered products of noncommuting local factors can retain path memory.
- `H` is positive deformation. It stretches or contracts energy directions and therefore performs measurable state-dependent work.

Propagation, explicit loss, radiation, topology/rank changes, sources, and readouts surround this transport, but they do not replace it. The complete graph emits one receipt:

```text
Delta E = W_boundary + W_supply + W_control
          - L_internal - L_radiation - W_output + epsilon_numeric.
```

This is the common ontology for a string, a throat, an amp-like powered body, a radiator, and a room.

## 2. What the electric guitar and amp become in that ontology

“Amp” and “cabinet” are behavioral names for regions of an Ariadne world, not component schematics.

A pure Ariadne terminal contains:

1. **A persistent nonlinear material body.** Its state is not the current audio sample. It continues across samples, blocks, notes, and silences.
2. **A finite supply reservoir.** Active behavior is energy conversion from stored supply into selected material directions.
3. **A path-ordered connection.** Sparse lossless factors circulate state. Overlapping closed commutators can leave ordered memory with zero local work.
4. **A spatial throat.** Finite input and radiation apertures couple to an extended body, not to a scalar transfer curve.
5. **A reciprocal room halo.** Long-delay courses exchange state bidirectionally with the material before they scatter, damp, and radiate.
6. **One ledger.** Boundary work, external recharge, control work, internal loss, radiation, room exchange, reservoir transfer, and numerical residual are separately visible.

The target behaviors associated with electric guitar amplification—touch sensitivity, asymmetric harmonic growth, intermodulation, compression, bloom, sag, sustained instability, speaker-like spatial narrowing, cabinet-like resonant color, and room feedback—must emerge from those mechanisms.

They are fit targets, not implementation vocabulary.

## 3. The first pure live material

The revision introduces `mus_dsp::ariadne_terminal` and makes it the engine behind the Ariadne CLAP/VST3 shell. The earlier `powered_radiator` remains in the repository as a historical and scientific A/B control; it is no longer the product-facing implementation.

### 3.1 Material cells

The terminal contains twelve canonical cells with normalized displacement and momentum

```text
(q_i, p_i), i = 0..11.
```

Each cell carries the positive asymmetric potential

```text
V_i(q; k_i, s_i)
  = 1/2 q^2 + k_i/4 [q(q + 2 s_i)]^2.
```

Properties:

- `k_i = 0` is the exact linear limit;
- `s_i = 0` is the exact symmetric limit;
- `V_i >= 0` for every real `q_i`;
- curvature and asymmetry are state law, not a function applied to the audio sample;
- changing `k_i` or `s_i` while `q_i != 0` changes stored energy and is booked as control work.

Expanded as

```text
V(q) = A q^2 + B q^3 + C q^4
A = 1/2 + k s^2
B = k s
C = k/4.
```

For old displacement `a` and new displacement `b`, the exact divided gradient is

```text
Vbar_q(a,b)
  = A(a+b)
  + B(a^2 + ab + b^2)
  + C(a^3 + a^2b + ab^2 + b^3).
```

The update solves

```text
(b-a)/h = (p_new+p_old)/2
(p_new-p_old)/h = -Vbar_q(a,b)
```

with a bounded Newton solve. In exact arithmetic this preserves

```text
1/2 p^2 + V(q)
```

for every fixed law. The nonlinearity cannot be described as `sample = f(sample)` because it is a conservative trajectory through stored state.

### 3.2 Boundary exchange

Stereo input is decomposed into common and differential boundary waves. Each wave traverses a finite aperture across the material by a sequence of Givens rotations. Every local exchange preserves the joint norm of boundary and material momentum.

The boundary work is measured from both sides:

```text
W_boundary
  = 1/2 dt (||u_in||^2 - ||u_return||^2)
  = Delta E_material_at_boundary.
```

The difference between those two evaluations is a permanent metric-residual field. The returned boundary wave is exposed in telemetry even though the wet product is the body’s radiation aperture.

This is reciprocal. A ringing material can return energy to an otherwise zero input boundary.

### 3.3 Powered deformation

Let `v(x)` be a normalized pump aperture and let

```text
d = v^T p
```

be the material momentum projected onto it. The powered step selects a multiplier `alpha >= 1` and proposes

```text
d' = alpha d.
```

The required work is exact:

```text
W_H = 1/2 dt (d'^2 - d^2).
```

Only the amount available in the finite reservoir is accepted. The actual update is

```text
p' = p + (d'-d) v.
```

This is a rank-one positive polar deformation along `v`. It is the `H` sector of Ariadne’s existing work–holonomy theory. It cannot excite exact zero state, cannot spend more than the reservoir contains, and has an independent metric residual.

“Drive” changes the maximum deformation rate. “Sag” changes external reservoir recovery. “Bloom,” “headroom,” and “sustain” change the state-dependent aperture and conversion law. None chooses a gain stage or a clipping curve.

### 3.4 Closed ordered transport

For overlapping material planes, define

```text
A = R_ij(a)
B = R_jk(b).
```

Each live step can execute the closed word

```text
C(a,b) = A B A^-1 B^-1.
```

Every authored angle returns to its endpoint. Every factor is metric-orthogonal. Nevertheless, for overlapping planes,

```text
C(a,b) != I
```

in general. At small angles the leading displacement lies in the remaining `(i,k)` plane and scales with the commutator of the generators.

The live implementation uses the existing `mus-watsn` Givens law and order-defect observable. It does not create a second factor executor. The factor angles depend on local strain and continuous controls, so curvature changes how the same closed path transports state without adding work.

This is called **closed ordered transport** in product code. A scientific holonomy claim still requires the project’s stronger closed-path, connection, gauge, and evidence conditions.

### 3.5 Dissipation and radiation

Material damping acts on momentum and books the exact kinetic-energy deficit. Radiation selects two orthonormal finite-aperture rows, contracts only their projected state by `sqrt(1-eta)`, and emits coordinates scaled by `sqrt(eta)`.

Therefore

```text
E_removed = 1/2 dt (y_left^2 + y_right^2)
```

up to floating-point residual. There is no cabinet filter and no impulse response.

The profile’s `host_observation_scale` is an explicit unit conversion from normalized emitted-wave coordinates to DAW full scale. It does not participate in the material dynamics or its energy claim.

### 3.6 Reciprocal spatial halo

The room contains twelve persistent delay courses. At every sample:

1. course heads exchange energy with selected material momenta through Givens rotations;
2. the head vector undergoes an orthogonal Householder circulation;
3. each course loses the amount required for its declared T60;
4. two orthonormal room apertures radiate;
5. the new heads return to the delay lines.

The body/room bridge is bidirectional. `space=0` is an exact decoupled limit. A previously excited room remains a persistent body rather than disappearing when the control closes.

This is not “reverb after distortion.” It is a long-delay region of the same world.

## 4. Why this is purer than electrical modeling

The discarded electrodynamic branch had several admirable properties: persistent state, reciprocal coupling, nonlinear stored potential, explicit loss, and local closure. Those properties are retained.

What is removed is the privileged explanation:

```text
voltage -> coil -> cone -> enclosure -> pressure.
```

That explanation would make Ariadne’s success depend on reproducing historical hardware categories. It would also encourage one simulator for pickups, another for amplifiers, another for loudspeakers, and then a glue graph around them—the opposite of the unified instrument.

The corrected explanation is:

```text
boundary wave
  <-> powered nonlinear metric material
  <-> ordered internal geometry
  <-> finite radiation aperture
  <-> persistent spatial halo.
```

A vintage combo, a transistor wall, a feedback installation, an electric harp, or an impossible resonant organism are calibrated anatomies of this same model class.

## 5. Fidelity without mechanism imitation

The absence of circuit labels does not lower the fidelity ambition. It raises the validation burden.

A profile is allowed to claim behavioral coverage only after fitting and held-out replay across a probe atlas containing:

- low, medium, and high level swept sines;
- multitone intermodulation probes;
- isolated notes across register and dynamics;
- dyads, dense chords, and low-interval grind;
- palm-muted attacks and long sustains;
- pickup transients and volume-knob trajectories;
- feedback onset, hysteresis, and decay;
- multiple source impedances or DI front ends where relevant;
- near and far microphone positions for spatial targets;
- sample rates and host block segmentations.

The fit may move geometry, material law, damping, pump aperture, reservoir law, ordered transport, radiation aperture, and halo delays. It may not add a sidecar EQ, waveshaper, cabinet IR, compressor, neural residual, or target-conditioned runtime correction.

The scientific claim ladder should be:

1. **lawful material** — invariants and ledger gates pass;
2. **behaviorally resemblant** — selected observables improve on declared controls;
3. **probe-generalizing** — fitted anatomy predicts untouched probes and playing gestures;
4. **rig-faithful** — blinded listeners and held-out measurements support one named rig family;
5. **class-covering** — a fixed primitive vocabulary spans a preregistered guitar/amp/cabinet corpus without sidecars.

“Every electric guitar ever made” belongs only at level 5, and only for the measured model class and uncertainty envelope. It cannot mean inferring an unknown instrument’s complete state from one mono recording.

## 6. Product boundary

### Ariadne Terminal

A VST/CLAP or pedal receives an already-transduced audio boundary. It cannot reconstruct the exact upstream string, body, pickup, cable, and player state. It can instantiate a downstream powered Ariadne world and return its emitted field.

The new plug-in is this product.

### Ariadne Instrument

A rendered or sensor-augmented instrument owns player contacts, multidimensional strings, bridge, body, readout, powered material, halo, and room in one graph. This is the route to Jazzmaster, bass, lap steel, twelve-string, electric harp, and impossible string bodies.

### Ariadne boundary hardware

A pedal or amplifier can expose more state without becoming a circuit simulator:

- calibrated high-impedance audio boundary;
- multiple physical sensor inputs;
- body acceleration or bridge force;
- loudspeaker/microphone or transducer returns;
- explicit actuator outputs;
- clock-synchronized work and latency receipts.

These signals become boundary coordinates of the same Ariadne state. The hardware need not dictate the internal ontology.

## 7. No-cheating gates

The product-facing pure engine must satisfy:

1. no call equivalent to `sample = nonlinear(sample)`;
2. no cabinet filter, cabinet IR, post-EQ, compressor, limiter, or neural residual in the wet reference path;
3. no electrical component graph in the production ontology;
4. no gain event without reservoir transfer or boundary/control work;
5. no parameter change that alters stored energy without a control-work line;
6. no room send that cannot return energy to the material;
7. no key tracker, scale logic, pitch shifter, or compositional policy in physical-reference mode;
8. exact zero limits for supply, curvature, chirality, room bridge, and wet mix;
9. passive non-growth under adversarial stored states;
10. one global material/room/reservoir ledger;
11. host-block invariance;
12. no sample-path allocation, lock, I/O, FFT, or topology resize;
13. every fitted profile carries provenance, train/validation/test partitions, and claim ceiling.

A static check may reject new production references to cabinet sections, generic waveshaper helpers, or virtual component names. Mathematical functions remain legal inside state laws; their role must be visible in the energy contract.

## 8. Remaining work

This revision is a foundational live material, not the completed universal guitar.

Highest-value next work:

1. **Independent Rust execution and listening.** Compile, run the law battery, render DI probes, and inspect aliasing, CPU, and audible range. The current branch could not receive a GitHub runner because Actions billing was blocked.
2. **Material refinement.** Compare the twelve-cell reduction with larger sparse fields and a structure-preserving multirate refinement.
3. **Behavioral fitting.** Build a differentiable or derivative-free offline profile fitter constrained by the exact live engine and held-out probe atlas.
4. **Instrument coupling.** Lower the full Jazzmaster string/body graph into the same terminal anatomy rather than feeding it through an insert boundary.
5. **Contact and player state.** Replace excitation initializers with distributed compliant contacts and paid continuous gestures.
6. **Measured body geometry.** Fit passive bridge/body multiport operators without making them a separate renderer.
7. **Hardware boundary study.** Determine which additional sensors and actuators produce the largest identifiable expansion of the Ariadne state.

## 9. Final judgment

The original diagnosis remains: the live source and cabinet cascade was too conventional.

The corrected repair is not a more accurate conventional amplifier. It is a stranger and more coherent object:

> **a powered, nonlinear, path-dependent acoustic metamaterial whose calibrated conservative limits include guitars and whose active spatial limits can cover the behaviors artists associate with pedals, amplifiers, loudspeakers, feedback systems, and instruments that have never existed.**

That is the Ariadne worth making.
