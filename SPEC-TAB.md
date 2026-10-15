# MUS-T: annotated tab

*Draft 0.1, 2026-10-14. Answers the "Tablature / fret notation" open question in [`SPEC.md`](SPEC.md). Its first instrument is the Ariadne steel-string ([`SPEC-ARIADNE-STEEL-STRING.md`](SPEC-ARIADNE-STEEL-STRING.md)).*

A MUS-T score is an ordinary MUS score in which every fretted-instrument note
also says **where on the instrument it is played and by which hand**. The
pitch stays: MUS pitches are absolute, and the tab position is an annotation
that must agree with it. A reader gets both readings at once. An LLM reasons
about harmony from the pitches, a guitarist reads string and fret, and a
physical renderer gets a contact on a string. A checker refuses a score where
the two disagree.

```text
A3e@3:2[ho]
│ │ │ │  └── how: hammered on (no new pluck)
│ │ │ └───── fret 2
│ │ └─────── string 3 (counted from the highest, as guitarists do)
│ └───────── an eighth
└─────────── sounding pitch: A3
```

## 1. The position suffix: `@string:fret`

- Grammar: `<pitch><duration>@<string>:<fret>[<flags/params>]`. The suffix comes
  after the duration, including ties (`E4h~e@1:0`), and before any `[...]`.
- `string` is 1-based from the highest-pitched course: on a guitar, `1` is the
  high E and `6` the low E. This is the guitarist's convention and the one tab uses.
- `fret` is an integer from 0 (open) up. A capo moves fret 0: with `capo=2`,
  `@3:0` means the third string stopped at the capo.
- In chords, each member carries its own position: `<A3@3:2 C4@2:1 E4@1:0>q`.
  Either every member has one or none does.
- **Agreement rule.** `midi(pitch) == open_midi(string) + capo + fret`. For a
  natural harmonic (`[harm]`), add 12 at fret 12, 19 at fret 7, and 24 at fret 5.
  A disagreement is an error, not a warning.
- No other MUS token uses `@` outside `[...]`. Trajectory knots use `@` only
  inside brackets, so the suffix is unambiguous.

## 2. Tuning and capo: the instrument line

```text
# tab: standard
# instruments:
#   gt   = steel-string guitar (treble, -12, anatomy=steel-string-v0, tuning=E2 A2 D3 G3 B3 E4, capo=0)
```

- `tuning=` lists the open courses **low to high**, as the instrument's
  strings are physically ordered. String numbers count from the other end, high to low.
- `-12` keeps the guitar's engraving convention: written an octave above sounding pitch.
- `# tab: standard` is a header hint for readers and engravers. The
  instrument line is what the checker and renderers obey.

## 3. Hands are voices

The guitar is one instrument with several hands, written with SPEC's dot
notation for multi-hand instruments:

```text
#   gt.p = thumb, thumbpick (treble, pos=0.10, pick=0.85, pm=0.45)
#   gt.v = melody, fingers (treble, pos=0.20, pick=0.5)
#   gt.r = fills, fingers (treble, pos=0.17, pick=0.4, gain=-6)
```

- `gt.p`, the thumb (*pulgar*), carries the bass. `gt.v`, the *voz*, carries the
  melody. `gt.r`, the *relleno*, carries fills and inner notes. Further
  dot-voices are allowed. These names are conventions; what matters is that
  every `gt.*` track plays the **same physical instrument**.
- Each hand's line declares how it touches the strings:
  - `pos` is the pluck point, as a fraction of the speaking length measured from the bridge.
  - `pick` is contact hardness, from fingertip `0` to plectrum `1`.
  - `pm` is how far the palm rests on the strings this hand plays, from `0` to `1`.
- Engraved, these are the usual stems-up/stems-down voices of fingerstyle
  notation. Physically, they are hands meeting strings at different places.
- Dynamics are per voice (`{p}` on fills, `{mf}` on melody). This is how a
  fingerstyle player balances a melody over its accompaniment.

## 4. Technique flags

| flag | meaning on the instrument |
|---|---|
| `ho` | hammer-on: the left hand stops the already-sounding string at the new fret; no pluck |
| `po` | pull-off: the left hand leaves a fret on a sounding string and plucks it in leaving |
| `sl` | slide: the stopping finger travels from the previous fret on this string to this one |
| `pm` / `pm=0.6` | palm mute for this note only (a hand's `pm` sets its default) |
| `x` | dead note: fretting hand touching, not stopping |
| `harm` | natural harmonic at the notated fret |
| `ring` | keep sounding past the notated duration until the string is used again |
| `acc`, `fer`, ... | everything in SPEC.md keeps its meaning |

The flags `ho`, `po` and `sl` need a sounding note on the same string just
before, in any voice of the same instrument. The checker enforces this.
`slide` (schleifer) and `->` (glissando) keep their SPEC.md meanings; `sl` is
the guitar slide.

## 5. Time: duration is how long the hand lets it ring

- A note's duration is how long the fretting finger stays down, or how long the
  open string is left alone. When it ends, the string is damped.
  - If the next event on the same string comes first, that event takes the string over.
  - `[ring]` keeps a note sounding until its string is next used.
  - A renderer that cannot damp yet must say so. The v0 steel-string lets every note ring until its string is next touched; see its limits.
- Swing comes from SPEC-AUDIO's header: `# swing: 8 64` delays every off-beat eighth.
  - A 16th inside a swung pair is placed proportionally inside its half of the pair.
  - In other words, the swing is a piecewise-linear warp of the beat, so ornaments stay inside it.
- Chord symbols are bar metadata, not notes: `bar 5 [chord=Am|E7]: ...`.
  - `|` splits the bar into equal parts.
  - They are for readers, engravers, and the checker's harmony report.

## 6. What a checker verifies

1. Every `gt.*` note has a position, and the position agrees with the pitch (§1).
2. No string is plucked twice at the same instant, across all hands of the instrument.
3. `ho`, `po` and `sl` land on a string that is sounding, and `sl` names a different fret.
4. Fretting-hand reach: at every instant, the fretted notes that must be held span at most four frets.
   - Spans of five or more are errors; four is reported as a stretch.
5. Each voice's durations add up to the bar's length.

## 7. A bar

```text
bar 5 [chord=Am|E7]: gt.p=A2q@5:0 E3q@4:2 E2q@6:0 D3q@4:0  gt.v=E4h~e@1:0 D4e@2:3 E4e@1:0 D4e@2:3  gt.r={p} Re C4e@2:1 Re A3e@3:2 G#3h@3:1
```

The thumb walks the bass on every beat. The melody holds an open high E,
then turns D–E–D on the second string. Under the held E, the fills touch C
and A, then hold the G♯ of the E7.
