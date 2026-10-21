# The Ovillo release video

A 42-second video for LinkedIn: a title card, four instruments (guitar, piano, trombone,
pink trombone), an end card. Each
instrument is an Ariadne World, and the video is drawn from that World's own data:
its WAV is the sound, its trace is the picture, its receipt is the ledger on screen.

```bash
./build.sh                 # data (renders only what is missing), soundtrack, frames, MP4s, checks
./build.sh --renders       # re-render every World first
./build.sh --variant 4x5   # one variant: 4x5 (1080x1350, primary) or 16x9 (1920x1080)
./build.sh --piano-at ffb2219   # the piano as released (see "Provenance"), not this checkout's
```

Outputs: `out/ovillo-release-4x5.mp4`, `out/ovillo-release-16x9.mp4`,
`out/<variant>-ffprobe.txt`, and stills and contact sheets read back from the MP4 in
`samples/<variant>/`. Everything under `data/`, `build/`, `frames/`, `out/`,
`samples/` is regenerated (git-ignored).

Needs: Rust (the release was built with rustc 1.95.0; see "Provenance" for why the version matters to the
tract), the `wasm32-unknown-unknown` target, Node 22, pnpm, uv, ffmpeg 8 at `/usr/local/bin`,
and Playwright's Chromium for Testing 149 (revision 1228, the one
`playwright-core@1.61.1` pins; `pnpm exec playwright-core install chromium` if it is not
cached). Built and checked on macOS (the panel's face is the site's `ui-monospace`,
SF Mono there).

## Rules it keeps

- Every sound is an Ariadne World's WAV. No samples, synths, beds or stock audio.
  The cards are silent.
- The browser never makes a sound: headless Chromium with `--mute-audio`, a page with
  no audio element and no AudioContext, a throwaway profile per capture job. The
  soundtrack is assembled by ffmpeg from the WAVs.
- The pictures are driven by the renders' data (below says what drives what, and
  what is a drawing choice). Captions state only what a receipt or spec supports.

## Pipeline

| step | tool | what it does |
|---|---|---|
| renders | `build.sh` | `mus render ... --energy` (guitar), `trombone_nd_solo data/trombone solo trace` (trombone), `tools/tract-render.mjs` (tract), `tools/piano-render.sh [rev]` (piano) |
| prep | `tools/prep.py` | cuts each trace to its window for the page (`build/<id>.json`), the timeline (`build/timeline.json`) |
| soundtrack | `tools/audio.py` | sample-exact excerpts, fades, per-segment loudness match, loudnorm two-pass (`build/audio/`, receipt in `build/audio/receipt.json`) |
| frames | `tools/capture.mjs` | `page/index.html`, `window.renderFrame(n)` for every frame, PNG screenshots at 2x (`frames/<variant>/`) |
| MP4 | `tools/assemble.sh` | lanczos to size, BT.709, H.264 High CRF 16, AAC 192k 48 kHz stereo, `+faststart` |
| checks | `tools/verify.sh` | ffprobe, every frame's brightest pixel (no blank frames), EBU R128 of the encoded audio, stills and contact sheets |

The page is a pure function of the frame number: three.js 0.170.0 (pinned, served from
`node_modules`, no CDN), software WebGL (SwiftShader, 4x MSAA), captured at device scale
factor 2 and downscaled with lanczos.

## The segments

| | source | window | why this window |
|---|---|---|---|
| 01 guitar | `examples/tab/wayfaring-stranger-doc.mus`, steel-string-v0 | 0.00-10.00 s | the intro from the World at rest: alternating thumb, hammer-ons and pull-offs; it ends on the bar-5 downbeat (A minor), faded over its first 0.38 s |
| 02 piano | the Ariadne piano's demo, `examples/piano/prelude-c-major.mus` (Bach, BWV 846), piano-v1 | 0.00-8.50 s | the opening from the first strike: bars 1-2 and half of bar 3, faded over 0.4 s |
| 03 trombone | `trombone_nd_solo` solo (trombone v1) | 14.90-24.50 s | the solo's third phrase: scoop into F4, rip to C5 (partial 10), tongued triplets, smears down to position 5, then the next phrase's smear out to position 7 |
| 04 pink trombone | `tools/tract-render.mjs` on ovillo-tract-world | 0.00-8.40 s | one scripted take through the tract page's own controls: /a/, a lip closure (pa), /i/, /u/, a nasal (ma), a voiceless alveolar hiss |

### Guitar: what drives what

Follows `blender/README.md`'s rules. The top's lines are the sum of its real coupled
mode shapes (the visual anatomy's 72; the 24 with most energy in the window), each at
its real energy (displacement `sqrt(2E)/omega`, high modes lifted by `(f/f0)^0.6`); the
swing's tempo (`0.45 sqrt(f/f0)` Hz) is invented and its size is compressed (the 0.4 power
of the summed amplitudes; the window's loudest moment is drawn at 45 mm). Each string
glows and swings with its body's stored energy over its current speaking length (the
stop the tab names; the 7.5 Hz shimmer is invented). Amber marks are the touches at the
hand's pluck point (diamond: thumb; circle: fingers; at the fret for hammer-ons,
pull-offs and slides). Ticks at the saddle are the energy per second crossing each
string's bridge edges into the box. Blue rings in the soundhole are the air-dominated
share of the box's modal energy; blue shells are onsets of radiated power (their shape
is invented: radiation here is monopole-weighted). The floor graticule's brightness is
the room field's stored energy. The ledger is `ledgerFrames`, the World's receipt every
10 ms (added to the trace for this video; the WAV is bit-identical with it).

### Trombone

The bore is the plan's tube network: 124 volumes in five tubes, each ring at a volume's
centre with its declared area's radius. The slide tube is `1.19 m + 2x` (the plan's
`tube_length` term), x the World's slide coordinate at that moment, and its sections
stretch uniformly (the model's "rubber slide"). The path the tubes are laid along (legs,
crook, bow, the bell section beside the slide) is a drawing, not the plan. The orange
trace riding outside the bore is each volume's RMS pressure over 10 ms
(`SpaceBody::node_pressure`, the World's modes at that volume), its shape against that
moment's largest pressure; the bell's rings are lit by it too. The lips are the flow
valve's half-gap (smallest and largest over the 10 ms, drawn x12), amber with its mean
flow. Blue wavefronts leave the bell with the World's radiated power at the moment each
leaves (their spacing and speed are invented). The ruler is the anatomy receipt's seven
positions. The note readout and "measured" cents are the notes table and
`mus_analysis.trombone_world_analyse` of this WAV. The solo is a note list in
`examples/trombone_nd_solo.rs`, not MUS, so the panel shows the notes table the render
wrote (the player's placement) instead of a score.

### Pink trombone

`tools/tract-render.mjs` drives `ovillo-tract-world`'s wasm (`TractNdAnatomy::public_v1`,
full schedule) at 48 kHz in 80-sample blocks with the page's controls (`tract_set`,
`tract_preset`, `tract_constrict`, `tract_vibrato`), snapshots the state vector and the
World's receipt (`ndt_receipt`, added for this video) at every video frame, and writes
the WAV from the same run. `--check-observers` renders again with no snapshots and
asserts the samples are bit-identical. The take uses slider-like ramps on the lips and
velum (a step target made the articulators' finite speed click). The p is the lips
control closing early enough that the narrowest aperture reads closed for about 0.1 s
(the frames at 1.33-1.43 s); the copied `pa` gesture is not used, since it holds its
closure only about 13 ms (`TRACT-WORLD.md`). The airway rings are the 44 oral sections' areas hanging from the tract
page's fixed outer wall, lit by their RMS pressure; the nasal passage is the 31 nasal
sections; the folds are the valve's half-gap envelope (x8); amber at the narrowest
column is the turbulence the jet injected in that frame; blue arcs are the World's
radiation at the lips and nostrils. Gaps of the World tract, on screen too: no
aspiration noise, one jet (`TRACT-WORLD.md`).

### Piano

`tools/piano-render.sh [rev]` renders `examples/piano/prelude-c-major.mus` (`anatomy=piano-v1`)
into `data/piano/`: `mus render` writes the WAV, its receipt and its energy trace, and the
`mus-engine` example `piano_world` writes the notes table; `SOURCE.txt` names the tree and
the sha256 of every file. With no `rev` it is this checkout's piano; with one, the piano at that
commit (`git archive <rev>` into `.cache/`, built there). The trace carries `ledgerFrames`
(`World::receipt` every 10 ms): the hammer throws are booked outside the per-hop source
flows, so a running ledger needs the World's own books.

On screen: 88 keys (a key is down from onset to release in the notes table; its
hammer is drawn reaching the string at the onset and falling back, a drawing timed by
the table), every played key's choir of strings lit by its string bodies' stored energy,
the soundboard's bridge lit by the 14 board bodies' energy, the grand's case a drawing.
Readouts: the last strike's hammer velocity and felt contact time (the trace's strikes).
Caption: board radiation 0.010972 J of 0.010974 J radiated (the released piano's receipt, `a20d520`).
The current piano's receipt says 0.019257 J of 0.019258 J (99.99 %): a rebuild from this
checkout should update the caption in `manifest.json` to its own receipt.
`placeholder/piano-demo-notes.PLACEHOLDER.tsv` and the placeholder path in
`page/scenes/piano.js` were the silent stand-in used before the World existed.

## Sound

Each excerpt is cut at `round(window[0] * 48000)` for exactly `frames * 1600` samples,
with short half-sine fades, mono duplicated to stereo, gained to -16 LUFS integrated.
The tract's lip closure and burst would pass the ceiling at that gain, so that one
excerpt goes through a look-ahead limiter at -2.5 dBFS (recorded in
`build/audio/receipt.json`); nothing else is limited. loudnorm then runs two-pass over
the whole track (-16 LUFS, -1.5 dBTP) and stays linear. The guitar's WAV is
`mus render`'s output: the World's listener into the engine's dry bus (send 0, so no
reverb), its 28 Hz high-pass and master stage.

## Provenance

The released MP4s are not committed (`out/` is ignored). Built on 2026-10-21 (macOS, x86_64):

| file | sha256 |
|---|---|
| `out/ovillo-release-4x5.mp4` | `b064ba0426c0e65444396a36e83fcf3c3b124d6efce1a1915021f57f334342f3` |
| `out/ovillo-release-16x9.mp4` | `c38841f9e95645df6e02ea77a42081e632d58b59ee369e3d8d5cf9d9ae98d534` |
| `build/audio/soundtrack.wav` | `944f7c0ac787ff10373e4967469b77ae5544d20810d10da178adc2dc75edca07` |

- The page and the tools: `video/` at `434eaec`. The MP4s were assembled after it and before
  `96c000b`, which changed only `build.sh` and this README and added `tools/piano-ledger.sh` (since
  replaced by `tools/piano-render.sh`).
- The Worlds, built with the machine's default toolchain, rustc 1.95.0 (the tract's take reproduces
  only with it; see below):
  - guitar: `examples/tab/wayfaring-stranger-doc.mus` on `fc016dd` (WAV sha256
    `9759473617875482d0ec86a278c9b85450d166fe1fda02dabefb4fbed8aaa27d`, its `renderDigest`);
  - piano: rendered at `a20d520` (`piano-demo.wav` sha256
    `011c2e4b7d716543c458e4e1ceeda8f1789354d5c83fe0b9e336e5855bdb77e5`), its picture's trace
    re-rendered at `ffb2219`, which is `a20d520` plus the read-only `ledgerFrames` and renders the
    same WAV;
  - trombone: `trombone_nd_solo <dir> solo trace` on `0091a9a` (WAV sha256
    `66021535a1c6a5965bdaf11c24e66ac0e706a7dff2141f21ebb4d97ad2be9191`);
  - tract: `tools/tract-render.mjs` on `ovillo-tract-world` at `277283e` (WAV sha256
    `a983cbbeabd2623a46a7e50be596d9ac66e89caab8bcfdf1f319f84190f8cce5`).

**The piano changed after the release.** `77db03d` and `9374486` (mode-diagonal radiation from the
soundboard, heard at two far-field points) changed its sound: on this checkout the demo renders
sha256 `997ce53b9a61f5d62881dad00f75ddb2dc8c3cf4c74eed2e7005e9f9a9bdc643`, so a plain `./build.sh`
on a fresh clone makes a video with the current piano. The released piano is reproduced by
`git archive a20d520` (its `mus render` gives `011c2e4b...`); `./build.sh --piano-at ffb2219`
(that is, `tools/piano-render.sh ffb2219`) does it with the ledger the picture shows.

Checked on 2026-10-21 from the merged `main` (frames and MP4s not re-rendered): with
`tools/piano-render.sh ffb2219`, the piano's WAV, notes table, energy trace and score are
byte-identical to the release's (the receipt differs in `wallSeconds` only), and `tools/prep.py`
and `tools/audio.py` give the release's `build/*.json` and `soundtrack.wav` byte for byte. On
`main` the guitar's and the trombone's WAVs and traces are the release's; so is the tract's take
when its wasm is built with rustc 1.95.0, while rustc 1.99.1's wasm renders a different take
(sha256 `d08b03f726157ff9194ed25dfed2037981d355b2252475414bd2de5b1da6fc60`), the same at
`96c000b`: the tract's wasm build is not bit-stable across compiler versions.
