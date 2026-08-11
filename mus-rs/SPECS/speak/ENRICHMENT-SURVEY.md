# SPEAK — Enrichment Survey: Forensic Decomposition & Paralinguistic Annotation

**Survey window: 2026-08-11 / 2026-08-12.** Merged from three parallel scout passes (A: alignment/prosody/decomposition · B: voice-quality/nonverbals/laughter · C: affect/hygiene) against `mus-rs/SPECS/speak/ENRICHMENT-BRIEF.md`. Composes with `SOTA-SURVEY.md` (the generation/eval stack already chosen); its license discipline is treated as **binding**: weights-license per checkpoint, NC/ND outputs taint the corpus, Apple Silicon 16-core + MPS local frame, occasional spot boxes for heavy batch.

**Method.** ~75 web searches + ~50 primary-source fetches across three scouts. Licenses were verified this week on the actual repos / HF model cards (GitHub license API SPDX + HF `cardData.license`) for: MFA, FALCON, SOFA, BFA, silero-vad, TEN VAD, Respiro-en, SwiftF0, emphases, wavelet_prosody_toolkit, goruut, misaki, DisVoice, opensauce, DeepFry, GOLF, jrgillick, LaughterSegmentation, VocalSound, Deeply-NVV, PodcastFillers, emotion2vec+, SenseVoice, MERaLiON-SER, Vox-Profile, Empathic-Insight-Voice, audEERING dim models, Emotion2Vec-S, dataspeech, Brouhaha, DNSMOS, audiobox-aesthetics, LAION-CLAP, PANNs, Dasheng, CED, MSST, NeMo-SDP. Anything dated after January 2026 was verified by fetch unless explicitly marked otherwise.

**Confidence tags** (used throughout, per-claim): **tried-and-true** (verified primary source + community mileage) · **reported** (verified from primary/secondary source, no hands-on) · **rumor** (single weak source or unverified training knowledge). Classic phonetics constants that predate the knowledge cutoff and are stable science are tagged **(lit., training knowledge)**.

**Two negative findings are search-absences, not proofs:** no maintained open *neural* glottal inverse filter (§C), and no 2025-26 breakthrough in high-F0/sung formant tracking (§C).

---

## Composition note — read before the sections

Four facts about the already-chosen stack determine how this pipeline should be built, and they cut across every section below:

1. **ZIPA-CR + torchaudio label-priors forced alignment is already the chosen intelligibility oracle** (`SOTA-SURVEY` §B). The same emission matrices give phone alignments for free. One model, two jobs — the in-pipeline aligner costs approximately nothing on top of a pass we are already paying for.
2. **For TTS-generated corpora the phoneme sequence is known at generation time.** Kokoro runs misaki G2P internally; MFA-style re-inference is only needed for models with no phoneme interface (Qwen3-TTS is LLM-text-in) and for real/expressive/sung audio. **Log G2P output in the manifest at generation; align, don't re-guess.**
3. **The generation-side condition is the label of record for affect** (§E). March 2026 work shows SER models do not generalize to TTS-synthesized speech; post-hoc SER on our own corpus is a *verifier*, not an annotator.
4. **Dedup and provenance are nearly free on generated corpora** (seed + params in the manifest, `SOTA-SURVEY` §I). The hygiene problem for us is TTS *failure modes*, not Emilia-style in-the-wild cleanup — except on the singing/found-audio ingest path.

---

## Conflicts and disagreements between scouts — reconciled up front

The brief asks for explicit reconciliation. These are the places where the three passes disagreed, contradicted the sibling survey, or overlapped:

| # | Disagreement | Resolution |
|---|---|---|
| 1 | **SwiftF0 license.** `SOTA-SURVEY` §H lists it "Unknown"; Scout A verified **MIT** on the repo (Aug 2026). | **MIT stands.** The later, more specific repo fetch supersedes the earlier unknown. Note the benchmark-author-is-tool-author caveat separately (below). |
| 2 | **SenseVoiceSmall license.** Scout B: "Custom 'model license' — flag (not OSI)." Scout C: FunASR **Model Open Source License — commercial + redistribution OK**, attribution + no-disparagement clause. | **Both are right about different things.** It is a custom, non-OSI license *and* it permits commercial use and redistribution with attribution. Classify it **T2 (use-restricted, commercial-OK)**: usable in the annotation pipeline, attribution recorded in the manifest, not describable as "open source" in a model card. |
| 3 | **penn's role.** `SOTA-SURVEY` §H puts penn in the adopted eval stack; Scout A demotes it (−29 pts accuracy at 10 dB SNR on the 2026 pitch benchmark). | Not a contradiction: penn is fine on the *clean* TTS strata that dominate our corpus and is a legitimate agreement partner there. **Demote it only for noisy/expressive/found audio**, where SwiftF0 (−2.3 pts at 10 dB) and RMVPE (singing) take over. Keep penn's eval-stack slot. |
| 4 | **Who owns VAD.** Scout A assigns silero-vad to alignment-grade pause/breath-group segmentation; Scout C assigns it to hygiene speech-ratio columns. | One model, one pass, two consumers. **Run silero once** (Pass 2 below) and emit both the segment tier and the `qc_speech_ratio` scalar. Scout A's warning stands: silero's own docs concede weakness on **singing and very high-pitched voices** — the sung stratum uses SOFA silences + F0 voicing instead. |
| 5 | **Nonverbal-event detection ownership.** Scout B recommends PANNs/AST coarse tagging → jrgillick laugh spans; Scout C notes SenseVoice AED already emits laughter/crying/cough/sneeze/applause/bgm inline with ASR. | **Merge into one pass and pick SenseVoice as the free rider** (it is already running for the ASR-consistency gate if we choose it) — but keep **jrgillick (MIT)** as the license-clean laugh-span refiner on flagged clips, because SenseVoice gives clip-level tags, not spans, and its license is T2 not T0. PANNs (MIT) only where a fixed AudioSet vocabulary is required. |
| 6 | **Chatterbox paralinguistic tag inventory.** `SOTA-SURVEY` §A lists `[cough] [laugh] [sigh]`; Scout B's fetch of the docs found `[laugh] [chuckle] [cough]`. | **Version-dependent, unresolved.** Turbo and Multilingual V3 differ. Resolve empirically at generation time by probing the actual checkpoint and recording the inventory in the manifest (`gen_nv_tag_vocab`). |
| 7 | **Praat/Parselmouth's tier.** Scout A ranks it 5th among F0 trackers; Scout B makes it the *primary* voice-quality engine. | Different jobs, no conflict. Praat is a mediocre-but-bit-exact F0 tracker and the **gold standard for jitter/shimmer/HNR/CPPS/formants**. Both scouts agree on the license handling: **GPL-3.0, subprocess/harness only, never linked into the Rust core.** |
| 8 | **Clip-duration assumption behind throughput.** Scout A assumes ~5 s/clip (500k ≈ 700 h); Scout C assumes ~8 s/clip (500k ≈ 1,100 h). | §I standardizes on **5 s mean → 25k ≈ 35 h, 100k ≈ 139 h, 500k ≈ 694 h**, and states the scaling rule: multiply all wall-clock numbers by 1.6× if the realized mean is 8 s. Every §I estimate is tagged as an estimate. |
| 9 | **VocalSound CC-BY-SA → weights.** Scout B flags "SA-to-weights is the usual contested question." | Same unsettled question `SOTA-SURVEY` §I flags for NC-SA singing corpora. **Conservative position: treat weights fine-tuned on VocalSound as SA-derivatives** — fine for internal annotation, requires a decision before shipping such a checkpoint. |
| 10 | **NonverbalTTS license.** HF metadata says Apache-2.0; the README defers to source licenses (VoxCeleb + Expresso, the latter CC-BY-NC). | **Internal contradiction in the dataset itself.** Treat as **NC-tainted on the Expresso half**; use for eval and pipeline validation, never fold its audio into a permissive corpus without per-source filtering. |
| 11 | **Survey dates.** Scouts A/B dated 2026-08-11, Scout C 2026-08-12. | Immaterial; recorded as a window. No source moved between those dates. |

**Cross-cutting license flags carried forward unresolved:** TEN VAD's "Apache 2.0 with additional conditions"; the arXiv 2508.04814 pitch-accent code license; PhoneticXeus (already flagged in `SOTA-SURVEY` §B); creapy's TU Graz GitLab license; GFM-IAIF (no license stated); the flagship community Mel-Band-RoFormer vocal checkpoint (GitHub license API returns **null**).

---

## A. Alignment & segmentation at every granularity

### A.1 Candidates — forced alignment

| Tool | Version / date | License | Apple Silicon footprint | Maturity / accuracy | For-our-use one-liner |
|---|---|---|---|---|---|
| **MFA** ([docs](https://montreal-forced-aligner.readthedocs.io/), [2026 paper](https://arxiv.org/abs/2606.18466)) | 3.4.0, Jul 2026; companion paper Jun 16 2026 (McAuliffe, Gunter, Wagner, Sonderegger) | **MIT** | CPU-only, conda osx-arm64; process-heavy but parallelizes across 16 cores | The reference. Paper benchmarks MFA 3 on 4 datasets (EN/JA/KO): **mean boundary error <15 ms**, SOTA or near-SOTA vs classic + neural aligners; adaptation & cross-language phone remapping documented | Offline gold phone boundaries on clean TTS output — its home turf |
| **FALCON** ([arXiv 2606.25460](https://arxiv.org/abs/2606.25460), [repo](https://github.com/MLSpeech/FALCON)) | Jun 24 2026 (Rousso, Cohen, Keshet) | **MIT** (verified on repo) | PyTorch, BiLSTM-scale nets — CUDA "recommended" but small enough for MPS/CPU *(unverified on MPS)* | Brand-new. Differentiable soft-DP aligner: **TIMIT 83.9% @≤25 ms vs MFA 72.3%; Buckeye 69.9% vs 60.6%**; 3 checkpoints incl. zero-shot multilingual (NL/DE/HE) | The new accuracy ceiling for phone boundaries; low mileage — verify then adopt as re-scoring pass |
| **torchaudio CTC align + label priors** ([tutorial](https://docs.pytorch.org/audio/2.8/tutorials/ctc_forced_alignment_api_tutorial.html), [arXiv 2406.02560](https://arxiv.org/abs/2406.02560)) | torchaudio 2.8-2.10 | BSD-2 | Negligible on top of the acoustic model; MPS OK | Already chosen as oracle glue in `SOTA-SURVEY` §B. ⚠ torchaudio is in **maintenance phase** ([issue #3902](https://github.com/pytorch/audio/issues/3902)) — vendor the ~200-line alignment routine | **Default in-pipeline aligner**: reuse ZIPA emissions → phone boundaries + per-phone scores in the same pass |
| **BFA** (Bournemouth) ([arXiv 2509.23147](https://arxiv.org/html/2509.23147v1), [repo](https://github.com/tabahi/bournemouth-forced-aligner)) | v1.1.0, Feb 2026 | **GPL-3.0** (flag; + espeak-ng system dep) | pip; ~50 MB models; **~0.2 s per 10 s audio on one CPU core** (~240× faster than MFA) | Competitive recall at 20 ms on TIMIT (71.4% vs MFA 71.9%); onset+offset per phone + confidence + `is_estimated` flag; 80+ langs, no tonal | Fastest CPU aligner; GPL keeps it out of the Rust core — acceptable as a subprocess annotation pass |
| **ctc-forced-aligner** ([repo](https://github.com/MahmoudAshraf97/ctc-forced-aligner)) | v0.2, Jun 3 2026 (dropped torch C++ dep; 10× smaller, 50% faster) | Code BSD; **default model MMS-300m-1130 is CC-BY-NC-4.0** (flag) | Light; ONNX community port exists | Popular WhisperX-adjacent long-audio aligner, 1130 languages | Only with a swapped-in permissive CTC model (ZIPA); default weights taint |
| **WhisperX** ([repo](https://github.com/m-bain/whisperX)) | Active 2026, 22k+ stars | BSD-2 *(reported)* | faster-whisper backend; runs on Mac, best batched on GPU | Word-level (not phone-level) timestamps via wav2vec2 DTW; VAD-batched, 12× speedup; the de-facto "subtitles-grade" standard | Word timestamps + transcripts for *found/real* audio only; not our phone-boundary tool |
| **charsiu** ([repo](https://github.com/lingjzhu/charsiu)) | Frozen ~2022 | MIT | 10 ms frames, wav2vec2-scale | Pioneered text-independent phone alignment (EN/Mandarin); author (lingjzhu) moved on to ZIPA | Historical; superseded by ZIPA+label-priors and FALCON. Its **textless mode** remains the cite for aligning unintelligible/expressive audio |
| **SOFA** ([repo](https://github.com/qiuqiao/SOFA)) | Active; 437 commits, 2025-26 activity | **MIT** | PyTorch (+optional ONNX); light | **Singing-Oriented Forced Aligner** — the standard in the OpenVPI/DiffSinger voicebank ecosystem (feeds MakeDiffSinger); per-language community .ckpt models (Mandarin opencpop-extension default); evaluated by boundary-edit metrics, no published benchmark | **The sung-audio phone aligner.** Community-proven on exactly the data MFA fails on (melisma, sustained vowels) |
| NeMo Forced Aligner (NFA) | NeMo 2.x | Apache-2.0 *(training knowledge)* | CUDA-centric — spot box, not Mac | Solid CTC aligner; listed among 2026 neural aligners in the MFA paper | Only if already on a NeMo spot box |
| MAPS (Mason-Alberta Phonetic Segmenter) ([PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC11449383/)) | 2024 | open *(reported)* | small | DNN + interpolation for sub-frame boundaries | Niche; the interpolation idea is worth stealing if we need <10 ms precision |
| Ensemble boundary CIs ([arXiv 2506.01256](https://arxiv.org/pdf/2506.01256)) | Jun 2025 | paper | — | Gradient boundaries + confidence intervals from aligner ensembles | Method note: the manifest should carry per-boundary **confidence**, not point estimates |

### A.2 Candidates — VAD / pause / breath

| Tool | Version / date | License | Apple Silicon | Maturity | For-our-use one-liner |
|---|---|---|---|---|---|
| **silero-vad** ([repo](https://github.com/snakers4/silero-vad), [releases](https://github.com/snakers4/silero-vad/releases)) | v6 lineage; **v6.2.1 Feb 24 2026** (ONNX runtime now optional) | **MIT**, no strings | Trivial; ONNX; <1 ms per 30 ms chunk per CPU thread; [MLX port](https://huggingface.co/mlx-community/silero-vad-v6) | The default VAD everywhere; 6000-language claim; v6: −16% errors on noisy real-life data. **Known weakness: singing, very high-pitched voices** (own docs) | Default VAD for speech strata — emits both the segment tier (§A) and hygiene speech-ratio (§F). Do **not** trust on sung strata |
| **TEN VAD** ([repo](https://github.com/TEN-framework/ten-vad)) | 2025, active | **"Apache 2.0 with additional conditions"** — read LICENSE before shipping (flag) | ten_vad.framework macOS arm64; Python/ONNX; WASM | Frame-level (10/16 ms hop); beats WebRTC + silero on PR curves; lower compute than silero; hundreds-of-ms lower latency | Candidate frame-level VAD if the license conditions clear; else silero |
| **pyannote segmentation-3.0 / community-1** ([HF](https://huggingface.co/pyannote/segmentation-3.0), [blog](https://www.pyannote.ai/blog/community-1)) | community-1 in pyannote.audio 4.0 (2025) | **MIT** (gated HF download) | MPS OK; [MLX](https://huggingface.co/mlx-community/pyannote-segmentation-3.0-mlx) + [ONNX](https://huggingface.co/onnx-community/pyannote-segmentation-3.0) ports | VAD + overlapped-speech + diarization in one 10-s-window model | Use when clips may contain >1 voice (Dia2 dialogue stratum); overkill otherwise |
| **Respiro-en** ([repo](https://github.com/ydqmkkx/Respiro-en), [paper](https://arxiv.org/abs/2402.00288)) | Interspeech 2024 (Best Student Paper nominee); checkpoint `respiro-en.pt` in repo | **MIT** | Conformer blocks, small; CPU fine | **Frame-wise breath probability** detector, self-trained on LibriTTS-R; manually-annotated val/test sets included | **The breath annotator.** Breath events per clip → manifest column; its val set doubles as our QA set |
| **Brouhaha** ([repo](https://github.com/marianne-m/brouhaha-vad), [paper](https://arxiv.org/abs/2210.13248)) | 2023; stable | Code **MIT**; HF model **openRAIL** | Small pyannote model; ~50-100× RT/core *(estimate)* | Standard joint **VAD + SNR + C50** | Speech-aware SNR (not global RMS) + room acoustics — see §F; shares the VAD pass conceptually but is run for its SNR/C50 heads |
| breath-removal ([repo](https://github.com/lukaszliniewicz/breath-removal)) | 2024-25 | open *(reported)* | trivial | Respiro-based detect+attenuate utility | Utility precedent; we want detection only, **never removal** |
| FoxBreatheLabeler (OpenVPI) | referenced in OpenVPI tooling; direct repo URL 404'd this pass | unverified | — | Breath annotation for singing, consumes SOFA TextGrids *(rumor — could not fetch)* | Check under the openvpi org if the singing stratum needs breath tiers |

### A.3 Best practice for phone-level boundaries — the brief's direct question

**(a) Clean TTS output** *(tried-and-true skeleton, reported at the edges)*:

1. **Capture G2P at generation time** (misaki phonemes for Kokoro; goruut or an espeak-ng subprocess for others) — the transcript-to-phone mapping is then exact, not inferred.
2. **In-pipeline default: torchaudio CTC forced alignment with label priors over ZIPA-CR emissions.** Already in the stack as the intelligibility oracle; boundaries and per-phone log-likelihoods come from the same forward pass. Vendor the alignment code (torchaudio maintenance-phase risk).
3. **Gold pass (5-10% sample + anything the inverse net finds hard): MFA 3.4** — <15 ms mean boundary error, the citable standard; CPU-parallel over 16 cores.
4. **Watch/verify: FALCON.** If its TIMIT/Buckeye gains (+11 pts @≤25 ms over MFA) replicate locally on synthetic audio, promote it to the gold pass; MIT, small, three checkpoints.

**(b) Expressive / sung audio** *(reported)*:

- **Singing: SOFA** (MIT) is the community answer — built for exactly the failure modes (long sustained vowels, melisma, breaths between phrases) and battle-tested by the DiffSinger voicebank world. Pair with an F0 tier (SwiftF0/RMVPE, §B) and treat MFA output on singing as untrusted. **No peer-reviewed singing-alignment benchmark exists** — plan a 50-clip hand-checked QA sample.
- **Expressive speech (laughs, cries, nonverbal insertions):** align the *lexical* stretches normally and mark nonverbal regions via VAD/breath/laughter detectors (§D) rather than forcing phones onto them. charsiu's textless mode / ZIPA phone posteriors give a "what phone-like material is here" lattice over nonverbal regions — useful raw material for GARGANTA's laugh decomposition (laughter calls often decode as /h/+vowel sequences).
- **Syllabification:** derive from phone alignments by maximum-onset + sonority over the aligned phone string (deterministic code, no model — §G); syllable boundaries inherit phone-boundary timestamps.
- **Pause/breath-group segmentation:** silence tokens from alignment ∪ VAD gaps ≥ 150-250 ms, with Respiro-en breath events splitting pauses into breath-pauses vs silent-pauses. This is the standard recipe; no dedicated 2026 tool superseded it.

**Throughput frame** (25k-500k clips at the standardized 5 s/clip ⇒ 35-694 h): CTC-align adds ~nothing to the oracle pass; MFA on 16 cores handles 35 h in ~1-2 h *(reported)*; BFA does the same in minutes if GPL-as-subprocess is acceptable for the annotation tier.

---

## B. Prosody extraction & event annotation

### B.1 F0 trackers — ranked

Ranking source: **[lars76/pitch-benchmark](https://github.com/lars76/pitch-benchmark)** (MIT) — 13 algorithms × 8 datasets (speech PTDB/PTDBNoisy/SpeechSynth + singing MIR1K/Vocadito + instruments), the closest thing to a 2026 league table. *(reported — rerun locally before trusting per-stratum; note the benchmark author is also the SwiftF0 author)*

| Rank | Tracker | Version / date | License | Apple Silicon | Benchmark avg | For-our-use one-liner |
|---|---|---|---|---|---|---|
| 1 | **SwiftF0** ([repo](https://github.com/lars76/swift-f0), [paper](https://arxiv.org/pdf/2508.18440)) | Aug 2025; pip `swift-f0` | **MIT** (verified on repo — supersedes `SOTA-SURVEY` §H's "unknown") | **95k params, ONNX; 132 ms per 5 s audio on CPU** (~38× RT/core) | **90.2%**, 42-90× faster than CREPE; only −2.3 pts at 10 dB SNR; range 46.9-2093.75 Hz, 16 kHz / hop 256 (16 ms) | **Primary F0 tracker** — speed, noise-robustness and singing range in one tiny model |
| 2 | RMVPE ([repo](https://github.com/Dream-High/RMVPE)) | 2023; ubiquitous in SVC | MIT *(training knowledge)* | Heavier (E2E mel U-net); MPS OK | 87.2% | Singing specialist — second opinion on the sung stratum |
| 3 | CREPE / torchcrepe | 2018; frozen | MIT | Slow full model | 85.3% | Legacy cross-check only |
| 4 | **penn** ([repo](https://github.com/interactiveaudiolab/penn)) | FCNF0++; maintained | MIT | pip; fast | 84.8% clean — **but −29 pts at 10 dB SNR** | Already in `SOTA-SURVEY` §H's eval stack; keep it there and as the agreement partner **on clean strata**; demote on noisy/expressive (conflict #3) |
| 5 | **Praat via parselmouth** ([repo](https://github.com/YannickJadoul/Parselmouth)) | 0.4.7 (Nov 2025); 0.5.0.dev (Jun 2026) | **GPL-3.0** | CPU, bit-exact | Classic; still the phonetics reference on clean speech | Gold-standard QA tier, and the primary engine for jitter/shimmer/HNR/CPPS anyway (§C). GPL ⇒ subprocess/eval harness only, never linked into the Rust core |
| 6 | pyWORLD (harvest/dio) | stable | MIT/BSD lineage | trivial | vocoder-grade | Worth keeping for one reason: **D4C band-aperiodicity** per frame ≈ voicing-fraction lattice — directly useful for laugh/breath forensics (§C, §D) |
| — | pYIN / YAAPT / SWIPE / RAPT | classic | various | trivial | mid-pack | Superseded for our purposes |

**Recommendation:** SwiftF0 everywhere + penn (clean speech) or RMVPE (singing) as the agreement partner; frames where trackers disagree >50 cents or on voicing decisions get flagged in the manifest (`pros_f0_conf`, `pros_f0_agree_cents`). Praat tier on the QA sample. *(reported)*

### B.2 Energy / duration lattices

No tool gap: per-frame RMS + spectral tilt (librosa/torch, 10-16 ms hop) and per-phone/syllable durations from §A alignments. **Norm the lattice, don't ship raw values**: z-score log-duration per phone-class per speaker, z-score log-energy per speaker — this is what every prosody paper (and the wavelet toolkit) does internally, and it is the shape SPEAK's conditioning wants. *(tried-and-true)*

### B.3 Prosodic event detection — what replaced AuToBI

Nothing replaced it one-for-one; [AuToBI](https://github.com/AndrewRosenberg/AuToBI) (Java, 2010s) is unmaintained legacy. The 2026 field runs in three lanes:

| Lane | Tool / paper | Date | License | Output | State |
|---|---|---|---|---|---|
| SSL-probe event detectors | **Prosodic-prominence-ASR** ([arXiv 2508.04814](https://arxiv.org/html/2508.04814v1), [code](https://github.com/sasudavid/prosodic_prominence_asr)) — ITU Copenhagen + Apple | Aug 2025 | code released, **license unverified** (flag) | Frame-level pitch-accent detection, **F1 0.90 @100 ms** (semi-supervised, wav2vec2-base) — beat Wav2ToBI, closing 41% of the gap | Current published SOTA for pitch accents |
| | Wav2ToBI ([Interspeech 2023](https://www.isca-archive.org/interspeech_2023/zhai23_interspeech.html)) | 2023 | code+weights available | Pitch accents F1 0.82, phrase boundaries F1 0.86 (BURNC) | The lineage baseline |
| Continuous prominence (CWT) | **wavelet_prosody_toolkit** ([repo](https://github.com/asuni/wavelet_prosody_toolkit)) | dormant since ~2020, **still used in 2026 papers** | **MIT** (verified) | TSV: `file start end unit prominence_strength boundary_strength` — per-syllable/word continuous scores from CWT over combined F0+energy+duration | De-facto continuous-prominence standard; batch tool included |
| Word-level emphasis | **emphases** ([repo](https://github.com/interactiveaudiolab/emphases), ICASSP 2024) | maintained; pip | **MIT** (verified) | Float emphasis score per word (+ binary), trained on crowdsourced annotations | Best turnkey emphasis scorer; its default P2FA aligner is swappable — feed it our §A TextGrids |

**2026 practice — and what SPEAK should adopt:** categorical ToBI at scale remains unreliable (cross-annotator κ was always the bottleneck); the field annotates **continuous per-syllable prominence and boundary-strength scores**, discretizing only when needed. That is also exactly the right conditioning-signal shape for a control-program space. Recommendation: wavelet-toolkit prominence/boundary columns (deterministic, MIT, CPU-trivial) as the base lattice on all clips; `emphases` word scores as a second opinion on English; the 2508.04814 detector as a categorical pitch-accent tier **if its license clears**. *(reported)*

### B.4 Speech-rate estimation

- **With text (our generated corpora): count syllables from the §A alignment** — speaking rate = syl/s total, articulation rate = syl/s excluding pauses ≥150 ms. Deterministic, free, best practice. *(tried-and-true)*
- Without reliable text: de Jong & Wempe syllable-nuclei intensity-peak method via parselmouth (GPL) — still the standard 17 years on; no 2026 successor tool emerged. *(tried-and-true)*
- Cheap runaway-repetition detector: dataspeech's speaking-rate columns (§F) catch TTS repetition loops without any extra model. *(reported)*

### B.5 Output schema norm — per-syllable row

```
clip_id, syl_idx, t0, t1, nucleus_phone, phones, lexical_stress(0/1/2),
f0_mean_st, f0_slope_st_s, f0_peak_align(0-1), energy_z, dur_z,
prominence_cwt, boundary_cwt, emphasis_word, pause_after_ms,
breath_after(bool), f0_confidence
```

Prior art for the shape: wavelet-toolkit TSV + [Helsinki Prosody Corpus](https://github.com/Helsinki-NLP/prosody) discrete 0-2 prominence labels. **Continuous columns primary, discrete derived.** This table is a *sidecar* (Parquet), referenced from the JSONL manifest — see §I. *(reported)*

---

## C. Voice-quality forensics

Two consumers: (1) per-frame/per-segment VQ lattices to condition SPEAK's program space; (2) empirical glottal-pulse targets for GARGANTA's glottal port.

### C.1 Candidates — scalar VQ measures (jitter/shimmer/HNR/CPP) & creak/breathiness

| Tool | Version / date | License | Apple Silicon footprint | Maturity | For-our-use one-liner |
|---|---|---|---|---|---|
| **Praat via Parselmouth** ([repo](https://github.com/YannickJadoul/Parselmouth)) | 0.4.7 (Nov 2025), 0.5.0.dev Jun 2026 | **GPL-3.0** (inherits Praat) | Pure CPU, faster than RT per core; trivial | The gold standard; actively maintained | Primary engine for jitter, shimmer, HNR, formants, **CPPS** (`PowerCepstrogram` → `Get CPPS...`); GPL fine for an annotation harness that never links into the Rust release |
| **openSMILE / eGeMAPSv02** ([repo](https://github.com/audeering/opensmile-python), [about](https://audeering.github.io/opensmile/about.html)) | 3.x, maintained by audEERING | **Source-available, research/education only; commercial product needs a paid license** | Fast C++; pip wheel; CPU trivial | The affective-computing standard (88-dim eGeMAPS incl. jitter, shimmer, HNR, slopes) | One-call 88-dim VQ/prosody vector per clip — but **not open source**; internal annotation only, do not redistribute an openSMILE-derived tool. Flagged in the roll-up |
| **DisVoice** ([repo](https://github.com/jcvasquezc/DisVoice)) | active-ish, 170 commits | **MIT (verified)** | Python/CPU; light | Respected in clinical-speech work | MIT alternative for phonation (jitter/shimmer/APQ/PPQ) **plus glottal features (GCI + IAIF-lineage: NAQ, QOQ, HRF)** in one pip package |
| **Surfboard** (Novoic) ([repo](https://github.com/novoic/surfboard)) | **archived Jan 29 2025, read-only** | GPL-3.0 (dual commercial) | CPU trivial | Was the tidy VQ kit | Skip — archived; DisVoice/Parselmouth cover it |
| **opensauce-python** ([repo](https://github.com/voicesauce/opensauce-python)) | dormant | **Apache-2.0 (verified)** | CPU; wraps Praat/Snack/REAPER | VoiceSauce port (H1\*-H2\*, H1\*-A3\*, CPP, SHR) | The permissive route to **corrected spectral-tilt measures (H1\*-H2\*)** — the phonation-type axis GARGANTA's glottal port needs; dusty but functional |
| VoiceSauce (MATLAB) ([site](http://www.phonetics.ucla.edu/voicesauce/)) | UCLA, stable | Free for research | MATLAB | The phonetics-literature standard for H1\*-H2\* | Reference implementation to validate opensauce against *(reported)* |
| **creapy** (TU Graz) ([post](https://www.spsc.tugraz.at/posts/2023/07/01/creapy-a-Python-based-tool-for-the-detection-of-creak-in-conversational-speech.html), [repo](https://gitlab.tugraz.at/speech/creapy)) | ICPhS 2023 | License not stated on post (repo on TU Graz GitLab) — **flag** | CPU; TextGrid output | Semi-automatic aid; recall 0.79 (M) / 0.60 (F) | Recall-tuned creak *interval* detector with Praat TextGrid export — good first pass; verify license before bundling |
| **DeepFry** ([repo](https://github.com/bronichern/DeepFry), [paper](https://arxiv.org/pdf/2203.17019)) | Interspeech 2022; 6 commits | **MIT (verified)** | Small CNN on raw waveform; CPU/MPS trivial | Best published creak F1 on American English | **Primary frame-level creak detector**; jointly predicts creak + voicing + pitch at 5 ms frames |
| Social-creak classifier (Alku group) ([arXiv 2410.17028](https://arxiv.org/abs/2410.17028)) | J. Voice, accepted 2024-25 | No code found | — | 71.1% acc, 90 Finnish female speakers, mel-spec + AdaBoost | Evidence that creak-*amount* classification is only modestly accurate even supervised; calibrate expectations. Paper-only *(reported)* |
| **covarep** ([repo](https://github.com/covarep/covarep)) | dormant (MATLAB/Octave, 498 commits) | GPL-3.0 *(training knowledge — LICENSE.txt not displayed in fetch)* | MATLAB/Octave | The canonical VQ algorithm museum | Reference implementations: **SEDREAMS GCI, IAIF, QCP, Kane-Drugman creaky-voice detector**; port *from*, don't depend *on* |
| **vTAD, training-free acoustic params** ([arXiv 2603.05091](https://arxiv.org/pdf/2603.05091)) | **Mar 2026** (Chiu, Xiao, Kong, Lee) | Code not confirmed | — | New | 2026 signal: interpretable compact acoustic parameter sets are competitive for *timbre-attribute* comparison — philosophically aligned with control-program conditioning; watch for code *(reported)* |

**Best-practice notes (2026):**

- **CPPS is the consensus single most robust breathiness/dysphonia scalar on running speech**; jitter/shimmer are only meaningful on clean, correctly-F0-tracked voiced stretches. At corpus scale, compute per *voiced segment* with an F0 gate and treat clip-level averages as noisy. *(tried-and-true practice; lit.)*
- Cheap scalable breathiness proxy: **WORLD/D4C band aperiodicity** (BSD-3, fast C++) per frame — pairs well with CPPS. *(reported)*

### C.2 Glottal source — GCI, inverse filtering, and "usable glottal flow at scale?"

| Tool / resource | Date | License | Footprint | For-our-use one-liner |
|---|---|---|---|---|
| **REAPER** (Google, D. Talkin) ([repo](https://github.com/google/REAPER)) | stable | **Apache-2.0** *(training knowledge, long-standing)* | C++, very fast, CPU | Epoch (GCI) + F0 at corpus scale; the workhorse for pitch-synchronous windows feeding everything below |
| SEDREAMS / DYPSA / MSF GCI | covarep | GPL-3.0 | MATLAB | Accuracy references for GCI; port SEDREAMS if REAPER epochs prove insufficient *(tried-and-true lineage)* |
| **IAIF** + **GFM-IAIF** ([repo](https://github.com/operrotin/GFM-IAIF)) | 2019 | **No license stated — flag** | MATLAB (trivial to port; ~50 lines) | The default inverse filter; the GFM variant improves glottal-formant handling |
| **QCP** (quasi-closed-phase WLP) ([IEEE TASLP 2014](https://ieeexplore.ieee.org/document/6680604/)) | 2014; still SOTA classical | Algorithm (covarep impl GPL) | Light DSP | **Best classical GIF on high-pitched voices** — the one to implement in-house (weighted LP with attenuated-main-excitation weights; needs GCIs) |
| **OPENGLOT** ([paper](https://www.sciencedirect.com/science/article/abs/pii/S0167639318303509)) | 2019 | Free (Aalto) | Synthetic + physical test audio with **ground-truth glottal flow** | **The validation set**: calibrate whatever GIF stack we build before trusting it on TTS output *(reported)* |
| Neural GIF ([ResearchGate](https://www.researchgate.net/publication/363646279_Glottal_inverse_filtering_based_on_articulatory_synthesis_and_deep_learning)) | 2022-23 | **No open tooling found** | — | **Negative finding:** as of Aug 2026 no maintained open-source *neural* glottal inverse filter surfaced in any search. Classical IAIF/QCP remain the scale tools *(reported-negative)* |
| **GOLF** ([repo](https://github.com/yoyololicon/golf), [TISMIR](https://transactions.ismir.net/articles/10.5334/tismir.210)) | Interspeech 2024 line; active, 279 commits | **MIT (verified)** | PyTorch + torchlpc; MPS-friendly; small | **The differentiable route:** glottal-flow wavetable + differentiable LPC vocal tract, trained on VCTK. Fit it to audio by gradient → glottal wavetable + filter estimates as *analysis-by-synthesis* GIF. Directly reusable machinery for GARGANTA's glottal port |
| Pathological-voice glottal features ([arXiv 2309.14080](https://arxiv.org/pdf/2309.14080)) | 2023 | — | — | Worked example of QCP-derived glottal features at corpus scale *(reported)* |

**Verdict — glottal flow at scale: yes, with discipline.** REAPER epochs → QCP (IAIF fallback) pitch-synchronously → parametrize *per cycle* as **NAQ, QOQ, H1\*-H2\*, HRF, Rd-fit** rather than storing raw waveforms; validate on OPENGLOT first; expect degradation on creaky frames (gate by DeepFry) and above F0 ≈ 300 Hz (QCP tolerates high pitch best). On clean TTS output — our corpora — conditions are *favorable*: no noise, no reverb, known text. Throughput: all-DSP, >10× RT/core ⇒ 25k-500k clips is hours on the 16-core Mac. *Confidence: reported (stack composition is ours; components tried-and-true).*

### C.3 Empirical glottal targets for GARGANTA — the design table

Stable phonetics/voice-science consensus ranges; **(lit., training knowledge)** unless linked. Use as design targets **and** as sanity bounds on our own QCP measurements.

| Phonation type | OQ (open quotient) | NAQ | LF **Rd** | H1\*-H2\* | Pulse-shape notes |
|---|---|---|---|---|---|
| **Modal** | 0.45-0.70 | 0.10-0.20 | ~0.8-1.7 | +1 to +8 dB | Right-skewed pulse (speed quotient ~1.5-3); abrupt closure → rich spectrum; closed phase 30-55% of cycle |
| **Breathy** | 0.70-0.95 (incomplete closure → DC leak) | 0.20-0.35 | >2 (2-2.7+) | +8 to +16 dB | Near-sinusoidal pulse; steep tilt; strong aspiration noise (HNR 5-15 dB); H1 dominant |
| **Pressed / tense** | 0.25-0.45 | 0.05-0.10 | 0.3-0.8 | −5 to +2 dB | Short open phase, sharp closure; shallow tilt, energy at 2-5 kHz; the "edge" |
| **Creak / fry** | 0.1-0.35 (long closed phase) | very low | <0.3 (ill-defined) | negative | F0 ~20-90 Hz; period-doubling/subharmonics, aperiodic pulse spacing, individually resolved damped pulses; jitter huge |
| **Falsetto / M2** | 0.5-0.95 | high | high | large positive | Near-sinusoidal; weak harmonics; relevant for laughter squeals |
| **Singing (M1, effort ↑)** | 0.30-0.80, falls with vocal effort ([Henrich et al. 2005, EGG/DECOM](https://pubmed.ncbi.nlm.nih.gov/15957767/)) | — | — | — | OQ is *the* effort/register axis in singing; GARGANTA's glottal port should expose it directly |

Key sources: Klatt & Klatt 1990; Hanson 1997; Holmberg et al. 1988; Alku et al. 2002 (NAQ); Fant 1995 (Rd); Henrich 2005; Blomgren et al. 1998. The **Rd single knob** (0.3 = pressed → 2.7+ = breathy, a monotone morph of the whole LF shape) is the best one-float glottal control program ever published — **adopt it as GARGANTA's primary glottal-port parameter**, with OQ and a noise-mix as second and third. *(lit., tried-and-true science; boundaries approximate)*

### C.4 Formant tracking that survives high F0 / singing

- **Negative finding first:** no open 2025-26 breakthrough tracker for high-F0/sung formants surfaced. The physics is the limit — at F0 ≳ 350 Hz harmonics undersample the envelope and point-formants become ill-defined. *(reported-negative)*
- Practical 2026 stack: (1) **Praat/Parselmouth** with per-voice ceiling tuning for speech-range F0 *(tried-and-true)*; (2) **DeepFormants + LP-refinement hybrid** ([arXiv 2308.09051](https://arxiv.org/abs/2308.09051), Aalto 2023) as the data-driven upgrade *(reported)*; (3) for singing/high F0, store the **WORLD/CheapTrick spectral envelope (or QCP's vocal-tract filter)** + peak candidates **with uncertainty**, instead of committing to F1/F2 points *(reported)*; (4) sanity-check formant lattices against **SPARC** articulatory trajectories where both exist (`SOTA-SURVEY` §E).

### C.5 For our use (C)

1. **Parselmouth (GPL harness) + DisVoice (MIT)** for jitter/shimmer/HNR/CPPS/phonation lattices; openSMILE eGeMAPS only if its research license is acceptable for internal tooling. *(tried-and-true)*
2. **DeepFry (MIT)** frame-level creak + creapy for interval proposals. *(reported)*
3. **REAPER → QCP/IAIF → NAQ/QOQ/H1\*-H2\*/Rd-fit** pitch-synchronous glottal lattice, validated on OPENGLOT, creak-gated. *(reported)*
4. **GOLF (MIT)** as differentiable analysis-by-synthesis glottal estimator and donor code for GARGANTA's glottal port. *(reported)*
5. Manifest columns contributed: see the `vq_*` block in §I.

---

## D. Nonverbal vocalizations

### D.1 Detection / segmentation candidates

| Tool / model | Date | License | Apple Silicon footprint | Maturity | For-our-use one-liner |
|---|---|---|---|---|---|
| **jrgillick/laughter-detection** ([repo](https://github.com/jrgillick/laughter-detection)) | 2021, updated PyTorch model | **MIT (verified)** | Small ResNet; CPU fine | Community default 5 years running; known noise sensitivity | Still the baseline laugh detector; fine on clean TTS output. **The license-clean span refiner** (conflict #5) |
| **LaughterSegmentation** (Omine et al., Interspeech 2024) ([repo](https://github.com/omine-me/LaughterSegmentation)) | 2024 | **Code MIT; model weights "research use only" — flag** | 1.26 GB safetensors; GPU-preferred, MPS workable | Robust boundaries via synthesized diverse training data | Best open laughter *boundary* quality; the weights license blocks shipped pipelines — offline gold-labeling only, or retrain |
| **MultiLinguahah** ([arXiv 2605.06309](https://arxiv.org/html/2605.06309)) | **May 2026** | Paper; code TBD | BYOL-A + Isolation Forest — light | New | First credible *multilingual, unsupervised* laughter segmentation; watch for code *(reported)* |
| **SenseVoiceSmall** ([HF](https://huggingface.co/FunAudioLLM/SenseVoiceSmall), [repo](https://github.com/FunAudioLLM/SenseVoice)) | 2024; card updated 2026-06-20; 26k dl/mo | **FunASR Model Open Source License** — custom/non-OSI **but commercial + redistribution OK** with attribution and a no-disparagement clause (conflict #2) | ~234M, Whisper-Small-scale; non-autoregressive, very fast; CPU/MPS | Huge download base, FunASR-maintained through 2026 | One-pass **ASR + LID + SER + audio-event detection** (`laughter, applause, crying, coughing, sneezing, bgm`), 50+ langs — the cheapest joint transcript + NVV + emotion tagger |
| **Whisper-AT** ([used in TIC-TALK](https://arxiv.org/pdf/2603.21803)) | 2023; used Mar 2026 | Code MIT *(reported)* | Whisper-large + AT head | Proven in 2026 pipelines | AudioSet-vocabulary tagging riding on Whisper; high-recall at 0.8 s stride is the published pattern |
| **PANNs** ([repo](https://github.com/qiuqiangkong/audioset_tagging_cnn)) / **AST** / **BEATs** | 2019-23 | **MIT** / MIT / MS-open *(tried-and-true)* | CNN14 80-90M; CPU/MPS trivial | Boring-reliable | Coarse tagging layer for *all* NVV classes via the AudioSet ontology; pairs with CLAP zero-shot (§F) |
| **Dasheng** (Xiaomi) ([repo](https://github.com/XiaoMi/dasheng)) | 2024-active | **Apache-2.0** | Small variants available | Active AudioSet tagger | Modern permissive alternative to PANNs where a fixed vocabulary is wanted |
| CED ([repo](https://github.com/RicherMans/CED)) | 2023 | **GPL-3.0 ⚠** | — | Strong AudioSet tagger | **Skip** — GPL in anything that ships |
| **NVV-in-ASR / "Beyond Words"** ([arXiv 2607.01563](https://arxiv.org/abs/2607.01563)) | **Jul 2026** | Paper | — | New | Data-centric recipes (generic-NV-token curriculum, laugh→cry transfer, VC augmentation) for NVV-emitting ASR — where transcription-side annotation is heading *(reported)* |
| **SMILE-Next** ([arXiv 2605.28084](https://arxiv.org/html/2605.28084)) | **May 2026** | Benchmark | — | New | Laughter detect / classify / *reason* moved into AV-LLMs; an eval idea, not a pipeline tool *(reported)* |

### D.2 Datasets + licenses — the load-bearing table

| Dataset | Contents | Size | License | For-our-use |
|---|---|---|---|---|
| **VocalSound** ([repo](https://github.com/YuanGongND/vocalsound)) | laughter, sigh, cough, throat-clear, sneeze, sniff + speaker metadata | 21,024 clips / 3,365 speakers | **CC BY-SA 4.0 (verified)** | **Primary NVV classifier-training set** — permissive, balanced, exactly our six most-needed classes. SA-to-weights propagation is contested (conflict #9) |
| **NonverbalTTS** ([HF](https://huggingface.co/datasets/deepvk/NonverbalTTS), [paper](https://arxiv.org/abs/2507.13155), SSW 2025) | 10 NV types (breath, laugh, sigh, sneeze, cough, throat-clear, groan, grunt, snore, inhale) + 8 emotions, **text-aligned** | 17 h | **HF metadata says Apache-2.0; README defers to source licenses (VoxCeleb + Expresso). Expresso = CC-BY-NC → NC taint on that half. License contradiction — flag** (conflict #10) | Open blueprint + benchmark for tag-in-transcript training (§H). Eval + pipeline validation only; don't fold audio into a permissive corpus without per-source filtering |
| **Deeply Nonverbal Vocalization** ([OpenSLR 99](https://www.openslr.org/99/)) | 16 classes incl. moan, scream, pant, yawn | 0.6 h sample (57 h full, proprietary) | **CC BY-NC-ND 4.0 (verified)** | Eval-only; ND blocks derivatives |
| **PodcastFillers** ([site](https://podcastfillers.github.io/)) | 35k uh/um + 50k breath/laughter/music/noise events, aligned | 145 h / 199 episodes | **Annotations NC (verified); audio CC-from-SoundCloud, mixed** | The filler/breath event reference; research-only. Its CNN baseline is the standard filler detector — **re-implement the recipe, don't ship the data** |
| **VIVAE** (graded affect bursts: achievement, pain, fear, surprise, pleasure, anger) | 1,085 clips / 11 speakers | small | NC-family on Zenodo *(rumor — verify the record before use)* | Affect-burst *taxonomy* anchor + intensity-graded probes |
| **Hume-VB / ExVo** ([ExVo](https://arxiv.org/pdf/2206.11968)) | 59k vocal bursts, 10 emotions, 4 cultures | 36+ h | Research EULA *(reported)* | Largest burst corpus; the EULA keeps it out of shipped models |
| **AMI meeting corpus** | meetings with laughter annotations | 100 h | **CC BY 4.0** *(tried-and-true)* | The permissive *conversational* laughter source (ELaTE used it); real laughs in context |
| Switchboard / Fisher / ICSI | telephone/meeting laughter | large | **LDC, paid, research** *(tried-and-true)* | Where the classic laugh detectors were trained; a budget item |
| **AudioSet** (laughter ontology: giggle, belly laugh, snicker, chuckle) | in-the-wild | 2M clips | Labels CC BY 4.0; audio = YouTube, not redistributed *(tried-and-true)* | Ontology + eval labels; not a corpus you own |
| UTokyo in-the-wild laughter corpus ([paper](https://arxiv.org/abs/2305.12442)) | Japanese laughter, segmented | 3.5 h | Research *(reported)* | Proof-scale for dedicated laughter corpora (§H) |
| **COUGHVID** | crowdsourced coughs | 25k+ | CC BY 4.0 *(reported)* | Permissive cough stratum |
| **NVBench / NVV-SuperBench** ([arXiv 2604.16211](https://arxiv.org/html/2604.16211v2)) | 4,500 bilingual (EN/ZH) NVV test instances, **45-type taxonomy** | eval set | License TBD | **Adopt its 45-type taxonomy as our tag vocabulary's superset**; first standardized NVV *generation* eval (Apr 2026) |
| Nonspeech7k / ESC-50 / ReCANVo | misc NVV | — | Nonspeech7k unverified; ESC-50 **CC BY-NC** *(tried-and-true)*; ReCANVo unverified | Secondary; check per-use |

### D.3 How expressive TTS systems tag nonverbals — verified inventories

| System | Syntax | Verified inventory | Source |
|---|---|---|---|
| **Orpheus** | angle tags | `<laugh> <chuckle> <sigh> <cough> <sniffle> <groan> <yawn> <gasp>` | [repo](https://github.com/canopyai/Orpheus-TTS) |
| **Dia / Dia2** | parentheticals | `(laughs) (clears throat) (sighs) (gasps) (coughs) (singing) (sings) (mumbles) (beep) (groans) (sniffs) (claps) (screams) (inhales) (exhales) (applause) (burps) (humming) (sneezes) (chuckle) (whistles)` — 21 tags, EN-only; README warns of artifacts on overuse | [repo](https://github.com/nari-labs/dia) (verified) |
| **Chatterbox (Turbo / Multilingual v3)** | square brackets | docs: `[laugh] [chuckle] [cough]`; `SOTA-SURVEY` §A lists `[cough] [laugh] [sigh]` — **version-dependent, unresolved (conflict #6)** | [docs](https://yocxy2-chatterboxyocxy.mintlify.app/guides/paralinguistic-tags) |
| **CosyVoice 2/3** | square-bracket events + **scoped spans** | `[laughter] [breath]` inline; `<laughter>…</laughter>`, `<strong>…</strong>` spans — the only open convention that can mark **speech-laughs** | [paper](https://arxiv.org/html/2412.10117v1) |
| **ChatTTS** | square-bracket control tokens | released: `[laugh] [uv_break] [lbreak]`; documented-but-unreleased graded `[laugh_0-2] [break_0-7] [oral_0-9]` | [repo](https://github.com/2noise/ChatTTS) (code AGPL-3; **weights CC BY-NC — taint**) |
| **Bark** | square brackets | `[laughter] [laughs] [sighs] [gasps] [clears throat]`, `♪` | *(tried-and-true, training knowledge)* |
| **ElevenLabs v3** (closed) | square brackets | `[laughs] [whispers] [sighs] [sarcastic] [crying] [excited]`… — the de-facto convention-setter; tops NVBench subjective | *(reported)* |
| **Fish / OpenAudio S1** | parentheticals + emotion markers | `(laughing) (chuckling)` etc. — NC, avoid anyway | *(reported)* |

**Convention synthesis:** three syntaxes coexist — square-bracket event tokens (Bark → ChatTTS → CosyVoice → Chatterbox → ElevenLabs), parenthetical stage directions (Dia, Fish), angle tags (Orpheus). **Square-bracket `[laugh]`-style is the modal open convention**; scoped spans (CosyVoice) are the only expressive superset (co-articulated laughter). NVBench (Apr 2026) documents per-system coverage from 1 type (ChatTTS) to 13 (Dia, ElevenLabs) against its 45-type taxonomy, and a **coverage↔correctness trade-off** in tag-based systems. *(reported)*

### D.4 For our use (D)

1. **Tag vocabulary:** square-bracket tokens mapped onto the **NVBench 45-type taxonomy** (store `nv_tag` + `nv_taxon`); add CosyVoice-style span variants for speech-laughs. *(reported)*
2. **Detection stack (ordered, deduplicated across scouts — conflict #5):** SenseVoice AED riding the ASR pass (clip-level event tags, T2 license) → **PANNs/Dasheng** coarse AudioSet tagging where a fixed vocabulary is needed (MIT/Apache, CPU-trivial) → **jrgillick (MIT)** laugh *spans* on flagged clips → **VocalSound-finetuned AST head** for the six core classes (CC-BY-SA data; SA-to-weights flagged) → breath/filler pass per the PodcastFillers recipe (re-implemented; the annotations themselves are NC). **LaughterSegmentation only as an offline gold-labeler.** *(reported)*
3. **Throughput:** everything at or below Whisper-small scale; the 16-core M-series does 25k-500k clips overnight per pass; no spot box needed except LaughterSegmentation. *(reported)*
4. **License flags:** SenseVoice custom model license (T2); LaughterSegmentation weights research-only; NonverbalTTS Apache-vs-NC contradiction; Deeply ND; PodcastFillers NC; Hume-VB EULA; CED GPL. **Clean core:** VocalSound (CC BY-SA), AMI (CC BY), jrgillick (MIT), DeepFry (MIT), COUGHVID (CC BY), PANNs (MIT), Dasheng (Apache).

---

## E. Emotion / affect annotation at scale

**Landscape.** No "Whisper moment" happened for SER. The field's 2024-25 consolidation (emotion2vec+, EmoBox, MSP-Podcast challenges) still defines the open stack in Aug 2026; the 2025-26 additions are (a) trait-profile suites (Vox-Profile), (b) synthetic-trained fine-grained emotion (LAION EmoNet-Voice / Empathic-Insight), (c) SEA-multilingual dimensional+categorical (MERaLiON-SER), and (d) audio-LLMs as zero-shot annotators — which benchmarks now show are *worse* than specialized models for hard labels but better-calibrated on ambiguity. **The single most consequential 2026 finding for SPEAK: SER models do not generalize to TTS-synthesized speech** ([arXiv 2603.16483](https://arxiv.org/abs/2603.16483), Mar 2026).

### E.1 Candidates

| Model / tool | Version / date | License (verified) | Apple Silicon footprint | Maturity / notes | For-our-use one-liner |
|---|---|---|---|---|---|
| **emotion2vec+ (seed/base/large)** ([HF](https://huggingface.co/emotion2vec/emotion2vec_plus_large), [repo](https://github.com/ddlBoJack/emotion2vec)) | +large Jun 2024; no 2025-26 weight refresh | Weights: FunASR "[Model Open Source License](https://github.com/modelscope/FunASR/blob/main/MODEL_LICENSE)" — **commercial + redistribution OK**, attribution + no-disparagement. ⚠ **GitHub code repo has no LICENSE file** (verified via API) | large ~300M, base ~90M; PyTorch/FunASR, CPU/MPS fine; est. 30-60× RT (large), 100×+ (base) per stream *(estimate)* | The de-facto open SER foundation model; 40k+ h pseudo-labeled finetune; 9 coarse categories; frame- or utterance-level embeddings | Default categorical annotator + emotion-embedding extractor; **use the embedding, not just the argmax** |
| **SenseVoiceSmall** ([HF](https://huggingface.co/FunAudioLLM/SenseVoiceSmall)) | card updated 2026-06-20 | Same FunASR model license (commercial OK, attribution) — see conflict #2 | ~234M; very fast (non-autoregressive); CPU-viable | ASR + LID + **SER** + **AED** in one pass | Cheapest way to get emotion + nonverbal-event tags in a single sweep; the AED output doubles as §D tagging |
| **MERaLiON-SER-v1** (A*STAR) ([HF](https://huggingface.co/MERaLiON/MERaLiON-SER-v1), [arXiv 2511.04914](https://arxiv.org/abs/2511.04914)) | Nov 2025, card upd. Feb 2026; 26.6k dl/mo | **MERaLiON Public License (custom)** — read before depending | 309M (frozen Whisper-medium encoder + LoRA); MPS fine *(est. 30-60× RT)* | Joint **categorical (7) + dimensional (V/A/D, 0-1)**; EN + SEA languages; claims competitive out-of-domain UAR | Freshest joint cat+dim model; license is the blocker — internal annotation defensible, derived-model redistribution needs the license read |
| **Vox-Profile suite** (USC) ([paper](https://arxiv.org/pdf/2505.14648), [repo](https://github.com/tiantiaf0627/vox-profile-release), [dim HF](https://huggingface.co/tiantiaf/whisper-large-v3-msp-podcast-emotion-dim), [cat HF](https://huggingface.co/tiantiaf/wavlm-large-categorical-emotion)) | 2025; cards upd. Aug 2025 | **RAIL** — code RAIL-1.1, models `openrail` (verified). Use-restricted (no surveillance/identity inference; synthetic media must be labeled), otherwise commercial-OK | WavLM-large / Whisper-large-v3 backbones (0.3-1.6B); MPS OK; est. 10-40× RT *(estimate)* | One consistent family for **emotion (cat + A/V/D) plus age/sex/accent/speech-flow/voice-quality** trait columns | Best "annotate everything with one family" option; the RAIL synthetic-media-labeling clause *matches* our provenance-manifest practice anyway |
| **LAION Empathic-Insight-Voice** (Small/Large) ([HF](https://huggingface.co/laion/Empathic-Insight-Voice-Small), [EmoNet-Voice paper](https://arxiv.org/abs/2506.09827)) | May-Jun 2025 | **Models CC-BY-4.0 (verified), code Apache-2.0** | Whisper-large-v3 encoder + ~54 MLP heads; est. 10-20× RT MPS *(estimate)* | **40 fine-grained emotions × 3 intensities + V/A** as continuous scores; trained on ~5k h *synthetic* voice acting + ~5k h in-the-wild; EmoNet-Voice Bench = 13k expert-verified clips | The taxonomy-rich annotator — and **the only candidate whose training domain is synthetic speech**, i.e. matched to our corpus. Clean license |
| **audEERING w2v2-large-robust-12-ft-emotion-msp-dim** ([HF](https://huggingface.co/audeering/wav2vec2-large-robust-12-ft-emotion-msp-dim)) + **wav2small** ([HF](https://huggingface.co/audeering/wav2small)) | 2022; wav2small 2024-25 | **CC-BY-NC-SA-4.0 (both, verified)** — NC | 317M / tiny | The citation-standard A/V/D regressor (valence CCC ≈ 0.60 on MSP-Podcast in-domain) | **Avoid for the pipeline** (NC taints downstream weights if annotations condition training); internal cross-check only |
| **Emotion2Vec-S** (ASLP-lab) ([HF](https://huggingface.co/ASLP-lab/Emotion2Vec-S), [arXiv 2502.18186](https://arxiv.org/abs/2502.18186)) | Feb 2025 | **Apache-2.0 (verified)** | emotion2vec-scale; MPS fine | Semi-supervised variant, EmoBox-evaluated | The Apache-clean emotion *embedding* alternative if the FunASR attribution chain bothers us |
| **EmoBox** toolkit + benchmark ([repo](https://github.com/emo-box/EmoBox), [Interspeech 2024](https://www.isca-archive.org/interspeech_2024/ma24b_interspeech.html)) | 2024; still the reference harness | Open toolkit | Harness only | 32 datasets / 14 languages intra + 4-dataset cross-corpus leaderboard | The harness to *qualify* whichever annotators we pick, especially cross-corpus |
| **Audio-LLM annotators** — Qwen2-Audio-7B, Audio Flamingo 3, Qwen2.5/Qwen3-Omni ([VoxEmo, arXiv 2603.08936](https://arxiv.org/html/2603.08936), [OmniVox, arXiv 2503.21480](https://arxiv.org/pdf/2503.21480)) | Benchmarked Mar 2026 | Qwen family Apache-2.0; **AF3 weights NVIDIA non-commercial** *(reported)* | 7-30B — spot box or slow Mac | Zero-shot hard-label SER is **poor**; but output *distributions* align better with multi-annotator ambiguity than classifiers do | Not the per-clip annotator. Niche: free-text affect captions on a small stratum; ambiguity-aware soft labels |

### E.2 The cross-corpus caveat, investigated honestly

**Still poor, now well-quantified, with a new synthetic-speech wrinkle that hits SPEAK directly:**

1. **In-domain ceilings are already low on naturalistic data.** The [Interspeech 2025 SER-in-naturalistic-conditions challenge](https://www.isca-archive.org/interspeech_2025/naini25_interspeech.pdf) (MSP-Podcast, 8-class): best systems reached only **~0.40 Macro-F1** — with audio+text fusion and in-domain training. Acted corpora flatter: EmoBox intra-corpus Whisper-large-v3 hits F1 73-75 on IEMOCAP/RAVDESS/SAVEE but only **32.95 on MELD** (naturalistic TV dialogue) ([leaderboard](https://emo-box.github.io/leaderboard1.html)). *(tried-and-true)*
2. **Cross-corpus drops are severe and asymmetric.** EmoBox [cross-corpus leaderboard](https://emo-box.github.io/leaderboard2.html): best model (Whisper-large-v3 probing) ranges **36-67% accuracy** by pair; SSL encoders fall to **~15%** (HuBERT-large, IEMOCAP→RAVDESS) — below chance. ASR-pretrained (Whisper-style) encoders generalize best; the pattern replicates across 2025-26 papers. *(tried-and-true)*
3. **Audio-LLMs don't rescue it.** [VoxEmo (Mar 2026)](https://arxiv.org/html/2603.08936): across 35 corpora / 15 languages, zero-shot Macro-F1 of Qwen2-Audio-7B and Audio Flamingo 3 falls **below 20% on eight datasets**; prompt choice alone swings results by up to **49 F1 points**; after LoRA, Q2A beats EmoBox specialized baselines on only 10/30 datasets, AF3 on 0/30. Their one advantage: probability distributions that track annotator disagreement. *(reported)*
4. **The synthetic-speech wrinkle (new, and ours).** ["On the Emotion Understanding of Synthesized Speech" (Mar 2026)](https://arxiv.org/abs/2603.16483): current SER models **fail to generalize to TTS-synthesized speech** — representation mismatch from token-based synthesis, shortcut learning, and speech-LMs leaning on text semantics over paralinguistics. Consequence: *post-hoc SER labels on our Qwen3-TTS / Chatterbox / Step-Audio-EditX corpus are systematically less reliable than on human speech.* Mitigations: (a) **the generation-side condition is the ground truth** — log the emotion tag / description / edit-instruction fed to the TTS and treat SER as a *verification* signal; (b) prefer annotators trained on synthetic speech — **Empathic-Insight-Voice is the only candidate whose training set is majority synthetic voice acting**, uniquely in-domain. *(reported; mitigation (a) is design reasoning)*
5. **Dimensional (V/A/D) is the more transferable frame** — arousal transfers decently; valence from audio alone is the weak axis (in-domain valence CCC ≈ 0.6 is the public ceiling; cross-domain lower). **Store continuous V/A/D + per-class probabilities, never argmax alone.** *(tried-and-true pattern; numbers reported)*

### E.3 Throughput on our hardware

All recommended annotators are 90M-1.6B encoders ⇒ MPS-viable. Estimated wall-clock for **500k clips** on the 16-core M-series *(estimates, not measured)*: emotion2vec+ base sweep ~1 day CPU-parallel; emotion2vec+ large or Vox-Profile dim (Whisper-encoder) ~1-3 days MPS-batched; Empathic-Insight full 40-emotion profile similar (one encoder pass, 54 cheap MLP heads). SenseVoice SER+AED rides along a transcription pass at negligible marginal cost. Audio-LLM captioning: spot-GPU only, ≤10k-clip stratum. A 25k-clip pilot bank: hours, not days, for the full stack. (§I folds these into the consolidated pass table.)

### E.4 For our use (E)

1. **Primary annotation battery — three families, stored as distributions:** **emotion2vec+ large** (9-way categorical probs + emotion embedding; FunASR license, attribution recorded in the manifest) + **Empathic-Insight-Voice-Small** (40-emotion × intensity + V/A continuous; CC-BY-4.0; the synthetic-domain match) + **Vox-Profile whisper-large-v3 A/V/D head** (dimensional anchor; openrail). Persist raw scores; derive discrete tags at query time. *(reported)*
2. **Ground truth is the generation condition; SER is the verifier.** Record the requested affect as `emo_intent` per clip; run the battery; store agreement. **Disagreement is signal** — it flags failed expressive renders for exclusion or hard-example mining, turning the 2603.16483 failure mode into a filter. *(tried-and-true reasoning on reported findings)*
3. **Do not build on the audEERING dimensional models** (CC-BY-NC-SA) despite their citation status; MERaLiON-SER only after reading its custom license. *(tried-and-true — licenses verified)*
4. **Qualify the battery once with EmoBox** and hand-audit ~200 clips before trusting thresholds. Expect naturalistic-grade accuracy (≈0.40 macro-F1 territory) and set consumer expectations accordingly: **coarse affect regions in program space are realistic; fine 40-way emotion geography is not, in 2026.** *(tried-and-true — the ceiling numbers are solid)*

---

## F. Corpus hygiene & tagging

**Landscape.** Hygiene splits into two problems for SPEAK: **(1) QC of corpora we generate** — dedup nearly free via provenance; the risks are TTS failure modes (hallucination, truncation, buzz, watermark interactions) — and **(2) Emilia-style ingest of in-the-wild expressive/singing audio** (separation → segmentation → SNR/quality → tagging → dedup). Reference pipeline design: **Emilia-Pipe** ([Amphion](https://github.com/open-mmlab/Amphion/tree/main/preprocessors/Emilia), [paper](https://arxiv.org/pdf/2501.15907)): standardize → source-separate (UVR-MDX-Net) → diarize → VAD-segment → ASR → filter (lang-conf >80%, DNSMOS OVRL >3.0, anomalous char/phone-duration rejection). **Adopt the shape, swap the parts.** *(tried-and-true design, reported parts)*

### F.1 Candidates

| Tool / model | Version / date | License (verified) | Apple Silicon footprint | Maturity | For-our-use one-liner |
|---|---|---|---|---|---|
| **dataspeech** (HF) ([repo](https://github.com/huggingface/dataspeech)) | Parler-TTS annotation stack; low 2026 churn | **MIT** | CPU-parallel via `datasets.map`; light | Proven at LibriTTS-R scale | One-command per-clip **SNR, C50, speech duration, speaking rate, pitch stats, PESQ, SI-SDR** columns — the manifest-enrichment workhorse |
| **Brouhaha** ([repo](https://github.com/marianne-m/brouhaha-vad), [paper](https://arxiv.org/abs/2210.13248)) | 2023; stable | Code **MIT**; HF model **openRAIL** | Small pyannote model; ~50-100× RT/core *(estimate)* | Standard joint **VAD + SNR + C50** (what dataspeech calls) | Speech-aware SNR (not global RMS) + room acoustics per clip |
| **DNSMOS P.835** ([repo](https://github.com/microsoft/DNS-Challenge/tree/master/DNSMOS)) | ICASSP 2022; unchanged | Repo **CC-BY-4.0** (ONNX in-repo) | Tiny ONNX; fast per-core | Industry-default noisiness gate (Emilia uses OVRL >3.0) | SIG/BAK/OVRL columns; the cheap "is this clean" gate for ingest |
| **audiobox-aesthetics** (Meta) ([repo](https://github.com/facebookresearch/audiobox-aesthetics), [paper](https://arxiv.org/abs/2502.05139)) | Feb 2025 | **CC-BY-4.0** (code + weights) | ~300M; MPS OK, ~20-50× RT *(estimate)* | Widely adopted 2025-26 for corpus filtering | Four orthogonal axes (**PQ/PC/CE/CU**) — a better stratifier than one MOS number; the PC axis also flags multi-source clips |
| torchaudio **SQUIM** | torchaudio 2.10 | BSD-2 | Trivial | torchaudio in **maintenance phase** (`SOTA-SURVEY` §H) | No-reference PESQ/STOI/SI-SDR columns today; plan for sunset |
| Distill-MOS / UTMOSv2 / NISQA | see `SOTA-SURVEY` §H | MIT / MIT / code-MIT + **NC weights** | CPU-fast / heavier / — | §H already adopted these | Reuse §H's **Distill-MOS** as the MOS column; add nothing new here |
| **silero-vad** ([repo](https://github.com/snakers4/silero-vad)) | v6 line; v6.2.1 Feb 2026 | **MIT** | <1 ms per 30 ms chunk per CPU thread | 6000-lang; ubiquitous | Silence/speech-ratio hygiene columns — **same single pass that §A uses for segmentation** (conflict #4) |
| **LAION-CLAP** ([repo](https://github.com/LAION-AI/CLAP), [larger_clap_general](https://huggingface.co/laion/larger_clap_general)) | 2023 weights; still the workhorse — **no verified post-2025 successor surfaced this pass** | Code **CC0-1.0**; `larger_clap_general` weights **Apache-2.0** (verified; 255k dl/mo) | ~650M pair; embeddings ~50× RT batched on MPS *(estimate)* | In `transformers`; boring-reliable | Zero-shot tagging for stratification (register, style, "contains laughter?", setting) **and** the shared embedding space for near-dup detection |
| **PANNs** ([repo](https://github.com/qiuqiangkong/audioset_tagging_cnn)) / **Dasheng** ([repo](https://github.com/XiaoMi/dasheng)) / CED ([repo](https://github.com/RicherMans/CED)) | 2019-frozen / 2024-active / 2023 | **MIT** / **Apache-2.0** / **GPL-3.0 ⚠** | CNN14 tiny; Dasheng has small variants | AudioSet closed-vocab taggers | Fixed-vocab event columns where CLAP zero-shot is too fuzzy; **skip CED (GPL)**; SenseVoice AED (§E) covers most speech-adjacent events anyway |
| **Music-Source-Separation-Training** (ZFTurbo) ([repo](https://github.com/ZFTurbo/Music-Source-Separation-Training)) | Active through 2026 | Code **MIT**; **checkpoints per-model — many community weights lack license files ⚠** | RoFormers heavy: ~1-4× RT on MPS *(estimate)*; spot GPU at scale | De-facto harness + model zoo; MVSEP leaderboards rank **BS-RoFormer-SW** best 6-stem (community consensus, Feb 2026); Mel-Band RoFormer lineage best vocals ([paper](https://arxiv.org/abs/2310.01809), ~11.2 dB SDR MUSDB18HQ) | The separation engine for singing ingest; **verify each checkpoint's license before its outputs enter the corpus** — the flagship [Kimberley Jensen vocal model](https://github.com/KimberleyJensen/Mel-Band-Roformer-Vocal-Model) repo has **no LICENSE file** (GitHub API: null; "MIT" asserted only in an issue thread) |
| **Demucs (htdemucs)** | Frozen since 2023 | MIT *(training knowledge)* | Comfortable on MPS | Superseded on SDR, license-clean end-to-end | Conservative fallback when checkpoint provenance matters more than +2 dB |
| **NeMo Speech Data Processor** ([repo](https://github.com/NVIDIA/NeMo-speech-data-processor)) | Active | **Apache-2.0** | CPU pipeline | Manifest-centric curation; **ASR-consistency filtering** built in; no SNR/dedup/CLAP processors (verified README) | Steal the pattern (drop utterances where reference text ≠ ASR output) rather than adopt the framework |
| **Dedup:** Chromaprint/AcoustID; audfprint; CLAP+FAISS ([Doppelganger, Jul 2026](https://arxiv.org/pdf/2607.04337); [variable-length fingerprinting, Mar 2026](https://arxiv.org/pdf/2603.23947)) | 2026 evidence | Chromaprint LGPL-2.1 *(training knowledge)*; FAISS MIT | Trivial-to-light | Fingerprinting catches exact/transcoded dups only: Chromaprint scores **R@1 0.066** on synthetic near-twins vs frozen **CLAP embeddings far higher** (Doppelganger) | Two-tier dedup: PCM/file hash for exact; **CLAP-embedding cosine + FAISS** for near-dup / mode-collapse. Skip fingerprinting for generated audio |
| **Clipping / bandwidth / loudness** | classic DSP | pyloudnorm MIT *(training knowledge)*; the rest is ~50 lines | >1000× RT/core | — | Sample-peak run-length clipping counter; **spectral-rolloff bandwidth estimate** (catches upsampled fakes — TTS vocoders emit fixed rates, so off-nominal rolloff = resampling artifact); LUFS via BS.1770 |

### F.2 The generated-corpus QC problem — our main case

Not Emilia: dedup and provenance are free (seed + params in the manifest, `SOTA-SURVEY` §I), and noise/reverb are absent by construction. What actually fails in TTS output, and the filters:

1. **Hallucination / truncation / repetition** → **dual-ASR consistency filter** (`SOTA-SURVEY` §H norm: whisper-large-v3 + Parakeet): WER-vs-input-text over threshold ⇒ quarantine; plus Emilia-style **char/phone-duration anomaly** rejection (dataspeech speaking-rate columns catch runaway repetition cheaply). *(tried-and-true pattern)*
2. **Vocoder buzz / degraded renders** → **Distill-MOS + audiobox-aesthetics PQ** double gate; disagreement = review flag, not auto-drop. *(reported)*
3. **Clipping / DC / silence pathologies** → the DSP row above; min/max speech-ratio via silero. *(tried-and-true)*
4. **Watermark stratum tracking** — Chatterbox output (PerTh) is a *systematic* sub-audible artifact: keep `watermarked: true` as a manifest column so the inverse-net ablation (`SOTA-SURVEY` §A honorable-mention decision) stays runnable. *(design note)*
5. **Mode-collapse near-dups across seeds** → CLAP-embedding cosine within same-text / same-speaker strata; audit clusters >0.98 cosine. Embedding, not fingerprint, per Doppelganger. *(reported)*

### F.3 For our use (F)

1. **Manifest enrichment pass (all corpora):** dataspeech (MIT) columns — SNR, C50, speaking rate, pitch stats — plus DNSMOS OVRL, Distill-MOS, audiobox-aesthetics 4-axis, clipping/rolloff/LUFS, silero speech-ratio. All CPU/MPS, all permissive (Brouhaha weights openRAIL = use-restricted-but-commercial-OK). *(tried-and-true components, reported at our scale)*
2. **Generated-corpus QC:** the five-filter battery above; thresholds tuned on a 1k-clip hand-audited sample **per generator**. The dual-ASR WER gate is the highest-yield single filter. *(reported)*
3. **Tagging / stratification:** LAION-CLAP `larger_clap_general` (Apache) zero-shot prompts for register/style/event presence + SenseVoice AED (from the §E sweep) for laughter/cry/cough columns; PANNs (MIT) or Dasheng (Apache) if a fixed AudioSet vocabulary is needed. **Avoid CED (GPL) in anything that ships.** *(reported)*
4. **Singing ingest separation:** ZFTurbo MSST harness (MIT) with a **license-verified** Mel-Band/BS-RoFormer checkpoint — check the exact checkpoint card first (community SOTA weights frequently ship with *no* license file); htdemucs (MIT) as the provenance-safe fallback. Spot GPU for bulk; MPS workable for pilots. *(reported; checkpoint-license risk tried-and-true — verified null-license on the flagship community model)*
5. **Dedup:** exact = PCM hash; near = CLAP + FAISS (both permissive). The provenance manifest makes generated-corpus dedup a non-problem by construction. *(tried-and-true mechanism / reported evidence)*

---

## G. Word & subword decomposition

### G.1 G2P candidates

| Tool | Version / date | License | Footprint | Maturity | For-our-use one-liner |
|---|---|---|---|---|---|
| **espeak-ng** (+ [phonemizer](https://github.com/bootphon/phonemizer)) | stable, 100+ langs | **GPL-3.0** (both) — the ecosystem is actively fleeing it: sherpa-onnx [removing the dependency](https://github.com/k2-fsa/sherpa-onnx/issues/3731), [LGPL petition open](https://github.com/espeak-ng/espeak-ng/issues/2131) | trivial CPU | The multilingual workhorse; also defines the token set of our chosen wav2vec2-espeak oracle | Keep as a **subprocess-only annotation pass** (GPL never linked into the Rust core); its IPA output stays the cross-check reference |
| **misaki** ([repo](https://github.com/hexgrad/misaki)) | active 2026 (hexgrad / Kokoro) | **Apache-2.0** | trivial | Kokoro's own G2P: EN gold/silver + ja/ko/zh/vi extras; stress-marked output | **Free gold labels**: for every Kokoro-generated clip, log misaki phonemes + stress in the manifest at generation time |
| **goruut** ([repo](https://github.com/neurlang/goruut)) | active; 927 commits | **MIT** (verified) | Go binary; ms-per-sentence; self-hostable server | **140 languages, G2P AND P2G (dephonemizer)**; lexicon + hashtron-model fallback | The permissive multilingual G2P — the license-clean espeak replacement. Per-language accuracy unpublished: spot-check before trusting a stratum |
| piper-plus-g2p ([PyPI](https://libraries.io/pypi/piper-plus-g2p)) | 0.2.0, 2026 | **MIT**, espeak-free | trivial | New TTS-oriented multilingual G2P from the Piper lineage | Watch; second permissive option *(reported)* |
| OpenPhonemizer ([repo](https://github.com/NeuralVox/OpenPhonemizer)) | **archived Mar 15 2026** | permissive | — | Was the "espeak-compatible permissive phonemizer"; dead | Do not adopt; its DeepPhonemizer base lives on |
| DeepPhonemizer ([repo](https://github.com/as-ideas/DeepPhonemizer)) | stable | MIT | small transformer | Trainable G2P; train on WikiPron data for any language | The "roll your own" path if goruut misses a language |
| CharsiuG2P ([repo](https://github.com/lingjzhu/CharsiuG2P)) | frozen ~2022 | Code open; **training data "mostly redistributable, some unspecified"** (flag) | ByT5-small/tiny; MPS OK | 100-language neural G2P, Interspeech 2022 | Historical; superseded by goruut for coverage, misaki for EN |
| g2p_en / gruut | stale | Apache / MIT | trivial | EN CMUdict + OOV nets; gruut has stress + syllable marks | Acceptable EN fallbacks; misaki is better maintained |

### G.2 IPA normalization, phoneme-class lattices, stress, syllables

| Need | Tool | License | Verdict |
|---|---|---|---|
| **IPA normalization** | Unicode NFD + an explicit segment inventory; **[panphon](https://github.com/dmort27/panphon)** as the validity oracle (is this a well-formed IPA segment?) | MIT *(training knowledge)* | Normalize every G2P/oracle output (misaki, espeak, goruut, ZIPA) to panphon-valid segments; drop/flag what doesn't parse. ZIPA's IPAPack++ conventions and MFA's harmonized-IPA dictionaries ([2026 paper](https://arxiv.org/abs/2606.18466)) are the two big public precedents |
| **Phoneme-class lattice** (the brief's conditioning ask) | **panphon**: segment → ~24 binary articulatory features (voice, nasal, continuant, place…) | MIT | **The answer.** Phone tier → per-frame articulatory-feature lattice: deterministic, language-independent, 0 params. This is also FALCON's multilingual transfer mechanism — independent evidence the representation carries. [Epitran](https://github.com/dmort27/epitran) (MIT) from the same lab for transparent-orthography languages |
| **Lexical stress** | CMUdict numeric stress (EN); misaki/espeak/gruut stress marks; **[WikiPron](https://github.com/CUNY-CL/wikipron)** (code Apache-2.0; data CC-BY-SA from Wiktionary, 1.7M+ pronunciations, many with stress + syllable marks) | mixed — CC-BY-SA is data-side only, fine for annotation | Stress from the lexicon at G2P time → per-syllable `lexical_stress` column; §B prominence *vs* lexical stress = the emphasis-deviation signal, exactly what conditioning wants |
| **Syllabification** | Maximum-onset + sonority over the phone string — deterministic code; Gorman's `syllabify` for ARPABET *(training knowledge)*; the wavelet toolkit and FonetiToBI-style scripts do the same internally | MIT-class | Write the ~100-line pure function in the pipeline (Rust or Python); no model, no license, no drift. Syllable tier = a grouping over the §A phone tier |
| **Morpheme lattices** | Morfessor 2.0 (BSD, unsupervised, stale-but-standard); MorphyNet (CC-BY-SA, 15 langs derivational + inflectional); UniMorph (CC-BY-SA); SIGMORPHON 2022 shared-task models — all *(training knowledge; search budget exhausted before verification)* | BSD / CC-BY-SA | **Deprioritize**: morpheme boundaries are weakly acoustic; phone-class + syllable + stress lattices carry nearly all the conditioning signal SPEAK can use. If wanted later: Morfessor lookup tables frozen at corpus-build time |

### G.3 For our use (G)

misaki at generation time for Kokoro strata (*tried-and-true* — it ships inside the chosen TTS); goruut as the permissive multilingual annotator with per-language spot-checks (*reported*); espeak-ng as the GPL-quarantined subprocess cross-check and the token bridge to the wav2vec2-espeak oracle (*tried-and-true*); panphon articulatory-feature lattices as the phoneme-class conditioning representation (*tried-and-true*); syllabification as deterministic in-house code (*tried-and-true*). **Everything in this section is CPU-trivial on the Mac; nothing needs MPS.**

---

## H. How models learned to laugh — the synthesis-side survey + laugh design sheet

### H.1 Three annotation regimes that produced laughing TTS

| Regime | Exemplars | What the training annotation actually was | Data scale | Lesson for an instrument |
|---|---|---|---|---|
| **1. Tags-in-transcript over in-the-wild data** | Bark, ChatTTS (100k h train / 40k h open ckpt), Dia, CosyVoice2, Fish, Orpheus (fine-tune), Chatterbox | Event detectors + ASR fused into "rich transcripts": NVV spans become inline tokens; the model learns tag→sound by ordinary TTS training. **Open blueprint: NonverbalTTS** ([paper](https://arxiv.org/abs/2507.13155)) — ASR + NV tagging + emotion classification + multi-annotator fusion + human validation; fine-tuning an open TTS on its **17 h** reached CosyVoice2-parity NV fidelity | 10⁴-10⁵ h wild (closed), but **17 h tagged suffices for fine-tune parity** | Tag + span annotation is enough; **no phonetic decomposition of the laugh itself is required** to reach current open-model quality |
| **2. Frame-level detector-embedding conditioning** | **ELaTE** ([arXiv 2402.07383](https://arxiv.org/abs/2402.07383), MS 2024); EmoCtrl-TTS / "Laugh Now Cry Later" ([arXiv 2407.12229](https://arxiv.org/pdf/2407.12229)) | A frozen ResNet laughter detector (Switchboard-trained) over AMI+SWB+Fisher → 32-d final-layer embeddings @43.1 fps as extra conditioning; fine-tune a 60k-h-pretrained flow-matching TTS on **459.8 h** laughter-bearing speech mixed 50:50 with neutral pretraining data. No code/weights released | 460 h laughter-bearing | **A detector's continuous output is a sufficient control signal** (timing corr 0.673, laugh SIM 0.796 vs ≈0 baseline). For GARGANTA: a laugh is a *low-dimensional continuous control envelope*, not a symbol — exactly the control-program thesis |
| **3. Discrete units over a dedicated laughter corpus** | Xin, Takamichi, Morimatsu, Saruwatari, Interspeech 2023 ([arXiv 2305.12442](https://arxiv.org/abs/2305.12442)) | SSL features → clustering → **pseudo-phonetic tokens** of laughter; 3.5 h in-the-wild Japanese laughter corpus; a token LM generates unconditional laughs | **3.5 h** | Laughter has its own learnable "phonetics"; **hours, not thousands of hours**, teach a laugh vocabulary |

Precursor worth citing: **Sundaram & Narayanan, JASA 2007** ([pdf](https://sail.usc.edu/publications/files/sundaram_jasa_2007.pdf), extracted this survey) — acceptable synthetic laughter from a **parametric bout model + LP vowel calls**, zero learned components. Lineage: parametric (2007) → HMM/DNN (2010s) → detector-conditioned & token-based (2023-24) → tag-based at scale (2024-26); NVBench (Apr 2026) is the first standardized eval. *(reported)*

**Minimal annotation set for a NEW voice (ours) to learn a convincing laugh** — the intersection of what each regime consumed:
1. laugh **span boundaries**;
2. **call-level segmentation** within the span (voiced call vs aspiration/silence);
3. per-call **F0 + amplitude peak**;
4. **voicing flag** per call;
5. optional bout-level **arousal/valence** for variety (EmoCtrl shows time-varying A/V curves suffice).

No IPA, no lexical context. *(reported synthesis of verified sources)*

### H.2 Acoustic anatomy of laughter — the design sheet

Hierarchy: **episode → bout (one exhalation) → call/note (one "ha") → pulse (glottal)**. Sources: Bachorowski, Smoski & Owren, JASA 2001 ([pubmed](https://pubmed.ncbi.nlm.nih.gov/11572368/) — 1,024 bouts, 97 adults; abstract verified this pass, the numbers below from the full paper via training knowledge); Provine 1991/1996; Trouvain 2003; Ruch & Ekman 2001; Sundaram & Narayanan 2007 (verified in full this pass).

| Parameter | Empirical value | Source / tag |
|---|---|---|
| Call (note) duration | ~50-150 ms (Provine ~75 ms; Bachorowski mean ~110-120 ms) | (lit., training knowledge) |
| Inter-call onset interval | ~200-230 ms (Provine ~210 ms) → **call rate ~4.3-5 Hz**; the brief's 4-6 Hz confirmed | (lit., training knowledge) |
| Calls per bout | typically 2-8, mean ~3-4 | (lit., training knowledge) |
| Bout duration | ~0.5-1.5 s, mean ~0.9 s | (lit., training knowledge) |
| Temporal envelope | **decrescendo**: first call longest & loudest; amplitude (often F0) declines across the bout. Sundaram: a **damped simple-harmonic oscillator** {A, m, k, b} reproduces call durations, inter-call gaps *and* peak-amplitude decay in one 4-parameter model — voiced calls on positive half-cycles, unvoiced gaps on negative | **tried-and-true** (Sundaram 2007 verified; Provine) |
| Voicing mix | voiced "song-like" bouts ~⅓; unvoiced snort-/grunt-like ~⅓+; mixed the rest. Within a voiced call voicing is high; between calls, glottal aspiration [h] | (lit., training knowledge; the voiced/unvoiced split is a verified headline finding) |
| F0 | far above speech: male mean ~250-280 Hz, female ~400-420 Hz; extremes >1 kHz (squeals, register breaks to M2); rise-fall arc per call; bout declination | (lit., training knowledge) |
| Vowel / filter | **central, unarticulated** — "consistent lack of articulation effects in supralaryngeal filtering" (abstract, verified): tract static, schwa-to-open-central; mouth degree separates "ha" from closed-mouth "hm" giggles | **tried-and-true** (verified abstract) |
| Source quality | high subglottal-pressure onsets; breathy-to-pressed alternation; jitter/shimmer elevated vs speech; aspiration noise dominates gaps; occasional period-doubling | (lit., training knowledge) |
| Respiration | the whole bout on one forced exhalation; an audible **ingressive inhalation** frequently terminates the bout | (lit., training knowledge) |
| Speech-laughs | a large fraction of conversational laughter is superimposed on running speech (Trouvain; Nwokah ~50%) — amplitude/F0 modulation of speech at the same ~5 Hz, not separate calls | (lit., training knowledge) |

**Control-program laugh for GARGANTA — concrete starting spec:**

- `laugh_rate` 4-6 Hz LFO (default 4.7) gating the breath-pressure port; duty cycle 0.3-0.5;
- `n_calls` 2-8 with −2 to −4 dB/call decay — **or** drive all timing from the Sundaram damped-oscillator {A, k, b, m}: **4 floats**, perfectly matched to the ~dozens-of-floats program budget;
- per-call F0 arc: rise-fall, base = 2-2.5× the voice's modal F0, bout declination 10-30%;
- glottal port per call: OQ 0.6-0.9 (breathy side), Rd ≥ 1.7, jitter 1-3%; inter-call: voicing off, aspiration noise on;
- tract frozen neutral/open-central for the whole bout;
- terminal 200-500 ms breathy **ingressive** (reverse-flow noise burst);
- variants: unvoiced snort (nasal-branch noise, no voicing), grunt (single short low-F0 call), speech-laugh (apply the LFO to a running speech program);
- optional call-1 register break (M2: OQ ~0.8, F0 ×1.5-2).

*Confidence: anatomy is tried-and-true literature; the parameter mapping is our design (untested).*

---

## I. Pipeline recommendation

**Written by the synthesis editor.** Design rule from the brief: *prefer few tools that compose over many that don't*. The stack below is built around four load-bearing choices:

- **One acoustic forward pass serves three jobs.** ZIPA-CR emissions give the intelligibility oracle (already paid for in `SOTA-SURVEY` §B), the phone alignment (§A), and the per-phone confidence lattice — no separate aligner in the hot path.
- **Everything deterministic is done in code, not models.** Syllabification, pause/breath-grouping, panphon feature lattices, z-lattices, clipping/rolloff/LUFS. Zero license surface, zero drift, ~free.
- **Neural passes are batched into exactly two sweeps** — one small-model CPU sweep and one MPS/GPU encoder sweep — because audio decode and resampling, not inference, dominate at 500k clips. Decode once, fan out to heads.
- **Generation-time capture beats post-hoc inference wherever the generator knows the answer** (phonemes, stress, affect intent, NV tags, seed, voice identity).

### I.1 Hardware and scale frame

Apple Silicon 16-core + MPS, occasional spot boxes. **Standardized clip assumption: 5 s mean** (reconciling scouts A and C — conflict #8):

| Bank | Clips | Audio hours | Note |
|---|---|---|---|
| Pilot | 25,000 | ~35 h | thresholds tuned here; full stack must fit one night |
| Mid | 100,000 | ~139 h | the working corpus |
| Full | 500,000 | ~694 h | the target |

**Scaling rule:** if the realized mean clip length is 8 s, multiply every wall-clock number below by **1.6×** (500k → ~1,110 h, Scout C's frame). CPU passes assume **12 concurrent workers** on a 16-core M-series; MPS passes assume single-stream batched inference. All figures are **estimates**, not measurements, unless a per-core rate is cited from a primary source.

### I.2 The ordered annotation stack

| # | Pass | Tools (license tier) | Emits | 25k | 100k | 500k | Where |
|---|---|---|---|---|---|---|---|
| **0** | **Generation-time capture** | misaki / goruut / espeak-ng subprocess; the generator itself | `gen_*` columns: phones, stress, seed, voice id, affect intent, NV tag vocabulary, watermark flag | free | free | free | inline with generation |
| **1** | **DSP hygiene** | pyloudnorm + ~50 lines (T0) | `qc_lufs_i`, `qc_dbtp`, `qc_clip_*`, `qc_dc_offset`, `qc_rolloff95_hz`, `qc_bandwidth_flag` | <1 min | ~2 min | **~10 min** | Mac CPU ×12 |
| **2** | **VAD + segmentation** | silero-vad v6 ONNX (T0); SOFA silences + F0 voicing on sung strata (T0) | speech/silence tier; `qc_speech_ratio`; pause list | ~6 min | ~25 min | **~2 h** | Mac CPU ×12 |
| **3** | **Oracle + alignment** (the anchor pass) | ZIPA-CR-small INT8 ONNX + vendored torchaudio label-priors CTC alignment (T0/BSD) | phone tier + per-phone log-likelihood + boundary confidence; `align_*` | ~20 min | ~1.2 h | **~4-6 h** | Mac CPU ×12, or ~1-2 h on one spot L4/A10 |
| **4** | **Derived tiers (pure code)** | max-onset syllabifier, panphon, breath-group logic, z-normalizers (T0) | syllable tier, pause/breath groups, articulatory-feature lattice, duration/energy z-lattices | <1 min | ~4 min | **~20 min** | Mac CPU ×12 |
| **5** | **F0 + voice-quality DSP** | SwiftF0 (T0) + penn/RMVPE agreement (T0); REAPER (T0) → QCP/IAIF (in-house) → NAQ/QOQ/Rd; Parselmouth CPPS/jitter/shimmer/HNR/formants (**T1 GPL subprocess**); WORLD D4C | `pros_*`, `vq_*`; per-frame F0/AP lattice | ~35-45 min | ~2.5 h | **~8-12 h** | Mac CPU ×12 |
| **6** | **Prosodic events** | wavelet_prosody_toolkit CWT (T0); `emphases` (T0, EN only) | `pros_prom_cwt*`, `pros_bound_cwt*`, `pros_emphasis_*`; per-syllable table | ~10 min | ~40 min | **~3 h** | Mac CPU ×12 |
| **7** | **Small-model neural sweep** (one decode, many heads) | DeepFry creak (T0); Respiro-en breath (T0); PANNs/Dasheng tagging (T0); jrgillick laugh spans on flagged clips (T0); Distill-MOS (T0); audiobox-aesthetics (CC-BY, T0); Brouhaha SNR/C50 (openRAIL, T2); LAION-CLAP embeddings (T0) | `vq_creak_*`, `nv_*`, `qc_mos_*`, `qc_aes_*`, `qc_snr_db`, `qc_c50_db`, CLAP embedding | ~1.5 h | ~5 h | **~20-28 h** | MPS batched |
| **8** | **ASR-consistency + event tags** | Parakeet-TDT-0.6B via parakeet-mlx (CC-BY, T2) on **all** clips; whisper-large-v3 second family only on the worst-decile tail; SenseVoice AED+SER ride-along (T2) | `qc_asr_*`, `qc_verdict`, clip-level event tags | ~1.5 h | ~6 h | **~30 h** (Mac) / ~3 h (spot) | MPS or spot GPU |
| **9** | **Affect battery** | emotion2vec+ large (T2) + Empathic-Insight-Voice-Small (CC-BY, T0) + Vox-Profile A/V/D head (RAIL, T2) | `emo_*` distributions, V/A/D, embeddings, `emo_intent_agree` | ~4-6 h | ~18 h | **~2-3 days** (Mac) / ~4-6 h (one L40S/A100 night) | **spot GPU at ≥100k** |
| **10** | **Dedup + stratification** | CLAP embeddings from pass 7 + FAISS (T0); PCM SHA-256 | `dup_*`, `tag_*` | ~5 min | ~20 min | **~2 h** | Mac CPU/MPS |
| **11** | **Gold / sampled passes** | MFA 3.4 on 5-10% (T0); FALCON evaluation (T0); SOFA QA on 50 sung clips (T0); LaughterSegmentation gold laugh spans on laugh-flagged clips (**T3, offline only**); OPENGLOT calibration of the GIF stack; ~200-clip hand audit | `align_gold_source`, QA tables, threshold calibration | ~1-2 h | ~3 h | **~8-12 h** (on the 5-10% sample) | Mac CPU ×12 |

**Wall-clock roll-up** *(estimates)*:

| Bank | Mac-only, end to end | With one spot-GPU night (passes 8-9) |
|---|---|---|
| 25k pilot | **~10-12 h — one overnight run** | not needed |
| 100k | ~1.5-2 days | **~1 day** |
| 500k | ~4-5 days | **~1.5 days** |

**Pole positions to watch:** pass 9 (Whisper-large-v3-encoder-based affect heads) is the single largest cost — it is the one pass to move to a spot box, or to run on a stratified 20% subsample with the generation-side `emo_intent` carrying the rest. Pass 8's second ASR family is deliberately *gated* to the worst decile: dual-family WER is a confound control (`SOTA-SURVEY` §H), not something every clip needs. Pass 5's QCP glottal inversion is the second pole and should be gated to voiced frames and skipped where `vq_creak_frac` is high (QCP degrades on creak) — those clips get NAQ/Rd = null rather than a wrong number.

**Composition properties this ordering buys:**
- Pass 3 depends only on pass 0 (phones) and pass 2 (speech regions); passes 4-6 depend only on pass 3; passes 7-9 are independent of alignment entirely and can run concurrently on a second device.
- Every gate that can *quarantine* a clip (passes 1, 2, 8) runs before the expensive passes where possible — pass 1 and 2 rejects never reach passes 7-9. On a corpus with a 3-5% TTS failure rate that is a small but free saving; more importantly it keeps the affect battery from being polluted by truncated renders.
- Nothing in passes 0-6 needs a GPU. A GPU-less fallback of the whole pipeline exists (drop passes 7-9 to CPU, ~3× slower) — worth preserving as the reproducibility path.

### I.3 Manifest schema extension

The existing threader-bank JSONL manifest carries: `id, stratum, group, request, probeKey, renderDigest, samples, peak, rms`. The extension below adds **scalar, per-clip columns to the JSONL** and moves everything dense (per-frame, per-phone, per-syllable) into **sidecar tables referenced by path + digest**. This is the important structural decision: a 500k-row JSONL with inline per-frame F0 arrays is unusable; a 500k-row JSONL of ~130 scalars is ~400 MB and greps fine.

**Conventions:** `float32` unless noted; `null` is meaningful and means *not computed* (distinguish from `0.0`); every column that comes from a model carries its producing tool in a `*_src` string or in the pass-level `enrich_tools` block; timestamps in seconds (float32), durations in ms (float32).

#### I.3.1 Provenance & generation-time (pass 0)

| Column | Type | Notes |
|---|---|---|
| `gen_engine` | string | e.g. `qwen3-tts-1.7b`, `kokoro-82m`, `chatterbox-turbo`, `soulx-singer` |
| `gen_engine_version` | string | checkpoint revision/sha |
| `gen_seed` | int64 | reproducibility; already implied by `probeKey` but stored explicitly |
| `gen_voice_id` | string | synthetic speaker identity (Qwen3-TTS voice-design descriptor hash, or Kokoro preset name) |
| `gen_voice_desc` | string, nullable | the raw voice-design text, where applicable |
| `gen_lang` | string | BCP-47 |
| `gen_text` | string | the exact input text |
| `gen_g2p_tool` | enum | `misaki` / `goruut` / `espeak-ng` / `internal` / `none` |
| `gen_phones` | string | space-separated IPA, panphon-normalized (§G) |
| `gen_stress` | string | per-syllable `0/1/2` string, aligned to `gen_syllables` |
| `gen_n_syllables` | int32 | from the deterministic syllabifier |
| `gen_nv_tags` | array\<string\> | NV tags present in the request, e.g. `["laugh","breath"]` |
| `gen_nv_tag_vocab` | string | the tag inventory of this checkpoint — resolves conflict #6 empirically |
| `gen_nv_syntax` | enum | `bracket` / `paren` / `angle` / `span` / `none` |
| `emo_intent` | string | the requested affect (tag, description, or edit instruction) |
| `emo_intent_scheme` | enum | `tag` / `freetext` / `editx-instruction` / `none` |
| `watermarked` | bool | true for Chatterbox/PerTh output |
| `watermark_scheme` | enum | `perth` / `audioseal` / `none` |
| `license_chain` | string | generator license → output license, per `SOTA-SURVEY` §I |

#### I.3.2 Hygiene & QC (passes 1, 2, 7, 8)

| Column | Type | Notes |
|---|---|---|
| `qc_dur_s` | float32 | derived from `samples` but denormalized for filtering |
| `qc_lufs_i` | float32 | BS.1770 integrated loudness |
| `qc_dbtp` | float32 | true peak |
| `qc_clip_frac` | float32 | fraction of samples in a clipped run |
| `qc_clip_runs` | int32 | count of runs ≥3 samples at full scale |
| `qc_dc_offset` | float32 | |
| `qc_rolloff95_hz` | float32 | spectral rolloff — catches upsampled fakes |
| `qc_bandwidth_flag` | enum | `native` / `upsampled` / `unknown` |
| `qc_speech_ratio` | float32 | silero v6 |
| `qc_lead_silence_ms`, `qc_trail_silence_ms` | float32 | truncation forensics |
| `qc_snr_db`, `qc_c50_db` | float32 | Brouhaha |
| `qc_dnsmos_sig`, `qc_dnsmos_bak`, `qc_dnsmos_ovrl` | float32 | DNSMOS P.835 |
| `qc_mos_distill` | float32 | Distill-MOS (§H of the sibling survey) |
| `qc_aes_pq`, `qc_aes_pc`, `qc_aes_ce`, `qc_aes_cu` | float32 | audiobox-aesthetics 4-axis |
| `qc_asr_text_primary` | string | Parakeet transcript |
| `qc_asr_wer_primary` | float32 | vs `gen_text` |
| `qc_asr_wer_secondary` | float32, nullable | Whisper; only populated on the gated tail |
| `qc_asr_family_delta` | float32, nullable | dual-family confound check |
| `qc_verdict` | enum | `pass` / `review` / `quarantine` |
| `qc_reasons` | array\<string\> | e.g. `["wer>0.25","clip_frac>0.001","aes_pq<3.0"]` |

#### I.3.3 Alignment (passes 3, 4, 11)

| Column | Type | Notes |
|---|---|---|
| `align_tool` | enum | `zipa_ctc_lp` / `mfa34` / `sofa` / `falcon` |
| `align_model_rev` | string | checkpoint sha |
| `align_n_phones` | int32 | |
| `align_phone_ll_mean`, `align_phone_ll_min` | float32 | per-phone log-likelihood from the label-priors path score |
| `align_boundary_conf_mean` | float32 | per-boundary confidence, not a point estimate (arXiv 2506.01256) |
| `align_frac_estimated` | float32 | BFA-style `is_estimated` fraction, where the aligner reports it |
| `align_n_syllables` | int32 | |
| `align_n_pauses`, `align_pause_total_ms` | int32 / float32 | gaps ≥150 ms |
| `align_breath_count`, `align_breath_total_ms` | int32 / float32 | Respiro-en |
| `align_speech_rate_syl_s`, `align_artic_rate_syl_s` | float32 | total vs pause-excluded |
| `align_gold_source` | enum | `none` / `mfa` / `sofa` / `human` |
| `align_gold_delta_ms` | float32, nullable | mean \|boundary − gold\| on the sampled clips — **the pipeline's own accuracy telemetry** |

#### I.3.4 Prosody (passes 5, 6)

| Column | Type | Notes |
|---|---|---|
| `pros_f0_src` | string | e.g. `swiftf0+penn` |
| `pros_f0_med_hz`, `pros_f0_min_hz`, `pros_f0_max_hz` | float32 | |
| `pros_f0_iqr_st`, `pros_f0_range_st` | float32 | semitones — the speaker-invariant form |
| `pros_f0_slope_st_s` | float32 | utterance declination |
| `pros_voiced_frac` | float32 | |
| `pros_f0_agree_cents` | float32 | median inter-tracker disagreement |
| `pros_f0_conf` | float32 | 0-1; low = trackers disagree or voicing ambiguous |
| `pros_energy_z_mean`, `pros_energy_z_std` | float32 | speaker-normalized |
| `pros_dur_z_std` | float32 | per-phone-class normalized duration spread |
| `pros_prom_cwt_mean`, `pros_prom_cwt_max` | float32 | wavelet toolkit |
| `pros_bound_cwt_max` | float32 | |
| `pros_emphasis_max_word` | float32, nullable | `emphases`, EN only |
| `pros_emphasis_argmax_word` | string, nullable | |
| `pros_stress_prom_corr` | float32 | correlation of lexical stress with measured prominence — **the emphasis-deviation signal** |

#### I.3.5 Voice quality & glottal (pass 5)

| Column | Type | Notes |
|---|---|---|
| `vq_jitter_loc`, `vq_shimmer_loc` | float32 | voiced-segment gated |
| `vq_hnr_db`, `vq_cpps_db` | float32 | CPPS is the robust breathiness scalar |
| `vq_h1h2_star_med` | float32 | corrected spectral tilt (opensauce) |
| `vq_ap_band_mean` | float32 | WORLD D4C band aperiodicity |
| `vq_creak_frac`, `vq_creak_max_run_ms` | float32 | DeepFry |
| `vq_naq_med`, `vq_naq_iqr` | float32 | |
| `vq_qoq_med`, `vq_oq_med` | float32 | |
| `vq_rd_med`, `vq_rd_iqr` | float32 | **the one-float glottal knob** (§C.3) |
| `vq_gif_valid_frac` | float32 | fraction of cycles where QCP inversion passed sanity bounds |
| `vq_gif_src` | enum | `qcp` / `iaif` / `golf` / `none` |
| `vq_f1_med`, `vq_f2_med`, `vq_f3_med` | float32, nullable | null above F0 ≈ 350 Hz by design |
| `vq_formant_conf` | float32 | |
| `vq_phonation_label` | enum | `modal` / `breathy` / `pressed` / `creaky` / `falsetto` / `mixed` |
| `vq_phonation_conf` | float32 | derived from the §C.3 target table, not a trained classifier |

#### I.3.6 Nonverbal (pass 7)

| Column | Type | Notes |
|---|---|---|
| `nv_event_count` | int32 | |
| `nv_classes` | array\<string\> | NVBench 45-type taxonomy terms |
| `nv_detector_stack` | string | e.g. `sensevoice+jrgillick+respiro` |
| `nv_laugh_frac` | float32 | fraction of clip duration inside laugh spans |
| `nv_laugh_bouts` | int32 | |
| `nv_laugh_calls_med` | int32 | median calls per bout |
| `nv_laugh_call_rate_hz` | float32 | **the 4-6 Hz design parameter, measured** |
| `nv_laugh_voiced_frac` | float32 | voiced/unvoiced call ratio (§H.2) |
| `nv_laugh_decay_db_call` | float32 | measured decrescendo slope — fits the Sundaram oscillator |
| `nv_breath_count`, `nv_breath_ingressive` | int32 / bool | |
| `nv_sigh_count`, `nv_filler_count` | int32 | |
| `nv_speechlaugh` | bool | CosyVoice-style co-articulated laughter |
| `nv_intent_match` | float32 | agreement between `gen_nv_tags` and detected `nv_classes` |

#### I.3.7 Affect (pass 9)

| Column | Type | Notes |
|---|---|---|
| `emo_cat_top` | string | argmax — **stored but never used alone** |
| `emo_cat_probs` | map\<string,float32\> | emotion2vec+ 9-way distribution |
| `emo_valence`, `emo_arousal`, `emo_dominance` | float32 | 0-1 |
| `emo_dim_src` | enum | `vox-profile` / `meralion` / `empathic` |
| `emo_fine_top3` | array\<{label:string, intensity:int8, score:float32}\> | Empathic-Insight 40-way |
| `emo_intent_agree` | float32 | **the verifier column** — agreement with `emo_intent` |
| `emo_verifier_verdict` | enum | `agree` / `weak` / `disagree` — `disagree` is a hard-example mining flag, not necessarily a reject |

#### I.3.8 Tagging & dedup (pass 10)

| Column | Type | Notes |
|---|---|---|
| `tag_clap_zs` | map\<string,float32\> | zero-shot prompt scores (register, style, setting) |
| `tag_audioset_top` | array\<{label:string, score:float32}\> | PANNs/Dasheng |
| `tag_register` | enum | `read` / `conversational` / `expressive` / `sung` / `whisper` / `other` |
| `dup_pcm_sha256` | string | exact-dup key |
| `dup_near_cluster_id` | string, nullable | FAISS cluster over CLAP embeddings |
| `dup_max_cos` | float32 | max cosine to any other clip in the same text/speaker stratum |

#### I.3.9 Sidecar tables (dense tiers)

Referenced from the JSONL, never inlined:

| Column | Type | Notes |
|---|---|---|
| `sc_phone_path`, `sc_phone_digest` | string | Parquet: `clip_id, idx, t0, t1, phone_ipa, ll, conf, is_estimated` |
| `sc_syl_path`, `sc_syl_digest` | string | Parquet: the §B.5 per-syllable schema, verbatim |
| `sc_frame_path`, `sc_frame_digest` | string | Parquet or NPZ: `t, f0_hz, voiced, f0_conf, energy_db, ap_band[k], creak_p, breath_p, naq, rd, artic_feat[24]` |
| `sc_frame_hop_ms` | float32 | 10 or 16 ms — declare it, don't infer it |
| `sc_embed_path`, `sc_embed_digest` | string | NPZ: CLAP + emotion2vec embeddings |
| `sc_format` | enum | `parquet` / `npz` |
| `enrich_version` | string | semver of the enrichment pipeline itself |
| `enrich_tools` | map\<string,string\> | tool → version/sha for every model that touched this row — **the attribution record required by the T2 licenses** |

### I.4 License roll-up

Tiers, defined by what each license permits *for this project*:

- **T0 — core-clean.** MIT / Apache-2.0 / BSD / CC0 / CC-BY. May be linked into the Rust core, shipped, and used to condition trained weights.
- **T1 — GPL, subprocess-quarantined.** Never linked into the Rust core or any shipped binary; invoked as a separate process whose *measurements* enter the manifest. (This is the standard reading and the one `SOTA-SURVEY` already applies to Parselmouth; it is a process-boundary discipline, not a legal opinion.)
- **T2 — use-restricted but commercial-OK.** RAIL / openRAIL / FunASR model license / CC-BY with attribution obligations. Usable in the pipeline; **obligations recorded in `enrich_tools`**; cannot be called "open source" in a model card.
- **T3 — eval/QA only.** NC / ND / research-only weights. May never condition training or ship; outputs must not enter the corpus.
- **T4 — avoid / unresolved.** Blocked pending a license read, or excluded outright.

| Tool / asset | License | Tier | Role in the recommended stack | Note |
|---|---|---|---|---|
| ZIPA-CR-small | MIT | **T0** | Pass 3 anchor (oracle + alignment) | already adopted in `SOTA-SURVEY` §B |
| torchaudio CTC align + label priors | BSD-2 | **T0** | Pass 3 glue | **vendor it** — torchaudio in maintenance |
| MFA 3.4 | MIT | **T0** | Pass 11 gold sample | |
| FALCON | MIT | **T0** | Pass 11 evaluation | promote if TIMIT/Buckeye gains replicate |
| SOFA | MIT | **T0** | Pass 2/3 sung strata | |
| silero-vad v6 | MIT | **T0** | Pass 2 | weak on singing (own docs) |
| Respiro-en | MIT | **T0** | Pass 7 breath | |
| SwiftF0 / penn / RMVPE / CREPE | MIT | **T0** | Pass 5 F0 | SwiftF0 MIT **verified** — supersedes `SOTA-SURVEY` §H "unknown" |
| WORLD / pyWORLD (D4C) | MIT/BSD | **T0** | Pass 5 aperiodicity | |
| REAPER | Apache-2.0 | **T0** | Pass 5 GCI | |
| DisVoice | MIT | **T0** | Pass 5 phonation + glottal | |
| opensauce-python | Apache-2.0 | **T0** | Pass 5 H1\*-H2\* | permissive route to corrected tilt |
| GOLF | MIT | **T0** | GARGANTA glottal-port donor code | |
| DeepFry | MIT | **T0** | Pass 7 creak | |
| jrgillick/laughter-detection | MIT | **T0** | Pass 7 laugh spans | |
| PANNs / AST | MIT | **T0** | Pass 7 tagging | |
| Dasheng | Apache-2.0 | **T0** | Pass 7 tagging (modern alt) | |
| wavelet_prosody_toolkit | MIT | **T0** | Pass 6 prominence | |
| emphases | MIT | **T0** | Pass 6 word emphasis (EN) | |
| misaki | Apache-2.0 | **T0** | Pass 0 G2P (Kokoro) | |
| goruut | MIT | **T0** | Pass 0 G2P (multilingual) | spot-check per language |
| panphon / Epitran | MIT | **T0** | Pass 4 feature lattice | |
| WikiPron code | Apache-2.0 | **T0** | stress lexicon build | data is CC-BY-SA (annotation-side only) |
| dataspeech | MIT | **T0** | Pass 1/7 columns | |
| DNSMOS | CC-BY-4.0 | **T0** | Pass 7 gate | |
| audiobox-aesthetics | CC-BY-4.0 | **T0** | Pass 7 stratifier | |
| Distill-MOS | MIT | **T0** | Pass 7 MOS | already adopted §H |
| LAION-CLAP (`larger_clap_general`) | code CC0-1.0 / weights Apache-2.0 | **T0** | Pass 7 embeddings, Pass 10 dedup | |
| FAISS | MIT | **T0** | Pass 10 | |
| Empathic-Insight-Voice-Small | weights CC-BY-4.0, code Apache-2.0 | **T0** | Pass 9 fine-grained affect | **the synthetic-domain match** |
| Emotion2Vec-S | Apache-2.0 | **T0** | Pass 9 alternative embedding | if the FunASR chain is unwanted |
| htdemucs | MIT | **T0** | singing-ingest fallback separator | |
| pyloudnorm / pystoi | MIT | **T0** | Pass 1 | |
| **Parselmouth / Praat** | **GPL-3.0** | **T1** | Pass 5 CPPS/jitter/shimmer/HNR/formants | subprocess only; never linked |
| **espeak-ng / phonemizer** | **GPL-3.0** | **T1** | Pass 0 cross-check + oracle token bridge | subprocess only; the ecosystem is migrating away |
| **BFA** | **GPL-3.0** | **T1** | optional fast CPU aligner | 240× MFA speed; subprocess only |
| SenseVoiceSmall | FunASR Model License (custom, commercial-OK, attribution + no-disparagement) | **T2** | Pass 8 ride-along AED/SER | conflict #2 resolved here |
| emotion2vec+ large | FunASR Model License; **code repo has no LICENSE** | **T2** | Pass 9 categorical + embedding | code-license gap is a real flag |
| Vox-Profile (code + models) | RAIL-1.1 / openrail | **T2** | Pass 9 V/A/D anchor | its synthetic-media-labeling clause matches our manifest practice |
| Brouhaha weights | openRAIL | **T2** | Pass 7 SNR/C50 | code is MIT |
| Parakeet-TDT-0.6B | CC-BY-4.0 | **T2** | Pass 8 primary ASR | attribution |
| VocalSound | CC-BY-SA-4.0 | **T2** | NVV classifier training data | **SA→weights contested** (conflict #9) — decide before shipping a finetune |
| AMI | CC-BY-4.0 | **T2** | conversational laughter reference | |
| COUGHVID | CC-BY-4.0 | **T2** | cough stratum | |
| openSMILE / eGeMAPS | source-available, research/education; commercial needs paid license | **T3** | optional 88-dim VQ vector, internal only | do not redistribute derivatives |
| LaughterSegmentation weights | research-use-only (code MIT) | **T3** | Pass 11 gold laugh spans, offline | |
| MERaLiON-SER-v1 | MERaLiON Public License (custom) | **T3 pending read** | candidate joint cat+dim | read before any dependency |
| audEERING w2v2 dim / wav2small | CC-BY-NC-SA-4.0 | **T3** | internal cross-check only | **do not** let it condition training |
| NISQA weights | CC-BY-NC-SA | **T3** | internal eval | already flagged §H |
| NonverbalTTS | Apache metadata **vs** NC sources (Expresso) | **T3** | eval + pipeline validation | contradiction #10 — per-source filtering required |
| PodcastFillers annotations | NC | **T3** | recipe reference | re-implement, don't ship |
| Deeply-NVV | CC-BY-NC-ND-4.0 | **T3** | eval only | ND blocks derivatives |
| Hume-VB / ExVo | research EULA | **T3** | eval only | |
| ESC-50 | CC-BY-NC | **T3** | eval only | |
| ChatTTS weights | CC-BY-NC (code AGPL-3) | **T3** | tag-convention reference only | do not generate corpus with it |
| ctc-forced-aligner default weights (MMS-300m) | CC-BY-NC-4.0 | **T3** | — | code is BSD; **swap in ZIPA** or don't use |
| CED | GPL-3.0 | **T4** | — | skip; Dasheng/PANNs cover it |
| TEN VAD | "Apache-2.0 with additional conditions" | **T4 pending read** | candidate frame-level VAD | resolve the conditions or stay on silero |
| Prosodic-prominence-ASR (2508.04814) | unstated | **T4 pending read** | candidate categorical pitch-accent tier | code is public, license is not |
| creapy | unstated (TU Graz GitLab) | **T4 pending read** | creak interval proposals | DeepFry covers the need meanwhile |
| GFM-IAIF | no license stated | **T4 pending read** | GIF reference | port the algorithm from the paper instead |
| Community Mel-Band-RoFormer vocal checkpoints | **null** (GitHub API) | **T4** | singing separation | "MIT" asserted only in an issue thread — use htdemucs unless a card is verified |
| MMS-1b-all, XEUS, Fish/OpenAudio, F5 base, Higgs, Voxtral | NC / restrictive (see `SOTA-SURVEY` §I) | **T4** | — | already excluded upstream |

**Net position:** the entire recommended hot path (passes 0-4, 6, 10) is **T0**. Exactly three GPL tools are used, all as **T1 subprocesses** (Parselmouth, espeak-ng, optionally BFA). Five **T2** components carry attribution obligations, all recorded in `enrich_tools`. No **T3** asset touches the corpus or conditions any trained weight — they appear only in pass 11 (gold labeling and QA) and in evaluation. Four **T4** items are blocked pending a license read, and in every case a T0 substitute is already in the stack, so none of them is on the critical path.

### I.5 Ordered adoption plan

1. **Week 1 — passes 0-4 on the 25k pilot.** Generation-time capture, hygiene, VAD, ZIPA alignment, derived tiers. This is the whole skeleton and it is entirely T0. Hand-audit 200 clips; measure `align_gold_delta_ms` against MFA on 5%.
2. **Week 2 — passes 5-6.** F0 + voice-quality + glottal lattice. Calibrate the GIF stack on **OPENGLOT before** trusting a single NAQ/Rd number on TTS output. This is where GARGANTA's glottal-port design gets its empirical grounding.
3. **Week 3 — passes 7-8.** Small-model sweep and the ASR-consistency gate. Tune quarantine thresholds per generator on 1k hand-audited clips. Expect this to be the highest-yield QC investment.
4. **Week 4 — pass 9 + 10.** Affect battery (qualify once with EmoBox) and dedup. Treat `emo_intent` as the label and the battery as the verifier from day one; do not let a 0.40-macro-F1 annotator become the ground truth of the conditioning space.
5. **Then scale to 100k → 500k**, moving passes 8-9 to a spot box, and keep pass 11 running at 5-10% forever as the pipeline's own accuracy telemetry.

**Deliberate omissions**, so the "few tools that compose" rule holds: no morpheme segmentation (weakly acoustic, §G); no categorical ToBI (unreliable at scale, §B); no diarization unless the Dia2 dialogue stratum needs it; no source separation except on the singing-ingest path; no audio-LLM captioning outside a ≤10k-clip stratum.

---

## What moved since early 2026 — merged across scouts

### Alignment & segmentation
- **MFA got a paper and a number** ([arXiv 2606.18466](https://arxiv.org/abs/2606.18466), Jun 2026; v3.4.0 Jul 2026): <15 ms mean boundary error, formally benchmarked against neural aligners — the citable baseline is now explicit.
- **Differentiable alignment beat MFA in public**: **FALCON** (MIT, Jun 2026, Keshet lab) +11 pts @≤25 ms on TIMIT and +9 on Buckeye, with zero-shot multilingual checkpoints. New and unmileaged; worth a local replication.
- **Real-time CPU alignment arrived**: BFA v1.1.0 (Feb 2026, GPL-3.0) at ~240× MFA speed with per-phone confidence; ctc-forced-aligner v0.2 (Jun 2026) dropped its torch C++ dependency (10× smaller, 50% faster).
- **torchaudio entered maintenance phase** — the chosen label-priors alignment glue should be vendored, not depended on. (Corroborated independently by scouts A and C and by `SOTA-SURVEY` §H.)
- **VAD**: silero v6.2.1 (Feb 2026, MIT) is steady-state; TEN VAD (Apache-with-conditions) now beats it on PR curves at lower compute; silero's own docs concede weakness on singing/high pitch — a real gap for our sung stratum that SOFA + F0-voicing fills.

### Prosody & decomposition
- **F0 tracking got a league table**: [lars76/pitch-benchmark](https://github.com/lars76/pitch-benchmark) (13 trackers × 8 datasets) with **SwiftF0** (MIT, 95k params, Aug 2025) on top and **penn shown fragile in noise (−29 pts @10 dB)** — this changes the default tracker choice, and resolves `SOTA-SURVEY` §H's open license question on SwiftF0 (MIT, verified).
- **Pitch-accent detection SOTA moved** (Aug 2025, [arXiv 2508.04814](https://arxiv.org/html/2508.04814v1)): F1 0.90 @100 ms with semi-supervised wav2vec2, code public — but still **no maintained AuToBI-style tool**; continuous CWT prominence remains the practical annotation norm, and it happens to be the right conditioning shape for a control-program space.
- **The espeak-ng exodus became structural**: OpenPhonemizer archived (Mar 2026), sherpa-onnx removing espeak-ng, MIT alternatives consolidating (goruut 140-lang G2P+P2G; piper-plus-g2p). **GPL G2P is now avoidable at annotation quality** — a genuine change from January.

### Voice quality & glottal
- **Voice-quality forensics got no new flagship open tool in 2025-26.** Surfboard archived Jan 2025; covarep dormant; creapy and DeepFry unchanged. The Praat/covarep-lineage classical stack remains SOTA for scalar VQ — meaning **our QCP + NAQ/Rd lattice will not be commoditized under us**. The one fresh 2026 signal points our way: training-free interpretable acoustic parameters for timbre attributes ([arXiv 2603.05091](https://arxiv.org/pdf/2603.05091), Mar 2026).
- **No open neural glottal inverse filter appeared** (negative finding, multiple search angles). Classical QCP/IAIF plus the differentiable **GOLF** (MIT) analysis-by-synthesis route are the Aug-2026 state of the art for glottal recovery at scale.
- **No high-F0/sung formant breakthrough either** (second negative finding). Store spectral envelopes + candidates with uncertainty above F0 ≈ 350 Hz rather than committing to F1/F2 points.

### Nonverbals & laughter
- **NVBench / NVV-SuperBench (Apr 2026, [arXiv 2604.16211](https://arxiv.org/html/2604.16211v2))** — the first standardized benchmark for NVV *generation*: 45-type taxonomy, 4,500 bilingual instances, 15 systems. It documents the tag-convention chaos and a coverage↔correctness trade-off; ElevenLabs v3 tops the subjective ranking. **Adopt its taxonomy as our tag superset.**
- **Laughter understanding moved into (AV-)LLMs**: SMILE-Next (May 2026); SenseVoice-class ASR emits event tags inline; "Beyond Words" (Jul 2026) gives data-centric recipes for NVV-emitting ASR. Meanwhile **dedicated open laughter detection plateaued** — jrgillick (2021) is still the practical default; Omine 2024 is the boundary-quality upgrade (research-only weights); MultiLinguahah (May 2026) is the first multilingual/unsupervised entry.
- **NonverbalTTS (SSW 2025): 17 h of tag + text-aligned NV data fine-tunes an open TTS to closed-model NV parity** — the single most useful number for GARGANTA's expressive-ladder planning. Note its Apache-metadata-vs-NC-sources contradiction before touching the audio.
- **Dedicated laughter-*synthesis* research went quiet in 2025-26** — the capability was absorbed into general expressive TTS (the tags-in-transcript regime); the last dedicated-method papers are ELaTE/EmoCtrl (2024). The *analysis* literature (Bachorowski 2001, Provine, Sundaram 2007) remains the only place the laugh's internal structure is actually written down — **which is exactly the layer a control-program instrument needs and the layer today's tag-based models skip.**

### Affect
- **SER on synthesized speech was shown broken** ([arXiv 2603.16483](https://arxiv.org/abs/2603.16483), Mar 2026) — representation mismatch, shortcut learning, text-semantic leaning. For anyone annotating TTS corpora (us), **generation-side condition tags become the label of record and SER demotes to verifier.** Corollary opportunity: LAION's synthetic-trained Empathic-Insight (CC-BY-4.0) is accidentally the in-domain annotator.
- **Audio-LLMs got honestly benchmarked for SER and lost** ([VoxEmo, Mar 2026](https://arxiv.org/html/2603.08936)): sub-20% zero-shot Macro-F1 on 8 of 35 corpora, 49-point prompt sensitivity, LoRA-finetuned 7B models beating specialized baselines on only a third of datasets. Their genuine niche: ambiguity-calibrated soft labels.
- **No new open SER foundation model displaced emotion2vec+** — the "Whisper of SER" still does not exist. Fresh entrants are region/taxonomy specialists: MERaLiON-SER (Nov 2025, custom license), Vox-Profile trait suite (RAIL), Emotion2Vec-S (Apache). The in-domain naturalistic ceiling remains **~0.40 Macro-F1** (Interspeech 2025 challenge), unmoved.

### Hygiene, tagging, separation
- **Corpus-hygiene practice consolidated around Emilia-Pipe's shape** (separate → segment → ASR-consistency → DNSMOS-gate), with **audiobox-aesthetics (CC-BY-4.0)** displacing single-number MOS as the stratification signal in 2025-26 pipelines.
- **Dedup evidence flipped from fingerprints to embeddings** ([Doppelganger, Jul 2026](https://arxiv.org/pdf/2607.04337): Chromaprint R@1 0.066 on near-twins vs frozen CLAP far higher) — embedding-cosine near-dup detection is now the defensible default for ML-corpus curation.
- **Separation SOTA kept moving inside the RoFormer family** (BS-RoFormer-SW community-ranked best 6-stem, Feb 2026) while the **checkpoint-license hygiene problem got worse** — leading community vocal checkpoints ship with no license file at all; MIT-clean htdemucs remains the conservative floor.

### License watch-items verified this pass
- **SwiftF0 is MIT** (closes an open question in `SOTA-SURVEY` §H).
- **emotion2vec's GitHub repo has no code LICENSE**; the weights ride the FunASR model license (commercial-OK + attribution + no-disparagement).
- **audEERING's dimensional models, including the new wav2small, stay CC-BY-NC-SA.**
- **DNS-Challenge/DNSMOS is CC-BY-4.0**; **LAION `larger_clap_general` weights are Apache-2.0**; **Vox-Profile is RAIL across code and models**; **Dasheng is Apache-2.0**; **CED is GPL-3.0**.
- **Surfboard was archived Jan 2025**; **OpenPhonemizer archived Mar 2026**.
- Unresolved and blocking nothing critical: TEN VAD's "additional conditions", the 2508.04814 code license, creapy, GFM-IAIF, and the flagship community Mel-Band-RoFormer vocal checkpoint (**license API returns null**).

---

*Merged from `enrich-A.md` (alignment/prosody/decomposition), `enrich-B.md` (voice-quality/nonverbals/laughter), `enrich-C.md` (affect/hygiene). Section I authored by the synthesis editor. Survey window 2026-08-11/12.*

Written to `/private/tmp/claude-501/-Users-vera-dev-sophia-mus/cabb4478-df39-45dc-8904-49117bfb6f3d/scratchpad/ENRICHMENT-SURVEY.md`