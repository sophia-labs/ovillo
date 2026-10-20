from tab import Guitar, midi, spell
from mk import bar
GG = "E1 A1 D2 G2 B2 E3"; JJ = "A2 D3 G3 C4 E4 A4"
gg, jj = Guitar(GG), Guitar(JJ)
def place(g, m):
    """lowest-fret position for a pitch, preferring the higher string on ties"""
    best = None
    for s in range(1, 7):
        f = m - g.low_to_high[-s]
        if 0 <= f <= 12 and (best is None or f < best[1]):
            best = (s, f)
    assert best, f"no place for {spell(m)}"
    return best
T = [("E4","q"),("G4","q"),("A4","q"),("B4","q"), ("B4","q"),("A4","q"),("G4","q"),("E4","q"),
     ("D4","q"),("E4","q"),("G4","q"),("A4","q"), ("B4","h."),("Rq",None),
     ("E5","q"),("D5","q"),("B4","q"),("A4","q"), ("G4","q"),("A4","q"),("B4","e"),("A4","e"),("G4","q"),
     ("E4","q"),("D4","q"),("E4","q"),("F#4","q"), ("E4","w")]
def line(g, shift):
    out, cur, bars = [], [], []
    acc = 0
    for p, d in T:
        if p == "Rq":
            cur.append("Rq"); acc += 8
        else:
            s, f = place(g, midi(p) + shift)
            cur.append(g.n(s, f, d)); acc += {"q": 8, "h.": 24, "e": 4, "w": 32}[d]
        if acc == 32:
            bars.append(cur); cur, acc = [], 0
    return bars
toy, giant = line(jj, 0), line(gg, -24)
L = []
for b in range(1, 11):
    t = toy[b-1] if b <= len(toy) else "-"
    gi = giant[b-2] if 2 <= b <= len(giant) + 1 else "-"
    if b == 1: t = ["{mf}"] + t
    if b == 2: gi = ["{f}"] + gi
    L.append(bar(b, {"jj.v": t, "gg.p": gi}, "", 32))
L.append(bar(11, {"jj.v": "-", "gg.p": "-"}))
head = f"""# score: Gigante y juguete
# summary: 11 bars / 2 guitars / ~0.4 min — one tune in canon on two guitar-shaped worlds four octaves apart: a giant tuned an octave down, a toy a fourth up (a requinto)
# tempo: 92    time: 4/4    key: Em    bars: 11
# reverb: 1.2
# master: rms=-17

# text @b1: the toy (tuned {JJ}) plays the tune where it is written
# text @b2: one bar later the giant (tuned {GG}) plays it two octaves below; same anatomy, same body, strings at other tensions

# instruments:
#   jj   = steel-string guitar (treble, -12, anatomy=steel-string-v0, tuning={JJ}, capo=0)
#   jj.v = the toy's fingers (treble, pos=0.10, pick=0.85)
#   gg   = steel-string guitar (treble, -12, anatomy=steel-string-v0, tuning={GG}, capo=0)
#   gg.p = the giant's thumb (treble, pos=0.16, pick=0.4, gain=4)
"""
open("gigante.mus", "w").write(head + "\n" + "\n".join(L) + "\n")
print(L[1][:200])
