from tab import Guitar
from mk import bar
g = Guitar("B1 E2 A2 D3 F#3 B3"); n, c = g.n, g.c
B = 32
L = []
L.append(bar(1, {"gt.p": ["{mp}", n(6,0,"w")], "gt.v": ["{mp}", "Rh", n(2,0,"q"), n(1,0,"q")]}, "chord=Bm", B))
L.append(bar(2, {"gt.p": "-", "gt.v": [n(1,3,"q."), n(1,2,"e"), n(1,0,"h")]}, "chord=Bm", B))
L.append(bar(3, {"gt.p": [n(5,3,"w")], "gt.v": [n(1,3,"q"), n(1,5,"q"), n(1,7,"q."), n(1,5,"e")]}, "chord=G", B))
L.append(bar(4, {"gt.p": "-", "gt.v": [n(1,3,"h."), "Rq"]}, "chord=G", B))
L.append(bar(5, {"gt.p": [n(5,0,"w")], "gt.v": ["{mf}", n(1,8,"q."), n(1,7,"e"), n(1,5,"q"), n(1,3,"q")]}, "chord=Em", B))
L.append(bar(6, {"gt.p": [n(5,0,"h"), n(5,2,"h","sl")], "gt.v": [n(1,2,"h"), n(2,4,"h")]}, "chord=F#", B))
L.append(bar(7, {"gt.p": [n(6,0,"w")], "gt.v": ["{mp}", n(1,0,"h"), n(1,3,"q"), n(1,7,"q")]}, "chord=Bm", B))
L.append(bar(8, {"gt.p": [n(5,3,"h"), n(5,2,"h","sl")], "gt.v": [n(1,5,"h"), n(1,2,"h")]}, "chord=G|F#", B))
L.append(bar(9, {"gt.p": [n(6,0,"h"), n(6,3,"q","sl"), n(6,0,"q","sl")], "gt.v": ["{p}", c([(3,0),(2,0),(1,0)], "w")]}, "chord=Bm", B))
L.append(bar(10, {"gt.p": [n(6,0,"h"), n(6,5,"q","sl"), n(6,0,"q","sl")], "gt.v": "-"}, "chord=Bm", B))
L.append(bar(11, {"gt.p": "-", "gt.v": "-"}))
L.append(bar(12, {"gt.p": "-", "gt.v": "-"}))
head = """# score: Bajo la casa
# summary: 12 bars / 1 guitar, 2 hands / ~0.8 min — a baritone in B minor, soft fingertips, the low string sliding under the floor
# tempo: 60    time: 4/4    key: Bm    bars: 12
# reverb: 2.0
# master: rms=-18
# tab: B1 E2 A2 D3 F#3 B3 (baritone, a fourth below standard)

# text @b1: i - VI - iv - V on a guitar tuned a fourth low, plucked over the soundhole with the soft of the finger
# text @b6: the bass slides a whole tone; the string carries its energy along
# text @b9: the low B slides up and back, twice, under a held chord

# instruments:
#   gt   = steel-string guitar (treble, -12, anatomy=steel-string-v0, tuning=B1 E2 A2 D3 F#3 B3, capo=0)
#   gt.p = thumb, soft (treble, pos=0.24, pick=0.15, gain=2)
#   gt.v = melody, fingertips (treble, pos=0.26, pick=0.2)
"""
open("bajo.mus", "w").write(head + "\n" + "\n".join(L) + "\n")
