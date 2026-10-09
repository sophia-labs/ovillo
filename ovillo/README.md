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

```text
ovillo [--keys "<key> <key> …"] [--socket <path>|off] [--mute] [--no-activate]
```

`--keys` presses keys at startup; `--socket` moves the control socket (also
`$OVILLO_SOCKET`; `off` disables it); `--mute` (or `OVILLO_MUTE=1`) starts with
the master muted, silent from the first sample; `--no-activate` opens the
window without taking keyboard focus (for a second, test instance).

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
| `-` `=` | master level down / up 1 dB (Shift: 0.1 dB) |
| `\` (or backtick) | master mute |
| Esc | back to the main menu |

The main menu's eighth softkey is OUTPUT: MASTER (the knob turns it in dB),
MUTE, REC and RESET WORLD.

Drag in the square (MAP) to move through Ariadne's configuration space: left
is the six-course string body (tuned to a guitar in standard tuning), right is
the B♭ brass bore, and up is the parameter chosen under DISPLAY ▸ MAP Y.

## The guitarist

SOURCE ▸ PATTERN cycles what the simulated guitarist plays (the transport tool
does the same). Five pieces never repeat: seeded randomness walks fixed tables
of real voicings, bass patterns, licks and rhythms, so the audio thread never
allocates, and the tab and the chord truth stay exact (they come from the
events the guitarist plays).

| | |
|---|---|
| **WANDER** (default) | a Markov chord progression in G, D, A, E or C over open and barre voicings with colours (sus, add9, maj7, 7ths); sections of 4–16 bars, each strummed, half-time, palm-muted drive, broken chords or sparse; patterns that mutate bar to bar (dropped hits, ghost upstrokes, chucks), swells, cadences, the odd bend or top-string melody note, and now and then a pivot through the new key's V7 into a neighbouring key |
| **TRAVIS** | alternating-bass fingerpicking in C and G: the thumb on root and fifth (often palm-muted), syncopated fingers from six templates, scale tones borrowed on the off-beats for a melody, verses that cadence |
| **BLUES** | a 12-bar shuffle in E or A: boogie (5–6 or 5–6–♭7–6) that breathes bar to bar, stop-time and solo choruses, eight licks from two positions with curls and bends, quick-change, three turnarounds |
| **AMBIENT** | E or A, dorian or aeolian, in tides: open drones, slowly rolled colour chords, high-fret plucks with late vibrato, fourths on the top strings, rare slow bends, and bars of silence (the one for hearing Ariadne ring) |
| **MELODY** | a slow single line in A or D minor, phrased antecedent and consequent (the motif returns, sequenced), half cadence then home, over a soft bass |
| STRUM, ARPEGGIO, RIFF, NOTES | the original short loops |
| SCORE | the last score sent (below) |

`cargo run -p ovillo-dsp --example render -- 45 ambient renders` renders any
of them offline (`renders/` is ignored by git); a `.json` path renders a score.

## Scores

`play_score` (over the control socket or the MCP) hands the guitarist written
music. Strings count from the low E: 0 = E2 … 5 = E4, or `"E" "A" "D" "G" "B"
"e"`. Beats are quarter notes from 0. A chord is a shape name (`Em9`, `Cadd9`,
`F#m11`, `A5`, …: `ovillo_dsp::compose::SHAPES`), six frets low to high
(`[3,2,0,0,0,3]`, `-1`/`null`/`"x"` for a string not played), or tab
(`"x32010"`, `"x-5-7-7-6-5"`).

```json
{
  "name": "first light", "tempo": 76, "loop": false, "then": "resume", "humanize": 0.4,
  "events": [
    {"type": "strum",   "beat": 0,    "chord": "Em9", "spread_ms": 140, "velocity": 0.55},
    {"type": "pluck",   "beat": 1.5,  "string": "e", "fret": 7, "velocity": 0.6},
    {"type": "vibrato", "beat": 2.5,  "string": "e", "depth": 0.15},
    {"type": "strum",   "beat": 4,    "chord": "Cmaj7", "spread_ms": 120},
    {"type": "pluck",   "beat": 6,    "string": "e", "fret": 5},
    {"type": "bend",    "beat": 6.25, "string": "e", "semitones": 2},
    {"type": "strum",   "beat": 9.5,  "chord": "G", "dir": "up", "strings": [3, 4, 5]},
    {"type": "mute",    "beat": 15,   "strings": [0, 1]},
    {"type": "rest",    "beat": 20,   "beats": 4}
  ]
}
```

Events: `pluck` (string, fret 0–22, velocity, palm), `strum` (chord, dir
down|up, velocity, spread_ms from first string to last, strings, palm),
`mute` (strings, default all), `bend` (string, semitones 0–3; 0 releases),
`vibrato` (string, depth 0–1; 0 stops), `rest` (beats). A score lasts to the
bar line after its last event (or `length_beats`). It is checked on the UI
thread, with errors that name the event and field, and compiled into a sorted
boxed slice the audio thread only walks; replaced scores come back on a
garbage ring to be freed off the audio thread. **A one-shot score plays once
and the guitarist then returns to the composition it was playing before** (or
silence if it was stopped); `"then": "stop"` ends in silence; `"loop": true`
repeats until something else is chosen. `scores/first-light.json` is the whole
of the example.

## Master output

A smoothed gain (−60…0 dB) and mute after Ariadne, before the speakers. It is
the monitor, not the instrument: it is not part of any patch (patches stay
loadable in the plug-in), and the screens, meters and takes see Ariadne's
output before it. MASTER on the meter row and the MUTE annunciator show it.
A mute ramps (10 ms time constant) to exact silence in about 90 ms.

## Control from Claude (MCP)

The app listens on a control socket (JSON lines: `{"id", "op", "args"}` →
`{"id", "ok", "result" | "error"}`) and applies requests on the UI thread
through the same paths as the keys, so the screen, menus and `live.json` stay
coherent. The default socket is `ovillo.sock` in the per-user temporary
directory (`confstr(_CS_DARWIN_USER_TEMP_DIR)`, so it does not depend on
`$TMPDIR`); `--socket`/`$OVILLO_SOCKET` move it. If another live Ovillo already
answers there, the new one leaves it alone and shows **CTL OFF**; a stale socket
nobody answers on is replaced. **CTL** is lit while a client is connected or
spoke in the last two seconds.

`ovillo-mcp` is a stdio MCP server (hand-written JSON-RPC, protocol
`2025-06-18`) that bridges a Claude session to that socket. Build it and
register it from this workspace:

```sh
cargo build -p ovillo-mcp
claude mcp add ovillo -- "$PWD/target/debug/ovillo-mcp"
# for an instance on another socket:
claude mcp add ovillo-test -e OVILLO_SOCKET=/path/to/ovillo.sock -- "$PWD/target/debug/ovillo-mcp"
```

Tools: `get_state` (everything on screen as data), `press_keys` (any key
sequence), `set_params`, `get_patch`, `load_patch`, `save_patch`,
`default_patch`, `set_guitar`, `set_display`, `transport`, `pluck`, `strum`,
`mute_strings`, `reset_world`, `set_output`, `record`, `play_score`, and
`screenshot` (a PNG of only this instance's window, found by its pid and
window number; the terminal that launched Claude needs Screen Recording).
`ovillo-mcp --shot out.png [--pid N]` takes one picture from the shell.
`cargo run -p ovillo-mcp --example e2e -- --socket <path> --out <dir>` drives
every tool through the real binary against a running instance.

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
- The whole rig (guitar + Ariadne + taps): ≈5.7× real time (≈3.9× in the
  renders of the new pieces, run beside a live instance).
- The rig adds no allocation to the audio path (`tests/realtime.rs` counts);
  the Ariadne World itself still allocates and frees twice a sample, inside
  `mus-ariadne` (a body-id `String` clone in `set_body_configuration_index`
  and the `Vec` a body's `radiate` returns).
- Driven hard (a sustained −6 dBFS sine) the World is chaotic: a one-ULP input
  nudge reaches full scale within 0.1 s. At guitar levels it is stable.
