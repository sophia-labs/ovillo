#!/usr/bin/env python3
"""Assemble the reviewed performance parts into an ordinary, editable MUS score."""
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
HARMONY = (
    "C6 Am7 Dm9 G13 "
    "C6 A7 Dm9 G13 Em7 A7 Dm9 G13 "
    "Fmaj7 Fm6 C6 A7 Dm9 G13 C6 G13 "
    "Am9 Em7 Fmaj7 C6 Dm9 Am9 D7 G13 "
    "C6 A7 Dm9 G13 Em7 A7 Dm9 G13 "
    "Fmaj7 Fm6 C6 A7 Dm9 G13 C6 G13 "
    "Fmaj7 G13 C6 C6"
).split()
SECTIONS = {
    1: "Shoes on the pavement [1-4]",
    5: "Sunskip [5-12]",
    13: "Around the corner [13-20]",
    21: "A little shade [21-28]",
    29: "Two steps ahead [29-36]",
    37: "The long way home [37-44]",
    45: "One more thing [45-48]",
}
DECLARATIONS = """# instruments:
#   sw = Sunwire paired-course lead (treble, anatomy=sunwire.plan.json, tuning=D3 D3 G3 G3 B3 B3 E4 E4, gain=-5)
#   sw.v = melodic hand (treble, pos=0.105, pick=0.68, pm=0.035)
#   tb = Tumbler bass (bass, anatomy=tumbler.plan.json, tuning=E1 A1 D2 G2, gain=0)
#   tb.p = bass hand (bass, pos=0.14, pick=0.60, pm=0.22)
#   pr = Prism plucked chords (treble, anatomy=prism.plan.json, tuning=C3 F3 A3 C4 E4 G4, gain=-8)
#   pr.r = chord hand (treble, pos=0.065, pick=0.88, pm=0.12)
#   pn = Ariadne grand (treble, anatomy=piano-v1, gain=-20)
#   pn.l = piano lower staff (bass)
#   pn.r = piano upper staff (treble)
#   pn.ped = sustain pedal (perc, pedal=sustain)
#   pk = Pocket modal percussion (perc, anatomy=pocket.plan.json, tuning=E2 A2 D3 G3 B3 E4, gain=-15)
#   pk.low = low pulse (perc, pos=0.30, pick=0.16, pm=0.08, gain=0)
#   pk.tom = low reply (perc, pos=0.20, pick=0.32, pm=0.10, gain=-3)
#   pk.rim = rim figure (perc, pos=0.08, pick=0.91, pm=0.14, gain=-7)
#   pk.side = side figure (perc, pos=0.12, pick=0.70, pm=0.12, gain=-6)
#   pk.bead = bead figure (perc, pos=0.055, pick=0.97, pm=0.07, gain=-5)
#   pk.seed = seed figure (perc, pos=0.04, pick=0.83, pm=0.06, gain=-5)
"""


def assemble():
    parts = {}
    for name in ("melody", "harmony", "percussion"):
        incoming = json.loads((HERE / f"{name}.parts.json").read_text())
        assert not (parts.keys() & incoming.keys()), "duplicate performance voice"
        parts.update(incoming)
    assert len(HARMONY) == 48
    for voice, bars in parts.items():
        assert set(bars) == {str(b) for b in range(1, 49)}, voice
    text = [
        "# score: Sunskip",
        "# summary: 48 bars / 5 canonical Ariadne instrument worlds / original instrumental",
        "# tempo: 128    time: 4/4    key: C    bars: 48",
        "# swing: 8 54",
        "# master: rms=-19",
        "# tab: standard",
        "# text @b1: Light feet. The groove carries the detail; the theme keeps its shape.",
        "# text @b13: Let the minor-coloured answer pass without leaning on it.",
        "# text @b21: Piano in front; the other hands make room.",
        "# text @b29: Recognise the tune, then take the longer route.",
        "# text @b48: One last shared step, then let the world decay.",
        "", DECLARATIONS,
    ]
    for b, chord in enumerate(HARMONY, 1):
        if b in SECTIONS:
            text += ["", "# section: " + SECTIONS[b]]
        text.append(f"bar {b} [chord={chord}]: " + "  ".join(
            f"{voice}={bars[str(b)]}" for voice, bars in parts.items()
        ))
    (HERE / "sunskip.mus").write_text("\n".join(text) + "\n")
    print(f"sunskip.mus: {len(parts)} performance voices, 48 bars")


if __name__ == "__main__":
    assemble()
