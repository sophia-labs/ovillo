# GARGANTA G7 — speaker-specific throat evolution from reference audio

**Status:** first executable reference implementation landed with this spec. The Python path is the oracle and search harness; production lowering into the unified Rust state/ledger remains a separate gate.

**Supersedes the narrow reading of G7.2 in `ROUND-5-GARGANTA-SPEC.md`.** Anatomy evolution has two first-class modes:

1. **inventory discovery** — evolve anatomies for contrast, robustness, and effort;
2. **speaker identification** — evolve an anatomy/source posterior specific to a provided, consented voice sample.

The second mode is the primary product requirement. A user gives GARGANTA a recording; GARGANTA returns a persistent throat that can subsequently be driven by new articulatory and F0 programs while preserving the inferred speaker identity.

---

## 1. Exact claim

The output is a **speaker-conditioned acoustic throat**, not a medical reconstruction of the person's biological vocal tract.

Audio-only inversion is non-identifying. Different area functions, source spectra, articulatory trajectories, microphones, and rooms can produce nearly the same waveform. Therefore the artifact is a posterior ensemble

\[
  p(a,\ell \mid y) / \sim_{\text{iso}},
\]

where `a` is static tract anatomy, `ℓ` is the speaker's laryngeal/source prior, `y` is the reference recording, and `~iso` is G7.3 isophonemy: indistinguishability under the declared control atlas and observables.

Every receipt must carry:

```json
{
  "claim": {
    "class": "acoustically_equivalent_speaker_throat",
    "medical_anatomy_reconstruction": false,
    "speaker_identity_model": true
  }
}
```

A single MAP anatomy may be convenient for rendering, but it is never allowed to erase posterior spread or weakly identified directions.

---

## 2. Input contract

The executable face is:

```bash
garganta-evolve reference.wav \
  --out out/vera-throat \
  --consent-attestation "speaker supplied this recording for model fitting"
```

Required input:

- one audio file, mono or stereo, with a content digest recorded in the receipt;
- at least several stable voiced frames spanning more than one vowel-like state;
- a provenance/consent statement for any distributable identity artifact.

Useful rather than mandatory:

- 8–30 seconds of relatively dry, clean, phonetically varied speech or singing;
- an optional transcript and phone alignment when the G8 alignment face lands;
- multiple takes in the same file, which materially improve anatomy/source separation.

The tool accepts shorter clips but emits uncertainty diagnostics. A 300 ms vowel can produce a timbral fit; it cannot support a strong speaker-throat claim.

---

## 3. The hierarchical inverse problem

The implementation must not put every degree of freedom into one optimizer. It separates three timescales.

### 3.1 Static speaker anatomy `a`

The v1 genome contains ten bounded coordinates:

- vocal-tract length;
- oral/pharyngeal length warp;
- four neutral log-area deformations (pharynx, tongue root, oral cavity, lips);
- speaker-specific front- and back-tongue mode scales;
- lip-area scale;
- distributed wall-loss coefficient.

The neutral tract is a positive smooth area function. Anatomy changes the baseline and the affordance map; it does not encode the utterance.

### 3.2 Dynamic articulation `q(t)`

The v1 control chart contains:

- jaw opening;
- front-tongue displacement;
- back-tongue displacement;
- lip rounding;
- apical constriction.

For section centers `x_i`, the fast model is

\[
  \log A_i(t)
  = \log A_{0,i}(a) + \sum_j q_j(t)\,B_{j,i}(a),
  \qquad A_i(t) > 0.
\]

The positive-area cone is enforced by construction. Near closure is represented by a small positive floor in the frequency-domain oracle; exact zero and cavity splitting belong to G2's rank-change runtime law.

### 3.3 Source and channel nuisances

The reference analysis records, but does not ask the tract to explain:

- median F0 and F0 spread;
- local jitter proxy;
- periodicity/aspiration proxy;
- broad source spectral tilt;
- low-order shared spectral slope attributable to source, microphone, and room.

The tract objective uses LPC-derived, smoothed spectral envelopes with shared quadratic log-frequency tilt projected out. This is an anti-cheating measure: microphone EQ must not become “anatomy.” G3 will replace the diagnostic source profile with a physical fold parameter posterior.

### 3.4 Bilevel objective

For static anatomy `a`, representative reference states `s`, and a canonical control bank `Q`, the reference loop minimizes

\[
\begin{aligned}
L(a) =
 &\operatorname{mean}_{s\in\mathcal T}
   \min_{q\in Q}
   \left[D(E(F(a,q)), E(y_s)) + \lambda_W W(q)\right] \\
 &+ \lambda_H
   \operatorname{mean}_{s\in\mathcal H}
   \min_{q\in Q}
   \left[D(E(F(a,q)), E(y_s)) + \lambda_W W(q)\right] \\
 &+ \lambda_A R_{\text{human}}(a)
  + \lambda_S R_{\text{smooth}}(a).
\end{aligned}
\]

`T` and `H` are disjoint calibration and held-out acoustic states from the same clip. `F` is the fast transmission-line map; `E` is the tilt-normalized envelope observable. The outer loop is a seeded evolutionary search with a Sobol exploration tranche. Champion controls receive local continuous polish after anatomy search.

This is deliberately not raw log-mel imitation. The same static anatomy must explain a family of acoustically separated states while articulation is allowed to move.

---

## 4. Fast forward model

The Python oracle uses a frequency-domain Webster/transmission-line chain. Each uniform section has characteristic impedance

\[
  Z_i = \rho c/A_i
\]

and propagation matrix

\[
M_i(\omega)=
\begin{bmatrix}
\cosh(\gamma\Delta x) & Z_i\sinh(\gamma\Delta x)\\
Z_i^{-1}\sinh(\gamma\Delta x) & \cosh(\gamma\Delta x)
\end{bmatrix}.
\]

The product is terminated by an unflanged-pipe radiation approximation at the lips. The returned observable is lip volume velocity divided by glottal volume velocity. This is the G6.1 fast map for the first implementation, not the final certified runtime.

The same module exposes the Kelly–Lochbaum reflection coordinates

\[
  r_i = \frac{A_i-A_{i+1}}{A_i+A_{i+1}}, \qquad |r_i|<1,
\]

which are the bridge into the production scattering executor.

Zero/limit obligations for the Rust lowering remain those in G1.2:

- equal adjacent areas imply zero junction reflection;
- a uniform closed–open tube approaches the quarter-wave series;
- zero articulator velocity recovers the static transfer map;
- transparent adjacent junctions collapse to the shorter equivalent tube;
- all time-varying updates carry `W_metric` receipts.

---

## 5. Reference analysis

`analyze_reference_audio` performs a deterministic first-pass analysis:

1. mono conversion, peak normalization, and resampling to 16 kHz;
2. 30 ms frames / 10 ms hop;
3. energy gate plus autocorrelation F0/periodicity gate;
4. pre-emphasized LPC envelope and formant extraction;
5. source-tilt estimate from the raw frame spectrum;
6. deterministic farthest-point selection in formant-plus-envelope space;
7. fixed train/holdout partition of representative states.

The representative-state atlas is intentionally content-agnostic in v1. G8 will replace or augment it with phone-aligned states and explicit novel-phone tests.

---

## 6. Posterior and isophonemy

The outer search retains an archive, not only the champion. Candidate anatomies are deduplicated using a symmetric spectral-atlas distance:

\[
 d_{\mathrm{iso}}(a,b)=\frac12\left[
  \frac1{|Q|}\sum_{q\in Q}\min_{q'\in Q}D(F(a,q),F(b,q'))
  +
  \frac1{|Q|}\sum_{q'\in Q}\min_{q\in Q}D(F(b,q'),F(a,q))
 \right].
\]

This is observable-relative isophonemy, not geometric equality. Posterior members receive normalized Boltzmann weights from objective loss after deduplication. The receipt also reports a local profile-curvature rank and names weak anatomy directions.

The correct behavior for an under-informative recording is a wide posterior and low identifiability rank—not false precision.

---

## 7. What makes the throat speaker-specific

Fitting the supplied clip is necessary and insufficient. The binding test is **frozen-anatomy generalization**.

After fitting:

1. freeze `a` and the inferred laryngeal prior `ℓ`;
2. discard the calibration articulation paths;
3. drive the throat with held-out phone/control sequences;
4. drive it with F0 trajectories not present in the reference;
5. score content and speaker identity separately.

A model counts as speaker-specific only when:

- held-out content remains intelligible under the G8 machine-listener protocol;
- speaker-verification similarity to the reference exceeds a preregistered threshold;
- similarity survives F0 normalization and a prosody-swap control;
- a mismatched anatomy from another speaker scores materially worse under matched content/source;
- room/channel perturbations do not substantially change the inferred anatomy posterior.

Same-content spectral reconstruction alone is capped at `resemblant`.

---

## 8. Acceptance gates

### G7-S0 — executable reference oracle — **LANDED**

- `mus_analysis/garganta_anatomy.py`
- `mus_analysis/garganta_cli.py`
- `schemas/garganta-speaker-throat.schema.json`
- `tests/test_garganta_anatomy.py`

The tool accepts a reference audio file, evolves a static anatomy against multiple states, records held-out loss, returns an isophonemy-deduplicated posterior, and writes a diagnostic atlas WAV.

### G7-S1 — synthetic recovery

Generate a corpus from known anatomies and articulation paths. The true anatomy need not be the unique MAP member, but it must lie inside the preregistered posterior credible set under the declared isophonemy metric. Report coverage, tract-length error, and held-out envelope error over at least 100 seeds.

### G7-S2 — same-speaker repeatability

Fit two disjoint recordings of the same speaker. Their posterior atlases must be closer than matched-duration recordings from different speakers, with a preregistered effect size and bootstrap interval.

### G7-S3 — held-out phonetic coverage

Fit on one phone subset and optimize only articulation on held-out phones. Anatomy remains frozen. Report envelope/formant error, phone-recognition accuracy, and refusal rate for unreachable targets.

### G7-S4 — novel-content identity

Render at least ten unseen sentences or sung phrases. Report dual-ASR WER, speaker-embedding similarity, and human ABX identity judgments. This is the first gate capable of promoting to `speaker-faithful`; machine metrics alone do not promote above the machine-listener evidence class.

### G7-S5 — source/tract disentanglement

Cross the inferred tract of speaker A with source/prosody priors from speaker B and vice versa. The experiment must show which identity cues follow anatomy and which follow the larynx. No single “voice embedding” may silently absorb both.

### G7-S6 — channel invariance

Repeat fits after convolution with preregistered room impulses, mild EQ, and microphone responses. Posterior movement must remain below a threshold or the receipt must state channel dependence.

### G7-S7 — real-anatomy calibration

On speakers with MRI/rtMRI area functions, test whether the audio-only posterior contains the measured area-function atlas up to the declared geometry/observable map. This is the gate for any stronger anatomical interpretation; until then `medical_anatomy_reconstruction=false` is permanent.

---

## 9. Artifact contract

The primary artifact is `speaker-throat.json`, validated by `garganta-speaker-throat.schema.json`. It contains:

- input digest and consent/provenance;
- exact search configuration and seed;
- reference feature states and train/holdout split;
- MAP anatomy;
- posterior ensemble and weights;
- per-state fitted controls, modeled formants, and area functions;
- training and held-out losses;
- identifiability rank, weak directions, and diagnostics.

The CLI additionally writes:

- `evolution-progress.jsonl` — generation-by-generation receipt;
- `summary.json` — compact stop-condition report;
- `speaker-throat-atlas.wav` — diagnostic static-state preview.

The atlas renderer uses a declared Rosenberg-like diagnostic pulse. It is not the production G3 fold source and cannot support a naturalness or speaker-faithfulness claim.

---

## 10. Production lowering

The reference implementation deliberately stops at the seam where the unified packet must take over.

1. Lower each fitted area function to KL reflection coordinates.
2. Instantiate the tract as a persistent fixed-capacity region in `mus-watsn`/`mus-dsp`.
3. Compile articulation paths as time-varying junction schedules through `mus-gesture`.
4. Charge moving-geometry work to `W_metric`.
5. Couple the G3 fold state bidirectionally at the glottal port.
6. Add G2 closures/velum as rank/topology changes with disposal receipts.
7. Replace the Python radiation/loss approximations with versioned certified laws.
8. Replay the Python oracle fixtures against Rust within forensic tolerance.

The Python artifact is therefore not disposable research code. It is the semantic oracle and fixture generator for the production implementation.

---

## 11. Literature anchors

The design is continuous with established and current work while making a narrower claim than medical inversion:

- Story, Titze & Hoffman (1996), *Vocal tract area functions from magnetic resonance imaging*, JASA 100:537–554, DOI `10.1121/1.415960` — speaker-specific area-function catalogues.
- Story (2005), *A parametric model of the vocal tract area function for vowel and consonant simulation*, JASA 117:3231–3254, DOI `10.1121/1.1869752` — neutral substrate, spatial modes, consonant superposition.
- Story (2009), *Vocal tract modes based on multiple area function sets from one speaker*, JASA 125:EL141, DOI `10.1121/1.3082263` — speaker-specific mode atlases and inverse mapping.
- Lammert & Narayanan (2015), *On short-time estimation of vocal tract length from formant frequencies*, PLoS ONE 10:e0132193, PMID `26177102` — audio-derived VTL with MRI validation and explicit uncertainty.
- Cámara et al. (2023), *Vocal Tract Area Estimation by Gradient Descent*, DAFx — differentiable physical area inversion.
- Cho et al. (2024), *Coding Speech through Vocal Tract Kinematics* (`arXiv:2406.12998`) — explicit separation of articulatory trajectories and speaker identity in a universal speech code.
- Azzouz, Vuissoz & Laprie (2026), *Acoustic-to-Articulatory Inversion of Clean Speech Using an MRI-Trained Model* (`arXiv:2603.11845`) and *Complete Vocal Tract from RT-MRI* (`arXiv:2603.28723`) — current complete-tract inversion evidence and the importance of speaker/data regime.
- Lu & Reiss (2026), *Learning Vocal-Tract Area and Radiation with a Physics-Informed Webster Model* (`arXiv:2602.13834`) — current physics-informed area/radiation fitting from audio.

The open cell GARGANTA occupies is not “audio can predict articulators.” It is: **a deterministic, inspectable, work-accounted physical throat inferred as an uncertainty-bearing anatomy posterior and then reused as a persistent musical/speech instrument.**
