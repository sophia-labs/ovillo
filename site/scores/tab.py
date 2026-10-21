"""Spell MUS-T tokens from string/fret so pitch and position always agree.

    g = Guitar("D2 A2 D3 G3 A3 D4")
    g.n(1, 0, "q")            -> "D4q@1:0"
    g.n(2, 12, "q", "harm")   -> "A4q@2:12[harm]"
    g.c([(2, 0), (1, 2)], "h") -> "<A3@2:0 E4@1:2>h"
"""

NAMES = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"]
SHARP = {"C#": 1, "Db": 1, "D#": 3, "Eb": 3, "F#": 6, "Gb": 6, "G#": 8, "Ab": 8, "A#": 10, "Bb": 10}
NAT = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}
HARM = {12: 12, 7: 19, 5: 24}


def midi(p):
    name, octv = (p[:2], p[2:]) if len(p) > 2 and p[1] in "#b" else (p[:1], p[1:])
    pc = SHARP[name] if name in SHARP else NAT[name]
    return 12 * (int(octv) + 1) + pc


def spell(m):
    return f"{NAMES[m % 12]}{m // 12 - 1}"


class Guitar:
    def __init__(self, tuning, capo=0):
        self.low_to_high = [midi(p) for p in tuning.split()]
        self.capo = capo

    def pitch(self, s, f, harm=False):
        m = self.low_to_high[-s] + self.capo + f
        return m - f + HARM[f] if harm else m

    def n(self, s, f, d, *flags):
        harm = "harm" in flags
        fl = f"[{','.join(flags)}]" if flags else ""
        return f"{spell(self.pitch(s, f, harm))}{d}@{s}:{f}{fl}"

    def c(self, members, d, *flags):
        fl = f"[{','.join(flags)}]" if flags else ""
        inner = " ".join(f"{spell(self.pitch(s, f))}@{s}:{f}" for s, f in members)
        return f"<{inner}>{d}{fl}"
