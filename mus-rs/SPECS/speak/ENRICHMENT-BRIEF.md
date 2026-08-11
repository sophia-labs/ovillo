# SPEAK enrichment brief — forensic decomposition and paralinguistic
# annotation of input audio

Context: sibling brief to `RESEARCH-BRIEF.md` (see `SOTA-SURVEY.md` for
the generation/baseline stack already chosen — treat its license
discipline as binding: weights-license per checkpoint, NC/ND outputs
taint, Apple Silicon 16-core + MPS local, occasional spot boxes).

Purpose, two consumers:
1. **SPEAK's encoder**: the inverse network learns better from richly
   annotated targets — phone boundaries, prosodic events, voice
   quality, paralinguistic events — and the control-program space
   should be *conditioned* on these annotations (a laugh should map to
   a recognizable region of program space).
2. **GARGANTA's expressive ladder**: the instrument will eventually
   laugh, sigh, breathe, and creak — as control programs. To design
   those we need forensic decompositions of real laughs, sighs, and
   creaks: what are they made of, acoustically, at the segment level?
   ("How do models know how to laugh" — what annotation and data made
   that possible, and what is the minimal version for an instrument?)

Deliverable format identical to the first survey: per section a
candidates table (version/date, license, Apple Silicon footprint,
maturity) + a "for our use" recommendation with confidence tags
(tried-and-true / reported / rumor) + citations; closing section "what
moved since early 2026." Assume knowledge cutoff ~Jan 2026; verify by
web. It is August 2026.

## Sections

A. **Alignment & segmentation at every granularity.** Word / syllable /
   phone forced alignment (MFA 3.4 lineage, torchaudio CTC alignment
   with label priors, whisperX-style word timestamps, charsiu);
   syllabification; VAD (silero and successors); pause/breath-group
   segmentation. What is the 2026 best practice for phone-level
   boundaries on (a) clean TTS output, (b) expressive/sung audio?

B. **Prosody extraction & event annotation.** F0 trackers ranked
   (penn, SwiftF0, praat/parselmouth, 2026 entrants); energy/duration
   lattices; prosodic event detection (pitch accents, boundary tones —
   what replaced the AuToBI lineage?); emphasis/prominence detection;
   speech-rate estimation. Output schema norms (e.g., per-syllable
   prominence scores).

C. **Voice-quality forensics.** Jitter, shimmer, HNR, CPP; creak/
   vocal-fry detection; breathiness measures; glottal source analysis
   — GCI detection, inverse filtering (IAIF/QCP lineage, anything
   neural and newer): can we recover usable glottal flow estimates
   from TTS/real audio at scale? (Direct consumer: GARGANTA's glottal
   port design — we want empirical pulse shapes and OQ/tilt ranges.)
   Formant tracking that survives high F0 and singing.

D. **Nonverbal vocalizations.** Laughter detection (state of the art
   post-laughter-detection-repo era); breaths, sighs, coughs, fillers,
   cries; taxonomies of affect bursts; datasets (VocalSound lineage,
   laughter corpora, 2025-26 additions) WITH licenses; how expressive
   TTS systems tag these ([laugh]/[sigh] token conventions across
   Chatterbox/Dia2/etc.) and what their training annotation looked
   like.

E. **Emotion / affect annotation at scale.** Open SER: categorical and
   dimensional (valence/arousal/dominance); 2026 leaders with clean
   licenses; reliability caveats (cross-corpus generalization is
   historically poor — current state?); per-segment annotation
   throughput on our hardware.

F. **Corpus hygiene & tagging.** SNR/bandwidth/clipping estimation,
   artifact detection for filtering generated corpora; CLAP-style
   audio tagging for stratification; source separation (voice/music)
   if singing corpora need it; dedup/near-dup detection for audio.

G. **Word & subword decomposition.** G2P (espeak-ng, g2p variants,
   charsiu, 2026 state), IPA normalization across languages, lexical
   stress marking, syllable structure; anything that gives us
   morpheme/phoneme-class lattices to condition on.

H. **How models learned to laugh — the synthesis-side survey.** What
   data + annotation regimes gave expressive TTS its paralinguistics
   (token tags vs style embeddings vs natural data abundance)? What is
   the minimal annotation set a NEW voice (ours: an instrument) needs
   to learn a convincing laugh? Include: acoustic anatomy of laughter
   from the phonetics literature (bout/call structure, voicing
   fraction, rhythm ~4-6 Hz) — the design sheet for a control-program
   laugh.

I. **Pipeline recommendation.** A concrete annotation stack (ordered
   passes, throughput estimates at 25k-500k clips on our hardware), a
   manifest schema extension (columns to add to threader-bank-style
   manifests), and license roll-up. Prefer few tools that compose over
   many that don't.
