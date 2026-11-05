# u01-chorale: sections 01-m1 and 02-m17

The Largo chorale as the valves can play it: four cellos and two violas, each a gut.
Sections 01-m1 (bars 1-16) and 02-m17 (bars 17-32), 60 bpm, 48 beats each.

## Pass 1: the hands, and a dry first render (01-m1)

**What I meant.** The lines are the tune and its inner parts, slow and sustained. The
parts should sound as written, one octave down where the baseline put them: cello 1 is
the top voice, so it has to sit above the others, and the baseline's uniform octave is
what keeps the order. Slurred (tongue off): these are chorale notes, not articulated
ones.

**What I checked before rendering.** Probes at the baseline hands (push 0.7, cheeks 0.4,
tone 0.75) land far sharp: at 85.2 Hz the valve measured 96.9 Hz (+223 cents), at 140.9
Hz it measured 160.2 Hz (+222 cents), at 233.2 Hz 244.9 Hz (+85 cents). At tone 0.4 the
offset is much smaller: 85.2 Hz gave 90.2 Hz (+100 cents), 233.2 Hz gave 233.6 Hz (+3
cents). So I took tone 0.4 as the first hand, and found the tension for each target
by a fine probe. Every target I checked then landed within about a cent of itself.

**What I set.** push 0.6, cheeks 0.4, tone 0.4, slurred. Tensions for each target note
from the probes. Check passed: 55/56/52/50/37/55 notes, 48.0 beats each.

**What the World measured (render 1, no mix).** Every part went dry early:

- v1 (cello 1): 44 of 55 notes started dry, 46 silent, gas at end 0.004.
- v2 (cello 2): 45 of 56 started dry, 47 silent, gas at end 0.004.
- v3 (cello 3): 43 of 52 started dry, 45 silent, gas at end 0.004.
- v4 (viola 1): 43 of 50 started dry, 43 silent, gas at end 0.004.
- v5 (viola 2): 30 of 37 started dry, 30 silent, gas at end 0.005.
- v6 (cello 4): 44 of 55 started dry, 44 silent, gas at end 0.004.

No clipping. Residuals 4.4e-16 J. In v1 the reservoir fraction fell from 1.0 at 2.0 s
to 0.217 at 7.0 s; the note at 7.0 s swung for 11% of its length, and the next notes did not swing at all. Reading the
gas column as a fraction of the 1.5 J reservoir, the valve drew about 0.25 W while it
sounded at push 0.6 (my arithmetic from the table). A reservoir that starts full runs
out in about 7 s at that rate, and the valve stops swinging near a fifth of full.

**A mistake in this pass.** My generator wrote the settings line from an earlier constant
(tone 0.4), while the tensions were solved at tone 0.6. So this render ran the parts at
tone 0.4 with tensions meant for tone 0.6. Some of the "NO SWING" notes are that
mismatch. The dry-out is still the main finding, because the gas column falls the same
way with or without it. I fixed the generator so the settings line and the probe
settings come from the same constants.

## Pass 2: less push, and food (01-m1 and 02-m17)

**What I changed.**

- Push from 0.6 to 0.4 (tone 0.6, cheeks 0.4). A probe at push 0.4, tone 0.6 still
  tunes the register: commanded Eb2 at 72 Hz, Ab2 at 95 Hz, Eb3 at 149 Hz, Ab3 at 203 Hz,
  C4 at 263 Hz, all within 0 cents of target. The lowest notes tune there too: commanded
  F1 at 45 Hz (+2 cents), G1 at 49 Hz (0 cents), Ab1 at 51 Hz (0 cents), Bb1 at 56 Hz (+1
  cent). I chose push 0.4 to cut the air the valve draws. I did not measure the draw at
  push 0.4 separately, so the saving is a hope until the renders show it; they did show
  no dry starts.
- Feeding: beans, four at a time (0.6 J each, 8 s to digest), at beats 0, 12, 24 and 36
  of each part, with digest on for the whole part. A part's reservoir is refilled while
  its food digests, so a note does not start dry.
- Octaves. The baseline's one-octave-down register stays. Where a note fell under about
  41 Hz at these settings (cello 4, 02-m17: four written Eb2 notes, which would sound at
  38.9 Hz), I raised it back to its written octave, Eb2 at 77.8 Hz (commanded 71.6 Hz).
  In my probes Eb1 (38.9 Hz) did not tune at push 0.6 to 0.9 (no swing, or -63 cents at push 0.7,
  tone 0.75), so the alternative was a smeared note.

**What the World measured, 01-m1 (render 2, no mix).**

| part | in tune | mean error (cents) | started dry | silent | gas at end | stomach left (J) | sharpness |
|---|---|---|---|---|---|---|---|
| v1 cello 1 | 56/56 | 0.3 | 0 | 0 | 0.997 | 6.511 | 4.3 |
| v2 cello 2 | 57/57 | 0.6 | 0 | 0 | 0.997 | 5.676 | 5.7 |
| v3 cello 3 | 52/52 | 0.5 | 0 | 0 | 0.996 | 4.419 | 7.0 |
| v4 viola 1 | 51/51 | 0.5 | 0 | 0 | 0.996 | 3.459 | 7.8 |
| v5 viola 2 | 38/38 | 0.4 | 0 | 0 | 0.993 | 3.129 | 9.0 |
| v6 cello 4 | 56/56 | 1.6 | 0 | 0 | 0.306 | 0.0 | 10.3 |

No clipping in any part. Residuals 4.4e-16 J. Mean attack pops (slurred notes) run 7.4 to
11.2 dB.

**What I heard in the numbers.** Every note landed. Cello 4 is the one to watch: its gas
ends at 0.306 with the stomach empty, so its last 12 seconds rely on the food that was
digesting. Its sharpness is 10.3, at the top of the held-note range the brief gives
(6.8 to 10.3). Its pitches are the lowest in the section (Ab1 51 Hz, Bb1 56 Hz), where a
tongued note would also pop more.

**What the World measured, 02-m17 (render 1, no mix, same push and feeding).**

| part | in tune | mean error (cents) | started dry | silent | gas at end | stomach left (J) | sharpness |
|---|---|---|---|---|---|---|---|
| v1 cello 1 | 31/31 | 0.1 | 0 | 0 | 0.994 | 7.246 | 4.3 |
| v2 cello 2 | 30/30 | 0.4 | 0 | 0 | 0.999 | 6.360 | 6.1 |
| v3 viola 1 | 28/28 | 0.8 | 0 | 0 | 0.995 | 5.715 | 6.4 |
| v4 viola 2 | 28/28 | 0.6 | 0 | 0 | 0.994 | 4.518 | 7.9 |
| v5 cello 3 | 28/28 | 0.9 | 0 | 0 | 0.997 | 4.724 | 7.8 |
| v6 cello 4 | 30/30 | 2.3 | 0 | 0 | 0.991 | 0.888 | 10.1 |

No clipping. Mean attack pops 3.6 to 7.9 dB.

**What I heard in the numbers.** Also every note landed, with no dry starts. The feeding is
generous: 16 beans were given to every part, and the stomach still holds 4.5 to 7.2 J in
five of the six parts, which means I fed more than they used. My arithmetic from the
tables gives the energy each part spent (food eaten minus food left, plus the change in
the reservoir): about 2.4 to 5.1 J for the five parts that kept food, and about 8.7 J for
cello 4. I did not trim the feeding, because cello 4 needs most of it, and I did not
want to spend a render on it.

## Pass 3: the mix (01-m1 and 02-m17)

**01-m1 mix.** rms -17.0 dB, sharpness 5.3 over the section. The sharpest half-second is
6.0 at 16.0 s and 14.0 s (-16.5 and -16.6 dB). That is a held-note level, not a click train.

**02-m17 mix.** rms -16.7 dB, sharpness 5.2 over the section. The sharpest half-second is
12.3 at 19.0 s, at -30.8 dB, and 12.0 at 8.0 s, also at -30.8 dB. Both are quiet moments,
probably the attack pops of a slurred note in the bass, where the sound is at its lowest.
They are above the 6.8 to 10.3 held-note range, but they are not a click train (a
click train at 85 Hz would be near 17).

**Open.** I did not change the feeding after pass 2, and I did not use the last render
on either section. Cello 4 in 01-m1 ends with an empty stomach and a reservoir at 0.306;
it has no margin if a part is longer. The attack pops on slurred notes are 4 to 11 dB,
which I accept as the cost of changing tension between notes without a clench.

## Review (second composer)

**What I found.** I compared every 01-m1 part with `source/01-m1` (cello 1 and cello 4 line by line; the rest by the check's note and beat counts). Melody (Eb-F-G-Ab in cello 1), bass (cello 4 on Eb/Ab/Bb/C) and register order are as the source writes them, one octave down. The first composer's reports, re-read from cache: 485/485 notes in tune, 0 dry, 0 silent, 0 clipped. The only weak spot was cello 4 in 01-m1: gas at end 0.306, stomach left 0.0 J, sharpness 10.3. 02-m17 is unchanged (mix sharpness 5.2; its sharpest half-seconds, 12.3 at 19.0 s and 12.0 at 8.0 s, are at -30.8 dB, quiet).

**What I changed.** 01-m1/v6 only: eat=beans*4 became eat=beans*5 at each of the four feedings (12.0 J eaten, was 9.6 J), to leave the bass a margin.

**Before and after (v6, 01-m1).** Gas at end 0.306 -> 0.992; stomach left 0.0 -> 1.324 J; in tune 56/56 both; mean error 1.6 c both; started dry 0, silent 0, clipped 0; sharpness 10.3 both. The extra beans did not change the pitch or the sharpness. 01-m1 mix: rms -17.0 dB, sharpness 5.3 before; after: rms -17.6 dB, sharpness 5.4, sharpest half-second 6.0 at 16.0 s (-17.1 dB). Renders used: one no-mix and one mix for 01-m1; 02-m17 rendered from cache.

**Not changed.** The slurred attack pops (3.6-13.3 dB) and cello 4's sharpness (10.3 and 10.1) stay; they are at the top of the brief's held-note range, and the settings that tune the register are the ones that cause them.

## Retune (fart_v1)

**What I meant.** The same chorale: the tune and its three inner voices, slurred, one octave
below the source, the bass as a bass. The music does not change in this pass; only the
tensions, the settings and the feeding do.

**What the new instrument did.** Each voice now plays its own anatomy: v1-v2 soprano,
v3-v4 alto (fart_v1), v5-v6 bass. The first render on the old tensions (push 0.4, cheeks
0.4, tone 0.6, fed as before) measured, in tune within 50 cents:

- 01-m1: v1 56/56, v2 54/57, v3 39/52, v4 23/51, v5 3/38, v6 9/56 (mean error 23.2, 27.0,
  42.9, 51.3, 66.1, 65.8 cents). Silent 0, started dry 0, clipped 0.
- 02-m17: v1 31/31, v2 26/30, v3 19/28, v4 14/28, v5 3/28, v6 3/30 (mean error 24.7, 32.2,
  42.1, 47.9, 63.3, 72.3 cents). Silent 0, started dry 0.

The soprano was nearly right (its anatomy is close to the old valve), the alto was
partly sharp in the low register, and the bass was far off: the bass valve sits about
90 to 120 cents sharp of its tension at tone 0.6 above about 60 Hz, and flat below it.

**What I set.** I probed each anatomy at push 0.4, cheeks 0.4 over the register its voice
plays (`probe --score`, 64 steps per band, every band joined into one table per voice and
tone). From each table I took, for every target, the tension where the smoothed measured
pitch crosses the target. Soprano (v1, v2) and alto (v3, v4) stay at tone 0.6: at tone 0.4
the soprano does not swing above a tension of about 140 Hz, and at 0.5 it is 50 cents
sharp to flat across its range. The bass moved:

- 01-m1 v5 and v6: tone 0.4. At this tone the bass lands within about 10 cents of target
  from 60 Hz up (probe: tension 73.95 for D2, 77.92 for Eb2), and its 40 to 50 Hz notes
  need tensions about 10 to 20 percent above target.
- 02-m17 v5 (cello 3, which reaches Bb2 to C3): tone 0.5. At tone 0.4 the bass stops
  swinging above a tension of about 115 Hz (the 120-130 Hz bins swing in 1 of 34 probe
  rows); at 0.5 every bin swings up to 140 Hz.
- 02-m17 v6 (cello 4): tone 0.6. At tone 0.4 the valve sits near a dead zone between
  tensions 44 and 48 Hz (probe rms -65 to -74 dB, sharpness 3 to 4), which is where F1 is
  (target 43.65 Hz). At 0.6 F1 lands at tension 46.0 (probe: 43.7 Hz, +3 cents).

Every note's tension is the solved value, not the old one. Examples: Eb2 (77.8 Hz) on
cello 4 was commanded at 71.6 Hz and now at 77.7 (01-m1, tone 0.4); Bb2 on cello 3 at
110.0 Hz and now at 114.6 (02-m17, tone 0.5).

**Starts after a rest.** Short notes after a rest did not start: in 01-m1 v5 the 1/2-beat
D2 after an r:1 measured -175 and -167 cents (swung 0.5 and 0.41, sharpness 12.5 and
14.3), and in 01-m1 v6 the first Eb2 (after r:2) measured -154 cents (rms -64.8 dB). In
02-m17 v6, three notes after an r:1/2 measured -164 to -204 cents. I raised push to 0.6 on
each note that follows a rest and dropped it to 0.4 after the note. That fixed every one
of them: 01-m1 v5 38/38 and v6 56/56; 02-m17 v5 28/28 and v6 25/30 (the rest of v6's
misses are the F1 notes below). No note started dry in any render.

**Feeding.** The bass ran short. Before the change, 01-m1 v6 ended with its gas at 0.485
and an empty stomach. I raised v6 to beans*6 at each of its four feedings (14.4 J, was 12.0
J) in both sections. v5 in 02-m17 went to beans*5 (12.0 J, was 9.6 J). The other voices
keep beans*4.

**What the World measured, final renders.**

01-m1 (final, with the mix; the parts are the same as the render before it):
- v1 56/56, v2 57/57, v3 52/52, v4 51/51, v5 38/38, v6 56/56: 310 of 310 in tune, mean error
  7.3, 7.9, 8.3, 7.7, 8.1, 11.2 cents. Silent 0, started dry 0, clipped 0.
- gas at end 0.997, 0.998, 0.993, 0.995, 0.989, 0.99; stomach left 7.14, 6.53, 4.99, 4.36,
  0.84, 1.54 J.
- Sharpness of the parts 4.2 to 7.7. Mix: rms -16.0 dB, sharpness 4.1, sharpest
  half-second 5.3 (at 2.0 s and 2.5 s, -17.9 and -17.3 dB).

02-m17 (final, with the mix):
- v1 31/31, v2 30/30, v3 28/28, v4 28/28, v5 28/28, v6 29/30: 174 of 175 in tune, mean error
  7.3, 8.9, 6.4, 6.6, 9.2, 13.2 cents. Silent 0, started dry 0, clipped 0.
- gas at end 0.997, 0.995, 0.999, 0.996, 0.992, 0.992; stomach left 7.70, 7.11, 6.14, 5.31,
  5.85, 6.06 J.
- The one miss is v6's first F1 (the 1/2 at 27.5 s, -81 cents, sharpness 10.3). The other
  F1 notes are within 15 cents, so I have left it.
- Mix: rms -16.4 dB, sharpness 5.2. Its sharpest half-seconds are 18.2 at 19.0 s (-30.0
  dB), 13.1 at 41.0 s (-28.7 dB) and 11.0 at 8.0 s (-29.5 dB). They are quiet, but 18.2 is
  above the held-note range and above the 17 of a click train at 85 Hz. I did not find
  the cause in a per-note table: at 19.0 s no part has a note starting, so it is probably
  the release of the notes that end at 18 s; that is a guess, not a measurement.

**What I heard in the numbers.** The valve's pitch depends on the tone and the register,
not just on the tension, so one tone per part was not enough for the bass. Two things
were worth more than the rest of the work: the rest-start fix, and the tone choice per
part. The soprano and alto needed no tone change. I did not change any tension by hand:
every value is from the probe tables above.

**Renders used.** 01-m1: four (three no-mix, one with the mix). 02-m17: four (three
no-mix, one with the mix). Cannon parts left as written (they are shots, not pitches).

**Not changed.** The alto and soprano keep tone 0.6 and push 0.4 (those were the old
settings). Bass tones now differ between the two sections (0.4 in 01-m1; 0.5 for v5 and
0.6 for v6 in 02-m17), so the same valve plays slightly different timbres across the two
sections. I accepted that to keep each part in tune; the director may want one tone
for the bass, and then the 02-m17 bass parts need a new probe.

### Review (retune)

Found: re-rendered both sections (cached, unchanged) and read the reports. 01-m1 was 310/310 in tune, silent 0, started dry 0, clipped 0. 02-m17 was 174/175; the one miss was v6's first F1 (line 8, `push=0.6 F1@46.3:1/2`, -81 cents). Melody (v1 cellos on the chorale line, F3 G3 Eb3 Ab3 ...) and the bass (v5/v6 an octave below, held roots) are where the ear expects them; I kept their tones, feeding and rest-start pushes.

Changed: one token. That F1 is the only note played at push 0.6 at that pitch, and I probed the bass at push 0.6, cheeks 0.4, tone 0.6: tension 47.1 Hz measured 44.2 Hz (F1+22), while 46.3 measured 42.2. So that single note went from F1@46.3 to F1@47.3 (the following F1 notes at push 0.4 stay at 46.3; they were within 15 cents).

After (02-m17 v6, no-mix render): 30/30 in tune, mean error 11.4 cents (was 13.2), silent 0, started dry 0, clipped 0. Section 02-m17 is now 175/175; 01-m1 unchanged at 310/310. Total 485/485.

Mix of 02-m17 (render with mix): rms -17.9 dB, sharpness 5.2. The quiet sharpness peaks are unchanged: 18.2 at 19.0 s (-31.6 dB), 13.0 at 41.0 s, 11.0 at 8.0 s. I did not find their cause and did not touch them: they are 30 dB down, and no part-level number points at a note. Bass tone differing by section is left, as the composer flagged; one bass tone would need new probes of every bass note.
