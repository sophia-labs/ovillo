# THREADER — the inverse instrument

**The SPEAK keystone.** Binding sources: `ROUND-5-GARGANTA-SPEC.md` §1
(the speech claim lattice governs every claim here), `SOTA-SURVEY.md`
(baselines, oracles, licenses), `ENRICHMENT-SURVEY.md` (annotation
stack), `SPECS/garganta/GARGANTA.md` (the measured walls), and the
first executable: `mus-rs/experiments/threader_poc/` (merged from
`agent/threader-poc-v0`; its CI is the acceptance harness).

## 0. The bet

From the round-4 ask, sharpened by the SOTA survey's empty claims cell:
**no existing system offers explicit-bitrate + interpretable physical
controls + deterministic decode, for speech and singing at once.** Our
wager: control programs for a physical instrument are a superior
*encoder* on those named axes — rate (hundreds of bps), editability
(the code IS articulation), lawfulness (every decode carries receipts)
— while conceding raw distortion to neural codecs. Threader is the
machine that takes audio in and emits programs: the inverse instrument.

## 1. Architecture (proven in miniature, 2026-08-11)

The POC implements the commitments; production scales them, never
trades them away:

```text
K recordings of one voice + confidence-weighted analysis bundle
        │
        ▼
audio + bundle encoders
        │
  ┌─────┴────────────┐
  ▼                  ▼
shared posterior   utterance-local programs
(anatomy/voice)    (gestures, source, prosody)
  │                  │        + nuisance channel (mic/room —
  └─────┬────────────┘          never part of the voice)
        ▼
physical decoder (a declared class, §2)
        ▼
reconstruction → multiscale masked losses → amortized training
        ▼
MAP refinement UNDER THE POSTERIOR TRUST REGION
```

**Design law (measured, not argued):** the POC's unconstrained polish
improved acoustics while nearly doubling anatomy error; the
trust-region refinement halved the fit objective while *holding*
anatomy (0.2999 → 0.3105). Pure sound-matching destroys
identifiability — the ventriloquist degeneracy caught in a third
independent codebase, after the speaker-throat posterior's weak
directions and round-5's G6.3 prior. Production Threader is MAP
inference under physical/program priors, permanently. A build that
drops the prior for a better loss number is a regression, whatever the
loss says.

The three-way split is likewise law: **shared** (who is speaking —
anatomy, source priors) / **local** (what is said — the program) /
**nuisance** (how it was recorded). Bits are accounted per stratum;
identity amortizes across a voice's corpus, programs pay per utterance,
nuisance is never encoded as voice.

## 2. Decoder classes

A Threader claim names its decoder class explicitly (their N17: no
silent claim transfer between classes):

| Class | Decoder | Status |
|---|---|---|
| D0 | POC toy surrogate (6-section all-pole, 4 kHz) | landed; architecture tests only |
| D1 | `mus_analysis.garganta_anatomy` oracle (Kelly–Lochbaum transfer, diagnostic pulse) | landed; fitted-throat mimic runs here |
| D2 | the mus engine via `renderWeaveProbe` / `renderBraidProbe` (Rust, receipted, deterministic) | landed; the V0 bank's renderer |
| D3 | the unified U3+ backbone voice with G1–G3 machinery | pending on the mathematicians |

Results never promote across classes. The production claim lives at
D2/D3, where every decode carries a render digest.

## 3. Data plane

- **Threader V0 bank** (landed): 23,600 receipted renders — manifest
  -first (probeKey + renderDigest self-verifying), Sobol over the
  PARAMETERS §8 safe region + PW10 palette, contrast cells, witness
  stratum with compiled closed words. The instrument-side corpus.
- **Speech corpus** (per `SOTA-SURVEY.md`, licenses binding): TTS
  generation with the phoneme sequence logged at generation time
  (Kokoro/misaki; Qwen3-TTS for voice diversity) — align, don't
  re-guess. Seeds + params in the manifest; dedup free by construction.
- **Enrichment** (per `ENRICHMENT-SURVEY.md`): ZIPA-CR emission
  matrices double as free phone alignments on the intelligibility pass
  we already pay for; silero VAD once, two consumers; SenseVoice AED
  rides the ASR gate with jrgillick as the license-clean laugh-span
  refiner; Praat behind a subprocess boundary (GPL never linked). The
  generation-side condition is the affect label of record; post-hoc SER
  verifies, never annotates.
- **License discipline:** weights-license per checkpoint; NC/ND taints
  are corpus-fatal; the manifest records attribution classes (T0–T2).

## 4. Evaluation

The **speech claim lattice** (round-5 §1) governs: resemblant →
segmentally-structured → oracle-intelligible → human-intelligible →
prosodically-natural → speaker-faithful, with machine-listener evidence
capped at level 3, always.

- **Standing floors** (same 64-band log-mel loss, same target, matched
  ~7k budgets — the two-machine plateau):

  ```text
  rung-1 generic instrument + gesture ES     1.9204
  fitted speaker throat     + gesture ES     2.0162
  ```

  Both machines converge on the same wall; the wall is G1/G2/G3
  (transitions, bursts, folds), measured from two directions. Every
  decoder-class or machinery advance re-runs the mimic and attributes
  its delta.
- **Bitrate accounting:** program bits per second, explicit, per
  stratum (shared amortized / local / nuisance-discarded). Baselines:
  FocalCodec, Mimi, NanoCodec at their published rates; ours reported
  beside them with the interpretability and determinism columns filled
  in — the claims cell we occupy.
- **Intelligibility oracle:** ZIPA-CR IPA-CTC, content-addressed pin,
  dual-family WER normalization per the survey.
- **Identifiability receipts:** posterior coverage (the POC's 92.7% on
  nominal 90% is encouraging and explicitly not yet a calibration
  claim), weak-direction naming (the speaker-throat receipts' idiom),
  isophonomy-quotiented anatomy claims only.

## 5. Scale-up gates

- **S0 — POC landed.** Done (merged; its CI runs pytest + a training
  smoke per push).
- **S1 — D1 swap.** The POC's surrogate replaced by the
  `garganta_anatomy` oracle; gradient-free outer loop is acceptable
  (Instrumental 2026: CMA-ES beats gradients through simulators) with
  the amortized net proposing and the oracle disposing. Gate: held-out
  fitted-throat mimic ≤ the ES-only 2.0162 at equal render budget.
- **S2 — real corpus.** TTS corpus + enrichment pipeline at 25k
  utterances; manifests validate; license ledger clean. Gate: oracle
  WER measured end-to-end through D1 decode, number recorded whatever
  it is.
- **S3 — D2 decode.** Programs render through the mus engine with
  receipts; the V0 bank becomes training data for the instrument side.
  Gate: byte-reproducible decode from program + digest.
- **S4 — scale training.** Spot-box budget; **needs Vera's explicit
  nod, named here so it cannot be assumed.** Gate to unlock: S1–S3
  green and a written estimate of box-hours.
- **S5 — D3.** When the mathematicians land G1–G3, the plateau either
  falls or the bet is wrong. Either way we publish the number.

## 6. Refusals (standing)

- No waveform-autoencoder drift: a model that stops emitting
  *programs for a physical decoder* is not Threader, whatever its SNR.
- No claim above oracle level without a preregistered human study.
- No sound-matching refinement outside the posterior trust region.
- No anatomy claim finer than its isophonomy class.
- No cross-class result promotion (D0 numbers never advertise D2).
- No NC/ND-tainted audio in any training corpus.
- No training-scale spend without the S4 nod.

## 7. Open dependencies

On the mathematicians: G1 fast-passage (the wall's largest brick), G2
closure/burst laws, G3 the fold source (D1's pulse is declared
diagnostic), G6 quantal atlas + fast formant map with adjoints, round-4
II.3 rate-distortion bounds for physically-constrained decoder classes
— the theory that would let us state the bet's limits before the
empirical comparisons land.
