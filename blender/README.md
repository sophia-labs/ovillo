# Seeing Ariadne play (Blender 4.5)

Three files describe a performance, and every one of them comes from the repo's own tools:

```bash
# 1. play the score on the world, keeping its energy books (every 10 ms: bodies, modes, flows, touches)
mkdir -p examples/tab/out
mus render examples/tab/wayfaring-stranger-doc.mus -o examples/tab/out/wayfaring-stranger-doc.wav \
  --energy examples/tab/out/wayfaring-stranger-doc.energy.json
# 2. the guitar's geometry and the top's 72 coupled mode shapes (shipped; regenerate after anatomy changes)
python -m mus_analysis.steel_string_anatomy --visual profiles/ariadne-steel-string-v0.visual.json
# 3a. build a .blend headless ...
blender --background --python blender/ariadne_scene.py -- --out examples/tab/out/wayfaring-stranger.blend
# 3b. ... or live: in Blender's Scripting workspace open blender/follow_along.py and Run Script.
#     It rebuilds the "Ariadne" scene whenever the builder, the geometry, the trace or the audio changes.
#     (Its OPTIONS at the top point at the render above; aim them at any other `--energy` output.)
```

Builder options (after `--`): `--fps 30`, `--modes 24` (how many top modes become shape keys),
`--flatten 0.6` (how much the high modes are lifted for the eye), `--seconds N` (build only the start).

## What you are looking at

| on screen | what drives it | invented? |
|---|---|---|
| the top flexing | the sum of its real coupled mode shapes, each at its real energy (displacement ∝ √(2E)/ω) | the rate: each mode swings at a slowed, log-compressed tempo, not its frequency; high modes are lifted by `--flatten`; the loudest swing is drawn at 20 mm |
| string glow and swing | each string body's stored energy, over its current speaking length | the swing's tempo |
| finger on a fret | the stop the tab names, as the world was told | — |
| pluck-point flashes | every touch (amber thumb, blue fingers), at the hand's pluck point | — |
| the warm pool round the bridge | energy per second crossing the bridge from all strings into the top | — |
| blue glow in the box | the air-dominated modes' share of the top's modal energy (A0 is 80 % air) | — |
| shells from the soundhole | onsets of radiated power, sized and faded by it | their shape: radiation here is monopole-weighted, not a measured field |
| the top's own glow, the room light | radiated power; the room field's stored energy | — |

Everything is on a log scale. A trace's summed flows equal the render receipt's ledger
(source work, radiation, internal loss), so the picture and the books are the same numbers.
