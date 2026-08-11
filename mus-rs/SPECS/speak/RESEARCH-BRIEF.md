# SPEAK — state-of-the-art survey brief

Context for the surveyor: we build a deterministic, receipt-carrying
physical-modeling instrument (extended Karplus-Strong string network,
Rust, renders ~10k short probes/hr/core, exact reproducibility from
~25-50-float control programs). Research bet under evaluation: this
instrument's control-program space is a superior ENCODER for speech and
singing on specific axes — bitrate (~hundreds of bps), editability/
disentanglement, lawfulness (no decoder hallucination) — while
conceding raw distortion to neural codecs. Architecture: Sobol render
banks -> amortized audio->program inverse network -> per-utterance
CMA-ES polish (the "Threader" recipe, already specced by our
collaborators; single-word evolutionary limit case already running).
Hardware reality: Apple Silicon Mac (16 cores, MPS) + occasional spot
CPU/GPU boxes.

Knowledge cutoff caution: our assistant's training ends January 2026;
it is August 2026. Prioritize discovering what is NEW since early 2026.

## Questions, per section — recommend 2-4 candidates each, with
## version/date, license, local footprint on Apple Silicon, quality
## reputation, and a one-line "for our use" verdict

A. **Open TTS for corpus generation.** Mass-generating diverse paired
   text/audio (many voices, rich prosody). Candidates to check and
   supersede: F5-TTS, Kokoro, StyleTTS2, Parler, XTTS, Fish, Orpheus,
   Zonos. License terms for using generated audio as training data.

B. **Open ASR / phoneme recognizers as intelligibility oracles.**
   For loss functions: phoneme-posterior access matters (CTC-style
   models preferable to seq2seq for scoring). Whisper lineage and
   2026 successors, wav2vec2-phoneme, MMS, Parakeet/Canary, anything
   newer. Footprint for in-loop scoring (thousands of calls/hr).

C. **Speech SSL encoders as representation baselines.** HuBERT, WavLM,
   w2v-BERT 2.0, 2025-26 entrants; the current probing-benchmark norm
   (SUPERB successors) our transfer-per-dimension claims would be
   judged against.

D. **Neural codecs as rate-distortion baselines.** EnCodec, DAC,
   SpeechTokenizer, Mimi, SemantiCodec, and the current low-bitrate
   frontier (what is SOTA at <1 kbps in mid-2026?); standard eval
   protocol (ViSQOL/PESQ/UTMOS/MUSHRA norms) we should adopt.

E. **Articulatory synthesis and inversion.** VocalTractLab and
   successors, Pink Trombone lineage, EMA-based speech inversion,
   "articulatory codec/encodec" papers 2023-26 — the literature our
   bet most directly extends. Who has claimed articulatory features
   as speech codes, and what did they measure?

F. **Analysis-by-synthesis / sound matching.** DDSP and successors,
   differentiable physical modeling 2024-26, synthesizer parameter
   estimation (InverSynth/Sound2Synth lineage), evolutionary-vs-neural
   inversion comparisons. Anyone doing physical-model control programs
   as codes?

G. **Open singing synthesis** (our home-turf domain): DiffSinger,
   NNSVS, current SVS/SVC state and licenses.

H. **Eval instrumentation.** Intelligibility metrics (STOI/ESTOI,
   WER-as-metric norms), MOS predictors (UTMOS lineage), formant
   trackers in maintained OSS.

I. **License/ethics flags.** Voice-cloning restrictions relevant to
   corpus generation; anything that would taint an open-source release.

## Deliverable

One markdown report, sections A-I, each: candidates table + "for our
use" recommendation; then a closing section "What moved since early
2026 that the requesters likely do not know." Cite URLs throughout.
Rank confidence per recommendation (tried-and-true / reported / rumor).
