# Ovillo

A bench instrument for playing a guitar through the Ariadne World and
watching what happens inside it. Native Rust on GPUI, drawn after Tesitura's
interface doctrine (a 90s instrument CRT: graticules, stroke traces, softkeys).

```text
guitarist ─▶ guitar (DI) ─▶ Ariadne World ─▶ output
 (player)    (simulated)    (mus-ariadne)     (CoreAudio)
                 │                │
                 └── scope tap ───┴── telemetry tap ─▶ screen
```

The guitar is simulated until the audio interface arrives: six waveguide
strings in standard tuning, a pick that darkens when soft, pickup-position
combs, a coil-and-cable resonance under a tone knob, and single-coil hum.
The engine wrapper is the Ariadne World plug-in's own (`ovillo_dsp::ariadne`,
lifted from `mus-rs/plugins/ariadne-clap`), so a patch made here loads into
the plug-in unchanged.

## Run

```sh
cargo run -p ovillo-app --release
```

The first build compiles GPUI and takes a while. `--release` is optional: the
DSP crates are optimized even in dev builds.

## Playing it

| key | does |
|---|---|
| 1–8 / F1–F8 | the softkeys on the right bezel |
| wheel, ↑ ↓ | turn the knob (the lit softkey's quantity); Shift is fine |
| Z X C V B N | pluck the open strings, low E to high e (Shift is harder) |
| Space | the simulated guitarist starts or stops |
| Tab | upper screen: spectrum or scope |
| R | record a take: `takes/take-*/dry.wav`, `wet.wav`, `patch.json` |
| M | mute the strings |
| H | clear the max hold |
| 0 | reset the World (body, reservoir, space at rest) |
| Esc | back to the main menu |

Drag in the square (MAP) to move through Ariadne's configuration space: left
is the six-course string body (tuned to a guitar in standard tuning), right is
the B♭ brass bore, and up is the parameter chosen under DISPLAY ▸ MAP Y.

## Patches

`patches/live.json` is the patch being played. The app writes it as you turn
knobs, and follows it when it changes on disk, so you can edit it in any
editor while the guitar plays. `patches/slot-{a,b,c}.json` are the PATCH
menu's slots; `patches/ovillo-default.json` is the default, which raises the
listener and output gains so the wet signal sits near the dry level.

## Screens

- **Spectrum** (default): dry in blue, Ariadne in orange, the body's 24 modes
  as green stems at their current frequencies (heights relative, 60 dB), the
  guitar's six strings as blue ticks on the top edge.
- **Scope**: dry and wet, triggered on the dry signal's rising crossing.
- **World**: ten seconds of the World's own energies (10·log10 E): material
  (green), reservoir (purple, dashed), shared space (blue).
- **Square**: the configuration map, or the output as an XY (M/S) display.

## Measured (Intel i9-9880H, 48 kHz)

- Ariadne World alone: ~3.1 µs per sample (≈6.6× real time) after the shared
  space's stored energy became running sums (commit `3201293`); it was
  70–100 µs per sample (0.25× real time) before.
- The whole rig (guitar + Ariadne + taps): ≈5.7× real time.
- Driven hard (a sustained −6 dBFS sine) the World is chaotic: a one-ULP input
  nudge reaches full scale within 0.1 s. At guitar levels it is stable.
