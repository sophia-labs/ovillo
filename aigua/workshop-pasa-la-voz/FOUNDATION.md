# Pasa la Voz — foundation

A chamber piece for five voices and a percussionist, all built on the
hilo (string network + one weave halo). The organizing idea: the melody
is a guest that never stays — each section hands it to the next player,
and in the final section the handoffs compress until the whole ensemble
is one distributed instrument. Genre: none. Chamber-folk, impressionist
color, minimalist patience, and whatever each musician brings.

Tempo 92, 4/4, Em modal center. 84 bars (~3:39).

## The theme (the guest)

The guide statement (bars 1-8, guitar):
  E4q F#4e G4e B4h | A4q. G4e F#4q E4q | G4q A4e B4e D5h | C5q B4e A4e B4h
  E5q. D5e B4q G4q | A4q B4e C5e B4q A4e G4e | F#4h E4q D4q | E4w
Every leader VARIES it — ornament, invert, stretch, fragment. Never
quote it verbatim after section A.

## Form and handoffs

| bars  | section | leader | support |
|-------|---------|--------|---------|
| 1-12  | A  la guitarra abre | gt states theme + 4-bar echo | hr sparse arpeggio bloom |
| 13-24 | B  el piano responde | pn takes theme, varied | gt comps; pd brushed entry |
| 25-36 | C  el arpa canta | hr lyrical inversion | pn counterline; am halo enters {pp} |
| 37-48 | D  el clavecin corre | hc ornamented, figurative; dialogue w/ pm bells | pd active; key brightens toward G |
| 49-60 | E  el aire | am foreground: theme stretched to long tones | pn + hr distant fragments; ps breath pulse |
| 61-76 | F  pasa la voz | 2-bar handoffs: gt→pn→hr→hc→pm(pitched!)→am, then 1-bar exchanges, building | everyone; pd drives |
| 77-84 | coda | gt reclaims the theme whole | all soft support; end Em9, let ring |

Harmonic skeleton (one chord/bar unless marked; voice freely):
A/B: Em Em C Am | Em C Bm7 Em | (repeat with color: Am9, Cmaj7)
C:   Am F C G | Am F Bm7sus Em
D:   G G Em C | G D/F# C G  (brighter, quicker harmonic surface ok)
E:   Cmaj7 | Am9 | suspended, slow — two chords per 4 bars
F:   2-bar cells cycling Em C Am Bm7, tension rising each cycle
coda: Em9 throughout.

## The expressiveness charter (the swarm of eyes)

This piece lives or dies on NOTE-TO-NOTE variation. Binding for every
musician:
1. Nearly every note carries its own event params: [gain=±0.5..5] shaped
   in phrase arcs (rise/fall), never flat, never sawtooth-random.
2. Attack sculpted per note where idiomatic: [pick=], [pos=] (piano:
   harder pick on accents; guitar: pos toward bridge for tension,
   soundhole for warmth; harp: pick softer at phrase ends).
3. Dynamics marks {pp}..{ff} at phrase level ON TOP of per-note gains.
4. Rhythmic life: mixed durations, syncopation, rests that breathe,
   grace-note ornaments (s/t durations), NO unbroken grids except as a
   deliberate effect (harpsichord may motor, but shaped).
5. MEASURE yourself: render with --stems, load your stem, report the
   std of per-second RMS and your count of per-note overrides. If your
   RMS contour is flat, revise before finishing.

## Protocol

- The living score: workshop-pasa/pasa_la_voz.mus. Edit ONLY your own
  instrument's tokens (and, sparingly, your own instrument declaration's
  params — you may retune your instrument, you may not touch another's).
- Append to RIFF-LOG.md: what you did, why, and one concrete request or
  offer to another musician ("harp: leave me the low register in F",
  "piano: I echo your bar-14 turn in bar 27 — lean into it").
- Round 1: lay your part for the whole form. Round 2: RIFF — react to
  what the others wrote and to the log; revise your part (handoffs
  especially: receive the melody the way the previous player releases
  it).
- Gates: `mus check` clean (real exit code), full render receipt
  (badTokens must be []), self-measurement reported. The engine is at
  mus-rs/target/release/mus; render with --base /Users/vera/dev/sophia/mus/aigua.
