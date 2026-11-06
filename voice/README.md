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
