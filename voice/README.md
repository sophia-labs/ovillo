# ovillo-voice

Vera speaks and an Ovillo Body shapes it. Her waveform is the energy that enters a physical World at a boundary. The Body is what she speaks through. Her performance is never resynthesised (see verabot `film/audio/voice/DESIGN.md`, "North star").

This package is the scaffolding around the Body: recording, analysis, comparison, a studio page and an MCP server. The Body itself is the Rust crate `ovillo/crates/ovillo-body`, with its knobs mapped in its `SURFACE.md`. This package calls its binary as `ovillo-body describe | presets | render`.

```bash
ov() { uv run -q --project ~/dev/ovillo-voice/voice ovillo-voice "$@"; }
ov serve                                  # the studio page, http://127.0.0.1:8023 (REC / STOP / ANALYSE live there too)
ov record start sample-01                 # chime, then the mic goes to takes/sample-01/dry.wav
ov record stop
ov segments sample-01                     # the stretches between pauses, to label
ov sections sample-01 room=0.2:9.8 ah=11.0:14.2 passage=60.1:88.0
ov analyse sample-01                      # report.json + overview.png
ov presets && ov describe                 # the Body surface
ov render sample-01 v --set some_knob=0.4 --as v-try3   # then dry/wet comparison: compare.json + compare.png
```

The MCP server is `ovillo-voice-mcp`, registered for ~/dev as `ovillo-voice`. It exposes the same operations as tools, and `analyse`, `render` and `compare` return their plots as images.

The studio is the directory everyone shares: Vera, the page, the CLI and any Claude. It is `$OVILLO_VOICE_STUDIO`, or by default `~/dev/verabot/film/audio/voice/studio`, because the takes belong to the film.

- **`takes/<take>/`:** `dry.wav` (never altered), `take.json` (sections) and `analysis/`.
- **`bodies/*.json`:** Body specs.
- **`renders/<take>/<body>/`:** `wet.wav`, `receipt.json`, `body.json`, `compare.json` and `compare.png`.
- **`log.jsonl`:** what everyone did, shown on the page.

What the analysis measures, and how, is described at the top of `ovillo_voice/analysis.py`. The comparison treats the Body as a system running from dry to wet:
- the cross-spectral transfer |H| (the resonances and notches it imposes)
- coherence, which falls where the Body is nonlinear
- the apparent vocal-tract length, dry against wet (the size cue a listener hears)
- the alpha ratio
- F0 (which a Body should not move)

The analysis was checked against a known EQ (a +12 dB peak at 3 kHz and a −18 dB notch at 700 Hz). It recovered both, with coherence 0.997.

## Prompts, dialogue and circled takes

- **Prompt sessions:** `ov prompts set <session> <file.json>` loads `[{id, say, kind, hint, target_hz}]`, and the page shows them one at a time.
- **Taking a line:** space records each prompt as its own take, `<session>-<id>`. A retake gets a `-2`, `-3`… suffix.
- **Measuring:** each take is measured on stop, by the quick op: pitch, swing, tract or formants, brightness, level.
- **Dialogue:** `ov script d01 "R1-01..R1-18,G-01..G-06" --order character` turns the film's script table (verabot `film/script/before_the_law.md`) into prompts, each with its cue line and the character's notes.
- **Circling and export:**
  - `enter` on the page (or `ov select`) circles the newest take of a line.
  - `ov export-selects` writes the circled takes, trimmed, to verabot `film/vo/vera/<line>.wav`, plus `selects.json`.
- **The pitch meter:** browser mic only; it displays and never records. It shows:
  - a live F0 trace
  - the floor (the 10th percentile of the last 3 s)
  - the prompt's target band
  - a **weight** chip: live H1–H2, roughly 0 to +6 dB for light-but-connected

## The non-physics comparison

`ov shift <take> <ratio>` is Praat's "Change gender" (PSOLA): every formant times the ratio, with her pitch kept. It's NOT Ovillo and NOT physics. It exists to show the physical Bodies what a size cue sounds like, and the page and `body.json` label it so.

## Gotchas

- **Mic open delay:** the mic takes 1–7 s to open, so the recorder waits for samples before it chimes or shows SPEAK NOW.
- **Phone mic:** the "Fairfield Park" Continuity mic writes files 6–8% shorter than the time recorded, so don't trust it until it passes a tone test.
- **Speed:** CPPS uses a straight least-squares trend line, because Praat's robust fit is about 50× slower.
