# Theory of the protected non-Abelian Ariadne reference instrument

## 1. Target

The earlier Ariadne program reached two concrete but separate precursors:

- an Abelian topological pump with a nonzero first Chern number;
- a constant-gap rank-two bundle with noncommuting Wilson loops.

The missing object was their fusion with sparse acoustic execution, a work
ledger, and an audible realization.  This branch supplies a reference fusion
through a four-dimensional Wilson--Dirac bundle and retains the experimentally
grounded acoustic `Z3` pump as a complementary discrete target.

The claim hierarchy is intentionally explicit:

```text
geometric non-Abelian transport
    < gapped topological non-Abelian family
    < quantized/discrete protected pump operation
    < fault-tolerant protected gate
```

This packet reaches the second level directly and includes an exact algebraic
fixture for the third.  It does not claim the fourth.

## 2. Rank-two Wilson--Dirac bundle

Choose five Hermitian Clifford generators satisfying

```text
Gamma_a Gamma_b + Gamma_b Gamma_a = 2 delta_ab I.
```

For momentum/control coordinates `k in T4`, define

```text
d_i(k) = sin(k_i),                    i=1..4
d_5(k) = m + sum_i cos(k_i)
H(k)   = sum_a d_a(k) Gamma_a.
```

The spectrum is doubly degenerate:

```text
spec H(k) = {-|d|,-|d|,+|d|,+|d|}.
```

The occupied projector is therefore available without a gauge choice:

```text
P(k) = 1/2 (I - H(k)/|d(k)|).
```

At `m=-3`, the occupied rank-two bundle has second Chern number `+1` in the
orientation convention used here.  The undeformed lattice model has the exact
corner-mass formula

```text
C2(m) = 1/2 sum_{r=0}^4 (-1)^r choose(4,r)
                    sign(m + 4 - 2r).
```

The phase boundaries are `m=-4,-2,0,2,4`, where the gap closes.

## 3. Gauge-free numerical second Chern certificate

For `n=d/|d| : T4 -> S4`, the second Chern number of this Dirac family equals
the degree of `n`:

```text
C2 = 1/Vol(S4) integral_T4
       det[n, partial_1 n, partial_2 n, partial_3 n, partial_4 n] d^4k,

Vol(S4) = 8 pi^2 / 3.
```

`protected.py` evaluates the derivatives analytically and integrates this
gauge-free density on a periodic midpoint grid.

The full reference run gives

```text
C2 numeric = 0.9965881059699271
nearest integer = 1
sampled gap = 2.0138843976445044.
```

A trivial control at `m=-5` gives `0.0003063847846279023`, while the critical
model at `m=-4` has exactly zero gap at the origin.

## 4. Declared protection family

The perturbation family changes:

- the mass within `±0.15`;
- four sine velocities within `±12%`;
- four cosine weights within `±8%`;
- four smooth, coupled coordinate warps within `±0.18`;
- warp phases over the full circle.

For every deterministic perturbation, the runner also samples the straight
homotopy from the base model.  Across twelve trials:

```text
minimum endpoint sampled gap      1.7713890821
minimum sampled homotopy gap      1.9638610623
maximum numerical C2 error        0.0749953598
```

The Chern integration error is discretization error rather than evidence that
the integer invariant drifts.  The important contract is that the homotopy
remains sampled-gapped and the estimate remains in the basin of the same
integer phase.

A rigorous protection proof would replace sampled gaps with analytic interval
bounds or certified numerics.  That remains a valuable next formal step.

## 5. Kato transport and non-Abelian gates

For adjacent projectors `P0` and `P1`, define

```text
X = P1 P0 + (I-P1)(I-P0).
```

The unitary polar factor of `X` is the direct rotation that carries `P0` to
`P1`.  Multiplying these direct rotations along a closed path gives a
gauge-free full-space Kato holonomy.  Restriction to the base occupied frame
produces the `U(2)` Wilson operator.

The reference uses two smooth loops through the same base point:

```text
base = (-0.4, 0.0, 0.9, -0.7)

A: plane (k3,k4), radii (1.50,1.20)
B: plane (k1,k4), radii (1.30,1.00).
```

Their occupied holonomies have eigenphases

```text
A: ±0.9219435973
B: ±0.8602599981
```

and

```text
||AB - BA||_F = 1.6876418777.
```

For the fixed reference input, changing the order produces endpoint-state
distance `1.1933430159`.

The Wilson matrices change continuously if the loops are deformed.  Their
noncommutativity is therefore geometric rather than quantized.  Over the
declared perturbation family, however, the minimum observed commutator norm is
still `1.3133096471`.

## 6. Finite-time adiabatic realization

Kato transport is a geometric oracle, not by itself a finite-speed physical
render.  The runner separately integrates

```text
d psi / dt = -i H(k(t)) psi
```

with midpoint piecewise-constant propagators.  Because the occupied pair is
exactly degenerate, its dynamical phase is scalar and can be removed before
comparing the resulting gate to the Kato holonomy.

At gesture durations `16, 32, 64, 128`, the projective gate error and leakage
decrease.  At the longest duration, the worst of the two loops is

```text
projective holonomy error = 5.06e-4
occupied-subspace leakage = 1.98e-2.
```

This is the first operational adiabaticity receipt.  A production compiler
should expose duration, gap, leakage, and geometric endpoint error as separate
constraints rather than treating “slow” as an untyped adjective.

## 7. Acoustic realification and sparse lowering

For a complex unitary `U=A+iB`, its action on `[Re psi, Im psi]` is

```text
R(U) = [[ A, -B ],
        [ B,  A ]].
```

`R(U)` is a real orthogonal matrix.  The four complex modes therefore become
eight real, energy-normalized acoustic coordinates.  Each full Wilson
holonomy is compiled on the authored eight-coordinate path graph by the exact
tree-QR compiler:

```text
gate count per loop = 8*7/2 = 28.
```

The full-run reconstruction errors are

```text
A: 4.26e-15 Frobenius
B: 1.08e-14 Frobenius.
```

Every factor is a Givens rotation, so the declared Euclidean state energy is
preserved to binary64 roundoff.  A generated Rust fixture independently
replays both words through `mus_watsn::OperatorProgram` and verifies the AB/BA
difference.

This lowering proves executable equivalence at the normalized operator layer.
It does not prove that arbitrary physical resonator couplings realize the same
metric without actuator work.

## 8. Exact acoustic Z3 pump algebra

The acoustic non-Abelian Thouless-pump experiment motivates two order-three
operations over the states `plus`, `minus`, and `link` in periodic cells.

The reference uses

```text
C3:
  plus_N -> link_N -> minus_N -> plus_N

C3':
  plus_N -> minus_N -> link_(N-1) -> plus_N.
```

Both satisfy

```text
C3^3 = I
C3'^3 = I,
```

but they do not commute.  Starting from `plus` in the center cell,

```text
C3,C3,C3',C3   -> minus in the left cell
C3',C3,C3,C3   -> minus in the center cell.
```

The internal state matches while the transport differs.  The exact algebra is
tested in Python and Rust and kernel-checked in Lean.

This finite fixture imports the published pump action as a target.  It does
not rederive the waveguide Hamiltonian or establish robustness of a new
physical sample.

## 9. Audible reference instrument

The Yang-bundle renderer maps the four complex coordinates to four resonant
regions with fixed frequencies and stereo locations.  The state is transported
continuously along the Kato path; the oscillator mapping observes the state but
does not participate in the transport proof.

The two deterministic files `yang-loop-ab.wav` and `yang-loop-ba.wav` have:

```text
relative RMS distance      1.16414
RMS level difference       0.00095 dB
spectral centroids         248.34 Hz and 260.20 Hz.
```

The `Z3` renderer converts cell position into spatial/pitch displacement and
the three internal states into distinct resonant identities.  One sequence
audibly moves and the other remains trapped.

These stimuli establish audibility in the weak engineering sense that the
rendered waveforms differ under tightly matched level.  They are not a
psychoacoustic discrimination result.

## 10. Protection receipt

A compiled protected gesture must name at least:

```text
family digest
bundle rank
topological invariant and convention
gap floor
allowed perturbation/homotopy family
path and base point
adiabatic duration
leakage estimate
Wilson endpoint and noncommutativity observables
realification convention
sparse lowering and replay residual
energy/work/loss receipt
claim level
```

The distinction between compiler prediction and measured production behavior
remains binding.

## 11. Claim boundary

Established in this packet:

- one rank-two family with analytic `C2=1`;
- independent numerical degree and gap checks;
- strong noncommuting Kato/Wilson gates;
- persistence over a declared gapped perturbation family;
- finite-time convergence toward the geometric gates;
- exact sparse acoustic lowering;
- independent Rust replay;
- exact `Z3` pump algebra;
- deterministic audible stimuli.

Not established:

- a hardware metamaterial;
- production Weave integration;
- certified interval bounds on the gap;
- quantization of individual Wilson matrices;
- fault-tolerant gates;
- listener discrimination;
- a real-valued Lean proof of second-Chern protection or DSP soundness.

## 12. Prior-art boundary

The relevant prior art includes:

- the Yang monopole and second-Chern characterization of a degenerate
  non-Abelian bundle;
- Wilson loops and Wilczek--Zee phases;
- acoustic non-Abelian braiding of degenerate modes;
- the experimental acoustic `Z3` non-Abelian Thouless pump;
- lattice-gauge algorithms for second Chern numbers.

Ariadne's candidate contribution is their integration with exact
graph-restricted lowering, state-energy accounting, inverse gesture
compilation, instrument/body/space semantics, and an audible software
instrument.
