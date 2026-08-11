# Threader POC — hierarchical inverse instrument compiler

This directory is a runnable miniature of the first model that the SPEAK audio
bundle should feed.  It is deliberately small enough to train on CPU, but it
implements the architectural commitments rather than replacing them with a
waveform autoencoder.

## What is implemented

```text
K recordings from one voice
    + confidence-weighted analysis bundle
              │
              ▼
      audio + bundle encoders
              │
       ┌──────┴─────────┐
       ▼                ▼
shared anatomy     utterance-local program
posterior          gestures + source + nuisance
       └──────┬─────────┘
              ▼
 differentiable toy GARGANTA renderer
              ▼
 reconstructed recordings
```

The toy renderer has a positive six-section area profile, stable area-derived
reflection coefficients, a voiced/aspirated glottal source, framewise gesture
deformations, and a separate observation-channel nuisance transform.  It is an
explicitly replaceable surrogate, not a claim that a six-section all-pole model
is GARGANTA.

Training mixes two forms of supervision:

1. **Exact simulator episodes** know anatomy and local programs and train the
   amortized posterior directly.
2. **Bundle-only episodes** hide those latent targets and train through
   waveform, spectral, measured-feature, voice-consistency, contrast, and
   program-complexity losses.

After amortized inference, `refine_episode` optimizes one shared anatomy and all
utterance-local programs through the renderer.  The run writes a program
solution and a content-digested receipt.

## Run the miniature

From this directory:

```bash
python -m venv .venv
. .venv/bin/activate
pip install -e '.[test]'
pytest
python -m threader_poc.train --steps 300 --output artifacts/toy-run
```

The run emits:

```text
threader-poc.pt          checkpoint and frozen configs
program-solution.json   shared anatomy + local programs + fit metrics
receipt.json            run/config/artifact digests
                         
target.wav               first held-out utterance
amortized.wav            direct posterior-mean reconstruction
refined.wav              analysis-by-synthesis reconstruction
*-sequence.wav           all four held-out utterances with short gaps
```

## Data-bundle adapter seam

The toy `EpisodeBatch` uses dense tensors.  Production should adapt the frozen
corpus tables into the same semantic fields:

```python
# Concrete fields whose final storage API is not yet fixed:
EpisodeBatch(
    audio=load_native_or_training_view(sample_rows),
    analysis=assemble_dense_observation_vector(
        phone_intervals,
        syllable_intervals,
        f0_frames,
        glottal_cycles,
        spectral_envelopes,
        event_intervals,
        prosody,
        quality,
    ),
    analysis_confidence=assemble_confidence_and_applicability_masks(...),
    latent_supervision=program_solution_exists.astype(float),
    anatomy_target=program_solution.voice_anatomy,   # only where solved/exact
    local_target=program_solution.utterance_program, # only where solved/exact
    nuisance_target=program_solution.observation_model,
)
```

The production bundle should not flatten everything permanently.  A collator
can create dense model tensors from the typed Parquet sidecars while preserving
source table IDs in the training receipt.

## Explicit limitations

* The renderer is a differentiable all-pole tube surrogate, not the real Rust
  GARGANTA engine.
* The posterior is diagonal Gaussian; topology search and genuinely multimodal
  anatomy posteriors remain future work.
* Frame boundaries are fixed.  Production controls should be phone/event-aware
  splines or gesture events.
* The feature bundle contains only differentiable toy acoustics and F0.  The
  real enrichment schema should provide the complete multiscale observation
  set.
* The POC reconstructs its deterministic turbulence carrier.  Production
  should model stochastic residual sources probabilistically.

Those seams are narrow and named.  The important thing demonstrated here is
that one voice episode produces one shared physical body, several local
performances, rendered evidence, and a reproducible fit receipt.
