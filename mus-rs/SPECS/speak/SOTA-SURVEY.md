# SPEAK — State-of-the-Art Survey: Open Speech & Audio ML

**Survey date: 2026-08-11.** Prepared against `mus-rs/SPECS/speak/RESEARCH-BRIEF.md` for the Threader bet: extended Karplus-Strong control programs (~25-50 floats, ~hundreds of bps) as an interpretable, lawful speech/singing encoder, with Sobol render banks → amortized inverse network → per-utterance CMA-ES polish. Hardware frame: Apple Silicon 16-core + MPS, occasional spot boxes.

**Method.** Six parallel research passes (~170 web searches/fetches total) against GitHub repos (LICENSE files, commit activity), arXiv 2025-26, HuggingFace model cards, and live leaderboards, weighted recency > fame. Everything dated after January 2026 was verified by fetch this week unless explicitly marked otherwise. Confidence tags per recommendation: **tried-and-true** (verified primary source + community mileage), **reported** (verified from primary/secondary sources but not hands-on), **rumor** (single weak source or training knowledge, unverified).

---

## A. Open TTS for corpus generation

The landscape moved hard between January and August 2026: half the candidate list in the brief is superseded, and there is a **license-tightening trend among quality leaders** (Fish → research license, Spark-TTS retroactively Apache→CC-NC, Higgs added an output non-compete). Check *weights* licenses, not code licenses, every time.

| Model | Version / date | License (model / outputs) | Apple Silicon footprint | Quality reputation | For-our-use one-liner |
|---|---|---|---|---|---|
| **Qwen3-TTS** (Alibaba) | 0.6B + 1.7B, Jan 2026 (announced Jan 15, repo release Jan 22 — Apache-2.0 verified on [repo](https://github.com/QwenLM/Qwen3-TTS) this survey) | **Apache 2.0** / outputs unrestricted | 2.5-4.5 GB; MLX path via mlx-audio + [Qwen3-TTS-MLX](https://github.com/odiak/Qwen3-TTS-MLX) | SOTA zero-shot cloning on Seed-TTS-eval; avg WER 1.835% / SIM 0.789 across 10 langs ([tech report](https://arxiv.org/pdf/2601.15621)) | Best all-rounder: 3-s cloning **plus text-described voice design** → mint unlimited synthetic speaker identities; permissive; Mac-native |
| **Kokoro** | 82M v1.0 | **Apache 2.0** / unrestricted | Tiny (~330 MB); mlx-audio first-class; faster than real-time on CPU | Top-5 open Elo ([TTS Arena v2](https://tts-agi-tts-arena-v2.hf.space/leaderboard)) | Bulk workhorse for the clean high-volume stratum; ~54 preset voices, **no cloning**, limited prosody range |
| **Chatterbox** (Resemble) | Turbo 350M + Nano 110M (Dec 2025), Multilingual V3 500M / 23-25 langs (Jun 2026) | **MIT** / unrestricted — but **all output carries Perth neural watermark** | Small; [ONNX release](https://huggingface.co/ResembleAI/chatterbox-turbo-ONNX) runs CPU/MPS | Very strong; 1-step distilled decoder; exaggeration knob + paralinguistic tags ([cough],[laugh],[sigh]) | Best prosody-diversity dial; watermark is a systematic sub-audible artifact your inverse net would train on — assess deliberately (see §I) |
| **CosyVoice 3** (Alibaba) | Fun-CosyVoice3-0.5B-2512, Dec 15 2025 | **Apache 2.0** / unrestricted ([HF](https://huggingface.co/FunAudioLLM/Fun-CosyVoice3-0.5B-2512)) | 0.5B; no first-class MLX but small enough for MPS/CPU | Beats CosyVoice 2 on consistency/similarity/prosody | Cheap second cloning engine; 9 langs + 18 Chinese dialects for speaker-diversity coverage |
| **OmniVoice** (k2-fsa / Povey) | Mar 31 2026 | **Apache 2.0** / unrestricted | In mlx-audio; 40x RT claimed on GPU | 3.8k stars, 460k HF downloads in 3 weeks; **646 languages**, 581k h training ([repo](https://github.com/k2-fsa/OmniVoice), [arXiv](https://arxiv.org/html/2604.00688v1)) | Widest phonological coverage of anything open — for stress-testing "control programs are a general speech code" |
| **ZONOS2** (Zyphra) | Jun 12 2026 | **Apache 2.0** / unrestricted | MoE 900M active / 8B total — spot GPU, heavy for Mac ([repo](https://github.com/Zyphra/ZONOS2)) | First open MoE TTS; 6M h training; high-fidelity cloning | Expressive-speech tier when GPU hours are available; supersedes Zonos v0.1 |
| **Step-Audio-EditX** (StepFun) | 3B, Nov 2025 (upd. Jan 2026) | **Apache 2.0** ([repo](https://github.com/stepfun-ai/Step-Audio-EditX)) | 3B — marginal on Mac, fine on spot GPU | Top open-weights Elo ~1118 in [2026 roundups](https://offlinetts.com/blog/tts-arena-leaderboard-2026/) | Unique: **iterative emotion/style edits of existing audio** → controlled prosody-contrast pairs (same text/voice, different affect) for disentanglement training |
| **Dia2** (Nari Labs) | 1B + 2B, streaming ([repo](https://github.com/nari-labs/dia2)) | Apache 2.0 / unrestricted | 1-2B, workable on M-series | Ultra-realistic multi-speaker *dialogue* + nonverbals; EN-only | Conversational-register data (turn-taking, laughter) read-aloud TTS won't give you |
| Sesame CSM-1B | Mar 2025, no major open update | Apache 2.0 | 1B; in mlx-audio | Good conversational prosody | Superseded by Qwen3-TTS/Dia2 |
| Kitten TTS | v0.8: 15-80M ([repo](https://github.com/KittenML/KittenTTS)) | Apache 2.0 | ONNX, CPU-only, 25-80 MB | Good for size; dev-preview | Only as a deliberate "low-fidelity synthetic" stratum; Kokoro otherwise dominates |
| Marvis TTS | 250M v0.2 ([HF](https://huggingface.co/Marvis-AI/marvis-tts-250m-v0.2)) | Apache 2.0 | MLX-native, ~500 MB quantized | Decent; EN/FR/DE | Mac-native convenience, not a quality leader |
| NVIDIA Magpie-TTS Multilingual | 357M, v2602 Mar 2026 ([HF](https://huggingface.co/nvidia/magpie_tts_multilingual_357m)) | NVIDIA Open Model License (commercial OK) | NeMo = CUDA-centric; spot GPU not Mac | Top-5 open Elo (1065) | Solid licensed multilingual option if already on a CUDA box |
| **Fish Audio S2 / OpenAudio S1-mini** | S2 Feb 2026 (weights Mar 2026); S1-mini 0.5B | S2: **Fish Audio Research License (NC)**; S1-mini: **CC-BY-NC-SA-4.0** ([discussion](https://github.com/fishaudio/fish-speech/discussions/1001)) | S1-mini local-able; S2 4B | **#1 open-weights Elo (1122)**; word-level emotion tags, 80 langs | Best open quality, wrong license — NC taints the corpus. Avoid |
| IndexTTS-2.5 (Bilibili) | Aug 10 2026 | Custom Bilibili license, restricted/unclear outputs ([repo](https://github.com/index-tts/index-tts)) | 2B; vLLM | Excellent; **precise duration control** + emotion vectors | Scientifically interesting (duration control), legally murky. Watch, don't build on |
| Higgs Audio v2 / TTS-3 (Boson) | 3-4B ([HF](https://huggingface.co/bosonai/higgs-tts-3-4b)) | Community license with **explicit "no using outputs to improve other models" clause** | 3-4B; v3 in mlx-audio | Very strong expressive speech | The output clause targets exactly your use. Avoid |
| Voxtral TTS (Mistral) | 4.1B, Mar 26 2026 ([announcement](https://mistral.ai/news/voxtral-tts/)) | **CC BY-NC 4.0** | 4B; in mlx-audio | Near-ElevenLabs, 90 ms latency | Great model, NC weights — avoid for corpus |
| F5-TTS | v1.1.x, base weights unchanged | Code MIT; **weights CC-BY-NC-4.0** (Emilia-trained). Apache retrain: [OpenF5-TTS](https://huggingface.co/mrfakename/OpenF5-TTS-Base) | ~330M DiT; community MLX ports | 2025 darling, now mid-pack | Superseded; if you want flow-matching TTS use OpenF5 (Apache) |
| MegaTTS3 / Llasa / Spark-TTS / XTTS-v2 | 2023-25 | Crippled cloning encoder / CC-NC / relicensed Apache→CC-NC-SA ([HF](https://huggingface.co/SparkAudio/Spark-TTS-0.5B/blob/main/README.md)) / CPML | — | — | All avoid (license or staleness) |
| StyleTTS2 / Parler / Orpheus / Zonos-v0.1 | 2023-25, no 2026 movement found | MIT / Apache / Apache+Llama-3.2 weights / Apache | various | superseded | Historical; Orpheus weights carry Llama 3.2 Community License terms (see §I) |

**For our use** — mass paired-corpus generation with clean output-license chain:

1. **Qwen3-TTS 1.7B — primary engine.** Apache 2.0, 10 languages, SOTA cloning, and the killer feature: **description-based voice design** — programmatically sample voice descriptions (age × timbre × accent × affect) to mint thousands of distinct *synthetic* speakers, sidestepping cloning-consent questions entirely. MLX on the Mac; batch on spot GPU. *Confidence: reported.*
2. **Kokoro 82M — bulk/clean stratum.** Apache 2.0, faster than real-time on CPU; generate the high-volume "easy" stratum that anchors the inverse network before the diverse tail. *Confidence: tried-and-true.*
3. **Step-Audio-EditX — prosody-contrast generator.** Apache 2.0. Take any utterance and re-render it angry/whispered/faster — same text, same speaker, different prosody: precisely the paired data that teaches an encoder to disentangle prosody dimensions in control-program space. Spot GPU. *Confidence: reported.*
4. **CosyVoice3-0.5B and/or OmniVoice — diversity stratum.** Both Apache 2.0, both light. CosyVoice3 for accent/dialect breadth; OmniVoice for phonological coverage far beyond English (646 languages). *Confidence: reported.*

Honorable mention: **Chatterbox (MIT)** has the best expressiveness knob but every sample carries the Perth watermark — for a project whose thesis is *lawful, artifact-free encoding*, training on systematically watermarked audio deserves a deliberate decision (or a test whether ~hundreds-of-bps resynthesis strips it — it almost certainly does, which is itself a publishable finding). *Confidence: tried-and-true.*

Anti-recommendations despite quality: Fish S2 (NC research license), Voxtral (CC-NC), Higgs (output non-compete), IndexTTS-2.5 (custom license), Llasa/Spark/F5-base (NC weights).

---

## B. Open ASR / phoneme recognizers as intelligibility oracles

Framing: (a) word-level WER judges vs (b) frame-level phoneme-posterior oracles. The 2026 leaderboard top is now **LLM-decoder ASR** (Granite, ARK, MOSS, Qwen3-ASR, Canary-Qwen) — none expose frame-level CTC posteriors. The CTC world now lives in the phonetics community (ZIPA lineage) and NVIDIA's speed tier. That split matters for you: the loss function wants (b).

| Model | Version/date | License | Posterior access (CTC?) | Apple Silicon footprint / speed | Quality | For-our-use one-liner |
|---|---|---|---|---|---|---|
| **ZIPA-CR** ([repo](https://github.com/lingjzhu/zipa), [HF](https://huggingface.co/anyspeech/zipa-large-crctc-ns-800k)) | ACL 2025; active | **MIT** | **Yes — pure CTC over IPA phones**; PyTorch + official ONNX FP32/FP16/INT8 | 64M (small) / 300M (large); INT8 ONNX runs per-core on CPU | SOTA multilingual PER on IPAPack++ (17k h, >4000 langs); the 64M beats 300M+ finetuned baselines | **Best-fit cheap frame-level IPA posteriorgram oracle** — ONNX for the evo loop, torch for differentiable loss |
| **wav2vec2-xlsr-53-espeak-cv-ft** ([HF](https://huggingface.co/facebook/wav2vec2-xlsr-53-espeak-cv-ft)) | 2021; still 441K downloads/mo | **Apache 2.0** | **Yes — CTC over ~392 IPA tokens**; `model(x).logits` one-liner | ~315M (training knowledge); MPS via PyTorch; ONNX-exportable | Zero-shot cross-lingual PER strong for its era | Tried-and-true fallback phoneme oracle; differentiable; the safe default |
| **PhoneticXeus** ([repo](https://github.com/changelinglab/PhoneticXeus), [arXiv 2603.29042](https://arxiv.org/html/2603.29042v2)) | Interspeech 2026 (Mar 2026) | **Not stated in repo — flag** | Yes — self-conditioned CTC logits, documented | XEUS encoder ~575M; MPS untested | SOTA accented-English phone recognition; 70-100+ langs IPA | Accuracy-ceiling phoneme oracle for periodic re-scoring, if license clears |
| BranchShine ([arXiv 2606.22824](https://arxiv.org/pdf/2606.22824)) | Jun 2026 | Paper CC-BY; **weights unconfirmed** | CTC (RoPE E-Branchformer) | 33M — ideal if released | 9.19% IPA CER, beats 575M PhoneticXeus | Watch quarterly for weights |
| Allosaurus ([repo](https://github.com/xinjli/allosaurus)) | uni2005 (2020); aging | **GPL-3.0 (flag)** | Yes — phone posteriors, top-k, adjustable priors | ~11-13M; trivial CPU | Dated accuracy; 2000+ langs | Battle-tested ultra-cheap fallback; GPL + age argue for ZIPA |
| MMS-1b-all ([HF](https://huggingface.co/facebook/mms-1b-all)) | 2023 | **CC-BY-NC 4.0 (flag)** | Yes — wav2vec2 CTC (per-lang char vocab) | 1B — heavy for inner loop | 1100+ langs | Only for exotic-language coverage, non-commercial |
| **torchaudio CTC forced-alignment API + label priors** ([tutorial](https://docs.pytorch.org/audio/2.8/tutorials/ctc_forced_alignment_api_tutorial.html), [arXiv 2406.02560](https://arxiv.org/abs/2406.02560)) | torchaudio 2.8 | BSD-2 (API) | **This is the scoring glue**: align target phone string against any CTC emission matrix | Negligible on top of acoustic model | 12-40% better phone-boundary error than vanilla CTC alignment | Feed it ZIPA/wav2vec2 emissions + target phones → per-phone log-likelihood fitness |
| MFA 3.4.0 ([docs](https://montreal-forced-aligner.readthedocs.io/en/latest/installation.html), [2026 paper](https://arxiv.org/abs/2606.18466)) | v3.4.0, Jul 2026 | MIT | No posteriors (HMM-GMM) | CPU; too process-heavy for inner loop | <15 ms mean boundary error | Offline ground-truthing of phone boundaries, not an in-loop oracle |
| **Parakeet-TDT 0.6B v2/v3** ([HF](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3), [parakeet-mlx](https://github.com/senstella/parakeet-mlx)) | v3 Aug 2025 (25 langs) | **CC-BY-4.0** | Transducer (not clean frame-CTC); sibling **parakeet-tdt_ctc-110m** ([HF](https://huggingface.co/nvidia/parakeet-tdt_ctc-110m)) exposes a true CTC head (BPE) | 600M; **native MLX**, ~24x RT on M4 (RTF ≈0.042) | v3: 6.34% avg multilingual WER, top throughput | **Best speed/quality WER judge on a Mac**; the 110M hybrid is a bonus CTC word-oracle |
| Whisper large-v3-turbo (+ whisper.cpp / mlx-whisper) | Oct 2024; **no open OpenAI successor as of Aug 2026** | MIT | No — seq2seq; decoder token log-probs only | 809M; ~9x RT on M3 | ~3.7% WER clean EN; **hallucination-prone on non-speech/singing** | Boring-reliable WER judge; risky as sole oracle in an adversarial evo loop |
| distil-large-v3.5 ([HF](https://huggingface.co/distil-whisper/distil-large-v3.5)) | Mar 2025 | MIT | No | 756M, ~6x Whisper speed | Within ~1% of large-v3, EN-only | Faster Whisper-family judge |
| **Qwen3-ASR 0.6B/1.7B** ([repo](https://github.com/QwenLM/Qwen3-ASR)) | **Jan 29 2026** | **Apache 2.0** | No — LLM-decoder | 0.6B/1.7B; community GGUF + MLX conversions | 52 langs; **explicit speech/music/song support** | The **singing-aware** WER judge — rare explicit song-recognition support |
| **Granite Speech 4.1 2B** ([HF](https://huggingface.co/ibm-granite/granite-speech-4.1-2b)) | **Apr 30 2026** | **Apache 2.0** | No (speech-LLM) | 2B — batch re-eval, not per-probe | **5.33% avg WER — top-3 Open ASR Leaderboard mid-2026** | Strongest permissive WER arbiter for periodic tournament scoring |
| OWSM-CTC v4 1B ([HF](https://huggingface.co/espnet/owsm_ctc_v4_1B)) | Interspeech 2025 | CC-BY-4.0 | **Yes — encoder-only self-conditioned CTC** (BPE) + CTCSegmentation | 1B; espnet stack | 7.42 mean leaderboard WER | Fully-open CTC WER-oracle if you tolerate espnet deps |
| Moonshine v2 ([HF](https://huggingface.co/UsefulSensors/moonshine-tiny), [paper](https://arxiv.org/pdf/2602.12241)) | v2 Feb 2026 | MIT (EN) | No | 27M/61M ONNX; near-negligible | Beats Whisper-tiny class | Ultra-cheap sanity-check judge |
| 2026 leaderboard churn: Cohere Transcribe (Mar, 5.42%), ARK-ASR-3B/0.6B (Apache, May, [MLX](https://huggingface.co/leope/ark-asr-0.6B-mlx)), MOSS-Transcribe (Apache, Jul) ([roundup](https://www.marktechpost.com/2026/07/23/best-open-speech-recognition-asr-models-in-2026-wer-languages-latency-and-license-compared/)) | Mar-Jul 2026 | ARK/MOSS Apache 2.0 | No — all LLM-decoder | ARK-0.6B has MLX conversion | Top entries within <1 WER pt | Churning frontier; ARK-ASR-0.6B-MLX worth a look as a light 2026-gen judge |

**For our use:**

1. **Primary frame-level phoneme CTC oracle: ZIPA-CR-small (64M, MIT).** IPA-phone CTC posteriors; official INT8 ONNX for the evolutionary loop (one instance per core fits the 10k probes/hr/core budget); torch checkpoint for a differentiable posteriorgram loss. Pair with torchaudio's forced-alignment API (label-priors variant) to turn emissions + target phones into per-phone log-likelihood fitness. *Confidence: reported.*
2. **Cross-check oracle: wav2vec2-xlsr-53-espeak-cv-ft (Apache 2.0).** Five years of mileage, three-line logits API, MPS, differentiable. Two-oracle agreement is cheap insurance against the evolver posterior-hacking ZIPA's idiosyncrasies. *Confidence: tried-and-true.*
3. **Fast WER judge on the Mac: Parakeet-TDT-0.6B v2/v3 via parakeet-mlx (CC-BY-4.0)** — ~24x RT on M4-class; the hybrid `parakeet-tdt_ctc-110m` gives a 114M true-CTC word oracle from the same family. *Confidence: reported.*
4. **Heavy arbiters, batch-only: Granite Speech 4.1 2B (Apache, 5.33% WER)** nightly; plus **Qwen3-ASR-0.6B (Apache)** specifically for **singing** — the only permissive 2026 model advertising stable song recognition; Whisper-lineage judges hallucinate on singing. *Confidence: reported (singing-robustness is vendor-stated).*

**Loss-design caveat that matters:** CTC posteriors are **peaky** — raw framewise max-posterior scoring under-rewards near-misses. Use forced-alignment path scores with label priors ([arXiv 2406.02560](https://arxiv.org/abs/2406.02560)) or soft-DTW over the posteriorgram rather than greedy PER alone.

---

## C. Speech SSL encoders as representation baselines

| Name | Version/date | License | Footprint (Apple Silicon) | Reputation | For-our-use one-liner |
|---|---|---|---|---|---|
| **HuBERT** (Base/Large) | 2021, frozen | MIT (fairseq); HF mirrors Apache *(training knowledge)* | Base 95M ≈ 0.4 GB fp32; fine on MPS via `transformers` | Most-cited SSL baseline; its units still tokenize speech LMs | The cheap, universally-recognized control baseline (768-d @ 50 Hz) |
| **WavLM** (Base+/Large) | 2021-22, frozen | MIT via [microsoft/UniSpeech](https://github.com/microsoft/UniSpeech/blob/main/LICENSE) | Large 316M ≈ 1.3 GB fp32; MPS-friendly | Still cited in 2025-26 lit as the best open SUPERB performer | The strongest English probe-benchmark reference (1024-d @ 50 Hz) |
| **w2v-BERT 2.0** | Jan 2024 (Seamless) | **MIT** ([HF](https://huggingface.co/facebook/w2v-bert-2.0)) | 580M Conformer ≈ 1.2 GB fp16; MPS OK | Standard multilingual workhorse (4.5M h / 96 langs) | Best MIT multilingual baseline; one HF call |
| **XEUS** | Jul 2024 (CMU/ESPnet) | **Now CC-BY-NC-SA-4.0** on [HF main](https://huggingface.co/espnet/xeus) — earlier revisions said MIT; **license changed** | 577M E-Branchformer; ESPnet stack, less MPS-turnkey | Tops ML-SUPERB; 4,000+ langs | Paper baseline only; NC blocks product use |
| **NEST** (NVIDIA) | 2024-25 | CC-BY-4.0 ([HF](https://huggingface.co/nvidia/ssl_en_nest_xlarge_v1.0)) | 115M/600M; FastConformer 8x subsample (low frame rate → cheap probes) | Growing rep as the efficient English SSL encoder | Efficient low-frame-rate contrast point; commercial-friendly |
| **Omnilingual wav2vec 2.0** (Meta) | **Nov 2025** | **Apache-2.0** ([Meta blog](https://ai.meta.com/blog/omnilingual-asr-advancing-automatic-speech-recognition/)) | 300M variant fine; 7B = spot-box | The big post-2025 encoder release: 1,600+ langs | Freshest permissively-licensed encoder family (300M→7B) |
| Codec2Vec | Nov 2025 ([arXiv](https://arxiv.org/pdf/2511.16639)) | unknown | — | SSL over discrete codec units | Watch-item; field moving token-native |

**Current probing-benchmark norm** (what "transfer-per-dimension" claims get judged against):

- **SUPERB / s3prl is in pure maintenance mode** since 2024 ([repo](https://github.com/s3prl/s3prl); last release Sep 2024), but its **frozen weighted-sum-over-layers + lightweight-head protocol is still the 2026 norm** (e.g., [CSPB, EACL 2026](https://aclanthology.org/2026.eacl-long.275.pdf), [TS-SUPERB 2025](https://arxiv.org/html/2505.06660v1)). *(reported)*
- **ML-SUPERB 2.0** ([arXiv 2406.08641](https://arxiv.org/pdf/2406.08641)) — 142 languages with a **≤100M tunable-parameter constraint**: the closest existing thing to efficiency-normalized transfer. *(tried-and-true)*
- **DASB** ([arXiv 2406.14294](https://arxiv.org/pdf/2406.14294), in [speechbrain/benchmarks](https://github.com/speechbrain/benchmarks), active) — evaluates **discrete tokens directly, no waveform reconstruction**: the right frame for a bitrate-normalized claim. *(reported)*
- **X-ARES** ([arXiv 2505.16369](https://arxiv.org/abs/2505.16369), Interspeech 2025) — 22 tasks, **linear and parameter-free probes**; emerging cross-domain norm. *(reported)*
- **Probe-capacity caveat is now doctrine:** Zaiem et al. showed model *rankings flip* with probe head capacity ([arXiv 2308.14456](https://arxiv.org/pdf/2308.14456)) — report both linear and small-MLP heads. *(tried-and-true)*
- **No benchmark natively reports per-dimension-normalized transfer** — that claim frame is yours to define. The community-legible versions are (i) bits/sec (DASB, LRAC) and (ii) matched probe-parameter budgets (ML-SUPERB 2.0 style).

**For our use:**
1. **WavLM Large + HuBERT Base** as the mandatory pair — one SOTA-class, one cheap control; both MIT-lineage, both MPS-trivial. *(tried-and-true)*
2. **w2v-BERT 2.0** if any multilingual/singing generality claim is made. *(tried-and-true)*
3. **Omnilingual wav2vec 300M** (Apache) as the fresh multilingual point; XEUS only as a paper baseline (NC). *(reported)*
4. **Protocol:** s3prl-style frozen weighted-sum layer probing with both linear and small-MLP heads (citing Zaiem) on a compact battery (phone recognition, speaker ID, emotion, F0/prosody regression), **plus a DASB-style bitrate-normalized track where the 25-50-float control program competes as a "token stream" against codec tokens.** *(reported)*

---

## D. Neural codecs as rate-distortion baselines

Everything except X-Codec2 (0.8B) and diffusion decoders is 60-160M params and runs comfortably on an M-series laptop. Metrics below are clean read speech (LibriSpeech/LibriTTS test-clean or MLS) unless noted; "dWER" = WER of resynthesis.

| Codec | Version / date | Bitrate(s) | License / open weights | Quality numbers (metric@bitrate) | For-our-use one-liner |
|---|---|---|---|---|---|
| **EnCodec** (Meta) | 2022, stable; in `transformers` | 1.5 / 3 / 6 / 12 / 24 kbps | MIT, open weights ([repo](https://github.com/facebookresearch/encodec)) | Widely reproduced; at 1.5 kbps clearly below modern codecs | Boring-standard lower anchor every reviewer knows |
| **DAC** (Descript) | 2023, stable; in `transformers` | 6-8 kbps typical | MIT, open weights | Near-transparent at 8 kbps; degrades badly <2 kbps | Transparency ceiling for the top of the R-D curve |
| SpeechTokenizer | 2023 | 4 kbps | Apache-2.0 ([HF](https://huggingface.co/OpenMOSS-Team/SpeechTokenizer)) | Superseded | Skip — historical (semantic-distillation pioneer) |
| **Mimi** (Kyutai) | Sep 2024, still current Aug 2026 ([HF](https://huggingface.co/kyutai/mimi)) | **1.1 kbps** @ 12.5 Hz | **CC-BY-4.0**, 96M; PyTorch + **Rust + MLX** ([moshi](https://github.com/kyutai-labs/moshi)) | @1.1 kbps: SQMOS 4.333, PESQ 2.132, SECS 0.678, CER 7.2 (per [NanoCodec tbl](https://arxiv.org/html/2508.05835)) | De-facto speech-LM codec reference; native Apple Silicon (MLX + Rust — symmetric with your stack) |
| **WavTokenizer** | ICLR 2025 | 0.48 / 0.9 kbps | MIT ([repo](https://github.com/jishengpeng/WavTokenizer)) | UTMOS 4.05@0.9 kbps but **CER 15.3, SECS 0.576** | Cautionary baseline: high UTMOS, hallucinates content — exactly the failure your instrument avoids; cite it |
| BigCodec | 2024 | 1.04 kbps, 159M | Open weights ([repo](https://github.com/Aria-K-Alethia/BigCodec)) | UTMOS ~4.1 (training knowledge) | Superseded by FocalCodec/NanoCodec |
| SemantiCodec | 2024 | 0.31-1.40 kbps | **MIT**, **explicit MPS support** ([repo](https://github.com/haoheliu/SemantiCodec-inference)) | Diffusion decoder → slow | Optional second ~0.3 kbps point; slow decode |
| **Stable Codec / TAAE** (Stability) | v0.1.2 Jan 2025 | **0.4 / 0.7 / 1.0 kbps** | Code MIT; weights **Stability Community License (NC restrictions)** ([repo](https://github.com/Stability-AI/stable-codec)) | @700 bps: PESQ 3.09, STOI 0.92 ([ICLR 2025](https://arxiv.org/pdf/2411.19842)) | Best published PESQ in the 400-700 bps band; compare-against, not ship-with |
| **FocalCodec** (+ Stream) | v0.0.2 Sep 2025, NeurIPS 2025; [stream paper](https://arxiv.org/abs/2509.16195) | **0.16 / 0.33 / 0.65 kbps**; streaming 0.55-0.8 kbps @80 ms | **Apache-2.0**, HF ckpts, ONNX ([repo](https://github.com/lucadellalib/focalcodec)); [SpeechBrain recipe](https://github.com/speechbrain/speechbrain/tree/develop/recipes/LibriTTS/focalcodec) | @0.65: UTMOS ~4.05, dWER ~2.2; @0.16: UTMOS 4.22, dWER 7.94 (LibriTTS) ([paper](https://arxiv.org/abs/2502.04465)) | **The open SOTA in your exact 160-650 bps band. Primary rate-matched baseline.** |
| TS3-Codec (Microsoft) | Nov 2024, paper only | 0.85 kbps | **No public weights** ([paper](https://arxiv.org/pdf/2411.18803)) | @0.85: PESQ 2.39, CER 8.2 | Cite numbers only |
| **NVIDIA NanoCodec** | Aug 2025 | **0.6 / 0.8 / 1.1 / 1.78 kbps** @12.5 fps, 62M | **NVIDIA Open Model License** (commercial OK) ([HF](https://huggingface.co/nvidia/nemo-nano-codec-22khz-1.78kbps-12.5fps)) | @1.1 beats Mimi on everything (CER 3.6 vs 7.2, SECS 0.788 vs 0.678); @0.8: SQMOS 4.37 ([paper](https://arxiv.org/pdf/2508.05835)) | Quality-per-bit champion 0.8-1.8 kbps; NeMo awkward on macOS — third-party [ONNX export](https://huggingface.co/Knehm/nemo-nano-codec-22khz-1.78kbps-12.5fps-ONNX) exists. The 0.6 kbps variant is flagged "for fine-tuning, not general-purpose" |
| X-Codec2 (HKUST/LLaSA) | Transformers-native Jun 2026 ([HF](https://huggingface.co/HKUSTAudio/xcodec2)) | **0.8 kbps** (50 Hz FSQ) | **CC-BY-NC-4.0**, 0.8B | Standard TTS-LM tokenizer | "What speech LLMs actually use at 800 bps"; heavy, NC |
| **TaDiCodec** (Amphion) | NeurIPS 2025 ([repo](https://github.com/HeCheng0625/Diffusion-Speech-Tokenizer)) | **0.0875 kbps** (6.25 Hz) | **Apache-2.0**, open weights | Strong WER/SIM/UTMOS — **but decoder is text-conditioned**: transcript is uncounted side information ([paper](https://arxiv.org/abs/2508.16790)) | The only open ~0.1 kbps point — with an asterisk your paper should exploit: your codes need no transcript |
| 2026 paper wave (no weights) | Jan-Jun 2026 | 0.25-1.1 kbps | paper-only (*rumor tier until code*) | [P2PSynCodec](https://arxiv.org/abs/2606.05876) 0.5 kbps ≈ rivals@2 kbps; [FMelCodec](https://arxiv.org/html/2605.25669) 250 bps; [VoCodec](https://arxiv.org/abs/2606.05892) 1.1 kbps voicing-driven; [ECC](https://arxiv.org/pdf/2606.11631) entropy-constrained w/ **BD-rate**; [U-Codec](https://arxiv.org/abs/2510.16718) 5 Hz; [PhoenixCodec](https://arxiv.org/pdf/2510.21196) (ICASSP 2026 low-resource challenge) | Cite as frontier context; none runnable yet |

**For our use — baseline set (four codecs spanning the R-D curve):**

1. **FocalCodec (0.16 / 0.33 / 0.65 kbps)** — *the* rate-matched competitor at your operating point. Apache-2.0, plain PyTorch (MPS/CPU), ONNX, three bitrates from one family = a clean competitor R-D curve. *Confidence: reported (re-measure their numbers yourself).*
2. **Mimi (1.1 kbps)** — the field's shared reference; MLX + Rust implementations make it easiest on Apple Silicon and pleasingly symmetric with the Rust instrument. *Confidence: tried-and-true.*
3. **EnCodec (1.5/3/6 kbps) + DAC (8 kbps)** — the boring-standard anchors reviewers demand; MIT. *Confidence: tried-and-true.*
4. **NanoCodec (0.8/1.1/1.78 kbps)** — mid-2026 quality-per-bit SOTA with commercially-open weights; its comparison table is the de-facto sub-2 kbps league table — lift its protocol. *Confidence: reported.*

Optional fifth points: Stable Codec @400/700 bps (best in-band PESQ; NC weights) and TaDiCodec @87.5 bps (only open ~0.1 kbps point; flag its text side-channel — your control programs need no transcript, a genuine differentiator worth a paragraph).

**Eval protocol to adopt (mid-2026 norms):**
- **WER/CER of resynthesis** with a fixed, named ASR — the anti-hallucination headline metric (WavTokenizer's 15% CER is the cautionary counterexample). Per §H, use **two ASR families**.
- **Speaker similarity (SECS/SIM)** — cosine of speaker-verification embeddings (WavLM-large/ECAPA-based).
- **No-reference MOS: UTMOS** (what every codec paper reports) + **[UTMOSv2](https://github.com/sarulab-speech/UTMOSv2)** (adopted by [ICASSP 2026 URGENT](https://github.com/urgent-challenge/urgent2026_challenge_track2)).
- **Full-reference: PESQ-WB, STOI/ESTOI, ViSQOL v3, mel/STFT distance** — report them, but argue the case on UTMOS/WER/SIM: generative resynthesis (and your instrument) legitimately scores low on waveform-matching metrics while sounding fine.
- **BD-rate over PESQ/ViSQOL curves** — the [ECC paper (Jun 2026)](https://arxiv.org/pdf/2606.11631) shows the field now frames codec comparisons as BD-rate; it is the rate-distortion-native headline number and legitimizes exactly the comparison you want to run.
- **MUSHRA** (ITU-R BS.1534, hidden reference + low anchor) — small test for the camera-ready.
- **Harnesses:** **[VERSA](https://github.com/wavlab-speech/versa)** (90+ metrics behind one YAML — use as the runner), **[Codec-SUPERB](https://github.com/voidful/Codec-SUPERB)** ([leaderboard](https://eric-lam.com/Codec-SUPERB/), submit via GitHub issue — cheap external validation), **[DASB](https://github.com/speechbrain/benchmarks/tree/main/benchmarks/DASB)** (only if you claim your tokens feed LMs).

**Bottom line:** the honest sub-1 kbps SOTA with runnable open weights is FocalCodec (0.16-0.65 kbps) and NanoCodec (0.8-1.78 kbps), with Mimi@1.1 kbps as the shared reference and TaDiCodec@87.5 bps the only (asterisked, text-conditioned) point near 0.1 kbps. A ~200-500 bps interpretable, deterministic, non-hallucinating encoder slots into a genuinely contested band.

---

## E. Articulatory synthesis and inversion

**Bottom line up front:** "articulatory features as speech codes" is now firmly staked by Berkeley's **SPARC** (IEEE JSTSP, Dec 2024) at 14 continuous channels × 50 Hz, and pushed to **6 dimensions** by USC's **ARTI-6** (ICASSP 2026). But **nobody has quantized articulatory codes into an explicit bits-per-second budget with rate-distortion numbers, nobody decodes deterministically (all use neural vocoders), and nobody claims sparse editable programs or singing coverage.** That is the open lane.

### The "articulatory codec" thread — the claims-space to position against

| System / paper | Date | What it is | Code/weights + license | Claimed numbers | Relevance to our bet |
|---|---|---|---|---|---|
| **SPARC / "Articulatory Encodec"** — *Coding Speech through Vocal Tract Kinematics*, Cho et al. (Berkeley) — [arXiv 2406.12998](https://arxiv.org/abs/2406.12998), [repo](https://github.com/Berkeley-Speech-Group/Speech-Articulatory-Coding) | Jun 2024 → IEEE JSTSP 18(8), Dec 2024 | Audio→(12-ch EMA + pitch + loudness @ 50 Hz + 64-d speaker emb) via WavLM-based inversion; HiFi-GAN(FiLM) decode. "First universal articulatory inference and synthesis" | `pip install speech-articulatory-coding`; en/multi/en+ weights. **No LICENSE file** (verified via GitHub API, Aug 2026); last push May 2025; training code never released | Resynth WER 5.43% LibriTTS-R (GT 4.22, Whisper-lv3); MOS 3.82 vs GT 4.01; UTMOS 4.12 (≈GT); AAI corr 0.878 MNGU0; zero-shot VC 94.6% spk-ID / WER 4.83%. **No bits/s claim** — unquantized floats (~700 floats/s) | **The paper to position against.** Claims interpretable/controllable/universal — but not *low-bitrate*, not quantized, not deterministic; speaker code is a black-box embedding |
| **ARTI-6** — Lee et al. (USC SAIL) — [arXiv 2509.21447](https://arxiv.org/abs/2509.21447), [code](https://github.com/lee-jhwn/arti-6) | Sep 2025, rev. Jan 2026; **ICASSP 2026** | **6-dim** articulatory encoding from rtMRI (lips, tongue tip/body/root, velum, larynx) + inversion + synthesis | Code + samples public (license unchecked) | Inversion corr 0.87; "intelligible, natural" from 6 dims; no WER/MOS in abstract | The dimensionality-floor claim: cite as the current minimum-dimension articulatory code |
| **RT-VC** — Liu et al. — [arXiv 2506.10289](https://arxiv.org/abs/2506.10289), [ACL 2025 demo](https://aclanthology.org/2025.acl-demo.37.pdf) | Jun 2025 | Real-time zero-shot VC through SPARC space + DDSP vocoder | Demo system | **61.4 ms CPU latency**; quality "comparable to SOTA" | Proves articulatory codes run real-time on CPU — a bar your Rust engine can beat |
| **DDSP articulatory vocoder** — [arXiv 2409.02451](https://arxiv.org/abs/2409.02451), SLT 2024 | Sep 2024 | EMA+F0+loudness → speech via differentiable DSP | Berkeley ecosystem | **0.4M params**, 4.9x faster CPU, WER 6.67%, MOS 3.74 | Closest neural analog to a "small deterministic decoder"; obvious efficiency baseline |
| **Teaching Machines to Speak Using Articulatory Control** — [arXiv 2510.05619](https://arxiv.org/abs/2510.05619) | Oct 2025 | PPO policy drives SPARC articulatory space; speech as motor control | Not stated | Similarity >0.85 on 6 syllables | Berkeley moving toward *control programs* over articulatory space — watch closely |
| **Sylber** — Cho et al., ICLR 2025 | 2025 | Syllabic embeddings, **4.27 tokens/s** | — | Ultra-low token rate, not interpretable | The non-articulatory ultra-low-rate contrast point |
| **MirrorNet** — Siriwardena et al. (UMD) — [arXiv 2210.16454](https://arxiv.org/abs/2210.16454) | 2022-23 | Sensorimotor autoencoder learning 6 tract variables to control a synthesizer | Academic code | Corr ≈ supervised inversion | Conceptual ancestor of "learn synth controls by self-supervision"; **no 2025-26 continuation found** |
| EMG→articulatory / emg2speech — [arXiv 2505.13814](https://arxiv.org/pdf/2505.13814), [2510.23969](https://arxiv.org/pdf/2510.23969) | 2025 | Surface EMG → articulatory features → speech | varies | r ≈ 0.9 EMG→articulatory | Articulatory space as silent-speech interface — an application a lawful codec could serve |

### AAI (acoustic-to-articulatory inversion), 2023-26

- **Universal-kinematics basis:** WavLM linearly encodes EMA (best r=0.81; cross-speaker ≈ one affine transform) — [arXiv 2310.10788](https://arxiv.org/pdf/2310.10788) (ICASSP 2024). The theoretical ground of SPARC's universality claim.
- Speaker-independent AAI SOTA ~0.784 corr ([arXiv 2406.17329](https://arxiv.org/html/2406.17329)); interspeaker-consistency training ([Interspeech 2025](https://www.isca-archive.org/interspeech_2025/mcghee25_interspeech.html)); +nasality ([arXiv 2506.09231](https://arxiv.org/pdf/2506.09231)); survey ([arXiv 2504.13308](https://arxiv.org/abs/2504.13308)). Datasets: MNGU0, MOCHA-TIMIT, HPRC, XRMB.
- **2026:** complete-tract inversion from rtMRI, RMSE 1.48 mm ([arXiv 2603.28723](https://arxiv.org/abs/2603.28723), Mar 2026); **first cross-lingual AAI benchmark** (FROST-EMA, [arXiv 2606.20478](https://arxiv.org/pdf/2606.20478), Jun 2026): language mismatch degrades inversion 2x more than speaker mismatch — a crack in "universal inversion" claims worth citing; phoneme-recognition-through-articulation proposed as the eval metric for articulatory synthesis ([arXiv 2605.20920](https://arxiv.org/abs/2605.20920), EUSIPCO 2026).

### VocalTractLab lineage (physics-based)

| Item | Status | License / activity |
|---|---|---|
| **VTL 2.4** ([site](https://www.vocaltractlab.de/index.php?page=vocaltractlab-download), [manual](https://vocaltractlab.de/download-vocaltractlab/VTL2.4-manual.pdf)) | **Released Dec 5, 2025** — first mainline release since 2.3 (2020); GUI + API + source | **GPL**; Birkholz group active (speaker adaptation from MRI 2026, prosody prediction 2025) |
| VocalTractLab-Python ([repo](https://github.com/paul-krug/VocalTractLab-Python)) | `pip install vocaltractlab`; alpha-stage | **GPL-3.0** |
| **PAULE / articubench** ([repo](https://github.com/quantling/articubench), [ISSP 2024](https://www.isca-archive.org/issp_2024/sering24_issp.html)) | ML control model for VTL + **the only articulatory-synthesis benchmark** | Open (quantling) |
| Copy synthesis via CRNN — Gao & Birkholz ([TASLP 2024](https://ieeexplore.ieee.org/document/10458336/)) | Audio → VTL gesture scores by neural inversion | — |
| FEM/aeroacoustic ([FEMVoQ](https://www.isca-archive.org/iberspeech_2024/guasch24_iberspeech.pdf)) | 3D FEM VCV sequences | Physics-first; nowhere near continuous intelligible speech |

### Pink Trombone lineage

- Original (Thapen 2017); MIT modularization [pink-trombone-mod](https://github.com/chdh/pink-trombone-mod); **Rust ports exist** ([crates.io `pink-trombone`](https://crates.io/crates/pink-trombone)) — all hobby-grade.
- **Differentiable Pink Trombone:** *Vocal Tract Area Estimation by Gradient Descent* ([arXiv 2307.04702](https://arxiv.org/abs/2307.04702), DAFx 2023).
- **Cámara et al., [EURASIP JASMP 2025](https://link.springer.com/article/10.1186/s13636-025-00414-5):** genetic optimizer + grey-box fitting of Pink Trombone params to human non-speech vocalizations — methodologically the closest published thing to your render-bank → inverse-net → evolutionary-polish loop, but non-speech.
- **VocalTrax** (MIT Media Lab, [project](https://vocaltrax.media.mit.edu), [code](https://github.com/PapayaResearch/vocaltrax), NeurIPS AI-audio workshop 2024): JAX-accelerated Pink Trombone, optimization-only inversion — admits "persistent challenges matching natural voice quality," i.e., **optimization-only inversion of a physical model still doesn't reach intelligibility parity.**
- **PinkVocalTransformer** ([SPECOM 2025](https://link.springer.com/chapter/10.1007/978-3-032-07959-6_12)): transformer AAI onto 44-dim Pink Trombone tract diameters, trained on **4M+ synthetic audio-parameter pairs** — **your amortized-inversion-from-synthetic-render-banks pattern, published** — but vowels-only, static configs.
- **Differentiable articulatory copy-synthesis of biphonic singing** ([arXiv 2606.04943](https://arxiv.org/abs/2606.04943), **DAFx 2026**, Jun 2026): differentiable Kelly-Lochbaum waveguide + second source fitting Tuvan sygyt; 30-38% LSD reduction — differentiable physical copy-synthesis reached singing.

**For our use:**
- **SPARC** — *the* comparator for inversion quality, resynthesis WER/UTMOS, and the interpretability claim. ⚠ Repo has **no license file** — fine as benchmark, risky as dependency. *(tried-and-true: repo state verified)*
- **ARTI-6** — the low-dimensionality comparator bracketing your control-program dimensionality from below. *(reported)*
- **DDSP articulatory vocoder / RT-VC** — the efficiency bar (0.4M params, 61.4 ms CPU); measure against WER 6.67 / MOS 3.74. *(reported)*
- **VTL 2.4 + VocalTractLab-Python** — physics-based reference synth (GPL — keep out of the Rust core, drive via subprocess); **articubench** as the only existing articulatory-synthesis benchmark harness. *(tried-and-true lineage)*
- **VocalTrax + PinkVocalTransformer + Cámara 2025** — the exact prior art for "physical model + amortized inverse + optimizer polish"; cite all three and beat them on intelligibility (they stall at vowels/non-speech). *(reported)*

**Explicit positioning answer:** SPARC claims interpretable + low-dimensional + universal — **never low-bitrate**: no quantization, no bits/s, no rate-distortion curve exists anywhere in the articulatory literature found (multiple search angles, Aug 2026). **Unclaimed gaps you can own:** (a) an actual bitrate number (~hundreds of bps) for an interpretable physical-control code with WER/UTMOS at that budget; (b) **deterministic, bit-exact decode** — every articulatory system decodes through HiFi-GAN/DDSP neural vocoders; (c) **sparse editable programs** (25-50 floats/utterance) vs their dense ~700 floats/s trajectories; (d) **singing + speech in one code**; (e) a **non-anthropomorphic physical model** as carrier — nobody has claimed speech codes through *any* non-vocal physical model; (f) speaker identity as interpretable physical parameters rather than a 64-d neural embedding.

---

## F. Analysis-by-synthesis / sound matching

| Name | Version/date | License | Footprint | Reputation | For-our-use one-liner |
|---|---|---|---|---|---|
| **diffKS_torchLPC** ([repo](https://github.com/ptablasdpaula/diffKS_torchLPC)) | WASPAA 2025 | **Not stated in repo** — ask author | PyTorch + torchLPC, small | New, tiny (1 star) but paper-backed | ***The* direct prior art: differentiable Karplus-Strong, with a gradient-vs-genetic comparison included** |
| **"Four Decades of Digital Waveguides"** — Tablas de Paula, **J.O. Smith III**, Välimäki, Reiss ([arXiv 2604.12878](https://arxiv.org/pdf/2604.12878)) | **JAES 74(7/8), 2026** | CC (arXiv draft) | survey | Field-canonizing review incl. differentiable DWG + classical/evolutionary/neural parameter optimization | Your citation anchor — legitimizes exactly your problem statement |
| **DMSP** ([arXiv 2407.05516](https://arxiv.org/abs/2407.05516)) | NeurIPS 2024 | code available (license unverified) | PyTorch | Nonlinear planar string | Differentiable modal alternative to your waveguide network |
| DiffSound ([arXiv 2409.13486](https://arxiv.org/abs/2409.13486)) | 2024 | acad. code | FEM-heavy | Modal inverse rendering | Full physical inverse-rendering is viable but expensive |
| magenta/ddsp | Last commit Jul 2026 (infra only) | Apache-2.0 | TensorFlow (wrong stack) | Caretaker mode | Historical reference; field moved to PyTorch/JAX ([dasp-pytorch](https://github.com/csteinmetz1/dasp-pytorch); [Hayes et al. review](https://arxiv.org/pdf/2308.15422)) |
| **torchsynth / SynthAX** ([torchsynth](https://github.com/torchsynth/torchsynth), [SynthAX](https://www.media.mit.edu/publications/synthax/)) | 2021 / AES 2023 | Apache-2.0 | 16,200x / 80,000x realtime | Standard for massive synth render banks | Precedent + tooling pattern for your Sobol render banks |
| **DiffMoog** ([arXiv 2401.12570](https://arxiv.org/pdf/2401.12570)) | Jan 2024 | open-source | PyTorch modular synth | The reference open sound-matching testbed | Ready-made harness design for amortized-inverse + polish experiments |
| **Instrumental** ([arXiv 2603.15905](https://arxiv.org/html/2603.15905)) | **Mar 2026** | CC-BY paper; code n/a | 28-param differentiable subtractive synth + CMA-ES | Single-author, new | Fresh evidence **CMA-ES beats gradient descent** on non-convex synth landscapes; a *mismatched* pretrained neural inverse lands 138-322x worse than CMA-ES |
| SynthRL ([IJCAI 2025](https://www.ijcai.org/proceedings/2025/1129.pdf)) | 2025 | — | RL | Peer-reviewed | Third pole in the evolutionary-vs-neural-vs-RL inversion triangle; RL ≈ CMA-ES quality at lower per-query cost after ~30 min training |
| **SCRAPL** ([arXiv 2602.11145](https://arxiv.org/pdf/2602.11145)) | **Feb 2026** | — | scattering-transform loss w/ random path sampling | New | Perceptual loss built specifically for sound-matching gradients — candidate inverse-net objective |
| **STCTS** ([arXiv 2512.00451](https://arxiv.org/pdf/2512.00451)) | Dec 2025 | code public, CC-BY paper | TTS-based decode | New | **80 bps** text+prosody+timbre codec, NISQA >4.26 — the semantic-decomposition competitor at your bitrate; content passes through an ASR/TTS bottleneck (non-deterministic decode) |

**Parameter-estimation lineage:** InverSynth ([2018](https://arxiv.org/pdf/1812.06349)) → Sound2Synth ([IJCAI 2022](https://github.com/Sound2Synth/Sound2Synth)) → Masuda/Saito ([ISMIR 2021](https://archives.ismir.net/ismir2021/paper/000053.pdf), [TASLP 2023](https://ieeexplore.ieee.org/document/10017350/) — nothing newer from them) → DiffMoog (2024) → [AST sound matching, DAFx24 (Native Instruments)](https://www.dafx.de/paper-archive/2024/papers/DAFx24_paper_95.pdf) → SynthRL (2025) → Instrumental (2026). **No "synthesizer sound matching challenge" exists** — the [DAFx26 challenge](https://dafx26.mit.edu/challenge/) is reverb-IR modeling; that slot remains open.

**Evolutionary vs neural inversion — 2025-26 evidence favors your hybrid:** Instrumental (Mar 2026) shows CMA-ES > gradient descent on a 28-param non-convex landscape, and that amortized inverses only pay off in-distribution — exactly why the amortized-net + CMA-ES-polish split is the right architecture. diffKS itself benchmarks gradient vs genetic. *(reported)*

**KEY QUESTION — physical-model control programs as speech/audio codes: nobody found.** Multiple search phrasings surfaced no one using *instrument* physical-model control programs as compression codes. The occupied neighbors: **SPARC** (physiological, continuous, not rate-optimized), **STCTS** (80 bps semantic decomposition, generative non-deterministic decode), neural-codec floor-pushers ([0.275 kbps generative compression, Feb 2026](https://arxiv.org/abs/2602.00648) — 1.8B params, opaque; FocalCodec; the **[LRAC Challenge](https://blog.webex.com/engineering/lrac-challenge-2025-pushing-the-limits-of-speech-coding/)** with its ICASSP 2026 satellite workshop), and post-hoc interpretability bolted onto neural codecs ([arXiv 2506.04492](https://arxiv.org/pdf/2506.04492)). Classical LPC/MELP at 600-2400 bps is the historical precedent for model-parameter transmission. **Your slot — "instrument-physics control programs as an interpretable ~hundreds-bps code with exact deterministic decode" — sits in an empty cell** between SPARC, STCTS, and LRAC codecs. Your risk, equally clear: speaker timbre/intelligibility through a string network has no precedent; nothing above suggests it is easy.

**For our use:**
1. **diffKS_torchLPC** — adopt as direct baseline and closest prior art; contact the author about the missing license (he is also first author of the JAES waveguide review — the person to know). *(reported)*
2. **DiffMoog + Instrumental protocol** — reuse their harness/loss design (mel-STFT + centroid + MFCC composite) for benchmarking the CMA-ES polish stage. *(reported)*
3. **SPARC + STCTS** — the two interpretable-codec baselines your compression claims must beat or bracket; both have public code. *(reported)*
4. **SCRAPL or JTFS-based perceptual losses** for the inverse-net objective; **torchsynth/SynthAX** as the render-bank engineering pattern. *(reported)*

---

## G. Open singing synthesis (SVS/SVC)

The central constraint of this domain: **essentially all major open singing corpora are non-commercial** (OpenCpop NC, M4Singer CC-BY-NC-SA, GTSinger CC-BY-NC-SA) — so most pretrained SVS weights carry NC taint even when the code is MIT/Apache.

| Name | Version/date | License | Footprint/maintenance | Reputation | For-our-use one-liner |
|---|---|---|---|---|---|
| **OpenVPI DiffSinger** ([repo](https://github.com/openvpi/DiffSinger)) | v2.3.0, 2026-01-30 (new voicing/tension params) | **Apache-2.0** code; voicebanks community-licensed per-voicebank (many NC) | Very active; de-facto hobbyist/pro SVS ecosystem (ONNX → OpenUtau) | Dominant open SVS stack | Best "train your own singer from clean data" pipeline; per-frame parametric control (pitch/energy/breathiness/tension) maps directly onto your interpretable-control worldview |
| **NNSVS** ([repo](https://github.com/nnsvs/nnsvs)) | Alive, slow-moving | **MIT** (code + recipes) | Maintained (Yamamoto et al.); CPU-friendly small models | Respected research toolkit | **Cleanest license story in all of SVS**: MIT everything, you own the voicebank end-to-end |
| **OpenUtau** ([repo](https://github.com/stakira/OpenUtau)) | Active (wiki upd. Jun 2026) | MIT *(training knowledge)* | Active editor front-end | Standard open singing editor | Score→phoneme→timing front-end for generating controlled singing corpora at scale |
| **SoulX-Singer** ([repo](https://github.com/Soul-AILab/SoulX-Singer), [HF](https://huggingface.co/Soul-AILab/SoulX-Singer)) | Feb 6 2026 (SVC variant Mar 16 2026) | **Apache-2.0 code AND weights — verified on repo this survey** | New; DiT, 42k+ h Mandarin/English/Cantonese; transcription-free SVC variant | Headline zero-shot SVS release of 2026 | The big win: Apache zero-shot SVS with **F0-contour and MIDI-score conditioning** — the license-clean singing corpus generator |
| Vevo2 (Amphion) ([repo](https://github.com/open-mmlab/Amphion/tree/main/models/svc/vevo2), [paper](https://arxiv.org/abs/2508.16332)) | 2026-03-25 in Amphion | Code MIT; **weights CC-BY-NC-ND-4.0** | Active | Strong academic credibility; unified TTS+SVS+VC+SVC | Great research baseline for unified singing control; **ND weights = do not feed outputs into your release** |
| GPT-SoVITS ([repo](https://github.com/RVC-Boss/GPT-SoVITS)) | v4 / v2Pro | MIT code; pretrained-weight terms undocumented | Very active, 59k+ stars; explicit macOS MPS/CPU inference | Go-to few-shot cloning | Speech-first, not score-conditioned; weight provenance undocumented → treat outputs as taint-risky |
| so-vits-svc | Frozen ~2023 | AGPL-3.0 *(training knowledge)* | **Unmaintained** — maintainers declared the era over | Historic SVC standard | Skip; superseded |
| RVC → **Applio** ([repo](https://github.com/IAHispano/Applio)) | Applio active 2026 | Applio **MIT** | Living branch of the RVC family | Practical SVC daily-driver | Best maintained SVC baseline for timbre-transfer comparisons |
| TechSinger ([repo](https://github.com/gwx314/TechSinger)) | AAAI 2025 | License unconfirmed; GTSinger-trained → NC taint likely | Research code, low activity | Technique-controllable (breathy/falsetto/vibrato) via flow matching | Interesting control-dimension baseline (technique axes ≈ your control-program axes); research-only |
| VISinger2 ([repo](https://github.com/zhangyongmao/VISinger2)) | 2022-23 | Apache-2.0 *(training knowledge)* | Dormant but complete | Classic E2E SVS + **DSP-synthesizer hybrid** | Conceptually closest to you (neural + DSP synth); good citation baseline, dated quality |
| "StableSinger" | — | — | — | **Did not surface in any 2026 search** | Treat as nonexistent under that name |

**Datasets:** [GTSinger](https://github.com/AaronZ345/GTSinger) (80.6 h, 9 langs, 6 techniques, phoneme alignments + scores; CC-BY-NC-SA) · [OpenCpop](https://xinshengwang.github.io/opencpop/liscense/) (5.2 h Mandarin; NC, commercial by contact) · [M4Singer](https://openreview.net/forum?id=qiDmAaG6mP) (CC-BY-NC-SA). **SVC SOTA marker:** [SVCC 2025 analysis](https://arxiv.org/abs/2509.15629) — singer-*identity* conversion at ground-truth parity for 5 of 33 systems; singing-*style* conversion and naturalness still below human.

**For our use:**
- **(a) Corpus generation:** **SoulX-Singer** (Apache code+weights verified; zero-shot, score- and F0-conditioned; *reported* — new model, no hands-on mileage yet) + **self-trained NNSVS or OpenVPI DiffSinger voicebank** on data you record or license CC-BY (*tried-and-true*, fully taint-free), with **OpenUtau** as the score/phoneme front-end.
- **(b) Baselines:** **OpenVPI DiffSinger** (community quality bar; *tried-and-true*), **VISinger2** (the DSP-hybrid conceptual cousin; *tried-and-true*, dated), **Vevo2** (2026 research SOTA, eval-only; *reported*), **Applio** for SVC timbre comparisons (*reported*).
- Apple Silicon: DiffSinger/NNSVS/GPT-SoVITS run PyTorch-MPS or CPU inference fine; SoulX-Singer is DiT-scale — expect slow-but-feasible on a 16-core M-series (*unverified*).

---

## H. Eval instrumentation

| Name | Version/date | License | Footprint/maintenance | Reputation | For-our-use one-liner |
|---|---|---|---|---|---|
| **pystoi** ([repo](https://github.com/mpariente/pystoi)) | 0.4.1 | **MIT** | Stable; ~514k downloads/mo | Reference STOI/ESTOI implementation | STOI + ESTOI (`extended=True`); pure NumPy, trivially Apple-Silicon-safe |
| Whisper large-v3 as WER-of-synthesis norm | Still the 2026 norm (Seed-TTS-Eval, CV3-Eval etc.) | MIT | mlx-whisper / whisper.cpp on MPS | Standard | ~2.7% WER LibriSpeech-clean floor; see confound caveat below |
| **Dual-ASR WER caution** ([arXiv 2607.08256](https://arxiv.org/pdf/2607.08256)) | Jul 2026 | — | — | New methodological finding | WER-of-synthesis is confounded by ASR-family alignment → **report WER from two ASR families** (Whisper + Parakeet/Canary) |
| **UTMOSv2** ([repo](https://github.com/sarulab-speech/UTMOSv2)) | VoiceMOS 2024 winner | **MIT** | pip-from-git; Python 3.9-3.13 | SOTA-adjacent naturalness MOS | Heavier (wav2vec2 + EfficientNetV2 ensemble); secondary MOS |
| **Distill-MOS** ([repo](https://github.com/microsoft/Distill-MOS)) | ICASSP 2025 | **MIT (code + weights)** | `pip install distillmos`; compact distilled XLS-R, CPU-friendly | Microsoft-backed | **Primary MOS predictor**: small, fast, MIT, happy on CPU/MPS |
| NISQA ([repo](https://github.com/gabrielmittag/NISQA)) | v2.x | Code MIT; **weights CC-BY-NC-SA** | Semi-maintained | Classic multidimensional predictor | Internal eval only; don't bundle weights in your release |
| torchaudio SQUIM | torchaudio 2.10, Jan 2026 | BSD-2 | **torchaudio now in maintenance phase** ([issue #3902](https://github.com/pytorch/audio/issues/3902)) | Convenient no-reference PESQ/STOI/SI-SDR estimates | Usable today; plan for sunset |
| **VERSA** ([repo](https://github.com/wavlab-speech/versa), [NAACL 2025 demo](https://aclanthology.org/2025.naacl-demo.19/)) | Active; May 2026 metric-registry refactor | Apache-2.0 *(training knowledge)* | CMU WAVLab; 65 metrics / 729 variants incl. singing-adjacent | The 2025-26 "one harness to run them all" | Best single wrapper: STOI+MOS+WER+F0 behind one config |
| **Parselmouth** (Praat) ([repo](https://github.com/YannickJadoul/Parselmouth)) | 0.4.7 (Nov 2025); 0.5.0.dev (Jun 2026) | **GPL-3.0** (inherits Praat) | Actively developed | Formant/jitter/shimmer gold standard | Bit-exact Praat formant tracks from Python; GPL fine for an eval harness you don't link into the Rust release |
| penn ([repo](https://github.com/interactiveaudiolab/penn)) | FCNF0++ line | MIT *(training knowledge)* | pip-installable | Solid neural pitch+periodicity | F0/periodicity tracks for singing eval |
| SwiftF0 ([paper](https://arxiv.org/pdf/2508.18440)) | Aug 2025 | Unknown | New | Fast monophonic pitch | Benchmark against penn/Praat for on-device speed (*reported*) |

**Recommended metric stack (concrete):**
1. **Intelligibility:** ESTOI via pystoi (reference-based resynthesis tests) + **WER-of-synthesis with two ASR families** — mlx-whisper large-v3 primary, Parakeet (ONNX/MLX) as the non-Whisper control, per the July 2026 confound finding. *(tried-and-true + reported)*
2. **MOS proxy:** **Distill-MOS primary** (MIT weights, CPU-fast — suits deterministic CI), **UTMOSv2 secondary**. Flag explicitly: all are trained on *speech* MOS — on singing they are out-of-domain (SVCC 2025 still used human listening tests); treat MOS-predictor scores on singing as relative, not absolute. A "SingMOS" predictor exists *(rumor)*.
3. **Acoustic ground truth:** **Parselmouth** (F1-F3, HNR, jitter/shimmer) + **penn** (F0/periodicity). Bit-exact Praat fits your reproducibility bar.
4. **Harness:** **VERSA** as unified runner if its dependency weight is acceptable; else a thin script over 1-3.

All CPU/MPS-viable; nothing requires CUDA.

---

## I. License / ethics flags

### Can generated audio train an open-sourced model?

- **Coqui XTTS-v2 — NO.** [CPML](https://huggingface.co/coqui/XTTS-v2) is non-commercial and **explicitly covers outputs**; Coqui shut down Jan 2024, so no commercial license is purchasable ([discussion](https://github.com/coqui-ai/TTS/discussions/4304)). *(tried-and-true)*
- **Fish Speech / OpenAudio — NO for permissive release.** [openaudio-s1-mini weights CC-BY-NC-SA-4.0](https://huggingface.co/fishaudio/openaudio-s1-mini) (verified this pass); S2 reports conflict (research license vs MIT) — assume NC, verify the exact checkpoint card. *(tried-and-true for s1-mini)*
- **F5-TTS — NO for base weights** (CC-BY-NC-4.0 because Emilia-trained; [discussion](https://github.com/SWivid/F5-TTS/discussions/997)). Retraining on Emilia-YODAS (CC-BY-4.0) would be clean; OpenF5-TTS already did. *(tried-and-true)*
- **Kokoro-82M — YES.** [Apache-2.0 weights](https://huggingface.co/hexgrad/Kokoro-82M), no output restrictions. Provenance nuance for your datasheet: itself trained partly on synthetic audio from closed commercial TTS. *(tried-and-true)*
- **Zonos / ZONOS2 — YES.** Apache-2.0. *(reported)*
- **Chatterbox — YES, with a watermark note.** MIT incl. [Multilingual v3 (Jun 2026)](https://www.resemble.ai/resources/chatterbox-multilingual-v3-tts-with-embedded-watermarking-for-25-languages); every output carries the **PerTh watermark by default**. Legally fine to train on; document it, and don't strip the watermarker (EU signal). *(reported, consistent across sources)*
- **Orpheus — CAUTION.** Code Apache but **weights under Llama 3.2 Community License** (Llama-3.2-3B backbone; [HF discussion](https://huggingface.co/canopylabs/orpheus-3b-0.1-ft/discussions/2)) — output-reuse/derivative-naming clauses make training an unrelated open model on its outputs murky. Prefer Kokoro/Chatterbox/Qwen3-TTS. *(reported)*
- **Higgs Audio — NO.** Community license with an explicit clause prohibiting using outputs to improve other models. *(reported)*

### Dataset licenses

- **Emilia:** CC-BY-NC-4.0 (101k h) — **but Emilia-YODAS (114k h) is CC-BY-4.0** ([Amphion docs](https://github.com/open-mmlab/Amphion/blob/main/preprocessors/Emilia/README.md), [paper](https://arxiv.org/html/2501.15907v2)). The YODAS half is the commercially-viable path. *(tried-and-true)*
- **LibriTTS/-R: CC-BY-4.0 · VCTK: CC-BY-4.0 · Common Voice: CC0** — clean with attribution. *(reported)* LibriLight: per-file public-domain claims *(unverified)*.
- **Singing datasets — all NC** (OpenCpop, M4Singer, GTSinger). NC-**SA** arguably propagates share-alike to weights — legally contested, but the conservative community reading treats weights as derivatives. *(tried-and-true for licenses; contested for propagation)*

### Regulatory context, mid-2026

- **EU AI Act Article 50 became applicable 2026-08-02** (nine days before this survey). Providers of generative audio must mark outputs machine-readably as AI-generated (Art. 50(2)); deployers must label deepfakes (Art. 50(4)) — *intent to deceive not required*. Commission [guidelines adopted 2026-07-20](https://www.gtlaw.com/en/insights/2026/6/deepfakes-chatbots-ai-generated-text-european-commission-details-transparency-obligations-under-the-ai-act) + a [Code of Practice on marking](https://digital-strategy.ec.europa.eu/en/policies/code-practice-ai-generated-content). Fines to €15M / 3% turnover. The **Digital Omnibus did NOT delay Art. 50** — it delayed high-risk categories and gave a grace period to Dec 2, 2026 only for systems on the market before 2026-08-02 ([Gibson Dunn](https://www.gibsondunn.com/eu-ai-act-omnibus-agreement-postponed-high-risk-deadlines-and-other-key-changes/), [Art. 50 FAQ](https://digital-strategy.ec.europa.eu/en/faqs/transparency-obligations-under-article-50-ai-act)). *(tried-and-true, primary EU sources)*
- **NO FAKES Act — still not law, but closest ever.** Reintroduced 2026-05-20 as [S.4591](https://www.congress.gov/bill/119th-congress/senate-bill/4591/text)/[H.R.8915](https://www.congress.gov/bill/119th-congress/house-bill/8915/text/ih); **Senate Judiciary advanced it unanimously 2026-06-18** ([Holland & Knight](https://www.hklaw.com/en/insights/publications/2026/06/senate-judiciary-committee-advances-legislation-to-protect-name)). Federal voice/likeness digital-replica right with DMCA-style notice regime. Assume it may be law within your release window. *(tried-and-true)*
- **ELVIS Act (TN)** in force since Jul 2024: civil + criminal liability for unauthorized AI voice simulation; active enforcement interest reported in 2026 ([tracker](https://efros.com/research/state-ai-law-tracker/tennessee/)). California AB 1836/2602 and other state laws exist *(training knowledge)*.
- **Watermarking norms:** [AudioSeal](https://github.com/facebookresearch/audioseal) is now **fully MIT including the model**; PerTh ships by default in Chatterbox. With Art. 50(2) live, watermark or C2PA-style provenance on distributed synthetic audio is table-stakes in the EU. *(tried-and-true)*

### Concrete guidance — what would taint your release

1. **Taints (do not train on):** XTTS-v2 outputs (CPML), F5-TTS base-weight outputs, Fish/OpenAudio outputs (NC-SA), Higgs outputs (explicit non-compete), Vevo2 outputs (NC-**ND** — ND arguably forbids derivative models outright), anything fine-tuned on GTSinger/M4Singer/OpenCpop if the release is Apache/MIT-style. The mechanism is contract/license, not copyright — "synthetic audio isn't copyrightable" does not defeat a license you accepted on download.
2. **Clean generators:** Qwen3-TTS, Kokoro, Chatterbox (watermarked), CosyVoice3, OmniVoice, ZONOS2, Step-Audio-EditX (speech); SoulX-Singer (pending LICENSE confirmation) and self-trained NNSVS/DiffSinger voicebanks (singing). Clean datasets: Emilia-YODAS, LibriTTS(-R), VCTK, Common Voice.
3. **Ship with:** a per-item provenance manifest (generator + version + seed + license) — doubles as EU Art. 50 marking evidence and is trivially compatible with your deterministic reproducibility story; AudioSeal (MIT) on demo audio; no cloning of real identifiable voices without written consent (ELVIS now, NO FAKES likely soon); a model card stating the synthetic-data chain (the Kokoro precedent shows the community accepts this when disclosed).
4. **Residual uncertainty to flag in your docs:** whether NC/SA dataset terms legally propagate to model weights is unsettled in the US and EU; every reputable 2026 release follows the conservative position (treat weights as derivatives).

---

## What moved since early 2026 that the requesters likely do not know

**TTS / corpus generation**
- **Qwen3-TTS open-sourced Jan 2026** (Apache-2.0, 0.6B/1.7B) with *description-based voice design* — mint unlimited synthetic speakers from text descriptions; instantly the permissive-license quality leader ([blog](https://qwen.ai/blog?id=qwen3tts-0115), [Willison](https://simonwillison.net/2026/Jan/22/qwen3-tts/)).
- **Mistral entered TTS** (Voxtral TTS, Mar 2026, near-ElevenLabs at 90 ms) — but CC-BY-NC weights.
- **License-tightening trend among quality leaders:** Fish S2 → research license; Spark-TTS retroactively Apache→CC-NC; Higgs added an explicit "no training other models on outputs" clause. The permissiveness/quality frontier is diverging — audit weights licenses per checkpoint, always.
- **OmniVoice** (k2-fsa, Mar 2026): 646-language zero-shot TTS, Apache-2.0. **ZONOS2** (Jun 2026): first open MoE TTS. **IndexTTS-2.5** (Aug 10, 2026): explicit duration control, license-encumbered. **Step-Audio-EditX**: LLM-grade iterative emotion/style *editing* of existing audio — a new tool category, directly useful for disentanglement pairs.
- **mlx-audio matured into the de-facto Apple Silicon TTS runtime** (20+ models, quantization, MIT) ([repo](https://github.com/Blaizzy/mlx-audio)); the open/closed TTS quality gap collapsed to ~81 Elo.

**ASR / oracles**
- **The Open ASR leaderboard flipped three times in five months** (Cohere Transcribe → Granite Speech 4.1 2B, Apache, 5.33% → ARK-ASR / MOSS-Transcribe); Canary-Qwen, the late-2025 champion, is mid-pack. Top of leaderboard is now **LLM-decoder ASR — bad news for posterior access**; frame-level CTC lives on in the phonetics community (ZIPA → PhoneticXeus → BranchShine, all IPAPack++-based) and NVIDIA's speed tier.
- **Qwen3-ASR (Jan 2026, Apache)** is the first permissive open model advertising stable **song** recognition. Still **no open Whisper successor from OpenAI**.
- **WER-of-synthesis was shown to be ASR-family-confounded** ([arXiv 2607.08256](https://arxiv.org/pdf/2607.08256), Jul 2026) — the emerging norm is dual-family WER reporting.

**Codecs / rate-distortion**
- **The sub-1 kbps league table got owners:** NanoCodec (NVIDIA, 62M, open weights) beats Mimi across the board at 1.1 kbps; **FocalCodec (Apache) is the open default at 0.16-0.65 kbps** with streaming variants and ONNX. The ~0.1 kbps region opened via **TaDiCodec (87.5 bps)** — but with text-conditioned decoding (uncounted side information).
- **BD-rate arrived in codec papers** ([ECC, Jun 2026](https://arxiv.org/pdf/2606.11631)) — rate-distortion formalism is now community-legible; adopt it.
- **LRAC low-bitrate codec challenge** now has a standing ICASSP 2026 satellite venue with fixed metrics; UTMOSv2 was adopted by the ICASSP 2026 URGENT challenge; **VERSA** consolidated as the one-YAML eval harness; **torchaudio entered maintenance phase** (don't build long-term on SQUIM).

**Articulatory & physical modeling (the bet's home literature)**
- **VTL 2.4 shipped Dec 2025** — first mainline VocalTractLab release since 2020 (GPL).
- **ARTI-6 (ICASSP 2026):** "6 articulatory dimensions suffice for intelligible synthesis" is now peer-reviewed with code. **SPARC is quiet** (last push May 2025, no license file, no quantized/codec follow-up) — Berkeley's energy moved to RT-VC, RL articulatory control, and BCI. **The low-bitrate articulatory-codec slot remains unclaimed.**
- **First cross-lingual AAI benchmark** (FROST-EMA, Jun 2026): language mismatch hurts inversion 2x more than speaker mismatch — dents "universal inversion" claims.
- **"Four Decades of Digital Waveguides" (JAES 2026, Smith/Välimäki/Reiss)** canonizes differentiable waveguides and parameter-optimization approaches — the field just got its authoritative review, and it legitimizes exactly this problem statement.
- **diffKS (WASPAA 2025):** differentiable Karplus-Strong with gradient-vs-genetic comparison — direct prior art now exists. **Differentiable articulatory copy-synthesis reached singing** (DAFx 2026, Kelly-Lochbaum + second source on Tuvan sygyt). **Instrumental (Mar 2026):** fresh evidence CMA-ES beats gradient descent on non-convex synth inversion and that amortized inverses only pay in-distribution — direct support for the amortized-net + CMA-ES-polish architecture. **SCRAPL (Feb 2026):** scattering-based loss purpose-built for sound-matching gradients.
- **STCTS (Dec 2025): an 80 bps interpretable semantic codec** (text+prosody+timbre, TTS decode) — the nearest competitor at your bitrate, with a generative (non-deterministic) decoder you can differentiate against.

**Singing**
- **SoulX-Singer (Feb 2026, Apache-2.0 code+weights, verified):** first credibly permissive large-scale zero-shot SVS (42k h, F0 + MIDI conditioning) — changes the license-clean singing-corpus picture entirely. **Vevo2** (Mar 2026, Amphion) unified speech+singing control but with CC-BY-NC-ND weights. OpenVPI DiffSinger v2.3.0 (Jan 2026) added voicing/tension params — its control surface keeps converging toward interpretable per-frame parameters. so-vits-svc era formally over; Applio is the surviving SVC lineage. Singing datasets remain uniformly NC.

**SSL / benchmarks**
- **Omnilingual wav2vec 2.0 (Meta, Nov 2025, Apache, 300M-7B, 1,600+ langs)** is the freshest permissive encoder family; **XEUS quietly relicensed to CC-BY-NC-SA** — check before building on it. SUPERB is frozen/maintenance; energy moved to ML-SUPERB 2.0, DASB (discrete-token eval — the right frame for bitrate-normalized claims), X-ARES, Dynamic-SUPERB Phase-2.

**Law / ethics**
- **EU AI Act Article 50 became applicable 2026-08-02** — machine-readable marking of AI-generated audio is now a legal obligation in the EU (Commission guidelines Jul 2026; the Digital Omnibus did *not* delay Art. 50). **NO FAKES Act cleared Senate Judiciary unanimously Jun 2026** — closest ever to passage. **AudioSeal relicensed fully MIT** (model included). Watermark-by-default (Chatterbox/PerTh) is now the reference pattern for responsible open TTS.

---

## Cross-cutting synthesis for the Threader bet

1. **The claims-space is still open, and the wording matters.** SPARC owns "interpretable articulatory coding" (no bitrate); TaDiCodec owns ~0.1 kbps (text side-channel); STCTS owns 80 bps semantic coding (generative decode); FocalCodec owns open sub-0.5 kbps neural coding (opaque). Nobody owns: *explicit-bitrate interpretable physical-control coding with deterministic decode, covering speech and singing*. Claim all four qualifiers together.
2. **The architecture is now independently validated:** PinkVocalTransformer (amortized inversion from synthetic render banks), Instrumental (CMA-ES > gradients out-of-distribution; amortized inverse only pays in-distribution), VocalTrax (optimization-only stalls) — the Sobol-bank → amortized-inverse → CMA-ES-polish recipe matches the 2025-26 evidence exactly.
3. **The eval protocol is prescribed by the field now:** dual-family WER-of-resynthesis + SECS + UTMOS/UTMOSv2 + PESQ/STOI/ViSQOL (reported but not argued from) + BD-rate curves, run under VERSA, with a Codec-SUPERB submission for external credibility and rate-matched baselines FocalCodec/Mimi/NanoCodec/EnCodec.
4. **License discipline is a real risk surface in 2026:** verify *weights* licenses per checkpoint (they change retroactively — Spark-TTS, XEUS), keep NC/ND outputs out of the training corpus, ship a provenance manifest (which doubles as EU AI Act Art. 50 evidence), and note that the two most directly relevant prior-art repos (SPARC, diffKS) have **no license files** — benchmark against them, don't depend on them.
