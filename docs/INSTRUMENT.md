# instrument

A system for composing, executing, inferring, and certifying continuous
gestures over persistent physical bodies.

The repository began as **MUS**, a compact score language for musical
conversation between humans and language models. It now contains one larger
program:

```text
score / observation
    -> authored or inferred trajectory
    -> persistent physical body
    -> sound + state/work/claim receipt
```

The major clients are:

- **MUS** — notation, sample-bound scoring, and the human/LLM musical surface;
- **Weave and Ariadne** — coupled string networks, persistent modal state,
  lawful control-path execution, work ledgers, and protected/non-Abelian
  research;
- **GARGANTA** — speaker-static and evolved pseudobiological throats with
  articulation and source controls;
- **Threader** — inverse compilation from sound into anatomy and local control
  programs.

`SPEC-TRAJECTORY.md` is the current integration seam. A trajectory is a direct,
content-addressed path through physical coordinates; a gesture is a higher-level
target whose compiler may choose such a path. The runtime-neutral Rust crate is
`mus-rs/crates/mus-trajectory`, the Python oracle is
`mus_analysis/trajectory.py`, and the first live adapters are
`mus_dsp::trajectory` and `GargantaTrajectoryAdapter`.

The project maintains an explicit claim boundary. A successful render is not by
itself a theorem about transport, a biological reconstruction, intelligible
speech, or listener perception. Open obligations travel in receipts and strict
modes refuse them.

## Repository map

| Area | Purpose |
|---|---|
| `SPEC.md`, `mus.py` | MUS notation and MusicXML/MIDI round trips |
| `SPEC-AUDIO.md`, `mus_audio.py`, `aigua/` | sample-bound MUS and worked music |
| `mus-rs/` | Rust notation, engine, DSP, gesture, trajectory, and protected-path runtime |
| `SPEC-ARIADNE-STEEL-STRING.md`, `SPEC-ARIADNE-PIANO.md`, `profiles/` | the instruments `mus render` plays on Ariadne Worlds (the steel-string guitar, the piano) and their anatomy plans and receipts |
| `ovillo/`, `TROMBONE-WORLD.md`, `TRACT-WORLD.md` | the bench app, the trombone v1 World (`ovillo-dsp`), the vocal tract Worlds (`ovillo-tract`, `ovillo-tract-world`) |
| `site/`, `video/` | the sources of ovillo.sophia-labs.com and of the release video |
| `paper/` | Ariadne mathematical constructions, oracles, and evidence packets |
| `mus-rs/SPECS/garganta/`, `mus_analysis/garganta_anatomy.py` | throat and anatomy program |
| `mus-rs/experiments/threader_poc/` | hierarchical inverse-instrument POC |
| `formal/` | Lean refinement experiments |
| `.string-research/` | the lab's research notes (briefs, returns, receipts, state maps), kept in the private lab repository; code comments that cite `.string-research/...` point there |

## MUS — the score language

MUS is a compact, LLM-readable score notation. It is designed to enable musical
conversation between humans and LLMs as compositional peers. The asymmetry is
the design: the human reads engraved sheet music in its native form, the LLM
reads MUS, and the two readings are faithful enough to the same source that the
human and the LLM can talk about the music as peers. It is lossless with respect
to typical sheet-music content, approximately 10–20× more compact than
MusicXML, and tractable for an LLM to read as a score where engraved notation
is not.

Spec: [`SPEC.md`](./SPEC.md).

### Why

Existing score formats were not designed for the human-and-LLM-together
audience. MusicXML round-trips between notation editors. LilyPond engraves to
print. Humdrum analyses statistically. ABC fits folk-tune corpora. NoteSequence
and REMI tokenize for ML training. Each does its own job; none enables the
conversation.

MUS is the first attempt at a format whose first audience is the dialogue
itself: a composer looks at the engraved score, the LLM reads the MUS, and both
are reading the same music. The LLM responds as something closer to a peer than
a search engine returning facts. MUS is not asking the human to read MUS instead
of sheet music. Engraved notation is the human-native form and should not be
replaced. MUS is what the LLM reads on its side of the conversation. It remains
human-readable enough to write, edit, and diff by hand when needed.

### Quick example

A 16-bar fragment of "This Is My Body" (5-instrument metal):

```text
# score: This Is My Body / sketch-v3
# summary: 16 bars / 5 voices / ~1 min
# tempo: 82    time: 4/4    key: Dm    bars: 16

# instruments:
#   g1 = elec gtr distorted (treble)
#   g2 = elec gtr distorted (treble)
#   b  = elec bass (bass, -12)
#   d  = drum kit (perc)
#   v  = vocals (treble)

# section: intro [1-4]

g1 1-4: D2w (4×)
g2 1-4: -
b  1-4: D2w (4×)
d  1-4: Kq Sq Kq Sq (4×)
v  1-4: -

# section: v1 [5-12] — vocals enter bar 6, Phrygian inflection bar 9

g1 5-6:  D2w (2×)
g1 7-8:  F2w (2×)
g1 9-12: F2w D2w D2w D2w
b  9-12: F2w D2w D2w D2w
d  5-12: Kq Sq Kq Sq (8×)
v  6: D4q "i" D4q "loved" D4q "you" D4q "from"
v  7: D4q "the" D4q "start" D4h R
v  9: D4q "traced" D4q "your" D4q "steps" D4q "with"
v 10: D4q "my" D4q "fin_" D4q "gers" D4h R
```

That is 16 bars in roughly 30 lines. The same content in MusicXML would carry
8–12 KB of tag overhead.

### Usage

The converter is a single self-contained script with PEP 723 inline metadata.
`uv` handles the `music21` dependency automatically.

```bash
./mus.py path/to/song.musicxml > song.mus
./mus.py path/to/song.mid > song.mus
uv run mus.py path/to/song.mxl
```

The direction is auto-detected from the input extension. MUS can also be
converted back to MusicXML or MIDI.

### Status

**MUS v0.4.** Implements bar-major output; within-bar pattern compression;
tacet and identical-bar collapse; multi-tempo/key/time headers; tuplets;
lyrics; expanded articulation taxonomy; dynamics and hairpins; rehearsal
sections; system text; slurs; multi-hand naming; diagnostics; and reverse
conversion.

Tested on:

- *This Is My Body* — 171 bars, multi-tempo metal;
- *Crave* — 131 bars, meter and key changes;
- *Composition 1* — oboe and string trio with dense articulation;
- *Verklärte Nacht* — 421 bars, four piano staves, 60 tempo changes, and
  late-Romantic chromatic figuration.

## Sample-bound MUS (MUS-A)

MUS's absolute-pitch choice is exactly what a sampler needs. `A6` means a
frequency, which can be compared with a sample's measured fundamental to derive
transposition. MUS-A adds notation for stereo position, filter motion, reverb
send, time-stretch, and related recorded-audio operations inside the existing
bracket suffix:

```text
sb 5: A1+22w[pan=-1->1, lpf=70->900]{ff}
```

It also adds cents offsets and score-local tuning:

```text
A6+22
# tuning: A=445.6
```

Render with:

```bash
./mus_audio.py aigua/aigua.mus -o out.wav
```

`aigua/` is a worked example: a 56-second field recording from Aiguá, Uruguay,
analysed into a playable instrument and used to score a 2.5-minute piece.

## Trajectory example

The new vertical slice can be run directly through the live Rust string body:

```bash
cd mus-rs
cargo run --release -p mus-dsp --example trajectory_etude -- trajectory-etude.wav
```

The example writes audio and a sibling trajectory receipt. Python examples for
both the string body and GARGANTA are under `examples/trajectory/`.

## License

MIT. See [`LICENSE`](./LICENSE).
