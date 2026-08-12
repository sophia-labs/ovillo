# GARGANTA — the instrument grows a throat

The historical arc: Kelly-Lochbaum modeled the vocal tract as a chain
of waveguide segments with scattering junctions in 1962; waveguides
begat Karplus-Strong begat the hilo. The string came out of the throat;
this program walks it home. Three rungs; rung 1 lands with this spec,
rungs 2-3 are the round-4 handoff to the mathematicians.

## Rung 1 — the bowed vowel (LANDED with this commit)

The voice is source-filter. We own both halves in primitive form:
- Source: the bow — the engine's one self-sustained, pressure-driven
  nonlinear oscillator, the same species of thing as vocal folds
  (flow-valve / stick-slip cousins). `breath` is aspiration.
- Filter: five vowel body profiles (`bodyp=a|e|i|o|u`) — formant
  patterns as modal tables (F1-F4, Peterson/Barney neighborhood, plus a
  shared singer's-formant mode at 2.9 kHz). Design profiles, not
  measurements, per registry law. All compat and distinctness laws
  hold (45-pair minimum spectral distance 0.40 vs 0.25 floor; guitar
  byte-law untouched).

A bowed string wearing a vowel IS the primitive singer. Vowel
sequences: `[bodyp=...]` per event. Diphthongs: crossfade two
vowel-bodied tracks in-language until a body-morph seam earns its way
into the engine.

Known honest gaps at rung 1: glottal-pulse asymmetry and spectral tilt
(the bow's spectrum is not a glottal source's — declared
approximation); no formant glides within a note; nasality absent.

## Rung 2 — string-as-larynx (engineering, next drill)

One course IS the larynx: bowed, tuned to F0, its output injected
bridge-common into the rest of the network (the sing seam's
architecture, sourced from inside the instrument). Source and filter
as regions of one graph — consensus §8 with "throat" added. Needs: an
internal routing seam (course output -> bridge drive), dry/wet on the
injection (the queued sing dry/wet generalizes), and la cantora's next
draft rewritten larynx-first.

## Rung 3 — the true tract (round-4 handoff)

A Kelly-Lochbaum tract in our language: a chain graph of ~20 waveguide
segments whose junction scattering coefficients ARE the articulators.
The Phase 2 gesture seam already applies scattering rotations at
declared junctions at declared samples; the mus-gesture compiler
already synthesizes certified paths through coefficient space.
Articulation = a control path through a scattering network.

Asks for the mathematicians:
1. Junction parameterization: KL reflection coefficients from area
   ratios, mapped onto the plane-rotation form the executor speaks —
   the exact correspondence, and what the energy ledger says about a
   moving tract (time-varying scattering is their home territory;
   Phase 0's laws apply).
2. Articulatory gestures as control paths: vowel-to-vowel transitions
   compiled with thermodynamic timing; "say /aia/ as a closed loop and
   measure its monodromy" is now a well-posed sentence — is it a
   musically meaningful one?
3. Side-branch topology (nasal stub course) and its rank/disposal
   semantics when the velum opens — a topology edit under their §7
   policies.
4. A glottal source port with an honest work contract — vocal folds
   are an energy source like the bow; the nonlinear source-port ledger
   queued for round 4 covers both.

Area-function data for real vowels exists (Story et al. MRI corpora) —
rung 3 tables should cite it and keep the design-profile honesty labels
until a fit is actually performed.

### Rung-3 product contract — a throat from a provided voice sample

The true tract is not only a hand-authored synthesizer. Its first
product-facing inverse mode is:

```text
consented voice recording
  -> static speaker anatomy posterior
  + dynamic articulation paths
  + separate laryngeal/source prior
  + declared channel nuisance
  -> one persistent throat reusable on new material
```

The output is an **acoustically equivalent speaker throat**, never an
unqualified biological reconstruction: one waveform does not uniquely
identify anatomy. GARGANTA therefore returns an isophonemy-deduplicated
posterior ensemble, a MAP member for rendering, held-out fit receipts,
and named weak anatomy directions.

The binding test is frozen-anatomy generalization. Fit on the supplied
sample, discard its articulation paths, and drive the inferred throat
with phones and F0 trajectories absent from the sample. It is
speaker-specific only if identity survives while content and prosody
change; same-clip spectral imitation remains merely `resemblant`.

The executable reference protocol and first implementation live in
`SPEAKER-THROAT-EVOLUTION.md`; the round-5 binding amendment is
`.string-research/ROUND-5A-SPEAKER-THROAT-AMENDMENT.md`.

---

## Rung-1 intelligibility results (2026-08-11, appended same day)

1. **Hand-authored "apple"**: segmentally structured (vowel, closure,
   burst, schwa, lateral — spectrogram shows true stop anatomy) but
   NOT intelligible as the word (listener n=1: Vera, negative). The
   burst rings pitched (a pluck, not aperiodic release); the vowels
   are a harmonic comb at constant musical F0 (speech-mode gating cue
   absent); formant transitions absent. Word-intelligibility is
   UNCLAIMED at rung 1 and currently absent.
2. **Evolutionary mimic floor** (speak_mimic.py, 220 gens x 32 against
   a TTS target, 64-band log-mel MSE): 3.33 -> 1.92, plateau over the
   final ~40 generations. **~1.9 log-mel units is the measured rung-1
   residual** — the quantified size of what the engine cannot say
   without formant transitions, a true noise burst, and F0 prosody.
   This number is the empirical case for rungs 2-3 and the baseline
   every later rung must beat.
3. Priority order this fixes for rung 2/3: (a) formant transitions
   (tract morphing / articulated junctions), (b) aperiodic burst
   source, (c) F0 prosody + speech register (musical decorations off).

## The two-machine plateau (2026-08-11, evening — unified-SOTA measurement)

Same target, same 64-band log-mel loss, matched render budgets (~7k):

```text
rung-1 generic instrument + gesture ES        1.9204   (220 gens, plateaued)
fitted speaker throat    + gesture ES         2.0162   (350 gens, plateaued;
                                                        diagnostic pulse source)
warm start (fitted states, no gesture)        3.6384
```

Two architectures reach the same **order of aggregate spectral
residual** at ~1.9-2.0. What the run establishes: the fitted throat is
usable in a word-level optimization loop; the receipt's static state
fits are not an utterance trajectory (the dynamic source/articulation
solve bought 1.62 points, 44.6% of the warm start); and a large
residual survives thousands of evaluations on both machines.

**Attribution correction (mathematicians' review, 2026-08-11, accepted
in full):** the 3.64 → 2.02 improvement is a *gesture/source-program*
effect on a FIXED anatomy — the anatomy never varied, so this
measurement does not isolate the resonator's contribution, and the
first recording of this section overclaimed it. Nor is 1.9204 vs
2.0162 a controlled architecture comparison: the generic run carries a
hand-authored word scaffold with an exact closure (the fitted run has
none), budgets differ per dimension (282 vs 131 evals/dim), and the
unified renderer has two confessed confounds — per-state peak
normalization (erases the absolute attenuation G2 physics) and
overlap-add without a weight buffer (1024 Hann at 480 hop is not
COLA). Evidence class: **exploratory construction-oracle
convergence**, per `TWO-MACHINE-PLATEAU.md` (binding G7/G8 amendment,
PR #1), which defines the staged causal program: P0 renderer
validation gates everything; P1 anatomy counterfactuals (MAP vs
neutral/shuffled/random/posterior under paired seeds) is the most
important immediate experiment; P2-P6 bound G1/G2/G3 separately and
replicate before any wall is called shared.

G1/G2/G3 remain the priority order as *hypotheses with engineering
value* — supported, not yet individually confirmed. Tool:
`mus-rs/tools/speak_mimic_unified.py`; run receipts committed beside
this file's artifacts; audio in the listening folder
(`garganta-evolved/`).
