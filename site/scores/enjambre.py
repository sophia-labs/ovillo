from tab import Guitar
from mk import bar
g = Guitar("D3 D#3 E3 F3 F#3 G3"); n, c = g.n, g.c
B = 32
OFF = {5: 0, 3: 0, 2: 1, 1: 3}          # the moving shape; strings 6 and 4 stay open
last = {}                                 # string -> last fret sounded
def fret(s, x):
    return OFF[s] + x if (s in OFF and x is not None) else 0
def note(s, x, d, slide_ok, dyn=None):
    f = fret(s, x)
    flags = []
    if slide_ok and s in last and last[s] != f:
        flags.append("sl")
    last[s] = f
    return n(s, f, d, *flags)
def trem(strings, steps, xs, slide_every, offset=False):
    out = ["Rt"] if offset else []
    k = 0
    for i in range(steps):
        s = strings[i % len(strings)]
        d = "t" if (offset and i == steps - 1) else "s"
        out.append(note(s, xs[i], d, slide_ok=(k % slide_every == 0)))
        k += 1
    return out
L = []
L.append(bar(1, {"gt.p": ["{mf}", c([(6,0),(5,0),(4,0)], "w", "acc")], "gt.v": ["{mf}", c([(3,0),(2,0),(1,0)], "w", "acc")]}, "chord=cluster", B))
for s in (1,2,3,4,5,6): last[s] = 0
L.append(bar(2, {"gt.p": ["Rw"], "gt.v": ["Rw"]}, "", B))
# 3-4: the swarm on open strings
for b in (3, 4):
    dyn = ["{pp}"] if b == 3 else ["{p}"]
    L.append(bar(b, {"gt.p": dyn + trem([6,4,5], 16, [None]*16, 99, offset=True), "gt.v": dyn + trem([1,3,2], 16, [None]*16, 99)}, "chord=cluster", B))
# put the shape on at x=0 (bar 5 start), then slide it up to x=9 over bars 5-8
xs_all = [min(9, i * 10 // 64) for i in range(64)]
for j, b in enumerate((5, 6, 7, 8)):
    xs = xs_all[16*j:16*j+16]
    dyn = ["{mf}"] if b == 5 else (["{f}"] if b == 7 else [])
    L.append(bar(b, {"gt.p": dyn + trem([6,4,5], 16, xs, 2, offset=True), "gt.v": dyn + trem([1,3,2], 16, xs, 2)}, f"chord=shape+{xs[0]}..{xs[-1]}", B))
# 9-10: arrived: C add9 swarm, then thinning
for b in (9, 10):
    dyn = ["{ff}"] if b == 9 else ["{>}"]
    L.append(bar(b, {"gt.p": dyn + trem([6,4,5], 16, [9]*16, 99, offset=True), "gt.v": dyn + trem([1,3,2], 16, [9]*16, 99)}, "chord=Cadd9", B))
# 11: one slow roll of the chord; 12 let it ring
L.append(bar(11, {"gt.p": ["{mp}", n(6,0,"e"), n(5,9,"e"), n(4,0,"e"), "Re", "Rh"], "gt.v": ["{mp}", "Rq", n(3,9,"e"), n(2,10,"e"), n(1,12,"h")]}, "chord=Cadd9", B))
L.append(bar(12, {"gt.p": ["Rw"], "gt.v": ["Rw"]}, "", B))
# 13-14: the top string slides home to the cluster, one fret at a time, no new plucks
v13 = ["{mf}", n(1, 12, "e", "acc")] + [n(1, f, "e", "sl") for f in (11,10,9,8,7,6,5)]
v14 = [n(1, f, "e", "sl") for f in (4,3,2,1)] + [n(1, 0, "h", "sl")]
L.append(bar(13, {"gt.p": ["Rw"], "gt.v": v13}, "", B))
L.append(bar(14, {"gt.p": ["Rw"], "gt.v": v14}, "", B))
L.append(bar(15, {"gt.p": ["{pp}", c([(6,0),(5,0),(4,0)], "w")], "gt.v": ["{pp}", c([(3,0),(2,0),(1,0)], "w")]}, "chord=cluster", B))
L.append(bar(16, {"gt.p": "-", "gt.v": "-"}))
L.append(bar(17, {"gt.p": "-", "gt.v": "-"}))
head = """# score: Enjambre
# summary: 17 bars / 1 guitar, 2 hands / ~0.6 min — a guitar tuned to six adjacent semitones; the swarm opens into C add9 and closes again
# tempo: 120    time: 4/4    key: C    bars: 17
# reverb: 1.0
# master: rms=-17
# tab: D3 D#3 E3 F3 F#3 G3 (a semitone cluster)

# text @b1: every open string is a semitone from the next: the cluster, struck once, left to ring
# text @b3: two plectrums at the bridge, offset by a 32nd: the swarm
# text @b5: one fretting shape (the 2nd string a fret ahead, the 1st three) slides up nine frets; every other change is a slide, not a pluck
# text @b9: arrived: D3 C4 E3 D4 E4 G4, C add9
# text @b13: the first string slides home fret by fret, without a new pluck

# instruments:
#   gt   = steel-string guitar (treble, -12, anatomy=steel-string-v0, tuning=D3 D#3 E3 F3 F#3 G3, capo=0)
#   gt.p = plectrum, low three (treble, pos=0.04, pick=1.0, gain=-2)
#   gt.v = plectrum, high three (treble, pos=0.05, pick=1.0)
"""
open("enjambre.mus", "w").write(head + "\n" + "\n".join(L) + "\n")
print(L[4][:300])
