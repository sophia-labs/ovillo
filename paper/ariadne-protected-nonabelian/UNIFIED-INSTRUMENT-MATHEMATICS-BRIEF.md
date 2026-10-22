# Unified protected Ariadne — mathematics research brief

**Audience:** topology, geometry, operator theory, control, dynamical systems,
numerical analysis, mathematical acoustics, and formal methods teams
**Date:** 2026-08-11
**Code baseline:** `59035222728154f613a03fccd098e00149b965a5`
(`agent/weave-string-network`)
**Status:** full-strength research assignment, not an implementation sketch
**Primary objective:** make the protected bundle, the work-accounted acoustic
network, and the heard instrument one dynamical object.

All code-line references in this brief refer to the baseline above. Symbol
names are normative when later edits move the lines. The local, untracked
`mambo_hilo` experiment is intentionally excluded from the evidence baseline;
it motivates this assignment but cannot be a dependency until it is reviewed
and versioned.

---

## 1. The mandate

Solve the strong problem. Completion does **not** mean placing a protected
latent controller beside an acoustic renderer, applying its overlap as gain,
or sonifying an independently transported state. Completion means constructing
and validating one persistent, coupled acoustic dynamical system for which all
of the following are simultaneously true:

1. A frozen-configuration conservative family belonging to the complete
   instrument—played strings, body, and spatial/room modes—has an isolated
   rank-two complex spectral subbundle over a declared four-dimensional
   periodic control space.
2. That subbundle has a precisely defined, independently reproducible nonzero
   second Chern invariant, with a certified gap over the complete admitted
   family and perturbation class.
3. Named closed control paths `A` and `B` are paths through that **same
   physical family**. Their Kato/Wilczek–Zee transport is noncommuting, has
   gauge-invariant receipts, and is executed continuously by the production
   state update—not substituted by an unrelated endpoint rotation.
4. The protected modes are acoustically active. They exchange energy
   bidirectionally with string, body, and room state, and the emitted signal is
   radiation from that coupled state. No sidecar scalar gain or post hoc
   oscillator bank is the causal carrier of the result.
5. Source input, actuator work, metric/configuration work, internal loss,
   radiation, state retirement, and numerical residual close one declared
   ledger across the whole instrument.
6. The system remains meaningful under finite speed, damping, drive,
   discretization, host block partition, retuning, and the admitted structural
   perturbations. Every qualification is explicit; no closed-system theorem is
   silently transferred to a driven dissipative render.
7. Non-Abelian transport has a robust audible observable that is not reducible
   to gain, level mismatch, a basis choice, or an accidentally favorable
   pickup. The invariant and the heard transfer response are reported
   separately.
8. The mathematical model, float64 oracle, Rust execution, formal model, and
   audio experiments cross-replay the same content-addressed family, paths,
   schedules, metrics, and receipts.

The desired end statement is therefore stronger than “a protected controller
modulates a synthesizer” and more precise than “topological sound”:

> The coupled Ariadne acoustic family carries a gapped rank-two complex bundle
> with second Chern number one; two same-basepoint physical control cycles
> induce noncommuting, gauge-covariantly transported acoustic states; the
> sample-rate implementation realizes those cycles within certified finite-time,
> work, loss, and numerical budgets; and declared radiation ports preserve an
> experimentally distinguishable order effect.

The individual Wilson matrices remain geometric and path-dependent. A nonzero
`C2` does not quantize them and does not make them fault-tolerant. The team must
retain that distinction even after the strong instrument exists.

---

## 2. What exists, and what does not

The project has unusually strong components. The remaining problem is not a
missing slogan or a missing matrix multiply; it is the mathematical
correspondence among layers.

| Object or claim | Baseline status | Evidence | Missing for the unified instrument |
|---|---|---|---|
| Four-dimensional Wilson–Dirac family | Implemented in the independent Python oracle | [`Dirac4DParameters`, `dirac4d_hamiltonian`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L64-L113) | A realization as the frozen family of the coupled acoustic system |
| Rank-two occupied projector and `C2=1` | Analytic phase label and numerical degree integral agree | [`occupied_projector`, `analytic_second_chern`, `second_chern_degree`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L115-L214) | Certified rather than sampled bounds over the production family and coupling homotopy |
| Closed paths `A`, `B` | Two 193-point smooth torus loops at one base point | [`torus_smooth_loop`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L294-L317), [`verify_protected_nonabelian_instrument`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L747-L901) | A map from physical controls to this torus and proof that the audio operator follows it |
| Kato/Wilson transport | Gauge-free adjacent-projector direct rotations; loop certificates | [`kato_direct_rotation`, `kato_wilson_loop`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L325-L416) | A production path schedule and a correspondence theorem for its live state |
| Finite-time adiabatic replay | Schrödinger replay converges toward the Kato oracle | [`simulate_adiabatic_loop`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L435-L524) | Physical units, sample-rate schedule, coupled-system leakage, and a computable production bound |
| Noncommuting holonomies | `||AB-BA||_F = 1.68764`; an initial-state order distance is `1.19334` | [`results.json`](results.json) | A certified lower bound over the admitted physical perturbations and an acoustic transfer observable |
| Realification and sparse endpoint lowering | Full `U(4)` endpoints become `SO(8)` matrices and compile to 28 path-edge Givens gates | [`realify_unitary`, `lower_unitary_to_acoustic_graph`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L527-L588) | A decision about complex/symplectic physical semantics and a continuous path realization, not endpoint replay only |
| Independent Rust endpoint replay | Dense targets and sparse words agree below `2e-13`; energy is preserved | [`protected_nonabelian.rs`](../../mus-rs/crates/mus-watsn/tests/protected_nonabelian.rs#L149-L187) | Runtime family/path types, scheduled path execution, full-system coupling and receipts |
| Exact graph-constrained rotation compiler | Production Rust emits `OperatorProgram` with deterministic receipts | [`mus-gesture`](../../mus-rs/crates/mus-gesture/src/lib.rs), [`CompileCertificate`](../../mus-rs/crates/mus-gesture/src/compile.rs#L100-L190) | Protected-family IR and a compiler from physical path, not just target `SO(n)` |
| Same-endpoint live path memory | Closed `SO(3)` words execute on live Weave wave variables | [`PHASE2-same-endpoint.md`](../../mus-rs/SPECS/ariadne/PHASE2-same-endpoint.md), [`apply_gestures`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L1648-L1694) | A transported spectral subspace, connection, invariant observable, and adiabatic separation |
| Stateful acoustic kernel | `StringNetworkVoice` is persistent within one event and allocation-free per sample | [`StringNetworkVoice`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L1139-L1204), [`next_sample`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L1746-L1922) | An instrument lifetime beyond a note slot; excitation/release/transfer without reconstructing the body |
| Work-accounted local stages | Exact allpass storage, Givens work, body/scatter/source telemetry | [`mus-watsn`](../../mus-rs/crates/mus-watsn/src/lib.rs), [`EnergyTrace`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L2173-L2314) | Correct moving-delay connection, physical radiation port, and one ledger spanning protected core/string/body/room |
| Time-varying delay state | Exact filter-state repair exists; active-line transport is measured but unsolved | [`NEXT-PHASE-RESULTS.md`](../ariadne-work-holonomy/NEXT-PHASE-RESULTS.md), [`DelayString` open seam](../../mus-rs/crates/mus-dsp/src/pluck.rs#L900-L930) | A metric-consistent, state-faithful fixed-capacity or variable-rank connection |
| Instrument/body/room union | Float64 modal surrogate only | [`EXACT-RESULTS.md`](../ariadne-work-holonomy/EXACT-RESULTS.md#L78-L106) | A production bidirectional graph; current room/rendering paths are not that object |
| Plugin persistence | Each MIDI note owns a live `StringNetworkVoice` across blocks | [`VoiceSlot`, `AriadneEngine`](../../mus-rs/plugins/ariadne-clap/src/lib.rs#L216-L244) | Shared material memory, sample-rate-parametric kernel, note lifecycle, real-time-safe state reuse, physical outputs |
| Formal proof | Exact finite skeletons for work, passivity effects, reachability, braid, and `Z3` pump | [`formal/MusFormal`](../../formal/MusFormal.lean) | Real inner-product spaces, projectors, connections, spectral gaps, realification, error intervals, and Rust IR soundness |

The numeric reference packet currently establishes:

- analytic `C2 = 1` and grid estimate `0.99658810597`;
- base sampled gap `2.01388439764`;
- loop minimum gaps `2.09060500193` and `2.00015916398`;
- occupied unitarity errors below `7.1e-15`;
- non-Abelian commutator norm `1.68764187766`;
- minimum sampled commutator norm `1.31330964709` over twelve declared
  random perturbations;
- endpoint lowering errors `4.26e-15` and `1.08e-14`;
- at dimensionless duration `128`, worst projective error `5.06e-4` and
  leakage `1.98e-2`.

These are excellent oracle results. They do not establish the production
instrument claim because the production acoustic operator is not yet the
operator family whose bundle was certified.

---

## 3. The mathematical object to construct

### 3.1 Frozen conservative family

Let the authored periodic control point be

\[
  \lambda \in \Lambda \cong \mathbb T^4.
\]

The reference family is

\[
  H_D(\lambda)=\sum_{a=1}^{5}d_a(\lambda)\Gamma_a,
\]

with doubly degenerate eigenvalues `±||d||` and occupied projector

\[
  P_D(\lambda)=\frac12\left(I-H_D(\lambda)/\|d(\lambda)\|\right).
\]

The production object must instead define a frozen conservative acoustic
family `K(λ)` on the actual fixed-capacity state used by the instrument. The
team must choose and justify one primary category:

1. a self-adjoint continuous-time generator/Hamiltonian;
2. a skew-adjoint real port-Hamiltonian generator with a compatible complex
   structure;
3. a unitary discrete-time/Floquet step;
4. a lossless paraunitary scattering colligation.

Whichever category is chosen, it must have a well-defined isolated rank-two
complex spectral projector `P(λ)` belonging to the **coupled acoustic state**,
not to an external controller. There must be a gapped homotopy from the
stabilized reference family to the production family:

\[
  K_s(\lambda),\qquad s\in[0,1],
\]

where `K_0` contains `H_D` plus explicitly trivial acoustic bands, `K_1` is the
production conservative family, the selected rank remains two, and the gap
does not close anywhere on `Λ × [0,1]`. This is the preferred route for
transporting the known `C2=1` result into the full instrument.

If the team selects a discrete-time unitary family, it must specify the
quasienergy gap and branch cut and prove that the invariant agrees with the
continuous reference under the chosen discretization. If it selects a
port-Hamiltonian family, it must specify the energy variables, symplectic or
Dirac structure, and the complex spectral construction. “The matrices look
similar” is not a correspondence.

### 3.2 Total live state and energy

The live state must include, at minimum:

\[
  x=(x_{\mathrm{protected}},x_{\mathrm{strings}},x_{\mathrm{filters}},
     x_{\mathrm{body}},x_{\mathrm{room}},x_{\mathrm{contacts}}).
\]

At configuration `λ`, its stored energy is

\[
  E_\lambda(x)=\tfrac12 x^\top M(\lambda)x,
  \qquad M(\lambda)\succ0,
\]

or the corresponding quadratic form in the selected complex representation.
Every coordinate called “normalized” must carry a derivation of its
normalization from physical or declared impedance/storage data.

A provisional normalized discrete-time factorization is

\[
 z_{n+1}=L_n\,C_n\,U_n\,P_n z_n+B_nu_n,
\]

where:

- `P_n` is conservative propagation;
- `U_n` is the protected/path transport at the current control step;
- `C_n` is bidirectional coupling among protected, string, body, and room
  regions;
- `L_n` exposes internal loss and radiation ports rather than hiding them;
- `B_n u_n` is an explicit source injection.

The ordering above is not prescribed; deriving a physically and numerically
sound splitting is part of the assignment. The exact ledger must close

\[
  E_{n+1}-E_n = W_{\mathrm{source},n}+W_{\mathrm{control},n}
  -D_{\mathrm{internal},n}-E_{\mathrm{radiated},n}+r_n.
\]

The current engine's output is an observer tap and therefore is **not** a loss
term. The strong instrument requires a declared physical radiation port or a
clear statement that a particular output remains observation-only. See the
existing distinction in `.string-research/WORK-ACCOUNTED-NETWORKS.md` (private lab notes, not in this repository).

### 3.3 Protection in a driven, lossy instrument

The reference Kato and adiabatic arguments are closed and unitary. The heard
instrument is driven and damped. The team must not define an ad hoc
non-Hermitian topological invariant merely to bridge this gap.

The default strong route is:

1. define topology on the full **conservative acoustic skeleton**;
2. represent damping and radiation through passive ports or a conservative
   dilation;
3. prove or bound how finite coupling to those ports affects the isolated
   subspace and transport over the experimental time horizon;
4. state separately what survives in the reduced open dynamics.

If a non-Hermitian or Lindbladian formulation is ultimately superior, the team
must supply its gap notion, invariant, adiabatic theorem, boundary conditions,
and correspondence to the real sample update. It may not replace the
conservative route by terminology alone.

### 3.4 Persistent material identity

The protected state must survive musical articulation. A note is a source or
boundary event applied to one material body, not construction of a new body.
Mathematically, note segmentation, host block segmentation, and score-event
segmentation must be refinements of the same evolution. If two event lists
describe the same sample-indexed source/control path, they must yield the same
state and audio regardless of segmentation.

This requires a stateful instrument API conceptually closer to

```text
prepare(configuration)
excite(port, event)
set_control(path_sample)
release(port, event)
transfer_thread(from, to, policy)
render_block(output, receipt)
```

than the current per-event constructors called by
[`mus-engine/src/source.rs`](../../mus-rs/crates/mus-engine/src/source.rs#L37-L110).
The mathematics team must define state identity, excitation, release,
retirement, and transfer before engineering chooses the exact Rust API.

---

## 4. Decisions that must be made explicitly

### 4.1 What exactly carries `C2`?

The answer must be “a spectral subbundle of the coupled acoustic conservative
family,” followed by an exact definition. A protected eight-coordinate core
weakly driving an unrelated network is useful as a test harness but is not the
final result. The team must determine whether the protected subspace is:

- a modal subspace of a finite-dimensional reduced string/body/room model;
- a spectral subspace of a fixed-capacity discrete waveguide update;
- a subspace of a unitary colligation including traveling-wave ports; or
- another rigorously related object.

The selected model must admit an implementable projection or basis-free
transport algorithm at audio rate or construction time.

### 4.2 How does the four-torus map to physical controls?

The reference `λ` consists of four periodic Wilson–Dirac momenta. Production
Weave exposes `couple`, `chirality`, `orbit`, `orbit_depth`, `curvature`, body
parameters, delays, and topology. The team must define a smooth periodic map

\[
  F:\mathbb T^4\rightarrow\mathcal C_{\mathrm{physical}}
\]

and the induced frozen acoustic operator. It must be possible to identify the
same base point after a loop exactly, including internal actuator state and
hysteresis. An edge-angle sum of zero is not sufficient if the physical
actuator has memory.

The design should favor controls with physically interpretable periodicity:
phases, orientations, cyclic scattering coordinates, boundary-condition
phases, or synthetic dimensions. If any coordinate is only software-periodic,
say so and prove that the implemented operator is periodic.

### 4.3 Is endpoint realification physically admissible at every intermediate gate?

For `U ∈ U(4)`, the realification

\[
  \mathcal R(U)=\begin{bmatrix}\Re U&-\Im U\\\Im U&\Re U\end{bmatrix}
\]

lies in `SO(8)` and preserves the canonical complex structure. The current
tree-QR compiler decomposes this endpoint into arbitrary real coordinate-plane
Givens factors. Individual factors need not preserve the complex structure or
be symplectic under a quadrature interpretation, even though the product is
orthogonal and has the correct endpoint.

The team must decide which physical semantics apply:

- If the eight coordinates are energy-normalized scattering amplitudes,
  arbitrary orthogonal factors may be lawful; prove their passive port
  realization.
- If they are four oscillator quadrature pairs, require a `U(4)`-compatible
  decomposition—complex Givens/beam-splitter blocks plus phases, or another
  graph-constrained construction—and prove endpoint, locality, and energy
  laws.
- If the production state mixes both interpretations, define the embedding and
  metric explicitly and reject illegal factors at compile time.

This question is load-bearing. The current
[`lower_unitary_to_acoustic_graph`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L556-L588)
proves Euclidean endpoint equivalence, not intermediate physical
realizability.

### 4.4 Is transport geometric compilation or physical adiabatic evolution?

Applying exact Kato direct rotations along the path implements parallel
transport by construction. It is an oracle/counterdiabatic digital operation;
it is not automatically the finite-speed evolution generated by the frozen
Hamiltonian. Conversely, evolving under `H(λ(t))` may approach Kato transport
only in an adiabatic regime and may leak under finite speed and coupling.

The final instrument should support and distinguish both:

1. **compiled geometric mode:** execute a structure-preserving discrete Kato
   connection with explicit lowering residual and control work;
2. **physical adiabatic mode:** evolve under the realized acoustic family with
   duration, gap, leakage, dynamical phase, and discretization receipts.

Their convergence and disagreement are experimental observables. The compiled
mode may be musically preferable, but it must not be reported as emergent
adiabatic transport.

### 4.5 How do topology and the changing delay connection coexist?

The code still changes active delay interpretation without a satisfactory
state correspondence. A scalar energy-normalized remap was tested and rejected
because it preserved one scalar while badly changing state/waveform fidelity.
The strong instrument cannot put topological claims on top of an unresolved
state connection.

For protected paths, prefer a fixed-capacity state bundle if possible. For
general retuning and topology edits, derive either:

- material-coordinate propagation with fixed rank and explicit boundary work;
- a weighted polar/partial-isometry connection with certified approximation;
- or a port-Hamiltonian moving-boundary discretization.

Rank contraction must radiate, dissipate, retire, or reject the discarded
subspace. Rank expansion must initialize new orthogonal state explicitly. The
quantized shadow exists in
[`PassivityTypes.lean`](../../formal/MusFormal/PassivityTypes.lean); the real
theory and Rust correspondence do not.

### 4.6 What is the protected family when the instrument is nonlinear?

The production update is not globally linear. Weave's coupling angle depends
on live course-energy gradients, contact is nonlinear, and the bow is a
state-dependent source. See
[`scatter_weave`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L1527-L1590) and the
source/contact stages in
[`next_sample`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L1746-L1922). Ordinary
band-bundle and Kato theory cannot be applied to that nonlinear map without a
declared reduction.

The team must choose and prove one of the following, or supply a stronger
alternative:

1. topology belongs to the unforced linear conservative skeleton; nonlinear
   stages are passive or work-accounted perturbations whose effect is bounded
   over a declared amplitude domain;
2. topology belongs to a smooth family of linearizations about a specified
   equilibrium or periodic orbit, with uniform rank/gap and a theorem relating
   the linearized transport to finite-amplitude trajectories;
3. a nonlinear cocycle/invariant replaces the spectral-bundle formulation,
   together with an executable definition and a correspondence to the existing
   `C2=1` reference.

The final claim must say which object is protected and what the nonlinear
instrument inherits from it. “Frozen parameters” is not enough when the
frozen update still depends on state.

### 4.7 Can a real, reciprocal, spectrally crowded acoustic graph carry this bundle?

The reference uses a complex `U(4)` family. A classical acoustic state is real,
and an ordinary reciprocal time-reversal-symmetric network may impose
symmetry constraints that forbid the desired characteristic class. A valid
realization therefore needs an explicit complex structure—positive-frequency
modal amplitudes, quadrature pairs, a Floquet construction, or another
declared mechanism—and a symmetry-class analysis.

The team must establish:

- how the complex occupied rank-two bundle is recovered from the real
  acoustic state and how its `C2` is represented after realification;
- which reciprocity, time-reversal, particle-hole, or chiral symmetries the
  physical family has, and which must be broken;
- whether breaking them requires time modulation, nonreciprocal elements, or
  merely ordered digital scattering, with the associated actuator work;
- whether long string and room delays introduce so many eigenvalues or
  quasienergies that a uniform rank-two gap in the **full** graph is impossible;
- if spectral crowding is an obstruction, the precise conservative extension,
  resonance, or reduced-order theorem that still makes the protected modes
  modes of the acoustic instrument rather than an external sidecar.

An impossibility theorem here would be highly valuable. It should force a
specific architectural change, not a retreat to metaphor.

---

## 5. Research work packages

Each work package must return five things: definitions, theorem statements and
proofs or certified bounds, an independent executable oracle, machine-readable
test vectors/receipts, and an engineering-facing failure taxonomy. A prose
memo alone is not a completed package.

### M0 — Claim system and model selection

**Question:** What is the exact category of the production family and what is
the strongest claim supported at each integration stage?

Deliver:

1. A model-selection note comparing self-adjoint modal, real
   port-Hamiltonian, unitary/Floquet, and lossless scattering formulations.
2. A typed claim lattice at least as discriminating as:
   `ordered`, `closed-control`, `geometric`, `gapped-family`,
   `topological-bundle`, `adiabatic-realization`, `protected-acoustic`, and
   `fault-tolerant`.
3. Necessary evidence fields and refusal conditions for each claim.
4. A theorem/measurement/oracle/listener-evidence ledger. These epistemic
   classes must never collapse into one boolean.
5. A final selection of the primary mathematical model and an explicit
   correspondence target in production code.

Start from the existing claim discipline in
[`ARIADNE-IMPLEMENTATION-CONSENSUS.md`](../../ARIADNE-IMPLEMENTATION-CONSENSUS.md)
and the current protected claim boundary in [`THEORY.md`](THEORY.md#11-claim-boundary).

### M1 — Construct the coupled protected acoustic family

**Question:** How can the known `C2=1` Wilson–Dirac bundle be stabilized and
homotoped into an instrument/body/room acoustic family without closing its gap?

Deliver:

1. A concrete frozen family `K(λ)` whose state variables map to implemented
   acoustic regions and ports.
2. A trivial-band stabilization of `H_D`, followed by an explicit coupling
   homotopy to `K`.
3. Analytic sufficient bounds and interval/numerical certification that the
   selected gap remains open over `T4 × [0,1]`.
4. A proof that the selected rank-two bundle of `K` has the same second Chern
   invariant as the reference.
5. A dimensional reduction/model-order argument if the implemented graph is
   larger than the certified family, with a posteriori error bounds.
6. A constructive mapping from the protected modes to played-string, body,
   near-space, and late-room regions. Every coupling must be bidirectional.
7. A proof or counterexample for whether the desired family can live directly
   in the current nine-course Weave topology. If it cannot, specify the
   minimum topology/API change instead of weakening the claim.
8. A symmetry-class and reciprocity analysis, including the mechanism that
   permits nonzero `C2` in a real acoustic implementation.
9. A spectral-crowding analysis for the fixed-capacity string/body/room graph,
   with a proved isolated-subspace construction or an impossibility result.
10. A definition of the protected object in the presence of the engine's
    nonlinear state-dependent scattering and contacts.

The existing implementation handoff proposed four complex modes interpreted as
played string, body, near spatial cycle, and late room/shadow mode; see
[`IMPLEMENTATION-HANDOFF.md`](IMPLEMENTATION-HANDOFF.md#p4--instrumentbodyspace-interpretation).
Treat that as a hypothesis to prove or replace, not a binding physical model.

### M2 — Certified topology and perturbation theory

**Question:** Can the gap, invariant, rank, and non-Abelian separation be
certified over the complete admitted production domain rather than sampled at
twelve random perturbations?

Deliver:

1. An independent second-Chern computation: lattice-gauge, projector-curvature,
   degree, or another method not algebraically identical to the current degree
   integral.
2. Validated numerics or interval bounds for the global gap over the base
   family and the reference-to-production homotopy.
3. A Davis–Kahan/Kato-style projector perturbation bound, specialized to the
   selected family and executable from receipt data.
4. A certified allowed perturbation set for manufacturing/model/parameter
   error, including coupling to trivial bands and passive ports.
5. A lower bound, or a clearly delimited empirical certificate, for the
   noncommutativity measure throughout that set. State explicitly that `C2=1`
   alone does not bound a particular Wilson commutator away from zero.
6. Certified rejection near gap closures and rank changes.
7. Grid-convergence, floating-point, and conditioning analysis for projector,
   Chern, polar, and eigenphase calculations.

Extend, do not merely rerun,
[`second_chern_degree`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L157-L214),
[`sampled_homotopy_gap`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L235-L252),
and the perturbation loop in
[`verify_protected_nonabelian_instrument`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L817-L864).

### M3 — Connection, Kato transport, and gauge invariants

**Question:** What exact discrete and continuous connection is executed, and
which observables survive a change of occupied frame?

Deliver:

1. A proof of the direct-rotation transport law for adjacent projectors under
   explicit overlap hypotheses, including conditioning near the cut locus.
2. Gauge covariance of the discrete path product and conjugacy invariance of
   loop eigenphases, trace, determinant, and commutator observables.
3. Convergence of the discrete Kato product as path resolution increases,
   with an actionable error estimator per path.
4. A path compiler that preserves the complete sequence of local projectors or
   generators, not only the loop endpoint.
5. Exact path-reversal/inversion and concatenation laws under the production
   composition convention.
6. A comparison of direct Kato steps, parallel-transport ODE integration,
   counterdiabatic driving, and physical adiabatic evolution.
7. A definition of base-point/frame calibration for a physical instrument and
   the distinction between gauge-invariant scientific receipts and
   basis-specific musical excitation/pickup choices.

The current oracle is at
[`kato_direct_rotation`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L325-L336)
and [`kato_wilson_loop`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L357-L416).
The current fixture stores only endpoint holonomies and endpoint Givens words;
that is insufficient for this package.

### M4 — Finite-time dynamics and physical units

**Question:** At 48 kHz and at arbitrary host sample rate, what duration and
control bandwidth realize the claimed transport in the coupled acoustic
family?

Deliver:

1. A nondimensionalization connecting the reference Hamiltonian, acoustic
   modal frequencies, seconds, and samples.
2. A quantitative adiabatic error bound using gap, path speed, projector or
   generator derivatives, and coupling/loss terms.
3. A structure-preserving time discretization with global error and host-block
   invariance bounds.
4. Leakage, dynamical-phase, and geometric-endpoint estimators computed from
   production telemetry rather than copied from the oracle.
5. Duration optimization subject to leakage, work, audible bandwidth, and
   actuator-rate budgets.
6. Fast, slow, reversed, paused, and nonuniform path controls, including an
   explicit refusal when a requested duration cannot meet its budget.
7. A sample-rate-parametric formulation; the current fixed-48-kHz kernel and
   wrapper rescaling are not acceptable for the final correspondence.

Build on
[`simulate_adiabatic_loop`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L435-L483),
but recompute the error for the coupled production family.

### M5 — Realification, local factorization, and actuator realizability

**Question:** Which local operations realize the protected transport without
hidden work or violation of the selected complex/port structure?

Deliver:

1. Proofs that realification is a homomorphism, preserves the declared metric,
   and lands in the exact subgroup required by the physical model.
2. A characteristic-class correspondence between the complex occupied bundle
   and its real acoustic representation, including the retained complex
   structure and any Pontryagin/Chern relation used by the proof.
3. A determination of whether the current real `SO(8)` tree-QR factors are
   physically admissible intermediate operations.
4. If not, a graph-constrained `U(4)` or symplectic factor compiler with exact
   endpoint replay, reachability conditions, gate counts, and cost model.
5. An actuator model for each factor: normalized scattering junction,
   complex-mode coupler, boundary motion, or another declared mechanism.
6. Work and bandwidth bounds for time-distributed factors; an instantaneous
   state rotation may be a valid digital scattering operation but is not a
   free physical actuator by assertion.
7. Reachability and obstruction theorems for the actual heterogeneous
   instrument/body/room graph, not only a connected abstract edge graph.
8. A compiler refusal when a mathematically reachable orthogonal target is
   not physically reachable with the declared actuators.

Relevant code:

- [`realify_unitary`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L527-L535);
- [`mus-gesture::compile`](../../mus-rs/crates/mus-gesture/src/compile.rs);
- [`OperatorProgram`](../../mus-rs/crates/mus-watsn/src/program.rs#L29-L84);
- live execution in [`apply_gestures`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L1648-L1694);
- combinatorial reachability skeleton in
  [`Reachability.lean`](../../formal/MusFormal/Reachability.lean).

### M6 — Metric, moving delay, topology edits, and the full work law

**Question:** What connection transports actual recursive acoustic state when
delay, impedance, metric, or graph topology changes?

Deliver:

1. Metric provenance for every stored coordinate in strings, allpasses,
   damping filters, bodies, room delays, contacts, and protected modes.
2. A dense weighted polar/partial-isometry oracle for changing delay state.
3. At least two implementable approximations—fixed-capacity material
   coordinates and a sparse variable-rank method—compared by operator error,
   state fidelity, spectral artifacts, CPU, and ledger closure.
4. A theorem and implementation policy for rank expansion and contraction,
   including radiation, dissipation, retirement, and rejection.
5. A compositional passivity theorem for propagation, protected transport,
   coupling, loss, sources, radiation, and configuration changes.
6. Adversarial automation bounds: frozen stability is insufficient; analyze
   products of time-varying state maps.
7. A final `retune=neutral` contract that repairs active-line state, not only
   first-order allpass state.
8. A proof that the protected family/path remains well-defined under the
   chosen state connection, or an explicit restriction that protected loops
   use fixed rank and fixed material coordinates.

The exact local allpass result is in
[`mus-watsn/src/allpass.rs`](../../mus-rs/crates/mus-watsn/src/allpass.rs).
The rejected scalar-neutral shortcut and remaining problem are documented in
[`NEXT-PHASE-RESULTS.md`](../ariadne-work-holonomy/NEXT-PHASE-RESULTS.md).
The current production ledger is
[`NetworkEnergyAccount`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L1183-L1204)
and [`EnergyTrace`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L2173-L2314).

### M7 — Loss, radiation, contact, and conservative dilation

**Question:** How does the topological conservative family remain scientifically
connected to the lossy, driven, radiating instrument that is heard?

Deliver:

1. A passive port model for damping and radiation, with an optional
   conservative/unitary dilation used for proofs.
2. An exact supply rate for pluck, external drive, breath, bow, moving
   boundaries, and control actuators.
3. A radiation operator that removes the energy it emits, plus a separate
   observer-only pickup for diagnostics.
4. A physically lawful contact model or a stated bounded approximation with a
   complete energy split among stored contact state, heat, and radiation.
5. Perturbation bounds for lossy reduced evolution over finite path duration.
6. A decision on whether any non-Hermitian topology is needed. If pursued,
   provide the full theory and controls; otherwise explicitly bound claims to
   the conservative skeleton plus passive open realization.
7. Silence/decay theorems under source removal for the coupled graph, with no
   hidden recirculating lossless component.

Current source accounting and known contact/bow qualifications live in
[`StringNetworkVoice::next_sample`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L1746-L1922).
The current pure-core shadows are
[`Holonomy.lean`](../../formal/MusFormal/Holonomy.lean) and
[`WorkHolonomy.lean`](../../formal/MusFormal/WorkHolonomy.lean).

### M8 — One instrument/body/room graph

**Question:** What graph and metric make space part of the instrument rather
than downstream reverberation?

Deliver:

1. A bidirectional graph with short string cycles, intermediate body modes,
   and long room/spatial cycles under one update and ledger.
2. A passive bridge/port construction between regions with independently
   recoverable zero-coupling limits.
3. A frozen spectral analysis showing where the protected rank-two subspace
   lives and how much it participates in each region.
4. A proof or bound that room coupling preserves the protected phase over its
   declared domain.
5. Spatial radiation/pickup maps with physical units or declared normalized
   semantics.
6. Late-field feedback into string/body state, demonstrated by causal
   intervention—not correlation in a mixed render.
7. Rank/topology-edit fixtures spanning the regional boundary.
8. A computational plan for long delays that preserves the mathematical
   state connection and real-time constraints.

The intended unified graph law is stated in
[`paper/ariadne-work-holonomy/README.md`](../ariadne-work-holonomy/README.md#central-result),
but only a six-resonator surrogate exists today.

### M9 — Robust acoustic observability and experiment design

**Question:** When does noncommuting protected transport survive excitation,
coupling, loss, and radiation as a reproducible audible distinction?

Let `W_A`, `W_B` be the occupied or full transported operators under the
chosen convention. The team must distinguish:

\[
  D_{\mathrm{inv}}=\|W_AW_B-W_BW_A\|
\]

from a physical transfer measure such as

\[
  D_{C,B}=\|C\,\Phi_{\mathrm{post}}
  (W_AW_B-W_BW_A)\Phi_{\mathrm{pre}}B\|,
\]

where `B` is an admissible excitation map, `C` a radiation/pickup map, and the
`Φ` factors include the surrounding dynamics. Deliver:

1. Gauge-invariant operator receipts: eigenphases, traces, conjugacy class,
   commutator norms, and projector data.
2. Basis-specific physical transfer functions kept explicitly separate.
3. Conditions for a pickup/excitation family to observe the order defect;
   characterize null spaces and blind pickups.
4. Robust lower bounds or worst-case optimization over admitted perturbations,
   initial states, pickups, coupling, and loss.
5. Inverse design of paths, excitations, and pickups maximizing robust
   distinction subject to gap, duration, work, bandwidth, and loudness
   constraints.
6. Controls: identity, commuting, inverse, orientation reversal, trivial phase,
   near-gap failure, zero coupling, one-way coupling, and matched non-topological
   transformations.
7. Level-, spectrum-, and duration-matched stimuli followed by preregistered
   discrimination and musical-control studies. Waveform inequality is not a
   listener result.
8. A demonstration that the effect persists without scalar gain modulation or
   downstream convolution.

The current state- and pickup-dependent distinction begins at
`.string-research/ARIADNE-MATHEMATICS.md` §6, "Discrete control holonomy" (private lab notes, not in this repository).
It must be lifted to the protected coupled family.

### M10 — Persistent performance and score semantics

**Question:** How are protected paths, sources, note events, and material
memory represented so musical segmentation cannot reset or counterfeit the
physics?

Deliver:

1. A mathematical state-machine semantics for prepare, excite, sustain,
   control path, release, thread transfer, retirement, and reset.
2. A segmentation invariance law for host blocks and semantically equivalent
   score-event partitions.
3. Policies for polyphony: one shared body with multiple excitation ports,
   multiple protected bodies, or a declared hybrid. Voice stealing may not
   silently destroy protected/material state.
4. A note-off/release law and state-retirement energy receipt.
5. A definition of “one instrument” precise enough for offline MUS rendering
   and CLAP execution to share it.
6. A digest identity for the persistent initial state, family, path, source
   schedule, and render segmentation.
7. A score surface for named protected families and paths only after the IR and
   refusal semantics are stable.

Current fresh-event construction is visible in
[`mus-engine/src/source.rs`](../../mus-rs/crates/mus-engine/src/source.rs#L37-L110),
the probe service constructs another fresh voice at
[`mus-cli/src/service.rs`](../../mus-rs/crates/mus-cli/src/service.rs#L300-L329),
and the plugin allocates one body per note at
[`ariadne-clap/src/lib.rs`](../../mus-rs/plugins/ariadne-clap/src/lib.rs#L286-L340).

### M11 — Formal soundness and validated numerics

**Question:** Which parts can be proved kernel-checkably, and how are the
floating implementation and interval certificates connected to those proofs?

Deliver:

1. A deliberate decision whether to add mathlib. The present zero-dependency
   Lean package cannot conveniently carry the required real spectral theory;
   preserving zero dependencies is not a scientific requirement.
2. Formal definitions and proofs, in a sensible dependency order, for:
   - finite-dimensional inner-product and metric spaces;
   - orthogonal/unitary realification and composition;
   - Givens energy preservation and program composition;
   - projectors, direct rotation, discrete parallel transport, and gauge
     covariance;
   - trace/eigenvalue or characteristic-polynomial loop invariants;
   - work-ledger composition and passive ports;
   - partial-isometry disposal laws;
   - claim/refusal soundness for the protected IR.
3. Clearly scoped analytic results left outside Lean, with checked hypotheses
   and machine-verifiable certificates.
4. Interval or ball arithmetic for gap, projector, holonomy, reconstruction,
   and residual bounds.
5. A refinement/conformance theorem or generated proof obligations connecting
   the Rust IR fields to the formal objects.
6. No `sorry`, no theorem inferred from a float test, and no float test omitted
   because a theorem exists.

Existing pure-core modules are intentionally modest:
[`ProtectedNonAbelian.lean`](../../formal/MusFormal/ProtectedNonAbelian.lean)
proves only finite `Z3` pump algebra;
[`Braid.lean`](../../formal/MusFormal/Braid.lean) proves a signed-axis quotient;
[`Reachability.lean`](../../formal/MusFormal/Reachability.lean) proves the graph
skeleton; [`PassivityTypes.lean`](../../formal/MusFormal/PassivityTypes.lean)
uses natural-number energy. None currently proves the strong result.

### M12 — Inverse design, optimization, and impossibility results

**Question:** Given the complete physical and protection constraints, which
families and loops are actually buildable, robust, audible, and efficient?

Deliver:

1. A constrained inverse problem over family parameters, coupling topology,
   loops, schedules, excitations, and pickups.
2. Objectives that separate invariant robustness, acoustic observability,
   actuator work, leakage, path time, spectral quality, and numerical cost.
3. Exact constraints first; optimization may never repair an invalid gap,
   endpoint, passivity law, or claim level.
4. Adjoint/differentiable methods or certified search where appropriate, with
   independent replay of every accepted candidate.
5. Lower bounds and impossibility certificates: insufficient graph
   controllability, unavoidable pickup blindness, gap/work tradeoffs,
   dimension/rank obstructions, and bandwidth limits.
6. A comparison between optimized full physical paths and the current
   endpoint `SO(8)` lowering.
7. A final selected family and `A/B` pair justified by robust—not nominal—audio
   performance.

This package consumes M1–M9. It should not start by optimizing the existing
sidecar or endpoint-only construction.

---

## 6. Minimum theorem and certificate obligations

The final theory packet must state and discharge, or explicitly refute and
replace, at least the following obligations.

1. **Frozen-family well-posedness.** `K(λ)` is in the selected operator class
   and depends on `λ` with the regularity required by the invariant and
   transport theorem.
2. **Uniform spectral isolation.** The chosen rank-two projector exists over
   all of `T4` and the admitted reference-to-production homotopy.
3. **Invariant transport.** The production bundle has `C2=1` under a declared
   convention independently checked by two methods.
4. **Coupling stability.** The protected phase persists under the actual
   string/body/room coupling and admitted parameter uncertainty.
5. **Connection correctness.** The compiled discrete transport carries
   `P_n` to `P_{n+1}`, composes in the pinned order, and converges to the chosen
   continuous connection.
6. **Gauge covariance.** Changing local occupied frames conjugates Wilson
   matrices and leaves declared invariant receipts unchanged.
7. **Non-Abelian witness.** `A` and `B` do not commute, with a certified
   lower bound over the admitted perturbation class or a precisely labeled
   empirical domain.
8. **Physical-path closure.** Every external and internal control coordinate,
   including hysteretic state, returns to the same base configuration.
9. **Finite-time realization.** Leakage and endpoint error meet a bound derived
   from the executed family, path, duration, and discretization.
10. **Local-factor legality.** Every compiled factor is realizable by a named
    acoustic element/port and satisfies its metric/work law.
11. **Whole-step passivity/accounting.** The complete source/control/loss/
    radiation ledger closes, including configuration changes and delay state.
12. **Persistent-state equivalence.** Host-block and score segmentation do not
    change evolution; reset and retirement are explicit state operations.
13. **Open-system correspondence.** The reduced lossy render is connected by a
    theorem or certified approximation to the conservative protected family.
14. **Observable transfer.** The chosen excitation/radiation maps have a
    nonzero, robust AB/BA transfer distinction not attributable to gain.
15. **Numerical certification.** Every matrix, projector, gap, schedule, and
    receipt error is bounded with conditioning and precision recorded.
16. **Implementation refinement.** Python, Rust, formal certificates, and
    production telemetry name the same objects and composition convention.
17. **Real/complex correspondence.** The real acoustic state carries the
    complex structure and characteristic class claimed by the reference
    bundle.
18. **Symmetry admissibility.** Reciprocity/time-reversal constraints and any
    required symmetry breaking are derived for the implemented family.
19. **Nonlinear inheritance.** The relationship between the protected linear
    object and finite-amplitude nonlinear performance is proved or bounded on
    a declared domain.

A failure of one obligation is not necessarily project failure. It must return
an impossibility result or a changed model—not a softer adjective.

---

## 7. Protected path fixture and receipt v2

The existing fixture contains endpoint holonomies and 28-gate endpoint
lowerings. The next fixture must make the path and physical correspondence
replayable. At minimum it must contain:

```text
schema/version
oracle/code/environment digests

family
  identifier and content digest
  operator category and complex-structure convention
  T4 coordinate convention and physical-control map
  full frozen-family parameters
  energy metric provenance
  protected rank and projector convention
  C2 convention, independent estimates, certified error
  global gap bound and proof/certificate method
  admitted perturbation/homotopy family

path A/B
  exact base point and closure rule
  control points or analytic path
  physical actuator state at every endpoint
  sample/time schedule and units
  local projectors/generators or their digests
  adjacent overlaps and gap samples/bounds
  Kato steps and/or physical propagators
  occupied and full holonomies
  gauge-invariant eigenphases/traces/commutators
  path reversal and concatenation witnesses

lowering
  graph and port identifiers
  coordinate/complex-structure map
  local executable factors with sample times
  endpoint, per-step, orthogonality/unitarity/symplectic residuals
  actuator work and bandwidth predictions

production execution
  initial persistent-state digest
  sources, releases, and control schedule
  measured gap/leakage proxies
  source/control/loss/radiation/state-retirement ledger
  numerical residual and worst local residual
  host-block partition and block-invariance digest
  full-state or checkpoint digests sufficient for replay

observability
  excitation and radiation/pickup operators
  invariant AB/BA distance
  physical transfer distance
  RMS/peak/loudness/spectral controls
  perturbation and listener-study identifiers

claim
  exact promoted level
  satisfied gates
  withheld stronger claims and reasons
```

The compiler must refuse missing conventions, an unverified gap, illegal
factors, unsupported duration, a path leaving the family, unmatched base
configuration, absent disposal policy, or a claim stronger than its evidence.

Suggested ownership, extending the existing boundary:

- `mus-gesture`: protected family/path IR, invariant/gap/adiabatic compiler
  receipts, graph-constrained lowering;
- `mus-watsn`: fixed-capacity normalized state, executable programs, passive
  ports, transports, work/loss/radiation ledger;
- `mus-dsp`: physical string/body/room realization and production telemetry;
- Python: independent dense/topological/validated oracle and fixture generator;
- Lean: specification and certificate soundness;
- `mus-engine`/MUS: persistent performance scheduling and content-addressed
  render receipt, after the above contracts stabilize.

Do not generate production constants silently at runtime. Fixtures and family
tables must be content-addressed, reviewable, and independently reproducible.

---

## 8. Required experimental matrix

Every row below must report invariant/operator, live state, ledger, and audio
results where applicable.

| Experiment | Purpose | Required result |
|---|---|---|
| `A`, `B`, `AB`, `BA` | Primary non-Abelian witness | Same base configuration; invariant and physical order distances nonzero |
| `A A⁻¹`, `B B⁻¹` | Inversion | Operator and live protected state return within certified tolerance; ledger explains loss/source effects |
| Identity path | Zero limit | Byte- or tolerance-inert according to the exact declared path representation |
| Commuting/disjoint path | Negative control | Commutator invariant and controlled audio contrast vanish |
| Same endpoint, non-geometric target word | Confound control | Distinguish generic ordered scattering from protected-family transport |
| Trivial `C2=0` family | Topology control | Similar local spectra where possible, but invariant receipt is trivial |
| Critical gap closing | Refusal control | Compiler rejects protected claim/path; conditioning failure is explicit |
| Gauge/frame change | Invariance | Wilson matrices conjugate; eigenphase/trace/commutator receipts agree |
| Path-resolution sweep | Discrete connection | Kato/endpoint error converges at predicted rate |
| Duration sweep | Adiabaticity | Production leakage/error follows a derived bound or exposes model failure |
| Coupling sweep | Full-family robustness | Gap, participation, commutator, and audio remain within certified domain; threshold is identified |
| Loss/radiation sweep | Open realization | Conservative invariant remains fixed; reduced transfer degradation is quantified |
| Perturbation corpus | Protection | Certified or validated gap and invariant persist; worst-case noncommutativity reported separately |
| Pickup/excitation sweep | Observability | Blind subspaces identified; selected ports have robust lower bound |
| Host block partitions | Runtime semantics | Identical state/audio/receipt for all legal partitions |
| Score segmentation variants | Persistent identity | Equivalent source/control timelines produce identical evolution |
| Retuning reversal/rate sweep | State connection | No unexplained work or drift; state fidelity and sidebands reported separately |
| Rank expansion/contraction | Topology edits | Every removed degree is radiated, dissipated, retired, or rejected |
| Room bridge zero/one-way/two-way | Coupling causality | Zero recovers limits; one-way cannot mimic two-way intervention; feedback is measurable |
| No scalar gain modulation | Causal integrity | Protected order effect remains audible through state/radiation alone |
| Matched non-topological surrogate | Specificity | Listener/descriptor performance exceeds an equally complex nonprotected control if such a claim is made |

The exact `Z3` pump remains a valuable separate algebraic and acoustic control.
Do not use it as evidence for the Wilson–Dirac bundle unless a new theorem
connects the two constructions.

---

## 9. Acceptance gates

### 9.1 Geometry and topology

- The full coupled conservative family, not merely a subroutine, is defined
  over an exact periodic `T4` control space.
- Protected rank is two everywhere in the declared domain.
- The gap has a certified positive lower bound over family and coupling
  homotopy.
- The real/complex structure and acoustic symmetry class permit the claimed
  invariant; required symmetry breaking is explicit and work-accounted.
- Spectral crowding from string/body/room state does not silently destroy the
  isolated subspace.
- `C2=1` is reproduced by two independent methods within declared error.
- Trivial and critical controls behave correctly.
- Wilson loop invariants are gauge-covariant/invariant as appropriate.
- AB/BA noncommutativity is bounded over the admitted perturbation set.
- The nonlinear operating domain and its relation to the protected
  conservative family are explicit.

### 9.2 Dynamics and lowering

- The complete local path—not just its endpoint—is available for replay.
- Compiled Kato and physical adiabatic modes are distinguished and compared.
- Every local factor is on an authored physical edge/port and belongs to the
  lawful transformation group.
- Dense oracle, path compiler, Rust executor, and live acoustic state agree
  within certified bounds.
- Host sample rate and block partition do not alter the mathematical path.
- Path inversion, concatenation, and identity laws pass in the live system.

### 9.3 Energy and open-system behavior

- One metric/port ledger spans protected modes, strings, filters, body, room,
  contacts, sources, controls, loss, and radiation.
- The moving-delay connection is solved; `neutral_filters` alone is not enough.
- No observer perturbs audio; every physical output removes or accounts for
  the energy it radiates.
- Bow, breath, external drive, pluck, and moving controls have explicit supply
  rates.
- Rank edits cannot compile without a disposal policy.
- Under source removal the declared passive domain decays/silences with a
  proved or certified bound.

### 9.4 Acoustic unity and persistence

- Protected modes participate nontrivially in string, body, and room regions.
- Energy can travel both directions across each claimed regional bridge.
- A causal intervention on room state changes later instrument state.
- Musical notes excite/release one persistent body; event or block
  segmentation cannot reset it.
- The primary AB/BA audio does not rely on a sidecar gain, independent
  oscillator sonification, or post-processing room convolution.

### 9.5 Observability and evidence

- Scientific invariant receipts and basis-specific audio transfer receipts are
  both present and never conflated.
- Excitation and pickup blind spots are characterized.
- AB/BA stimuli are level, duration, and relevant spectral descriptors matched
  without a master limiter.
- Identity, commuting, trivial-family, non-topological, and near-gap controls
  are included.
- Audibility claims require preregistered listener evidence; waveform
  inequality alone supports only deterministic distinguishability.

### 9.6 Formal and reproducibility

- Formal modules build without admissions.
- Analytic assumptions appear as explicit certificate fields or proof
  hypotheses.
- Validated numerics cover every global bound used to promote a claim.
- Python and Rust independently replay committed fixtures.
- Receipts are deterministic, content-addressed, schema-versioned, and reject
  unknown stronger claims.
- The full optimized CI/test battery is green; profile-sensitive byte witnesses
  are either pinned to the intended profile or replaced by a profile-stable
  law.

No single WAV, endpoint matrix, numerical Chern estimate, Lean toy theorem, or
closed energy scalar can substitute for this conjunction.

---

## 10. Language discipline

Until all strong gates pass, use exact terms:

- **ordered-scattering path memory** for current Weave control order effects;
- **closed-control monodromy** for current same-endpoint production words;
- **Kato/Wilson holonomy of the reference family** for the existing protected
  oracle;
- **causally coupled protected-core prototype** only if an intermediate core is
  embedded but the full acoustic spectral bundle is not yet established;
- **protected acoustic instrument** only after the coupled-family, gap,
  transport, work, and observability gates above pass.

Never infer:

- quantization of an individual Wilson matrix from `C2=1`;
- fault tolerance from a gapped topological phase;
- geometric transport from noncommutativity alone;
- physical adiabaticity from exact endpoint replay;
- passivity of the whole render from orthogonal local factors;
- physical state fidelity from scalar energy closure;
- acoustic protection from modulation by a protected sidecar;
- audibility from a nonzero matrix or waveform norm;
- room participation from downstream convolution.

The final paper may say that the bundle is topologically protected while its
individual non-Abelian Wilson gates are geometric and continuously variable.
That is not moderation; it is the actual stronger result.

---

## 11. Code and document map

### Binding architecture and open phases

- [`ARIADNE-IMPLEMENTATION-CONSENSUS.md`](../../ARIADNE-IMPLEMENTATION-CONSENSUS.md)
  — module ownership, scientific decisions, and phases 0–6. At this baseline,
  phases 0–2 are represented in production; the full delay connection,
  instrument–room graph, and protected fusion remain open.
- [`paper/ariadne-protected-nonabelian/IMPLEMENTATION-HANDOFF.md`](IMPLEMENTATION-HANDOFF.md)
  — earlier P0–P6 protected-production ladder. This brief strengthens its end
  condition from “protected core coupled to sound” to “protected spectral
  bundle of the coupled acoustic family.”
- [`paper/ariadne-work-holonomy/README.md`](../ariadne-work-holonomy/README.md)
  — metric/polar/work framework and the one-graph instrument/body/room thesis.
- [`paper/ariadne-work-holonomy/NEXT-PHASE-RESULTS.md`](../ariadne-work-holonomy/NEXT-PHASE-RESULTS.md)
  — exact filter transport and the failed scalar-normalized delay shortcut.

### Independent topology and transport oracle

- [`protected.py:64`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L64-L105)
  — deformed Wilson–Dirac parameter family.
- [`protected.py:107`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L107-L129)
  — Hamiltonian and occupied projector.
- [`protected.py:131`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L131-L214)
  — analytic and numerical second Chern computations.
- [`protected.py:235`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L235-L252)
  — sampled homotopy gap.
- [`protected.py:294`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L294-L317)
  — smooth same-basepoint torus paths.
- [`protected.py:325`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L325-L416)
  — Kato direct rotation and loop product.
- [`protected.py:435`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L435-L524)
  — finite-time Schrödinger replay.
- [`protected.py:527`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L527-L588)
  — realification and endpoint graph lowering.
- [`protected.py:747`](../ariadne-gesture-compiler/reference/ariadne_gesture/protected.py#L747-L901)
  — complete current reference experiment and perturbation corpus.
- [`run_experiments.py:83`](run_experiments.py#L83-L142)
  — current audible reference: transported complex state observed by a fixed
  oscillator bank. It is deliberately not a coupled acoustic realization.
- [`fixture.json`](fixture.json), [`results.json`](results.json), and
  [`artifacts/MANIFEST.sha256`](artifacts/MANIFEST.sha256) — current immutable
  endpoint/reference evidence.

### Compiler and law executor

- [`mus-watsn/src/program.rs:6`](../../mus-rs/crates/mus-watsn/src/program.rs#L6-L84)
  — `PlaneRotation` and `OperatorProgram`, the only low-level factor executor.
- [`mus-watsn/src/ledger.rs:4`](../../mus-rs/crates/mus-watsn/src/ledger.rs#L4-L69)
  — `WorkFrame` and `WorkLedger`.
- [`mus-watsn/src/allpass.rs`](../../mus-rs/crates/mus-watsn/src/allpass.rs)
  — exact normalized allpass storage and neutral coefficient transport.
- [`mus-watsn/src/delay.rs`](../../mus-rs/crates/mus-watsn/src/delay.rs)
  — periodic remap experiments; scalar-neutrality is not final state
  correspondence.
- [`mus-gesture/src/lib.rs`](../../mus-rs/crates/mus-gesture/src/lib.rs)
  — dependency and composition conventions.
- [`mus-gesture/src/compile.rs:100`](../../mus-rs/crates/mus-gesture/src/compile.rs#L100-L190)
  — exact graph-constrained certificate and executable program.
- [`mus-gesture/src/closed_loop.rs`](../../mus-rs/crates/mus-gesture/src/closed_loop.rs)
  — exact closed-control `SO(3)` construction, not a protected-family path.
- [`mus-gesture/src/receipt.rs:88`](../../mus-rs/crates/mus-gesture/src/receipt.rs#L88-L230)
  — current deterministic compiler receipt; extend rather than fork it.
- [`mus-gesture/src/work.rs`](../../mus-rs/crates/mus-gesture/src/work.rs)
  — measured fixed-metric work through the WATSN executor.

### Production acoustic kernel and lifecycle

- [`mus-dsp/src/pluck.rs:1139`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L1139-L1204)
  — live state fields and current network accounting.
- [`mus-dsp/src/pluck.rs:1527`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L1527-L1590)
  — ordinary Weave ring scattering.
- [`mus-dsp/src/pluck.rs:1592`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L1592-L1694)
  — gesture validation and explicit Givens execution on course waves.
- [`mus-dsp/src/pluck.rs:1696`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L1696-L1744)
  — body-mode coupling pattern; useful precedent for energy-preserving
  heterogeneous coupling.
- [`mus-dsp/src/pluck.rs:1746`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L1746-L1922)
  — complete sample ordering, source injection, body coupling, contact, and
  output tap.
- [`mus-dsp/src/pluck.rs:1924`](../../mus-rs/crates/mus-dsp/src/pluck.rs#L1924-L2047)
  — block rendering, retargeting, and gestured render entry point.
- [`mus-dsp/tests/phase0_telemetry.rs`](../../mus-rs/crates/mus-dsp/tests/phase0_telemetry.rs)
  — observer byte law and current whole-network energy measurements.
- [`mus-dsp/tests/phase2_same_endpoint.rs`](../../mus-rs/crates/mus-dsp/tests/phase2_same_endpoint.rs)
  — live closed-control monodromy gates and controls.
- [`mus-dsp/tests/e15_protocol.rs`](../../mus-rs/crates/mus-dsp/tests/e15_protocol.rs)
  — current same-control-endpoint path experiment and its explicit geometric
  limitations.
- [`mus-engine/src/source.rs:37`](../../mus-rs/crates/mus-engine/src/source.rs#L37-L110)
  — per-score-event fresh source/voice construction.
- [`mus-cli/src/service.rs:143`](../../mus-rs/crates/mus-cli/src/service.rs#L143-L329)
  — probe API, trajectory/gesture parsing, and another fresh voice boundary.
- [`ariadne-clap/src/lib.rs:216`](../../mus-rs/plugins/ariadne-clap/src/lib.rs#L216-L384)
  — plugin voice lifetime and known real-time/sample-rate limitations.

### Formal specification

- [`formal/MusFormal/ProtectedNonAbelian.lean`](../../formal/MusFormal/ProtectedNonAbelian.lean)
  — exact finite `Z3` pump only.
- [`formal/MusFormal/WorkHolonomy.lean`](../../formal/MusFormal/WorkHolonomy.lean)
  — work telescoping and finite noncommuting zero-work witnesses.
- [`formal/MusFormal/PassivityTypes.lean`](../../formal/MusFormal/PassivityTypes.lean)
  — quantized supply/disposal effect calculus.
- [`formal/MusFormal/Reachability.lean`](../../formal/MusFormal/Reachability.lean)
  — graph/bracket combinatorial skeleton.
- [`formal/MusFormal/Braid.lean`](../../formal/MusFormal/Braid.lean)
  — signed-axis braid quotient, not topological protection.
- [`formal/MusFormal/Holonomy.lean`](../../formal/MusFormal/Holonomy.lean)
  — quantized decay skeleton.

### Literature and novelty boundary

The three `.string-research/` notes below are private lab notes, not in this repository.

- `.string-research/LITERATURE.md` —
  waveguides, passive bodies, lossless networks, non-Abelian acoustic
  transport, and immediate reading order.
- `.string-research/LITERATURE-ROUND-2.md`
  — time-varying work, port-Hamiltonian systems, scattering/FDNs,
  differentiable design, and claim matrix.
- `.string-research/ACADEMIC-THESIS.md`
  — proposed WATSN contribution and required instrument–room evidence.

The mathematics team should update the bibliography with primary sources for
the selected operator category, adiabatic theorem, validated Chern
computation, passive realization, and acoustic non-Abelian transport. Novelty
claims require a separate, explicit search; mathematical completion does not
by itself establish priority.

---

## 12. Required return package

The team should return one coherent package, even if work is divided among
specialists:

1. **`UNIFIED-INSTRUMENT-THEORY.md` or manuscript source** — definitions,
   theorems, proofs, model selection, claim boundaries, and exact correspondence
   to code.
2. **Independent oracle extensions** — full coupled family, validated gap and
   invariant computation, continuous paths, finite-time evolution, coupling,
   observability, and inverse design.
3. **Protected fixture v2** — complete path/family/metric/physical-control
   artifacts with hashes, not endpoint matrices alone.
4. **Formal package** — real-valued spectral/metric/transport core at the level
   judged feasible, with explicit certificate interfaces for remaining
   analysis.
5. **Rust API contract** — protected family/path types, legal factor group,
   passive port/state connection requirements, receipt schema, and typed
   refusals. This is an engineering specification precise enough to implement
   without inventing mathematics in the callback.
6. **Acceptance suite** — Python, Rust, formal, and audio fixtures for every row
   in §8 and gate in §9.
7. **Production experiment protocol** — persistent one-body performance,
   AB/BA and controls, perturbation sweeps, full ledger, and listener protocol.
8. **Negative-results ledger** — every attractive but invalid shortcut retained
   with a counterexample or failed gate.
9. **Claim promotion table** — exact statements that become defensible, exact
   statements still withheld, and the evidence class for each.
10. **Implementation handoff** — ordered commits/work packages, ownership
    boundaries, expected complexity, and failure recovery.

Every generated numeric result must record precision, library versions, random
seeds, tolerances, conditioning indicators, and source digests. Every global
claim must use validated numerics or an analytic bound, not an unqualified
grid minimum.

---

## 13. Suggested dependency order

The work is large, but its dependency graph is sharp:

1. **M0:** select the operator category and claim semantics.
2. **M1 + M5:** construct the coupled family and decide the lawful local
   transformation group.
3. **M2 + M3:** certify the bundle, connection, path invariants, and robust
   noncommutativity.
4. **M4:** derive physical time and structure-preserving sample execution.
5. **M6 + M7 + M8:** solve state connection, passive open realization, and the
   instrument/body/room graph.
6. **M9:** design robust excitation/radiation and the complete control matrix.
7. **M10:** define persistent musical semantics and score/host equivalence.
8. **M11:** formalize throughout, then close the Rust/certificate refinement
   layer after the IR stabilizes.
9. **M12:** optimize only the fully constrained object.

M2, M6, and M11 should begin early enough to expose impossibility, but no team
should optimize or sonify a construction whose operator category and physical
state meaning remain undecided.

---

## 14. Explicit non-solutions

The following may be useful intermediate fixtures. None completes this brief:

- a `BundleOrbit` that changes gain on separately rendered notes;
- fresh protected/string state for every note or probe;
- applying the 28-gate endpoint word without executing the family path;
- claiming adiabaticity from a slow animation rather than a bound;
- embedding a protected core that only drives, but is not driven by, the
  acoustic body;
- treating convolution or a downstream FDN as the room;
- preserving `sum(x²)` while changing the musically relevant state direction;
- proving `SO(8)` endpoint equivalence while ignoring intermediate actuator
  legality;
- sampling a dozen perturbations and calling the full domain certified;
- using `C2=1` as a lower bound on a particular Wilson commutator;
- reporting output waveform difference as a gauge-invariant observable;
- using the master limiter, normalization, or gain matching inside the energy
  proof;
- proving a finite combinatorial shadow in Lean and transferring it to real
  floating DSP without a refinement argument;
- resolving the topology while leaving the production delay-state connection
  and regional graph outside the model.

The goal is not to make the poetry more cautious. It is to make the complete
instrument strong enough that the poetry becomes a literal description of its
causal structure.

---

## 15. Baseline replay commands

Run from the repository root unless noted:

```bash
# Independent protected reference packet
ARIADNE_PACKET_OUT="$(mktemp -d)"
uv run --no-project --python 3.12 --with numpy --with scipy \
  python paper/ariadne-protected-nonabelian/run_experiments.py \
  --quick --no-audio --output "$ARIADNE_PACKET_OUT"

# Exact gesture compiler oracle
uv run --no-project --python 3.12 --with numpy --with scipy \
  python paper/ariadne-gesture-compiler/reference/run_experiments.py \
  --quick --no-write

# Rust protected endpoint replay
cargo test --release --manifest-path mus-rs/Cargo.toml \
  -p mus-watsn --test protected_nonabelian -- --nocapture

# Compiler and current live closed-control gates
cargo test --release --manifest-path mus-rs/Cargo.toml \
  -p mus-gesture --test acceptance_gates -- --nocapture
cargo test --release --manifest-path mus-rs/Cargo.toml \
  -p mus-dsp --test phase2_same_endpoint -- --nocapture
cargo test --release --manifest-path mus-rs/Cargo.toml \
  -p mus-dsp --test phase0_telemetry -- --nocapture

# Formal baseline
cd formal
~/.elan/bin/lake build
```

The final return package must add one command that regenerates the complete
coupled-family fixture and one optimized test target that independently replays
it through the production instrument.
