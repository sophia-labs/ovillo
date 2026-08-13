# Ariadne string–brass campaign v1

This campaign asks whether one persistent circuit-free Ariadne body can be
played as a clean electric guitar or as a trombone-like sustained brass body
without changing renderer, state dimension, or energy law.

## Reproduce the local law battery

```bash
python reference/unified_boundary_laws.py --trials 10000 --json
```

The law oracle uses only the Python standard library.

## Reproduce the behavioral reference

The behavioral renderer requires NumPy and Numba:

```bash
python reference/shared_body_campaign.py \
  --output artifacts/ariadne-string-brass-v1 \
  --sample-rate 96000
```

It writes:

```text
01_same_body_electric_guitar_clean.wav
02_same_body_trombone_sustain.wav
03_same_body_continuous_guitar_to_trombone.wav
campaign-results.json
```

All three renders use one 24-mode field and one aperture implementation. The
continuous file preserves state while geometry and playing regime move from
plucked-string to sustained-brass territory.

## Rust primitives

```text
mus-dsp/src/ariadne_morphing_field.rs
mus-dsp/src/ariadne_compliant_aperture.rs
mus-dsp/src/ariadne_string_brass_profiles.rs
```

They are exported behind `ariadne-live` but are not wired into the default
terminal. See `REPORT.md` for equations, measured results, negative findings,
and promotion gates.

## Claim boundary

The campaign demonstrates one-model reachability and local work identities. It
does not yet establish blinded listener recognition, named-instrument fidelity,
robust trombone slide control, or production Rust/plug-in performance.
