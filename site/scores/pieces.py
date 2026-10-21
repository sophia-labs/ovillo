import json, os, re, sys
# OVILLO_RENDERS: the directory the renders were written to (sounds/, scores/, bench/; site/README.md says how).
# OVILLO_CLONE: the checkout the repository pieces were rendered from (default $OVILLO_RENDERS/fresh/ovillo; on
# 2026-10-20 that was a fresh clone of e48d8f8). Writes $OVILLO_RENDERS/scores/pieces.json.
S = os.environ.get("OVILLO_RENDERS") or sys.exit("pieces.py: set OVILLO_RENDERS to the renders directory (site/README.md)")
F = os.environ.get("OVILLO_CLONE", S + "/fresh/ovillo"); SO = S + "/sounds/"; SC = S + "/scores/"; BE = S + "/bench/"
def mus(slug, num, title, group, tags, engine, note, score):
    return dict(slug=slug, title=title, group=group, tags=tags, engine=engine, note=note,
                wav=f"{SO}{num}.wav", score=score, receipt=f"{SO}{num}.json", energy=f"{SO}{num}.energy.json")
def bench(slug, title, group, tags, note, d, wav, score=None, patch=None):
    log = open(BE + d + "/log").read()
    m = re.search(r"closure residual (\S+)", log); lim = re.search(r"safety-limited frames (\d+)", log)
    rt = re.search(r"whole rig:\s+\S+ s\s+->\s+(\S+)x", log)
    b = {"closureResidual": m.group(1) if m else None, "safetyLimitedFrames": int(lim.group(1)) if lim else None,
         "patch": patch}
    return dict(slug=slug, title=title, group=group, tags=tags, engine="bench: simulated guitar → Ariadne World",
                note=note, wav=wav, score=score, bench=b)
P = [
 mus("hilo-de-luz", "10-hilo-de-luz", "Hilo de luz", "today", ["pretty"], "steel-string world · DADGAD",
     "A lullaby in 3/4 for three hands. The open strings of DADGAD rolled over a ringing low D, a melody with a hammer-on, then harmonics at the 12th and 7th frets (played as plucks at the harmonic's pitch, a v0 limit).", SC + "hilo_de_luz.mus"),
 mus("enjambre", "11-enjambre", "Enjambre", "today", ["weird"], "steel-string world · tuned to six adjacent semitones",
     "The guitar retuned D3 D♯3 E3 F3 F♯3 G3, a cluster, picked hard at the bridge by two offset plectrums. One fretting shape slides up nine frets and the swarm lands on C add9, then the top string slides home fret by fret with no new pluck.", SC + "enjambre.mus"),
 mus("pulgar-de-tierra", "12-pulgar", "Pulgar de tierra", "today", ["rhythmic"], "steel-string world · drop C",
     "A palm-muted thumb thumping near the bridge, chucks and a pentatonic lick with hammer-ons and pull-offs. The [x] dead notes are written but v0 plays them as ordinary plucks; the palm mute is real loss on every partial.", SC + "pulgar.mus"),
 mus("bajo-la-casa", "13-bajo", "Bajo la casa", "today", ["pretty", "dark"], "steel-string world · baritone tuning",
     "i–VI–iv–V in B minor on the same body tuned a fourth low, plucked with the soft of the finger. The low string slides under a held chord; a slide carries the string's energy along and the receipt bills the work.", SC + "bajo.mus"),
 mus("gigante-y-juguete", "17-gigante", "Gigante y juguete", "today", ["playful", "weird"], "two steel-string worlds",
     "One tune in canon on two guitar-shaped worlds: a giant tuned an octave down and a toy tuned a fourth up. A toy tuned a full octave up was refused by the engine: its 24th partial at the 12th fret would sit at 23 kHz, past the band.", SC + "gigante.mus"),
 bench("agua-mansa", "Agua mansa", "today", ["pretty"], "A written score for the bench's simulated guitarist (bends, vibrato, slow strums from Em9 to Bm7), played into the World with a patch of long sustain and a persistent shared space.", "agua", BE + "agua/agua-mansa-wet.wav", SC + "agua-mansa.json", "glass"),
 bench("toward-the-bore", "Toward the brass bore", "today", ["weird", "loud"], "The AMBIENT guitarist with the configuration map pushed all the way from the six-course string body to the B♭ brass bore. The World goes hot: 22,863 safety-limited frames, and the runaway guard fades it to rest and back in. Turn it down.", "brass", BE + "brass/ambient-wet.wav", None, "material_organization = 1.0"),
 bench("hungry-world", "Hungry world", "today", ["weird"], "The RIFF pattern into a World with hunger, metabolism, reciprocity and a shared space all turned up.", "hungry", BE + "hungry/riff-wet.wav", None, "hunger 1.0, metabolism 1.0, reciprocity 0.5, shared space 0.7"),
 bench("halfway", "Halfway", "today", ["pretty", "weird"], "ARPEGGIO with the map halfway between string body and brass bore, and more sustain.", "midway", BE + "midway/arpeggio-wet.wav", None, "material_organization = 0.5, sustain 0.6"),
 mus("steel-string-tour", "00-tour", "Steel-string tour", "repo", ["acceptance"], "steel-string world",
     "The repository's acceptance score for the steel-string: open strings left to ring, a hammer-on, a pull-off, a slide, the palm-muted thumb.", F + "/examples/tab/steel_string_tour.mus"),
 mus("wayfaring-stranger", "01-wayfaring", "Wayfaring Stranger", "repo", ["pretty"], "steel-string world · after Doc Watson",
     "The traditional hymn (first printed 1858) as a full annotated tab for thumb, melody and fills.", F + "/examples/tab/wayfaring-stranger-doc.mus"),
 bench("ambient", "AMBIENT", "bench", ["pretty"], "The bench guitarist's ambient piece: drones, slowly rolled colour chords, high plucks with late vibrato, bars of silence for hearing the World ring. Neck pickup.", "ambient", BE + "ambient/ambient-wet.wav"),
 bench("travis", "TRAVIS", "bench", ["pretty"], "Alternating-bass fingerpicking in C and G. Middle pickup.", "travis", BE + "travis/travis-wet.wav"),
 bench("blues", "BLUES", "bench", ["rhythmic"], "A 12-bar shuffle in E or A, licks with curls and bends. Bridge pickup.", "blues", BE + "blues/blues-wet.wav"),
 bench("wander", "WANDER", "bench", ["pretty"], "A Markov chord progression that modulates now and then through the new key's V7.", "wander", BE + "wander/wander-wet.wav"),
 bench("melody", "MELODY", "bench", ["pretty"], "A slow single line in A or D minor, antecedent and consequent, over a soft bass.", "melody", BE + "melody/melody-wet.wav"),
 bench("first-light", "first light", "bench", ["pretty"], "The bench's example score: Em9, a high pluck with vibrato, a bend, a partial up-strum.", "score", BE + "score/first-light-wet.wav", F + "/ovillo/scores/first-light.json"),
]
for p in P:
    if p.get("bench") and p["bench"]["patch"] is None and p["group"] == "bench":
        p["bench"]["patch"] = "ovillo-default"
json.dump(P, open(SC + "pieces.json", "w"), indent=1, ensure_ascii=False)
print(len(P), "pieces")
