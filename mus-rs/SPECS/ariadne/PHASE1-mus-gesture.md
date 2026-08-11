# Ariadne Phase 1 — `mus-gesture`: the exact rotation compiler

Binding source: `ARIADNE-IMPLEMENTATION-CONSENSUS.md` §5.2/§6 Phase 1/§7
Phase 1 gates; `paper/ariadne-gesture-compiler/IMPLEMENTATION-HANDOFF.md`
Commits A–D; Python oracle in
`paper/ariadne-gesture-compiler/reference/ariadne_gesture/`.

## Scope

Commits A–D only: crate + IR, exact tree-QR, thermodynamic scheduler +
finite search, receipts, fixture replay. Commit E (MUS-F parser), F
(closed SO(3)), and beyond are Phase 2 — do not start them.

## The crate

`mus-rs/crates/mus-gesture`: pure Rust, **depends only on `mus-watsn`**
(and std). No parser, RDF, host, or audio-device deps. It emits
`mus_watsn::OperatorProgram` — it must NOT define a second factor
executor, Givens routine, or work ledger (consensus rule 1).

IR per the handoff (ModeId, EdgeId, EdgeControl{friction,
max_angle_rate}, ControlGraph, RotationTarget, GivensGate) with
validation: stable IDs, no self-edges, finite positive friction, target
shape/orthogonality/determinant checks, connected-component report.
Dense matrix work is construction-time only. Use plain `Vec<f64>`
row-major helpers or a minimal internal matrix type — do NOT add a
linear-algebra dependency without strong reason; n ≤ 32 dense is
trivial.

## Corrections to the reference (from our deep review — these are
deviations from the Python oracle made deliberately; record them in the
crate docs):

1. **Hard-reject off-graph trees.** `compile_so_on_tree` in Python
   *reports* `all_gates_on_graph=false` if caller-supplied tree edges
   are not in the graph. The Rust compiler returns a typed error
   instead. Certificates never carry a false "on graph" bit.
2. **Pin the composition convention in the IR.** The Python package
   mixes left-action (`schedule_matrix`: `total = G @ total`, list
   order) and right-product (`_loop_product`) conventions.
   `OperatorProgram` execution order in mus-watsn is the single truth —
   document it on the IR type and test a 3-gate non-commuting word
   against a hand-computed product.
3. **Per-edge closure asserts ≤ 1e-12, never == 0.0** (their own HEAD
   relaxed CI to this; the prose overclaims).
4. **Do not port `zero_work_residual`** — in the reference it is an
   orthogonality probe on one fixed vector, not a work measurement.
   The receipt's work field comes from the mus-watsn ledger applied to
   the compiled program in the declared metric, or is absent.

## Tree-QR port

Follow `reference/ariadne_gesture/core.py::compile_so_on_tree`
faithfully otherwise: BFS spanning tree default, leaf-elimination
order, per-column peel with `theta = atan2(-b, a)`, reverse+negate for
synthesis, exactly n(n−1)/2 gates for a full target.

## Scheduler + search (Commit C)

`length_k = √w_k·|θ_k|`, `τ_k = τ·length_k/L`, `D_min = L²/τ` — closed
form, no iteration. Zero-angle gates get τ=0 and are dropped from the
executable schedule (documented). Then deterministic beam/random search
over spanning trees × leaf orders (seeded, candidate count reported,
monotone vs baseline by construction).

## Receipts (Commit D)

Fields per the handoff: graph digest, target digest, spanning-tree
digest, leaf order, gates+angles, segment durations, friction metric
digest, thermodynamic length, predicted minimum quadratic cost, actual
endpoint operator, endpoint error, work residual (see correction 4),
orthogonality residual, compiler version. Serde JSON, deterministic
field order.

## Fixture replay

The committed `reference/results.json` is CORRUPT (invalid JSON,
hand-edit artifacts) — do not parse it. Instead:
1. Regenerate fixtures: `cd paper/ariadne-gesture-compiler/reference &&
   <repo>/.venv/bin/python run_experiments.py --quick` (numpy/scipy live
   in the repo .venv; if missing, `uv pip install numpy scipy` there).
   Commit the regenerated file with a note that it replaces a corrupt
   artifact, and verify it parses.
2. Rust replay: load the by-dimension fixtures, re-run the same seeds'
   graph/target constructions where reproducible, and independently
   verify: gate count n(n−1)/2, reconstruction < 1e-11 Fro (their
   Phase 1 gate), every gate on an authored edge, det −1 and
   cross-component targets rejected typed.

## Gates (consensus §7 Phase 1, verbatim targets)

- Dimensions 2–32 (the Python oracle stops at 16 — go to 32 in Rust)
  over path, star, tree, complete, random connected graphs.
- Every gate on an authored edge. Generic count n(n−1)/2.
- Worst reconstruction < 1e-11 Frobenius over ≥1,000 random targets.
- Determinant −1 and cross-component targets rejected or explicitly
  decomposed (rejection with a typed error is sufficient this phase).
- Constant-speed identity closes analytically and numerically; search
  never worsens baseline; receipts reproduce under independent replay.
- House gates: fmt, workspace clippy -D warnings, full release tests.
