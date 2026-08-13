# mus-trajectory

`mus-trajectory` is the direct-authorship layer for persistent MUS instrument bodies.

It turns named, unit-bearing control tracks into deterministic sample-indexed frames and a receipt. A trajectory says **which path to take**. It does not infer a path from an acoustic target; that remains `mus-gesture`'s inverse-compilation responsibility.

The v1 contract provides:

- fixed-capacity versus topology-edit state semantics;
- typed refusals for invalid domains, discontinuous fixed-capacity motion, missing disposal policies, and exceeded rate/acceleration limits;
- `linear`, `smoothstep`, and `smootherstep` paths with analytic kinematic bounds;
- a declared quadratic path-effort metric, kept explicitly separate from physical work;
- deterministic zero-order-hold lowering at an authored control stride;
- cross-runtime semantic and frame digests based on binary, float-bit-exact canonicalization;
- JSON transport and an arrow-value helper for the MUS-facing surface.

The first production adapter lives in `mus-dsp::trajectory`. The Python mirror and the GARGANTA area-function adapter live in `mus_analysis.trajectory`.
