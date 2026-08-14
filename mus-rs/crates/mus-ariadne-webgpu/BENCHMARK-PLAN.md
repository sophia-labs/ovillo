# Performance and conformance plan

## Registered comparison

Compare the existing Ariadne Sea kernel with the first library graph:

```text
baseline: one dense field pass containing a per-cell loop over every event
candidate: local impulse raster + tiled dense wave pass
```

Sweep:

```text
events       1, 16, 64, 256, 1024
field        256^2, 384^2, 512^2, 768^2
workgroup    direct 8x8, tiled 8x8, tiled 16x16
precision    portable f32/RGBA16F; optional f16 arithmetic later
```

Measure after pipeline warm-up:

- median and p95 GPU time per pass;
- CPU encode time;
- upload and readback bytes;
- allocations after warm-up;
- effective updated cells per second;
- event-covered pixel count;
- numerical difference from the CPU reference;
- zero-law residuals.

## Required scientific controls

1. **Zero event**: impulse target is exactly zero after clear.
2. **Local support**: an event changes no pixel outside its declared radius.
3. **Zero field**: a zero state with zero impulse remains exactly zero.
4. **Damped field**: without new impulse, the discrete storage proxy does not
   grow beyond the registered numerical tolerance.
5. **Segmentation invariance**: identical fixed-step inputs yield identical
   state regardless of presentation-frame grouping.
6. **Refinement**: selected macroscopic observables converge over 128^2,
   256^2, and 512^2 grids under a declared parameter scaling.

## Initial performance gates

The first migration is promoted only when:

- it beats the baseline at 16 or more compact events on two independent GPU
  families;
- it never regresses the one-event case by more than 15%;
- p95 frame time remains inside the selected quality budget;
- no per-frame GPU allocation or synchronous readback appears in a capture;
- the CPU/GPU reference residuals and zero laws pass.

No absolute speedup is promised before the benchmark is run.
