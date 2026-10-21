# PoC (3): air that is more than linear acoustics, added on top of the N-D tract World

Branch `ariadne/nd-configuration-20261020`, on top of `checkpoint-2a-20261020`
(`81774e0`). Tag `poc-3-20261020`. Code: `mus-rs/crates/mus-ariadne/src/law/aero.rs`,
`configuration/network.rs` (`ContactSpec`), `space_body.rs`; driver
`ovillo/crates/ovillo-tract-world/examples/poc_air.rs`; laws
`tests/air.rs`; receipt `receipts/poc3-receipt.json`; WAVs in
`ovillo/renders/tract-world/poc3/` (not committed).

## What the model is, plainly

The smallest honest step past linear acoustics inside the modal World. It is
**not** a mean-flow/acoustic split with convection, vortex dynamics, or a CFD
of the jet. The lumped tube network already carries the mean flow (a steady
volume velocity lives in the columns' momenta); POC-3 adds what a jet does:

1. **Contact and closure as configuration** (`ContactSpec`, in the network
   map): each oral column's flow aperture is its unclamped area above
   GARGANTA's 0.025 cm² floor. The narrowest is the jet column, and its
   inertance follows that aperture. A column at zero aperture is sealed
   (inertance at a numerical 1e-13 m²; the closed cavity gets a mode near
   0.07 Hz that holds its pressure). The velopharyngeal port does the same.
   Sealing and unsealing are moves like any other, transported by the
   mass-momentum connection with work and disposal receipts. A sealed
   column's accumulated `L u` is dropped at each move (a contact carries no
   air; it is the wall's reaction).
2. **Separation loss** at the jet column: the quasi-steady free-jet law
   `Δp = ρ|u|u/(2A²)` (discharge coefficient 1, no pressure recovery),
   applied by the implicit midpoint rule on the column's flow. The field
   loses exactly `ρ|ū|³dt/(2A²) ≥ 0`.
3. **Turbulent sound**: a dipole (series pressure) source 3 columns
   downstream with power `η × (jet loss)`,
   `η = min(0.004 (M/0.1)³, 0.05) · smoothstep(Re; 3000, 6000)`. The M³ is
   Curle's surface-dipole scaling (the exponent is the law). The constants
   are design hypotheses, the ones `ovillo-tract` uses on
   `site/ovillo-demo-20261020` (fe4fb03). Its exact work is taken from the
   jet's loss, and the second law is kept cumulatively (§ below).
4. **DC-correct losses**: with flow on, wall loss acts on column momenta only
   and radiation on pressures only, each at twice the per-mode rate, so every
   mode decays as before. A static pressure no longer leaks through the
   frequency-independent wall loss, and a steady flow out of the mouth no
   longer counts as radiation.

**GARGANTA G2** (`ANATOMY-EVOLUTION.md` §3: "a zero static area refuses.
Exact closure is a dynamic G2 operation with a moving-state connection and
disposal receipt"): the *acoustic* areas still never reach zero, and no
mode is removed (no rank change). What POC-3 does is the flow side of G2. The
closed column is sealed by configuration, the move uses the mass-momentum
connection, and its disposal is booked: 1.3e-6 J over the whole /apa/.
The rank change (a true ω = 0 mode, or a topology edit of the network) is
still open.

## Gate 1: the linear limit reproduces Phase A exactly: PASS

- Plan with the flow terms absent: digest
  `sha256:5617b467134488c2747538b8899abffb7f659356a40eacaeb44f21b62921419c`,
  equal to CHECKPOINT-2A's (`tests/air.rs`
  `without_flow_terms_the_world_is_the_checkpoints`).
- Render fingerprints (`examples/render_hash.rs`, FNV-1a 64 of the f32
  output) at `checkpoint-2a-20261020` and on this commit: held /a/
  `b73c2e4694381c18` = `b73c2e4694381c18`; /aiua/ glide
  `a7bd4971535137af` = `a7bd4971535137af`.
- Aero law present with every coefficient at zero and no contact: the same
  bits as absent (`9d7c19c91a84f790` both; with the flow on `5f470efb8b96409e`).
- The steel-string tour, re-rendered with this commit's `mus`: WAV bytes,
  `renderDigest` and `planDigest` identical to base.
- `cargo test --release -p mus-ariadne --lib` 58 passed, `--test nd_configuration`
  6 passed, `-p ovillo-tract-world` `laws` 6 + `air` 3 passed.

## Gate 2: the ledger still closes: PASS

Max frame residual `8.9e-16 J` and cumulative `≤ 9.3e-13 J` in every run
below, against 3 J of lungs (`tests/air.rs`
`books_close_and_the_jet_pays_for_its_noise`: /a/, /s/, /a/, /p/ closure,
/a/: jet loss `5.9e-2 J`, turbulence `4.3e-5 J`, cumulative `-6.0e-14 J`).

## Gate 3a: one measured fricative spectrum and its energy budget

Articulation tongue index 0.95, diameter 1.0, lips 0.6, velum 0
(a tongue-tip constriction); folds abducted (tenseness 0), breath 0.8
(800 Pa). Last 0.5 s of a 1 s render (`fricative-air.wav`).

State: flow aperture 0.123 cm² at junction 33 (s ≈ 0.76); jet 343 cm³/s
(the glottis delivers 359); 27.9 m/s; Mach 0.081; Re 7,387. The folds do not
oscillate: voiceless.

Spectrum (8192-point LTAS, 1/3-octave levels relative to the loudest band):

| band (Hz) | 281 | 561 | 1122 | 2245 | 2828 | 3564 | 4490 | 5657 | 7127 | 8980 | 11314 | 14254 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dB | -47.6 | -44.0 | -44.2 | -41.7 | -31.9 | -13.6 | -14.2 | -8.5 | -3.1 | -2.7 | 0.0 | -4.0 |

Centroid (0.5-12 kHz) 9.07 kHz; peak 11.7 kHz. The same render without the
turbulent source is silent (every band below -131 dB re the same scale,
against +32 dB with it). The noise *is* the turbulence: the jet alone,
being quasi-steady, makes no sound. I make no claim that this spectrum is an
/s/. Its rise above 3 kHz is the front cavity's and the radiation's shaping
of a white dipole source; no spectral shape is given to the source itself.

Energy budget over the 0.5 s (J; `receipts/poc3-receipt.json`
`fricative.air.energy_budget`):

| flow | J |
|---|---|
| breathing muscles (source work) | 0.1377 |
| lungs → glottal jet + field (`ps U dt` + fold) | 0.1377 |
| glottal jet loss | 0.0119 |
| work into the tract at the glottis (`p̄ U dt`) | 0.1258 |
| constriction jet loss (separation) | 0.0807 |
| of which turned into sound by the turbulent force | 0.000270 (0.33%; η at M 0.081 is 0.22%, the rest is the force's zero-mean cross term with the steady flow) |
| wall loss (column momenta) | 0.0449 |
| radiation, booked (mode-diagonal) | 0.000502 |
| radiation, physical `Σ G p_end² dt` | 0.000089 (without the source: 0.000010) |
| ledger: cumulative / max frame residual | -3.9e-14 / 8.9e-16 |

## Gate 3b: one burst and its energy budget

/apa/: /a/ (tenseness 0.6, breath 0.6, F0 tuning 120 Hz), lips commanded to
0 from 0.30 to 0.48 s, then back (`plosive-apa.wav`).

- The lips reach contact at 0.424 s; the release comes at 0.481 s (57 ms
  sealed). During closure the output is silent (rms over the 30 ms before
  release `3.7e-9`). Oral pressure (mid-tract volume) rises to the lungs'
  600 Pa, peaking at 636 Pa (the decelerated column overshoots). Voicing
  stops (glottal flow 10 ms before release `-1.4 cm³/s`).
- Stored in the field at release: **53.9 µJ**. The closed-form compression
  energy of the cavity, `V p²/(2ρc²)` with V ≈ 3 cm² × 15 cm and p = 600 Pa,
  is ≈ 57 µJ.
- Release (1 ms trace): oral pressure 600 → 527 → 87 → 38 Pa, jet flow
  0 → 104 → 326 → 303 cm³/s, then a decaying pressure oscillation; voicing
  resumes about 7 ms later. Peak output in the 20 ms after release 0.093,
  against the vowel's rms 0.014.
- Burst budget, 20 ms from release (µJ): field 53.9 → 15.9; constriction jet
  loss 158.9; turbulence 1.5; glottal work 545.7 (voicing resumed); wall loss
  452.9; radiation booked 10.3 / physical 4.2; control work +37.8 (the lips
  opening); disposal over the whole run 1.3.

## Law, approximation, design hypothesis

| piece | status |
|---|---|
| separation loss `ρ|u|u/(2A²)` | quasi-steady free-jet law, no recovery, `C_d = 1` (approximation of real jets) |
| exact energy of the loss and the source (implicit midpoint) | by construction |
| closure as configuration, sealed by a numerical 1e-13 m² | flow side of G2; the acoustic rank change is not done |
| dropping a sealed column's `L u` at moves | modelling choice: contact reaction, booked |
| turbulence efficiency 0.004 (M/0.1)³, Re 3000-6000, offset 3 columns | M³ exponent: Curle's law; constants: design hypotheses (copied) |
| white source spectrum | design hypothesis (no Strouhal shaping) |
| q-only wall loss / p-only radiation | approximation: right for static pressure and for each mode's decay. Wrong for steady flow: the wall loss charges 0.0449 J/0.5 s on 343 cm³/s, an equivalent ~260 Pa drop where Poiseuille would give ~0.3 Pa |
| mode-diagonal radiation | approximation. It drains static pressure: booked 0.50 mJ against 0.089 mJ physical in the fricative |

## Open problems this PoC surfaced

1. A wall-loss law that is right at DC and in band (boundary layer `∝ √ω`)
   inside a real-mode modal body. Frequency-independent loss either leaks
   static pressure (on pressures) or over-charges steady flow (on momenta).
2. Radiation that is exact both at the stiff lip-volume time scale (the
   conductance's `C/G ≈ 16 µs` is shorter than a sample) and at DC: the
   rank-1 projection is right at DC but under-damps formants about 3x at
   48 kHz, and the mode-diagonal law is the reverse.
3. Closure as a true rank change (G2): an ω = 0 mode, or a network
   topology edit, with a moving-state connection that needs no numerical
   seal.
4. A turbulent source whose work on the steady flow is separated from its
   sound (here a zero-mean cross term adds ~50% to the mean in the fricative,
   and makes the net sign random when the jet is slow).
