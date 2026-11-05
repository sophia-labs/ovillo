# u20-ritardando diary (sections 26-m340 to 43-m357)

Eighteen sections, one bar each, slowing from 138 to 63 bpm (26-m340 at 138, 33-m347 at 101 with 3 beats, 43-m357 at 63). Sections 26-m340 to 32-m346 are 4 beats (8 eighths per part); 33-m347 to 43-m357 are 3 beats (6 eighths per part). Parts: v1 Flauto I, v2 Flauto II, v3 Oboi I, v4 Oboi II, v5 Clarinetto in B♭ I, v6 Clarinetto in B♭ II (v5 and v6 are the two Clarinetti in B♭ throughout; the baseline set the parts, and I kept them). The cannon is a rest in all eighteen: the source has no cannon shot in these sections (the cannon sources are in 24-m320, 46-m380 and 47-m396).

Every number below is from a render report or an analyze report. Renders are cached when a part is unchanged, so a re-read is not a new render.

## Pass 0: the baseline, and what I meant

**What I meant.** The baseline lines are the top lines of the six busiest parts, folded into 36 to 300 Hz. They are the right tune: the eighth-note figure of the source, one octave per voice register, the bass at the bottom. I kept every pitch the baseline gave me, except that D1 and Eb1 (10 notes in all) I moved up an octave (see pass 1). Nothing else in the music changes; the ritardando is the slowing of the tempo, which the source gives me.

**What I measured at the baseline.** Only section 26 was rendered at baseline (naive tensions = the targets' own frequencies, tongued, nothing eaten). Section 26-m340, render 1 (cached baseline):
- v1 2/8 in tune (50 c), mean 89.1 c, mean attack pop 26.5 dB
- v2 0/8 in tune, mean 220.4 c, sharpness 9.2
- v3 0/8, same as v2; v4, v5, v6 0/8 each, mean 165.0 c, sharpness 11.5
- No note started dry; gas at end 0.908 (v1) to 0.394 (v4 to v6).
So 2 of 48 sounding notes were in tune at the baseline. The baseline commanded the target frequencies, and the valve landed sharp of them (mean 89 to 220 cents off).

## Pass 1: tensions from the probes, slurred

**What I meant.** The same notes, each one landing on its target, with the valve's sharpness taken out of the command.

**What I probed.** Push 0.7 and cheeks 0.4 throughout (the baseline hands), with the tone the one thing I changed. A fine probe (window of plus or minus 12% around each target, 60 steps) gave, for each target, the tension at which the held note measured the target:
- Tone 0.4: G1 (49.0 Hz) at tension 50.9; Bb2 (116.5 Hz) at 109.9; D3 (146.8 Hz) at 141.9; Bb3 (233.1 Hz) at 232.6; C4 (261.6 Hz) at 265.5. The sharpness of a held note was 8.2 to 9.9 across 49 to 117 Hz and 4.1 to 4.9 from 196 to 262 Hz.
- Tone 0.6: F1 (43.7 Hz) at tension 46.1, sharpness 8.4. D4 (293.7 Hz) at 295.2, sharpness 4.2. At tone 0.4 the same D4 needs a tension of 300.6 Hz, past 300 Hz.
- Below 46 Hz the valve jumps. In the probe at tone 0.6 the held note went 39.9 to 29.5 Hz, then 40.4 to 36.1 Hz; at tone 0.4 46.0 to 36.3 Hz, then 46.2 to 42.1 Hz. D1 (36.7 Hz) and Eb1 (38.9 Hz) sit in that jumpy region, so I moved them up an octave, to D2 and Eb2 (10 notes). I did not try to tune them.

**What I set.** Tensions from the inversion for every target, tone 0.4 except F1 and D4 (tone 0.6, switched on for the note and back to 0.4 after). Tongue off: the baseline was tongued, and its attack pops measured 26.5 dB on v1 of section 26. The pops in the first tuned render of section 26 were 7.1 dB on v1 (the change of tension and of articulation came together, so I cannot say which one lowered them). Nothing fed, nothing digested in this pass.

**What the World measured (render 2, all eighteen sections, no mix).** In tune within 50 cents, per section:
- 26: 48 of 48; 27: 48 of 48; 28: 48 of 48; 32: 48 of 48; 29: 47 of 48 (v6 7 of 8); 30: 45 of 48; 31: 44 of 48 (v6 4 of 8)
- 33 to 38: 36 of 36 each; 39: 35 of 36 (v6 5 of 6); 40: 35 of 36 (v6); 41: 33 of 36; 42: 34 of 36 (v6 4 of 6); 43: 31 of 36 (v4 and v5 4 of 6 each)
- Pass 1 total: 712 of 732 sounding notes within 50 cents (97.3%). No silent note, no note started dry, no clipping.
- The misses are all in the low parts (v4 to v6, from 46 to 74 Hz) and they grow in the later sections. Their gas at the end was low: 42 v6 at 0.104, 41 v6 at 0.111, 43 v4 at 0.150 (pass 1 reports).

**What I heard.** The tune reads, the top voices sit in tune, and the low voices slide sharp then flat within a bar, the way a gut does when its reservoir goes down. The F1 notes after G1 read 25 to 96 cents sharp in those parts (31 v6, 42 v6). The gut is starving, not mistuned: the notes that miss are the ones late in each bar in the low parts.

## Pass 2: feeding and digestion

**What I set.** In the header of every part: `pre=1 eat=beans*12 digest=on`. Beans are 0.60 J each, so the stomach starts with 7.2 J and digestion pays the reservoir at 12 beans × 0.075 W = 0.9 W, under the 1 W cap. Digestion is on from the start, so the reservoir is refilled while it drains; the one second of pre-silence is trimmed, and nothing is paid while the reservoir is full. The tensions and tones are unchanged.

**What the World measured (render 3, all eighteen sections, no mix).**
- In tune: 730 of 732 (99.7%). Misses: 30 v6 (7 of 8) and 31 v6 (7 of 8). Every other part was all in tune.
- Gas at the end in the low parts: 0.76 (40 v6, the lowest), 0.80 (42 v6), 0.82 (41 v6), 0.85 (38 v6), 0.82 (29 v6). The upper parts sat at 0.97 to 0.99.
- No note started dry; no silent note; no clipping.
- Stomach left 4.7 to 6.8 J. I fed more than the parts used, which I did not test against a smaller feed; I kept the margin because the reservoir ran to 0.76 at its lowest with this feed.

**What I heard.** The late low notes that missed in pass 1 now land. The two that still miss are both in the same place: the G1 right after a leap down into the low register (30 v6, Ab1 after Eb2: +108 cents; 31 v6, G1 after D2: +98 cents), with the reservoir at 0.95 (30 v6) and 0.99 (31 v6). That is not starvation: the gut is full and the valve overshoots a large downward leap at this tension.

## Pass 3: two leap corrections

**What I set.** The two notes that overshot, lowered by the offset the analyze report showed on that note, and nothing else:
- 30-m344 v6, Ab1 after Eb2 (4th note): tension 53.1 → 50.0 (the report's measured 55.3 Hz at 53.1 is +2.2 Hz over the command; 50.0 should read about 52.2 Hz, +10 cents from Ab1 at 51.9 Hz).
- 31-m345 v6, G1 after D2 (5th note): tension 50.9 → 48.0 (the report's measured 51.9 Hz at 50.9 is +1.0 Hz over; 48.0 should read about 49.0 Hz).

**What the World measured (render 4 for these two sections, no mix).**
- 30-m344: v6 8 of 8 in tune, mean 19.5 c; v1 to v5 unchanged at 8 of 8.
- 31-m345: v6 8 of 8 in tune, mean 16.3 c; v1 to v5 unchanged at 8 of 8.
- Total after pass 3: 732 of 732 sounding notes within 50 cents (100%), no silent note, no note started dry, no clipping.
- Gas at the end: 0.885 (30 v6) and 0.964 (31 v6).

## Pass 4: the section mixes

**What I rendered.** Each section mixed once, then analyzed once. Section mixes (rms, sharpness and sharpest half-second):
- 26: rms -16.5 dB, sharpness 7.4, sharpest half second 9.1 at 1.5 s
- 27: -16.9, 7.5, 8.0 at 1.5 s
- 28: -15.0, 5.1, 5.7 at 1.5 s
- 29: -15.0, 7.2, 7.2 at 0.0 s
- 30: -13.5, 7.2, 12.0 at 2.0 s (the release tail; -19.7 dB)
- 31: -15.8, 6.1, 9.7 at 2.0 s (-20.9 dB)
- 32: -15.9, 6.7, 8.6 at 2.0 s
- 33: -16.8, 7.6, 9.8 at 1.5 s
- 34: -16.2, 6.9, 7.4 at 1.5 s
- 35: -16.3, 7.2, 7.6 at 1.5 s
- 36: -13.9, 5.4, 5.9 at 0.0 s
- 37: -15.8, 5.2, 7.1 at 2.0 s (-20.2 dB)
- 38: -14.6, 6.7, 9.7 at 2.0 s (-17.5 dB)
- 39: -14.9, 6.9, 7.7 at 2.0 s
- 40: -14.2, 7.0, 8.2 at 2.0 s
- 41: -13.8, 7.0, 12.2 at 2.5 s (-20.2 dB, the release tail)
- 42: -15.3, 5.9, 9.0 at 2.5 s
- 43: -15.3, 6.0, 8.1 at 1.0 s
- No section clipped. The mix sharpness is 5.1 to 7.6 across the unit. The sharpest half second in each section ranges 5.7 to 12.2, at -13 to -21 dB, and most of those sit at the end of the bar (the release tail), not in the attacks.

**What I heard.** The mixes are clean: no clipping, sharpness in the band of a held note, and the section centroids run 492 Hz (30) to 753 Hz (42) with no single trend across the slowing. I did not change anything after the mix; the in-tune count was already 732 of 732, so I stopped rather than spend a render on the mix's tail.

## What I changed and what I would still change

- Changed: tensions (probe inversion, every note), tone 0.6 for F1 and D4, tongue off everywhere, D1 and Eb1 moved up an octave (10 notes), feeding `pre=1 eat=beans*12 digest=on` in all parts, two leap corrections (30 v6, 31 v6).
- Not changed: the pitches (except the 10 octave-ups), the beats, the cannon (rests, since the source has no shot in these sections), push 0.7, cheeks 0.4.
- Not tested: a smaller feed (the stomach left 4.7 to 6.8 J), or a tongued version at these tensions (the baseline tongued parts clipped in an earlier unit; I did not render a tongued version here).
- F1 (43.7 Hz, 14 notes in the unit) is the note at the edge: it sits at tension 46.1 on tone 0.6, beside the lower jump in the probe. A dry start there would drop it an octave. No note started dry in any render, and every F1 in the final renders is within 50 cents.
- Renders: section 26 used 4 render commands (baseline, pass 1, pass 2, mix). Sections 30 and 31 used 4 (pass 1, pass 2, pass 3, mix). The other sixteen used 3 (pass 1, pass 2, mix). Re-reads of an unchanged render were cached and are not counted.

## Notes for reviewers

- The totals to check: 732 sounding notes, 732 within 50 cents at the end; 0 silent, 0 started dry, 0 clipped in every final render.
- The octave-ups (D1 to D2, Eb1 to Eb2) are the one place I changed the notes the baseline gave me. If the score wants them at their folded pitch, the valve cannot tune them; say so and I will thin or keep them as octave-ups.
- The tone 0.6 notes (F1, D4) switch the tone for one note each and come back to 0.4. A reviewer who wants one tone for the whole part should know the F1 tension at 0.4 (46.6 Hz by the inversion) sits beside a jump at 46.0 Hz.

## Retune (fart_v1)

**What I meant.** The same eighteen sections, the same tune and beats, slowing from 138 to 63 bpm. Parts v1 and v2 (Flauto I and II, soprano valve), v3 and v4 (Oboi, alto), v5 and v6 (Clarinetti, bass). The cannon stays a rest. Nothing in the music changed; the instrument did, and the parts had to land on it.

**What the new instrument did to them (as found, cached render reports, no mix).** 478 of 732 sounding notes within 50 cents (65.3%), down from 732 of 732 at the end of the first run. No silent note, no note started dry, no clipping. Section 26: v1 5 of 8 in tune, v5 and v6 4 of 8 (mean 51.4 c); section 36: 22 of 36; section 41: 20 of 36. The misses were almost all flat: the analysis for section 26 v5 gives Eb2 (target 77.8 Hz) at tension 74.2 measured 72.5 Hz (-121 cents). Soprano D4 (target 293.7) at tension 295.2 read 286.3 Hz (-44 cents) in 26 v1.

**What I checked before I changed anything.** I probed each anatomy at the settings the parts use (push 0.7, cheeks 0.4, tone 0.4). The probe (`probe --score scores/fart/1812 --voice v5 --push 0.7 --cheeks 0.4 --tone 0.4`) reads tension 74.2 Hz as 75.8 Hz, which would put Eb2 46 cents flat of 77.8 Hz; the render read the same tension 121 cents flat. The probe holds each note 0.6 s and reads its median from 0.25 s on; my notes are 0.22 s (a half beat at 138 bpm), so the valve has not settled in them. A probe inversion therefore does not predict these notes, and I set the tensions from the render's own per-note measurements instead (the analyze tables, every occurrence). The probe at tone 0.6 reached D4 (293.7 Hz) only at the top of the valve's range (tension 300.0 read 293.2 Hz), and the render's pass-1 tensions for D4 ran past the 300 Hz limit (301.0 to 310.2 Hz, refused in 8 parts).

**Pass 1 (tensions from the measured notes, tongue off, feeding unchanged).** Each occurrence's tension multiplied by target / measured (the render's measured F0 for that note), every note of the eighteen sections, 726 tensions changed. Result (render 2, no mix): 674 of 678 in tune in the parts that rendered. Eight soprano parts did not render: their D4 at tensions 301.0 to 310.2 Hz is past the valve's 300 Hz limit. Those eight are sections 26, 27, 32, 33, 34, 35 and 36 (v1) and 36 (v2).

**Pass 2 (render 3, no mix).** Residuals beyond 15 cents corrected by 0.8 of the same ratio (capped at 8%; 70 tensions changed); D4 in the soprano parts changed to tone 0.75 at tension 289.0 (the tone switch is for that one note; the tone goes back to 0.4 after it). The first note of a bass part is a fresh gut and jumped: Ab1 commanded at 54.7 Hz read 82.4 Hz (+799 cents) in 30 v6, 59.6 Hz (+239) in 38 v6 and 57.7 Hz (+183) in 41 v6, so I set those three first notes back to 53.1 Hz, where the render measured 50.4 Hz (-51 cents). The first G1 in 31 v6 read 51.1 Hz (+73) at 53.5 Hz, so I set it to 52.3 Hz. Result: 726 of 732 in tune (99.2%). The soprano D4 at 289.0 Hz read 282.7 to 294.7 Hz (-66 to +6 cents) over its thirteen occurrences, three of them past -50.

**Pass 3 (the final render, with the mix).** Soprano D4 raised to tension 294.0 Hz at tone 0.75 (13 notes in v1 and v2). Result: 729 of 732 sounding notes within 50 cents (99.6%), 26, 27, 28, 29, 31 and 32 at 48 of 48; 30 at 47 of 48; 33 to 37, 39, 40, 42 and 43 at full; 38 and 41 at 35 of 36. No silent note, no note started dry, no clipping. Gas at the end of the lowest part was 0.917 (40 v6); the stomach left at least 4.84 J (43 v6). Feeding stayed `pre=1 eat=beans*12 digest=on` throughout: push 0.7 and cheeks 0.4 never changed.

**The three misses.** The first Ab1 in 30 v6, 38 v6 and 41 v6, at 50.4 Hz against Ab1 (51.9 Hz): -51, -53 and -51 cents. Commanding 54.7 Hz jumped the valve (see pass 2); I did not test any tension between 53.1 and 54.7 Hz, and I had no render left to search it.

**Mix (analyze, no render).** Section mix sharpness 4.6 to 5.9 across the unit, no clipping. The sharpest half second in each section runs 4.7 to 12.5; the highest are 31 (12.5 at 2.0 s, -16.1 dB), 41 (12.0 at 2.5 s, -24.0 dB), 30 (11.7 at 2.0 s, -21.1 dB) and 37 (10.7 at 2.0 s). Mix levels run -11.9 dB (38) to -18.0 dB (41).

**Renders used.** Four per section: the as-found no-mix (cached), pass 1, pass 2, and the final with mix. No section needed a fifth.

**Reviewers.** The three first-note Ab1 misses are the one thing I would change if I had a render left: on a fresh start 54.7 Hz jumped and 53.1 Hz did not. The soprano D4 is now at tone 0.75 on a switched note; a reviewer who wants one tone for the whole soprano line should know that at tone 0.6 the D4 needed more than the valve's 300 Hz, and I did not test D4 at tone 0.4 on this valve.
