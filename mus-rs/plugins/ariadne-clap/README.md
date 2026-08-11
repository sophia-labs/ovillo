# ariadne-clap

A CLAP instrument plugin that plays the Ariadne string network — the
`StringNetworkVoice` weave voice from `mus-dsp` — inside a DAW.

This is the **P1 skeleton** from PW9 §3's technical ladder, item 4 of §5's
order of work. It is a wrapper and nothing else: *no DSP is implemented in
this crate*. Every sample the host hears came out of
`StringNetworkVoice::render_block`, and every parameter range came out of
`mus_vocab::param_specs()`. Design doc §8 is the rule being followed —
"the plugin wrapper should not reimplement synthesis", and "the plugin
should be a client of the declared vocabulary, not a second hand-maintained
parameter registry."

## Build

From `mus-rs/`:

```sh
cargo build -p ariadne-clap --release
```

That produces `target/release/libariadne_clap.dylib` (macOS),
`libariadne_clap.so` (Linux) or `ariadne_clap.dll` (Windows). A CLAP
bundle is that shared library renamed with a `.clap` extension:

```sh
# macOS — a bare .clap file is a legal CLAP plugin, no bundle needed
mkdir -p ~/Library/Audio/Plug-Ins/CLAP
cp target/release/libariadne_clap.dylib ~/Library/Audio/Plug-Ins/CLAP/ariadne.clap

# Linux
mkdir -p ~/.clap
cp target/release/libariadne_clap.so ~/.clap/ariadne.clap
```

The library exports the `clap_entry` symbol (checked with
`nm -gU target/release/libariadne_clap.dylib | grep clap_entry`). The
plugin ID is `com.sophia-labs.ariadne`.

Gates, all from `mus-rs/`:

```sh
cargo build -p ariadne-clap --release
cargo clippy -p ariadne-clap -- -D warnings
cargo test  -p ariadne-clap
```

### Why no VST3

nih-plug can emit CLAP and VST3 from one codebase, and PW9 §3 asks for
both — but its VST3 wrapper links `vst3-sys`, which is **GPLv3**, while
this workspace is MIT. Shipping VST3 from here would relicense the
codebase, so the dependency is declared `default-features = false` (the
`vst3` feature is off) and only `nih_export_clap!` is called. This is a
licence decision, not a technical one; revisit if a permissive VST3
binding appears. CLAP-first was the plan anyway (PW9 §3: CLAP's
polyphonic modulation is the same shape as our per-course control seam).

The `nih_plug` dependency is pinned to rev
`f36931f7af4646065488a9845d8f8c2f95252c23`. It builds on stable Rust
(verified on 1.95.0); nih-plug's nightly-only `simd` feature is not used.

## What it does

- **MIDI in, stereo out.** Note-on weaves a `StringNetworkVoice` at that
  key's frequency.
- **Nine block-automatable parameters** — `couple`, `chirality`, `orbit`,
  `orbit_depth`, `curvature`, `body`, `buzz`, `sus`, `damp` — pushed into
  every live voice once per block via `apply_control`, which is exactly
  the PW9 §4 automation seam. Turning `orbit` mid-note cranks the field
  on the ringing note; the engine's loop-gain clamp guarantees no
  reachable state is expansive.
- **Two construction-time parameters** — `courses` (3..24) and
  `dimension` (0.55..3.0) — read when a voice is woven, because they set
  the network's topology. They affect the *next* note, not a sounding one.
- Ranges, defaults and the automation classification are all read from
  `mus-vocab` at instantiation. A test in `src/lib.rs`
  (`vocab_ranges_match_the_engine_clamps`) fails if the vocabulary and the
  engine's clamps ever drift apart.

## Honest status

What P1 **does not** do. Each of these is a deliberate omission with a
named fix, not an oversight:

1. **Not allocation-clean.** Note-on calls `StringNetworkVoice::weave`,
   which allocates delay lines, body modes and excitation buffers on the
   audio thread, and it builds a `BTreeMap<String, String>` to get there.
   Retiring a finished voice frees on the audio thread too. This is a real
   xrun risk under dense MIDI. **P2 fix:** a voice pool — `MAX_VOICES`
   voices constructed in `prepare()` at the worst-case `courses`, plus an
   engine-side `re_weave(f0, slot)` that re-excites existing buffers in
   place. That is a mus-dsp seam; faking it in the wrapper would mean
   reimplementing construction here, which is the thing this crate must
   not do.
2. **Note-off is ignored.** Events are slot-shaped, as MUS score events
   are: a note rings for a fixed 2.0 s slot plus the kernel's decay tail
   (`min(sus*0.78, 4.0)` s) and then retires itself. Making note-off mean
   something needs a release seam in the engine — a ruling, not a wrapper
   improvisation.
3. **Block-granular event timing.** Note-ons start at the block boundary,
   so onset timing is quantised to the host's buffer size.
   `SAMPLE_ACCURATE_AUTOMATION` is `false` for the same reason: the
   engine's automation class is Block. Sample-accurate onset needs
   per-voice start offsets in the render loop (P2).
4. **Six voices, oldest-steal, no release.** Stealing cuts the stolen
   voice dead, with no fade — audible as a click when it happens.
5. **Velocity is an output gain, not an excitation level.** The kernel
   takes no velocity, so playing harder is louder but not brighter. The
   real seam exists (`physical_excitation` already takes a `level`); it
   just is not plumbed out of `StringNetworkVoice::new` yet.
6. **Mono signal, duplicated to both channels.** The kernel emits one
   signal. Inventing a stereo image out here would be exactly the
   forbidden move. Real stereo is a bridge/pickup pair in the engine (P2).
7. **No sidechain, no `sing`.** PW9 §3 argues the *entry product is the
   resonator, not the synth* — an FX plugin whose sidechain drives the
   `sing` seam. P1 is the instrument shape (no main input), because that
   is the smaller honest step. The FX shape needs `AUDIO_IO_LAYOUTS` to
   grow a stereo input, a `sing` parameter (the engine already declares
   `ControlTarget::Sing`), and a streaming `attach_external`, which today
   takes a whole slot's buffer up front.
8. **No parameter smoothing.** Block-stepped control values go straight
   into `apply_control`. Fast automation of `couple` or `curvature` can
   zipper. The engine's declared smoothing classes (PW9 §4: none |
   linear-over-block | coefficient-safe ramp) are where this gets fixed.
9. **No GUI.** The host draws a generic slider rail — which is precisely
   what Observatory v1 got wrong (PW9 §2). The Telar face (Ring as
   controller) is P2 and explicitly gated on the FX proving the surface
   first.
10. **The kernel is fixed at 48 kHz.** `mus_dsp::SR_F64` is a constant, so
    a host at another rate gets the 48 kHz render reinterpreted — a pure
    time-scale by `R = 48000 / host_sr`. P1 undoes that scale uniformly
    (note frequency and `orbit` are multiplied by `R`; slot length and
    `sus` are divided by it), so pitch, note length, decay and crank speed
    are correct in wall-clock terms at any rate. The residual is spectral
    and cannot be fixed from the wrapper: body resonances and filter
    corners scale too, so at 44.1 kHz the *body* is transposed down about
    1.5 semitones. A sample-rate-parametric kernel is the real fix.
11. **Not host-tested.** The gates prove it builds, is clippy-clean,
    exports `clap_entry`, and renders finite audible audio through the
    processor struct. Nobody has loaded it in a DAW yet, and no CLAP
    validator (`clap-validator`) has been run against it.
12. **No state versioning or presets.** Parameter state is whatever
    nih-plug's default `Params` serialisation gives. There is no preset
    format, and no score round-trip (PW9 §3 wants gesture → mus event).
13. **`pos`, `pick`, `stiff`, `tension`, `symp`, `detune`, `strum`, `pm`,
    `body_size`** are not exposed; voices are woven with the engine's
    `PluckPatch` defaults for those.

## Layout

- `src/lib.rs` — the whole plugin. `AriadneParams` (the vocabulary
  client), `AriadneEngine` (voice allocation, sample-rate mapping,
  summing — deliberately free of nih-plug types so tests can drive it
  without a host), and `AriadneClap` (the `Plugin`/`ClapPlugin` impl,
  whose `process()` is a thin shell over `AriadneClap::render_block`, the
  same entry point the tests use).
