# N-dimensional configuration spaces for Ariadne bodies

Branch `ariadne/nd-configuration-20261020`. Binding context:
`ARIADNE-PURE-PHYSICS-CONSTITUTION.md` (what the World may do),
`ARIADNE-IMPLEMENTATION-CONSENSUS.md` §4 (a transport names its metric and
state correspondence), `SPEC-ARIADNE-STEEL-STRING.md` (how an anatomy
compiles into a plan; frets as receipted transport along a 1-D path),
`SPECS/garganta/` (the throat program, G1-G3). Evidence for every claim
below is in `CHECKPOINT-2A.md` at the worktree root.

## 1. What changes, in one paragraph

Today a body's configuration is one scalar `amount ∈ [0,1]` on a
`ConfigurationPath` between two anchor `WorldConfiguration`s, and
`Body::set_configuration_amount` moves it at fixed modal state, billing
`ΔE` as control work. This spec adds a body whose configuration is a point
`x` in a box of named coordinates (1 to 8), mapped to the body's modes by a
declared **map**, moved under a declared **connection**. A move `x → x'`
is a reconfiguration at fixed physical state with an exact work receipt;
a path through the space is a sequence of moves. Nothing about existing
bodies, plans, digests or renders changes.

## 2. IR

New constitution in `ariadne-world-plan-v3`: `kind: "modal_space"`
(`BodyConstitutionSpec::ModalSpace { body: SpaceBodySpec }`).
`ariadne-world-plan-v2` is untouched; it cannot carry the new body.

```text
SpaceBodySpec
  anatomy_name
  configuration_space:
    coordinates: [{ name, lower, upper, initial }]      1..=8, unique names
    map:        multilinear | stiff_string | tube_network
    connection: modal_coordinates | mass_momentum { disposal: dissipate | radiate | reject }
  laws:        modal { source_direction, active_directions, transport }   (multilinear, stiff_string)
             | tube_network { valve: FlowValveSpec }                     (tube_network)
  reservoir:   { capacity }
```

### 2.1 Maps

| map | what it is | label |
|---|---|---|
| `multilinear { axes, anchors }` | tensor grid of `WorldConfiguration` anchors; between grid values the legacy 1-D rules axis by axis (log-linear frequency, linear T60 and aperture, linear-then-normalized radiation direction) | **fit** (interpolation between authored anchors) |
| `stiff_string { open_fundamental_hz, open_inharmonicity, fret_coordinate, tension_coordinate?, decay a, b, t60_numerator, aperture, radiation_direction }` | pinned stiff string: `L = L0 2^(-frets/12)`, `T = t T0`, `f0 = f0_open 2^(frets/12) sqrt(t)`, `B = B_open 4^(frets/12)/t`, `f_n = n f0 sqrt(1 + B n²)`, `σ(ω) = a + b ω²` | **law** (Euler-Bernoulli stiff string; the loss form is the steel-string compiler's design hypothesis) |
| `tube_network { network }` | lumped acoustic tube network; areas a declared function of `x`; the 24 lowest modes **computed** at every point (§3) | **law** (discretized lossless Webster horn equation); the area family is the anatomy's |

The tube network IR (`TubeNetworkSpec`): constants (`c`, `ρ`, positive area
cone), tubes (`length_m`, per-section `base_log_area_m2`,
`attenuation_np_per_m`), branches (a later tube's first volume joins an
earlier tube's section: a tree), area terms (`gaussian`, `log_scale_gaussian`,
`section_area`, with coefficients affine in `x`), radiating ends (`tube`,
`end_correction_ratio`). No electrical vocabulary: the elements are
compressible air volumes (acoustic compliance) and inertive air columns
(acoustic inertance), per constitution §3.1/§4.

### 2.2 Connections (the declared state correspondence)

- `modal_coordinates`: hold normalized modal `(q_k, m_k)` by mode index;
  the metric changes under them. The legacy connection, generalized. Flat:
  an instantaneous closed loop returns the state and bills exactly zero.
  Valid for every map. For a tube network it is only meaningful away from
  eigenvalue crossings (mode signs are aligned to the previous point by
  `φ'ᵀ C φ ≥ 0`; across an avoided crossing the index no longer names the
  same physical shape).
- `mass_momentum { disposal }`: hold the physical air state, the excess air
  volume `C p` of every compressible volume and the air momentum per area
  `L u` of every column. These are what the network's equations integrate
  when `C(t)`, `L(t)` vary (`d(Cp)/dt = Σu`, `d(Lu)/dt = -Δp`), so this is
  the law of the time-varying lumped network (the wall's own volume pumping,
  a mean-flow effect, is not in linear acoustics; see POC-3). The state is
  re-projected onto the new retained modes in the energy metric:
  `m' = Φ'ᵀ (C Φ m)`, `q' = Ω'⁻² F'ᵀ L'⁻¹ (F q)` (`F = DΦ`). Energy the 24
  retained modes cannot hold (Bessel: physical energy after ≥ retained) is
  **disposed** and booked: `dissipate` → internal loss, `radiate` →
  radiation, `reject` → the move is refused (state and configuration
  restored) if it exceeds `1e-9` of the stored energy. Curved: an
  instantaneous closed loop does not return the state; its net work and
  disposal are billed (holonomy of the truncated connection). Requires a
  tube-network map.

### 2.3 Laws of a tube-network body

- 24-mode `ModalMetricWaveField`, exact per-mode rotation (unchanged).
  Ariadne momentum is modal pressure: `p = √dt Φ m`.
- Flow valve (`law::valve`, new): a fold on a tension spring with
  unilateral contact (midpoint + discrete gradient), a Bernoulli + Poiseuille
  slit jet into one volume. Mechanics copied from `ovillo-tract`
  (`TractWorld::glottis`, `fold_update`). Exact contract at the node:
  `m += √dt φ(n) U`, field gains `p̄ U dt`, jet dissipates `(ps - p̄) U dt ≥ 0`,
  the reservoir (the lungs) pays `ps U dt`. The aerodynamic work on the fold
  is booked to the reservoir, as in `ovillo-tract` (approximation).
  Retuning (tension, adduction) at fixed fold state bills `ΔE` as control
  work (`World::set_body_valve_tuning`).
- Wall loss and radiation, diagonal in the modes (modal approximation of
  non-proportional damping): wall `σ_k = c/2 (Σ α C φ² + Σ α F²/(L ω_k²))`;
  radiation through a conductance `G = A r²/(4 ρ c ℓ²)`, `ℓ = dx/2 + 0.61 r`,
  at each radiating volume (chosen so the parallel end-column/conductance
  load has exactly GARGANTA's low-`ka` resistance `(ρc/A)(ka)²/4`),
  `σ_k = G φ_k(n)²/2`. Radiation is booked as radiation; the emitted
  coordinates are the radiating volumes' pressures (Pa), non-loading.

## 3. Computing the modes

`S = C^-1/2 K C^-1/2`, `K = Dᵀ L⁻¹ D`, the lowest 24 eigenpairs, then
`φ = C^-1/2 ψ`. Two solvers, both to roundoff:

1. dense: Householder tridiagonalization (reflectors kept), implicit QL for
   all eigenvalues, inverse iteration for the 24 vectors, back-transform
   (`configuration/eigen.rs`; checked against random matrices, the path
   Laplacian's closed form, and degenerate blocks);
2. warm tree path (`configuration/tree_eigen.rs`): the off-diagonal graph is
   a tree, so `S - λI = LDLᵀ` from the leaves has no fill. Each mode is
   refined from the previous point's vector by Rayleigh-quotient iteration;
   accepted only if certified (every residual at `64 ε |S|`, distinct
   eigenvalues, orthonormal after Gram-Schmidt, and a Sylvester inertia
   count proving no lower eigenvalue was skipped); otherwise the dense
   solve runs. Along a 400-step walk that opens a side branch: 400/400
   certified, frequencies equal to the dense solve's within `2.3e-13`
   relative.

Uniform closed-open tube without end correction: the modes are exactly
`ω = (2c/dx) sin(k dx/2)`, `k = (2m+1)π/(2L)` (GARGANTA G1.2's quarter-wave
obligation, with the grid's dispersion as its declared error).

## 4. Runtime and World API

`SpaceBody` (new) runs inside `WorldBody::ModalSpace`. A modal-laws body
runs exactly `Body::process_local`'s sequence. New `World` methods:
`set_body_configuration_point(index, &[f64])`,
`set_body_valve_tuning(index, hz, adduction)`,
`body_configuration_point`, `body_space_observation`, `space_body`,
`disposal_total`. Controls: `ConstitutiveBodyControls::ModalSpace(SpaceBodyControls)`.
`set_body_configuration_index` on an N-D body is refused
(`UnsupportedReconfiguration`).

## 5. Ledger and receipts

A move's `control_work` enters `pending_control_work` and its energy start
`pending_energy_start`, exactly as a legacy reconfiguration's does; its
disposal enters the next frame's internal loss or radiation (two new
pending accumulators, added only when nonzero). By construction
`E_after = E_before + control_work - disposal` (work is computed from the
measured energies), so every frame closes at roundoff.
`SpaceReconfigurationFrame` carries the point before/after, energies,
control work, disposal and its policy, the modal state displacement, and
whether modes were recomputed. `WorldReceipt` is unchanged.

## 6. Planner

`SpaceBodySpec::validate`: coordinates (finite, `lower < upper`, initial
inside, unique whitespace-free names), map/law/connection compatibility,
and a sampled band check: multilinear at every cell's corners and centre;
stiff string at the box's corners (frequencies are monotone in frets and
tension); tube network at the box's corners and centre (all 24 modes in
`(0, 0.48 fs)`, positive T60). Network: tree topology, 24..=128 volumes, at
least one radiating end, every term inside the network and the space. These
are sampled checks, not proofs, like the legacy path's 33 samples.

## 7. Digests and what stays byte-identical

- v2 plans: no new fields, same serialization, same digest.
- v3 plans with `modal` or `local_medium` bodies: same serialization.
- `WorldConfiguration`, `MetricSpec`, `ApertureSpec`: unchanged.
- Hot paths of existing bodies are untouched. Two World-loop additions act
  only on N-D bodies: their radiation writes straight into the World's
  per-body buffer, and their shared-space port exchange is skipped when all
  its weights are exactly zero (every angle is 0; legacy bodies always run
  it). The disposal accumulators are added only when nonzero.
- The connection is part of the physical identity: the same anatomy with a
  different connection has a different plan digest.

Measured (CHECKPOINT-2A G-A1): identical `renderDigest`, `planDigest` and
WAV bytes for the steel-string tour and *Wayfaring Stranger*; a 1-D
two-anchor `modal_space` twin of a legacy body renders bit for bit the same
frames, emissions, states and ledger inside the full steel-string world.

## 8. Observation

`body_configuration_amount` is `None` for N-D bodies;
`body_configuration_point` returns the point. `SpaceBodyObservation`: valve
frame (supply pressure, flow, node pressure, jet speed, losses), fold gap,
radiating-volume pressures and per-end radiation, current frequencies and
T60s. `SpaceBody::network_modes` exposes the computed modes, decay rates and
the shapes at the valve and radiating volumes (non-loading). Trajectory
binding (`TrajectoryBinding`) still binds v2 plans only; binding N-D
coordinates (`body:<id>/config.<name>`) is a follow-on.

## 9. Not done, said plainly

- No exact closure: areas stay in the positive cone (GARGANTA G2; a zero
  area is a rank change with a disposal policy, not implemented).
- The configuration moves in steps (the caller's schedule). The staircase
  is measurable: on an /aiua/ glide, moving every 32 samples raises the
  energy above 6 kHz by 4.2 dB against moving every sample (CHECKPOINT-2A).
  A continuous-in-time transport between computed frames is open.
- Diagonal (modal) losses; no frequency-dependent wall loss beyond what the
  per-mode Rayleigh quotient gives for a frequency-independent `α`.
- Multilinear maps cannot use the mass-momentum connection (anchors carry
  no physical frame).
