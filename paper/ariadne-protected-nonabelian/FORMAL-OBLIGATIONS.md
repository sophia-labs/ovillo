# Formal obligations for the unified protected instrument

## Status

The executable packet supplies exact finite algebra and floating certificates,
but it does not claim a completed Lean proof. The existing zero-dependency
formal package is too weak for real/complex spectral theory. The recommended
path is to add mathlib and formalize the finite-dimensional core while treating
global analytic and interval computations as checked certificates with
explicit hypotheses.

No theorem below may be discharged by a float test. No float test may be
removed merely because a theorem exists.

## 1. Dependency decision

Adopt mathlib. Required areas include:

- finite-dimensional real and complex inner-product spaces;
- matrices, adjoints, unitary and orthogonal operators;
- spectral projectors for Hermitian matrices;
- determinants, traces, characteristic polynomials;
- positive-definite square roots and polar decomposition;
- normed-space perturbation bounds;
- differential forms or a deliberately scoped characteristic-class interface.

A zero-dependency implementation would spend effort rebuilding foundations and
would not improve the scientific claim.

## 2. Proposed module order

```text
MusFormal/Unified/ComplexStructure.lean
MusFormal/Unified/Realification.lean
MusFormal/Unified/UnitaryProgram.lean
MusFormal/Unified/PassivePort.lean
MusFormal/Unified/WorkLedger.lean
MusFormal/Unified/PartialIsometry.lean
MusFormal/Unified/ProjectorTransport.lean
MusFormal/Unified/GaugeCovariance.lean
MusFormal/Unified/DiracFamily.lean
MusFormal/Unified/ClaimSoundness.lean
MusFormal/Unified/FixtureV2.lean
```

The first six modules should be completed before attempting the full bundle
statement.

## 3. Core definitions

### 3.1 Complex structure on a real space

For a real inner-product space `V`, define a compatible complex structure `J`:

```text
J ∘ J = -I
J† = -J
```

Define the lawful real subgroup:

```text
OrthogonalComplex(V,J) = {Q | Q†Q=I and QJ=JQ}.
```

Prove equivalence, in finite dimensions, with the realification of a unitary
operator on the induced complex space.

### 3.2 Realification

For a complex matrix `U=A+iB`, define

```text
R(U) = [[A,-B],[B,A]].
```

Required theorems:

```text
realify_id
realify_mul
realify_star
realify_unitary_is_orthogonal
realify_unitary_commutes_complexStructure
realify_unitary_is_symplectic
realify_injective
```

Also prove a counterexample theorem: for real dimension at least four, an
orthogonal matrix need not commute with the canonical complex structure. This
formalizes the refusal of arbitrary `SO(2n)` lowering.

### 3.3 Complex two-mode gate

Define an embedded `U(2)` block and local phase. Prove:

```text
embedded_twoMode_unitary
embedded_twoMode_support
embedded_phase_unitary
program_fold_unitary
program_fold_realification_lawful
program_inverse
program_concat
```

The executable schedule convention must be pinned as left multiplication in
listed execution order.

## 4. Complex tree compiler

The Python compiler uses leaf elimination on a connected spanning tree.
Formalization may separate algebraic correctness from executable search.

### 4.1 Elimination block

For `a,b : ℂ` and `r=sqrt(|a|²+|b|²)>0`, define

```text
G(a,b) = [[conj a/r, conj b/r],[-b/r,a/r]].
```

Prove:

```text
eliminationBlock_unitary
eliminationBlock_det_one
eliminationBlock_hits_axis
```

Handle `r=0` by identity and prove the branch is lawful.

### 4.2 Tree elimination

Required theorem shape:

```text
connected graph ->
valid spanning tree ->
U : unitary n ->
compiler returns program p ->
all two-mode factors lie on graph edges /\
programMatrix p = U.
```

The exact `n(n-1)/2` two-mode gate count and `n` terminal phases should be
proved for the selected algorithm, not claimed for every optimal compiler.

## 5. Passive port and ledger

### 5.1 Passive colligation

For retained map `S` and radiation map `R`, assume

```text
S†S + R†R = I.
```

Prove:

```text
||Sx||² + ||Rx||² = ||x||².
```

Separate observer maps, which have no state update and therefore no supply or
loss term.

### 5.2 Source work

For `x' = x + b`, define source work exactly as

```text
W_source = ||x+b||² - ||x||².
```

Do not replace this with `||b||²`; the cross term is physically part of source
work.

### 5.3 Ledger composition

Define one frame equation:

```text
E1 = E0 + Ws + Wc + Wm - Di - Er - Et + residual.
```

Prove telescoping over a list of frames and compositional closure for:

- unitary control;
- passive radiation;
- diagonal internal loss;
- explicit source injection;
- partial-isometry retirement;
- metric transport under stated hypotheses.

The formal residual may be exactly zero. The Rust refinement theorem accounts
for floating residual separately.

## 6. Weighted partial isometry

Let `M0,M1` be positive definite and `A` a raw correspondence. Define normalized
map

```text
B = sqrt(M1) A invsqrt(M0).
```

Let `Q` be the polar partial isometry and

```text
T = invsqrt(M1) Q sqrt(M0).
```

Required theorems:

### Expansion (`dim target >= dim source`, full source rank)

```text
T† M1 T = M0
energy_preserved_for_all_source_states
```

### Contraction (`dim target < dim source`, full target rank)

```text
T T† = identity in normalized target coordinates
T† M1 T is an M0-orthogonal projection
retiredEnergy x = E0 x - E1 (Tx) >= 0
```

Prove an impossibility theorem: no injective linear isometry exists from a
higher-dimensional positive-definite source space into a lower-dimensional
target space. This justifies mandatory disposal or rejection.

## 7. Projectors and direct rotation

Define orthogonal projector and the transition operator

```text
X(P,Q) = QP + (I-Q)(I-P).
```

Under an explicit overlap/invertibility hypothesis, define its unitary polar
factor `U(P,Q)`.

Required results:

```text
directRotation_unitary
directRotation_intertwines
inverse_directRotation
gauge_free_fullSpace_definition
```

For a path of projectors, define left-ordered product and prove exact reversal
and concatenation laws under the fixture convention.

The convergence from discrete products to continuous Kato transport may remain
an analytic theorem outside Lean initially, but the hypotheses and numerical
error certificate must be represented.

## 8. Gauge covariance

Given two base frames related by `G in U(r)`, prove the occupied loop matrix
transforms as

```text
W' = G† W G.
```

Formal invariants:

```text
trace_conjugation
characteristicPolynomial_conjugation
determinant_conjugation
commutator_norm_frameInvariant
```

For eigenphase receipts, prefer characteristic-polynomial or unordered spectrum
statements in the first formal version rather than depending on analytic branch
choices.

## 9. Wilson--Dirac family

### 9.1 Clifford algebra

Formalize the five `4 x 4` gamma matrices and prove:

```text
Gamma_a† = Gamma_a
Gamma_a Gamma_b + Gamma_b Gamma_a = 2 delta_ab I.
```

Derive:

```text
H(k)^2 = ||d(k)||² I
spec H(k) subset {-||d||,+||d||}
occupied projector formula
rank occupied = 2 when ||d||>0.
```

### 9.2 Exact gap at `m=-3`

Formalize `t_i=1-cos k_i` and prove:

```text
||d(k)||² = 1 + 2 sum_{i<j} t_i t_j
1 <= ||d(k)||
||d(k)|| <= 7.
```

The lower bound is load-bearing. The upper bound is needed for positive carrier
frequency and trivial-band placement.

### 9.3 Stabilization and coupling

Define

```text
K0 = H_D directSum 9I directSum 11I directSum 13I
Ks = Vs K0 Vs†.
```

Prove:

```text
Ks_selfAdjoint
Ks_isospectral_K0
protectedProjector_conjugation
rank_two
uniform_gap_two
regional_coupling_adjoint_pairs
```

The exact one-quarter regional participation at `s=1` can be proved from the
butterfly entries.

## 10. Characteristic class boundary

A complete formal proof of `C2=1` in Lean is valuable but not the first blocker.
Stage the work:

1. formalize the signed corner-mass phase formula as an imported analytic
   theorem with explicit hypotheses;
2. prove the `m=-3` evaluation equals one;
3. represent the independent degree and lattice-gauge outputs as certificates;
4. later formalize the homotopy/degree theorem or connect to a verified
   characteristic-class library.

The theorem interface must say exactly which orientation and occupied-subspace
convention is used.

For realification, target:

```text
p1(E_R) = c1(E)^2 - 2 c2(E)
```

and specialize to `c1=0`, `c2=1`. Until that theorem is formalized, keep it as
an analytic citation/hypothesis rather than a kernel claim.

## 11. Perturbation and interval certificates

Define a checked record:

```text
structure GapCertificate where
  familyDigest : Digest
  cut : ℝ
  lowerBound : ℝ
  lowerBoundPositive : 0 < lowerBound
  proofMethod : GapProofMethod
  payloadDigest : Digest
```

The full coefficient-box proof can be formalized directly from:

```text
||sum delta d_a Gamma_a|| = ||delta d||_2
Weyl gap shift <= 2 ||Delta H||.
```

Interval computations for more general families should be validated outside
Lean and imported only through a checker that verifies interval endpoints,
subdivision coverage, and arithmetic certificates.

## 12. Nonlinear inheritance

Do not formalize a “nonlinear Chern number” merely to bridge the production
model. Formalize the scoped statement:

- topology belongs to the linear conservative skeleton;
- a nonlinear Jacobian perturbation is bounded on a declared domain;
- Weyl preserves the linearized gap when `2L<g`;
- Grönwall bounds trajectory deviation.

The claim evaluator must not promote the nonlinear flow itself to
`topological-bundle` without a new theory.

## 13. Claim/refusal soundness

Formalize the claim lattice as evidence predicates rather than an ordered enum
alone. Example:

```text
ProtectedAcoustic evidence :=
  TopologicalBundle evidence /\
  PersistentState evidence /\
  LawfulLowering evidence /\
  BidirectionalAcousticParticipation evidence /\
  PassiveRadiation evidence /\
  WholeLedger evidence.
```

Then prove the evaluator never emits a claim whose predicate is false.

Specific non-implications should be represented:

```text
TopologicalBundle !-> FaultTolerant
Noncommuting !-> Geometric
EndpointReplay !-> AdiabaticRealization
WaveformDifference !-> ListenerDiscrimination
Orthogonal !-> ComplexLawful
EnergyScalarClosure !-> StateFidelity
```

## 14. Fixture refinement

The formal fixture decoder need not parse JSON in Lean. Generate a compact Lean
constant or binary-to-Rust checked representation with:

- schema version;
- source/family/path/program digests;
- dimensions and coordinate convention;
- gap and rank fields;
- claim gates;
- tolerances.

Prove the generated constant satisfies structural well-formedness. Rust then
checks the same digest and fields before execution.

The eventual refinement theorem should have the shape:

```text
validFixture f ->
validRustDecode bytes f ->
executeProgram bytes x = formalProgram f x + boundedFloatingError.
```

A full floating-point proof may be deferred, but every arithmetic residual used
for claim promotion must have an explicit bound and test vector.

## 15. Initial formal acceptance target

The first meaningful formal milestone is complete when Lean proves, with no
admissions:

1. realification homomorphism and orthogonal-symplectic law;
2. two-mode gate and program energy preservation;
3. passive radiation conservation;
4. ledger telescoping;
5. weighted partial-isometry expansion/contraction laws;
6. direct-rotation intertwining under explicit invertibility;
7. gauge conjugation invariants;
8. exact `m=-3` gap identity and lower bound;
9. unitary conjugation preserves the stabilized family gap;
10. claim evaluator soundness for the finite evidence predicates.

That milestone would materially strengthen production correspondence even
before a fully formalized second-Chern theorem is available.
