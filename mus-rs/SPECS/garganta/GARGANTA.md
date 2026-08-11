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
