# Sunskip

An original 48-bar instrumental in C, at 128 BPM, with a slightly swung eighth
grid. The musical aim is bright, playful and rhythmically detailed. The melody
introduces a G–A–C cell, gives the piano the middle, and returns with changes of
register, articulation and phrase length. The score has a composed ending.

The editable performance is [sunskip.mus](sunskip.mus). Rendered files live in
`out/`: `sunskip.mp3`, the 24-bit stereo `sunskip.wav`, the full render receipt,
energy trace, check report and a compact validation report. `pocket-groove.mp3`
isolates four bars of percussion with its own master normalization.
The compact full-song report is also retained as `validation.json` beside
the score, so its digests, execution counts and numeric ledgers travel with it.

## The instruments

| Instrument | Authored construction | Role |
| --- | --- | --- |
| Sunwire | Eight courses in four pairs, three reciprocal resonators; individually detuned paired modes | Theme, ornaments, register changes and selected intonation inflections |
| Tumbler | Four lower courses, three rescaled resonators, the bass hand's palm loss | Mobile bass and structural pulse |
| Prism | Six courses with a different inharmonicity/loss recipe and three rescaled resonators | Offbeat chord replies |
| Pocket | Six fixed modal percussion cells with distinct inharmonic spectra, losses and reciprocal connections to three resonators | Low pulse, tom replies, rim/side figures, bead and seed subdivisions |
| Ariadne piano | Existing `piano-v1` anatomy | Counterlines, selective pedal and the middle section |

The four new instruments are **fictional designs** assembled as ordinary
canonical `WorldPlan` JSON. Their altered parameters are design hypotheses,
recorded in the adjacent `*.design.json` files with baseline and plan hashes.
They are reusable by another MUS score through `anatomy=<path>`, with the
declared tuning and appropriate hands. Each instrument has persistent bodies,
reciprocal internal couplings, its field and its ledger. The five instruments
are mixed by the MUS bus; they are not one shared physical ensemble World.
Use frets 0–12 for Sunwire, Tumbler and Prism: their authored configuration
paths span one octave, and this score stays within those paths.

`build_instruments.py` gives every authored course's first mode, detuning,
partial rule, loss rule and resonator changes. These are parameter choices,
not measurements of the coupled sounding pitch. `build_percussion.py` records
the full Pocket construction. Its named cells are compositional roles; this
design does not claim a real membrane, snare-wire or shaker simulation.

Pocket is unfretted: use only `@string:0`. Its nominal pitch tokens address
fixed cells; the two configuration endpoints are identical, so higher frets
would not retune those cells. The mappings are `E2@6:0` low, `A2@5:0` tom,
`D3@4:0` rim, `G3@3:0` side, `B3@2:0` bead and `E4@1:0` seed. Its durations
place touches in time; the cell's losses determine its decay.

The canonical mechanisms and limits are documented in
[the constitution §§3, 5, 8](../../../ARIADNE-PURE-PHYSICS-CONSTITUTION.md),
[the steel-string specification, Layers 2–3 and Limits of v0](../../../SPEC-ARIADNE-STEEL-STRING.md),
and [the piano specification §§3–7](../../../SPEC-ARIADNE-PIANO.md).
In particular, the tab voices have a fixed pluck point per course and no
note-off damping; their decay comes from the authored losses and palm setting.
The piano supplies the held-key, staccato and pedal articulation.

## The performance

| Bars | Function |
| --- | --- |
| 1–4 | Gradual entrance; the lead appears as a pickup |
| 5–12 | Theme and harmonic replies |
| 13–20 | Longer answers and the F-minor colour |
| 21–28 | Piano foreground, lower lead replies, reduced percussion |
| 29–36 | Theme lifted and displaced, with a slide into the return |
| 37–44 | Expanded answers and a different percussion handoff |
| 45–48 | Short coda, shared arrival and decay |

Three GPT-6.1 Sol performance subagents supplied separate melody, harmony and
percussion parts under this form; the main agent integrated and revised them.
The editable inputs are `melody.parts.json`, `harmony.parts.json` and
`percussion.parts.json`. `assemble.py` combines them with the final instrument
declarations and balance. Changing a JSON part and rerunning the assembler
replaces the MUS file; edit the MUS directly when you want it to become the
independent performance source.

Pocket has 461 authored boundary pulses. Its figures include a two-bar low
conversation, alternating rim/side responsibility, a 3+3+2 bead pattern and a
seed accent phase every five sixteenths. Selected 32nd-note doubles, omissions,
ghost levels and individually written phrase endings change the performance
throughout. These details are in the percussion parts and the render's touch
trace, not random timing variation. The final bead and seed hands are raised
6 and 7 dB respectively relative to the initial design recommendation: the
isolated pulse probe put their initial window RMS about 34–36 dB below the low
pulse. That comparison is a waveform measurement, not a listening judgement.

A second study placed each instrument's representative passage in its own bar
under one shared master. Its initial one-second RMS windows put Pocket 5.85 dB
above Sunwire and the piano 13.33 dB above it. The final observation gains lower
Pocket by 15 dB, piano by 18 dB and Tumbler by 3 dB from that candidate. This changes
their mix contribution without changing the recorded playing gestures. The
study includes possible preceding tails and master limiting, so those window
measurements are not exact predictions of the complete ensemble's balance.
The final study's Pocket window is 4.83 dB below the lead and its piano window
0.61 dB below, recorded in `out/balance-final.metrics.json`.

Eighteen paired lead gestures request fine intonation, producing 36 individual
string touches with offsets from -8 to +6 cents. The new
[`intonation` control (SPEC-TAB §4.1)](../../../SPEC-TAB.md#41-fine-intonation-on-an-ariadne-tab-touch)
moves the existing stop coordinate and records it in the trace; it does not
guarantee that a coupled spectral peak moves by exactly that amount.
The four Sunwire pairs also have an authored static spread of ±1.7 cents in
their first-mode parameters (`sunwire.design.json`, `courses`).

## Reproduce

From the repository root:

```sh
cd mus-rs
cargo build --release -p mus-cli
cd ..
python3 examples/ensemble/sunskip/build_instruments.py
python3 examples/ensemble/sunskip/build_percussion.py
python3 examples/ensemble/sunskip/assemble.py
python3 examples/ensemble/sunskip/render.py
python3 examples/ensemble/sunskip/render.py --groove
```

The Python scripts need only the standard library. MP3 export additionally
uses `ffmpeg` when available. The render script runs `mus check` first and
requires a nonempty event stream, no bad tokens, the expected world set,
finite ledgers, zero refused piano strikes and the intended intonation count.
It writes the **actual numeric residuals** into `out/sunskip.validation.json`.
The trace's `schema` field identifies its format; stored body energies and
`ledgerFrames` should be read according to that file's columns.

The final audio receives the existing MUS bus high-pass and master, as
described in the piano specification §8. The listening MP3 additionally fades
its final 0.5 seconds to finish the remaining decay; the WAV, render digest
and energy receipts refer to the unchanged engine output. No outside recordings are used.
Verification covers score, execution, waveform levels and energy accounting;
there has been no independent critical listening assessment.

## Repository work

Custom plans with zero, duplicate, missing or malformed string IDs now return
an error before the engine can index their tuning. Fine intonation is checked
by both checker and renderer. Default and explicit-zero intonation were
verified to preserve an existing render byte for byte; nonzero gestures alter
the canonical observation and are receipted. Focused tests cover all four
touch types and rejection of invalid controls.

The prescribed release package tests pass: 90 engine, 3 notation and 6 WASM
tests. The prescribed CLI integration set passed 9 tests; the rebuilt CLI also
passed the explicit intonation compatibility probes. Both canonical architecture
and pure-physics source guards pass. The two pre-existing red tests listed in
the root `AGENTS.md` were not changed.
