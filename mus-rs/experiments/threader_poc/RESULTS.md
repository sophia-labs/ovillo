# Reproduced toy run

This result was produced with:

```bash
PYTHONPATH=. python -m threader_poc.train \
  --steps 600 \
  --batch-size 16 \
  --utterances 4 \
  --refinement-steps 60 \
  --cpu-threads 1 \
  --output artifacts/toy-run
```

Environment: PyTorch `2.10.0+cpu`, NumPy `2.3.5`, `cpu`.
The amortized training loop took **30.91 seconds** on the
available CPU.  This excludes final per-voice refinement and artifact writing.

## Held-out exact-simulator evaluation

| Measure | Value |
|---|---:|
| Anatomy RMSE, raw toy latent units | 0.3986 |
| Local-program RMSE, raw toy latent units | 0.5190 |
| Nuisance RMSE, raw toy latent units | 0.3040 |
| Mean anatomy posterior standard deviation | 0.4029 |
| Empirical coverage of nominal 90% anatomy interval | 92.7% |
| Deterministic posterior-mean reconstruction SNR | -2.86 dB |
| Confidence-weighted analysis loss | 0.2159 |

The 90% anatomy intervals covered **92.7%** of the
known simulator latents in this small evaluation batch.  That is encouraging,
but not a calibration claim: one deterministic toy run and 96 anatomy
coordinates are far too small for one.

## One held-out voice: amortized inference → MAP refinement

| Measure | Amortized | Refined |
|---|---:|---:|
| Objective | 0.4874 | 0.2361 |
| Mean waveform L1 | 0.2352 | 0.1393 |
| Reconstruction SNR | -2.82 dB | 0.36 dB |
| Anatomy RMSE | 0.2999 | 0.3105 |
| Local-program RMSE | 0.5136 | 0.5122 |

Refinement reduced the total fit objective to **48.4%**
of its initial value and reduced waveform L1 by
**40.8%**.  The
anatomy error moved from 0.2999 to
0.3105: essentially held rather than improved.  This is
the intended behavior of the posterior trust region.  An earlier unconstrained
polish improved acoustics while nearly doubling anatomy error, demonstrating
why production Threader must perform MAP inference under a physical/program
prior rather than pure sound matching.

## What this run establishes—and does not

It establishes that the code path is real:

```text
exact and bundle-only episodes
  → shared/body and local/program posteriors
  → differentiable physical-ish render
  → masked multiscale losses
  → held-out amortized reconstruction
  → posterior-regularized fit
  → typed program solution + content-digested receipt
```

It does not establish useful speech quality, human vocal-tract recovery, or
production-scale identifiability.  The sample rate is 4 kHz, each utterance is
128 ms, and the throat is a six-section all-pole surrogate.  Those restrictions
make the experiment a fast architecture test rather than a speech result.

Receipt digest: `0961d18d13179f6bcb92b87a8b3c915254814bc040994bb0c957c695bcdc1cba`.
