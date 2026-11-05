# The Year 1812, for guts: the film

A 14 min 36 s concert broadcast of `ovillo/scores/fart/1812` (Tchaikovsky's
1812 Overture played by seven guts, each a `TractNdAnatomy::fart_v1`-family
World) and a 30-second portrait short of the finale's first cannon shots.
Every sound is the score's own mix; every picture is drawn from the stems'
traces (the Worlds' observers at every 10 ms hop).

```bash
./build.sh                  # prep, frames, MP4s, checks (both variants)
./build.sh --variant 9x16   # the short only
./build.sh --gpu --jobs 3   # let Chromium use the GPU (about three times faster than SwiftShader)
```

Outputs: `out/1812-for-guts-16x9.mp4` (1920x1080), `out/1812-for-guts-short-9x16.mp4`
(1080x1920), `out/<variant>-ffprobe.txt`, stills in `samples/<variant>/`. `build/`,
`frames/`, `out/`, `samples/` are regenerated (git-ignored). The capture is
`../tools/capture.mjs`'s method (muted headless Chromium, Playwright's, no audio
element, no AudioContext) and uses `../node_modules`.

## Pipeline

| step | tool | what it does |
|---|---|---|
| render | `ovillo-fart render` | the score's stems, each with a `.trace.f32` (areas, pressures, valve gap, reservoir, stomach, radiation, turbulence, the commands) |
| prep | `tools/prep.py` | traces to video frames (`build/frames.f32`), the timeline, note tables, maxima, the shot list (`build/film.json`) |
| frames | `tools/capture.mjs` | `page/index.html`, `window.renderFrame(n)` per frame, PNG screenshots |
| MP4 | `tools/assemble.sh` | the mix padded for the cards, loudnorm two-pass to -16 LUFS / -1.5 dBTP, H.264 High CRF 17, AAC 192k |
| checks | `tools/verify.sh` | ffprobe, no blank frames, EBU R128 of the encoded audio, stills |

The loudness step is the one thing done to the sound after the mix. The
mix sits near -34 LUFS integrated with the cannon's peaks at -1 dBTP, so the
gain that would bring it to -16 LUFS has no headroom; loudnorm then uses its
dynamic mode (a time-varying gain) instead of one linear gain. Both released
files got the dynamic mode; `build/audio/<variant>-receipt.json` records the
measurement, the target and the mode applied. A version with one fixed gain
would sit around -34 LUFS.

## What drives what

Stated in `page/film.js`'s header and on screen (the ledger line). In short:
the canal is the 28 sections' areas (radii drawn 1.6x their length-true size,
canals to scale with each other: 20, 20, 28 and 40 cm), lit by each section's
RMS pressure; the lips are the valve's half-gap envelope (x8); the bladder is
the reservoir's energy; the flask is the stomach; beans drop when the part
eats; puffs leave the mouth with the radiated power; the clamps are the
commanded seal and rasp; the spark is the jet's turbulence; the camera shakes
with the ensemble's radiated power. The stage, the path the canal curls
along, the puffs' drift and the shot list are drawings and direction choices.

The shot list (`tools/prep.py`, `plan_shots`): cuts at section starts and
every few beats inside one; a close shot's subject is the gut that radiated
the most power over the shot; a cannon shot in the window makes it a cannon
shot; the finale cranes. The short's shots are hand-blocked in `film.js`.

The panel's numbers are the stems' reports: measured F0 per gut, the current
note's target and cents, gas, supply pressure, digestion, and the section's
worst frame residual across its seven Worlds.
