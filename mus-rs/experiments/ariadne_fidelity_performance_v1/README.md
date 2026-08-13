# Ariadne fidelity and performance v1

This directory records the first measured campaign after the circuit-free Ariadne constitution.

## Contents

- `REPORT.md` — experiment design, measured results, equations, negative results, promotion decisions, and next gates.
- `RESULTS.json` — compact machine-readable replay results.
- `reference/active_resonant_laws.py` — dependency-free independent oracle for the two landed candidate laws.

The production-shaped integrated campaign used a NumPy/Numba mirror of `mus_dsp::ariadne_terminal` plus isolated candidate physics. It exercised sine-level, two-tone, pluck, loud/soft memory, tail, native-rate performance, and sample-rate-agreement probes.

The full exploratory harness was intentionally not promoted as a second renderer. The durable artifacts are the equations, measurements, compact result record, independent law oracle, and Rust primitives.

## Re-run the independent law oracle

```bash
python mus-rs/experiments/ariadne_fidelity_performance_v1/reference/active_resonant_laws.py \
  --trials 10000 --json
```

Expected ceilings:

```text
orthogonality residual       <= 2e-14
active-work closure          <= 2e-13
transport metric residual    <= 2e-14
frozen-rate state error      <= 5e-12
zero state                   exact
room-triggered activation    > 0.05
```

The measured 10,000-trial reference run produced:

```text
orthogonality residual       2.220446049250313e-16
active-work closure          1.3976043629695956e-19
transport metric residual    5.421010862427522e-20
frozen-rate state error      2.886579864025407e-14
room-triggered activation    0.5437807169069131
zero state                   exact
```

## Rust modules

```text
mus_dsp::ariadne_active_inclusion
mus_dsp::ariadne_resonant_transport
```

They are research primitives and are not yet wired into the default live terminal. Integration must remain behind an explicit experimental anatomy until Rust, fixed-rate, external-fidelity, aliasing, listening, and host-performance gates pass.

## Claim boundary

Current results are controlled behavioral-region evidence. External hardware-recorded corpus replay and human/player evaluation remain open. Timing is from the Numba/LLVM reference on one Xeon Platinum 8573C core, not from a Rust plug-in binary.