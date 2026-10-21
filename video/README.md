# The Ovillo release video

A 42-second video for LinkedIn: a title card, four instruments (guitar, piano, trombone,
pink trombone), an end card. Each
instrument is an Ariadne World, and the video is drawn from that World's own data:
its WAV is the sound, its trace is the picture, its receipt is the ledger on screen.

```bash
./build.sh                 # data (renders only what is missing), soundtrack, frames, MP4s, checks
./build.sh --renders       # re-render every World first
./build.sh --variant 4x5   # one variant: 4x5 (1080x1350, primary) or 16x9 (1920x1080)
```

Outputs: `out/ovillo-release-4x5.mp4`, `out/ovillo-release-16x9.mp4`,
`out/<variant>-ffprobe.txt`, and stills and contact sheets read back from the MP4 in
`samples/<variant>/`. Everything under `data/`, `build/`, `frames/`, `out/`,
`samples/` is regenerated (git-ignored).

Needs: Rust stable (the repo's), Node 22, pnpm, uv, ffmpeg 8 at `/usr/local/bin`,
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
| renders | `build.sh` | `mus render ... --energy` (guitar), `trombone_nd_solo data/trombone solo trace` (trombone), `tools/tract-render.mjs` (tract), `tools/piano-ledger.sh` + `tools/piano-sync.sh` (piano) |
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

The render is the piano agent's (`~/dev/ovillo-piano-20261021`, branch
`ariadne/piano-world-v1-20261021` at `a20d520`; `mus render` of
`examples/piano/prelude-c-major.mus` on `anatomy=piano-v1`): `piano-demo.wav` (sha256
`011c2e4b7d716543...`, its receipt's `renderDigest`), its receipt and notes table.
Its trace books the hammer throws outside the per-hop source flows, so for a running
ledger `tools/piano-ledger.sh` re-renders the same committed score on
`site/video-piano-ledger-20261021` (that commit plus `ffb2219`, which only adds
`ledgerFrames`, `World::receipt` every 10 ms) and refuses unless the WAV is
bit-identical to the demo's; `tools/piano-sync.sh` then copies the demo files and that
trace into `data/piano/` with `SOURCE.txt` (branch, commits, sha256s).

On screen: 88 keys (a key is down from onset to release in the notes table; its
hammer is drawn reaching the string at the onset and falling back, a drawing timed by
the table), every played key's choir of strings lit by its string bodies' stored energy,
the soundboard's bridge lit by the 14 board bodies' energy, the grand's case a drawing.
Readouts: the last strike's hammer velocity and felt contact time (the trace's strikes).
Caption: board radiation 0.010972 J of 0.010974 J radiated (the receipt).
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
