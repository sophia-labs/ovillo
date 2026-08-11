# Unified U2 — protected family/path IR in `mus-gesture`

Binding sources: `paper/ariadne-protected-nonabelian/RUST-API-CONTRACT.md`
(fields and refusal semantics are normative), `UNIFIED-INSTRUMENT-THEORY.md`
§§4–6, 10–11, `UNIFIED-INSTRUMENT-IMPLEMENTATION-HANDOFF.md` stage U2.
U1 is landed: `mus_watsn::ComplexOperatorProgram` replays all four committed
words against NPZ-derived dense matrices, occupied eigenphases, and the
1.6877 non-Abelian witness (`mus-watsn/tests/unified_replay.rs`).

One new module: `mus-rs/crates/mus-gesture/src/protected.rs`. Consensus
rule 1 holds — no second executor, no second Givens, no second ledger. The
executable word type remains `mus_watsn::ComplexOperatorProgram`; this
module owns the *family*, the *paths*, the *budgets*, and the *refusals*.

## A. Closed-form family (no tables)

Per contract §6: "evaluate the closed-form Wilson–Dirac family directly."

- Five Gammas from sigma tensor products (theory §4), Hermitian, pairwise
  anticommuting; checked in tests, not assumed.
- `dirac_d(k, mass) -> [f64; 5]`, `dirac_hamiltonian(k, mass)` (4×4
  complex on `mus_watsn::Complex64`), `radius(k) = ||d||_2`.
- The Clifford norm identity does the heavy lifting: `H(k)^2 = r(k)^2 I`,
  so gaps and operator norms of coefficient perturbations need **no
  eigensolver anywhere in this module**.
- Exact gap law at `m = -3`: `r^2 = 1 + 2 * sum_{i<j} t_i t_j` with
  `t_i = 1 - cos k_i` — replayed as an identity test, giving
  `gap(k) = 2 r(k) >= 2` globally.

## B. Paths

`SmoothTorusLoop` ports `SmoothLoopSpec` exactly: quintic smoothstep
phase (velocity and acceleration vanish at endpoints), first axis moved
by `r1 (1 - cos θ)`, second by `r2 sin θ`, torus wrap. `LOOP_A` (axes
2,3; radii 1.50, 1.20) and `LOOP_B` (axes 0,3; radii 1.30, 1.00) at base
`(-0.4, 0.0, 0.9, -0.7)`. Closure is checked wrap-aware; an open path is
`PathNotClosed`, never a warning.

## C. The JRS receipt and the refusal that must survive

Port `jrs_adiabatic_bound` term for term (endpoint `m·||H'||/g²`;
integrand `m·||H''||/g² + 7 m √m ||H'||²/g³`; trapezoid on the same
4097-point grid; derivative norms via the analytic `d`-vector chain
rule — the Clifford identity again, no matrices). Acceptance values to
reproduce against the committed fixture:

```text
minimum sufficient duration, 5% budget:  A 22.362855480397428 s
                                         B 15.941498262892383 s
```

The load-bearing behavior: `PhysicalAdiabatic` at 1 s / 5% **must refuse**
(`AdiabaticBoundFailed`, carrying observed bound, required budget, and the
minimal sufficient duration). Numerically-good replay never overrides a
failed theorem receipt (their N9).

## D. Modes, budgets, refusals, claims

- `ProtectedExecutionMode::{CompiledGeometric, PhysicalAdiabatic}` per
  contract §8. CompiledGeometric consumes an already-validated U1 word
  and promotes at most `Geometric`. PhysicalAdiabatic runs the JRS gate.
- `ProtectedRefusal`: all 28 contract §9 variants exist from day one;
  each refusal returned through `ProtectedRefusalDetail { refusal,
  observed, required, field_path, strongest_available_claim }`. Variants
  not yet reachable in Rust are constructible but never silently mapped
  onto a neighbor.
- `ClaimLevel` (8 levels) + `EvidenceClass` (5 classes) + evaluator
  returning promoted *and* withheld-with-reasons. Hard rules encoded as
  tests: endpoint-only evidence cannot reach `Geometric`; no receipt
  combination reaches `AdiabaticRealization` without an accepted JRS
  certificate; `ProtectedAcoustic` stays withheld until U3+U5 exist
  (reason string names the missing stage). The evaluator never collapses
  to one boolean (contract §15).

## E. Digest lane

Family/path digests are computed by this crate over its own canonical
serialization (`digest.rs`, sha2), recorded in receipts as *production*
digests. The Python fixture digest is cited, never copied as evidence
(their N17). Cross-runtime agreement is demonstrated by replaying
numbers, not by comparing digest strings.

## Gates

1. Gamma algebra + exact gap identity + `H² = r² I` at seeded random `k`.
2. LOOP_A/LOOP_B closure exact; minimum sampled gap `>= 2 - 1e-12`.
3. JRS minimum durations match the fixture to relative `1e-9`.
4. 1 s `PhysicalAdiabatic` at 5% refuses with `AdiabaticBoundFailed`.
5. Perturbed-endpoint path refuses with `PathNotClosed`.
6. `m = -4` critical family violates a 1.999 gap floor on LOOP_A →
   `GapBudgetViolated` with observed minimum.
7. Claim evaluator: the three hard rules above, plus withheld reasons
   are strings a human can act on.
8. `cargo test -p mus-gesture` and workspace clippy `-D warnings` green.

## Deliberately deferred (named, not hidden)

- Second-Chern estimators in Rust (degree integral / lattice gauge):
  U-formal / production-protocol §5.2 territory, not U2.
- Kato projector-table replay from NPZ: the committed words already
  carry the transports; per-sample projector data joins with U3's
  fixture conversion step.
- `ProtectedFamilyV2` full struct with all fixture receipt fields:
  landed here as the subset the compiler consumes; the remainder arrives
  with the U3 fixture-conversion build step.
