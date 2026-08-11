# Ariadne protected non-Abelian acoustic instrument

**Status:** protected non-Abelian **reference instrument** achieved; production
Weave integration remains the next engineering phase.

**Branch:** `agent/ariadne-protected-nonabelian`  
**Base:** `agent/ariadne-research-consensus`

## Result

This branch fuses the two ingredients that the gesture-compiler research had
previously established separately:

1. a gapped, topologically nontrivial family; and
2. degenerate, noncommuting non-Abelian transport.

It then adds the missing acoustic lowering, work contract, finite-time replay,
Rust conformance fixture, and deterministic audio stimuli.

The reference instrument is built from the four-dimensional Wilson--Dirac
family

```text
H(k) = sum_{i=1}^4 sin(k_i) Gamma_i
     + (m + sum_{i=1}^4 cos(k_i)) Gamma_5,
m = -3.
```

Its negative-energy eigenspace is a doubly degenerate rank-two bundle over the
four-torus.  In the convention used by the code, this bundle has second Chern
number `C2=1`.

Two smooth same-basepoint loops generate Kato/Wilson holonomies `A` and `B`
inside that protected bundle.  They do not commute.  The full four-complex-mode
unitaries are realified to eight Euclidean acoustic coordinates and compiled
exactly into graph-edge Givens schedules.

## Full deterministic receipt

| Check | Result |
|---|---:|
| analytic second Chern number | `1` |
| numerical degree estimate, 12^4 grid | `0.996588105970` |
| sampled base spectral gap | `2.013884398` |
| trivial `m=-5` degree estimate | `0.000306384785` |
| critical `m=-4` gap at the origin | `0` |
| loop A eigenphases | `±0.9219435973` |
| loop B eigenphases | `±0.8602599981` |
| `||AB-BA||_F` | `1.6876418777` |
| AB/BA endpoint-state distance | `1.1933430159` |
| tested gap-preserving perturbations | `12` |
| minimum sampled homotopy gap | `1.9638610623` |
| minimum perturbed `||AB-BA||_F` | `1.3133096471` |
| longest-duration adiabatic gate error | `5.06e-4` |
| longest-duration occupied-subspace leakage | `1.98e-2` |
| real acoustic coordinates | `8` |
| graph-edge gates per Wilson loop | `28` |
| worst lowering reconstruction error | `1.08e-14` |
| maximum basis-state energy residual | `6.66e-16` |
| audible AB/BA relative RMS distance | `1.16414` |
| audible AB/BA RMS-level difference | `0.00095 dB` |

The exact acoustic `Z3` pump algebra is included as a second, discrete routing
instrument.  Both pump generators have exact order three, but the chronological
words `C3,C3,C3',C3` and `C3',C3,C3,C3` end in the same internal state at
different cells: one shifts by one cell and the other remains trapped.

## What “protected” means here

The strongest defensible statement is:

> The playable non-Abelian gates live in a rank-two acoustic state family whose
> nonzero second Chern class cannot change under a continuous deformation
> without closing the spectral gap.

The tested deformations preserve the sampled gap, preserve the numerical
second-Chern label, and retain strong noncommutativity.  The finite-time
Schrödinger simulation converges to the geometric Kato transport as the gesture
duration increases.

This does **not** mean that an individual Wilson matrix is quantized or
fault-tolerant.  Its value remains path dependent.  Protection belongs first to
the gapped bundle and its topological class.  The exact `Z3` pump supplies a
separate discrete topological-routing target grounded in prior acoustic
experiments.

See `THEORY.md` for the mathematical construction and
`IMPLEMENTATION-HANDOFF.md` for the production sequence.

## Artifacts

The deterministic runner emits:

```text
results.json
fixture.json
SUMMARY.md
yang-loop-ab.wav
yang-loop-ba.wav
z3-path-one.wav
z3-path-two.wav
```

The WAVs are listening stimuli, not a listener-study result.  The Python oracle
generates the targets; a separate Rust test replays the generated sparse
schedules through `mus_watsn::OperatorProgram`; the Lean module checks the exact
finite `Z3` pump algebra.
