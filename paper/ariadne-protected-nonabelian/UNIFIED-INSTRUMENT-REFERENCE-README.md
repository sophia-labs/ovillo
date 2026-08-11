# Unified protected instrument reference packet

This directory is an additive completion packet for
`paper/ariadne-protected-nonabelian/UNIFIED-INSTRUMENT-MATHEMATICS-BRIEF.md` at
commit `c0c149bab7a9e2fb1f7ba8b3328694fcd948adf6`.

It does not overwrite the earlier reference packet. It adds a stronger,
explicitly coupled construction and preserves the earlier packet as historical
evidence.

## What this packet establishes

The executable Python reference defines one sixteen-complex-mode conservative
instrument state split into four physical regions:

```text
played string | body | near field | late room
```

A stabilized Wilson--Dirac family is distributed across those regions by an
exact unitary coupling homotopy. The selected rank-two subbundle has an exact
global gap, analytic `C2=1`, an independent lattice-gauge estimate, and
noncommuting Kato transports for the named loops `A` and `B`.

The same state is then used for:

- fixed string-port excitation and preparation;
- persistent AB/BA control execution;
- bidirectional region coupling;
- fixed room-port readout;
- passive radiation and energy accounting;
- variable-rank state-connection experiments;
- deterministic audio and content-addressed receipts.

The compiler works in `U(n)`, not arbitrary `SO(2n)`. Every intermediate gate
realifies to an orthogonal-symplectic operation and therefore preserves the
declared oscillator complex structure.

## Claim boundary

The packet supports a **reference-oracle `protected-acoustic` claim**. It does
not promote the production instrument. The generated fixture records the
following as open or refused:

- independent Rust/Weave replay;
- Lean kernel verification;
- integration into the current raw long-delay Weave graph;
- theorem-certified one-second adiabatic execution at the requested 5% budget;
- spectrally matched listener stimuli and blinded listener evidence;
- fault tolerance.

The long-delay refusal is substantive: a raw `L`-sample periodic delay has
quasienergy spacing `2π/L`, so an isolated rank-two Floquet window cannot remain
uniformly gapped as the delay grows unless the architecture introduces a
protected modal backbone, engineered band gaps, or an explicit passive-port
reduction.

## Layout

```text
UNIFIED-INSTRUMENT-THEORY.md
    Definitions, exact construction, theorem statements, bounds, claim scope.

unified_reference/
    Independent float64 mathematical and acoustic reference implementation.

run_unified_experiments.py
    Deterministic full/quick fixture generator.

unified_artifacts/
    Full-resolution fixture v2, results, operator tables, executable words,
    audio controls, summary, and SHA-256 manifest.

schemas/unified_fixture_v2.schema.json
    Machine-readable fixture contract.

RUST-API-CONTRACT.md
    Production IR, runtime, ledger, and typed-refusal contract.

FORMAL-OBLIGATIONS.md
    Lean/mathlib proof plan and certificate refinement boundary.

UNIFIED-INSTRUMENT-IMPLEMENTATION-HANDOFF.md
    Ordered implementation stages and ownership.

PRODUCTION-EXPERIMENT-PROTOCOL.md
    Cross-runtime, acoustic, perturbation, and listener study protocol.

NEGATIVE-RESULTS.md
    Preserved counterexamples and architectural refusals.

SPEC-COMPLETION-MATRIX.md
    M0--M12 and acceptance-gate status.

VALIDATION.md
    Exact commands and generated headline results.
```

## Replay

From `paper/ariadne-protected-nonabelian`:

```bash
python -m unittest discover -s unified_reference/tests -v
python run_unified_experiments.py --output unified_artifacts
```

A faster development packet is available with:

```bash
python run_unified_experiments.py --quick --output /tmp/unified-quick
```

The quick mode is not the committed acceptance packet.

## Artifact rule

The JSON and NPZ artifacts are construction-time inputs. Production code must
not regenerate constants silently in an audio callback. Rust must independently
replay the committed path/projector tables and compiled words, compare named
residuals, and emit its own content-addressed execution receipt.
