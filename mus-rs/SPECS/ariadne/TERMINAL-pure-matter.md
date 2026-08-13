# Ariadne Terminal — pure-matter live contract

**Status:** binding implementation specification for the product-facing live plug-in  
**Supersedes:** the electrical/moving-coil repair proposed in the first guitar metamaterial audit  
**Preserves:** `powered_radiator.rs` only as a legacy/A-B control

## 1. Constitutional decision

Ariadne Terminal is not an amplifier circuit model. It is one persistent normalized acoustic/metamaterial graph. Historical guitar-amplification behavior is a calibration target, not a set of privileged internal component names.

The live graph is:

```text
host boundary
  <-> finite material aperture
  <-> nonlinear canonical cells
  <-> closed ordered transport
  <-> reservoir-limited positive deformation
  <-> finite radiation aperture
  <-> reciprocal long-delay halo
  -> host observation
```

No alternate wet renderer may sit beside this graph. Character presets and fitted profiles change its anatomy and control compilation only.

## 2. Fixed state

For `N=12` material cells:

```text
x_material = (q_0..q_11, p_0..p_11).
```

The room contains `R=12` fixed delay courses. The supply contains one finite scalar reservoir. Capacity and all delay allocations are fixed outside the audio callback.

The stored state reported by the global ledger is

```text
E_total = E_material + E_room + E_reservoir.
```

Host readout scaling is a coordinate conversion and is excluded from stored-state claims.

## 3. Material Hamiltonian

For each cell:

```text
H_i(q_i,p_i) = 1/2 p_i^2 + V_i(q_i)
V_i(q;k,s) = 1/2 q^2 + k/4 [q(q+2s)]^2.
```

`k >= 0`. Profile validation and control bounds keep the nominal law in the convex regime, though positivity does not require convexity.

The exact linear limit is `k=0`. The exact reflection-symmetric limit is `s=0`.

### 3.1 Discrete-gradient propagation

For phase step `h`, solve

```text
2(b-a)/h - 2p_old + h Vbar_q(a,b) = 0
p_new = 2(b-a)/h - p_old.
```

The polynomial divided gradient is evaluated analytically. Eight bounded Newton iterations are the production approximation. Tests compare old/new cell energy and retain the maximum equation residual.

No output sample enters `V`.

### 3.2 Material control work

When controls change `(k,s)` to `(k',s')`, before propagation book

```text
W_material_control
  = dt sum_i [V_i(q_i;k'_i,s'_i) - V_i(q_i;k_i,s_i)].
```

The state coordinate does not move during this measurement. A zero displacement therefore yields zero law-change work.

## 4. Boundary law

Transform stereo host coordinates into common/difference waves with an orthogonal map. Each boundary coordinate exchanges with material momenta through authored Givens factors over a finite aperture.

Required receipt identity:

```text
1/2 dt (||u_in||^2 - ||u_return||^2)
  = 1/2 dt (||p_after||^2 - ||p_before||^2).
```

The left-minus-right residual is accumulated independently of the global ledger.

A ringing material may return energy through a zero-valued host boundary. The wet output need not include that returned coordinate; it remains observable telemetry.

## 5. Powered deformation

Construct normalized direction `v(x, controls)` from the profile pump aperture and current material geometry. Let `d=v^T p`.

For target scalar stretch `alpha >= 1`:

```text
W_requested = 1/2 dt d^2 (alpha^2 - 1).
W_accepted  = min(W_requested, reservoir).
d'          = sign(d) sqrt(d^2 + 2 W_accepted/dt).
p'          = p + (d'-d)v.
```

Then

```text
Delta E_material = W_accepted
Delta E_reservoir = -W_accepted
```

up to floating-point residual. Exact zero state remains zero because `d=0` refuses the deformation.

External reservoir recovery is a separate source-work term. `drive=0` is an exact zero-supply-transfer law.

## 6. Ordered transport

Use `mus_watsn::givens`; do not implement a competing rotation law.

For each overlapping triplet `(i,j,k)`, execute the authored sequence corresponding to

```text
C(a,b)=R_ij(a) R_jk(b) R_ij(-a) R_jk(-b).
```

The execution schedule returns authored angles to their endpoints and preserves momentum norm. Angles may depend on local strain, but this dependence must remain pointwise orthogonal.

Receipts include:

- kinetic metric residual;
- maximum analytical order-defect chord;
- controls and profile digest in future serialized state.

Product text calls this `closed ordered transport`. Promotion to holonomy requires the separate scientific evidence contract.

`chirality=0` or `interlace=0` is an exact identity law.

## 7. Loss and radiation

### 7.1 Material loss

Each material momentum receives a positive retain factor derived from profile T60 and continuous controls. Book exact kinetic deficit.

### 7.2 Material radiation

Generate two orthonormal aperture rows from profile positions and weights. For projection vector `y`, retain `sqrt(1-eta)y` in state and emit `sqrt(eta)y`.

Book

```text
L_material_radiation = 1/2 dt ||y_emitted||^2.
```

### 7.3 Host observation

The profile’s `host_observation_scale` converts emitted normalized coordinates to DAW full scale. It does not feed the material and does not alter the ledger. User output gain is also outside the physical-state claim.

No limiter is permitted in the reference path.

## 8. Reciprocal halo

For each sample:

1. read one head from each persistent room course;
2. exchange selected heads and material momenta by Givens rotations;
3. apply one orthogonal Householder circulation;
4. apply course retains derived from delay and T60;
5. radiate two orthonormal projections;
6. write and advance all heads.

Track room stored energy incrementally from the replaced head coordinates. The body/room coupling residual must close against the material kinetic change.

`space=0` is an exact disconnected bridge. A nonzero stored halo is not destroyed by later disconnection.

## 9. Frozen sample order

```text
1. sanitize host/control boundary
2. book material-law control work
3. recharge finite reservoir
4. exchange host boundary and material aperture
5. apply reservoir-limited positive deformation H
6. execute closed ordered transport U
7. advance nonlinear material by discrete gradient
8. apply material loss
9. exchange material and room; scatter, damp, radiate room
10. radiate material aperture
11. close global work frame
12. convert emitted coordinates to host observation and optional dry mix
```

Changing this order is a model version change.

## 10. Global receipt

Every frame pushes the existing `mus_watsn::WorkFrame`:

```text
energy_start
source_work = boundary_work + external_reservoir_recharge
control_work = material_law_change
internal_loss = material_loss + room_loss
radiation_loss = material_radiation + room_radiation
output_work = 0
energy_end
```

Reservoir-to-material work and room exchange are internal transfers. They receive telemetry lines but cancel from the global state balance.

The public receipt additionally contains local metric residuals, equation residual, path chord, stored energies, peaks, and numerical hygiene counters.

## 11. Plug-in boundary

`ariadne-clap` exposes the new terminal under new host identities:

```text
CLAP: com.sophia-labs.ariadne-terminal
VST3 class: AriadneTerminal1
```

The shell owns no sound law. Sample-accurate controls compile to the core control struct. The engine reports zero latency and a conservative 20-second tail.

The old VST class identity is not silently reused for a different engine.

## 12. Required tests

1. nominal profile and sample-rate validation;
2. positive asymmetric potential and exact linear limit;
3. discrete-gradient energy preservation;
4. closed-transport nontriviality and zero work;
5. exact reservoir/material exchange under positive deformation;
6. exact acoustic silence fixed point;
7. driven global ledger closure;
8. exact zero room bridge;
9. passive non-growth under a stored tail;
10. explicit material-control work;
11. bit-exact dry path at zero mix;
12. block segmentation invariance;
13. non-finite input containment;
14. no audio-callback allocation, lock, I/O, or graph resize;
15. later: sample-rate agreement, aliasing atlas, CPU envelope, and listener validation.

## 13. Claim boundary

This landing may claim:

- a pure Ariadne live material implementation;
- exact algebraic work identities in real arithmetic;
- structure-preserving local laws implemented in Rust;
- no conventional source/cabinet render path in the product-facing engine.

It may not claim until execution evidence exists:

- Rust build/test success on this branch;
- real-time CPU suitability at all rates;
- perceptual superiority;
- faithful emulation of a named guitar rig;
- universal electric-guitar coverage;
- alias-free nonlinear output;
- geometric or topological holonomy.

Those ceilings remain in receipts and release notes.
