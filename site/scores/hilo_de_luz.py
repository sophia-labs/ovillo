from tab import Guitar
from mk import bar
g = Guitar("D2 A2 D3 G3 A3 D4"); n, c = g.n, g.c
B = 24  # 3/4 in 32nds
roll = lambda a, b, d: [n(*a, "e"), n(1, 0, "e"), n(3, 0, "e"), n(*a, "e"), n(1, 0, "e"), n(*b, "e")]
bass3 = [n(6, 0, "q"), n(5, 0, "q"), n(4, 0, "q")]
L = []
L.append(bar(1, {"gt.p": [n(6,0,"h.")], "gt.v": ["{mp}"] + [n(2,0,"e"), n(1,0,"e"), n(3,0,"e"), n(2,0,"e"), n(1,0,"e"), n(2,0,"e")], "gt.r": "-"}, "chord=D5", B))
L.append(bar(2, {"gt.p": [n(6,0,"h.")], "gt.v": [n(2,2,"e"), n(1,0,"e"), n(3,0,"e"), n(2,2,"e"), n(1,0,"e"), n(2,2,"e")], "gt.r": "-"}, "chord=G/D", B))
L.append(bar(3, {"gt.p": [n(5,2,"h.")], "gt.v": [n(2,0,"e"), n(1,0,"e"), n(4,4,"e"), n(2,0,"e"), n(1,0,"e"), n(3,0,"e")], "gt.r": "-"}, "chord=Bm7", B))
L.append(bar(4, {"gt.p": [n(5,0,"h.")], "gt.v": [n(2,0,"e"), n(1,2,"e"), n(3,2,"e"), n(2,4,"e"), n(1,2,"e"), n(2,4,"e")], "gt.r": "-"}, "chord=A", B))
L.append(bar(5, {"gt.p": bass3, "gt.v": ["{mf}", n(1,4,"q"), n(1,2,"e"), n(1,4,"e","ho"), n(1,7,"q")], "gt.r": "-"}, "chord=D", B))
L.append(bar(6, {"gt.p": bass3, "gt.v": [n(1,5,"q."), n(1,4,"e"), n(1,2,"q")], "gt.r": "-"}, "chord=G/D", B))
L.append(bar(7, {"gt.p": [n(5,2,"q"), n(4,4,"q"), n(5,2,"q")], "gt.v": [n(1,0,"q"), n(1,4,"q"), n(2,2,"q")], "gt.r": "-"}, "chord=Bm", B))
L.append(bar(8, {"gt.p": [n(5,0,"h.")], "gt.v": [c([(3,2),(2,4),(1,2)], "h.")], "gt.r": "-"}, "chord=A", B))
L.append(bar(9, {"gt.p": bass3, "gt.v": [n(1,4,"q"), n(1,2,"e"), n(1,4,"e","ho"), n(1,9,"q")], "gt.r": ["{p}", "Rh", n(2,12,"q","harm")]}, "chord=D", B))
L.append(bar(10, {"gt.p": bass3, "gt.v": [n(1,7,"q."), n(1,5,"e"), n(1,4,"q")], "gt.r": ["Rq", n(3,12,"q","harm"), "Rq"]}, "chord=G/D", B))
L.append(bar(11, {"gt.p": [n(4,2,"q"), n(5,0,"q"), n(4,2,"q")], "gt.v": [n(1,2,"q"), n(2,0,"q"), n(1,0,"q")], "gt.r": ["Rh", n(2,7,"q","harm")]}, "chord=Em7", B))
L.append(bar(12, {"gt.p": [n(5,0,"h.")], "gt.v": [n(1,0,"h.")], "gt.r": [n(3,0,"q"), n(2,0,"q"), "Rq"]}, "chord=A7sus4", B))
L.append(bar(13, {"gt.p": [n(6,12,"h.","harm")], "gt.v": ["{p}", n(4,12,"q","harm"), n(2,12,"q","harm"), n(1,12,"q","harm")], "gt.r": "-"}, "chord=D", B))
L.append(bar(14, {"gt.p": [n(5,12,"h.","harm")], "gt.v": [n(3,12,"q","harm"), n(2,7,"q","harm"), n(1,7,"q","harm")], "gt.r": "-"}, "chord=G", B))
L.append(bar(15, {"gt.p": [n(6,0,"h.")], "gt.v": ["{mp}", n(2,0,"e"), n(1,0,"e"), n(3,0,"e"), n(2,0,"e"), n(1,0,"e"), n(2,0,"e")], "gt.r": "-"}, "chord=D5", B))
L.append(bar(16, {"gt.p": [n(6,0,"h.")], "gt.v": ["{pp}", c([(3,0),(2,0),(1,0)], "h.")], "gt.r": "-"}, "chord=Dsus4", B))
L.append(bar(17, {"gt.p": "-", "gt.v": "-", "gt.r": "-"}))
L.append(bar(18, {"gt.p": "-", "gt.v": "-", "gt.r": "-"}))
head = """# score: Hilo de luz
# summary: 18 bars / 1 guitar, 3 hands / ~0.8 min — a lullaby in DADGAD; open strings left to ring, then harmonics
# tempo: 66    time: 3/4    key: D    bars: 18
# reverb: 1.4
# master: rms=-17
# tab: DADGAD

# text @b1: the open strings of DADGAD rolled under a ringing low D; every note rings until its string is touched again
# text @b5: a melody on the first string: hammer-on, then up to the 7th fret
# text @b9: a third hand adds harmonics at the 12th and 7th frets
# text @b13: only harmonics now, played as plucks at the harmonic's pitch (a v0 limit)

# instruments:
#   gt   = steel-string guitar (treble, -12, anatomy=steel-string-v0, tuning=D2 A2 D3 G3 A3 D4, capo=0)
#   gt.p = thumb (treble, pos=0.12, pick=0.55, gain=-2)
#   gt.v = melody, fingers (treble, pos=0.22, pick=0.35)
#   gt.r = harmonics, fingers (treble, pos=0.30, pick=0.3, gain=-3)
"""
open("hilo_de_luz.mus", "w").write(head + "\n" + "\n".join(L) + "\n")
print("\n".join(L[:3]))
