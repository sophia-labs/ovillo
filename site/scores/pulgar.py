from tab import Guitar
from mk import bar
g = Guitar("C2 G2 C3 F3 A3 D4"); n, c = g.n, g.c
B = 32
groove = [n(6,0,"e"), n(6,0,"s","x"), n(6,0,"s"), n(5,0,"e"), n(6,0,"e"), "Re", n(6,0,"e"), n(5,3,"e"), n(5,0,"e")]
chuck = ["Rq", c([(3,0),(2,0),(1,0)], "e", "x"), "Re", "Rq", c([(3,0),(2,0),(1,0)], "e", "x"), "Re"]
lick = [n(2,3,"e"), n(1,1,"s"), n(1,3,"s","ho"), n(1,1,"e","po"), n(2,3,"e"), n(2,1,"e"), n(2,3,"e","ho"), n(3,2,"e"), "Re"]
lick2 = [n(1,5,"e"), n(1,3,"s"), n(1,5,"s","ho"), n(1,3,"e","po"), n(1,1,"e"), n(2,3,"e"), n(2,1,"e"), n(2,3,"e","ho"), "Re"]
thumb16 = [n(6,0,"s"), n(6,0,"s","x"), n(6,0,"s","x"), n(6,0,"s"), n(5,0,"s"), n(6,0,"s","x"), n(6,0,"s"), n(6,0,"s","x")] * 2
L = []
L.append(bar(1, {"gt.p": ["{mf}"] + groove, "gt.v": "-"}, "chord=C5", B))
L.append(bar(2, {"gt.p": groove, "gt.v": ["{mp}"] + chuck}, "chord=C5", B))
L.append(bar(3, {"gt.p": groove, "gt.v": chuck}, "chord=C5", B))
L.append(bar(4, {"gt.p": groove, "gt.v": chuck}, "chord=C5", B))
L.append(bar(5, {"gt.p": groove, "gt.v": ["{f}"] + lick}, "chord=Cm", B))
L.append(bar(6, {"gt.p": groove, "gt.v": chuck}, "chord=C5", B))
L.append(bar(7, {"gt.p": groove, "gt.v": lick2}, "chord=Cm", B))
L.append(bar(8, {"gt.p": groove, "gt.v": ["{mp}"] + chuck}, "chord=C5", B))
L.append(bar(9, {"gt.p": ["{f}"] + thumb16, "gt.v": "-"}, "chord=C5", B))
L.append(bar(10, {"gt.p": thumb16, "gt.v": ["Rh", "{mf}", c([(3,0),(2,0),(1,0)], "e", "x"), c([(3,0),(2,0),(1,0)], "e", "x"), "Rq"]}, "chord=C5", B))
L.append(bar(11, {"gt.p": groove, "gt.v": ["{f}"] + lick}, "chord=Cm", B))
L.append(bar(12, {"gt.p": [c([(6,0),(5,0),(4,0)], "h", "acc"), "Rh"], "gt.v": ["Rh", c([(3,0),(2,0),(1,0)], "h", "acc")]}, "chord=C5|Dm/F", B))
L.append(bar(13, {"gt.p": "-", "gt.v": "-"}))
head = """# score: Pulgar de tierra
# summary: 13 bars / 1 guitar, 2 hands / ~0.5 min — a thumb that thumps: drop-C, palm down, dead notes, chucks and a pentatonic lick
# tempo: 104    time: 4/4    key: Cm    bars: 13
# swing: 8 58
# reverb: 0.6
# master: rms=-16
# tab: C2 G2 C3 F3 A3 D4

# text @b1: the thumb rests its palm on the bass strings and plucks near the bridge with a hard thumbpick
# text @b2: the fingers chuck the top three strings, fretting hand touching, not stopping
# text @b5: a lick: hammer-on, pull-off, hammer-on
# text @b9: the thumb alone in sixteenths, half of them dead

# instruments:
#   gt   = steel-string guitar (treble, -12, anatomy=steel-string-v0, tuning=C2 G2 C3 F3 A3 D4, capo=0)
#   gt.p = thumb, palm down (treble, pos=0.07, pick=1.0, pm=0.85, gain=3)
#   gt.v = fingers, chucks and licks (treble, pos=0.18, pick=0.6)
"""
open("pulgar.mus", "w").write(head + "\n" + "\n".join(L) + "\n")
